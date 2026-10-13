//! A wrap on the wire: URL-safe base64 carries it back unchanged.

mod common;

use baylee_seal::{Error, b64};
use common::pair;

#[test]
fn a_wrap_survives_url_safe_base64() {
    let k = pair().sealed.wraps[0].k;
    let text = b64::url::encode(&k);
    assert_eq!(text.len(), 64);
    assert_eq!(b64::url::decode_array::<48>(&text), Ok(k));
    assert_eq!(b64::url::decode_array::<32>(&text), Err(Error::Length));
}
