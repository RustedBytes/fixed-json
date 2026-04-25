use criterion::{Criterion, criterion_group, criterion_main};
use fixed_json::{Array, Attr, DefaultValue, EnumValue, read_array, read_object, validate_json};
use std::hint::black_box;

const SKY: &str = r#"{"class":"SKY","satellites":[{"PRN":10,"el":45,"az":196,"ss":34.0,"used":true},{"PRN":29,"el":67,"az":310,"ss":40.0,"used":true},{"PRN":28,"el":59,"az":108,"ss":42.0,"used":true},{"PRN":26,"el":51,"az":304,"ss":43.0,"used":true},{"PRN":8,"el":44,"az":58,"ss":41.0,"used":true},{"PRN":27,"el":16,"az":66,"ss":39.0,"used":true},{"PRN":21,"el":10,"az":301,"ss":0.0,"used":false}]}"#;

const NESTED: &str = r#"{"name":"wobble","value":{"inner":23,"innerobject":{"innerinner":123}}}"#;

const WATCH: &str = r#"{"class":"WATCH","enable":true,"json":true,"nmea":false,"raw":0,"scaled":false,"timing":false,"split24":false,"pps":false,"device":"/dev/ttyUSB0"}"#;

const DEVICES: &str = r#"{"devices":[{"path":"/dev/ttyUSB0","activated":1411468340.0},{"path":"/dev/ttyUSB1","activated":2.5},{"path":"/dev/ttyUSB2","activated":3.5}]}"#;

const VALIDATOR_MIXED: &[u8] = br#"[
    null,
    true,
    false,
    123,
    -45.67e+8,
    "text\nwith\u0041escapes",
    {"empty":{},"array":[1,2,3],"unicode":"\uD834\uDD1E"}
]"#;

const VALIDATOR_STRING_ESCAPES: &[u8] =
    br#""simple text with escapes: \" \\ \/ \b \f \n \r \t \u0041 \u20ac""#;

#[inline]
fn validate_json_ok(input: &[u8]) -> bool {
    validate_json(input).is_ok()
}

#[derive(Clone, Copy)]
struct DevConfig {
    path: [u8; 64],
    activated: f64,
}

impl DevConfig {
    const fn new() -> Self {
        Self {
            path: [0; 64],
            activated: 0.0,
        }
    }
}

fn bench_basic_object(c: &mut Criterion) {
    c.bench_function("object/basic primitives", |b| {
        b.iter(|| {
            let mut count = 0;
            let mut flag1 = false;
            let mut flag2 = false;
            let mut attrs = [
                Attr::integer("count", &mut count),
                Attr::boolean("flag1", &mut flag1),
                Attr::boolean("flag2", &mut flag2),
            ];

            let end = read_object(
                black_box(r#"{"count":23,"flag1":true,"flag2":false}"#),
                &mut attrs,
            )
            .unwrap();
            black_box((end, count, flag1, flag2));
        });
    });
}

fn bench_sky_parallel_object_array(c: &mut Criterion) {
    c.bench_function("object/parallel object array", |b| {
        b.iter(|| {
            let mut used = [false; 20];
            let mut prn = [0; 20];
            let mut elevation = [0; 20];
            let mut azimuth = [0; 20];
            let mut ss = [0.0; 20];
            let mut visible = 0usize;
            let mut sat_attrs = [
                Attr::integers("PRN", &mut prn),
                Attr::integers("el", &mut elevation),
                Attr::integers("az", &mut azimuth),
                Attr::reals("ss", &mut ss),
                Attr::booleans("used", &mut used),
            ];
            let mut attrs = [
                Attr::check("class", "SKY"),
                Attr::array(
                    "satellites",
                    Array::Objects {
                        attrs: &mut sat_attrs,
                        maxlen: 20,
                        count: Some(&mut visible),
                    },
                ),
            ];

            let end = read_object(black_box(SKY), &mut attrs).unwrap();
            black_box((end, visible, prn[0], elevation[6], used[6]));
        });
    });
}

fn bench_primitive_arrays(c: &mut Criterion) {
    c.bench_function("array/integers", |b| {
        b.iter(|| {
            let mut store = [0; 16];
            let mut count = 0usize;
            let mut array = Array::Integers {
                store: &mut store,
                count: Some(&mut count),
            };

            let end = read_array(black_box("[23,-17,5,0,42,99,-100,8]"), &mut array).unwrap();
            black_box((end, count, store[0], store[7]));
        });
    });

    c.bench_function("array/strings", |b| {
        b.iter(|| {
            let mut s0 = [0; 16];
            let mut s1 = [0; 16];
            let mut s2 = [0; 16];
            let mut s3 = [0; 16];
            let mut strings: [&mut [u8]; 4] = [&mut s0, &mut s1, &mut s2, &mut s3];
            let mut count = 0usize;
            let mut array = Array::Strings {
                store: &mut strings,
                count: Some(&mut count),
            };

            let end =
                read_array(black_box(r#"["foo","bar","baz","\u0042op"]"#), &mut array).unwrap();
            black_box((end, count));
        });
    });
}

fn bench_nested_object(c: &mut Criterion) {
    c.bench_function("object/nested", |b| {
        b.iter(|| {
            let mut name = [0; 32];
            let mut inner = 0;
            let mut innerinner = 0;
            let mut inner2_attrs = [Attr::integer("innerinner", &mut innerinner)];
            let mut inner1_attrs = [
                Attr::integer("inner", &mut inner),
                Attr::object("innerobject", &mut inner2_attrs),
            ];
            let mut attrs = [
                Attr::string("name", &mut name),
                Attr::object("value", &mut inner1_attrs),
            ];

            let end = read_object(black_box(NESTED), &mut attrs).unwrap();
            black_box((end, inner, innerinner, name[0]));
        });
    });
}

fn bench_check_ignore_and_defaults(c: &mut Criterion) {
    c.bench_function("object/check ignore defaults", |b| {
        b.iter(|| {
            let mut enable = false;
            let mut json = false;
            let mut mode = 99;
            let mut attrs = [
                Attr::check("class", "WATCH"),
                Attr::boolean("enable", &mut enable),
                Attr::boolean("json", &mut json),
                Attr::integer("mode", &mut mode).with_default(DefaultValue::Integer(-1)),
                Attr::ignore_any(),
            ];

            let end = read_object(black_box(WATCH), &mut attrs).unwrap();
            black_box((end, enable, json, mode));
        });
    });
}

fn bench_enum_map(c: &mut Criterion) {
    const MAP: &[EnumValue<'_>] = &[
        EnumValue {
            name: "BAR",
            value: 6,
        },
        EnumValue {
            name: "FOO",
            value: 3,
        },
        EnumValue {
            name: "BAZ",
            value: 14,
        },
    ];

    c.bench_function("object/enum map", |b| {
        b.iter(|| {
            let mut fee = 0;
            let mut fie = 0;
            let mut foe = 0;
            let mut attrs = [
                Attr::integer("fee", &mut fee).with_map(MAP),
                Attr::integer("fie", &mut fie).with_map(MAP),
                Attr::integer("foe", &mut foe).with_map(MAP),
            ];

            let end = read_object(
                black_box(r#"{"fee":"FOO","fie":"BAR","foe":"BAZ"}"#),
                &mut attrs,
            )
            .unwrap();
            black_box((end, fee, fie, foe));
        });
    });
}

fn bench_structobject_callback(c: &mut Criterion) {
    c.bench_function("object/structobject callback array", |b| {
        b.iter(|| {
            let mut devices = [DevConfig::new(); 4];
            let mut count = 0usize;
            let mut parse_device = |s: &str, index: usize| {
                let dev = &mut devices[index];
                let mut attrs = [
                    Attr::string("path", &mut dev.path),
                    Attr::real("activated", &mut dev.activated),
                ];
                read_object(s, &mut attrs)
            };
            let mut attrs = [Attr::array(
                "devices",
                Array::StructObjects {
                    maxlen: 4,
                    count: Some(&mut count),
                    parser: &mut parse_device,
                },
            )];

            let end = read_object(black_box(DEVICES), &mut attrs).unwrap();
            black_box((end, count, devices[0].path[0], devices[2].activated));
        });
    });
}

fn bench_validate_json(c: &mut Criterion) {
    c.bench_function("validate/basic object", |b| {
        b.iter(|| {
            let valid = validate_json_ok(black_box(br#"{"count":23,"flag1":true,"flag2":false}"#));
            black_box(valid);
        });
    });

    c.bench_function("validate/sky object", |b| {
        b.iter(|| {
            let valid = validate_json_ok(black_box(SKY.as_bytes()));
            black_box(valid);
        });
    });

    c.bench_function("validate/mixed array", |b| {
        b.iter(|| {
            let valid = validate_json_ok(black_box(VALIDATOR_MIXED));
            black_box(valid);
        });
    });

    c.bench_function("validate/string escapes", |b| {
        b.iter(|| {
            let valid = validate_json_ok(black_box(VALIDATOR_STRING_ESCAPES));
            black_box(valid);
        });
    });

    c.bench_function("validate/reject invalid", |b| {
        b.iter(|| {
            let result = validate_json(black_box(br#"{"bad":[1,2,]}"#));
            black_box(result.is_err());
        });
    });
}

criterion_group!(
    benches,
    bench_basic_object,
    bench_sky_parallel_object_array,
    bench_primitive_arrays,
    bench_nested_object,
    bench_check_ignore_and_defaults,
    bench_enum_map,
    bench_structobject_callback,
    bench_validate_json,
);
criterion_main!(benches);
