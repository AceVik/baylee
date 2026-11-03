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

/// NEW-10: the prompt names every step a model may stop at by the name the
/// seat reads, and the default it states is the rail's (`MIND_STOPS`). It
/// used to give one example with neither `blockers` nor `damage` in it, and
/// no model stopped where a combat trick is cast.
#[test]
fn the_prompt_teaches_every_stop_and_the_default_rail() {
    use crate::wake::{MIND_STOPS, stop_id, stop_row};
    use baylee_client_core::automation::{RAIL_ROWS, RailSide};
    let system = prompt::SYSTEM;
    let taught = "from upkeep, draw, main1, combat_begin, attackers, blockers, damage, combat_end, \
                  main2, end_step";
    assert!(system.contains(taught), "{system}");
    for row in RAIL_ROWS.into_iter().filter(|row| row.grants_priority()) {
        assert!(taught.contains(stop_id(row)), "{row:?}");
        assert_eq!(stop_row(stop_id(row)), Some(row));
    }
    let side = |side: RailSide| -> Vec<&str> {
        MIND_STOPS
            .iter()
            .filter(|(s, _)| *s == side)
            .map(|&(_, row)| stop_id(row))
            .collect()
    };
    assert_eq!(side(RailSide::Mine), ["main1", "blockers", "main2"]);
    assert_eq!(
        side(RailSide::Theirs),
        ["attackers", "blockers", "end_step"]
    );
    assert!(
        system.contains(
            "Until you set stops they are your main1, blockers and main2, and their attackers, \
             blockers and end_step."
        ),
        "{system}"
    );
}
