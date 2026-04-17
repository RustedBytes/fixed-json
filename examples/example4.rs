use fixed_json::{Array, ObjectBuilder, error_string};

const ARR1_LENGTH: usize = 8;

fn main() {
    let input = std::env::args().nth(1).expect("usage: example4 JSON");
    let mut cur = 0usize;

    while cur < input.len() {
        let mut flag1 = false;
        let mut arr1 = [0; ARR1_LENGTH];
        let mut arr1_count = 0usize;
        let status = ObjectBuilder::<2>::new(&input[cur..])
            .boolean("flag1", &mut flag1)
            .array(
                "arr1",
                Array::Integers {
                    store: &mut arr1,
                    count: Some(&mut arr1_count),
                },
            )
            .read();
        println!(
            "status: {}, flag1: {}",
            status
                .map(|n| {
                    cur += n;
                    0
                })
                .unwrap_or_else(|e| e as i32),
            flag1 as u8
        );
        for value in &arr1[..arr1_count] {
            println!("arr1 = {value}");
        }
        if let Err(err) = status {
            println!("{}", error_string(err));
            break;
        }
    }
}
