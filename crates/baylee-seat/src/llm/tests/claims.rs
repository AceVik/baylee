//! What the system prompt claims about the table, held against what the
//! seat does: a model reasons from these sentences, and one that is false
//! sends it to report bugs that are not there.

use super::*;
use baylee_core::ids::ObjectId;

/// NEW-16: a card keeps its handle across zones, and the engine bumps the
/// object's version instead (CR 400.7 is kept through the version: a target
/// names the version it was chosen at). The narrator names an object by its
/// slot, so the #N a model reads is the same before and after the move. The
/// prompt used to say the card got a new id, and models reported the kept
/// #N as a rules error.
#[test]
fn the_prompt_says_what_a_zone_change_does_to_an_id() {
    // The same slot in a later generation is the same #N.
    assert_eq!(
        crate::narrator::tag(ObjectId::new(31, 4)),
        crate::narrator::tag(ObjectId::new(31, 5))
    );
    let system = prompt::SYSTEM;
    assert!(
        system.contains("A card keeps its #N when it moves from one zone to another"),
        "{system}"
    );
    assert!(!system.contains("new id"), "{system}");
    // The words `Table::named_target` writes beside a target.
    for word in ["current", "earlier incarnation"] {
        assert!(system.contains(&format!("\"{word}\"")), "{system}");
    }
}
