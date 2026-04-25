use crate::{Error, JSON_VAL_MAX, Result};

#[inline]
pub(crate) fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[inline]
pub(crate) fn parse_i32_at(bytes: &[u8], start: usize) -> Result<(i32, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_i32_bytes(&bytes[start..end])?, end))
}

#[inline]
pub(crate) fn parse_u32_at(bytes: &[u8], start: usize) -> Result<(u32, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_u32_bytes(&bytes[start..end])?, end))
}

#[inline]
pub(crate) fn parse_i16_at(bytes: &[u8], start: usize) -> Result<(i16, usize)> {
    let end = number_end(bytes, start)?;
    let v = parse_i64_bytes(&bytes[start..end])?;
    if v < i16::MIN as i64 || v > i16::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok((v as i16, end))
}

#[inline]
pub(crate) fn parse_u16_at(bytes: &[u8], start: usize) -> Result<(u16, usize)> {
    let end = number_end(bytes, start)?;
    let v = parse_u64_bytes(&bytes[start..end])?;
    if v > u16::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok((v as u16, end))
}

#[inline]
pub(crate) fn parse_f64_at(bytes: &[u8], start: usize) -> Result<(f64, usize)> {
    let end = number_end(bytes, start)?;
    Ok((parse_f64_bytes(&bytes[start..end])?, end))
}

#[inline]
pub(crate) fn parse_bool_at(bytes: &[u8], start: usize) -> Result<(bool, usize)> {
    if bytes[start..].starts_with(b"true") {
        Ok((true, start + 4))
    } else if bytes[start..].starts_with(b"false") {
        Ok((false, start + 5))
    } else {
        let (v, end) = parse_i32_at(bytes, start)?;
        Ok((v != 0, end))
    }
}

pub(crate) fn number_end(bytes: &[u8], mut i: usize) -> Result<usize> {
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
pub(crate) fn parse_i32_token(token: &[u8]) -> Result<i32> {
    parse_i32_bytes(&token[..token_len(token)])
}

#[inline]
pub(crate) fn parse_u32_token(token: &[u8]) -> Result<u32> {
    parse_u32_bytes(&token[..token_len(token)])
}

#[inline]
pub(crate) fn parse_i16_token(token: &[u8]) -> Result<i16> {
    let v = parse_i64_token(token)?;
    if v < i16::MIN as i64 || v > i16::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok(v as i16)
}

#[inline]
pub(crate) fn parse_u16_token(token: &[u8]) -> Result<u16> {
    let v = parse_u64_bytes(&token[..token_len(token)])?;
    if v > u16::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok(v as u16)
}

#[inline]
pub(crate) fn parse_i64_token(token: &[u8]) -> Result<i64> {
    parse_i64_bytes(&token[..token_len(token)])
}

#[inline]
pub(crate) fn parse_f64_token(token: &[u8]) -> Result<f64> {
    parse_f64_bytes(&token[..token_len(token)])
}

#[inline]
pub(crate) fn parse_i32_bytes(bytes: &[u8]) -> Result<i32> {
    let v = parse_i64_bytes(bytes)?;
    if v < i32::MIN as i64 || v > i32::MAX as i64 {
        return Err(Error::BadNum);
    }
    Ok(v as i32)
}

#[inline]
pub(crate) fn parse_u32_bytes(bytes: &[u8]) -> Result<u32> {
    let v = parse_u64_bytes(bytes)?;
    if v > u32::MAX as u64 {
        return Err(Error::BadNum);
    }
    Ok(v as u32)
}

pub(crate) fn parse_i64_bytes(bytes: &[u8]) -> Result<i64> {
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

pub(crate) fn parse_u64_bytes(bytes: &[u8]) -> Result<u64> {
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

pub(crate) fn parse_f64_bytes(bytes: &[u8]) -> Result<f64> {
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
pub(crate) fn pow10(exp: i32) -> f64 {
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
pub(crate) fn json_number_is_integer(token: &[u8]) -> bool {
    let bytes = &token[..token_len(token)];
    match_json_number(bytes).is_some_and(|is_integer| is_integer)
}

#[inline]
pub(crate) fn json_number_is_real(token: &[u8]) -> bool {
    let bytes = &token[..token_len(token)];
    match_json_number(bytes).is_some_and(|is_integer| !is_integer)
}

pub(crate) fn match_json_number(bytes: &[u8]) -> Option<bool> {
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

pub(crate) fn write_i32_token(mut value: i32, out: &mut [u8; JSON_VAL_MAX + 1]) {
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

#[inline]
fn token_len(token: &[u8]) -> usize {
    token.iter().position(|&b| b == 0).unwrap_or(token.len())
}

#[inline]
fn clear_cbuf(buf: &mut [u8]) {
    for b in buf {
        *b = 0;
    }
}
