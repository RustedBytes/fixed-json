use fixed_json::{Attr, DefaultValue, ObjectBuilder};

#[test]
fn reads_basic_object_with_builder() {
    let mut flag1 = false;
    let mut flag2 = false;
    let mut count = 0;

    let end = ObjectBuilder::<3>::new(r#"{"flag2":false,"count":7,"flag1":true}"#)
        .integer("count", &mut count)
        .boolean("flag1", &mut flag1)
        .boolean("flag2", &mut flag2)
        .read()
        .unwrap();

    assert_eq!(end, 38);
    assert_eq!(count, 7);
    assert!(flag1);
    assert!(!flag2);
}

#[test]
fn accepts_preconfigured_attrs() {
    let mut count = 1;

    ObjectBuilder::<1>::new(r#"{}"#)
        .attr(Attr::integer("count", &mut count).with_default(DefaultValue::Integer(5)))
        .read()
        .unwrap();

    assert_eq!(count, 5);
}
