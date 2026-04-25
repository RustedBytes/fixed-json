pub use fixed_json::{Error, Result};

mod model {
    #![allow(dead_code)]

    include!("../src/model.rs");
}

pub use model::{
    Array, Attr, AttrKind, DefaultValue, EnumValue, JSON_ATTR_MAX, JSON_VAL_MAX, Target,
    TargetBool, TargetChar, TargetF64, TargetI16, TargetI32, TargetU16, TargetU32,
};

mod number {
    #![allow(dead_code)]

    include!("../src/number.rs");
}

mod validator {
    #![allow(dead_code)]

    include!("../src/validator.rs");
}

mod parser {
    #![allow(dead_code)]

    include!("../src/parser.rs");

    pub(crate) use crate::number::*;
    pub(crate) use crate::validator::JsonValidator;
    pub(crate) use crate::validator::validate_json;

    #[allow(clippy::drop_non_drop)]
    mod tests {
        extern crate std;

        use self::std::string::String;
        use super::*;
        use crate::EnumValue;

        #[test]
        fn parses_basic_object() {
            let mut count = 0;
            let mut flag1 = false;
            let mut flag2 = true;
            let mut attrs = [
                Attr::integer("count", &mut count),
                Attr::boolean("flag1", &mut flag1),
                Attr::boolean("flag2", &mut flag2),
            ];

            let end = read_object(r#"{"flag2":false,"count":7,"flag1":true}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(end, 38);
            assert_eq!(count, 7);
            assert!(flag1);
            assert!(!flag2);
        }

        #[test]
        fn parses_object_array_into_parallel_slices() {
            let mut prn = [0; 4];
            let mut el = [0; 4];
            let mut used = [false; 4];
            let mut count = 0;
            let mut sat_attrs = [
                Attr::integers("PRN", &mut prn),
                Attr::integers("el", &mut el),
                Attr::booleans("used", &mut used),
            ];
            let mut attrs = [Attr::array(
                "satellites",
                Array::Objects {
                    attrs: &mut sat_attrs,
                    maxlen: 4,
                    count: Some(&mut count),
                },
            )];

            read_object(
            r#"{"satellites":[{"PRN":10,"el":45,"used":true},{"PRN":21,"el":10,"used":false}]}"#,
            &mut attrs,
        )
        .unwrap();
            drop(attrs);
            assert_eq!(count, 2);
            assert_eq!(prn[..2], [10, 21]);
            assert_eq!(el[..2], [45, 10]);
            assert_eq!(used[..2], [true, false]);
        }

        #[test]
        fn parses_concatenated_objects() {
            let mut flag = false;
            let input = r#"{"flag":true} {"flag":0}"#;
            let mut attrs = [Attr::boolean("flag", &mut flag)];
            let end = read_object(input, &mut attrs).unwrap();
            assert_eq!(end, 14);
            read_object(&input[end..], &mut attrs).unwrap();
            drop(attrs);
            assert!(!flag);
        }

        #[test]
        fn applies_defaults_for_omitted_fields() {
            let mut flag1 = false;
            let mut flag2 = true;
            let mut dftint = 0;
            let mut dftuint = 0;
            let mut dftshort = 0;
            let mut dftushort = 0;
            let mut dftreal = 0.0;
            let mut dftchar = 0;
            let mut attrs = [
                Attr::integer("dftint", &mut dftint).with_default(DefaultValue::Integer(-5)),
                Attr::uinteger("dftuint", &mut dftuint).with_default(DefaultValue::UInteger(10)),
                Attr::short("dftshort", &mut dftshort).with_default(DefaultValue::Short(-12)),
                Attr::ushort("dftushort", &mut dftushort).with_default(DefaultValue::UShort(12)),
                Attr::real("dftreal", &mut dftreal).with_default(DefaultValue::Real(23.17)),
                Attr::character("dftchar", &mut dftchar)
                    .with_default(DefaultValue::Character(b'X')),
                Attr::boolean("flag1", &mut flag1),
                Attr::boolean("flag2", &mut flag2),
            ];

            read_object(r#"{"flag1":true,"flag2":false}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(dftint, -5);
            assert_eq!(dftuint, 10);
            assert_eq!(dftshort, -12);
            assert_eq!(dftushort, 12);
            assert_eq!(dftreal, 23.17);
            assert_eq!(dftchar, b'X');
            assert!(flag1);
            assert!(!flag2);
        }

        #[test]
        fn nodefault_leaves_existing_value_untouched() {
            let mut count = 99;
            let mut attrs = [Attr::integer("count", &mut count)
                .with_default(DefaultValue::Integer(-1))
                .nodefault()];

            read_object(r#"{}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(count, 99);
        }

        #[test]
        fn parses_fixed_string_buffer_and_escapes() {
            let mut name = [b'!'; 16];
            let mut attrs = [Attr::string("name", &mut name)];

            read_object(r#"{"name":"A\n\u0042\/\""}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(cstr(&name), "A\nB/\"");
            assert_eq!(name[6], 0);
        }

        #[test]
        fn rejects_string_that_would_overflow_destination() {
            let mut name = [0; 4];
            let mut attrs = [Attr::string("name", &mut name)];

            let err = read_object(r#"{"name":"abcd"}"#, &mut attrs).unwrap_err();
            drop(attrs);
            assert_eq!(err, Error::StrLong);
            assert_eq!(cstr(&name), "");
        }

        #[test]
        fn parses_character_field() {
            let mut ch = 0;
            let mut attrs = [Attr::character("parity", &mut ch)];

            read_object(r#"{"parity":"N"}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(ch, b'N');
        }

        #[test]
        fn rejects_multi_byte_character_field() {
            let mut ch = 0;
            let mut attrs = [Attr::character("parity", &mut ch)];

            let err = read_object(r#"{"parity":"NO"}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::StrLong);
        }

        #[test]
        fn check_fields_accept_expected_value_and_reject_others() {
            let mut mode = 0;
            let mut attrs = [
                Attr::check("class", "TPV"),
                Attr::integer("mode", &mut mode).with_default(DefaultValue::Integer(-9)),
            ];

            read_object(r#"{"class":"TPV","mode":3}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(mode, 3);

            let mut mode = 19;
            let mut attrs = [
                Attr::check("class", "TPV"),
                Attr::integer("mode", &mut mode).with_default(DefaultValue::Integer(-9)),
            ];
            let err = read_object(r#"{"class":"foo","mode":-4}"#, &mut attrs).unwrap_err();
            drop(attrs);
            assert_eq!(err, Error::CheckFail);
            assert_eq!(mode, -9);
        }

        #[test]
        fn ignore_any_accepts_unknown_attributes() {
            let mut enable = false;
            let mut json = false;
            let mut attrs = [
                Attr::check("class", "WATCH"),
                Attr::boolean("enable", &mut enable),
                Attr::boolean("json", &mut json),
                Attr::ignore_any(),
            ];

            read_object(
            r#"{"class":"WATCH","enable":true,"json":true,"nmea":false,"raw":0,"device":"/dev/ttyUSB0"}"#,
            &mut attrs,
        )
        .unwrap();
            drop(attrs);
            assert!(enable);
            assert!(json);
        }

        #[test]
        fn maps_quoted_enum_values_to_integers() {
            const MAP: &[EnumValue<'_>] = &[
                EnumValue {
                    name: "BAR",
                    value: 6,
                },
                EnumValue {
                    name: "FOO",
                    value: 3,
                },
                EnumValue {
                    name: "BAZ",
                    value: 14,
                },
            ];
            let mut fee = 0;
            let mut fie = 0;
            let mut foe = 0;
            let mut attrs = [
                Attr::integer("fee", &mut fee).with_map(MAP),
                Attr::integer("fie", &mut fie).with_map(MAP),
                Attr::integer("foe", &mut foe).with_map(MAP),
            ];

            read_object(r#"{"fee":"FOO","fie":"BAR","foe":"BAZ"}"#, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!((fee, fie, foe), (3, 6, 14));
        }

        #[test]
        fn rejects_unknown_enum_value() {
            const MAP: &[EnumValue<'_>] = &[EnumValue {
                name: "SET",
                value: 1,
            }];
            let mut value = 0;
            let mut attrs = [Attr::integer("state", &mut value).with_map(MAP)];

            let err = read_object(r#"{"state":"NOPE"}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadEnum);
        }

        #[test]
        fn parses_integer_boolean_real_and_string_arrays() {
            let mut ints = [0; 4];
            let mut int_count = 0;
            let mut int_array = Array::Integers {
                store: &mut ints,
                count: Some(&mut int_count),
            };
            read_array("[23,-17,5]", &mut int_array).unwrap();
            drop(int_array);
            assert_eq!(int_count, 3);
            assert_eq!(ints, [23, -17, 5, 0]);

            let mut bools = [false; 4];
            let mut bool_count = 0;
            let mut bool_array = Array::Booleans {
                store: &mut bools,
                count: Some(&mut bool_count),
            };
            read_array("[true,false,7]", &mut bool_array).unwrap();
            drop(bool_array);
            assert_eq!(bool_count, 3);
            assert_eq!(bools, [true, false, true, false]);

            let mut reals = [0.0; 4];
            let mut real_count = 0;
            let mut real_array = Array::Reals {
                store: &mut reals,
                count: Some(&mut real_count),
            };
            read_array("[23.1,-17.2,5.3e1]", &mut real_array).unwrap();
            drop(real_array);
            assert_eq!(real_count, 3);
            assert_eq!(reals[0], 23.1);
            assert_eq!(reals[1], -17.2);
            assert_eq!(reals[2], 53.0);

            let mut s0 = [0; 8];
            let mut s1 = [0; 8];
            let mut s2 = [0; 8];
            let mut string_count = 0;
            {
                let mut strings: [&mut [u8]; 3] = [&mut s0, &mut s1, &mut s2];
                let mut string_array = Array::Strings {
                    store: &mut strings,
                    count: Some(&mut string_count),
                };
                read_array(r#"["foo","b\nr","\u0042az"]"#, &mut string_array).unwrap();
            }
            assert_eq!(string_count, 3);
            assert_eq!(cstr(&s0), "foo");
            assert_eq!(cstr(&s1), "b\nr");
            assert_eq!(cstr(&s2), "Baz");
        }

        #[test]
        fn parses_unsigned_and_short_arrays() {
            let mut uints = [0; 3];
            let mut shorts = [0; 3];
            let mut ushorts = [0; 3];

            let mut arr = Array::UIntegers {
                store: &mut uints,
                count: None,
            };
            read_array("[1,2,4294967295]", &mut arr).unwrap();
            drop(arr);
            assert_eq!(uints, [1, 2, u32::MAX]);

            let mut arr = Array::Shorts {
                store: &mut shorts,
                count: None,
            };
            read_array("[-32768,0,32767]", &mut arr).unwrap();
            drop(arr);
            assert_eq!(shorts, [i16::MIN, 0, i16::MAX]);

            let mut arr = Array::UShorts {
                store: &mut ushorts,
                count: None,
            };
            read_array("[0,65535,17]", &mut arr).unwrap();
            drop(arr);
            assert_eq!(ushorts, [0, u16::MAX, 17]);
        }

        #[test]
        fn parses_nested_objects() {
            let mut name = [0; 16];
            let mut inner = 0;
            let mut innerinner = 0;
            let mut inner2_attrs = [Attr::integer("innerinner", &mut innerinner)];
            let mut inner1_attrs = [
                Attr::integer("inner", &mut inner),
                Attr::object("innerobject", &mut inner2_attrs),
            ];
            let mut attrs = [
                Attr::string("name", &mut name),
                Attr::object("value", &mut inner1_attrs),
            ];

            read_object(
                r#"{"name":"wobble","value":{"inner":23,"innerobject":{"innerinner":123}}}"#,
                &mut attrs,
            )
            .unwrap();
            drop(attrs);
            assert_eq!(cstr(&name), "wobble");
            assert_eq!(inner, 23);
            assert_eq!(innerinner, 123);
        }

        #[derive(Clone, Copy)]
        struct DevConfig {
            path: [u8; 32],
            activated: f64,
        }

        impl DevConfig {
            const fn new() -> Self {
                Self {
                    path: [0; 32],
                    activated: 0.0,
                }
            }
        }

        #[test]
        fn parses_structobject_array_with_callback() {
            let mut devices = [DevConfig::new(); 4];
            let mut count = 0;
            let mut parse_device = |s: &str, index: usize| {
                let dev = &mut devices[index];
                let mut attrs = [
                    Attr::string("path", &mut dev.path),
                    Attr::real("activated", &mut dev.activated),
                ];
                read_object(s, &mut attrs)
            };
            let mut attrs = [Attr::array(
                "devices",
                Array::StructObjects {
                    maxlen: 4,
                    count: Some(&mut count),
                    parser: &mut parse_device,
                },
            )];

            read_object(
            r#"{"devices":[{"path":"/dev/ttyUSB0","activated":1411468340},{"path":"/dev/ttyUSB1","activated":2.5}]}"#,
            &mut attrs,
        )
        .unwrap();
            drop(attrs);
            assert_eq!(count, 2);
            assert_eq!(cstr(&devices[0].path), "/dev/ttyUSB0");
            assert_eq!(devices[0].activated, 1411468340.0);
            assert_eq!(cstr(&devices[1].path), "/dev/ttyUSB1");
            assert_eq!(devices[1].activated, 2.5);
        }

        #[test]
        fn empty_arrays_set_count_to_zero() {
            let mut ints = [42; 2];
            let mut count = 99;
            let mut arr = Array::Integers {
                store: &mut ints,
                count: Some(&mut count),
            };

            read_array("[]", &mut arr).unwrap();
            drop(arr);
            assert_eq!(count, 0);
            assert_eq!(ints, [42, 42]);
        }

        #[test]
        fn returned_end_skips_trailing_whitespace() {
            let mut flag = false;
            let input = "  {\"flag\":true}  next";
            let mut attrs = [Attr::boolean("flag", &mut flag)];

            let end = read_object(input, &mut attrs).unwrap();
            drop(attrs);
            assert_eq!(&input[end..], "next");
            assert!(flag);
        }

        #[test]
        fn rejects_array_overflow() {
            let mut ints = [0; 2];
            let mut arr = Array::Integers {
                store: &mut ints,
                count: None,
            };

            let err = read_array("[1,2,3]", &mut arr).unwrap_err();
            assert_eq!(err, Error::SubTooLong);
        }

        #[test]
        fn rejects_unknown_attribute_without_ignore() {
            let mut known = 0;
            let mut attrs = [Attr::integer("known", &mut known)];

            let err = read_object(r#"{"unknown":1}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadAttr);
        }

        #[test]
        fn rejects_mismatched_quotedness() {
            let mut int_value = 0;
            let mut attrs = [Attr::integer("count", &mut int_value)];
            let err = read_object(r#"{"count":"1"}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::QNonString);

            let mut text = [0; 8];
            let mut attrs = [Attr::string("name", &mut text)];
            let err = read_object(r#"{"name":1}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::NonQString);
        }

        #[test]
        fn rejects_bad_numbers_and_range_overflow() {
            let mut int_value = 0;
            let mut attrs = [Attr::integer("count", &mut int_value)];
            let err = read_object(r#"{"count":2147483648}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadNum);

            let mut uint_value = 0;
            let mut attrs = [Attr::uinteger("count", &mut uint_value)];
            let err = read_object(r#"{"count":-1}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadNum);

            let mut real_value = 0.0;
            let mut attrs = [Attr::real("value", &mut real_value)];
            let err = read_object(r#"{"value":1e}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadNum);
        }

        #[test]
        fn rejects_bad_object_and_array_syntax() {
            let mut value = 0;
            let mut attrs = [Attr::integer("value", &mut value)];
            assert_eq!(read_object("", &mut attrs).unwrap_err(), Error::Empty);
            assert_eq!(read_object("[]", &mut attrs).unwrap_err(), Error::ObStart);
            assert_eq!(
                read_object("{value:1}", &mut attrs).unwrap_err(),
                Error::AttrStart
            );
            assert_eq!(
                read_object(r#"{"value":1 junk}"#, &mut attrs).unwrap_err(),
                Error::BadTrail
            );

            let mut arr = Array::Integers {
                store: &mut [0; 2],
                count: None,
            };
            assert_eq!(read_array("{}", &mut arr).unwrap_err(), Error::ArrayStart);
            assert_eq!(
                read_array("[1 2]", &mut arr).unwrap_err(),
                Error::BadSubTrail
            );
        }

        #[test]
        fn rejects_expected_array_or_object_when_missing() {
            let mut ints = [0; 2];
            let mut attrs = [Attr::array(
                "arr",
                Array::Integers {
                    store: &mut ints,
                    count: None,
                },
            )];
            assert_eq!(
                read_object(r#"{"arr":1}"#, &mut attrs).unwrap_err(),
                Error::NoBrak
            );

            let mut value = 0;
            let mut child = [Attr::integer("value", &mut value)];
            let mut attrs = [Attr::object("child", &mut child)];
            assert_eq!(
                read_object(r#"{"child":1}"#, &mut attrs).unwrap_err(),
                Error::NoCurly
            );
        }

        #[test]
        fn rejects_unexpected_array_or_object_values() {
            let mut value = 0;
            let mut attrs = [Attr::integer("value", &mut value)];
            assert_eq!(
                read_object(r#"{"value":[1]}"#, &mut attrs).unwrap_err(),
                Error::NoArray
            );
            assert_eq!(
                read_object(r#"{"value":{"x":1}}"#, &mut attrs).unwrap_err(),
                Error::NoArray
            );
        }

        #[test]
        fn rejects_too_long_attribute_name() {
            let mut value = 0;
            let mut attrs = [Attr::integer("value", &mut value)];
            let err =
                read_object(r#"{"abcdefghijklmnopqrstuvwxyzabcdef":1}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::AttrLen);
        }

        #[test]
        fn rejects_invalid_string_escape() {
            let mut text = [0; 8];
            let mut attrs = [Attr::string("text", &mut text)];

            let err = read_object(r#"{"text":"\u12x4"}"#, &mut attrs).unwrap_err();
            assert_eq!(err, Error::BadString);
        }

        #[test]
        fn rejects_parallel_object_arrays_with_string_fields_after_first_element() {
            let mut names = [0; 16];
            let mut subattrs = [Attr::string("name", &mut names)];
            let mut attrs = [Attr::array(
                "items",
                Array::Objects {
                    attrs: &mut subattrs,
                    maxlen: 2,
                    count: None,
                },
            )];

            let err = read_object(r#"{"items":[{"name":"one"},{"name":"two"}]}"#, &mut attrs)
                .unwrap_err();
            assert_eq!(err, Error::NoParStr);
        }

        #[test]
        fn read_object_requires_colon_and_returns_exact_end_for_empty_object() {
            let mut value = 0;
            let mut attrs = [Attr::integer("value", &mut value)];

            assert_eq!(read_object("{}", &mut attrs).unwrap(), 2);
            assert_eq!(
                read_object(r#"{"value" 1}"#, &mut attrs).unwrap_err(),
                Error::BadTrail
            );
        }

        #[test]
        fn read_array_handles_empty_strings_objects_and_trailing_commas() {
            let mut s0 = [0; 4];
            let mut count = 99usize;
            {
                let mut store: [&mut [u8]; 1] = [&mut s0];
                let mut arr = Array::Strings {
                    store: &mut store,
                    count: Some(&mut count),
                };
                assert_eq!(read_array("[]", &mut arr).unwrap(), 2);
                assert_eq!(count, 0);
            }

            let mut child_value = 0;
            let mut child_attrs = [Attr::integer("value", &mut child_value)];
            let mut obj_count = 0usize;
            let mut arr = Array::Objects {
                attrs: &mut child_attrs,
                maxlen: 1,
                count: Some(&mut obj_count),
            };
            assert_eq!(read_array(r#"[{"value":7}]"#, &mut arr).unwrap(), 13);
            drop(arr);
            assert_eq!(obj_count, 1);
            assert_eq!(child_value, 7);

            let mut s1 = [0; 4];
            let mut s2 = [0; 4];
            {
                let mut store: [&mut [u8]; 2] = [&mut s1, &mut s2];
                let mut arr = Array::Strings {
                    store: &mut store,
                    count: None,
                };
                assert_eq!(
                    read_array(r#"["x",]"#, &mut arr).unwrap_err(),
                    Error::BadString
                );
            }

            let mut ints = [0; 1];
            let mut arr = Array::Integers {
                store: &mut ints,
                count: None,
            };
            assert_eq!(read_array("[", &mut arr).unwrap_err(), Error::BadNum);

            let mut ints = [0; 1];
            let mut arr = Array::Integers {
                store: &mut ints,
                count: None,
            };
            assert_eq!(read_array("[1", &mut arr).unwrap_err(), Error::BadSubTrail);
        }

        #[test]
        fn helper_parsers_cover_exact_boundaries_and_terminators() {
            let bytes = br#"name""#;
            let mut i = 0usize;
            let mut attr = [0u8; JSON_ATTR_MAX + 1];
            assert_eq!(parse_attr_name(bytes, &mut i, &mut attr).unwrap(), 4);
            assert_eq!(&attr[..4], b"name");

            let mut i = bytes.len();
            let mut attr = [0u8; JSON_ATTR_MAX + 1];
            assert_eq!(
                parse_attr_name(bytes, &mut i, &mut attr).unwrap_err(),
                Error::BadString
            );

            let mut long_attr = [0u8; JSON_ATTR_MAX + 1];
            let mut i = 0usize;
            let long_name = [b'a'; JSON_ATTR_MAX + 2];
            assert_eq!(
                parse_attr_name(&long_name, &mut i, &mut long_attr).unwrap_err(),
                Error::AttrLen
            );

            let mut string_out = [b'!'; 3];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"ab""#, &mut i, &mut string_out, 2).unwrap(),
                2
            );
            assert_eq!(&string_out, b"ab\0");

            let mut string_out = [0; 2];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"abc""#, &mut i, &mut string_out, 8).unwrap_err(),
                Error::StrLong
            );

            let mut string_out = [0; 8];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"\u0041""#, &mut i, &mut string_out, 7).unwrap(),
                1
            );
            assert_eq!(string_out[0], b'A');

            let mut string_out = [0; 8];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"\u0Z41""#, &mut i, &mut string_out, 7).unwrap_err(),
                Error::BadString
            );

            let mut string_out = [0; 2];
            let mut i = 2usize;
            assert_eq!(
                parse_string_value(br#"x""#, &mut i, &mut string_out, 1).unwrap_err(),
                Error::BadString
            );

            let mut string_out = [b'!'; 2];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"a""#, &mut i, &mut string_out, 8).unwrap(),
                1
            );
            assert_eq!(&string_out, b"a\0");

            let mut string_out = [b'!'; 1];
            let mut i = 0usize;
            assert_eq!(
                parse_string_value(br#"""#, &mut i, &mut string_out, 0).unwrap(),
                0
            );
            assert_eq!(string_out[0], 0);

            let mut token = [0u8; JSON_VAL_MAX + 1];
            let mut i = 0usize;
            assert_eq!(parse_token_value(b"true]", &mut i, &mut token).unwrap(), 4);
            assert_eq!(token_str(&token).unwrap(), "true");
            assert_eq!(i, 4);

            let mut token = [0u8; JSON_VAL_MAX + 1];
            let mut i = 4usize;
            assert_eq!(parse_token_value(b"true", &mut i, &mut token).unwrap(), 0);
            assert_eq!(token[0], 0);
        }

        #[test]
        fn selection_and_string_helpers_use_expected_matching_rules() {
            let mut matched = 0;
            let mut ignored = false;
            let attrs = [
                Attr::integer("value", &mut matched),
                Attr::ignore_any(),
                Attr::boolean("flag", &mut ignored),
            ];
            assert_eq!(find_attr(&attrs, "value"), Some(0));
            assert_eq!(find_attr(&attrs, "unknown"), Some(1));
            drop(attrs);

            let mut exact = 0;
            let mut duplicate_real = 0.0;
            let attrs = [
                Attr::integer("value", &mut exact),
                Attr::real("value", &mut duplicate_real),
            ];
            let mut val = [0u8; JSON_VAL_MAX + 1];
            val[0] = b'7';
            assert_eq!(select_attr(&attrs, 0, "value", &val, false).unwrap(), 0);
            drop(attrs);

            let mut flag = false;
            let attrs = [Attr::boolean("flag", &mut flag)];
            let mut val = [0u8; JSON_VAL_MAX + 1];
            val[..5].copy_from_slice(b"true\0");
            assert!(value_fits_attr(&attrs[0], &val, false));
            val[..6].copy_from_slice(b"false\0");
            assert!(value_fits_attr(&attrs[0], &val, false));
            drop(attrs);

            let mut unnamed = 0;
            let mut ignored = false;
            let mut hit = 0;
            let attrs = [
                Attr::integer("", &mut unnamed),
                Attr::ignore_any(),
                Attr::integer("hit", &mut hit),
                Attr::boolean("flag", &mut ignored),
            ];
            assert_eq!(find_attr(&attrs, "missing"), Some(1));
            assert_eq!(find_attr(&attrs, "hit"), Some(2));
            drop(attrs);

            let mut out = [b'X'; 4];
            let mut val = [0u8; JSON_VAL_MAX + 1];
            val[..5].copy_from_slice(b"abcd\0");
            copy_cbuf(&mut out, &val);
            assert_eq!(&out, b"abc\0");

            assert_eq!(hex_val(b'0'), Some(0));
            assert_eq!(hex_val(b'9'), Some(9));
            assert_eq!(hex_val(b'a'), Some(10));
            assert_eq!(hex_val(b'f'), Some(15));
            assert_eq!(hex_val(b'A'), Some(10));
            assert_eq!(hex_val(b'F'), Some(15));
            assert_eq!(hex_val(b'g'), None);
        }

        #[test]
        fn length_and_parallel_string_rules_are_enforced() {
            let mut buf = [0; 5];
            let attr = Attr::string("name", &mut buf);
            assert_eq!(value_max_len(&attr), 4);

            let attr = Attr::check("class", "TPV");
            assert_eq!(value_max_len(&attr), 3);

            let mut time = 0.0;
            let attr = Attr::time("ts", &mut time);
            assert_eq!(value_max_len(&attr), JSON_VAL_MAX);

            const MAP: &[EnumValue<'_>] = &[EnumValue {
                name: "ON",
                value: 1,
            }];
            let mut mapped = 0;
            let attr = Attr::integer("mode", &mut mapped).with_map(MAP);
            assert_eq!(value_max_len(&attr), JSON_VAL_MAX);

            let mut text = [0; 8];
            let mut attr = Attr::string("name", &mut text);
            let mut val = [0u8; JSON_VAL_MAX + 1];
            val[..4].copy_from_slice(b"abc\0");
            apply_value(&mut attr, Some(false), 0, &val, true).unwrap();
            assert_eq!(cstr(&text), "abc");

            let mut text = [0; 8];
            let mut attr = Attr::string("name", &mut text);
            assert_eq!(
                apply_value(&mut attr, Some(false), 1, &val, true).unwrap_err(),
                Error::NoParStr
            );

            let mut text = [b'X'; 4];
            let mut attrs = [Attr::string("name", &mut text)];
            apply_defaults(&mut attrs, Some(false), 0).unwrap();
            drop(attrs);
            assert_eq!(&text, b"\0\0\0\0");

            let mut text = [b'X'; 4];
            let mut attrs = [Attr::string("name", &mut text)];
            assert_eq!(
                apply_defaults(&mut attrs, Some(false), 1).unwrap_err(),
                Error::NoParStr
            );
        }

        #[test]
        fn validate_json_enforces_depth_limit_and_balances_nesting() {
            let mut just_ok = String::new();
            just_ok.push_str(&"[".repeat(1024));
            just_ok.push('0');
            just_ok.push_str(&"]".repeat(1024));

            let mut too_deep = String::new();
            too_deep.push_str(&"[".repeat(1025));
            too_deep.push('0');
            too_deep.push_str(&"]".repeat(1025));

            let mut sibling_nests = String::new();
            sibling_nests.push('[');
            sibling_nests.push_str(&"[".repeat(600));
            sibling_nests.push('0');
            sibling_nests.push_str(&"]".repeat(600));
            sibling_nests.push(',');
            sibling_nests.push_str(&"[".repeat(600));
            sibling_nests.push('0');
            sibling_nests.push_str(&"]".repeat(600));
            sibling_nests.push(']');

            assert_eq!(validate_json(just_ok.as_bytes()).unwrap(), just_ok.len());
            assert_eq!(
                validate_json(too_deep.as_bytes()).unwrap_err(),
                Error::SubTooLong
            );
            assert_eq!(
                validate_json(sibling_nests.as_bytes()).unwrap(),
                sibling_nests.len()
            );
        }

        #[test]
        fn validator_number_parser_rejects_invalid_forms() {
            let cases = [
                b"01".as_slice(),
                b"1.".as_slice(),
                b"1e".as_slice(),
                b"-".as_slice(),
            ];

            for bytes in cases {
                let validator = JsonValidator { bytes, depth: 0 };
                assert_eq!(validator.parse_number(0).unwrap_err(), Error::BadNum);
            }

            let validator = JsonValidator {
                bytes: b"-12.5e+3]",
                depth: 0,
            };
            assert_eq!(validator.parse_number(0).unwrap(), 8);

            let validator = JsonValidator {
                bytes: b"0",
                depth: 0,
            };
            assert_eq!(validator.parse_number(0).unwrap(), 1);

            let validator = JsonValidator {
                bytes: b"",
                depth: 0,
            };
            assert_eq!(validator.parse_number(0).unwrap_err(), Error::BadNum);
        }

        #[test]
        fn validator_skip_ws_and_parse_string_return_exact_offsets() {
            let validator = JsonValidator {
                bytes: b" \t\n\rx",
                depth: 0,
            };
            assert_eq!(validator.skip_ws(0), 4);

            let validator = JsonValidator {
                bytes: br#""hi\n\u0041\"!"x"#,
                depth: 0,
            };
            assert_eq!(validator.parse_string(0).unwrap(), 15);
        }

        #[test]
        fn repeated_attribute_names_select_matching_types() {
            let mut int_value = 0;
            let mut real_value = 0.0;
            {
                let mut flag = false;
                let mut attrs = [
                    Attr::integer("value", &mut int_value),
                    Attr::real("value", &mut real_value),
                    Attr::boolean("value", &mut flag),
                ];

                read_object(r#"{"value":1.5}"#, &mut attrs).unwrap();
                drop(attrs);
                assert!(!flag);
            }
            assert_eq!(int_value, 0);
            assert_eq!(real_value, 1.5);

            let mut flag = false;
            let mut attrs = [
                Attr::integer("value", &mut int_value),
                Attr::real("value", &mut real_value),
                Attr::boolean("value", &mut flag),
            ];
            read_object(r#"{"value":true}"#, &mut attrs).unwrap();
            drop(attrs);
            assert!(flag);
        }

        #[test]
        fn number_helpers_classify_and_bound_values() {
            assert!(json_number_is_integer(b"12"));
            assert!(json_number_is_integer(b"-0"));
            assert!(!json_number_is_integer(b"12.0"));
            assert!(json_number_is_real(b"12.0"));
            assert!(json_number_is_real(b"12e3"));
            assert!(!json_number_is_real(b"12"));
            assert_eq!(match_json_number(b""), None);
            assert_eq!(match_json_number(b"-"), None);
            assert_eq!(match_json_number(b"0"), Some(true));
            assert_eq!(match_json_number(b"01"), None);
            assert_eq!(match_json_number(b"1e"), None);
            assert_eq!(match_json_number(b"1."), None);
            assert_eq!(match_json_number(b"1e+"), None);
            assert_eq!(match_json_number(b"-12"), Some(true));
            assert_eq!(match_json_number(b"-12.5e+3"), Some(false));

            assert_eq!(number_end(b"7", 0).unwrap(), 1);
            assert_eq!(number_end(b"7.5", 0).unwrap(), 3);
            assert_eq!(number_end(b"7e2", 0).unwrap(), 3);
            assert_eq!(number_end(b"-12.5e+3]", 0).unwrap(), 8);
            assert_eq!(number_end(b"-.5", 0).unwrap(), 3);
            assert_eq!(number_end(b"-", 0).unwrap_err(), Error::BadNum);
            assert_eq!(number_end(b"1e]", 0).unwrap_err(), Error::BadNum);

            assert_eq!(parse_i16_token(b"32767").unwrap(), i16::MAX);
            assert_eq!(parse_i16_token(b"-32768").unwrap(), i16::MIN);
            assert_eq!(parse_i16_token(b"-32769").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i16_token(b"32768").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i16_at(b"-32768]", 0).unwrap(), (i16::MIN, 6));
            assert_eq!(parse_i16_at(b"32768]", 0).unwrap_err(), Error::BadNum);
            assert_eq!(parse_u16_token(b"65535").unwrap(), u16::MAX);
            assert_eq!(parse_u16_token(b"65536").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i64_token(b"-900").unwrap(), -900);
            assert_eq!(parse_i64_bytes(b"").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i64_bytes(b"-").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i64_token(b"+1").unwrap_err(), Error::BadNum);
            assert_eq!(parse_i32_bytes(b"2147483647").unwrap(), i32::MAX);
            assert_eq!(parse_i32_bytes(b"-2147483648").unwrap(), i32::MIN);
            assert_eq!(parse_i32_bytes(b"2147483648").unwrap_err(), Error::BadNum);
            assert_eq!(parse_u64_bytes(b"").unwrap_err(), Error::BadNum);
            assert_eq!(parse_u64_bytes(b"+1").unwrap_err(), Error::BadNum);
            assert_eq!(parse_u64_bytes(b"-1").unwrap_err(), Error::BadNum);
        }

        #[test]
        fn float_parser_and_pow10_handle_sign_fraction_and_exponent() {
            assert_eq!(parse_f64_bytes(b"0").unwrap(), 0.0);
            assert_eq!(parse_f64_bytes(b"-0.25").unwrap(), -0.25);
            assert_eq!(parse_f64_bytes(b"12.5e-1").unwrap(), 1.25);
            assert_eq!(parse_f64_bytes(b"0.5").unwrap(), 0.5);
            assert_eq!(parse_f64_bytes(b"01").unwrap_err(), Error::BadNum);
            assert_eq!(parse_f64_bytes(b"1e").unwrap_err(), Error::BadNum);
            assert_eq!(parse_f64_bytes(b"10x").unwrap_err(), Error::BadNum);
            assert!(parse_f64_bytes(b"1e400").unwrap().is_infinite());
            assert_eq!(pow10(3), 1000.0);
            assert_eq!(pow10(0), 1.0);
            assert_eq!(pow10(-3), 0.001);
        }

        #[test]
        fn write_i32_token_formats_negative_and_zero_values() {
            let mut out = [b'X'; JSON_VAL_MAX + 1];

            write_i32_token(0, &mut out);
            assert_eq!(token_str(&out).unwrap(), "0");

            write_i32_token(-12345, &mut out);
            assert_eq!(token_str(&out).unwrap(), "-12345");
            assert_eq!(out[6], 0);
        }
    }
}
