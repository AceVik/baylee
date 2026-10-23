//! `cards/creatures/mv_1/keeper_of_kookus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn keeper_of_kookus() -> CardIndex {
    card_index("97417270-e525-4261-b7d9-0dde3360e666")
}

fn goblin_swine_rider() -> CardIndex {
    card_index("9ebe32fc-3b6e-4cb9-9af0-1a6c3c6ad885")
}

/// `Keeper of Kookus` ({R}, 1/1 Goblin, `Coverage::Implemented`):
/// `{R}: This creature gains protection from red until end of turn.`
#[test]
fn keeper_of_kookus_gains_protection_from_red_until_end_of_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), keeper_of_kookus()])
        .battlefield(1, &[goblin_swine_rider(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    let red_blocker = on_battlefield(&engine, p1, goblin_swine_rider()).unwrap();
    let green_blocker = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, keeper_of_kookus(), 0);
    pass_until(&mut engine, stack_is_empty);

    // Protection from red prevents a red creature from blocking Keeper of Kookus.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(keeper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(red_blocker, keeper)],
                },
            )
            .is_err(),
        "red creature cannot block Keeper of Kookus while it has protection from red"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(green_blocker, keeper)],
                },
            )
            .is_ok(),
        "a creature that is not red may still block it"
    );
}

/// Without the activation there is no protection: the red creature blocks the Keeper.
#[test]
fn keeper_of_kookus_without_the_ability_can_be_blocked_by_a_red_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), keeper_of_kookus()])
        .battlefield(1, &[goblin_swine_rider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    let red_blocker = on_battlefield(&engine, p1, goblin_swine_rider()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(keeper, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(red_blocker, keeper)],
                },
            )
            .is_ok(),
        "no protection yet"
    );
}

/// P1 casts a Lightning Bolt at the Keeper and passes priority, so P0 may respond.
#[track_caller]
fn bolt_the_keeper(engine: &mut Engine<RegistryLookup>, keeper: ObjectId) {
    let p1 = PlayerId::new(1);
    cast_from_hand(engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![keeper],
                players: vec![],
            },
        )
        .unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
}

fn keeper_board() -> Engine<RegistryLookup> {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), keeper_of_kookus()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine
}

/// The canonical use: in response to a Lightning Bolt aimed at the Keeper, {R} for protection from
/// red makes the Bolt's target illegal and it does nothing.
#[test]
fn keeper_of_kookus_protection_fizzles_a_red_spell_aimed_at_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = keeper_board();
    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    bolt_the_keeper(&mut engine, keeper);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, keeper_of_kookus(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, keeper_of_kookus()).is_some(),
        "protection from red made the Bolt's target illegal"
    );
    assert!(in_graveyard(&engine, p1, lightning_bolt()).is_some());
}

/// The same Bolt with no response kills the 1/1, so the test above shows the ability at work.
#[test]
fn keeper_of_kookus_dies_to_a_bolt_when_it_does_not_respond() {
    let p0 = PlayerId::new(0);
    let mut engine = keeper_board();
    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    bolt_the_keeper(&mut engine, keeper);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, keeper_of_kookus()).is_some(),
        "no protection, no survivor"
    );
}

/// "Until end of turn": the protection is gone on the next turn, when a Bolt may target it again.
#[test]
fn keeper_of_kookus_protection_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), keeper_of_kookus()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let keeper = on_battlefield(&engine, p0, keeper_of_kookus()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, keeper_of_kookus(), 0);
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&keeper),
        "a red spell may target the Keeper again on the next turn"
    );
}
