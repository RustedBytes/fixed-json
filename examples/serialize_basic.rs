use fixed_json::{JsonSerializer, error_string};

fn main() {
    let mut out = [0u8; 128];
    let mut json = JsonSerializer::<2>::new(&mut out);

    let status = (|| {
        json.begin_object()?;
        json.key("count")?;
        json.i32(23)?;
        json.key("flag1")?;
        json.bool(true)?;
        json.key("flag2")?;
        json.bool(false)?;
        json.end_object()?;
        Ok::<(), fixed_json::Error>(())
    })();

    match status.and_then(|_| json.finish()) {
        Ok(output) => println!("{output}"),
        Err(err) => {
            println!("status = {}", err as i32);
            println!("{}", error_string(err));
        }
    }
}
