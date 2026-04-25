#![allow(dead_code)]

use fixed_json::{Array, Attr, ObjectBuilder, read_array, read_object, validate_json};

pub fn fuzz_validate_only(data: &[u8]) {
    let _ = validate_json(data);
}

pub fn fuzz_read_object(input: &str) {
    let validation = validate_json(input.as_bytes());

    let mut id = 0i32;
    let mut enabled = false;
    let mut ratio = 0.0f64;
    let mut name = [0u8; 32];
    let mut child = 0i32;
    let mut values = [0i32; 4];
    let mut value_count = 0usize;

    let mut child_attrs = [Attr::integer("child", &mut child)];
    let mut attrs = [
        Attr::integer("id", &mut id),
        Attr::boolean("enabled", &mut enabled),
        Attr::real("ratio", &mut ratio),
        Attr::string("name", &mut name),
        Attr::object("meta", &mut child_attrs),
        Attr::array(
            "values",
            Array::Integers {
                store: &mut values,
                count: Some(&mut value_count),
            },
        ),
        Attr::ignore_any(),
    ];

    if let Ok(end) = read_object(input, &mut attrs) {
        assert!(end <= input.len());
        assert_eq!(validate_json(input[..end].as_bytes()), Ok(end));
        if end == input.len() {
            assert!(validation.is_ok());
        }
    }
}

pub fn fuzz_read_array(input: &str, selector: u8) {
    let validation = validate_json(input.as_bytes());

    let result = match selector % 5 {
        0 => {
            let mut values = [0i32; 8];
            let mut count = 0usize;
            let mut array = Array::Integers {
                store: &mut values,
                count: Some(&mut count),
            };
            read_array(input, &mut array)
        }
        1 => {
            let mut values = [false; 8];
            let mut count = 0usize;
            let mut array = Array::Booleans {
                store: &mut values,
                count: Some(&mut count),
            };
            read_array(input, &mut array)
        }
        2 => {
            let mut first = [0u8; 16];
            let mut second = [0u8; 16];
            let mut third = [0u8; 16];
            let mut store: [&mut [u8]; 3] = [&mut first, &mut second, &mut third];
            let mut count = 0usize;
            let mut array = Array::Strings {
                store: &mut store,
                count: Some(&mut count),
            };
            read_array(input, &mut array)
        }
        3 => {
            let mut value = 0i32;
            let mut flag = false;
            let mut attrs = [
                Attr::integer("value", &mut value),
                Attr::boolean("flag", &mut flag),
            ];
            let mut count = 0usize;
            let mut array = Array::Objects {
                attrs: &mut attrs,
                maxlen: 2,
                count: Some(&mut count),
            };
            read_array(input, &mut array)
        }
        _ => {
            let mut ids = [0i32; 4];
            let maxlen = ids.len();
            let mut count = 0usize;
            let mut parse_item = |s: &str, index: usize| {
                let mut attrs = [Attr::integer("id", &mut ids[index])];
                read_object(s, &mut attrs)
            };
            let mut array = Array::StructObjects {
                maxlen,
                count: Some(&mut count),
                parser: &mut parse_item,
            };
            read_array(input, &mut array)
        }
    };

    if let Ok(end) = result {
        assert!(end <= input.len());
        assert_eq!(validate_json(input[..end].as_bytes()), Ok(end));
        if end == input.len() {
            assert!(validation.is_ok());
        }
    }
}

pub fn fuzz_builder(input: &str) {
    let validation = validate_json(input.as_bytes());

    let mut count = 0i32;
    let mut enabled = false;
    let mut label = [0u8; 24];

    if let Ok(end) = ObjectBuilder::<4>::new(input)
        .integer("count", &mut count)
        .boolean("enabled", &mut enabled)
        .string("label", &mut label)
        .ignore_any()
        .read()
    {
        assert!(end <= input.len());
        assert_eq!(validate_json(input[..end].as_bytes()), Ok(end));
        if end == input.len() {
            assert!(validation.is_ok());
        }
    }
}
