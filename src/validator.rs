use crate::{Error, Result, number::hex_val};

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

pub(crate) struct JsonValidator<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) depth: usize,
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

    pub(crate) fn parse_string(&self, mut i: usize) -> Result<usize> {
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

    pub(crate) fn parse_number(&self, mut i: usize) -> Result<usize> {
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
    pub(crate) fn skip_ws(&self, mut i: usize) -> usize {
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
