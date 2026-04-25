use crate::{
    Array, Attr, AttrKind, DefaultValue, Error, JSON_ATTR_MAX, JSON_VAL_MAX, Result,
    number::{
        hex_val, json_number_is_integer, json_number_is_real, parse_bool_at, parse_f64_at,
        parse_f64_token, parse_i16_at, parse_i16_token, parse_i32_at, parse_i32_token,
        parse_i64_token, parse_u16_at, parse_u16_token, parse_u32_at, parse_u32_token,
        write_i32_token,
    },
};

#[inline]
pub fn read_object(input: &str, attrs: &mut [Attr<'_>]) -> Result<usize> {
    read_object_internal(input, attrs, None, 0)
}

#[inline]
pub fn read_array(input: &str, array: &mut Array<'_>) -> Result<usize> {
    read_array_internal(input, array)
}

#[inline]
pub fn cstr(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).unwrap_or("")
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
