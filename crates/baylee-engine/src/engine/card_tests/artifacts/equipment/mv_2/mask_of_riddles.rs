//! `cards/artifacts/equipment/mv_2/mask_of_riddles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn mask_of_riddles() -> CardIndex {
    card_index("a50c55a7-4e7e-40f6-b76c-1e32598dc9b6")
}

/// Mask of Riddles: "Whenever equipped creature deals combat damage to a player, you may draw a card. Equip {2}"
#[test]
fn mask_of_riddles_draws_a_card_on_combat_damage() {
    let p0 = PlayerId::new(0);
    let mut r = rig(mask_of_riddles(), llanowar_elves(), 2);
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
fn mask_of_riddles_draws_nothing_when_you_decline() {
    let p0 = PlayerId::new(0);
    let mut r = rig(mask_of_riddles(), llanowar_elves(), 2);
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

/// "Equipped creature has fear." Fear (CR 702.36) lets only artifact and/or
/// black creatures block it: of the green Elf and the artifact Wall across the
/// table, only the Wall is offered.
#[test]
fn mask_of_riddles_gives_fear() {
    let mut r = rig_vs(
        mask_of_riddles(),
        llanowar_elves(),
        2,
        &[],
        &[llanowar_elves(), steel_wall()],
        &[],
    );
    r.equip(2);
    assert!(keywords(&r.engine, r.host).contains(KeywordSet::FEAR));
    let wall = on_battlefield(&r.engine, PlayerId::new(1), steel_wall()).expect("their Wall");
    let Pending::ChooseBlockers { blockers, .. } = r.attack_to_the_blocks() else {
        unreachable!("to_the_blocks returns the blockers question")
    };
    let able: Vec<_> = blockers.iter().map(|b| b.blocker).collect();
    assert!(
        able.contains(&wall),
        "an artifact creature may block: {able:?}"
    );
    assert!(
        !able.contains(&r.theirs),
        "a green nonartifact creature may not: {able:?}"
    );
}
