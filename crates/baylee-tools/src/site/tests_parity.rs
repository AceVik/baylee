//! Captured offline vectors from the original Python implementations.
//! Fictional identities/IDs only. No Python runtime is needed by these tests.
use super::{bulk, catalog, impressum};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
#[test]
fn python_v2_blob_decodes_with_the_native_keystream() {
    let blob = "QllJTQIBAgMEBQYHCAkKCwwNDg8QCDvJ_Zax2-OxVpML43p0ziYU3fmC7pdfxTQkCvVjmpWhrnmfVMcxOsAcjWV11bGmoocFxWvqAI4IohALu99JHN7DaUPT5DsmVP9YPaSMzS3ZVVvYAN5Dl7RoqHIVH-uF61losl_kRIIWjqs6exlw8REHmxpMwOHAxkmRzxOlVJhuQClA3B4T4KTrDCMOg8a_J2E8-9t2N12Ev-2fyjr6FoSa3UBv1S_EWeG-W86NaeYD7E9MRUGnCAVbdfmHa8GRpv1YsqX7bSML1rHcv9NGNIBNT5MW3sb_xprPMXhw4NFEfBsPNsaOj27tsj-Opa1Gf98z6fpAQqJm1C41FEqunMa2aZ9pquoM1HTVzrjKehXN7C5wusDycOJX4xzkgIAjliB32l8shLghmvwldqZZLggvYLSQjm0M8tDTtUazTWMFznz0pefDzRXhmwIa8GOeOavHcmczEDIyRqlQEzWZSeiaLvw2DcF4-WCKmCg9CAtFspuEdU-7jlwfo1yw9lmd3IN53z2ICKw5NfeDFFJ47JgeMXOmaUEHv2PT1isg2pZUOHvF3jEoMNagJDj0UE6cfRZ8TvYG8f8H2IkFeB3SUQfFeh2sfDVVc3cEhwhzlN1DqDtMZUL0Djuxd1efiOkKVkYLRuGHbjhk3N8InExv2NQm8bObm9LMh2uSUgsvmqEmz3gTQCgLtnOZKjfrTV51AOJKD4S8RKtEvWjL5jiKigJdnHLkcZD3AKBkSnFMNFE1jbxQrZt-oySgp95SnCrfAxjOjkPTX0HoXXIiBBMVcRrFhOB8VsY4RFbAS3QsCAhtNbJSL6vEb5zpOBKmb3MEeTWd2RtSf6hcV8erolNvKxs0Gh2sXcOEFf56u4ll1f8E-QOSG1Q5KeiLBHEtfGV347CK4WqzRdWZiD-9B8KiIzarY6uSBdWCIIagZ0BD5fExxKYlGbUR1kJRf0FLw-JiDSrF95M_VwygM6AfdyfdAo-tK6PYBv2tUYap1ZlyaWxPjXH_FHTw8DTJrh0dFkWC0i299-8OFJv-F0ruTqjpiispeg8o6PmX1c76yCJXDMMPPrPbfnYRjzJvxn3GNIj9w6i6u25QltWU3-93FkWXykQTVdE";
    let expected: BTreeMap<String,String> = serde_json::from_str(r#"{"name": "Erika Mustermann", "street": "Musterweg 12", "postcode": "12345", "city": "Musterstadt", "country_de": "Deutschland", "country_en": "Germany", "email": "erika@example.org", "phone": "+49 30 1234567"}"#).unwrap();
    assert_eq!(impressum::decode(blob).unwrap(), expected);
}
#[test]
fn python_card_parser_reference_matches_native_fields_and_reason() {
    let card=catalog::read_card("fiction.rs",r#"card!(oracle_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", scryfall_id = "11111111-1111-4111-8111-111111111111", coverage = Coverage::Partial(concat!("cast ", "replacement\n", "Äther")));"#).unwrap();
    assert_eq!(json!([card.oracle,card.printing,card.status,card.note]),serde_json::from_str::<Value>(r#"["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "11111111-1111-4111-8111-111111111111", "partial", "cast replacement Äther"]"#).unwrap());
}
#[test]
fn python_bulk_printing_reference_matches_native_selection_and_chosen_facts() {
    let body = r#"[
{"id": "11111111-1111-4111-8111-111111111111", "oracle_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "set": "zzq", "collector_number": "1", "security_stamp": "triangle", "image_uris": {}, "scryfall_uri": "https://scryfall.com/card/zzq/1/fake?utm=1", "artist": "Fictional Artist"},
{"id": "22222222-2222-4222-8222-222222222222", "oracle_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa", "set": "zzq", "collector_number": "9", "image_uris": {}, "scryfall_uri": "https://scryfall.com/card/zzq/9/fake?utm=1", "artist": "Fictional Artist"}
]
"#;
    let wanted = BTreeSet::from(["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned()]);
    let mut facts = bulk::Facts::new();
    let got = bulk::choose(Cursor::new(body), &wanted, Some(&mut facts)).unwrap();
    assert_eq!(serde_json::to_value(got).unwrap(),serde_json::from_str::<Value>(r#"{"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa": {"zzq": "22222222-2222-4222-8222-222222222222", "*": "22222222-2222-4222-8222-222222222222"}}"#).unwrap());
    let tuples: BTreeMap<_, _> = facts
        .into_iter()
        .map(|(id, f)| (id, (f.stamp, f.uri, f.artist, f.set)))
        .collect();
    assert_eq!(serde_json::to_value(tuples).unwrap(),serde_json::from_str::<Value>(r#"{"22222222-2222-4222-8222-222222222222": [null, "https://scryfall.com/card/zzq/9/fake", "Fictional Artist", "zzq"]}"#).unwrap());
}
