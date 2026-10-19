//! All original Impressum/blob tests use fictional people; no private files are read.
use super::{checks, impressum::*, repo};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
const NONCE: [u8; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
fn fake() -> Value {
    json!({"name":"Erika Mustermann","street":"Musterweg 12","postcode":"12345","city":"Musterstadt","country_de":"Deutschland","country_en":"Germany","email":"erika@example.org","phone":"+49 30 1234567"})
}
fn umlaut() -> Value {
    json!({"name":"Jürgen Müller","street":"Hauptstraße 5","postcode":"80331","city":"Köln-Süd","country_de":"Österreich","country_en":"Austria","email":"j.mueller@example.net","phone":"+43 (0) 660 987 6543"})
}
fn fields(v: &Value) -> Fields {
    FIELDS
        .iter()
        .map(|k| ((*k).into(), v[*k].as_str().unwrap().trim().into()))
        .collect()
}
fn decrypt(blob: &str) -> Value {
    let raw = URL_SAFE_NO_PAD.decode(blob).unwrap();
    serde_json::from_slice(&xor(&raw[21..], &raw[5..21])).unwrap()
}
fn pack(doc: &Value) -> String {
    let mut raw = MAGIC.to_vec();
    raw.push(VERSION);
    raw.extend(NONCE);
    raw.extend(xor(&serde_json::to_vec(doc).unwrap(), &NONCE));
    URL_SAFE_NO_PAD.encode(raw)
}
fn assert_no_leaks(text: &str, values: &Fields) {
    for n in needles(values) {
        assert!(!text.to_lowercase().contains(&n.to_lowercase()));
    }
}

#[test]
fn test_roundtrip() {
    for fake in [fake(), umlaut()] {
        let mut padded = fake.clone();
        for key in FIELDS {
            padded[key] = json!(format!("  {}  ", fake[key].as_str().unwrap()));
        }
        padded["updated"] = json!("2026-10-13");
        padded["internal_note"] = json!("not for the page");
        assert_eq!(
            decode(&encode(&padded, None).unwrap()).unwrap(),
            fields(&fake)
        );
    }
}
#[test]
fn test_nonce() {
    let a = encode(&fake(), Some(&NONCE)).unwrap();
    let b = encode(&fake(), Some(&NONCE)).unwrap();
    assert_ne!(a, b);
    let mut header = MAGIC.to_vec();
    header.push(VERSION);
    header.extend(NONCE);
    for blob in [a, b] {
        assert_eq!(decode(&blob).unwrap(), fields(&fake()));
        assert_eq!(URL_SAFE_NO_PAD.decode(&blob).unwrap()[..21], header);
    }
    let a = encode(&fake(), None).unwrap();
    let b = encode(&fake(), None).unwrap();
    assert_ne!(a, b);
    assert_ne!(
        &URL_SAFE_NO_PAD.decode(a).unwrap()[5..21],
        &URL_SAFE_NO_PAD.decode(b).unwrap()[5..21]
    );
    let zero = encode(&fake(), Some(&[0; 16])).unwrap();
    assert_ne!(&URL_SAFE_NO_PAD.decode(zero).unwrap()[..21], header);
    assert!(encode(&fake(), Some(b"short")).is_err());
}
#[test]
fn test_alphabet() {
    for fake in [fake(), umlaut()] {
        for nonce in [Some(NONCE.as_slice()), None, None] {
            let blob = encode(&fake, nonce).unwrap();
            assert!(!blob.is_empty());
            assert!(
                blob.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            );
        }
    }
}
#[test]
fn test_no_plaintext() {
    let email = regex::Regex::new(checks::EMAIL).unwrap();
    for fake in [fake(), umlaut()] {
        let blob = encode(&fake, Some(&NONCE)).unwrap();
        assert!(!readable(&blob, &fields(&fake)));
        assert!(!email.is_match(&blob));
    }
}
#[test]
fn test_plaintext_check_can_fail() {
    for fake in [fake(), umlaut()] {
        let doc = json!({"v":1,"fields":fake});
        assert!(readable(
            &URL_SAFE_NO_PAD.encode(doc.to_string()),
            &fields(&fake)
        ));
    }
    let mut raw = MAGIC.to_vec();
    raw.push(VERSION);
    raw.extend(NONCE);
    raw.extend(fake()["name"].as_str().unwrap().as_bytes());
    assert!(readable(&URL_SAFE_NO_PAD.encode(raw), &fields(&fake())));
}
#[test]
fn test_v2_plaintext_is_code_points_not_text() {
    assert_eq!(VERSION, 2);
    for fake in [fake(), umlaut()] {
        for nonce in [Some(NONCE.as_slice()), None] {
            let doc = decrypt(&encode(&fake, nonce).unwrap());
            assert_eq!(doc["v"], 2);
            assert_eq!(
                doc["fields"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>(),
                FIELDS.into_iter().collect()
            );
            for key in FIELDS {
                let pairs = doc["fields"][key].as_array().unwrap();
                assert_eq!(pairs.len(), fake[key].as_str().unwrap().chars().count());
                assert_eq!(
                    pairs
                        .iter()
                        .map(|p| p[0].as_u64().unwrap())
                        .collect::<BTreeSet<_>>(),
                    (0..pairs.len() as u64).collect()
                );
                assert!(pairs.iter().all(|p| p[1].as_u64().is_some()));
            }
            assert_no_leaks(&doc.to_string(), &fields(&fake));
        }
    }
    let orders: BTreeSet<_> = (0..20)
        .map(|_| {
            decrypt(&encode(&fake(), Some(&NONCE)).unwrap())["fields"]["name"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p[0].as_u64().unwrap())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(orders.len() > 1);
}
#[test]
fn test_v2_field_length_is_bounded() {
    assert_eq!(MAX_LEN, 64);
    let mut ok = fake();
    ok["street"] = json!("x".repeat(MAX_LEN));
    assert_eq!(
        decode(&encode(&ok, Some(&NONCE)).unwrap()).unwrap()["street"],
        "x".repeat(MAX_LEN)
    );
    for key in FIELDS {
        let mut bad = fake();
        bad[key] = json!("y".repeat(MAX_LEN + 1));
        assert!(encode(&bad, None).is_err());
    }
}
#[test]
fn test_missing_or_empty_field() {
    for key in FIELDS {
        let mut bad = fake();
        bad.as_object_mut().unwrap().remove(key);
        assert!(encode(&bad, None).is_err());
        let mut bad = fake();
        bad[key] = json!("   ");
        assert!(encode(&bad, None).is_err());
    }
    let mut bad = fake();
    bad["phone"] = json!(4930);
    assert!(encode(&bad, None).is_err());
}
#[test]
fn test_decode_rejects_other_blobs() {
    let mut raw = b"NOPE".to_vec();
    raw.extend([0; 40]);
    assert!(decode(&URL_SAFE_NO_PAD.encode(raw)).is_err());
}
#[test]
fn test_readable_detects_plaintext() {
    test_plaintext_check_can_fail();
    assert!(!readable(
        &encode(&fake(), Some(&NONCE)).unwrap(),
        &fields(&fake())
    ));
}
#[test]
fn test_every_random_blob_is_clean() {
    for fake in [fake(), umlaut()] {
        for _ in 0..300 {
            let blob = encode(&fake, None).unwrap();
            assert!(!readable(&blob, &fields(&fake)));
            assert_eq!(decode(&blob).unwrap(), fields(&fake));
        }
    }
}
#[test]
fn test_a_blob_that_stays_readable_is_never_returned() {
    for nonce in [Some(NONCE.as_slice()), None] {
        let mut count = 0;
        let result = encode_checked(&fake(), nonce, |blob, _| {
            count += 1;
            if nonce.is_some() {
                assert_eq!(&URL_SAFE_NO_PAD.decode(blob).unwrap()[5..21], &NONCE);
            }
            true
        });
        assert!(result.is_err());
        assert_eq!(count, 64);
    }
}
#[test]
fn test_a_readable_first_draw_is_redrawn() {
    let mut count = 0;
    let blob = encode_checked(&fake(), None, |_, _| {
        count += 1;
        count == 1
    })
    .unwrap();
    assert_eq!(count, 2);
    assert_eq!(decode(&blob).unwrap(), fields(&fake()));
}

fn write_case(fake: &Value) {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("repo");
    fs::create_dir(&root).unwrap();
    let input = d.path().join("in.json");
    fs::write(&input, fake.to_string()).unwrap();
    let output = d.path().join("nested/impressum.bin");
    let size = write_blob(&root, &input, &output).unwrap();
    let bytes = fs::read(&output).unwrap();
    assert_eq!(size, bytes.len());
    assert!(bytes.ends_with(b"\n"));
    let text = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(decode(text).unwrap(), fields(fake));
    assert!(!readable(text.trim(), &fields(fake)));
    assert!(!output.with_extension("bin.tmp").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(output).unwrap().permissions().mode() & 0o777,
            0o644
        );
    }
}
#[test]
fn test_main_writes_blob() {
    let mut fake = fake();
    fake["updated"] = json!("2026-10-13");
    write_case(&fake);
}
#[test]
fn test_main_umlauts() {
    write_case(&umlaut());
}
#[test]
fn test_main_incomplete_json() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("repo");
    fs::create_dir(&root).unwrap();
    let input = d.path().join("in.json");
    let output = d.path().join("out.bin");
    let mut cases = vec!["{not json".to_owned(), "[1,2]".to_owned()];
    for key in ["email", "name"] {
        let mut bad = fake();
        bad.as_object_mut().unwrap().remove(key);
        cases.push(bad.to_string());
    }
    let mut bad = fake();
    bad["city"] = json!("");
    cases.push(bad.to_string());
    for body in cases {
        fs::write(&input, body).unwrap();
        let err = write_blob(&root, &input, &output).unwrap_err();
        assert_no_leaks(&format!("{err:#}"), &fields(&fake()));
        assert!(!output.exists());
    }
    assert!(write_blob(&root, &d.path().join("none.json"), &output).is_err());
}
#[test]
fn test_main_refuses_output_inside_repo() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("repo");
    fs::create_dir(&root).unwrap();
    for rel in ["web/site/x.bin", "web/private/impressum.bin", "x.bin"] {
        let output = root.join(rel);
        let err = write_blob(&root, &d.path().join("nonexistent.json"), &output).unwrap_err();
        assert!(err.to_string().contains("inside the repository"));
        assert!(!output.exists());
        assert_no_leaks(&format!("{err:#}"), &fields(&fake()));
    }
}
#[test]
fn test_default_out_is_outside_repo() {
    assert!(outside(&repo(), &default_out().unwrap()).is_ok());
}
#[test]
fn test_guard_finds_planted_files() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    fs::create_dir_all(root.join("site/private")).unwrap();
    for name in [
        "site/private/impressum.bin",
        "site/private/notes.txt",
        "impressum-data.json",
        "impressum-data-2.json",
        "site/ok.html",
        "data.json",
    ] {
        fs::write(root.join(name), "x").unwrap();
    }
    let found: BTreeSet<_> = checks::private_files(root)
        .unwrap()
        .iter()
        .map(|p| p.strip_prefix(root).unwrap().to_owned())
        .collect();
    assert_eq!(
        found,
        BTreeSet::from([
            "site/private/impressum.bin".into(),
            "site/private/notes.txt".into(),
            "impressum-data.json".into(),
            "impressum-data-2.json".into()
        ])
    );
    let d = tempfile::tempdir().unwrap();
    fs::create_dir(d.path().join("a")).unwrap();
    fs::write(d.path().join("a/blob.bin"), "x").unwrap();
    assert_eq!(checks::private_files(d.path()).unwrap().len(), 1);
}
#[test]
fn test_guard_finds_nothing_under_web() {
    assert!(
        checks::private_files(&repo().join("web"))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn test_gitignore_covers_private_paths() {
    checks::ignores(&repo()).unwrap();
}
#[test]
fn test_script_key_and_constants_agree_with_the_python() {
    checks::impressum(&repo(), "constants").unwrap();
}
#[test]
fn test_static_files_hold_no_data_shapes() {
    checks::impressum(&repo(), "shapes").unwrap();
}
#[test]
fn test_data_shape_regexes_can_fail() {
    assert!(
        regex::Regex::new(checks::EMAIL)
            .unwrap()
            .is_match("write to erika@example.org today")
    );
    assert!(
        regex::Regex::new(checks::DIGITS)
            .unwrap()
            .is_match("call 0301234567")
    );
    assert!(
        !regex::Regex::new(checks::DIGITS)
            .unwrap()
            .is_match("12345 Musterstadt")
    );
}
#[test]
fn test_impressum_page() {
    checks::impressum(&repo(), "page").unwrap();
}
#[test]
fn test_honeypot_link_is_hidden_from_people_and_the_page_names_no_private_path() {
    checks::impressum(&repo(), "honeypot").unwrap();
}
#[test]
fn test_index_links_to_the_impressum_twice() {
    checks::impressum(&repo(), "links").unwrap();
}
#[test]
fn test_css_orders_exactly_max_len_classes() {
    checks::impressum(&repo(), "orders").unwrap();
}
#[test]
fn test_decoys_stay_in_inner_text() {
    checks::impressum(&repo(), "decoys").unwrap();
}
#[test]
fn test_robots_keeps_crawlers_away() {
    checks::impressum(&repo(), "robots").unwrap();
}

#[test]
fn malformed_and_truncated_blobs_are_rejected_without_panics_or_pii() {
    let blob = encode(&fake(), Some(&NONCE)).unwrap();
    let raw = URL_SAFE_NO_PAD.decode(&blob).unwrap();
    // Every truncation of a real envelope, including within UTF-8 and JSON.
    for end in 0..raw.len() {
        assert!(decode(&URL_SAFE_NO_PAD.encode(&raw[..end])).is_err());
    }
    for body in ["!invalid!".into(), "A".into(), "a".repeat(65537)] {
        assert!(decode(&body).is_err());
    }
    let original = decrypt(&blob);
    let mut invalid = Vec::new();
    for v in [json!(null), json!(3), json!("2")] {
        let mut doc = original.clone();
        doc["v"] = v;
        invalid.push(doc);
    }
    let mut doc = original.clone();
    doc["fields"] = json!([]);
    invalid.push(doc);
    let mut doc = original.clone();
    doc["fields"].as_object_mut().unwrap().remove("name");
    invalid.push(doc);
    let mut doc = original.clone();
    doc["fields"]["unknown"] = json!([[0, 65]]);
    invalid.push(doc);
    for pairs in [
        json!([]),
        json!([[0, 65], [0, 66]]),
        json!([[2, 65]]),
        json!([[-1, 65]]),
        json!([[0, 0x0011_0000]]),
        json!([[0, 0xd800]]),
        json!([[0, 65.5]]),
        json!([[0, 65, 66]]),
        json!([[0, "A"]]),
        json!([[null, 65]]),
        json!([[18_446_744_073_709_551_615_u64, 65]]),
        json!("bad"),
        json!(vec![[0, 65]; 65]),
    ] {
        let mut doc = original.clone();
        doc["fields"]["name"] = pairs;
        invalid.push(doc);
    }
    for doc in invalid {
        let err = decode(&pack(&doc)).unwrap_err();
        assert_no_leaks(&format!("{err:#}"), &fields(&fake()));
    }
}
#[test]
fn canonical_output_fence_handles_rooted_and_canonical_paths() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    fs::create_dir(&root).unwrap();
    // canonicalize returns a verbatim Windows prefix. Both that prefix and
    // the ordinary rooted spelling must obey the same repository fence.
    for base in [dir.path().to_owned(), fs::canonicalize(dir.path()).unwrap()] {
        assert_eq!(resolve(&base).unwrap(), fs::canonicalize(&base).unwrap());
        assert!(outside(&root, &base.join("repo/missing/../blob.bin")).is_err());
        assert!(outside(&root, &base.join("sibling/blob.bin")).is_ok());
    }
}

#[cfg(unix)]
#[test]
fn canonical_output_fence_rejects_symlink_and_dotdot_escapes() {
    use std::os::unix::fs::symlink;
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("repo");
    fs::create_dir_all(root.join("nested")).unwrap();
    symlink(root.join("nested"), d.path().join("link")).unwrap();
    for out in [
        d.path().join("link/blob.bin"),
        d.path().join("link/../blob.bin"),
        root.join("missing/../blob.bin"),
    ] {
        assert!(outside(&root, &out).is_err());
    }
    symlink(root.join("never.bin"), d.path().join("dangling.bin")).unwrap();
    assert!(outside(&root, &d.path().join("dangling.bin")).is_err());
    let sibling = d.path().join("repo-other").join("blob.bin");
    assert!(outside(&root, &sibling).is_ok());
}
#[test]
fn malicious_input_never_appears_in_writer_errors_and_leaves_no_output() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("repo");
    fs::create_dir(&root).unwrap();
    let input = d.path().join("erika@example.org.json");
    let out = d.path().join("out.bin");
    fs::write(&input, "{\"Erika Mustermann\":broken}").unwrap();
    let err = write_blob(&root, &input, &out).unwrap_err();
    assert_no_leaks(&format!("{err:#}"), &fields(&fake()));
    assert!(!out.exists());
    fs::write(&input, "x".repeat(65537)).unwrap();
    assert!(write_blob(&root, &input, &out).is_err());
    assert!(!out.exists());
}
#[test]
fn unicode_scalar_values_count_as_characters_and_roundtrip() {
    let mut doc = umlaut();
    doc["name"] = json!("🦀".repeat(64));
    assert_eq!(
        decode(&encode(&doc, None).unwrap()).unwrap()["name"],
        "🦀".repeat(64)
    );
    doc["name"] = json!("🦀".repeat(65));
    assert!(encode(&doc, None).is_err());
}

#[test]
fn private_blob_is_also_refused_in_a_different_git_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("selected");
    let other = dir.path().join("another checkout");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&other).unwrap();
    // Worktrees use a .git file; ordinary clones use a .git directory.
    fs::write(other.join(".git"), "gitdir: /unused/common/worktrees/other").unwrap();
    assert!(outside(&root, &other.join("impressum.bin")).is_err());
    fs::remove_file(other.join(".git")).unwrap();
    fs::create_dir(other.join(".git")).unwrap();
    assert!(outside(&root, &other.join("private/impressum.bin")).is_err());
    assert!(outside(&root, &dir.path().join("outside/impressum.bin")).is_ok());
}
