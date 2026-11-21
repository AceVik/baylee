//! `cards/instants/mv_4/spirit_flare.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "d19a2c28-8d09-40bf-aeb1-e699672b009f"
fn spirit_flare() -> CardIndex {
    card_index("d19a2c28-8d09-40bf-aeb1-e699672b009f")
}

/// Spirit Flare ({3}{W}): from hand, taps target untapped creature you control
/// (excluding tapped creatures) to deal its power to target attacking creature
/// an opponent controls (excluding non-attackers).
#[test]
fn spirit_flare_from_hand_taps_untapped_ally_to_damage_attacking_foe() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                index::HILL_GIANT,
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[spirit_flare()])
        .battlefield(1, &[index::HILL_GIANT, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let my_giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let my_elf = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let their_giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    let _ = lk::attack(&mut engine, &[their_giant]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    // Tap P0's Elf for mana so only Giant is an untapped controlled creature.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_elf,
                ability_index: 0,
            },
        )
        .unwrap();
    let plains_ids = all_on_battlefield(&engine, P0, plains());
    tap_mana_where(&mut engine, P0, |id| plains_ids.contains(&id));
    cast_with_floating(&mut engine, P0, spirit_flare());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected first target choice, got {:?}", engine.pending());
    };
    assert_eq!(options, vec![my_giant], "only untapped controlled creature");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![my_giant],
                players: vec![],
            },
        )
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected second target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&their_giant));
    assert!(
        !options.contains(&their_bears),
        "non-attacking opponent creature excluded: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![their_giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, my_giant));
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&engine, P0, spirit_flare()).is_some());
}

/// Flashback—{1}{W}, Pay 3 life: casts from graveyard targeting a BLOCKING
/// opponent creature, spends {1}{W} and 3 life, kills the blocker, and exiles
/// Spirit Flare; also tests "If you do" when the first target becomes tapped
/// in response.
#[test]
fn spirit_flare_flashback_hits_blocker_and_respects_if_you_do() {
    let mut engine = lk::game(
        SEED,
        &[index::SAVANNAH_LIONS, index::HILL_GIANT],
        &[index::GRIZZLY_BEARS],
        &[spirit_flare()],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let spell = in_hand(&engine, P0, spirit_flare()).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            spell,
            ZoneLocation::Graveyard(P0),
            baylee_engine::zone::ZonePosition::Top,
            baylee_engine::event::Cause::Effect,
        )
        .unwrap();

    let _ = lk::attack(&mut engine, &[lions]);
    lk::block(&mut engine, &[(bears, lions)]);

    lk::float(&mut engine, P0, &[(ManaColor::White, 2)]);
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("flashback castable for {1}{W} + 3 life");
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let fb = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine.apply(P0, PlayerAction::ChooseMode(fb)).unwrap();
    }
    lk::aim(&mut engine, &[giant], &[]);
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::life(&engine, P0), 17, "paid 3 life for flashback");
    assert!(is_tapped(&engine, giant));
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(P0))
            .contains(&spell),
        "flashed-back Spirit Flare was exiled"
    );

    // "If you do": tapping the targeted untapped creature in response makes
    // slot 1 illegal on resolution so it deals 0 damage to the attacker.
    let mut tapped_in_response = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                index::LLANOWAR_ELVES,
            ],
        )
        .hand(0, &[spirit_flare()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut tapped_in_response);
    reach_their_main_phase(&mut tapped_in_response, P1);
    let elf = lk::obj(&tapped_in_response, P0, index::LLANOWAR_ELVES);
    let attacker = lk::obj(&tapped_in_response, P1, index::GRIZZLY_BEARS);
    let _ = lk::attack(&mut tapped_in_response, &[attacker]);
    lk::block(&mut tapped_in_response, &[]);
    pass_until(
        &mut tapped_in_response,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let ps = all_on_battlefield(&tapped_in_response, P0, plains());
    tap_mana_where(&mut tapped_in_response, P0, |id| ps.contains(&id));
    cast_with_floating(&mut tapped_in_response, P0, spirit_flare());
    lk::aim(&mut tapped_in_response, &[elf], &[]);
    lk::aim(&mut tapped_in_response, &[attacker], &[]);
    // P0 taps Elf for mana in response before Spirit Flare resolves!
    tapped_in_response
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: elf,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut tapped_in_response, stack_is_empty);
    assert_eq!(
        lk::damage_on(&tapped_in_response, attacker),
        0,
        "first target became tapped in response, so \"If you do\" deals 0 damage"
    );
}
