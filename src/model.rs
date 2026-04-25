use crate::{Error, Result};

pub const JSON_ATTR_MAX: usize = 31;
pub const JSON_VAL_MAX: usize = 512;

#[derive(Clone, Copy)]
pub struct EnumValue<'a> {
    pub name: &'a str,
    pub value: i32,
}

#[derive(Clone, Copy)]
pub enum DefaultValue<'a> {
    None,
    Integer(i32),
    UInteger(u32),
    Short(i16),
    UShort(u16),
    Real(f64),
    Boolean(bool),
    Character(u8),
    Check(&'a str),
}

pub enum Target<'a, T> {
    One(&'a mut T),
    Many(&'a mut [T]),
}

impl<T: Copy> Target<'_, T> {
    #[inline]
    pub(crate) fn set(&mut self, offset: usize, value: T) -> Result<()> {
        match self {
            Target::One(target) if offset == 0 => {
                **target = value;
                Ok(())
            }
            Target::Many(targets) => {
                *targets.get_mut(offset).ok_or(Error::SubTooLong)? = value;
                Ok(())
            }
            Target::One(_) => Ok(()),
        }
    }
}

pub type TargetI32<'a> = Target<'a, i32>;
pub type TargetU32<'a> = Target<'a, u32>;
pub type TargetI16<'a> = Target<'a, i16>;
pub type TargetU16<'a> = Target<'a, u16>;
pub type TargetF64<'a> = Target<'a, f64>;
pub type TargetBool<'a> = Target<'a, bool>;
pub type TargetChar<'a> = Target<'a, u8>;

pub enum AttrKind<'a> {
    Integer(TargetI32<'a>),
    UInteger(TargetU32<'a>),
    Real(TargetF64<'a>),
    String(&'a mut [u8]),
    Boolean(TargetBool<'a>),
    Character(TargetChar<'a>),
    Time(TargetF64<'a>),
    Object(&'a mut [Attr<'a>]),
    Array(Array<'a>),
    Check(&'a str),
    Ignore,
    Short(TargetI16<'a>),
    UShort(TargetU16<'a>),
}

pub struct Attr<'a> {
    pub name: &'a str,
    pub kind: AttrKind<'a>,
    pub default: DefaultValue<'a>,
    pub map: Option<&'a [EnumValue<'a>]>,
    pub nodefault: bool,
}

impl<'a> Attr<'a> {
    #[inline]
    pub fn integer(name: &'a str, target: &'a mut i32) -> Self {
        Self::new(name, AttrKind::Integer(TargetI32::One(target)))
    }

    #[inline]
    pub fn integers(name: &'a str, target: &'a mut [i32]) -> Self {
        Self::new(name, AttrKind::Integer(TargetI32::Many(target)))
    }

    #[inline]
    pub fn uinteger(name: &'a str, target: &'a mut u32) -> Self {
        Self::new(name, AttrKind::UInteger(TargetU32::One(target)))
    }

    #[inline]
    pub fn uintegers(name: &'a str, target: &'a mut [u32]) -> Self {
        Self::new(name, AttrKind::UInteger(TargetU32::Many(target)))
    }

    #[inline]
    pub fn short(name: &'a str, target: &'a mut i16) -> Self {
        Self::new(name, AttrKind::Short(TargetI16::One(target)))
    }

    #[inline]
    pub fn shorts(name: &'a str, target: &'a mut [i16]) -> Self {
        Self::new(name, AttrKind::Short(TargetI16::Many(target)))
    }

    #[inline]
    pub fn ushort(name: &'a str, target: &'a mut u16) -> Self {
        Self::new(name, AttrKind::UShort(TargetU16::One(target)))
    }

    #[inline]
    pub fn ushorts(name: &'a str, target: &'a mut [u16]) -> Self {
        Self::new(name, AttrKind::UShort(TargetU16::Many(target)))
    }
}

impl<'a> Attr<'a> {
    #[inline]
    pub fn real(name: &'a str, target: &'a mut f64) -> Self {
        Self::new(name, AttrKind::Real(TargetF64::One(target)))
    }

    #[inline]
    pub fn reals(name: &'a str, target: &'a mut [f64]) -> Self {
        Self::new(name, AttrKind::Real(TargetF64::Many(target)))
    }

    #[inline]
    pub fn string(name: &'a str, target: &'a mut [u8]) -> Self {
        Self::new(name, AttrKind::String(target))
    }

    #[inline]
    pub fn boolean(name: &'a str, target: &'a mut bool) -> Self {
        Self::new(name, AttrKind::Boolean(TargetBool::One(target)))
    }

    #[inline]
    pub fn booleans(name: &'a str, target: &'a mut [bool]) -> Self {
        Self::new(name, AttrKind::Boolean(TargetBool::Many(target)))
    }

    #[inline]
    pub fn character(name: &'a str, target: &'a mut u8) -> Self {
        Self::new(name, AttrKind::Character(TargetChar::One(target)))
    }

    #[inline]
    pub fn characters(name: &'a str, target: &'a mut [u8]) -> Self {
        Self::new(name, AttrKind::Character(TargetChar::Many(target)))
    }
}

impl<'a> Attr<'a> {
    #[inline]
    pub fn time(name: &'a str, target: &'a mut f64) -> Self {
        Self::new(name, AttrKind::Time(TargetF64::One(target)))
    }

    #[inline]
    pub fn object(name: &'a str, attrs: &'a mut [Attr<'a>]) -> Self {
        Self::new(name, AttrKind::Object(attrs))
    }

    #[inline]
    pub fn array(name: &'a str, array: Array<'a>) -> Self {
        Self::new(name, AttrKind::Array(array))
    }

    #[inline]
    pub fn check(name: &'a str, expected: &'a str) -> Self {
        let mut attr = Self::new(name, AttrKind::Check(expected));
        attr.default = DefaultValue::Check(expected);
        attr
    }

    #[inline]
    pub fn ignore_any() -> Self {
        Self::new("", AttrKind::Ignore)
    }

    #[inline]
    pub fn with_default(mut self, default: DefaultValue<'a>) -> Self {
        self.default = default;
        self
    }

    #[inline]
    pub fn with_map(mut self, map: &'a [EnumValue<'a>]) -> Self {
        self.map = Some(map);
        self
    }

    #[inline]
    pub fn nodefault(mut self) -> Self {
        self.nodefault = true;
        self
    }

    #[inline]
    fn new(name: &'a str, kind: AttrKind<'a>) -> Self {
        Self {
            name,
            kind,
            default: DefaultValue::None,
            map: None,
            nodefault: false,
        }
    }
}

pub enum Array<'a> {
    Strings {
        store: &'a mut [&'a mut [u8]],
        count: Option<&'a mut usize>,
    },
    Integers {
        store: &'a mut [i32],
        count: Option<&'a mut usize>,
    },
    UIntegers {
        store: &'a mut [u32],
        count: Option<&'a mut usize>,
    },
    Shorts {
        store: &'a mut [i16],
        count: Option<&'a mut usize>,
    },
    UShorts {
        store: &'a mut [u16],
        count: Option<&'a mut usize>,
    },
    Reals {
        store: &'a mut [f64],
        count: Option<&'a mut usize>,
    },
    Booleans {
        store: &'a mut [bool],
        count: Option<&'a mut usize>,
    },
    Objects {
        attrs: &'a mut [Attr<'a>],
        maxlen: usize,
        count: Option<&'a mut usize>,
    },
    StructObjects {
        maxlen: usize,
        count: Option<&'a mut usize>,
        parser: &'a mut dyn FnMut(&str, usize) -> Result<usize>,
    },
}
