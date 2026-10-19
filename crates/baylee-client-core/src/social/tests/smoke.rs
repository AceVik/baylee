//! A first conversation through the fake gateway.

use super::fakegw::{Client, Gateway};

#[test]
fn alice_whispers_bob_and_bob_reads_it() {
    let mut gw = Gateway::default();
    gw.handles.insert("af03".into(), "Alice#af03".into());
    gw.handles.insert("0012".into(), "Bob#0012".into());
    gw.befriend("af03", "0012");
    let mut alice = Client::new(&mut gw, "af03", "Alice#af03", 1);
    let mut bob = Client::new(&mut gw, "0012", "Bob#0012", 2);
    assert!(alice.social.online(), "alice welcomed");
    assert!(bob.social.online(), "bob welcomed");
    alice.say(&mut gw, "/w Bob hello [@Bob#0012]");
    assert_eq!(alice.texts("dm:0012:af03"), ["hello [@Bob#0012]"]);
    bob.deliver(&mut gw);
    assert_eq!(bob.texts("dm:0012:af03"), ["hello [@Bob#0012]"]);
    assert!(
        gw.waiting(&bob.device()).is_empty(),
        "Bob acknowledged his inbox"
    );
    alice.deliver(&mut gw);
    assert_eq!(alice.texts("dm:0012:af03"), ["hello [@Bob#0012]"]);
    assert!(
        gw.slots.values().all(Vec::is_empty),
        "both inboxes acknowledged"
    );
}
