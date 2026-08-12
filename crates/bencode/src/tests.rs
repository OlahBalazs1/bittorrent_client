use std::{any::Any, collections::HashMap, hash::Hash};

use num::BigInt;

use crate::parse_bencode;
#[test]
fn strings() {
    let good_bencodes = [("3:abc", "abc"), ("0:", ""), ("1:e", "e")];

    for (bencode, data) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();
        let Ok(parsed) = parsed.downcast::<String>() else {
            panic!("Good bencode didn't parse into string: {}", bencode)
        };

        assert_eq!(parsed.as_ref(), data);
    }
}
#[test]
fn integers() {
    let good_bencodes: &[(&str, BigInt)] = &[
        ("i3e", 3.into()),
        ("i0e", 0.into()),
        ("i-132e", (-132).into()),
    ];

    for (bencode, data) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();
        let Ok(parsed) = parsed.downcast::<BigInt>() else {
            panic!("Good bencode didn't parse into string: {}", bencode)
        };

        assert_eq!(parsed.as_ref(), data);
    }
}

#[test]
fn string_list() {
    let good_bencodes: &[(&str, Vec<String>)] =
        &[("l3:abc0:1:ae", vec!["abc".into(), "".into(), "a".into()])];

    for (bencode, data) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();
        let Ok(parsed) = parsed.downcast::<Vec<Box<dyn Any>>>() else {
            panic!("Good bencode didn't parse into list: {}", bencode)
        };

        let cast = parsed
            .into_iter()
            .filter_map(|e| Some(String::clone(e.downcast_ref::<String>()?)))
            .collect::<Vec<String>>();

        assert_eq!(cast, *data);
    }
}

#[test]
fn mixed_list() {
    let bencode = "ll3:abci-3ee3:abci-3ee";

    let elems = (
        ("abc".to_string(), BigInt::from(-3)),
        "abc".to_string(),
        BigInt::from(-3),
    );

    let parsed = parse_bencode(bencode).unwrap();

    let parsed = parsed.downcast::<Vec<Box<dyn Any>>>().unwrap();

    let elem1 = parsed[0].downcast_ref::<Vec<Box<dyn Any>>>().unwrap();

    assert_eq!(elem1[0].downcast_ref::<String>().unwrap(), &elems.0.0);
    assert_eq!(elem1[1].downcast_ref::<BigInt>().unwrap(), &elems.0.1);

    assert_eq!(parsed[1].downcast_ref::<String>().unwrap(), &elems.1);
    assert_eq!(parsed[2].downcast_ref::<BigInt>().unwrap(), &elems.2);
}

#[test]
fn mixed_dictionary() {
    // {
    // "a": -1,
    // "ab": "abc"
    // }
    let bencode = "d1:ai-1e2:ab3:abce";

    let parsed = parse_bencode(bencode)
        .unwrap()
        .downcast::<HashMap<String, Box<dyn Any>>>()
        .unwrap();

    assert_eq!(
        parsed
            .get(&"a".to_string())
            .unwrap()
            .downcast_ref::<BigInt>()
            .unwrap(),
        &BigInt::from(-1)
    );
    assert_eq!(
        parsed
            .get(&"ab".to_string())
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "abc"
    );
}
