pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum Error {
    ObStart = 1,
    AttrStart = 2,
    BadAttr = 3,
    AttrLen = 4,
    NoArray = 5,
    NoBrak = 6,
    StrLong = 7,
    TokLong = 8,
    BadTrail = 9,
    ArrayStart = 10,
    ObjArr = 11,
    SubTooLong = 12,
    BadSubTrail = 13,
    SubType = 14,
    BadString = 15,
    CheckFail = 16,
    NoParStr = 17,
    BadEnum = 18,
    QNonString = 19,
    NonQString = 20,
    Misc = 21,
    BadNum = 22,
    NullPtr = 23,
    NoCurly = 24,
    Empty = 25,
    WriteLong = 26,
    NestTooDeep = 27,
    NestMismatch = 28,
    BadSerialize = 29,
}

impl Error {
    #[inline]
    pub const fn message(self) -> &'static str {
        match self {
            Error::ObStart => "non-whitespace when expecting object start",
            Error::AttrStart => "non-whitespace when expecting attribute start",
            Error::BadAttr => "unknown attribute name",
            Error::AttrLen => "attribute name too long",
            Error::NoArray => "saw [ when not expecting array",
            Error::NoBrak => "array element specified, but no [",
            Error::StrLong => "string value too long",
            Error::TokLong => "token value too long",
            Error::BadTrail => "garbage while expecting comma or } or ]",
            Error::ArrayStart => "didn't find expected array start",
            Error::ObjArr => "error while parsing object array",
            Error::SubTooLong => "too many array elements",
            Error::BadSubTrail => "garbage while expecting array comma",
            Error::SubType => "unsupported array element type",
            Error::BadString => "error while string parsing",
            Error::CheckFail => "check attribute not matched",
            Error::NoParStr => "can't support strings in parallel arrays",
            Error::BadEnum => "invalid enumerated value",
            Error::QNonString => "saw quoted value when expecting nonstring",
            Error::NonQString => "didn't see quoted value when expecting string",
            Error::Misc => "other data conversion error",
            Error::BadNum => "error while parsing a numerical argument",
            Error::NullPtr => "unexpected null value or attribute pointer",
            Error::NoCurly => "object element specified, but no {",
            Error::Empty => "input was empty or white-space only",
            Error::WriteLong => "JSON output buffer too small",
            Error::NestTooDeep => "JSON nesting limit exceeded",
            Error::NestMismatch => "JSON serializer nesting mismatch",
            Error::BadSerialize => "invalid JSON serializer call sequence",
        }
    }
}

impl core::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.message())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

#[inline]
pub fn error_string(err: Error) -> &'static str {
    err.message()
}
