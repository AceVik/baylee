//! `cards/creatures/mv_2/ondu_cleric.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn rebound_does_not_follow_a_card_out_of_exile_and_back() {
    let p0 = PlayerId::new(0);
    let ephemerate = card_index("0fd57894-b917-41c8-a394-360d1d31b236");
    let mut engine = Duel::new(21, forest())
        .battlefield(0, &[plains(), ondu_cleric()])
        .hand(0, &[ephemerate])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cleric = on_battlefield(&engine, p0, ondu_cleric()).unwrap();
    let original = in_hand(&engine, p0, ephemerate).unwrap();
    cast_from_hand(&mut engine, p0, ephemerate);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![cleric],
            },
        )
        .unwrap();
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    let delayed = engine.state.delayed.remove(0).action;
    engine
        .state
        .move_object(
            original,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            original,
            ZoneLocation::Exile(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.delayed_queue.push_back((p0, delayed));
    assert!(
        !engine.process_delayed(),
        "CR 400.7: the returning card is a new object"
    );
    assert!(engine.cast_wizard.is_none());
}

/// Dualcaster Mage ({1}{R}{R}, 2/2): "Flash. When this creature enters, copy
/// target instant or sorcery spell. You may choose new targets for the copy."
///
/// The two printed sentences only mean anything together, so they are played
/// together. `casting::timing_allows` lets a creature spell be cast only in
/// its controller's main phase with an **empty** stack, so a board with a
/// spell standing on it is exactly the one a creature without flash cannot be
/// cast onto — and it is also the only board on which the trigger has a legal
/// target at all. Flash is not decoration here; it is what reaches the rest of
/// the card.
///
/// The trigger is the first in the pool that goes and *picks* a spell: every
/// other card that reaches `Effect::CopyTargetSpell` from a trigger copies the
/// spell that caused it (`TargetSpec::EventObject`). What comes back is p0's
/// copy of p1's removal, and it is pointed somewhere new — one Swords to
/// Plowshares exiles two creatures, and the second one belongs to the player
/// who cast it.
#[test]
fn a_flashed_in_mage_copies_the_opponents_removal_and_points_it_back_at_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[dualcaster_mage()])
        .battlefield(1, &[plains(), ondu_cleric()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    let my_elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the elves are out");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("the cleric is out");

    // p0's main phase, and p0 hands priority straight over: the opponent's
    // instant is cast at p0's only creature.
    reach_main_phase(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, swords_to_plowshares());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the swords' target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&my_elves), "the elves are targetable");
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![my_elves],
            },
        )
        .unwrap();

    // Back to p0 with the swords — the only thing on the stack — still on it.
    pass_until(&mut engine, |e| {
        !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let swords_spell = engine.state().zones.list(crate::zone::ZoneLocation::Stack)[0];

    // Flash. The mana goes first because `castable` is an affordability answer
    // too, and an untapped board would hide the timing question behind a price.
    tap_all_mana_but(&mut engine, p0, None);
    let mage = in_hand(&engine, p0, dualcaster_mage()).expect("the mage is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected p0's priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&mage),
        "flash offers a creature spell onto a stack that is not empty"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage })
        .unwrap();

    // The mage resolves, enters, and its trigger goes looking for a spell.
    let offered = options_offered_including(&mut engine, swords_spell);
    assert_eq!(
        offered.len(),
        1,
        "the one instant still on the stack is the only thing to copy: {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_spell],
            },
        )
        .unwrap();

    // The copy is made under p0's control, and p0 is asked again where it
    // points; an empty answer keeps the original target.
    let retarget = options_offered_including(&mut engine, their_cleric);
    assert!(
        !retarget.contains(&my_elves) && retarget.contains(&their_cleric),
        "the copy may be pointed at any legal creature: {retarget:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_cleric],
            },
        )
        .unwrap();

    // One card, two creatures exiled — and the second is the caster's own.
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, ondu_cleric()).is_none()
            && on_battlefield(e, p0, llanowar_elves()).is_none()
    });
    assert!(
        on_battlefield(&engine, p0, dualcaster_mage()).is_some(),
        "the flashed-in mage is a permanent and stays where it landed"
    );
    assert!(
        in_graveyard(&engine, p1, ondu_cleric()).is_none(),
        "the copy exiled its new target, the way the card it copied reads"
    );
    assert!(
        in_graveyard(&engine, p1, swords_to_plowshares()).is_some()
            && in_graveyard(&engine, p0, swords_to_plowshares()).is_none(),
        "one card was cast and exactly one card lies in a graveyard: a copy \
         was never a card and leaves none behind"
    );
}

/// Viscera Seer ({B}, 1/1 Vampire Wizard): "Sacrifice a creature: Scry 1."
///
/// The cost is the whole ability, so nothing here can be read off the card.
/// The printed sentence names no creature, and what a player is handed is a
/// question — which one — asked between the targets there are none of
/// (CR 601.2c) and the payment (CR 601.2h). It arrives as
/// `Pending::ChooseCards` with `ChoicePrompt::CostSacrifice`, because
/// choosing what to sacrifice is not targeting (CR 115.1), and the variant
/// is asserted beside the options: a list alone passes against
/// `ChoicePrompt::Generic`, which tells a client to say "choose a card"
/// where what the game means is "which one are you giving up".
///
/// Both halves of that menu are struck. It holds the Elf **and the Seer
/// herself**, who is a creature her controller controls and so is her own
/// fodder; it holds neither the opponent's Cleric — CR 701.21a only lets a
/// player sacrifice a permanent they control — nor the Swamp beside her,
/// which is not a creature. Naming the Cleric anyway is refused, so the
/// list is the enumeration `apply` validates against and not a hint.
///
/// She is eaten from on the turn she was cast, which the cost allows:
/// CR 302.6 holds back an ability with `{T}` in its cost and this one has
/// none, so summoning sickness is not what decides who may be sacrificed.
///
/// Then the two things paying it does. The Elf lies in the graveyard while
/// the ability is still on the stack — a cost is paid on activation, not on
/// resolution — and the scry that follows *moves* a card rather than merely
/// asking a question: the card looked at is on the bottom afterwards, the
/// one beneath it is the new top, and the library is the length it was,
/// because scry reorders and draws nothing.
///
/// The second activation is the case the printed sentence hides. Her own
/// body is all that is left on the menu, the cost takes her off the
/// battlefield before the ability resolves, and the ability resolves anyway
/// — it exists on the stack independently of its source (CR 113.7a). That
/// scry bottoms nothing, which is the other half of "you may put that card
/// on the bottom": the card that was on top is still on top.
#[allow(clippy::too_many_lines)] // one game, two activations, both scries
#[test]
fn viscera_seer_eats_the_elf_then_herself_and_scries_for_each() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(23, swamp())
        .battlefield(0, &[swamp(), llanowar_elves()])
        .hand(0, &[viscera_seer()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    // Cast, not seeded: one Swamp pays the {B} the printing asks for.
    cast_from_hand(&mut engine, p0, viscera_seer());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, viscera_seer()).is_some()
    });
    let seer = on_battlefield(&engine, p0, viscera_seer()).expect("the Seer resolved");
    assert_eq!(pt(&engine, seer), (1, 1), "the printed body arrived");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the fodder stands");
    let land = on_battlefield(&engine, p0, swamp()).expect("the Swamp that paid for her");
    let theirs = on_battlefield(&engine, p1, ondu_cleric()).expect("the opponent has a creature");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    let offered = deeds(&legal, &[seer]);
    assert!(
        matches!(offered.as_slice(), [(0, Deed::Ability(0))]),
        "her one printed ability is offered, because something on the board \
         can pay for it: {offered:?}"
    );
    let def = baylee_cards::by_index(viscera_seer()).expect("the Seer is in the pool");
    assert!(
        def.is_implemented(),
        "and the card says the same to the deckbuilder, which is the promise \
         the rest of this test is the evidence for"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: seer,
                ability_index: 0,
            },
        )
        .expect("the ability the offer just named");

    // CR 601.2h, and the question this card is written about.
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("paying asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "her controller decides what she eats");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "the question is part of a cost, and the variant is the only thing \
         that says so — it is not a search and it is not a target"
    );
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature exactly: the cost is neither optional nor a pile"
    );
    assert!(
        options.contains(&elf),
        "the Elf is a creature her controller controls: {options:?}"
    );
    assert!(
        options.contains(&seer),
        "and so is the Seer, so she is on her own menu (CR 701.21a): \
         {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "the opponent's Cleric is a creature and is not this player's to \
         sacrifice (CR 701.21a): {options:?}"
    );
    assert!(
        !options.contains(&land),
        "and a Swamp is not a creature at all: {options:?}"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err(),
        "the list is the enumeration the answer is validated against, so a \
         creature it never held cannot be eaten by naming it"
    );

    // The cards the scry is about to look at, read after the answer but
    // before the ability resolves. The list's last entry is the top of the
    // library and its first is the bottom.
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("one of the two creatures just offered");
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "the Elf left the battlefield the moment the cost was paid"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "and is in its owner's graveyard while the ability is still on the \
         stack: a cost is paid on activation, not on resolution"
    );
    assert!(
        !stack_is_empty(&engine),
        "the ability it paid for is waiting to resolve"
    );

    let library_before = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library_before.last().expect("p0 has a library");
    let second = library_before[library_before.len() - 2];

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange {
        cards,
        piles,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        prompt,
        crate::choice::ArrangePrompt::Scry,
        "the cost is behind her; this question is the effect she was paid for"
    );
    assert_eq!(cards, vec![top], "scry 1 looks at exactly the top card");
    assert_eq!(
        piles,
        scry_piles(1),
        "\"you **may** put that card on the bottom\""
    );
    engine
        .apply(p0, look_answer(&cards, &[top]))
        .expect("the card just looked at");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library.first().copied(),
        Some(top),
        "the card she looked at is on the bottom"
    );
    assert_eq!(
        library.last().copied(),
        Some(second),
        "the one beneath it is the new top"
    );
    assert_eq!(
        library.len(),
        library_before.len(),
        "and scry drew nothing on the way"
    );

    // Again, with nothing left to eat but herself.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("`at_rest` waited for exactly this")
    };
    let offered = deeds(&legal, &[seer]);
    assert!(
        matches!(offered.as_slice(), [(0, Deed::Ability(0))]),
        "she is still an outlet, now with her own body as the fodder: \
         {offered:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: seer,
                ability_index: 0,
            },
        )
        .expect("an outlet with one creature left is an outlet");
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("paying asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!(
        options,
        vec![seer],
        "the Elf is eaten and the Cleric is not hers, so her own body is the \
         whole menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![seer],
            },
        )
        .expect("a creature may be sacrificed to its own ability");
    assert!(
        on_battlefield(&engine, p0, viscera_seer()).is_none(),
        "she ate herself to pay for the ability"
    );
    assert!(
        in_graveyard(&engine, p0, viscera_seer()).is_some(),
        "and lies in the graveyard beside the Elf"
    );

    // CR 113.7a: the ability is on the stack and its source is gone, and it
    // still does what it says.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });
    let Pending::Arrange { cards, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(prompt, crate::choice::ArrangePrompt::Scry);
    assert_eq!(
        cards,
        vec![second],
        "the top card, which is the one the first scry left there"
    );
    engine
        .apply(p0, look_answer(&cards, &[]))
        .expect("bottoming nothing is an answer scry allows");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let library_after = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(
        library_after.last().copied(),
        Some(second),
        "nothing was chosen, so the card looked at stayed on top"
    );
    assert_eq!(
        library_after.len(),
        library.len(),
        "and this scry drew nothing either"
    );
    assert_eq!(
        on_battlefield(&engine, p1, ondu_cleric()),
        Some(theirs),
        "the opponent's creature was never on the menu and never left"
    );
}
