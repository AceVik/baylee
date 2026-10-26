//! `cards/creatures/mv_4/saber_ants.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "9c3c9a22-e092-4845-b661-047590746498"
fn saber_ants() -> CardIndex {
    card_index("9c3c9a22-e092-4845-b661-047590746498")
}

/// `Saber Ants` (`Coverage::Generated`):
/// "Whenever this creature is dealt damage, you may create that many 1/1 green Insect creature tokens."
///
/// Verifies that when `Saber Ants` is dealt 2 damage by `Shock`, its triggered
/// ability creates two 1/1 green Insect creature tokens when accepted.
#[test]
fn saber_ants_creates_insect_tokens_equal_to_damage_dealt_to_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9612, mountain())
        .battlefield(0, &[saber_ants(), mountain()])
        .hand(0, &[index::SHOCK])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let ants = on_battlefield(&engine, p0, saber_ants()).expect("P0 Saber Ants");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::SHOCK);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ants],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let tokens = engine
        .state()
        .arena
        .iter()
        .filter(|(_, o)| o.zone == Zone::Battlefield && o.controller == p0 && o.card.is_none())
        .count();
    assert_eq!(
        tokens, 2,
        "Saber Ants creates two 1/1 green Insect tokens when dealt 2 damage"
    );
}
