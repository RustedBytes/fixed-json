use fixed_json::{Array, Attr, error_string, read_object};

const MAXCHANNELS: usize = 72;

fn main() {
    let input = std::env::args().nth(1).expect("usage: example2 JSON");
    let mut usedflags = [false; MAXCHANNELS];
    let mut prn = [0; MAXCHANNELS];
    let mut elevation = [0; MAXCHANNELS];
    let mut azimuth = [0; MAXCHANNELS];
    let mut visible = 0usize;

    let status = {
        let mut sat_attrs = [
            Attr::integers("PRN", &mut prn),
            Attr::integers("el", &mut elevation),
            Attr::integers("az", &mut azimuth),
            Attr::booleans("used", &mut usedflags),
        ];
        let mut json_attrs_sky = [
            Attr::check("class", "SKY"),
            Attr::array(
                "satellites",
                Array::Objects {
                    attrs: &mut sat_attrs,
                    maxlen: MAXCHANNELS,
                    count: Some(&mut visible),
                },
            ),
        ];
        read_object(&input, &mut json_attrs_sky)
    };

    println!("{visible} satellites:");
    for i in 0..visible {
        println!(
            "PRN = {}, elevation = {}, azimuth = {}",
            prn[i], elevation[i], azimuth[i]
        );
    }

    if let Err(err) = status {
        println!("{}", error_string(err));
    }
}
