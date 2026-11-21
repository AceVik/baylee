//! `cards/artifacts/equipment/mv_3/quietus_spike.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn quietus_spike() -> CardIndex {
    card_index("0ff0a309-9b4a-4f9e-9ea6-c4d07906ebec")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

/// Quietus Spike (0ff0a309-9b4a-4f9e-9ea6-c4d07906ebec) — {3} Artifact — Equipment:
/// "Equipped creature has deathtouch."
/// "Whenever equipped creature deals combat damage to a player, that player
/// loses half their life, rounded up."
/// "Equip {3}"
///
/// Deals odd-life combat damage to opponent: 20 -> 17 (from 3/3 Hill Giant) ->
/// loses ceil(17 / 2) = 9 life -> ends at 8 life!
#[test]
fn quietus_spike_halves_damaged_players_life_rounded_up() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[quietus_spike(), hill_giant(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spike = on_battlefield(&engine, p0, quietus_spike()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: spike,
                ability_index: 0,
            },
        )
        .expect("equip {3}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(keywords(&engine, giant).contains(KeywordSet::DEATHTOUCH));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    // 20 - 3 combat damage = 17; half of 17 rounded up = 9; 17 - 9 = 8.
    assert_eq!(
        life(&engine, p1),
        8,
        "20 - 3 = 17, then loses ceil(17/2) = 9 -> 8 life"
    );
}

/// Deathtouch kills a larger blocker (4/4 Serra Angel blocked against 2/2
/// Grizzly Bears), and combat damage to a creature does NOT trigger life loss.
#[test]
fn quietus_spike_deathtouch_kills_large_blocker_without_life_loss_trigger() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                quietus_spike(),
                grizzly_bears(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spike = on_battlefield(&engine, p0, quietus_spike()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: spike,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
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
                blockers: vec![(angel, bears)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        in_graveyard(&engine, p1, serra_angel()).is_some(),
        "deathtouch from Quietus Spike killed the 4/4 Serra Angel"
    );
    assert_eq!(
        life(&engine, p1),
        20,
        "blocked creature dealt no combat damage to player, so life loss did not trigger"
    );
}
