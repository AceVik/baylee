//! `cards/creatures/mv_5/orcish_squatters.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::{Phase, Step};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orcish_squatters() -> CardIndex {
    card_index("ce936736-915f-44bb-a709-b3bdcf90a3be")
}

fn declare_squatters_unblocked(engine: &mut Engine<RegistryLookup>, squatters: ObjectId) {
    pass_until(engine, |e| e.state().turn.step == Step::DeclareAttackers);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(squatters, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| e.state().turn.step == Step::DeclareBlockers);
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
}

/// Orcish Squatters (ce936736-915f-44bb-a709-b3bdcf90a3be) — {4}{R} 2/3 Creature — Orc:
/// "Whenever this creature attacks and isn't blocked, you may gain control of target land
/// defending player controls for as long as you control this creature. If you do, this
/// creature assigns no combat damage this turn."
#[test]
fn orcish_squatters_steals_defenders_land_for_as_long_as_you_control_it_or_deals_combat_damage() {
    // 1. Accepted ("Yes"): steals defending player's land (excluding P0's own land), assigns no combat
    // damage, and returns the land when Orcish Squatters leaves the battlefield.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[orcish_squatters(), mountain(), mountain()])
        .battlefield(1, &[forest()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);

    let squatters = on_battlefield(&engine, P0, orcish_squatters()).unwrap();
    let p0_mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    let p1_forest = on_battlefield(&engine, P1, forest()).unwrap();

    declare_squatters_unblocked(&mut engine, squatters);
    let options = aim_at(&mut engine, P0, p1_forest);
    assert!(
        !options.contains(&p0_mtn),
        "P0's own land is excluded by 'target land defending player controls'"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(p1_forest).unwrap().controller,
        P0,
        "P0 gains control of defending player's land"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "Orcish Squatters assigned no combat damage when the ability was accepted"
    );

    // Destroy Orcish Squatters with Lightning Bolt: control of the Forest returns to P1.
    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, squatters);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(p1_forest).unwrap().controller,
        P1,
        "control of the land ends when P0 no longer controls Orcish Squatters"
    );

    // 2. Declined ("No"): P1 keeps their land and Orcish Squatters deals its 2 combat damage.
    let mut engine2 = Duel::new(SEED, mountain())
        .battlefield(0, &[orcish_squatters()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine2);

    let squatters2 = on_battlefield(&engine2, P0, orcish_squatters()).unwrap();
    let p1_forest2 = on_battlefield(&engine2, P1, forest()).unwrap();

    declare_squatters_unblocked(&mut engine2, squatters2);
    aim_at(&mut engine2, P0, p1_forest2);
    pass_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine2.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine2, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine2.state().object(p1_forest2).unwrap().controller,
        P1,
        "declining the trigger leaves the land under P1's control"
    );
    assert_eq!(
        engine2.state().players[1].life,
        18,
        "declining the trigger lets Orcish Squatters deal its 2 combat damage"
    );
}

#[test]
fn orcish_squatters_does_nothing_when_it_is_blocked() {
    let mut engine = lk::game(
        SEED,
        &[orcish_squatters()],
        &[index::GRIZZLY_BEARS, index::FOREST],
        &[],
        &[],
    );
    let squatters = lk::obj(&engine, P0, orcish_squatters());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let their_forest = lk::obj(&engine, P1, index::FOREST);

    let offer = lk::attack(&mut engine, &[squatters]);
    assert!(lk::may_block(&offer, bears, squatters));
    lk::block(&mut engine, &[(bears, squatters)]);
    // A trigger that wrongly fired would stop here on its target question.
    lk::through_combat(&mut engine);
    assert_eq!(
        engine.state().object(their_forest).unwrap().controller,
        P1,
        "'attacks and isn't blocked': a blocked Orcish Squatters takes no land"
    );
}
