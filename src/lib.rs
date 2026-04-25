#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod builder;
mod error;
mod model;
mod number;
mod parser;
mod validator;

pub use builder::ObjectBuilder;
pub use error::{Error, Result, error_string};
pub use model::{
    Array, Attr, AttrKind, DefaultValue, EnumValue, JSON_ATTR_MAX, JSON_VAL_MAX, Target,
    TargetBool, TargetChar, TargetF64, TargetI16, TargetI32, TargetU16, TargetU32,
};
pub use parser::{cstr, read_array, read_object};
pub use validator::validate_json;
