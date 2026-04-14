# fixed-json

This directory contains `fixed-json`, a Rust implementation of the `microjson` library, a
fixed-storage JSON parser, made by Eric S. Raymond in C.

The library is designed for constrained environments:

- `#![no_std]`
- no heap allocation in the library
- caller-owned output storage
- fixed string buffers and fixed arrays
- descriptor-based parsing, matching the C library model

The examples and benchmarks use `std` for command-line I/O and Criterion, but
the library itself does not require `std`.

## Supported API

The primary entry points are:

```rust
fixed_json::read_object(input, attrs)
fixed_json::read_array(input, array)
fixed_json::validate_json(bytes)
fixed_json::error_string(error)
```

`read_object` and `read_array` parse into caller-provided descriptors and
storage. `validate_json` is a no-allocation JSON syntax validator used by the
JSONTestSuite integration tests.

## Basic Usage

```rust
use fixed_json::{Attr, read_object};

let mut count = 0;
let mut flag1 = false;
let mut flag2 = false;

let mut attrs = [
    Attr::integer("count", &mut count),
    Attr::boolean("flag1", &mut flag1),
    Attr::boolean("flag2", &mut flag2),
];

let end = read_object(
    r#"{"count":23,"flag1":true,"flag2":false}"#,
    &mut attrs,
)?;

drop(attrs);

assert_eq!(end, 39);
assert_eq!(count, 23);
assert!(flag1);
assert!(!flag2);
# Ok::<(), fixed_json::Error>(())
```

Descriptors hold mutable references into the destination storage. Drop the
descriptor array before reading the parsed values if the borrow checker requires
it.

## Fixed String Buffers

Strings are written into caller-provided byte buffers and nul-terminated when
space allows:

```rust
use fixed_json::{Attr, cstr, read_object};

let mut name = [0u8; 32];
let mut attrs = [Attr::string("name", &mut name)];

read_object(r#"{"name":"gpsd"}"#, &mut attrs)?;
drop(attrs);

assert_eq!(cstr(&name), "gpsd");
# Ok::<(), fixed_json::Error>(())
```

If the JSON string does not fit, parsing returns `Error::StrLong`.

## Arrays

Arrays are homogeneous and use caller-provided fixed slices:

```rust
use fixed_json::{Array, read_array};

let mut values = [0; 8];
let mut count = 0usize;

let mut array = Array::Integers {
    store: &mut values,
    count: Some(&mut count),
};

read_array("[10,20,30]", &mut array)?;
drop(array);

assert_eq!(count, 3);
assert_eq!(&values[..count], &[10, 20, 30]);
# Ok::<(), fixed_json::Error>(())
```

Supported array element types include integers, unsigned integers, shorts,
unsigned shorts, reals, booleans, strings, object arrays, and struct-object
arrays through a callback.

## Examples

The C examples have Rust equivalents:

```sh
cargo run --example example1 -- '{"count":23,"flag1":true,"flag2":false}'
cargo run --example example2 -- '{"class":"SKY","satellites":[{"PRN":10,"el":45,"az":196,"used":true}]}'
cargo run --example example3 -- '{"class":"DEVICES","devices":[{"path":"/dev/ttyUSB0","activated":1411468340}]}'
cargo run --example example4 -- '{"flag1":true} {"flag1":0,"arr1":[10,20]}'
```

## Tests

Run all tests:

```sh
cargo test
```

Run only the JSONTestSuite integration tests:

```sh
cargo test --test json_test_suite
```

The JSONTestSuite tests are generated at build time from
`JSONTestSuite/test_parsing/*.json`, with one Rust test case per JSON file.

Expectation mapping:

- `y_*.json`: must be accepted by `validate_json`
- `n_*.json`: must be rejected by `validate_json`
- `i_*.json`: implementation-defined, exercised but not forced either way

The descriptor parser intentionally remains stricter than a general JSON DOM
parser. For example, application parsing still requires known shapes and fixed
storage.

## Benchmarks

Criterion benchmarks are available for common parser paths:

```sh
cargo bench --bench parser
```

To compile the benchmark without running it:

```sh
cargo bench --bench parser --no-run
```

## no_std Check

Verify the library without default features:

```sh
cargo check --lib --no-default-features
```

## Limitations

This crate follows the spirit of the original C microjson library:

- parsing into application data is descriptor-based
- output storage must be supplied by the caller
- arrays are fixed-capacity
- application-level arrays are homogeneous
- no heap allocation is used by the library

Use `validate_json` when you need syntax validation of arbitrary JSON. Use
`read_object` and `read_array` when you need to unpack known JSON shapes into
fixed storage.
