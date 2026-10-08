//! The terms of use (WG-1): a gateway with a terms file asks at sign-in
//! until the account accepts the version it shows, and a gateway without
//! one answers exactly as it did before terms existed. A gateway with a
//! directory of them shows each player the terms in their language, English
//! where it has not got theirs, and one version for all.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, http_headers, json_field, spawn_gateway, spawn_gateway_with};

const PASSWORD: &str = "a-very-fine-password";

/// Registers, then signs in; both answers, raw.
fn register_and_login(port: u16, username: &str) -> (String, String) {
    let register = format!(
        "{{\"username\":\"{username}\",\"display_name\":\"{username}\",\"password\":\"{PASSWORD}\"}}"
    );
    let (status, registered) = http(port, "POST", "/auth/register", None, &register);
    assert_eq!(status, 200, "register: {registered}");
    (registered, login(port, username))
}

fn login(port: u16, username: &str) -> String {
    let creds = format!("{{\"username\":\"{username}\",\"password\":\"{PASSWORD}\"}}");
    let (status, body) = http(port, "POST", "/auth/login", None, &creds);
    assert_eq!(status, 200, "login: {body}");
    body
}

fn guest(port: u16) -> String {
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "guest: {body}");
    body
}

fn json(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|e| panic!("{e}: {body}"))
}

/// A fresh directory for one test, named after it: the tests of this binary
/// share a process id and run at once.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-terms-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// A directory of terms, one file per `(name, text)`.
fn terms_dir(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = temp_dir(name);
    for (file, text) in files {
        std::fs::write(dir.join(file), text).expect("write terms");
    }
    dir
}

#[test]
fn without_a_terms_file_nothing_changes() {
    let gw = spawn_gateway("terms-none");
    let (status, info) = http(gw.port, "GET", "/info", None, "");
    assert_eq!(status, 200);
    assert_eq!(json(&info)["terms"], serde_json::Value::Null, "{info}");
    let (status, _) = http(gw.port, "GET", "/terms", None, "");
    assert_eq!(status, 404, "a gateway without terms served some");

    // The answers carry no `terms_stale` at all, not `false`: a client from
    // before terms reads exactly what it always read.
    let (registered, logged_in) = register_and_login(gw.port, "plain");
    for answer in [registered, logged_in, guest(gw.port)] {
        assert!(json(&answer).get("terms_stale").is_none(), "{answer}");
    }
    let token = json_field(&login(gw.port, "plain"), "token").to_string();
    let (status, _) = http(
        gw.port,
        "POST",
        "/account/terms",
        Some(&token),
        r#"{"version":"x"}"#,
    );
    assert_eq!(status, 404, "accepted terms that do not exist");
}

#[test]
fn a_sign_in_asks_until_the_shown_version_is_accepted() {
    let dir = temp_dir("file");
    let path = dir.join("terms.md");
    std::fs::write(
        &path,
        "<!-- version: 2026-10 -->\n<!-- updated: 2026-10-07 -->\n\n# Terms\n\nBe kind.\n",
    )
    .expect("write terms");
    let gw = spawn_gateway_with(
        "terms",
        &[("BAYLEE_TERMS_PATH", path.to_string_lossy().into_owned())],
    );
    let port = gw.port;

    let (_, info) = http(port, "GET", "/info", None, "");
    assert_eq!(json(&info)["terms"], "2026-10", "{info}");
    // One file is every language's: asked in any, or in none, the answer is
    // the one it was before there were languages, naming none.
    for path in [
        "/terms",
        "/terms?lang=de",
        "/terms?lang=en",
        "/terms?lang=fr",
    ] {
        let (status, terms) = http(port, "GET", path, None, "");
        assert_eq!(status, 200, "{path}");
        assert_eq!(
            json(&terms),
            serde_json::json!({
                "version": "2026-10",
                "updated": "2026-10-07",
                "markdown": "# Terms\n\nBe kind.\n",
            }),
            "{path}"
        );
    }
    let (_, head) = http_headers(port, "GET", "/terms?lang=de", &[]);
    assert!(!head.contains("content-language"), "{head}");

    // A new account, a returning one that never accepted, a new guest: all
    // asked.
    let (registered, logged_in) = register_and_login(port, "reader");
    assert_eq!(json(&registered)["terms_stale"], true, "{registered}");
    assert_eq!(json(&logged_in)["terms_stale"], true, "{logged_in}");
    assert_eq!(json(&guest(port))["terms_stale"], true);

    let token = json_field(&logged_in, "token").to_string();
    let accept = |version: &str, token: Option<&str>| {
        http(
            port,
            "POST",
            "/account/terms",
            token,
            &format!("{{\"version\":\"{version}\"}}"),
        )
        .0
    };
    assert_eq!(accept("2026-10", None), 401, "a stranger accepted terms");
    // An older text than the one in force is not what is being agreed to.
    assert_eq!(accept("2026-09", Some(&token)), 409);
    assert_eq!(
        json(&login(port, "reader"))["terms_stale"],
        true,
        "a refused acceptance was recorded"
    );
    assert_eq!(accept("2026-10", Some(&token)), 200);
    assert_eq!(
        json(&login(port, "reader"))["terms_stale"],
        false,
        "accepted, and still asked"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

const EN: &str = "<!-- version: 2026-10 -->\n<!-- updated: 2026-10-08 -->\n\n# Terms\n\nBe kind.\n";
const DE: &str = "<!-- version: 2026-10 -->\n\n# Bedingungen\n\nSei freundlich.\n";

/// The owner's case (08.10.2026): the terms in German and English, each
/// player shown them in the language their client speaks, English for any
/// other, and one version accepted whichever was read.
#[test]
fn a_directory_shows_each_language_and_english_for_the_rest() {
    let dir = terms_dir(
        "langs",
        &[
            ("terms.en.md", EN),
            ("terms.de.md", DE),
            ("README", "not read"),
        ],
    );
    let gw = spawn_gateway_with(
        "terms_langs",
        &[("BAYLEE_TERMS_PATH", dir.to_string_lossy().into_owned())],
    );
    let port = gw.port;

    let (_, info) = http(port, "GET", "/info", None, "");
    assert_eq!(json(&info)["terms"], "2026-10", "{info}");
    let english = serde_json::json!({
        "version": "2026-10",
        "updated": "2026-10-08",
        "markdown": "# Terms\n\nBe kind.\n",
        "lang": "en",
    });
    let german = serde_json::json!({
        "version": "2026-10",
        "markdown": "# Bedingungen\n\nSei freundlich.\n",
        "lang": "de",
    });
    for (path, shown) in [
        ("/terms?lang=de", &german),
        ("/terms?lang=de-AT", &german),
        ("/terms?lang=en", &english),
        ("/terms?lang=fr", &english),
        ("/terms", &english),
    ] {
        let (status, terms) = http(port, "GET", path, None, "");
        assert_eq!(status, 200, "{path}: {terms}");
        assert_eq!(&json(&terms), shown, "{path}");
    }
    // Only `?lang=` decides: a browser's own language does not.
    let (_, head) = http_headers(port, "GET", "/terms", &[("Accept-Language", "de")]);
    assert!(head.contains("content-language: en"), "{head}");
    let (_, head) = http_headers(port, "GET", "/terms?lang=de", &[]);
    assert!(head.contains("content-language: de"), "{head}");

    // Read in German, accepted: the version is the account's in every
    // language.
    let (_, logged_in) = register_and_login(port, "leserin");
    assert_eq!(json(&logged_in)["terms_stale"], true, "{logged_in}");
    let token = json_field(&logged_in, "token").to_string();
    let (status, _) = http(
        port,
        "POST",
        "/account/terms",
        Some(&token),
        r#"{"version":"2026-10"}"#,
    );
    assert_eq!(status, 200);
    assert_eq!(
        json(&login(port, "leserin"))["terms_stale"],
        false,
        "accepted in one language, and still asked"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A directory the gateway cannot serve as one set of terms refuses
/// startup, before the database is asked anything, saying why.
#[test]
fn a_directory_that_is_not_one_set_of_terms_refuses_startup() {
    let older = "<!-- version: 2026-09 -->\n# Bedingungen\n";
    for (name, files, said) in [
        (
            "versions",
            vec![("terms.en.md", EN), ("terms.de.md", older)],
            "every language must name the same version",
        ),
        ("no_en", vec![("terms.de.md", DE)], "holds no terms.en.md"),
        (
            "bad_lang",
            vec![("terms.en.md", EN), ("terms.de-AT.md", DE)],
            "primary language tag",
        ),
    ] {
        let dir = terms_dir(&format!("refused-{name}"), &files);
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
            .env_remove("DATABASE_URL")
            .env("PORT", "0")
            .env("BAYLEE_TERMS_PATH", &dir)
            .output()
            .expect("run the gateway");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!out.status.success(), "{name}: started");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("BAYLEE_TERMS_PATH") && stderr.contains(said),
            "{name}: {stderr}"
        );
    }
}
