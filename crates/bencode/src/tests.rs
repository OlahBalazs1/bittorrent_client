use std::{any::Any, collections::HashMap, hash::Hash};

use num::BigInt;

use crate::{
    parse_bencode,
    token::{Token, Tokenize},
};
#[test]
fn strings() {
    let good_bencodes = [("3:abc", "abc"), ("0:", ""), ("1:e", "e")];

    for (bencode, data) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();
        let Ok(parsed) = String::try_from(parsed) else {
            panic!("Good bencode didn't parse into string: {}", bencode)
        };

        assert_eq!(parsed, data);
    }
}
#[test]
fn integers() {
    let good_bencodes: [(&str, BigInt); _] = [
        ("i3e", 3.into()),
        ("i0e", 0.into()),
        ("i-132e", (-132).into()),
    ];

    for (bencode, expected) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();

        assert_eq!(parsed, expected.tokenize());
    }
}

#[test]
fn string_list() {
    let good_bencodes: [(&str, Token); _] = [(
        "l3:abc0:1:ae",
        vec!["abc".to_string(), "".to_string(), "a".to_string()].tokenize(),
    )];

    for (bencode, expected) in good_bencodes {
        println!("testing: {bencode}");
        let parsed = parse_bencode(bencode).unwrap();

        assert_eq!(parsed, expected);
    }
}

#[test]
fn mixed_list() {
    use Token::*;
    let bencode = "ll3:abci-3ee3:abci-3ee";

    let parsed = parse_bencode(bencode).unwrap();

    let expected = List(vec![
        List(vec![String("abc".to_string()), Int(BigInt::from(-3))]),
        String("abc".to_string()),
        Int(BigInt::from(-3)),
    ]);

    assert_eq!(parsed, expected)
}

#[test]
fn mixed_dictionary() {
    use Token::*;
    // {
    // "a": -1,
    // "ab": "abc"
    // }
    let bencode = "d1:ai-1e2:ab3:abce";

    let parsed = parse_bencode(bencode).unwrap();

    let expected = Dictionary(HashMap::from([
        ("a".to_string(), Int(BigInt::from(-1))),
        ("ab".to_string(), String("abc".to_string())),
    ]));
    assert_eq!(parsed, expected);
}
