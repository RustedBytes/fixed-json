use fixed_json::{ObjectBuilder, error_string};

fn main() {
    let input = std::env::args().nth(1).expect("usage: example1 JSON");
    let mut flag1 = false;
    let mut flag2 = false;
    let mut count = 0;
    let status = ObjectBuilder::<3>::new(&input)
        .integer("count", &mut count)
        .boolean("flag1", &mut flag1)
        .boolean("flag2", &mut flag2)
        .read();
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
