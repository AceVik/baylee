//! `cards/artifacts/mv_6/obelisk_of_alara.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn obelisk_of_alara() -> CardIndex {
    card_index("75fcb9ce-ac01-4048-82b7-9100143e3b07")
}

fn elspeth_storm_slayer() -> CardIndex {
    card_index("f78af825-023a-42e9-8374-5c52303a1417")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn tap_land(engine: &mut Engine<RegistryLookup>, p0: PlayerId, card: CardIndex) {
    let land = on_battlefield(engine, p0, card).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
}

fn next_p0_main(engine: &mut Engine<RegistryLookup>, p0: PlayerId, p1: PlayerId) {
    pass_until(engine, |e| e.state().turn.active == p1);
    pass_until(engine, |e| e.state().turn.active == p0);
    reach_main_phase(engine, p0);
}

/// Obelisk of Alara (75fcb9ce-ac01-4048-82b7-9100143e3b07) — {6} Artifact:
/// "{1}{W}, {T}: You gain 5 life."
/// "{1}{U}, {T}: Draw a card, then discard a card."
/// "{1}{G}, {T}: Target creature gets +4/+4 until end of turn."
#[test]
fn obelisk_of_alara_white_blue_and_green_abilities() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                obelisk_of_alara(),
                plains(),
                island(),
                forest(),
                forest(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let obelisk = on_battlefield(&engine, p0, obelisk_of_alara()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    // 1) {1}{W}, {T} (ability 0): gain 5 life -> 25.
    tap_land(&mut engine, p0, plains());
    tap_land(&mut engine, p0, forest());
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 0,
            },
        )
        .expect("activate {1}{W} gain 5 life");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 25);

    // 2) Next turn: {1}{U}, {T} (ability 1): draw 1, then discard 1.
    next_p0_main(&mut engine, p0, p1);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_land(&mut engine, p0, island());
    tap_land(&mut engine, p0, forest());
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 1,
            },
        )
        .expect("activate {1}{U} loot");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards { .. } | Pending::DiscardChoice { .. }
        )
    });
    let to_discard = in_hand(&engine, p0, mountain()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![to_discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert!(in_graveyard(&engine, p0, mountain()).is_some());

    // 3) Next turn: {1}{G}, {T} (ability 4): target creature gets +4/+4 until EOT.
    next_p0_main(&mut engine, p0, p1);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 4,
            },
        )
        .expect("activate {1}{G} pump");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (6, 6));

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2), "+4/+4 wore off at end of turn");
}

/// "{1}{B}, {T}: Target creature gets -2/-2 until end of turn."
#[test]
fn obelisk_of_alara_black_shrinks_creature_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[obelisk_of_alara(), swamp(), forest()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let obelisk = on_battlefield(&engine, p0, obelisk_of_alara()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 2,
            },
        )
        .expect("activate {1}{B} -2/-2");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (1, 1), "3/3 - 2/-2 = 1/1");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, giant), (3, 3), "-2/-2 wore off at end of turn");
}

/// "{1}{R}, {T}: This artifact deals 3 damage to target player or planeswalker."
/// Rejects creatures as targets, damages planeswalkers, and damages players.
#[test]
fn obelisk_of_alara_red_damages_player_or_planeswalker_not_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[obelisk_of_alara(), mountain(), forest()])
        .battlefield(1, &[grizzly_bears(), elspeth_storm_slayer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let obelisk = on_battlefield(&engine, p0, obelisk_of_alara()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let elspeth = on_battlefield(&engine, p1, elspeth_storm_slayer()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 3,
            },
        )
        .expect("activate {1}{R} burn");
    let Pending::ChooseTargets {
        options: legal_objs,
        player_options: legal_players,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(legal_objs.contains(&elspeth));
    assert!(
        !legal_objs.contains(&bears),
        "creatures are NOT legal targets"
    );
    assert!(legal_players.contains(&p1));

    let loyalty_before = counters_on(&engine, elspeth, CounterKind::Loyalty);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elspeth],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, elspeth, CounterKind::Loyalty),
        loyalty_before - 3
    );

    next_p0_main(&mut engine, p0, p1);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: obelisk,
                ability_index: 3,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 17, "opponent took 3 damage");
}
