use core::fmt::{self, Write};

use crate::{Error, Result};

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
        self.begin_container(FrameKind::Object, b'{')
    }

    pub fn end_object(&mut self) -> Result<()> {
        self.end_container(FrameKind::Object, b'}')
    }

    pub fn begin_array(&mut self) -> Result<()> {
        self.begin_container(FrameKind::Array, b'[')
    }

    pub fn end_array(&mut self) -> Result<()> {
        self.end_container(FrameKind::Array, b']')
    }

    pub fn key(&mut self, key: &str) -> Result<()> {
        let frame = self.current_frame_mut()?;
        if frame.kind != FrameKind::Object || frame.expecting_value {
            return Err(Error::BadSerialize);
        }
        if !frame.first {
            self.write_byte(b',')?;
        }
        self.current_frame_mut()?.first = false;
        self.write_quoted(key)?;
        self.write_byte(b':')?;
        self.current_frame_mut()?.expecting_value = true;
        Ok(())
    }

    pub fn null(&mut self) -> Result<()> {
        self.write_value_str("null")
    }

    pub fn bool(&mut self, value: bool) -> Result<()> {
        self.write_value_str(if value { "true" } else { "false" })
    }

    pub fn string(&mut self, value: &str) -> Result<()> {
        self.before_value()?;
        self.write_quoted(value)
    }

    pub fn i16(&mut self, value: i16) -> Result<()> {
        self.write_display(value)
    }

    pub fn u16(&mut self, value: u16) -> Result<()> {
        self.write_display(value)
    }

    pub fn i32(&mut self, value: i32) -> Result<()> {
        self.write_display(value)
    }

    pub fn u32(&mut self, value: u32) -> Result<()> {
        self.write_display(value)
    }

    pub fn i64(&mut self, value: i64) -> Result<()> {
        self.write_display(value)
    }

    pub fn u64(&mut self, value: u64) -> Result<()> {
        self.write_display(value)
    }

    pub fn f64(&mut self, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(Error::BadNum);
        }
        self.write_display(value)
    }

    pub fn raw_number(&mut self, number: &str) -> Result<()> {
        if !json_number(number.as_bytes()) {
            return Err(Error::BadNum);
        }
        self.write_value_str(number)
    }

    fn begin_container(&mut self, kind: FrameKind, open: u8) -> Result<()> {
        if self.depth == DEPTH {
            return Err(Error::NestTooDeep);
        }
        self.before_value()?;
        self.write_byte(open)?;
        self.stack[self.depth] = Frame::new(kind);
        self.depth += 1;
        Ok(())
    }

    fn end_container(&mut self, kind: FrameKind, close: u8) -> Result<()> {
        if self.depth == 0 {
            return Err(Error::NestMismatch);
        }
        let frame = self.stack[self.depth - 1];
        if frame.kind != kind || frame.expecting_value {
            return Err(Error::NestMismatch);
        }
        self.depth -= 1;
        self.write_byte(close)
    }

    fn before_value(&mut self) -> Result<()> {
        if self.depth == 0 {
            if self.root_written {
                return Err(Error::BadSerialize);
            }
            self.root_written = true;
            return Ok(());
        }

        let frame = &mut self.stack[self.depth - 1];
        match frame.kind {
            FrameKind::Array => {
                if !frame.first {
                    self.write_byte(b',')?;
                }
                self.stack[self.depth - 1].first = false;
                Ok(())
            }
            FrameKind::Object if frame.expecting_value => {
                self.stack[self.depth - 1].expecting_value = false;
                Ok(())
            }
            FrameKind::Object => Err(Error::BadSerialize),
        }
    }

    #[inline]
    fn current_frame_mut(&mut self) -> Result<&mut Frame> {
        if self.depth == 0 {
            Err(Error::BadSerialize)
        } else {
            Ok(&mut self.stack[self.depth - 1])
        }
    }

    fn write_value_str(&mut self, value: &str) -> Result<()> {
        self.before_value()?;
        self.write_str(value)
    }

    fn write_display(&mut self, value: impl fmt::Display) -> Result<()> {
        self.before_value()?;
        write!(self, "{value}").map_err(|_| Error::WriteLong)
    }

    fn write_quoted(&mut self, value: &str) -> Result<()> {
        self.write_byte(b'"')?;
        for ch in value.chars() {
            match ch {
                '"' => self.write_str("\\\"")?,
                '\\' => self.write_str("\\\\")?,
                '\u{08}' => self.write_str("\\b")?,
                '\u{0c}' => self.write_str("\\f")?,
                '\n' => self.write_str("\\n")?,
                '\r' => self.write_str("\\r")?,
                '\t' => self.write_str("\\t")?,
                '\u{00}'..='\u{1f}' => {
                    self.write_str("\\u00")?;
                    self.write_hex_digit((ch as u8) >> 4)?;
                    self.write_hex_digit(ch as u8)?;
                }
                _ => self.write_str(ch.encode_utf8(&mut [0; 4]))?,
            }
        }
        self.write_byte(b'"')
    }

    #[inline]
    fn write_hex_digit(&mut self, value: u8) -> Result<()> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        self.write_byte(HEX[(value & 0x0f) as usize])
    }

    #[inline]
    fn write_byte(&mut self, byte: u8) -> Result<()> {
        if self.len == self.out.len() {
            return Err(Error::WriteLong);
        }
        self.out[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    #[inline]
    fn write_str(&mut self, value: &str) -> Result<()> {
        if self.out.len() - self.len < value.len() {
            return Err(Error::WriteLong);
        }
        self.out[self.len..self.len + value.len()].copy_from_slice(value.as_bytes());
        self.len += value.len();
        Ok(())
    }
}

impl<const DEPTH: usize> fmt::Write for JsonSerializer<'_, DEPTH> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        JsonSerializer::write_str(self, s).map_err(|_| fmt::Error)
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
