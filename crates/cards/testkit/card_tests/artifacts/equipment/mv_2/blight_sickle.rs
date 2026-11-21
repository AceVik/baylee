//! `cards/artifacts/equipment/mv_2/blight_sickle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn blight_sickle() -> CardIndex {
    card_index("5b95a5ed-a6b2-4044-9273-2021d27794e9")
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

/// Blight Sickle (5b95a5ed-a6b2-4044-9273-2021d27794e9) — {2} Artifact — Equipment:
/// "Equipped creature gets +1/+0 and has wither."
/// "Equip {2}"
///
/// Gives +1/+0 and wither: combat damage to a 4/4 blocker is dealt as three
/// -1/-1 counters (permanently shrinking it to 1/1 with 0 marked damage), while
/// unblocked combat damage to a player deals normal life loss.
#[test]
fn blight_sickle_grants_plus_one_zero_and_wither_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blight_sickle(), grizzly_bears(), forest(), forest()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sickle = on_battlefield(&engine, p0, blight_sickle()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let angel = on_battlefield(&engine, p1, serra_angel()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sickle,
                ability_index: 0,
            },
        )
        .expect("equip {2}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(pt(&engine, bears), (3, 2), "2/2 + 1/+0 = 3/2");
    assert!(keywords(&engine, bears).contains(KeywordSet::WITHER));

    // Attack into 4/4 Serra Angel: Bears dies, Angel takes 3 -1/-1 counters -> 1/1 (0 marked damage).
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

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(
        counters_on(&engine, angel, CounterKind::M1M1),
        3,
        "wither dealt 3 combat damage as three -1/-1 counters"
    );
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
    assert_eq!(pt(&engine, angel), (1, 1));
}

/// Wither still deals normal life loss to players (not poison counters).
#[test]
fn blight_sickle_deals_normal_combat_damage_to_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[blight_sickle(), grizzly_bears(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sickle = on_battlefield(&engine, p0, blight_sickle()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sickle,
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
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(
        life(&engine, p1),
        17,
        "3/2 with wither dealt 3 damage to player"
    );
}
