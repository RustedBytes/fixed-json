pub use fixed_json::{Error, Result};

mod model {
    #![allow(dead_code)]

    include!("../src/model.rs");

    mod tests {
        use super::Target;

        #[test]
        fn single_target_only_accepts_offset_zero() {
            let mut value = 7;
            let mut target = Target::One(&mut value);

            target.set(0, 11).unwrap();
            target.set(1, 99).unwrap();

            assert_eq!(value, 11);
        }

        #[test]
        fn many_target_writes_by_offset_and_rejects_overflow() {
            let mut values = [0, 1, 2];
            let mut target = Target::Many(&mut values);

            target.set(2, 9).unwrap();
            let err = target.set(3, 10).unwrap_err();

            assert_eq!(values, [0, 1, 9]);
            assert_eq!(err as i32, crate::Error::SubTooLong as i32);
        }
    }
}
