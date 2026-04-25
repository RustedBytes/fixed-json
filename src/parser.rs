use crate::{Array, Attr, AttrKind, DefaultValue, Error, JSON_ATTR_MAX, JSON_VAL_MAX, Result};

#[inline]
pub fn read_object(input: &str, attrs: &mut [Attr<'_>]) -> Result<usize> {
    read_object_internal(input, attrs, None, 0)
}

#[inline]
pub fn read_array(input: &str, array: &mut Array<'_>) -> Result<usize> {
    read_array_internal(input, array)
}

pub fn validate_json(input: &[u8]) -> Result<usize> {
    let mut validator = JsonValidator {
        bytes: input,
        depth: 0,
    };
    let end = validator.parse_value(validator.skip_ws(0))?;
    let end = validator.skip_ws(end);
    if end == input.len() {
        Ok(end)
    } else {
        Err(Error::BadTrail)
    }
}

#[inline]
pub fn cstr(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).unwrap_or("")
}

struct JsonValidator<'a> {
    bytes: &'a [u8],
    depth: usize,
}

impl JsonValidator<'_> {
    const MAX_DEPTH: usize = 1024;

    #[inline]
    fn parse_value(&mut self, i: usize) -> Result<usize> {
        if i >= self.bytes.len() {
            return Err(Error::BadTrail);
        }
        match self.bytes[i] {
            b'{' => self.parse_object(i),
            b'[' => self.parse_array(i),
            b'"' => self.parse_string(i),
            b't' => self.parse_literal(i, b"true"),
            b'f' => self.parse_literal(i, b"false"),
            b'n' => self.parse_literal(i, b"null"),
            b'-' | b'0'..=b'9' => self.parse_number(i),
            _ => Err(Error::BadTrail),
        }
    }

    fn parse_object(&mut self, mut i: usize) -> Result<usize> {
        self.enter()?;
        i += 1;
        i = self.skip_ws(i);
        if i < self.bytes.len() && self.bytes[i] == b'}' {
            self.leave();
            return Ok(i + 1);
        }
        loop {
            if i >= self.bytes.len() || self.bytes[i] != b'"' {
                self.leave();
                return Err(Error::AttrStart);
            }
            i = self.parse_string(i)?;
            i = self.skip_ws(i);
            if i >= self.bytes.len() || self.bytes[i] != b':' {
                self.leave();
                return Err(Error::BadTrail);
            }
            i = self.parse_value(self.skip_ws(i + 1))?;
            i = self.skip_ws(i);
            if i >= self.bytes.len() {
                self.leave();
                return Err(Error::BadTrail);
            }
            match self.bytes[i] {
                b',' => i = self.skip_ws(i + 1),
                b'}' => {
                    self.leave();
                    return Ok(i + 1);
                }
                _ => {
                    self.leave();
                    return Err(Error::BadTrail);
                }
            }
        }
    }

    fn parse_array(&mut self, mut i: usize) -> Result<usize> {
        self.enter()?;
        i += 1;
        i = self.skip_ws(i);
        if i < self.bytes.len() && self.bytes[i] == b']' {
            self.leave();
            return Ok(i + 1);
        }
        loop {
            i = self.parse_value(i)?;
            i = self.skip_ws(i);
            if i >= self.bytes.len() {
                self.leave();
                return Err(Error::BadTrail);
            }
            match self.bytes[i] {
                b',' => i = self.skip_ws(i + 1),
                b']' => {
                    self.leave();
                    return Ok(i + 1);
                }
                _ => {
                    self.leave();
                    return Err(Error::BadTrail);
                }
            }
        }
    }

    fn parse_string(&self, mut i: usize) -> Result<usize> {
        i += 1;
        let mut raw_start = i;
        while i < self.bytes.len() {
            match self.bytes[i] {
                b'"' => {
                    core::str::from_utf8(&self.bytes[raw_start..i])
                        .map_err(|_| Error::BadString)?;
                    return Ok(i + 1);
                }
                b'\\' => {
                    core::str::from_utf8(&self.bytes[raw_start..i])
                        .map_err(|_| Error::BadString)?;
                    i += 1;
                    if i >= self.bytes.len() {
                        return Err(Error::BadString);
                    }
                    match self.bytes[i] {
                        b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {
                            i += 1;
                        }
                        b'u' => {
                            i += 1;
                            for _ in 0..4 {
                                if i >= self.bytes.len() || hex_val(self.bytes[i]).is_none() {
                                    return Err(Error::BadString);
                                }
                                i += 1;
                            }
                        }
                        _ => return Err(Error::BadString),
                    }
                    raw_start = i;
                }
                0x00..=0x1f => return Err(Error::BadString),
                _ => i += 1,
            }
        }
        Err(Error::BadString)
    }

    #[inline]
    fn parse_literal(&self, i: usize, literal: &[u8]) -> Result<usize> {
        if self.bytes[i..].starts_with(literal) {
            Ok(i + literal.len())
        } else {
            Err(Error::BadTrail)
        }
    }

    fn parse_number(&self, mut i: usize) -> Result<usize> {
        if i < self.bytes.len() && self.bytes[i] == b'-' {
            i += 1;
        }
        if i >= self.bytes.len() {
            return Err(Error::BadNum);
        }
        if self.bytes[i] == b'0' {
            i += 1;
            if i < self.bytes.len() && self.bytes[i].is_ascii_digit() {
                return Err(Error::BadNum);
            }
        } else if self.bytes[i].is_ascii_digit() {
            while i < self.bytes.len() && self.bytes[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            return Err(Error::BadNum);
        }
        if i < self.bytes.len() && self.bytes[i] == b'.' {
            i += 1;
            let start = i;
            while i < self.bytes.len() && self.bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return Err(Error::BadNum);
            }
        }
        if i < self.bytes.len() && matches!(self.bytes[i], b'e' | b'E') {
            i += 1;
            if i < self.bytes.len() && matches!(self.bytes[i], b'+' | b'-') {
                i += 1;
            }
            let start = i;
            while i < self.bytes.len() && self.bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return Err(Error::BadNum);
            }
        }
        Ok(i)
    }

    #[inline]
    fn skip_ws(&self, mut i: usize) -> usize {
        while i < self.bytes.len() && matches!(self.bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
            i += 1;
        }
        i
    }

    #[inline]
    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > Self::MAX_DEPTH {
            return Err(Error::SubTooLong);
        }
        Ok(())
    }

    #[inline]
    fn leave(&mut self) {
        self.depth -= 1;
    }
}

fn read_object_internal(
    input: &str,
    attrs: &mut [Attr<'_>],
    parent_is_structobject: Option<bool>,
    offset: usize,
) -> Result<usize> {
    apply_defaults(attrs, parent_is_structobject, offset)?;

    let bytes = input.as_bytes();
    let mut i = skip_ws(bytes, 0);
    if i >= bytes.len() {
        return Err(Error::Empty);
    }
    if bytes[i] != b'{' {
        return Err(Error::ObStart);
    }
    i += 1;

    loop {
        i = skip_ws(bytes, i);
        if i >= bytes.len() {
            return Err(Error::AttrStart);
        }
        if bytes[i] == b'}' {
            i += 1;
            return Ok(skip_ws(bytes, i));
        }
        if bytes[i] != b'"' {
            return Err(Error::AttrStart);
        }
        i += 1;

        let mut attr_buf = [0u8; JSON_ATTR_MAX + 1];
        let attr_len = parse_attr_name(bytes, &mut i, &mut attr_buf)?;
        let attr_name = core::str::from_utf8(&attr_buf[..attr_len]).map_err(|_| Error::BadAttr)?;
        let first = find_attr(attrs, attr_name).ok_or(Error::BadAttr)?;

        i = skip_ws(bytes, i);
        if i >= bytes.len() || bytes[i] != b':' {
            return Err(Error::BadTrail);
        }
        i += 1;
        i = skip_ws(bytes, i);
        if i >= bytes.len() {
            return Err(Error::BadTrail);
        }

        if bytes[i] == b'[' {
            if !matches!(attrs[first].kind, AttrKind::Array(_)) {
                return Err(Error::NoArray);
            }
            let AttrKind::Array(array) = &mut attrs[first].kind else {
                unreachable!();
            };
            i += read_array_internal(&input[i..], array)?;
            i = skip_ws(bytes, i);
        } else if matches!(attrs[first].kind, AttrKind::Array(_)) {
            return Err(Error::NoBrak);
        } else if bytes[i] == b'{' {
            if !matches!(attrs[first].kind, AttrKind::Object(_)) {
                return Err(Error::NoArray);
            }
            let AttrKind::Object(child) = &mut attrs[first].kind else {
                unreachable!();
            };
            i += read_object_internal(&input[i..], child, None, 0)?;
            i = skip_ws(bytes, i);
        } else if matches!(attrs[first].kind, AttrKind::Object(_)) {
            return Err(Error::NoCurly);
        } else {
            let mut val_buf = [0u8; JSON_VAL_MAX + 1];
            let quoted = if bytes[i] == b'"' {
                i += 1;
                let maxlen = value_max_len(&attrs[first]);
                parse_string_value(bytes, &mut i, &mut val_buf, maxlen)?;
                true
            } else {
                parse_token_value(bytes, &mut i, &mut val_buf)?;
                false
            };
            let selected = select_attr(attrs, first, attr_name, &val_buf, quoted)?;
            apply_value(
                &mut attrs[selected],
                parent_is_structobject,
                offset,
                &val_buf,
                quoted,
            )?;
            i = skip_ws(bytes, i);
        }

        if i >= bytes.len() {
            return Err(Error::BadTrail);
        }
        if bytes[i] == b',' {
            i += 1;
        } else if bytes[i] == b'}' {
            i += 1;
            return Ok(skip_ws(bytes, i));
        } else {
            return Err(Error::BadTrail);
        }
    }
}

fn read_array_internal(input: &str, array: &mut Array<'_>) -> Result<usize> {
    let bytes = input.as_bytes();
    let mut i = skip_ws(bytes, 0);
    if i >= bytes.len() || bytes[i] != b'[' {
        return Err(Error::ArrayStart);
    }
    i += 1;
    i = skip_ws(bytes, i);
    if i < bytes.len() && bytes[i] == b']' {
        set_array_count(array, 0);
        return Ok(skip_ws(bytes, i + 1));
    }

    let maxlen = array_maxlen(array);
    let mut count = 0usize;
    while count < maxlen {
        i = skip_ws(bytes, i);
        match array {
            Array::Strings { store, .. } => {
                if i >= bytes.len() || bytes[i] != b'"' {
                    return Err(Error::BadString);
                }
                i += 1;
                parse_string_value(
                    bytes,
                    &mut i,
                    store[count],
                    store[count].len().saturating_sub(1),
                )?;
            }
            Array::Integers { store, .. } => {
                let (v, end) = parse_i32_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::UIntegers { store, .. } => {
                let (v, end) = parse_u32_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::Shorts { store, .. } => {
                let (v, end) = parse_i16_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::UShorts { store, .. } => {
                let (v, end) = parse_u16_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::Reals { store, .. } => {
                let (v, end) = parse_f64_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::Booleans { store, .. } => {
                let (v, end) = parse_bool_at(bytes, i)?;
                store[count] = v;
                i = end;
            }
            Array::Objects { attrs, .. } => {
                i += read_object_internal(&input[i..], attrs, Some(false), count)?;
            }
            Array::StructObjects { parser, .. } => {
                i += parser(&input[i..], count)?;
            }
        }
        count += 1;
        i = skip_ws(bytes, i);
        if i < bytes.len() && bytes[i] == b']' {
            set_array_count(array, count);
            return Ok(skip_ws(bytes, i + 1));
        }
        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
        } else {
            return Err(Error::BadSubTrail);
        }
    }

    Err(Error::SubTooLong)
}

fn apply_defaults(
    attrs: &mut [Attr<'_>],
    parent_is_structobject: Option<bool>,
    offset: usize,
) -> Result<()> {
    for attr in attrs {
        if attr.nodefault {
            continue;
        }
        match attr.default {
            DefaultValue::None => {
                if let AttrKind::String(buf) = &mut attr.kind {
                    if matches!(parent_is_structobject, Some(false)) && offset > 0 {
                        return Err(Error::NoParStr);
                    }
                    clear_cbuf(buf);
                }
            }
            DefaultValue::Integer(v) => set_i32(&mut attr.kind, offset, v)?,
            DefaultValue::UInteger(v) => set_u32(&mut attr.kind, offset, v)?,
            DefaultValue::Short(v) => set_i16(&mut attr.kind, offset, v)?,
            DefaultValue::UShort(v) => set_u16(&mut attr.kind, offset, v)?,
            DefaultValue::Real(v) => set_f64(&mut attr.kind, offset, v)?,
            DefaultValue::Boolean(v) => set_bool(&mut attr.kind, offset, v)?,
            DefaultValue::Character(v) => set_char(&mut attr.kind, offset, v)?,
            DefaultValue::Check(_) => {}
        }
    }
    Ok(())
}

fn parse_attr_name(
    bytes: &[u8],
    i: &mut usize,
    out: &mut [u8; JSON_ATTR_MAX + 1],
) -> Result<usize> {
    let mut len = 0usize;
    while *i < bytes.len() {
        let b = bytes[*i];
        *i += 1;
        if b == b'"' {
            return Ok(len);
        }
        if len >= JSON_ATTR_MAX {
            return Err(Error::AttrLen);
        }
        out[len] = b;
        len += 1;
    }
    Err(Error::BadString)
}

fn parse_string_value(
    bytes: &[u8],
    i: &mut usize,
    out: &mut [u8],
    max_content_len: usize,
) -> Result<usize> {
    clear_cbuf(out);
    let mut len = 0usize;
    while *i < bytes.len() {
        let b = bytes[*i];
        *i += 1;
        let outb = if b == b'\\' {
            if *i >= bytes.len() {
                return Err(Error::BadString);
            }
            let e = bytes[*i];
            *i += 1;
            match e {
                b'b' => 8,
                b'f' => 12,
                b'n' => b'\n',
                b'r' => b'\r',
                b't' => b'\t',
                b'u' => {
                    if *i + 4 > bytes.len() {
                        return Err(Error::BadString);
                    }
                    let mut v = 0u32;
                    for _ in 0..4 {
                        let Some(d) = hex_val(bytes[*i]) else {
                            return Err(Error::BadString);
                        };
                        v = (v << 4) | d as u32;
                        *i += 1;
                    }
                    v as u8
                }
                other => other,
            }
        } else if b == b'"' {
            if len < out.len() {
                out[len] = 0;
            }
            return Ok(len);
        } else {
            b
        };
        if len >= JSON_VAL_MAX || len >= max_content_len || len + 1 > out.len() {
            return Err(Error::StrLong);
        }
        out[len] = outb;
        len += 1;
    }
    Err(Error::BadString)
}

fn parse_token_value(
    bytes: &[u8],
    i: &mut usize,
    out: &mut [u8; JSON_VAL_MAX + 1],
) -> Result<usize> {
    let mut len = 0usize;
    while *i < bytes.len() {
        let b = bytes[*i];
        if is_ws(b) || b == b',' || b == b'}' || b == b']' {
            out[len] = 0;
            return Ok(len);
        }
        if len >= JSON_VAL_MAX {
            return Err(Error::TokLong);
        }
        out[len] = b;
        len += 1;
        *i += 1;
    }
    out[len] = 0;
    Ok(len)
}

fn find_attr(attrs: &[Attr<'_>], name: &str) -> Option<usize> {
    let mut ignore = None;
    for (idx, attr) in attrs.iter().enumerate() {
        if attr.name == name {
            return Some(idx);
        }
        if attr.name.is_empty() && matches!(attr.kind, AttrKind::Ignore) {
            ignore = Some(idx);
        }
    }
    ignore
}

fn select_attr(
    attrs: &[Attr<'_>],
    first: usize,
    name: &str,
    val: &[u8; JSON_VAL_MAX + 1],
    quoted: bool,
) -> Result<usize> {
    let mut idx = first;
    loop {
        if value_fits_attr(&attrs[idx], val, quoted) {
            break;
        }
        if idx + 1 >= attrs.len() || attrs[idx + 1].name != name {
            break;
        }
        idx += 1;
    }

    let attr = &attrs[idx];
    if quoted
        && !matches!(
            attr.kind,
            AttrKind::String(_)
                | AttrKind::Character(_)
                | AttrKind::Check(_)
                | AttrKind::Time(_)
                | AttrKind::Ignore
        )
        && attr.map.is_none()
    {
        return Err(Error::QNonString);
    }
    if !quoted
        && (matches!(
            attr.kind,
            AttrKind::String(_) | AttrKind::Check(_) | AttrKind::Time(_)
        ) || attr.map.is_some())
    {
        return Err(Error::NonQString);
    }
    Ok(idx)
}

fn value_fits_attr(attr: &Attr<'_>, val: &[u8; JSON_VAL_MAX + 1], quoted: bool) -> bool {
    if quoted {
        return matches!(attr.kind, AttrKind::String(_) | AttrKind::Time(_));
    }
    if token_eq(val, b"true") || token_eq(val, b"false") {
        return matches!(attr.kind, AttrKind::Boolean(_));
    }
    if json_number_is_integer(val) {
        return matches!(
            attr.kind,
            AttrKind::Integer(_) | AttrKind::UInteger(_) | AttrKind::Short(_) | AttrKind::UShort(_)
        );
    }
    if json_number_is_real(val) {
        return matches!(attr.kind, AttrKind::Real(_));
    }
    false
}

fn apply_value(
    attr: &mut Attr<'_>,
    parent_is_structobject: Option<bool>,
    offset: usize,
    val: &[u8; JSON_VAL_MAX + 1],
    _quoted: bool,
) -> Result<()> {
    let mut mapped_buf = [0u8; JSON_VAL_MAX + 1];
    let val = if let Some(map) = attr.map {
        let s = token_str(val)?;
        let mut found = None;
        for item in map {
            if item.name == s {
                found = Some(item.value);
                break;
            }
        }
        let Some(found) = found else {
            return Err(Error::BadEnum);
        };
        write_i32_token(found, &mut mapped_buf);
        &mapped_buf
    } else {
        val
    };

    match &mut attr.kind {
        AttrKind::Integer(_) => {
            let v = parse_i32_token(val)?;
            set_i32(&mut attr.kind, offset, v)
        }
        AttrKind::UInteger(_) => {
            let v = parse_u32_token(val)?;
            set_u32(&mut attr.kind, offset, v)
        }
        AttrKind::Short(_) => {
            let v = parse_i16_token(val)?;
            set_i16(&mut attr.kind, offset, v)
        }
        AttrKind::UShort(_) => {
            let v = parse_u16_token(val)?;
            set_u16(&mut attr.kind, offset, v)
        }
        AttrKind::Real(_) => {
            let v = parse_f64_token(val)?;
            set_f64(&mut attr.kind, offset, v)
        }
        AttrKind::String(buf) => {
            if matches!(parent_is_structobject, Some(false)) && offset > 0 {
                return Err(Error::NoParStr);
            }
            copy_cbuf(buf, val);
            Ok(())
        }
        AttrKind::Boolean(_) => {
            let v = if token_eq(val, b"true") {
                true
            } else if token_eq(val, b"false") {
                false
            } else {
                parse_i64_token(val)? != 0
            };
            set_bool(&mut attr.kind, offset, v)
        }
        AttrKind::Character(_) => {
            let len = token_len(val);
            if len > 1 {
                return Err(Error::StrLong);
            }
            set_char(&mut attr.kind, offset, if len == 0 { 0 } else { val[0] })
        }
        AttrKind::Check(expected) => {
            if token_str(val)? == *expected {
                Ok(())
            } else {
                Err(Error::CheckFail)
            }
        }
        AttrKind::Ignore => Ok(()),
        AttrKind::Time(_) => Ok(()),
        AttrKind::Object(_) | AttrKind::Array(_) => Ok(()),
    }
}

#[inline]
fn set_i32(kind: &mut AttrKind<'_>, offset: usize, value: i32) -> Result<()> {
    if let AttrKind::Integer(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn set_u32(kind: &mut AttrKind<'_>, offset: usize, value: u32) -> Result<()> {
    if let AttrKind::UInteger(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn set_i16(kind: &mut AttrKind<'_>, offset: usize, value: i16) -> Result<()> {
    if let AttrKind::Short(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn set_u16(kind: &mut AttrKind<'_>, offset: usize, value: u16) -> Result<()> {
    if let AttrKind::UShort(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn set_f64(kind: &mut AttrKind<'_>, offset: usize, value: f64) -> Result<()> {
    match kind {
        AttrKind::Real(target) | AttrKind::Time(target) => target.set(offset, value)?,
        _ => {}
    }
    Ok(())
}

#[inline]
fn set_bool(kind: &mut AttrKind<'_>, offset: usize, value: bool) -> Result<()> {
    if let AttrKind::Boolean(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn set_char(kind: &mut AttrKind<'_>, offset: usize, value: u8) -> Result<()> {
    if let AttrKind::Character(target) = kind {
        target.set(offset, value)?;
    }
    Ok(())
}

#[inline]
fn value_max_len(attr: &Attr<'_>) -> usize {
    match &attr.kind {
        AttrKind::String(buf) => buf.len().saturating_sub(1),
        AttrKind::Check(expected) => expected.len(),
        AttrKind::Time(_) | AttrKind::Ignore => JSON_VAL_MAX,
        _ if attr.map.is_some() => JSON_VAL_MAX,
        _ => JSON_VAL_MAX,
    }
}

#[inline]
fn array_maxlen(array: &Array<'_>) -> usize {
    match array {
        Array::Strings { store, .. } => store.len(),
        Array::Integers { store, .. } => store.len(),
        Array::UIntegers { store, .. } => store.len(),
        Array::Shorts { store, .. } => store.len(),
        Array::UShorts { store, .. } => store.len(),
        Array::Reals { store, .. } => store.len(),
        Array::Booleans { store, .. } => store.len(),
        Array::Objects { maxlen, .. } => *maxlen,
        Array::StructObjects { maxlen, .. } => *maxlen,
    }
}

#[inline]
fn set_array_count(array: &mut Array<'_>, value: usize) {
    match array {
        Array::Strings { count, .. }
        | Array::Integers { count, .. }
        | Array::UIntegers { count, .. }
        | Array::Shorts { count, .. }
        | Array::UShorts { count, .. }
        | Array::Reals { count, .. }
        | Array::Booleans { count, .. }
        | Array::Objects { count, .. }
        | Array::StructObjects { count, .. } => {
            if let Some(count) = count {
                **count = value;
            }
        }
    }
}

#[inline]
fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && is_ws(bytes[i]) {
        i += 1;
    }
    i
}

#[inline]
fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b)
}

#[inline]
fn clear_cbuf(buf: &mut [u8]) {
    for b in buf {
        *b = 0;
    }
}

#[inline]
fn copy_cbuf(out: &mut [u8], val: &[u8; JSON_VAL_MAX + 1]) {
    clear_cbuf(out);
    if out.is_empty() {
        return;
    }
    let n = core::cmp::min(token_len(val), out.len() - 1);
    out[..n].copy_from_slice(&val[..n]);
}

#[inline]
fn token_len(token: &[u8]) -> usize {
    token.iter().position(|&b| b == 0).unwrap_or(token.len())
}

#[inline]
fn token_str(token: &[u8]) -> Result<&str> {
    core::str::from_utf8(&token[..token_len(token)]).map_err(|_| Error::BadString)
}

#[inline]
fn token_eq(token: &[u8], expected: &[u8]) -> bool {
    &token[..token_len(token)] == expected
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[inline]
fn parse_i32_at(bytes: &[u8], start: usize) -> Result<(i32, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_i32_bytes(&bytes[start..end])?, end))
}

#[inline]
fn parse_u32_at(bytes: &[u8], start: usize) -> Result<(u32, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_u32_bytes(&bytes[start..end])?, end))
}

#[inline]
fn parse_i16_at(bytes: &[u8], start: usize) -> Result<(i16, usize)> {
    let end = number_end(bytes, start)?;
    let v = parse_i64_bytes(&bytes[start..end])?;
    if v < i16::MIN as i64 || v > i16::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok((v as i16, end))
}

#[inline]
fn parse_u16_at(bytes: &[u8], start: usize) -> Result<(u16, usize)> {
    let end = number_end(bytes, start)?;
    let v = parse_u64_bytes(&bytes[start..end])?;
    if v > u16::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok((v as u16, end))
}

#[inline]
fn parse_f64_at(bytes: &[u8], start: usize) -> Result<(f64, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_f64_bytes(&bytes[start..end])?, end))
}

#[inline]
fn parse_bool_at(bytes: &[u8], start: usize) -> Result<(bool, usize)> {
    if bytes[start..].starts_with(b"true") {
        Ok((true, start + 4))
    } else if bytes[start..].starts_with(b"false") {
        Ok((false, start + 5))
    } else {
        let (v, end) = parse_i32_at(bytes, start)?;
        Ok((v != 0, end))
    }
}

fn number_end(bytes: &[u8], mut i: usize) -> Result<usize> {
    let start = i;
    if i < bytes.len() && bytes[i] == b'-' {
        i += 1;
    }
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        saw_digit = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            saw_digit = true;
            i += 1;
        }
    }
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        i += 1;
        if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_start {
            return Err(Error::BadNum);
        }
    }
    if !saw_digit || i == start {
        Err(Error::BadNum)
    } else {
        Ok(i)
    }
}

#[inline]
fn parse_i32_token(token: &[u8]) -> Result<i32> {
    parse_i32_bytes(&token[..token_len(token)])
}

#[inline]
fn parse_u32_token(token: &[u8]) -> Result<u32> {
    parse_u32_bytes(&token[..token_len(token)])
}

#[inline]
fn parse_i16_token(token: &[u8]) -> Result<i16> {
    let v = parse_i64_token(token)?;
    if v < i16::MIN as i64 || v > i16::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok(v as i16)
}

#[inline]
fn parse_u16_token(token: &[u8]) -> Result<u16> {
    let v = parse_u64_bytes(&token[..token_len(token)])?;
    if v > u16::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok(v as u16)
}

#[inline]
fn parse_i64_token(token: &[u8]) -> Result<i64> {
    parse_i64_bytes(&token[..token_len(token)])
}

#[inline]
fn parse_f64_token(token: &[u8]) -> Result<f64> {
    parse_f64_bytes(&token[..token_len(token)])
}

#[inline]
fn parse_i32_bytes(bytes: &[u8]) -> Result<i32> {
    let v = parse_i64_bytes(bytes)?;
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok(v as i32)
}

#[inline]
fn parse_u32_bytes(bytes: &[u8]) -> Result<u32> {
    let v = parse_u64_bytes(bytes)?;
    if v > u32::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok(v as u32)
}

fn parse_i64_bytes(bytes: &[u8]) -> Result<i64> {
    let mut i = 0usize;
    let neg = bytes.first() == Some(&b'-');
    if neg {
        i = 1;
    }
    if i >= bytes.len() || bytes[i] == b'+' {
        return Err(Error::BadNum);
    }
    let mut v: i64 = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if !b.is_ascii_digit() {
            return Err(Error::BadNum);
        }
        v = v.checked_mul(10).ok_or(Error::BadNum)?;
        v = v.checked_add((b - b'0') as i64).ok_or(Error::BadNum)?;
        i += 1;
    }
    if neg {
        v.checked_neg().ok_or(Error::BadNum)
    } else {
        Ok(v)
    }
}

fn parse_u64_bytes(bytes: &[u8]) -> Result<u64> {
    if bytes.is_empty() || bytes[0] == b'-' || bytes[0] == b'+' {
        return Err(Error::BadNum);
    }
    let mut v: u64 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return Err(Error::BadNum);
        }
        v = v.checked_mul(10).ok_or(Error::BadNum)?;
        v = v.checked_add((b - b'0') as u64).ok_or(Error::BadNum)?;
    }
    Ok(v)
}

fn parse_f64_bytes(bytes: &[u8]) -> Result<f64> {
    if bytes.is_empty() {
        return Err(Error::BadNum);
    }
    let mut i = 0usize;
    let neg = bytes[i] == b'-';
    if neg {
        i += 1;
    }
    if i >= bytes.len() {
        return Err(Error::BadNum);
    }
    let mut value = 0f64;
    let int_start = i;
    if bytes[i] == b'0' {
        i += 1;
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            return Err(Error::BadNum);
        }
    } else {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            value = value * 10.0 + (bytes[i] - b'0') as f64;
            i += 1;
        }
    }
    if i == int_start {
        return Err(Error::BadNum);
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        let frac_start = i;
        let mut scale = 0.1f64;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            value += (bytes[i] - b'0') as f64 * scale;
            scale *= 0.1;
            i += 1;
        }
        if i == frac_start {
            return Err(Error::BadNum);
        }
    }
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        i += 1;
        let exp_neg = if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
            let n = bytes[i] == b'-';
            i += 1;
            n
        } else {
            false
        };
        let exp_start = i;
        let mut exp = 0i32;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            exp = exp
                .saturating_mul(10)
                .saturating_add((bytes[i] - b'0') as i32);
            i += 1;
        }
        if i == exp_start {
            return Err(Error::BadNum);
        }
        value *= pow10(if exp_neg { -exp } else { exp });
    }
    if i != bytes.len() {
        return Err(Error::BadNum);
    }
    Ok(if neg { -value } else { value })
}

#[inline]
fn pow10(exp: i32) -> f64 {
    let mut n = if exp < 0 { -exp } else { exp };
    let mut base = 10.0f64;
    let mut result = 1.0f64;
    while n > 0 {
        if n & 1 == 1 {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }
    if exp < 0 { 1.0 / result } else { result }
}

#[inline]
fn json_number_is_integer(token: &[u8]) -> bool {
    let bytes = &token[..token_len(token)];
    match_json_number(bytes).is_some_and(|is_integer| is_integer)
}

#[inline]
fn json_number_is_real(token: &[u8]) -> bool {
    let bytes = &token[..token_len(token)];
    match_json_number(bytes).is_some_and(|is_integer| !is_integer)
}

fn match_json_number(bytes: &[u8]) -> Option<bool> {
    if bytes.is_empty() {
        return None;
    }
    let mut i = 0usize;
    let mut saw_fraction = false;
    let mut saw_exponent = false;
    if bytes[i] == b'-' {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    if bytes[i] == b'0' {
        i += 1;
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            return None;
        }
    } else {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && bytes[i] == b'.' {
        saw_fraction = true;
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return None;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        saw_exponent = true;
        i += 1;
        if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
            i += 1;
        }
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return None;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i == bytes.len() {
        Some(!(saw_fraction || saw_exponent))
    } else {
        None
    }
}

fn write_i32_token(mut value: i32, out: &mut [u8; JSON_VAL_MAX + 1]) {
    clear_cbuf(out);
    if value == 0 {
        out[0] = b'0';
        return;
    }
    let mut tmp = [0u8; 12];
    let neg = value < 0;
    let mut i = tmp.len();
    while value != 0 {
        let digit = (value % 10).unsigned_abs() as u8;
        i -= 1;
        tmp[i] = b'0' + digit;
        value /= 10;
    }
    if neg {
        i -= 1;
        tmp[i] = b'-';
    }
    let n = tmp.len() - i;
    out[..n].copy_from_slice(&tmp[i..]);
}
