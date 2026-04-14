use fixed_json::validate_json;

#[derive(Clone, Copy)]
enum Expect {
    Accept,
    Reject,
    ImplementationDefined,
}

fn run_case(name: &str, bytes: &[u8], expect: Expect) {
    let result = validate_json(bytes);
    match expect {
        Expect::Accept => {
            assert!(result.is_ok(), "{name} should be accepted, got {result:?}");
        }
        Expect::Reject => {
            assert!(result.is_err(), "{name} should be rejected");
        }
        Expect::ImplementationDefined => {
            let _ = result;
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/json_test_suite_cases.rs"));
