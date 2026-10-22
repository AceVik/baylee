//! Helm of Obedience — `{4}` Artifact (Alliances)
//!
//! (`Coverage::Partial`: iteratively milling until a creature card or X cards
//! are put into the graveyard, sacrificing this artifact to put one of those
//! creature cards onto the battlefield under your control, and restricting X
//! from being 0 are not supported; `{X}`, `{T}` causes target opponent to mill
//! X cards.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn helm_of_obedience() -> CardIndex {
    card_index("16cadebf-c484-41f8-9e38-5c2c528f5b54")
}

#[test]
fn helm_of_obedience_causes_target_opponent_to_mill_x_cards() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[helm_of_obedience(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let helm = on_battlefield(&engine, p0, helm_of_obedience()).unwrap();
    let p1_lib_before = engine.state().zones.list(ZoneLocation::Library(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: helm,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).len(),
        p1_lib_before - 3
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );
}
