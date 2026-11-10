//! `cards/artifacts/equipment/mv_2/goldvein_pick.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn goldvein_pick() -> CardIndex {
    card_index("c1624d10-8838-4af8-aea1-a96c0fe6fd6b")
}

/// Goldvein Pick: "Equipped creature gets +1/+1. Whenever equipped creature
/// deals combat damage to a player, create a Treasure token. Equip {1}".
#[test]
fn goldvein_pick_pumps_and_pays_a_treasure_for_combat_damage() {
    let mut r = rig(goldvein_pick(), llanowar_elves(), 1);
    r.equip(1);
    assert_eq!(pt(&r.engine, r.host), (2, 2), "+1/+1");
    assert!(
        tokens_of(&r.engine, PlayerId::new(0)).is_empty(),
        "no token yet"
    );
    r.swing_unblocked();
    assert_eq!(r.engine.state().players[1].life, 18, "2 combat damage");
    let tokens = tokens_of(&r.engine, PlayerId::new(0));
    assert_eq!(tokens.len(), 1, "one Treasure for the hit");
    assert!(
        subtypes_of(&r.engine, tokens[0])
            .contains(baylee_core::generated::subtypes::artifact::TREASURE)
    );
}

/// The trigger is the equipped creature's: a hit by the creature nobody
/// equipped makes nothing.
#[test]
fn goldvein_pick_ignores_damage_by_other_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut r = rig(goldvein_pick(), llanowar_elves(), 1);
    r.equip(1);
    pass_until(&mut r.engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let other = r.bystander;
    r.engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(other, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut r.engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(r.engine.state().players[1].life, 18, "the Piker hit for 2");
    assert!(tokens_of(&r.engine, p0).is_empty(), "but no Treasure");
}
