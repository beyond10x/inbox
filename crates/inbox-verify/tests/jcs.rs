//! The examples of RFC 8785: the canonical form of section 3.2.2, the member order of section
//! 3.2.3, and the number serializations of Appendix B.

use inbox::server::json;
use inbox_verify::jcs::{CanonError, canonicalize, double, number};

fn canonical(text: &str) -> String {
    canonicalize(&json::parse(text).expect("the example is JSON"))
        .expect("the example canonicalizes")
}

#[test]
fn the_example_of_section_3_2_2() {
    let input = r#"{
      "numbers": [333333333.33333329, 1E30, 4.50,
                  2e-3, 0.000000000000000000000000001],
      "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
      "literals": [null, true, false]
    }"#;
    assert_eq!(
        canonical(input),
        r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#
    );
}

#[test]
fn the_member_order_of_section_3_2_3_is_by_utf16_code_units() {
    let input = r#"{
      "\u20ac": "Euro Sign",
      "\r": "Carriage Return",
      "\ufb33": "Hebrew Letter Dalet With Dagesh",
      "1": "One",
      "\ud83d\ude00": "Emoji: Grinning Face",
      "\u0080": "Control",
      "\u00f6": "Latin Small Letter O With Diaeresis"
    }"#;
    let expected = concat!(
        "{\"\\r\":\"Carriage Return\",",
        "\"1\":\"One\",",
        "\"\u{80}\":\"Control\",",
        "\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",",
        "\"\u{20ac}\":\"Euro Sign\",",
        "\"\u{1f600}\":\"Emoji: Grinning Face\",",
        "\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}"
    );
    assert_eq!(canonical(input), expected);
}

#[test]
fn the_number_serializations_of_appendix_b() {
    let cases: [(u64, &str); 24] = [
        (0x0000_0000_0000_0000, "0"),
        (0x8000_0000_0000_0000, "0"),
        (0x0000_0000_0000_0001, "5e-324"),
        (0x8000_0000_0000_0001, "-5e-324"),
        (0x7fef_ffff_ffff_ffff, "1.7976931348623157e+308"),
        (0xffef_ffff_ffff_ffff, "-1.7976931348623157e+308"),
        (0x4340_0000_0000_0000, "9007199254740992"),
        (0xc340_0000_0000_0000, "-9007199254740992"),
        (0x4430_0000_0000_0000, "295147905179352830000"),
        (0x44b5_2d02_c7e1_4af5, "9.999999999999997e+22"),
        (0x44b5_2d02_c7e1_4af6, "1e+23"),
        (0x44b5_2d02_c7e1_4af7, "1.0000000000000001e+23"),
        (0x444b_1ae4_d6e2_ef4e, "999999999999999700000"),
        (0x444b_1ae4_d6e2_ef4f, "999999999999999900000"),
        (0x444b_1ae4_d6e2_ef50, "1e+21"),
        (0x3eb0_c6f7_a0b5_ed8c, "9.999999999999997e-7"),
        (0x3eb0_c6f7_a0b5_ed8d, "0.000001"),
        (0x41b3_de43_5555_5553, "333333333.3333332"),
        (0x41b3_de43_5555_5554, "333333333.33333325"),
        (0x41b3_de43_5555_5555, "333333333.3333333"),
        (0x41b3_de43_5555_5556, "333333333.3333334"),
        (0x41b3_de43_5555_5557, "333333333.33333343"),
        (0xbecb_f647_612f_3696, "-0.0000033333333333333333"),
        (0x4314_3ff3_c1cb_0959, "1424953923781206.2"),
    ];
    for (bits, expected) in cases {
        assert_eq!(
            double(f64::from_bits(bits)).as_deref(),
            Some(expected),
            "{bits:016x}"
        );
        assert_eq!(
            number(expected).as_deref(),
            Ok(expected),
            "{expected} again"
        );
    }
}

#[test]
fn nan_and_the_infinities_have_no_canonical_form() {
    assert_eq!(double(f64::from_bits(0x7fff_ffff_ffff_ffff)), None);
    assert_eq!(double(f64::INFINITY), None);
    assert_eq!(double(f64::NEG_INFINITY), None);
    assert!(matches!(number("1e400"), Err(CanonError::Number(_))));
}

#[test]
fn a_spelling_that_is_not_a_json_number_is_refused() {
    for spelling in [
        "", "-", "01", "1.", ".5", "1e", "+1", "0x10", "NaN", "1_000",
    ] {
        assert!(number(spelling).is_err(), "{spelling:?} read");
    }
}

#[test]
fn a_duplicate_member_is_refused() {
    let value = json::parse(r#"{"a":1,"a":2}"#).expect("the reader keeps duplicates");
    assert_eq!(
        canonicalize(&value),
        Err(CanonError::DuplicateMember("a".to_owned()))
    );
}

#[test]
fn control_characters_are_escaped_as_ecmascript_escapes_them() {
    let value = json::Value::Text("\u{8}\u{c}\u{1}\u{1f}\u{7f}\u{2028}".to_owned());
    assert_eq!(
        canonicalize(&value).unwrap(),
        "\"\\b\\f\\u0001\\u001f\u{7f}\u{2028}\""
    );
}
