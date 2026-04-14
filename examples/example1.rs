use fixed_json::{Attr, error_string, read_object};

fn main() {
    let input = std::env::args().nth(1).expect("usage: example1 JSON");
    let mut flag1 = false;
    let mut flag2 = false;
    let mut count = 0;
    let status = {
        let mut attrs = [
            Attr::integer("count", &mut count),
            Attr::boolean("flag1", &mut flag1),
            Attr::boolean("flag2", &mut flag2),
        ];
        read_object(&input, &mut attrs)
    };
    println!(
        "status = {}, count = {}, flag1 = {}, flag2 = {}",
        status.map(|_| 0).unwrap_or_else(|e| e as i32),
        count,
        flag1 as u8,
        flag2 as u8
    );
    if let Err(err) = status {
        println!("{}", error_string(err));
    }
}
