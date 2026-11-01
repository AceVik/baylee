//! `cards/enchantments/mv_3/heat_wave.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn heat_wave() -> CardIndex {
    card_index("d8f08699-6210-41c3-bfa0-c4b3bcbbaf1c")
}

fn knight_of_the_mists() -> CardIndex {
    card_index("e9893534-3d05-42c2-93ff-f28ffae89aa3")
}

/// `Heat Wave` ({2}{R}, Enchantment, `Coverage::Partial`) prevents blue
/// creatures from blocking creatures you control.
#[test]
fn heat_wave_prevents_blue_creatures_from_blocking_your_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), GRIZZLY_BEARS])
        .battlefield(1, &[knight_of_the_mists(), GRIZZLY_BEARS])
        .hand(0, &[heat_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, heat_wave());
    pass_until(&mut engine, stack_is_empty);
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let blue_blocker = on_battlefield(&engine, p1, knight_of_the_mists()).unwrap();
    let green_blocker = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
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
                    blockers: vec![(blue_blocker, bear)],
                },
            )
            .is_err(),
        "blue creature cannot block creatures controlled by Heat Wave's controller"
    );
    // Partial: the printed card lets a nonblue creature block only if its controller pays 1 life
    // for each blocker; the card as written lets it block for free.
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(green_blocker, bear)],
                },
            )
            .is_ok(),
        "a nonblue creature may block (the 1-life price is not played)"
    );
}

/// "Blue creatures can't block creatures *you* control": the controller's own blue creature
/// blocks the opponent's attacker as usual.
#[test]
fn heat_wave_lets_its_controllers_blue_creature_block_the_opponents_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), knight_of_the_mists()],
        )
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[heat_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, heat_wave());
    pass_until(&mut engine, stack_is_empty);
    let blue_knight = on_battlefield(&engine, p0, knight_of_the_mists()).unwrap();
    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blue_knight, bear)],
            },
        )
        .expect(
            "a blue creature may block a creature its controller does not share with Heat Wave",
        );
}

/// Passes until P0 is asked whether to pay the cumulative upkeep and says for how much.
#[track_caller]
fn walk_to_price(engine: &mut Engine<RegistryLookup>) -> baylee_core::mana::ManaCost {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, PlayerId::new(0), "the controller is asked");
    assert_eq!(engine.state().turn.active, player, "on its own upkeep");
    cost
}

/// "Cumulative upkeep {R}": {R} with one age counter, {R}{R} with two; refusing sacrifices it.
#[test]
fn heat_wave_cumulative_upkeep_grows_by_r_each_turn_and_sacrifices_when_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[heat_wave()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, heat_wave());
    pass_until(&mut engine, stack_is_empty);
    let wave = on_battlefield(&engine, p0, heat_wave()).unwrap();
    let age = baylee_cards_dsl::counters::AGE;

    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{R}"),
        "one counter, {{R}}"
    );
    assert_eq!(counters_on(&engine, wave, age), 1);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, heat_wave()).is_some());

    assert_eq!(
        walk_to_price(&mut engine),
        baylee_core::mana!("{R}{R}"),
        "two counters, {{R}}{{R}}"
    );
    assert_eq!(counters_on(&engine, wave, age), 2);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(wave).unwrap().zone,
        Zone::Graveyard,
        "the unpaid upkeep sacrificed it"
    );
}
