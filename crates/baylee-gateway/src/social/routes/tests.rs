//! What the social routes read out of a request before any database.

use super::*;

#[test]
fn a_key_is_unpadded_base64url_of_exactly_its_length() {
    let key = [0xFB; 32];
    let text = URL_SAFE_NO_PAD.encode(key);
    assert_eq!(b64(&text, 32).ok(), Some(key.to_vec()));
    let refused = [
        (text.clone(), 31),
        (text.clone(), 33),
        (String::new(), 32),
        (format!("{text}="), 32),
        (format!(" {text}"), 32),
        (text.replace('-', "+"), 32),
    ];
    for (bad, len) in refused {
        let (status, _) = b64(&bad, len).expect_err(&bad);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{bad:?}");
    }
}

#[test]
fn a_device_label_is_short_visible_and_turns_no_text_around() {
    for good in ["m", "macOS", "Größe 7", &"é".repeat(wire::MAX_DEVICE_LABEL)] {
        assert!(label_ok(good), "{good:?}");
    }
    for bad in [
        String::new(),
        "   ".into(),
        "\u{3000}".into(),
        "x".repeat(wire::MAX_DEVICE_LABEL + 1),
    ] {
        assert!(!label_ok(&bad), "{bad:?}");
    }
    for hidden in [
        '\0', '\n', '\u{7F}', '\u{85}', '\u{AD}', '\u{61C}', '\u{180E}', '\u{200B}', '\u{200E}',
        '\u{200F}', '\u{202A}', '\u{202E}', '\u{2060}', '\u{2064}', '\u{2066}', '\u{2069}',
        '\u{FEFF}',
    ] {
        let label = format!("mac{hidden}OS");
        assert!(!label_ok(&label), "{:x}", u32::from(hidden));
    }
}

#[test]
fn a_query_names_tags_one_by_one_or_in_a_list() {
    let cases: [(Option<&str>, &[&str]); 8] = [
        (None, &[]),
        (Some(""), &[]),
        (Some("tag=af03"), &["af03"]),
        (Some("tag=a&tag=b"), &["a", "b"]),
        (Some("tag=a,b+c&x=d"), &["a", "b", "c"]),
        (Some("tag=,,&tag="), &[]),
        // A name that only ends or begins like it is another parameter.
        (Some("atag=a&tags=b&TAG=c"), &[]),
        // Percent-encoded text is never a tag, and never a list.
        (Some("tag=a%2Cb&tag=%00&tag=c"), &["c"]),
    ];
    for (query, want) in cases {
        assert_eq!(tags_in(query), want, "{query:?}");
    }
}

#[test]
fn a_group_change_reaches_every_member_and_invitee_once() {
    let who = |n: u128| Who {
        id: Uuid::from_u128(n),
        tag: 1,
        name: "N".into(),
        guest: false,
    };
    let group = db::Group {
        id: Uuid::from_u128(9),
        name: "G".into(),
        members: vec![(who(2), true), (who(1), false)],
        invited: vec![who(3), who(1)],
    };
    let told: Vec<_> = everyone(&group).into_iter().collect();
    assert_eq!(told, [1, 2, 3].map(|n| Uuid::from_u128(n).to_string()));
    assert_eq!(
        group_conv(group.id).parse::<Conv>().ok(),
        Some(Conv::Group {
            id: group.id.to_string()
        })
    );
}

#[test]
fn a_guest_has_no_friends_list() {
    let caller = |guest| Caller {
        account: Uuid::nil().to_string(),
        id: Uuid::nil(),
        guest,
    };
    assert!(registered(&caller(false)).is_ok());
    let (status, _) = registered(&caller(true)).expect_err("a guest");
    assert_eq!(status, StatusCode::FORBIDDEN);
}
