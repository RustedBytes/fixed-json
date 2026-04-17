use fixed_json::{Array, ObjectBuilder, cstr, error_string};

const MAXUSERDEVS: usize = 4;
const PATH_MAX: usize = 4096;

#[derive(Clone, Copy)]
struct DevConfig {
    path: [u8; PATH_MAX],
    activated: f64,
}

impl DevConfig {
    const fn new() -> Self {
        Self {
            path: [0; PATH_MAX],
            activated: 0.0,
        }
    }
}

struct DevList {
    ndevices: usize,
    list: [DevConfig; MAXUSERDEVS],
}

fn main() {
    let input = std::env::args().nth(1).expect("usage: example3 JSON");
    let mut devicelist = DevList {
        ndevices: 0,
        list: [DevConfig::new(); MAXUSERDEVS],
    };

    let mut parse_device = |s: &str, index: usize| {
        let dev = &mut devicelist.list[index];
        ObjectBuilder::<2>::new(s)
            .string("path", &mut dev.path)
            .real("activated", &mut dev.activated)
            .read()
    };

    let status = ObjectBuilder::<2>::new(&input)
        .check("class", "DEVICES")
        .array(
            "devices",
            Array::StructObjects {
                maxlen: MAXUSERDEVS,
                count: Some(&mut devicelist.ndevices),
                parser: &mut parse_device,
            },
        )
        .read();

    println!("{} devices:", devicelist.ndevices);
    for dev in &devicelist.list[..devicelist.ndevices] {
        println!("{} @ {}", cstr(&dev.path), dev.activated);
    }
    if let Err(err) = status {
        println!("{}", error_string(err));
    }
}
