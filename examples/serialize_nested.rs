use fixed_json::{JsonSerializer, error_string};

struct Satellite {
    prn: i32,
    elevation: i32,
    azimuth: i32,
    used: bool,
}

fn main() {
    let satellites = [
        Satellite {
            prn: 10,
            elevation: 45,
            azimuth: 196,
            used: true,
        },
        Satellite {
            prn: 32,
            elevation: 16,
            azimuth: 42,
            used: false,
        },
    ];

    let mut out = [0u8; 512];
    let mut json = JsonSerializer::<4>::new(&mut out);

    let status = (|| {
        json.begin_object()?;
        json.key("class")?;
        json.string("SKY")?;
        json.key("satellites")?;
        json.begin_array()?;

        for sat in satellites {
            json.begin_object()?;
            json.key("PRN")?;
            json.i32(sat.prn)?;
            json.key("el")?;
            json.i32(sat.elevation)?;
            json.key("az")?;
            json.i32(sat.azimuth)?;
            json.key("used")?;
            json.bool(sat.used)?;
            json.end_object()?;
        }

        json.end_array()?;
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
