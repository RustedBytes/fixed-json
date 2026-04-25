#![no_main]

mod common;

use core::str;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = str::from_utf8(data) {
        common::fuzz_read_object(input);
        common::fuzz_builder(input);
    }
});
