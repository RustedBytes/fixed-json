use std::fs;
use std::io::{self, Write};
use std::path::Path;

fn main() -> io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let suite_dir = Path::new(&manifest_dir).join("JSONTestSuite/test_parsing");
    let out_path = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR is set"))
        .join("json_test_suite_cases.rs");

    println!("cargo:rerun-if-changed={}", suite_dir.display());

    let mut files = Vec::new();
    if suite_dir.exists() {
        for entry in fs::read_dir(&suite_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                files.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    files.sort();

    let mut out = fs::File::create(out_path)?;
    for (index, file) in files.into_iter().enumerate() {
        println!("cargo:rerun-if-changed={}", suite_dir.join(&file).display());
        let test_name = test_name(index, &file);
        let expect = match file.as_bytes().first().copied() {
            Some(b'y') => "Accept",
            Some(b'n') => "Reject",
            Some(b'i') => "ImplementationDefined",
            _ => "ImplementationDefined",
        };
        writeln!(
            out,
            "#[test]\nfn {test_name}() {{ run_case({file:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/JSONTestSuite/test_parsing/{file}\")), Expect::{expect}); }}\n"
        )?;
    }

    Ok(())
}

fn test_name(index: usize, file: &str) -> String {
    let stem = file.strip_suffix(".json").unwrap_or(file);
    let mut name = format!("json_suite_{index:03}_");
    let mut last_was_underscore = false;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            name.push('_');
            last_was_underscore = true;
        }
    }
    while name.ends_with('_') {
        name.pop();
    }
    if name.ends_with(|ch: char| ch.is_ascii_digit()) {
        name.push_str("_case");
    }
    if name.is_empty() {
        format!("json_suite_{index:03}_case")
    } else {
        name
    }
}
