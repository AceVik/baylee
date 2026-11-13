
use super::*;

fn add(origin: &str) -> Link {
    Link::AddGateway(origin.into())
}

#[test]
fn the_three_links_are_read() {
    assert_eq!(read("baylee://open", Origins::Public), Some(Link::Open));
    assert_eq!(read("baylee://open/", Origins::Public), Some(Link::Open));
    assert_eq!(read("baylee://signin", Origins::Public), Some(Link::SignIn));
    // A scheme is read in any case (RFC 3986 §3.1); the rest is exact.
    assert_eq!(read("Baylee://open", Origins::Public), Some(Link::Open));
    assert_eq!(
        read(
            "baylee://gateway/add?origin=https%3A%2F%2Fgateway.example.org",
            Origins::Public
        ),
        Some(add("https://gateway.example.org"))
    );
    // The website's encodeURIComponent, a port, a trailing slash.
    assert_eq!(
        read(
            "baylee://gateway/add?origin=https%3A%2F%2Fplay.example.org%3A8443%2F",
            Origins::Public
        ),
        Some(add("https://play.example.org:8443"))
    );
    // An origin left unencoded still reads as the same origin.
    assert_eq!(
        read(
            "baylee://gateway/add?origin=https://gateway.example.org",
            Origins::Public
        ),
        Some(add("https://gateway.example.org"))
    );
}

#[test]
fn a_saved_address_is_spelled_the_way_the_typed_one_is() {
    // Upper case and the default port fall away, an IDN reads as its
    // ASCII form: what the player is asked about is what is saved.
    assert_eq!(
        gateway_origin("HTTPS://Gateway.Example.ORG:443/", Origins::Public).as_deref(),
        Some("https://gateway.example.org")
    );
    assert_eq!(
        gateway_origin("https://bücher.example", Origins::Public).as_deref(),
        Some("https://xn--bcher-kva.example")
    );
}

#[test]
fn anything_else_is_no_link() {
    for text in [
        "",
        "baylee:",
        "baylee://",
        "baylee://OPEN",
        "https://open",
        "baylee://open?x=1",
        "baylee://open/more",
        "baylee://open#top",
        "baylee://signin?username=someone",
        "baylee://user@open",
        "baylee://open:1",
        "baylee://gateway/add",
        "baylee://gateway/add/",
        "baylee://gateway/remove?origin=https%3A%2F%2Fa.example",
        "baylee://gateway/add?origin=",
        "baylee://gateway/add?url=https%3A%2F%2Fa.example",
        "baylee://gateway/add?origin=https%3A%2F%2Fa.example&name=Official",
        "baylee://gateway/add?origin=https%3A%2F%2Fa.example&origin=https%3A%2F%2Fb.example",
        "baylee://gateway/add?name=x&origin=https%3A%2F%2Fa.example",
        "baylee://gateway/add?origin=https%3A%2F%2Fa.example#x",
        "baylee://gateway/add?origin=https%3A%2F%2Fa.example\u{e9}",
        " baylee://open",
        "baylee://open\n",
        "baylee://sign\tin",
    ] {
        assert_eq!(read(text, Origins::AlsoLoopback), None, "{text}");
    }
    let long = format!(
        "baylee://gateway/add?origin=https%3A%2F%2F{}.example",
        "a".repeat(MAX_LEN)
    );
    assert_eq!(read(&long, Origins::Public), None);
}

#[test]
fn a_gateway_link_names_a_public_https_name_and_nothing_more() {
    for origin in [
        "http://gateway.example.org",
        "https://gateway.example.org/base",
        "https://gateway.example.org/?q=1",
        "https://gateway.example.org/#x",
        "https://user:pass@gateway.example.org",
        "https://user@gateway.example.org",
        "https://203.0.113.7",
        "https://[2001:db8::1]",
        "https://localhost",
        "https://printer",
        "https://gateway.example.org.",
        "ftp://gateway.example.org",
        "javascript:alert(1)",
        "file:///etc/passwd",
        "http://127.0.0.1:28766",
    ] {
        assert_eq!(gateway_origin(origin, Origins::Public), None, "{origin}");
    }
}

#[test]
fn a_dev_build_also_takes_a_numeric_loopback_gateway() {
    assert_eq!(
        gateway_origin("http://127.0.0.1:28766", Origins::AlsoLoopback).as_deref(),
        Some("http://127.0.0.1:28766")
    );
    assert_eq!(
        gateway_origin("http://[::1]:28766/", Origins::AlsoLoopback).as_deref(),
        Some("http://[::1]:28766")
    );
    for origin in ["http://localhost:28766", "http://192.168.1.5:28766"] {
        assert_eq!(
            gateway_origin(origin, Origins::AlsoLoopback),
            None,
            "{origin}"
        );
    }
}

#[test]
fn the_log_names_a_link_by_its_kind_only() {
    let link = read(
        "baylee://gateway/add?origin=https%3A%2F%2Fsecret-name.example",
        Origins::Public,
    )
    .expect("a link");
    assert!(!link.kind().contains("secret-name"));
    let link = read(&pair_text(&HANDLE, &SECRET), Origins::Public).expect("a link");
    assert_eq!(link.kind(), "a pairing link");
}

const HANDLE: [u8; 32] = [0xa5; 32];
/// Two `0xff` at a group's end write `_`, which only URL-safe base64 has.
const SECRET: [u8; 32] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 0xff, 0xff, 31, 250,
];

/// The link as the page writes it.
fn pair_text(handle: &[u8; 32], secret: &[u8; 32]) -> String {
    let (h, s) = (b64(handle), b64(secret));
    format!("baylee://pair?h={h}&s={s}")
}

fn b64(bytes: &[u8]) -> String {
    baylee_seal::b64::url::encode(bytes)
}

#[test]
fn a_pairing_link_is_read_exactly_as_the_page_writes_it() {
    let text = pair_text(&HANDLE, &SECRET);
    assert_eq!(
        text.len(),
        "baylee://pair?h=&s=".len() + 2 * PAIR_VALUE_CHARS
    );
    let Some(Link::Pair(link)) = read(&text, Origins::Public) else {
        panic!("a pairing link");
    };
    assert_eq!(link.handle(), &HANDLE);
    assert_eq!(link.secret(), &SECRET);
    assert_eq!(link, PairLink::new(HANDLE, SECRET));
    // As the other links: a root path and the scheme in any case.
    let (h, s) = (b64(&HANDLE), b64(&SECRET));
    for text in [
        format!("baylee://pair/?h={h}&s={s}"),
        format!("Baylee://pair?h={h}&s={s}"),
    ] {
        assert_eq!(
            read(&text, Origins::Public),
            Some(Link::Pair(PairLink::new(HANDLE, SECRET))),
            "{text}"
        );
    }
}

#[test]
fn a_pairing_link_takes_its_two_values_spelled_one_way_only() {
    let (h, s) = (b64(&HANDLE), b64(&SECRET));
    // The last character of a 32-byte value carries two bits past the
    // last byte, which the encoder leaves clear.
    let mut loose = s.clone();
    loose.pop();
    loose.push('B');
    let percent = format!("%{:02X}{}", s.as_bytes()[0], &s[1..]);
    let standard = s.replace('-', "+").replace('_', "/");
    for text in [
        format!("baylee://pair?s={s}&h={h}"),
        format!("baylee://pair?h={h}"),
        format!("baylee://pair?s={s}"),
        format!("baylee://pair?h={h}&s={s}&x=1"),
        format!("baylee://pair?h={h}&s={s}&s={s}"),
        format!("baylee://pair?h={h}&h={h}&s={s}"),
        format!("baylee://pair?x=1&h={h}&s={s}"),
        format!("baylee://pair?H={h}&S={s}"),
        format!("baylee://pair?h={h}&s={s}="),
        format!("baylee://pair?h={h}=&s={s}"),
        format!("baylee://pair?h={h}&s={}", &s[1..]),
        format!("baylee://pair?h={}&s={s}", &h[1..]),
        format!("baylee://pair?h={h}&s={s}A"),
        format!("baylee://pair?h={h}&s={loose}"),
        format!("baylee://pair?h={h}&s={percent}"),
        format!("baylee://pair?h={h}&s={standard}"),
        format!("baylee://pair?h=&s={s}"),
        format!("baylee://pair?h={h}&s="),
        format!("baylee://pair?h={h}&s={s}#x"),
        format!("baylee://pair/x?h={h}&s={s}"),
        format!("baylee://PAIR?h={h}&s={s}"),
        format!("baylee://user@pair?h={h}&s={s}"),
        format!("baylee://pair:1?h={h}&s={s}"),
        format!("baylee://pair?h={h}%26s={s}"),
        format!("baylee://pair?h= {h}&s={s}"),
        // A URL parser strips tabs and line breaks and trims spaces;
        // the reader takes none of them.
        format!(" baylee://pair?h={h}&s={s}"),
        format!("baylee://pair?h={h}&s={s} "),
        format!("baylee://pair?h={h}\n&s={s}"),
        format!("baylee://pair?h={}\t{}&s={s}", &h[..20], &h[20..]),
        format!("baylee://pair?h={h}&s={s}\r\n"),
        format!("baylee://open?h={h}&s={s}"),
        "baylee://pair".to_string(),
        "baylee://pair?".to_string(),
    ] {
        assert_eq!(read(&text, Origins::AlsoLoopback), None, "{text}");
    }
    assert_ne!(
        standard, s,
        "the sample has a character only URL-safe base64 writes"
    );
}

#[test]
fn a_pairing_link_never_prints_its_values() {
    let link = read(&pair_text(&HANDLE, &SECRET), Origins::Public).expect("a link");
    let printed = format!("{link:?}");
    assert_eq!(printed, "Pair(PairLink([REDACTED]))");
    for value in [b64(&HANDLE), b64(&SECRET)] {
        assert!(!printed.contains(&value));
    }
}

#[test]
fn ten_links_a_minute_are_read_and_the_rest_dropped() {
    let mut pace = Pace::default();
    let second = |s: u64| Duration::from_secs(s);
    for n in 0..10 {
        assert!(pace.admit(second(n)), "link {n}");
    }
    assert!(!pace.admit(second(10)));
    assert!(!pace.admit(second(59)));
    // The first one is a minute old: one more is read.
    assert!(pace.admit(second(60)));
    assert!(!pace.admit(second(60)));
}

/// Every byte string a link could be is answered without a panic, and
/// whatever is read as a gateway names an origin that reads again as
/// itself, and whatever is read as a pairing is the one spelling of its
/// values (a deterministic sweep standing in for a fuzzer).
#[test]
fn no_text_upsets_the_reader() {
    let (h, s) = (b64(&HANDLE), b64(&SECRET));
    let pieces = [
        "pair",
        "?h=",
        "&s=",
        h.as_str(),
        s.as_str(),
        &s[..42],
        "baylee://",
        "gateway",
        "/add",
        "?origin=",
        "https%3A%2F%2F",
        "https://",
        "a.example",
        "%2F",
        "%00",
        "%",
        "&",
        "=",
        "#",
        "@",
        ":",
        "[",
        "]",
        "..",
        "%E2%80%AE",
        "open",
        "signin",
        "\\",
        " ",
        "+",
    ];
    let mut seed = 0x9e37_79b9_u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        usize::try_from(seed).expect("a u32 fits")
    };
    for _ in 0..20_000 {
        let mut text = String::new();
        for _ in 0..next() % 7 {
            text.push_str(pieces[next() % pieces.len()]);
        }
        match read(&text, Origins::AlsoLoopback) {
            Some(Link::AddGateway(origin)) => assert_eq!(
                gateway_origin(&origin, Origins::AlsoLoopback).as_deref(),
                Some(origin.as_str()),
                "{text}"
            ),
            Some(Link::Pair(link)) => {
                let written = pair_text(link.handle(), link.secret());
                assert!(text.ends_with(&written["baylee://".len()..]), "{text}");
            }
            _ => {}
        }
    }
}
