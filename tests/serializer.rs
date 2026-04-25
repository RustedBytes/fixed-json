use fixed_json::{Error, JsonSerializer, validate_json};

#[test]
fn serializes_nested_object_without_allocating() {
    let mut out = [0u8; 128];
    let mut json = JsonSerializer::<4>::new(&mut out);

    json.begin_object().unwrap();
    json.key("name").unwrap();
    json.string("gpsd").unwrap();
    json.key("active").unwrap();
    json.bool(true).unwrap();
    json.key("values").unwrap();
    json.begin_array().unwrap();
    json.i32(-7).unwrap();
    json.u32(42).unwrap();
    json.null().unwrap();
    json.end_array().unwrap();
    json.end_object().unwrap();

    let output = json.finish().unwrap();
    assert_eq!(
        output,
        r#"{"name":"gpsd","active":true,"values":[-7,42,null]}"#
    );
    validate_json(output.as_bytes()).unwrap();
}

#[test]
fn escapes_json_strings() {
    let mut out = [0u8; 128];
    let mut json = JsonSerializer::<1>::new(&mut out);

    json.string("quote\" slash\\ newline\n tab\t control\u{01} pi\u{03c0}")
        .unwrap();

    assert_eq!(
        json.finish().unwrap(),
        r#""quote\" slash\\ newline\n tab\t control\u0001 piπ""#
    );
}

#[test]
fn reports_output_overflow() {
    let mut out = [0u8; 4];
    let mut json = JsonSerializer::<1>::new(&mut out);

    assert_eq!(json.string("abcd").unwrap_err(), Error::WriteLong);
}

#[test]
fn enforces_serializer_call_sequence() {
    let mut out = [0u8; 64];
    let mut json = JsonSerializer::<2>::new(&mut out);

    assert_eq!(json.finish().unwrap_err(), Error::BadSerialize);
    json.begin_object().unwrap();
    assert_eq!(json.string("value").unwrap_err(), Error::BadSerialize);
    json.key("value").unwrap();
    assert_eq!(json.key("again").unwrap_err(), Error::BadSerialize);
    json.i32(1).unwrap();
    json.end_object().unwrap();
    assert_eq!(json.bool(false).unwrap_err(), Error::BadSerialize);
}

#[test]
fn enforces_nesting_limit_and_finish_state() {
    let mut out = [0u8; 64];
    let mut json = JsonSerializer::<1>::new(&mut out);

    json.begin_array().unwrap();
    assert_eq!(json.begin_array().unwrap_err(), Error::NestTooDeep);
    assert_eq!(json.finish().unwrap_err(), Error::NestMismatch);
    json.end_array().unwrap();
    assert_eq!(json.finish().unwrap(), "[]");
}

#[test]
fn rejects_invalid_numbers() {
    let mut out = [0u8; 64];
    let mut json = JsonSerializer::<1>::new(&mut out);

    assert_eq!(json.raw_number("01").unwrap_err(), Error::BadNum);
    assert_eq!(json.f64(f64::NAN).unwrap_err(), Error::BadNum);
}
