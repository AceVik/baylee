//! `cards/creatures/mv_2/quickening_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn quickening_licid() -> CardIndex {
    card_index("2bdd9ca3-542a-43c0-9831-c6add23890d6")
}

/// `Quickening Licid` ({1}{W}, 1/1 Creature — Licid, `Coverage::Partial`):
/// activates `{1}{W}, {T}` to grant target creature first strike until end of
/// turn.
#[test]
fn quickening_licid_grants_target_creature_first_strike_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), quickening_licid(), GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, quickening_licid(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FIRST_STRIKE)
    );
}
