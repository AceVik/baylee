//! What the operator's console may name.

use super::*;

fn accounts(json: &str) -> Result<Vec<Uuid>, StatusCode> {
    accounts_of(&Bytes::copy_from_slice(json.as_bytes())).map_err(|(status, _)| status)
}

fn body(ids: usize) -> String {
    let ids: Vec<_> = (0..ids)
        .map(|n| Uuid::from_u128(n as u128 + 1).to_string())
        .collect();
    serde_json::json!({ "accounts": ids }).to_string()
}

#[test]
fn a_change_names_each_account_once_in_the_order_given() {
    let (a, b) = (Uuid::from_u128(7), Uuid::from_u128(3));
    let text = format!(
        r#"{{"accounts":["{a}","{b}","{}","{a}"]}}"#,
        b.to_string().to_uppercase()
    );
    assert_eq!(accounts(&text), Ok(vec![a, b]));
    assert_eq!(joined(&[a, b]), format!("{a},{b}"));
}

#[test]
fn a_change_names_one_account_to_the_bound() {
    let most = llm_access::MAX_CHANGE;
    assert_eq!(accounts(&body(1)).map(|ids| ids.len()), Ok(1));
    assert_eq!(accounts(&body(most)).map(|ids| ids.len()), Ok(most));
    for wrong in [0, most + 1] {
        assert_eq!(accounts(&body(wrong)), Err(StatusCode::BAD_REQUEST));
    }
}

#[test]
fn a_change_that_is_not_a_list_of_ids_names_nobody() {
    for bad in [
        "",
        "{}",
        "[]",
        r#"{"accounts":"00000000-0000-0000-0000-000000000001"}"#,
        r#"{"accounts":[1]}"#,
        r#"{"accounts":[null]}"#,
        r#"{"accounts":["*"]}"#,
        // One bad id refuses the whole change, the good ones included.
        r#"{"accounts":["00000000-0000-0000-0000-000000000001","x"]}"#,
    ] {
        assert_eq!(accounts(bad), Err(StatusCode::BAD_REQUEST), "{bad}");
    }
}

#[test]
fn a_profile_is_named_by_a_plain_word() {
    for good in ["a", "gpt-6.1_x", &"p".repeat(64)] {
        assert!(profile_id(good).is_ok(), "{good}");
    }
    for bad in ["", "a b", "../x", "a/b", "ä", "a\n", &"p".repeat(65)] {
        let (status, _) = profile_id(bad).expect_err(bad);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
