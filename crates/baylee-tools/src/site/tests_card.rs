//! All original card-status regression cases, plus source-parser and catalog guards.
use super::{bulk::*, card_status, catalog, checks, repo};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Write};
const IMG: &str = "11111111-1111-4111-8111-111111111111";
const IMG2: &str = "22222222-2222-4222-8222-222222222222";
const IMG3: &str = "33333333-3333-4333-8333-333333333333";
const IMG7: &str = "77777777-7777-4777-8777-777777777777";
const OID1: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const OID2: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const OID3: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const OTHER: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
fn facts(stamp: Option<&str>, set: Option<&str>) -> Facts {
    BTreeMap::from([(
        IMG.into(),
        Fact {
            stamp: stamp.map(str::to_owned),
            set: set.map(str::to_owned),
            ..Fact::default()
        },
    )])
}
fn payload(id: &str, oid: &str, set: &str, number: &str) -> Value {
    json!({"id":id,"oracle_id":oid,"set":set,"collector_number":number,"lang":"en",
        "image_uris":{"normal":"https://example.invalid/x.jpg"},"artist":format!("Artist {number}"),
        "scryfall_uri":format!("https://scryfall.com/card/{set}/{number}/made-up?utm_source=api")})
}
fn extra(mut c: Value, key: &str, value: Value) -> Value {
    c[key] = value;
    c
}
fn absent(mut c: Value, key: &str) -> Value {
    c.as_object_mut().unwrap().remove(key);
    c
}
fn jsonl(cards: &[Value]) -> String {
    let mut body = String::new();
    for c in cards {
        writeln!(body, "{c}").unwrap();
    }
    body
}
fn picks(cards: &[Value], wanted: &[&str], facts: Option<&mut Facts>) -> Printings {
    let body = jsonl(cards);
    choose(
        Cursor::new(body),
        &wanted.iter().map(|o| (*o).into()).collect(),
        facts,
    )
    .unwrap()
}
fn write(dir: &std::path::Path, name: &str, c: &Value) {
    fs::write(dir.join(name), c.to_string()).unwrap();
}
mod picturedtests {
    use super::*;
    #[test]
    fn test_every_status_in_the_pictured_list_keeps_its_picture() {
        assert_eq!(
            PICTURED,
            [
                "implemented",
                "generated",
                "partial",
                "unimplemented",
                "never"
            ]
        );
        for status in PICTURED {
            assert_eq!(
                pictured(status, "zzq", Some(IMG), &facts(None, Some("zzq"))),
                Some(IMG)
            );
        }
    }
    #[test]
    fn test_an_absent_card_never_has_a_picture() {
        for f in [Facts::new(), facts(None, Some("zzq"))] {
            assert_eq!(pictured("absent", "zzq", Some(IMG), &f), None);
        }
    }
    #[test]
    fn test_an_unknown_status_has_no_picture() {
        for s in ["banned", ""] {
            assert_eq!(pictured(s, "zzq", Some(IMG), &Facts::new()), None);
        }
    }
    #[test]
    fn test_no_printing_id_gives_no_picture() {
        for s in PICTURED {
            for img in [None, Some("")] {
                assert_eq!(pictured(s, "zzq", img, &Facts::new()), None);
            }
        }
    }
    #[test]
    fn test_a_triangle_stamp_drops_the_picture() {
        assert_eq!(
            pictured(
                "implemented",
                "zzq",
                Some(IMG),
                &facts(Some("triangle"), Some("zzq"))
            ),
            None
        );
    }
    #[test]
    fn test_another_stamp_keeps_the_picture() {
        for stamp in [Some("oval"), Some("acorn"), Some("arena"), Some(""), None] {
            assert_eq!(
                pictured("implemented", "zzq", Some(IMG), &facts(stamp, Some("zzq"))),
                Some(IMG)
            );
        }
    }
    #[test]
    fn test_a_printing_in_a_universes_beyond_set_drops_the_picture() {
        assert_eq!(
            pictured("implemented", "zzq", Some(IMG), &facts(None, Some("ltr"))),
            None
        );
    }
    #[test]
    fn test_every_universes_beyond_set_drops_a_listed_card() {
        assert!(!UB_SETS.is_empty());
        for code in UB_SETS {
            assert_eq!(
                pictured("implemented", code, Some(IMG), &Facts::new()),
                None
            );
        }
    }
    #[test]
    fn test_a_card_listed_in_a_universes_beyond_set_drops_a_printing_from_an_ordinary_one() {
        for s in ["implemented", "never"] {
            assert_eq!(
                pictured(s, "ltr", Some(IMG), &facts(None, Some("zzq"))),
                None
            );
        }
    }
    #[test]
    fn test_without_facts_the_listed_set_decides() {
        assert_eq!(
            pictured("partial", "zzq", Some(IMG), &Facts::new()),
            Some(IMG)
        );
        let f = BTreeMap::from([(
            IMG2.into(),
            Fact {
                stamp: Some("triangle".into()),
                set: Some("ltr".into()),
                ..Fact::default()
            },
        )]);
        assert_eq!(pictured("partial", "zzq", Some(IMG), &f), Some(IMG));
    }
    #[test]
    fn test_a_facts_entry_without_a_set_falls_back_to_the_listed_set() {
        let f = facts(None, None);
        assert_eq!(pictured("implemented", "zzq", Some(IMG), &f), Some(IMG));
        assert_eq!(pictured("implemented", "ltr", Some(IMG), &f), None);
    }
    #[test]
    fn test_the_universes_beyond_list_is_lowercase_codes() {
        for code in UB_SETS {
            assert!(valid_code(code));
        }
    }
}
mod facttests {
    use super::*;
    #[test]
    fn test_the_query_is_stripped_from_the_scryfall_uri() {
        assert_eq!(fact(&json!({"scryfall_uri":"https://scryfall.com/card/zzq/12/made-up?utm_source=api","set":"zzq"})).uri.as_deref(),Some("https://scryfall.com/card/zzq/12/made-up"));
    }
    #[test]
    fn test_an_address_elsewhere_is_dropped() {
        for uri in [
            json!("https://example.com/card/zzq/1"),
            json!("http://scryfall.com/card/zzq/1"),
            json!("https://scryfall.com/sets/zzq"),
            json!(""),
            Value::Null,
        ] {
            assert_eq!(fact(&json!({"scryfall_uri":uri})).uri, None);
        }
    }
    #[test]
    fn test_the_artist_comes_from_the_card() {
        assert_eq!(
            fact(&json!({"artist":"Made Up Artist","card_faces":[{"artist":"Face Artist"}]}))
                .artist
                .as_deref(),
            Some("Made Up Artist")
        );
    }
    #[test]
    fn test_the_artist_comes_from_the_first_face() {
        assert_eq!(
            fact(&json!({"card_faces":[{"artist":"First Face"},{"artist":"Second Face"}]}))
                .artist
                .as_deref(),
            Some("First Face")
        );
    }
    #[test]
    fn test_a_card_with_no_artist_gives_none() {
        for c in [
            json!({}),
            json!({"card_faces":[]}),
            json!({"card_faces":[{}]}),
        ] {
            assert_eq!(fact(&c).artist, None);
        }
    }
    #[test]
    fn test_the_tuple_is_stamp_uri_artist_set() {
        assert_eq!(
            fact(
                &json!({"security_stamp":"oval","scryfall_uri":"https://scryfall.com/card/zzq/1/x?a=b","artist":"A. Nonymous","set":"zzq"})
            ),
            Fact {
                stamp: Some("oval".into()),
                uri: Some("https://scryfall.com/card/zzq/1/x".into()),
                artist: Some("A. Nonymous".into()),
                set: Some("zzq".into())
            }
        );
    }
}
mod printingfactstests {
    use super::*;
    #[test]
    fn test_a_payload_becomes_a_fact_under_its_id() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "a.json",
            &json!({"id":IMG,"set":"zzq","security_stamp":"triangle","artist":"X","scryfall_uri":"https://scryfall.com/card/zzq/1/n?utm=1"}),
        );
        assert_eq!(
            printing_facts(d.path()),
            BTreeMap::from([(
                IMG.into(),
                Fact {
                    stamp: Some("triangle".into()),
                    uri: Some("https://scryfall.com/card/zzq/1/n".into()),
                    artist: Some("X".into()),
                    set: Some("zzq".into())
                }
            )])
        );
    }
    #[test]
    fn test_several_payloads_are_all_read() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.json", &json!({"id":IMG,"set":"zzq"}));
        write(d.path(), "b.json", &json!({"id":IMG2,"set":"zzr"}));
        assert_eq!(
            printing_facts(d.path())
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([IMG, IMG2])
        );
    }
    #[test]
    fn test_a_list_payload_is_skipped() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "list.json", &json!([{"id":IMG,"set":"zzq"}]));
        assert!(printing_facts(d.path()).is_empty());
    }
    #[test]
    fn test_invalid_json_is_skipped() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("bad.json"), "{not json").unwrap();
        write(d.path(), "good.json", &json!({"id":IMG,"set":"zzq"}));
        assert_eq!(printing_facts(d.path()).len(), 1);
        assert!(printing_facts(d.path()).contains_key(IMG));
    }
    #[test]
    fn test_an_id_that_is_no_uuid_is_skipped() {
        let d = tempfile::tempdir().unwrap();
        for (i, c) in [
            json!({"set":"zzq"}),
            json!({"id":"abc"}),
            json!({"id":"AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA"}),
            json!({"id":7}),
            json!({"id":null}),
        ]
        .iter()
        .enumerate()
        {
            write(d.path(), &format!("{i}.json"), c);
        }
        assert!(printing_facts(d.path()).is_empty());
    }
    #[test]
    fn test_other_files_are_ignored() {
        let d = tempfile::tempdir().unwrap();
        for n in ["a.txt", "a.json.bak"] {
            write(d.path(), n, &json!({"id":IMG,"set":"zzq"}));
        }
        assert!(printing_facts(d.path()).is_empty());
    }
    #[test]
    fn test_a_missing_directory_or_no_path_gives_nothing() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.json", &json!({"id":IMG}));
        for p in [
            d.path().join("nowhere"),
            d.path().join("a.json"),
            std::path::PathBuf::from(""),
        ] {
            assert!(printing_facts(&p).is_empty());
        }
    }
    #[test]
    fn test_an_empty_directory_gives_nothing() {
        let d = tempfile::tempdir().unwrap();
        assert!(printing_facts(d.path()).is_empty());
    }
    #[test]
    fn test_the_facts_feed_the_rule() {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            "a.json",
            &json!({"id":IMG,"set":"zzq","security_stamp":"triangle"}),
        );
        assert_eq!(
            pictured("implemented", "zzq", Some(IMG), &printing_facts(d.path())),
            None
        );
    }
}
mod bulkprintingstests {
    use super::*;
    #[test]
    fn test_a_plain_printing_is_chosen_for_its_set_and_for_any() {
        let got = picks(&[payload(IMG, OID1, "zzq", "7")], &[OID1], None);
        assert_eq!(
            got,
            BTreeMap::from([(
                OID1.into(),
                BTreeMap::from([("zzq".into(), IMG.into()), ("*".into(), IMG.into())])
            )])
        );
    }
    #[test]
    fn test_a_non_triangle_printing_beats_a_triangle_one_in_the_same_set() {
        let got = picks(
            &[
                extra(
                    payload(IMG, OID1, "zzq", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
                payload(IMG2, OID1, "zzq", "99"),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["zzq"], IMG2);
        assert_eq!(got[OID1]["*"], IMG2);
    }
    #[test]
    fn test_the_order_in_the_file_does_not_matter() {
        let got = picks(
            &[
                payload(IMG2, OID1, "zzq", "99"),
                extra(
                    payload(IMG, OID1, "zzq", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["zzq"], IMG2);
    }
    #[test]
    fn test_a_card_that_is_ub_in_one_printing_and_not_in_another() {
        let got = picks(
            &[
                extra(
                    payload(IMG, OID1, "ltr", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
                payload(IMG2, OID1, "zzq", "50"),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["*"], IMG2);
        assert_eq!(got[OID1]["zzq"], IMG2);
        assert_eq!(got[OID1]["ltr"], IMG);
    }
    #[test]
    fn test_a_card_with_only_a_triangle_printing_still_has_one() {
        let mut f = Facts::new();
        let got = picks(
            &[extra(
                payload(IMG, OID2, "zzr", "3"),
                "security_stamp",
                json!("triangle"),
            )],
            &[OID2],
            Some(&mut f),
        );
        assert_eq!(got[OID2]["zzr"], IMG);
        assert_eq!(got[OID2]["*"], IMG);
        assert_eq!(f[IMG].stamp.as_deref(), Some("triangle"));
        assert_eq!(
            pictured("implemented", "zzr", Some(&got[OID2]["zzr"]), &f),
            None
        );
    }
    #[test]
    fn test_a_non_english_printing_is_ignored() {
        let got = picks(
            &[
                extra(payload(IMG, OID1, "zzq", "1"), "lang", json!("de")),
                payload(IMG2, OID1, "zzq", "40"),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["zzq"], IMG2);
        assert_eq!(got[OID1]["*"], IMG2);
    }
    #[test]
    fn test_a_card_with_only_a_non_english_printing_is_absent() {
        assert!(
            picks(
                &[extra(payload(IMG, OID1, "zzq", "1"), "lang", json!("ja"))],
                &[OID1],
                None
            )
            .is_empty()
        );
    }
    #[test]
    fn test_a_card_without_lang_counts_as_english() {
        assert_eq!(
            picks(
                &[absent(payload(IMG, OID1, "zzq", "1"), "lang")],
                &[OID1],
                None
            )[OID1]["*"],
            IMG
        );
    }
    #[test]
    fn test_a_missing_or_placeholder_image_is_ignored() {
        let got = picks(
            &[
                extra(
                    payload(IMG, OID1, "zzq", "1"),
                    "image_status",
                    json!("missing"),
                ),
                extra(
                    payload(IMG2, OID1, "zzq", "2"),
                    "image_status",
                    json!("placeholder"),
                ),
                extra(
                    payload(IMG3, OID1, "zzq", "30"),
                    "image_status",
                    json!("highres_scan"),
                ),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["*"], IMG3);
        assert_eq!(got[OID1]["zzq"], IMG3);
    }
    #[test]
    fn test_a_printing_with_no_picture_at_all_is_ignored() {
        let got = picks(
            &[
                absent(payload(IMG, OID1, "zzq", "1"), "image_uris"),
                payload(IMG2, OID1, "zzq", "20"),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["zzq"], IMG2);
    }
    #[test]
    fn test_a_double_faced_card_is_read_by_its_face() {
        let c = extra(
            absent(
                absent(
                    absent(payload(IMG, OID3, "zzq", "4"), "image_uris"),
                    "oracle_id",
                ),
                "artist",
            ),
            "card_faces",
            json!([{"oracle_id":OID3,"image_uris":{},"artist":"Face Artist"},{"oracle_id":OID3,"image_uris":{}}]),
        );
        let mut f = Facts::new();
        let got = picks(&[c], &[OID3], Some(&mut f));
        assert_eq!(got[OID3]["zzq"], IMG);
        assert_eq!(got[OID3]["*"], IMG);
        assert_eq!(f[IMG].artist.as_deref(), Some("Face Artist"));
    }
    #[test]
    fn test_an_unwanted_oracle_id_is_skipped_and_leaves_no_facts() {
        let mut f = Facts::new();
        let got = picks(
            &[
                payload(IMG, OTHER, "zzq", "1"),
                payload(IMG2, OID1, "zzq", "2"),
            ],
            &[OID1],
            Some(&mut f),
        );
        assert_eq!(got.len(), 1);
        assert!(got.contains_key(OID1));
        assert_eq!(f.len(), 1);
        assert!(f.contains_key(IMG2));
    }
    #[test]
    fn test_facts_cover_exactly_the_chosen_printings() {
        let mut f = Facts::new();
        picks(
            &[
                extra(
                    payload(IMG, OID1, "zzq", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
                payload(IMG2, OID1, "zzq", "9"),
                payload(IMG3, OID1, "zzq", "12"),
            ],
            &[OID1],
            Some(&mut f),
        );
        assert_eq!(
            f,
            BTreeMap::from([(
                IMG2.into(),
                Fact {
                    stamp: None,
                    uri: Some("https://scryfall.com/card/zzq/9/made-up".into()),
                    artist: Some("Artist 9".into()),
                    set: Some("zzq".into())
                }
            )])
        );
    }
    #[test]
    fn test_facts_carry_the_stamp_and_set_for_the_rule() {
        let mut f = Facts::new();
        let got = picks(
            &[extra(
                payload(IMG, OID1, "ltr", "1"),
                "security_stamp",
                json!("triangle"),
            )],
            &[OID1],
            Some(&mut f),
        );
        assert_eq!(f[IMG].set.as_deref(), Some("ltr"));
        assert_eq!(
            pictured("implemented", "ltr", Some(&got[OID1]["ltr"]), &f),
            None
        );
    }
    #[test]
    fn test_facts_are_optional() {
        assert_eq!(
            picks(&[payload(IMG, OID1, "zzq", "1")], &[OID1], None)[OID1]["*"],
            IMG
        );
    }
    #[test]
    fn test_existing_facts_are_kept() {
        let mut f = BTreeMap::from([(
            IMG7.into(),
            Fact {
                stamp: Some("oval".into()),
                artist: Some("Kept".into()),
                set: Some("zzq".into()),
                ..Fact::default()
            },
        )]);
        let old = f[IMG7].clone();
        picks(&[payload(IMG, OID1, "zzq", "1")], &[OID1], Some(&mut f));
        assert_eq!(f[IMG7], old);
        assert!(f.contains_key(IMG));
    }
    #[test]
    fn test_the_lowest_collector_number_wins_by_number_not_text() {
        assert_eq!(
            picks(
                &[
                    payload(IMG, OID1, "zzq", "10"),
                    payload(IMG2, OID1, "zzq", "9")
                ],
                &[OID1],
                None
            )[OID1]["zzq"],
            IMG2
        );
    }
    #[test]
    fn test_a_promo_loses_to_a_regular_printing() {
        assert_eq!(
            picks(
                &[
                    extra(payload(IMG, OID1, "zzq", "1"), "promo", json!(true)),
                    payload(IMG2, OID1, "zzq", "80")
                ],
                &[OID1],
                None
            )[OID1]["zzq"],
            IMG2
        );
    }
    #[test]
    fn test_a_triangle_still_loses_to_a_better_printing_in_another_set() {
        let got = picks(
            &[
                extra(
                    payload(IMG, OID1, "zzr", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
                extra(payload(IMG2, OID1, "zzq", "200"), "promo", json!(true)),
            ],
            &[OID1],
            None,
        );
        assert_eq!(got[OID1]["*"], IMG2);
        assert_eq!(got[OID1]["zzr"], IMG);
    }
    #[test]
    fn test_each_oracle_id_is_chosen_on_its_own() {
        let got = picks(
            &[
                payload(IMG, OID1, "zzq", "1"),
                extra(
                    payload(IMG2, OID2, "zzq", "1"),
                    "security_stamp",
                    json!("triangle"),
                ),
                payload(IMG3, OID2, "zzq", "2"),
            ],
            &[OID1, OID2],
            None,
        );
        assert_eq!(got[OID1]["zzq"], IMG);
        assert_eq!(got[OID2]["zzq"], IMG3);
    }
    #[test]
    fn test_a_gzip_file_reads_like_a_plain_one() {
        let cards = [
            extra(
                payload(IMG, OID1, "zzq", "1"),
                "security_stamp",
                json!("triangle"),
            ),
            payload(IMG2, OID1, "zzq", "5"),
            extra(payload(IMG3, OID1, "zzq", "6"), "lang", json!("fr")),
        ];
        let body = jsonl(&cards);
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("bulk.jsonl.gz");
        let mut gz = flate2::write::GzEncoder::new(
            fs::File::create(&p).unwrap(),
            flate2::Compression::default(),
        );
        gz.write_all(body.as_bytes()).unwrap();
        gz.finish().unwrap();
        let got = bulk_printings(&p, &BTreeSet::from([OID1.into()]), None).unwrap();
        assert_eq!(got, picks(&cards, &[OID1], None));
        assert_eq!(got[OID1]["zzq"], IMG2);
    }
    #[test]
    fn test_a_json_list_with_trailing_commas_is_read_line_by_line() {
        let body = format!(
            "[
{},
{}
]
",
            payload(IMG, OID1, "zzq", "1"),
            payload(IMG2, OID2, "zzr", "2")
        );
        let got = choose(
            Cursor::new(body),
            &BTreeSet::from([OID1.into(), OID2.into()]),
            None,
        )
        .unwrap();
        assert_eq!(got[OID1]["*"], IMG);
        assert_eq!(got[OID2]["*"], IMG2);
    }
    #[test]
    fn test_an_empty_wanted_set_chooses_nothing() {
        let mut f = Facts::new();
        assert!(picks(&[payload(IMG, OID1, "zzq", "1")], &[], Some(&mut f)).is_empty());
        assert!(f.is_empty());
    }
}
mod committeddatatests {
    use super::*;
    #[test]
    fn test_the_set_files_are_there() {
        checks::committed(&repo(), "exists").unwrap();
    }
    #[test]
    fn test_each_file_names_its_own_code() {
        checks::committed(&repo(), "code").unwrap();
    }
    #[test]
    fn test_no_absent_card_has_a_picture() {
        checks::committed(&repo(), "absent").unwrap();
    }
    #[test]
    fn test_no_card_in_a_universes_beyond_set_has_a_picture() {
        checks::committed(&repo(), "ub").unwrap();
    }
    #[test]
    fn test_every_picture_is_a_uuid() {
        checks::committed(&repo(), "uuid").unwrap();
    }
    #[test]
    fn test_only_a_pictured_status_has_a_picture() {
        checks::committed(&repo(), "status").unwrap();
    }
    #[test]
    fn test_a_uri_and_an_artist_come_only_with_a_picture() {
        checks::committed(&repo(), "facts").unwrap();
    }
}

fn sources() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            catalog::LEDGER.into(),
            format!(
                r#"pub static ROWS: [Row; 3] = [
Row {{ index: CardIndex::new(0), oracle_id: "{OID1}", constant: "A", set: "zzq", name: "First \"card\"" }},
Row {{ index: CardIndex::new(1), oracle_id: "{OID2}", constant: "B", set: "zzr", name: "Second" }},
Row {{ index: CardIndex::new(2), oracle_id: "{OID3}", constant: "C", set: "zzq", name: "Third" }},
];"#
            ),
        ),
        (
            "crates/baylee-core/src/generated/index/set_zzq.rs".into(),
            "// 2 cards first printed in `zzq`.".into(),
        ),
        (
            "crates/baylee-core/src/generated/index/set_zzr.rs".into(),
            "// 1 card first printed in `zzr`.".into(),
        ),
        (
            "crates/baylee-cards/src/cards/a.rs".into(),
            format!(
                r#"card!( oracle_id = "{OID1}", scryfall_id = "{IMG}", coverage = Coverage::Partial(concat!("first ", "second")) );"#
            ),
        ),
        (
            "data/unplayable.tsv".into(),
            format!("# test\n{OID2}\tSecond\tfictional reason\n"),
        ),
    ])
}
#[test]
fn catalog_preserves_order_status_notes_and_picture_precedence() {
    let c = catalog::parse(&sources()).unwrap();
    assert_eq!(c.rows[0].name, "First \"card\"");
    let p = BTreeMap::from([
        (OID1.into(), BTreeMap::from([("zzq".into(), IMG2.into())])),
        (OID3.into(), BTreeMap::from([("zzq".into(), IMG3.into())])),
    ]);
    let (order, sets) = card_status::cards(&c, &p, &BTreeMap::new(), &Facts::new()).unwrap();
    assert_eq!(order, ["zzq", "zzr"]);
    assert_eq!(sets["zzq"][0].note.as_deref(), Some("first second"));
    assert_eq!(sets["zzq"][0].img.as_deref(), Some(IMG2));
    assert_eq!(sets["zzq"][1].status, "absent");
    assert_eq!(sets["zzq"][1].img, None);
    assert_eq!(sets["zzr"][0].status, "never");
}
#[test]
fn rust_string_unescapes_continuations_and_unicode() {
    assert_eq!(
        catalog::rust_string("Ä\\\n    foo\\n\\t\\\"\\\\\\'"),
        "Äfoo\n\t\"\\'"
    );
}
#[test]
fn catalog_rejects_wrong_declared_rows_and_noncontiguous_indices() {
    for (old, new) in [
        ("[Row; 3]", "[Row; 4]"),
        ("CardIndex::new(1)", "CardIndex::new(7)"),
        (OID3, OID1),
    ] {
        let mut f = sources();
        f.get_mut(catalog::LEDGER).unwrap().push_str("");
        f.insert(catalog::LEDGER.into(), f[catalog::LEDGER].replace(old, new));
        assert!(catalog::parse(&f).is_err());
    }
}
#[test]
fn catalog_rejects_core_counts_missing_headers_and_extra_sets() {
    for body in [
        "// 3 cards first printed in `zzq`.",
        "missing header",
        "// 2 cards first printed in `zzx`.",
    ] {
        let mut f = sources();
        f.insert(
            "crates/baylee-core/src/generated/index/set_zzq.rs".into(),
            body.into(),
        );
        assert!(catalog::parse(&f).is_err());
    }
}
#[test]
fn catalog_rejects_duplicate_and_stray_pool_cards() {
    let mut f = sources();
    let card = f["crates/baylee-cards/src/cards/a.rs"].clone();
    f.insert("crates/baylee-cards/src/cards/b.rs".into(), card.clone());
    assert!(catalog::parse(&f).is_err());
    f.insert(
        "crates/baylee-cards/src/cards/b.rs".into(),
        card.replace(OID1, OTHER),
    );
    assert!(catalog::parse(&f).is_err());
}
#[test]
fn catalog_rejects_unplayable_overlap_stray_malformed_and_duplicate_rows() {
    for body in [
        format!("{OID1}\tFirst\twhy"),
        format!("{OTHER}\tOther\twhy"),
        "bad TSV".into(),
        format!("{OID2}\tSecond\twhy\n{OID2}\tSecond\twhy"),
    ] {
        let mut f = sources();
        f.insert("data/unplayable.tsv".into(), body);
        assert!(catalog::parse(&f).is_err());
    }
}
#[test]
fn card_parser_requires_oracle_and_explicit_or_stub_coverage() {
    let base = format!(r#"oracle_id = "{OID1}""#);
    assert!(catalog::read_card("a.rs", "coverage = Coverage::Implemented").is_err());
    assert!(catalog::read_card("a.rs", &base).is_err());
    let c = catalog::read_card("a.rs", &format!("{base} // GENERATED STUB")).unwrap();
    assert_eq!(c.status, "unimplemented");
    for (cov, status) in [("Implemented", "implemented"), ("Generated", "generated")] {
        let c = catalog::read_card("a.rs", &format!("{base} coverage = Coverage::{cov}")).unwrap();
        assert_eq!(c.status, status);
    }
    assert!(
        catalog::read_card(
            "a.rs",
            &format!(
                "{base} coverage = Coverage::Implemented coverage = Coverage::Partial(\"why\")"
            )
        )
        .is_err()
    );
    for suffix in [
        "coverage = Coverage::Partial",
        "coverage = Coverage::Partial(\"unterminated",
        "coverage = Coverage::Partial(\"why\"",
    ] {
        assert!(catalog::read_card("a.rs", &format!("{base} {suffix}")).is_err());
    }
}
#[test]
fn previous_picture_cannot_bypass_absence_or_ub_rule() {
    let c = catalog::parse(&sources()).unwrap();
    let ids = BTreeMap::from([(OID2.into(), IMG2.into()), (OID3.into(), IMG3.into())]);
    let f = BTreeMap::from([(
        IMG.into(),
        Fact {
            stamp: Some("triangle".into()),
            ..Fact::default()
        },
    )]);
    let (_, sets) = card_status::cards(&c, &BTreeMap::new(), &ids, &f).unwrap();
    assert!(sets["zzq"].iter().all(|c| c.img.is_none()));
    assert_eq!(sets["zzr"][0].img.as_deref(), Some(IMG2));
}
#[test]
fn digital_printings_and_suffixes_have_stable_numeric_order() {
    let cards = [
        extra(payload(IMG, OID1, "zzq", "1"), "digital", json!(true)),
        payload(IMG2, OID1, "zzq", "0009b"),
        payload(IMG3, OID1, "zzq", "9a"),
    ];
    assert_eq!(picks(&cards, &[OID1], None)[OID1]["*"], IMG3);
    assert_eq!(
        picks(
            &[
                payload(IMG, OID1, "zzq", &"9".repeat(100)),
                payload(IMG2, OID1, "zzq", "10")
            ],
            &[OID1],
            None
        )[OID1]["*"],
        IMG2
    );
}
#[test]
fn malformed_bulk_and_invalid_printing_identifiers_fail() {
    assert!(choose(Cursor::new("{broken}\n"), &BTreeSet::new(), None).is_err());
    for (key, value) in [("id", "../escape"), ("set", "../../escape")] {
        let body = extra(payload(IMG, OID1, "zzq", "1"), key, json!(value)).to_string();
        assert!(choose(Cursor::new(body), &BTreeSet::from([OID1.into()]), None).is_err());
    }
}

/// A throwaway repository with the four card-status inputs and a measured
/// `web/site/data/sets.json`: a commit that changes no input keeps the status
/// fresh, one that changes a card makes it stale, and a status measured
/// on another line of history is refused.
#[test]
fn test_the_card_status_is_fresh_until_an_input_changes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .current_dir(root)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    let put = |path: &str, text: &str| {
        let file = root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    };
    git(&["init", "-q", "-b", "main"]);
    for input in catalog::INPUTS {
        let is_file = std::path::Path::new(input)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rs") || e.eq_ignore_ascii_case("tsv"));
        let file = if is_file {
            input.to_owned()
        } else {
            format!("{input}/a.rs")
        };
        put(&file, "// a\n");
    }
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "pool"]);
    let measured = git(&["rev-parse", "HEAD"]);
    put(
        "web/site/data/sets.json",
        &json!({ "rev": measured }).to_string(),
    );
    put("README.md", "unrelated\n");
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "status and an unrelated file"]);
    card_status::fresh(root, "HEAD").expect("no input changed");

    put("crates/baylee-cards/src/cards/a.rs", "// b\n");
    git(&["commit", "-qam", "a card changed"]);
    let stale = card_status::fresh(root, "HEAD").unwrap_err().to_string();
    assert!(stale.contains("1 card file(s) changed"), "{stale}");

    git(&["checkout", "-q", "--orphan", "elsewhere"]);
    git(&["commit", "-qam", "another history"]);
    let foreign = card_status::fresh(root, "HEAD").unwrap_err().to_string();
    assert!(foreign.contains("not an ancestor"), "{foreign}");
}
