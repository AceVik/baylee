//! `cards/artifacts/equipment/mv_2/rogue_s_gloves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rogue_s_gloves() -> CardIndex {
    card_index("308d7868-49f9-47a3-a7b1-4b0332d610f1")
}

/// Rogue's Gloves: "Whenever equipped creature deals combat damage to a player, you may draw a card. Equip {2}"
#[test]
fn rogue_s_gloves_draws_a_card_on_combat_damage() {
    let p0 = PlayerId::new(0);
    let mut r = rig(rogue_s_gloves(), llanowar_elves(), 2);
    r.equip(2);
    let library_before = library_size(&r.engine, p0);
    assert_eq!(r.hand_size(), 0);
    r.swing_unblocked();
    assert_eq!(r.engine.state().players[1].life, 19, "the Elf hit for 1");
    assert_eq!(r.hand_size(), 1, "one card drawn");
    assert_eq!(library_size(&r.engine, p0), library_before - 1);
}

/// "You may": declining the question draws nothing.
#[test]
fn rogue_s_gloves_draws_nothing_when_you_decline() {
    let p0 = PlayerId::new(0);
    let mut r = rig(rogue_s_gloves(), llanowar_elves(), 2);
    r.equip(2);
    r.attack_with_host();
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = r.engine.pending().clone() else {
        unreachable!("pass_until stops on the question")
    };
    assert_eq!(player, p0, "its controller is asked");
    r.engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(r.engine.state().players[1].life, 19, "the damage was dealt");
    assert_eq!(r.hand_size(), 0, "nothing drawn");
}
