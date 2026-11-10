//! `cards/creatures/mv_3/stalking_assassin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stalking Assassin — {1}{U}{B} for a 1/1 Human Assassin whose whole card is
/// two activated abilities: "{3}{U}, {T}: Tap target creature" and
/// "{3}{B}, {T}: Destroy target tapped creature."
///
/// One Assassin can pay its own tap symbol only once a turn, so the board
/// carries two of them — and that is what makes the printed word "tapped"
/// readable from both sides at once: the black half offers the Elf the blue
/// half has just tapped *and* the spent Assassin standing beside it, while
/// declining the Elf left untapped across the table and the Assassin that has
/// not paid its {T} yet. None of that is in the card file; the filter and the
/// four-mana prices are the engine's answers, and the empty-pool read before
/// the mana says those prices are real rather than labels.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn stalking_assassin_taps_a_creature_and_then_destroys_it_with_the_other_half() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                stalking_assassin(),
                stalking_assassin(),
            ],
        )
        .battlefield(1, &[quiet_creature(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let assassins = all_on_battlefield(&engine, p0, stalking_assassin());
    assert_eq!(assassins.len(), 2, "one Assassin per printed line");
    let (tap_half, kill_half) = (assassins[0], assassins[1]);
    let elves = all_on_battlefield(&engine, p1, quiet_creature());
    assert_eq!(elves.len(), 2, "one Elf to be tapped, one to be left alone");
    let (doomed, bystander) = (elves[0], elves[1]);

    // CR 601.2h read from the other side: `legal.abilities` is filtered
    // through `can_afford`, and that reads the mana pool rather than the
    // untapped lands — so on an empty pool neither price is offered at all.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat with the Assassins holds it");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == tap_half),
        "eight lands stand untapped and not one mana floats, so nothing is \
         offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "four Islands and four Swamps, and no creature on this board makes mana"
    );

    // The blue half: {3}{U}, {T}: Tap target creature.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, index)| *src == tap_half && *index == 0)
        .expect("{3}{U} is four of the eight floating, so ability 0 is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the cost is payable out of the pool");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        player_options.is_empty(),
        "\"target creature\" names no player: {player_options:?}"
    );
    for creature in [doomed, bystander, tap_half, kill_half] {
        assert!(
            options.contains(&creature),
            "\"target creature\" is any creature, tapped or not and on either \
             side of the table: {options:?}"
        );
    }

    // CR 601.2c picks the target and CR 601.2h pays afterwards, so while this
    // question stands the mana is still floating and the Assassin is untapped.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "the cost is the last step of the activation"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    assert!(is_tapped(&engine, tap_half), "{{T}} is half the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{3}}{{U}} came out of the pool"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        is_tapped(&engine, doomed),
        "the targeted creature is tapped"
    );
    assert!(
        !is_tapped(&engine, bystander),
        "and the creature nobody named never moved"
    );

    // The black half: {3}{B}, {T}: Destroy target tapped creature.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{3}}{{B}} is still floating in this same main phase (CR 500.5)"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == tap_half),
        "the Assassin that already spent its {{T}} has none left to pay with: {:?}",
        legal.abilities
    );
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, index)| *src == kill_half && *index == 1)
        .expect("{3}{B} is four of the four left, so ability 1 is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("the cost is payable out of the pool");

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target tapped creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&doomed),
        "the Elf the blue half just tapped is the card's own play: {options:?}"
    );
    assert!(
        options.contains(&tap_half),
        "\"target tapped creature\" is not \"a creature an opponent controls\": \
         the spent Assassin is tapped too: {options:?}"
    );
    assert!(
        !options.contains(&bystander),
        "the Elf standing untapped beside it is no legal target: {options:?}"
    );
    assert!(
        !options.contains(&kill_half),
        "and neither is the Assassin about to pay the {{3}}{{B}}, which is \
         still untapped: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doomed],
            },
        )
        .expect("the tapped Elf was one of the options it enumerated");
    assert!(
        is_tapped(&engine, kill_half),
        "{{T}} is paid by the Assassin that activated the ability"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{3}}{{B}} came out of the pool"
    );

    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p1, quiet_creature()).is_some(),
        "\"Destroy target tapped creature\": the Elf is in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p1, quiet_creature()).len(),
        1,
        "and only the creature that was named: the bystander still stands"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, stalking_assassin()).len(),
        2,
        "neither Assassin was its own target"
    );
}
