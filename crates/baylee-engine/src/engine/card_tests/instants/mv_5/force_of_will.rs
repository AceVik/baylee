//! `cards/instants/mv_5/force_of_will.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn force_of_will() -> CardIndex {
    card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
}

#[test]
fn force_of_will_counters_target_spell_when_cast_for_mana() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[force_of_will()])
        .hand(1, &[grizzly_bears()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    let bears_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, force_of_will());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears_spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "Force of Will should counter the target spell"
    );
}
