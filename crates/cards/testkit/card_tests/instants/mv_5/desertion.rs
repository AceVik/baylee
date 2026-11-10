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
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "Partial: the printed card would put the creature onto p0's battlefield instead"
    );
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}

/// "Counter target spell": a noncreature spell is countered as well, and it does not resolve.
#[test]
fn desertion_counters_a_noncreature_spell_and_it_does_not_resolve() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .battlefield(1, &[mountain()])
        .hand(0, &[desertion()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p0, desertion());
    aim_at(&mut engine, p0, bolt);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Bolt never resolved"
    );
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
    assert!(in_graveyard(&engine, p0, desertion()).is_some());
}
