#[allow(clippy::wildcard_imports)]
use super::*;

fn lilting_refrain() -> CardIndex {
    card_index("f91a71b7-9bc0-4609-a312-ffede52dc0ad")
}

#[test]
fn lilting_refrain_adds_verse_counter_and_sacrifices_to_counter_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[grizzly_bears()])
        .battlefield(0, &[lilting_refrain(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grizzly_bears());
    let spell_id = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .expect("spell on stack");

    activate(&mut engine, p0, lilting_refrain(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, spell_id);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}
