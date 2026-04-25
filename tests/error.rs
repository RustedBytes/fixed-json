extern crate std;

use self::std::string::ToString;
use fixed_json::{Error, error_string};

#[test]
fn error_helpers_return_human_readable_messages() {
    assert_eq!(
        Error::BadNum.message(),
        "error while parsing a numerical argument"
    );
    assert_eq!(
        error_string(Error::NoCurly),
        "object element specified, but no {"
    );
    assert_eq!(Error::BadAttr.to_string(), "unknown attribute name");
}
