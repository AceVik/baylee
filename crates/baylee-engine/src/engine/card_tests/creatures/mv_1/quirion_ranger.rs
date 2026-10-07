//! `cards/creatures/mv_1/quirion_ranger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quirion Ranger ({G}, 1/1): "Return a Forest you control to its owner's
/// hand: Untap target creature. Activate only once each turn."
///
/// **The Forest is tapped when it is returned, and that is the whole
/// card.** Tap the Forest for `{G}`, spend the land itself on the Ranger,
/// and the mana stays in the pool (CR 106.4) while the land goes to the
/// hand — so `cost_wizard::options` must *not* borrow the "untapped" that
/// CR 118.3 gives a tap cost. A menu of untapped Forests would offer this
/// player nothing at all.
///
/// **Two questions in the order CR 601.2 asks them**: the target at 601.2c,
/// the cost at 601.2h. Choosing what to bounce is not targeting (CR 115.1c
/// — only the word "target" targets), so it arrives as `ChooseCards` under
/// its own `ChoicePrompt::CostReturn` rather than as a second target.
///
/// **Two Forests, deliberately.** With one, the ability would stop being
/// offered because the cost had become unpayable, and the test would pass
/// without the activation limit existing at all. The second Forest is what
/// makes "not offered again" mean the printed sentence.
#[test]
// One card, two questions and two turns: cutting it in half would leave a
// second test that has to rebuild the board to ask the second half, which is
// what the 22 allows already in this file are for.
#[allow(clippy::too_many_lines)]
fn quirion_ranger_bounces_a_tapped_forest_and_only_once_a_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, forest())
        .battlefield(0, &[quirion_ranger(), forest(), forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ranger = on_battlefield(&engine, p0, quirion_ranger()).expect("the Ranger is on the table");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are with it");

    // Both lands and the mana creature: `mana_abilities` carries the CR 305.6
    // land shortcut and the Elf's {T} is a printed ability, and `tap_all_mana`
    // takes both lists (#159). What it leaves is the board this card was
    // printed for: everything tapped, and a Forest that is still a legal cost.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "two Forests and an Elf"
    );
    assert!(is_tapped(&engine, elves), "the Elf tapped for its own mana");

    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(ranger, 0))
        )
    };
    assert!(
        offered(&engine),
        "a tapped Forest still pays a cost that only returns it"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: ranger,
                ability_index: 0,
            },
        )
        .expect("two Forests on the battlefield, so the cost is payable");

    // CR 601.2c: the target, before anything is paid.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the untap asks which creature: {:?}", engine.pending())
    };
    assert!(
        options.contains(&elves) && options.contains(&ranger),
        "\"target creature\" names no controller and no state, so a tapped \
         Elf and the untapped Ranger itself are both legal: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: Vec::new(),
            },
        )
        .expect("the tapped Elf is a legal target");

    // CR 601.2h, and the one step of it the player takes.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Forest: {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one asked");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostReturn,
        "not `CostSacrifice`: the Forest named here comes back to a hand, \
         and a player shown \"which one are you giving up\" would decline"
    );
    assert_eq!((min, max), (1, 1), "one Forest, and exactly one");
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 2, "the board was dealt two of them");
    assert_eq!(
        options, forests,
        "both Forests are on the menu although both are tapped — CR 118.3's \
         \"untapped\" belongs to a tap cost and to no other: {options:?}"
    );
    assert!(
        !options.contains(&ranger) && !options.contains(&elves),
        "and nothing that is not a Forest: {options:?}"
    );

    let paid = forests[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![paid],
            },
        )
        .expect("a tapped Forest pays the cost");

    assert!(
        in_hand(&engine, p0, forest()).is_some(),
        "the cost is paid, so the Forest is in its owner's hand"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "and off the battlefield: a return is not a tap"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3,
        "the mana the Forest made is still in the pool (CR 106.4) — a land \
         that leaves takes nothing with it, which is the play this card was \
         printed for"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability is no mana ability (CR 605.1), so it uses the stack"
    );
    assert!(is_tapped(&engine, elves), "and nothing has untapped yet");

    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, elves), "the targeted Elf is untapped");
    assert!(
        !offered(&engine),
        "and the printed sentence takes the ability off the offer for the \
         rest of the turn — the second Forest is still standing, so the \
         cost is payable and the limit is the only thing refusing"
    );

    // The opponent's turn is a different turn.
    reach_their_main_phase(&mut engine, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    assert!(
        offered(&engine),
        "\"only once each turn\" is not \"only once each of your turns\""
    );
}
