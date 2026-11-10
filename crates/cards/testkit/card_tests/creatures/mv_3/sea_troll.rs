//! Sea Troll — `{2}{U}` 2/1 Creature — Troll (Homelands)
//!
//! `{U}`: Regenerate this creature. Activate only if this creature blocked or was blocked by a blue
//! creature this turn.
//!
//! The card keeps the "was blocked by a blue creature" half; the "blocked a blue creature" half is
//! not in the DSL (`Filter::BlockedThisTurn` takes no inner filter), so the card is `Partial`.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sea_troll() -> CardIndex {
    card_index("feabe30a-3c28-43a1-b538-b0207758520c")
}

/// P0 attacks with Sea Troll (two Islands behind it); P1 blocks with `blocker`. Returns the engine
/// at the declare-blockers step, and the Troll.
fn troll_blocked_by(blocker: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sea_troll(), island(), island()])
        .battlefield(1, &[blocker])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let troll = on_battlefield(&engine, p0, sea_troll()).unwrap();
    let blocker_id = on_battlefield(&engine, p1, blocker).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troll, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker_id, troll)],
            },
        )
        .unwrap();
    (engine, troll)
}

#[test]
fn sea_troll_cannot_regenerate_before_it_has_been_blocked_by_a_blue_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[sea_troll(), island(), island()])
        .battlefield(1, &[phantom_monster()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let troll = on_battlefield(&engine, p0, sea_troll()).unwrap();

    // Before combat, Sea Troll has neither blocked nor been blocked by a blue creature this turn.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: troll,
                    ability_index: 0,
                },
            )
            .is_err(),
        "Sea Troll cannot activate its regeneration ability before it was blocked by a blue creature"
    );
}

#[test]
fn sea_troll_regenerates_after_it_was_blocked_by_a_blue_creature() {
    let p0 = PlayerId::new(0);
    // P1 blocks the Troll with the blue 3/3 Phantom Monster.
    let (mut engine, troll) = troll_blocked_by(phantom_monster());

    // Now in the declare-blockers step after being blocked by a blue creature, {U}: Regenerate.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: troll,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        engine.state().object(troll).unwrap().zone,
        Zone::Battlefield,
        "Sea Troll regenerated from lethal combat damage"
    );
}

/// The negative side of the gate: a blocker that is not blue does not open it.
#[test]
fn sea_troll_cannot_regenerate_after_it_was_blocked_by_a_nonblue_creature() {
    let p0 = PlayerId::new(0);
    // P1 blocks the Troll with a green Grizzly Bears.
    let (mut engine, troll) = troll_blocked_by(grizzly_bears());

    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: troll,
                    ability_index: 0,
                },
            )
            .is_err(),
        "a green blocker is not a blue creature, so Sea Troll cannot regenerate"
    );
}
