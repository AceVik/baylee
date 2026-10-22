//! `cards/instants/mv_5/desertion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn desertion() -> CardIndex {
    card_index("ee3d1f44-e0ca-4ce9-be76-4b675a115156")
}

#[test]
fn desertion_counters_target_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[desertion()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, grizzly_bears());
    let spell_id = on_stack(&engine, grizzly_bears()).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    cast_from_hand(&mut engine, p0, desertion());
    aim_at(&mut engine, p0, spell_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
