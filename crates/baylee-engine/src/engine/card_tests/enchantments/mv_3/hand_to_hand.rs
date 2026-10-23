//! `cards/enchantments/mv_3/hand_to_hand.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn hand_to_hand() -> CardIndex {
    card_index("bf45f579-539c-4ffe-89df-4896a394158f")
}

/// Hand to Hand (`bf45f579-539c-4ffe-89df-4896a394158f`) — {2}{R} Enchantment (Partial):
/// "During combat, players can't cast instant spells or activate abilities that aren't mana abilities."
#[test]
fn hand_to_hand_prevents_players_from_casting_instants_during_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hand_to_hand(), forest(), index::GRIZZLY_BEARS])
        .hand(0, &[index::GIANT_GROWTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let growth = in_hand(&engine, P0, index::GIANT_GROWTH).unwrap();
    tap_all_mana(&mut engine, P0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected Priority");
    };
    assert!(legal.castable.contains(&growth));

    // Advance into combat; float new mana after the phase boundary.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    crate::engine::card_tests::legends_kit::float(&mut engine, P0, &[(ManaColor::Green, 1)]);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected Priority");
    };
    assert!(!legal.castable.contains(&growth));
    assert!(!legal.payable.contains(&growth));
}
