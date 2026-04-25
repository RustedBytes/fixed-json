use core::fmt::{self, Write};

use crate::{Error, Result};

macro_rules! integer_writer {
    ($name:ident, $ty:ty) => {
        pub fn $name(&mut self, value: $ty) -> Result<()> {
            write_display(self, value)
        }
    };
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum FrameKind {
    Object,
    Array,
}

#[derive(Clone, Copy)]
struct Frame {
    kind: FrameKind,
    first: bool,
    expecting_value: bool,
}

impl Frame {
    #[inline]
    const fn new(kind: FrameKind) -> Self {
        Self {
            kind,
            first: true,
            expecting_value: false,
        }
    }
}

/// Streaming JSON serializer that writes into caller-owned fixed storage.
///
/// The serializer never allocates. `DEPTH` is the maximum number of open
/// arrays/objects tracked by the internal stack.
pub struct JsonSerializer<'a, const DEPTH: usize> {
    out: &'a mut [u8],
    len: usize,
    stack: [Frame; DEPTH],
    depth: usize,
    root_written: bool,
}

impl<'a, const DEPTH: usize> JsonSerializer<'a, DEPTH> {
    #[inline]
    pub fn new(out: &'a mut [u8]) -> Self {
        Self {
            out,
            len: 0,
            stack: [Frame::new(FrameKind::Array); DEPTH],
            depth: 0,
            root_written: false,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.out.len()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: all bytes are written from UTF-8 string literals, input `str`
        // slices, or ASCII escape sequences.
        unsafe { core::str::from_utf8_unchecked(&self.out[..self.len]) }
    }

    #[inline]
    pub fn finish(&self) -> Result<&str> {
        if self.depth == 0 && self.root_written {
            Ok(self.as_str())
        } else if self.depth == 0 {
            Err(Error::BadSerialize)
        } else {
            Err(Error::NestMismatch)
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.len = 0;
        self.depth = 0;
        self.root_written = false;
    }

    pub fn begin_object(&mut self) -> Result<()> {
        begin_container(self, FrameKind::Object, b'{')
    }

    pub fn end_object(&mut self) -> Result<()> {
        end_container(self, FrameKind::Object, b'}')
    }

    pub fn begin_array(&mut self) -> Result<()> {
        begin_container(self, FrameKind::Array, b'[')
    }

    pub fn end_array(&mut self) -> Result<()> {
        end_container(self, FrameKind::Array, b']')
    }

    pub fn key(&mut self, key: &str) -> Result<()> {
        let frame = current_frame_mut(self)?;
        if frame.kind != FrameKind::Object || frame.expecting_value {
            return Err(Error::BadSerialize);
        }
        if !frame.first {
            write_byte(self, b',')?;
        }
        current_frame_mut(self)?.first = false;
        write_quoted(self, key)?;
        write_byte(self, b':')?;
        current_frame_mut(self)?.expecting_value = true;
        Ok(())
    }

    pub fn null(&mut self) -> Result<()> {
        write_value_str(self, "null")
    }

    pub fn bool(&mut self, value: bool) -> Result<()> {
        write_value_str(self, if value { "true" } else { "false" })
    }

    pub fn string(&mut self, value: &str) -> Result<()> {
        before_value(self)?;
        write_quoted(self, value)
    }

    integer_writer!(i16, i16);
    integer_writer!(u16, u16);
    integer_writer!(i32, i32);
    integer_writer!(u32, u32);
    integer_writer!(i64, i64);
    integer_writer!(u64, u64);

    pub fn f64(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(Error::BadNum);
        }
        write_display(self, value)
    }

    pub fn raw_number(&mut self, number: &str) -> Result<()> {
        if !json_number(number.as_bytes()) {
            return Err(Error::BadNum);
        }
        write_value_str(self, number)
    }
}

fn begin_container<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    kind: FrameKind,
    open: u8,
) -> Result<()> {
    if serializer.depth == DEPTH {
        return Err(Error::NestTooDeep);
    }
    before_value(serializer)?;
    write_byte(serializer, open)?;
    serializer.stack[serializer.depth] = Frame::new(kind);
    serializer.depth += 1;
    Ok(())
}

fn end_container<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    kind: FrameKind,
    close: u8,
) -> Result<()> {
    if serializer.depth == 0 {
        return Err(Error::NestMismatch);
    }
    let frame = serializer.stack[serializer.depth - 1];
    if frame.kind != kind || frame.expecting_value {
        return Err(Error::NestMismatch);
    }
    serializer.depth -= 1;
    write_byte(serializer, close)
}

fn before_value<const DEPTH: usize>(serializer: &mut JsonSerializer<'_, DEPTH>) -> Result<()> {
    if serializer.depth == 0 {
        if serializer.root_written {
            return Err(Error::BadSerialize);
        }
        serializer.root_written = true;
        return Ok(());
    }

    let frame = &mut serializer.stack[serializer.depth - 1];
    match frame.kind {
        FrameKind::Array => {
            if !frame.first {
                write_byte(serializer, b',')?;
            }
            serializer.stack[serializer.depth - 1].first = false;
            Ok(())
        }
        FrameKind::Object if frame.expecting_value => {
            serializer.stack[serializer.depth - 1].expecting_value = false;
            Ok(())
        }
        FrameKind::Object => Err(Error::BadSerialize),
    }
}

#[inline]
fn current_frame_mut<'s, 'out, const DEPTH: usize>(
    serializer: &'s mut JsonSerializer<'out, DEPTH>,
) -> Result<&'s mut Frame> {
    if serializer.depth == 0 {
        Err(Error::BadSerialize)
    } else {
        Ok(&mut serializer.stack[serializer.depth - 1])
    }
}

fn write_value_str<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    value: &str,
) -> Result<()> {
    before_value(serializer)?;
    write_raw_str(serializer, value)
}

fn write_display<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    value: impl fmt::Display,
) -> Result<()> {
    before_value(serializer)?;
    write!(serializer, "{value}").map_err(|_| Error::WriteLong)
}

fn write_quoted<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    value: &str,
) -> Result<()> {
    write_byte(serializer, b'"')?;
    for ch in value.chars() {
        match ch {
            '"' => write_raw_str(serializer, "\\\"")?,
            '\\' => write_raw_str(serializer, "\\\\")?,
            '\u{08}' => write_raw_str(serializer, "\\b")?,
            '\u{0c}' => write_raw_str(serializer, "\\f")?,
            '\n' => write_raw_str(serializer, "\\n")?,
            '\r' => write_raw_str(serializer, "\\r")?,
            '\t' => write_raw_str(serializer, "\\t")?,
            '\u{00}'..='\u{1f}' => {
                write_raw_str(serializer, "\\u00")?;
                write_hex_digit(serializer, (ch as u8) >> 4)?;
                write_hex_digit(serializer, ch as u8)?;
            }
            _ => write_raw_str(serializer, ch.encode_utf8(&mut [0; 4]))?,
        }
    }
    write_byte(serializer, b'"')
}

#[inline]
fn write_hex_digit<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    value: u8,
) -> Result<()> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    write_byte(serializer, HEX[(value & 0x0f) as usize])
}

#[inline]
fn write_byte<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    byte: u8,
) -> Result<()> {
    if serializer.len == serializer.out.len() {
        return Err(Error::WriteLong);
    }
    serializer.out[serializer.len] = byte;
    serializer.len += 1;
    Ok(())
}

#[inline]
fn write_raw_str<const DEPTH: usize>(
    serializer: &mut JsonSerializer<'_, DEPTH>,
    value: &str,
) -> Result<()> {
    if serializer.out.len() - serializer.len < value.len() {
        return Err(Error::WriteLong);
    }
    serializer.out[serializer.len..serializer.len + value.len()].copy_from_slice(value.as_bytes());
    serializer.len += value.len();
    Ok(())
}

impl<const DEPTH: usize> fmt::Write for JsonSerializer<'_, DEPTH> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_raw_str(self, s).map_err(|_| fmt::Error)
    }
}

fn json_number(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let mut i = 0usize;
    if bytes[i] == b'-' {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return false;
    }
    if bytes[i] == b'0' {
        i += 1;
        if i < bytes.len() && bytes[i].is_ascii_digit() {
            return false;
        }
    } else {
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
        i += 1;
        if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
            i += 1;
        }
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return false;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    i == bytes.len()
}
