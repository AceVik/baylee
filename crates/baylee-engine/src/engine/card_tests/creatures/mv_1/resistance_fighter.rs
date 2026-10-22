//! `cards/creatures/mv_1/resistance_fighter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn resistance_fighter() -> CardIndex {
    card_index("a95a2d1c-70a9-4eb2-ae30-a07fab26a9c6")
}

/// `Resistance Fighter` ({W}, 1/1 Creature — Human Soldier,
/// `Coverage::Implemented`):
/// `Sacrifice this creature: Prevent all combat damage target creature would
/// deal this turn.`
#[test]
fn resistance_fighter_sacrifices_to_prevent_combat_damage_from_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[resistance_fighter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, resistance_fighter());
    pass_until(&mut engine, stack_is_empty);
    let fighter = on_battlefield(&engine, p0, resistance_fighter()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    // Advance to P1's main phase, sacrifice Resistance Fighter targeting Grizzly Bears, then let it attack unblocked.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, p0, resistance_fighter(), 0);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(fighter).unwrap().zone,
        Zone::Graveyard
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[0].life, 20);
}
