//! `cards/creatures/mv_1/resistance_fighter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn resistance_fighter() -> CardIndex {
    card_index("a95a2d1c-70a9-4eb2-ae30-a07fab26a9c6")
}

/// `Resistance Fighter` ({W}, 1/1 Creature — Human Soldier,
/// `Coverage::Implemented`):
/// `Sacrifice this creature: Prevent all combat damage target creature would
/// deal this turn.`
///
/// Only the target is stopped (the Hill Giant that attacks beside it still
/// hits), and only this turn (the Bears hit again on their controller's next
/// turn). The prevention is `PreventDamageFromIt`; there is no journal event
/// for a prevented hit, so the life totals are what the test reads.
#[test]
fn resistance_fighter_sacrifices_to_prevent_combat_damage_from_target_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[GRIZZLY_BEARS, HILL_GIANT])
        .hand(0, &[resistance_fighter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, resistance_fighter());
    pass_until(&mut engine, stack_is_empty);
    let fighter = on_battlefield(&engine, p0, resistance_fighter()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    // Advance to P1's main phase, sacrifice Resistance Fighter targeting Grizzly Bears, then let
    // both of P1's creatures attack unblocked.
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
                attackers: vec![(bear, Defender::Player(p0)), (giant, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(
        engine.state().players[0].life,
        17,
        "the Hill Giant's 3 damage is dealt, the targeted Bears' 2 are prevented"
    );

    // "This turn": on P1's next turn the Bears hit for 2 again.
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
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
    assert_eq!(engine.state().players[0].life, 15);
}
