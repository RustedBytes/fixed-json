#![no_main]

mod common;

use core::str;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some((&selector, rest)) = data.split_first() {
        if let Ok(input) = str::from_utf8(rest) {
            common::fuzz_read_array(input, selector);
        }
    }
});
