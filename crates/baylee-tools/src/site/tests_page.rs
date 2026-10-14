//! Every original page/accessibility/privacy regression, plus stamp dependency order.
use super::{checks, html, repo, stamp};
use std::fs;
#[test]
fn test_nothing_inline_that_the_csp_refuses() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "csp").unwrap();
    }
}
#[test]
fn test_every_picture_has_an_alt_text() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "alt").unwrap();
    }
}
#[test]
fn test_every_form_control_has_a_label() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "label").unwrap();
    }
}
#[test]
fn test_buttons_have_a_type_and_dialogs_a_name() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "button").unwrap();
    }
}
#[test]
fn test_german_blocks_say_they_are_german() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "german").unwrap();
    }
}
#[test]
fn test_headings_go_down_one_level_at_a_time_in_each_language() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "headings").unwrap();
    }
}
#[test]
fn test_links_that_open_a_tab_say_noopener() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "noopener").unwrap();
    }
}
#[test]
fn test_the_content_stamps_are_current() {
    stamp::run(&repo(), true).unwrap();
}
#[test]
fn test_no_card_picture_is_filtered_faded_or_blended() {
    checks::picture_css(&repo()).unwrap();
}
#[test]
fn test_every_page_links_the_privacy_notice_and_the_legal_notice() {
    for name in checks::PAGES {
        checks::page(&repo(), name, "footer").unwrap();
    }
}
#[test]
fn test_every_foreign_host_the_page_may_reach_is_named_in_the_notice() {
    checks::privacy(&repo(), "hosts").unwrap();
}
#[test]
fn test_no_script_sets_a_cookie() {
    checks::privacy(&repo(), "cookie").unwrap();
}
#[test]
fn test_the_site_keeps_no_access_log() {
    checks::privacy(&repo(), "log").unwrap();
}
#[test]
fn test_the_notice_names_the_controller_only_through_the_impressum() {
    checks::privacy(&repo(), "controller").unwrap();
}

fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let site = d.path().join("web/site");
    fs::create_dir_all(site.join("fonts")).unwrap();
    fs::create_dir_all(site.join("img")).unwrap();
    fs::write(site.join("fonts/a.woff2"), b"font").unwrap();
    fs::write(site.join("img/a.avif"), b"image").unwrap();
    fs::write(
        site.join("site.css"),
        "a{src:url(\"fonts/a.woff2\")}b{background:url(\"img/a.avif?v=bad\")}",
    )
    .unwrap();
    fs::write(site.join("index.html"),"<link href=\"/site/site.css\"><img srcset=\"/site/img/a.avif 1x, /site/img/a.avif?v=abc 2x\">").unwrap();
    d
}
#[test]
fn stamping_covers_css_dependencies_before_html_and_is_idempotent() {
    let d = fixture();
    assert!(stamp::run(d.path(), true).is_err());
    stamp::run(d.path(), false).unwrap();
    stamp::run(d.path(), true).unwrap();
    let site = d.path().join("web/site");
    let html = fs::read_to_string(site.join("index.html")).unwrap();
    assert!(html.contains(&format!(
        "/site/site.css?v={}",
        stamp::digest(&site.join("site.css")).unwrap()
    )));
    assert_eq!(
        html.matches(&format!(
            "/site/img/a.avif?v={}",
            stamp::digest(&site.join("img/a.avif")).unwrap()
        ))
        .count(),
        2
    );
    let before = fs::read(site.join("index.html")).unwrap();
    fs::write(site.join("fonts/a.woff2"), b"changed font").unwrap();
    assert!(stamp::run(d.path(), true).is_err());
    assert_eq!(fs::read(site.join("index.html")).unwrap(), before);
    stamp::run(d.path(), false).unwrap();
    assert_ne!(fs::read(site.join("index.html")).unwrap(), before);
    stamp::run(d.path(), true).unwrap();
}
#[test]
fn stamp_preserves_data_html_absolute_urls_and_prefix_boundaries() {
    let d = fixture();
    let site = d.path().join("web/site");
    let text = r#"<a href="/site/impressum.html">x</a><i src="/site/data/x.js"></i><i src="https://host.invalid/site/missing.js"></i> foo/site/missing.js '/site/missing.js' url("https://host.invalid/x")"#;
    assert_eq!(stamp::html(&site, text).unwrap(), text);
    let css = "a{background:url(\"https://host.invalid/x\")}b{background:url('img/no.png')}";
    assert_eq!(stamp::css(&site, css).unwrap(), css);
}
#[test]
fn stamp_rejects_missing_assets_and_directory_escape() {
    let d = fixture();
    let site = d.path().join("web/site");
    assert!(stamp::html(&site, "\"/site/missing.js\"").is_err());
    fs::write(d.path().join("web/out.js"), "x").unwrap();
    assert!(stamp::html(&site, "\"/site/../out.js\"").is_err());
}
#[test]
fn html_tokenizer_handles_comments_quotes_void_elements_rawtext_and_ancestors() {
    let tree=html::parse(r#"<!-- <script>bad</script> --><label><input title="a > b"/><span>A &amp; B &#xC4;</span></label><script src="x">const s = '<a>';</script>"#).unwrap();
    assert_eq!(tree.nodes.len(), 5);
    assert_eq!(tree.nodes[2].attr("title"), "a > b");
    assert_eq!(tree.ancestors(2)[0].tag, "label");
    assert_eq!(tree.nodes[3].text, "A & B Ä");
    assert!(tree.nodes[4].text.contains("<a>"));
}
#[test]
fn page_guards_reject_planted_accessibility_and_csp_violations() {
    for (check, body) in [
        ("csp", "<p style='color:red'>x</p>"),
        ("csp", "<script>bad()</script>"),
        ("csp", "<p onclick='bad()'>x</p>"),
        ("alt", "<img src='x'>"),
        ("label", "<input id='x'>"),
        ("button", "<button>go</button>"),
        ("button", "<dialog></dialog>"),
        ("german", "<p class='de'>Hallo</p>"),
        ("headings", "<h1>x</h1><h3>y</h3>"),
        ("noopener", "<a target='_blank'>x</a>"),
    ] {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("web/site")).unwrap();
        fs::write(d.path().join("web/site/test.html"), body).unwrap();
        assert!(
            checks::page(d.path(), "test.html", check).is_err(),
            "{check}: {body}"
        );
    }
}
