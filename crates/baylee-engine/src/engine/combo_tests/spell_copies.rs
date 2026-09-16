//! A spell copied on the stack rather than a permanent copied on the battlefield: Storm of Saruman on the turn's second spell, Reflections of Littjara on a chosen type, and Emeritus of Woe's prepared cast, which is a real cast of a copy and therefore has to move the turn's counters, obey the copied spell's own timing (CR 307.1) and leave the Warlock unprepared. The rules being asked are that a copy is *put* on the stack and never cast (CR 707.10), that a copy of a permanent spell resolves into a token, and that a copy ceasing to exist leaves one card in the graveyard and not two (CR 704.5e). Two copy effects stand on one board where one would do, because a single enchantment copying its own copy is caught only by whatever stops a loop, and the per-turn counters are read before *and* after the spell that moves them, since a fix to one half passes a test that asks the other.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Storm of Saruman ("Whenever you cast your **second** spell each turn,
/// copy it") and two creature spells in one turn.
///
/// The count is the card, and it is read off a per-turn counter rather than
/// off the stack — so the two halves have to be asked in one game: the first
/// spell resolves alone, and the second arrives with a copy beside it. A
/// trigger that fired on every cast would pass the second assertion and fail
/// the first, which is why the board is measured between the two spells and
/// not only at the end.
#[test]
fn storm_of_saruman_copies_your_second_spell_and_not_your_first() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(101, forest())
        .battlefield(0, &[storm_of_saruman(), forest(), forest()])
        .hand(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        1,
        "the first spell of the turn is nobody's second"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the second spell arrives with a copy of itself beside it"
    );
    assert_eq!(
        permanents_of(&engine, p1, llanowar_elves()),
        0,
        "the copy arrives under the caster's control, not across the table"
    );
}

/// Reflections of Littjara ("Whenever you cast a spell of the chosen type,
/// copy that spell") with Storm of Saruman beside it, and one Elf cast into
/// both of them.
///
/// This is the rule that a copy is **put** on the stack rather than cast
/// (CR 707.10), and it is unreadable with one copy effect on the board: a
/// single enchantment copying its own copy would be caught only by whatever
/// stops a loop. Two of them make the arithmetic say it out loud. The Elf is
/// the turn's second spell, so exactly two triggers see it and exactly two
/// tokens arrive — while a copy that counted as a cast would be a spell of
/// the chosen type as well, and Reflections would answer its own answer.
#[test]
fn a_copy_is_put_on_the_stack_and_never_cast_again() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(102, forest())
        .battlefield(
            0,
            &[
                storm_of_saruman(),
                island(),
                island(),
                island(),
                island(),
                island(),
                forest(),
            ],
        )
        .hand(0, &[reflections_of_littjara(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The enchantment is the turn's first spell, so nothing copies it. Its
    // own question is asked as it enters, which is on resolution.
    cast_from_hand(&mut engine, p0, reflections_of_littjara());
    settle(&mut engine);
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!(
            "the enchantment names a creature type as it enters: {:?}",
            engine.pending()
        )
    };
    let elf = baylee_core::generated::subtypes::creature::ELF;
    assert!(
        options.contains(&elf),
        "Elf is a creature type: {options:?}"
    );
    engine
        .apply(player, PlayerAction::ChooseSubtype(elf))
        .unwrap();
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        0,
        "one spell so far, and it copied nothing"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the Elf, one copy from each enchantment — and none from the \
         copies, which were never cast"
    );
    assert_eq!(
        permanents_of(&engine, p1, llanowar_elves()),
        0,
        "none of it reached the other side of the table"
    );
}

/// Emeritus of Woe's prepared cast into an opponent's Esper Sentinel
/// ("whenever an opponent casts their **first** noncreature spell each turn,
/// draw a card unless that player pays {X}").
///
/// The card says "you may **cast** a copy of its spell", so a prepared cast
/// is a cast and the turn has to count it. It did not: `start_prepared_cast`
/// journalled `SpellCast` and left `per_turn.noncreature_spells` alone, and
/// the two halves of that are what this test asks in one game. The Sentinel
/// did not tax the tutor — and then taxed the Brainstorm after it, which is
/// the turn's *second* noncreature spell and should have been free. A fix
/// that only stopped the second wrong would pass the last assertion here and
/// fail the first.
#[test]
fn the_sentinel_taxes_a_prepared_cast_and_leaves_the_spell_after_it_alone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(105, forest())
        .battlefield(0, &[emeritus_of_woe(), swamp(), swamp(), swamp(), island()])
        .hand(0, &[brainstorm()])
        .battlefield(1, &[esper_sentinel(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    let island = on_battlefield(&engine, p0, island()).expect("the Island");
    // The blue stays untapped on purpose: generic mana is paid in colour
    // order, so an Island in the pool would pay the tutor's {1} and leave
    // Brainstorm uncastable.
    tap_mana_except(&mut engine, p0, island);

    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    assert_eq!(
        cast,
        crate::choice::PREPARED_CAST,
        "the prepared cast is the synthetic index, not a printed ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();

    advance_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop above stopped on the tax")
    };
    assert_eq!(player, p0, "the tax is asked of whoever cast the spell");
    assert_eq!(mana, 1, "an unequipped Sentinel taxes {{1}}");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    // The tutor is still on the stack; Brainstorm is an instant and joins it.
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the trigger resolved and the turn's player has priority again: {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, p0, brainstorm());
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the tutor and the Brainstorm are both waiting"
    );
    assert_eq!(
        abilities_on_the_stack(&engine),
        0,
        "the Sentinel already had its first noncreature spell this turn"
    );
}

/// The same prepared cast, counted by Storm of Saruman instead ("whenever
/// you cast your **second** spell each turn, copy it").
///
/// This is the other counter the prepared cast walked past —
/// `per_turn.spells_cast` — and it needs its own game, because a spell can
/// be a creature spell and still be somebody's second. An Elf from hand is
/// the first, the tutor is the second, and the copy beside it is the whole
/// assertion: with the counter unbumped the trigger looks at a count of one
/// and never fires.
#[test]
fn a_prepared_cast_is_the_second_spell_saruman_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                storm_of_saruman(),
                emeritus_of_woe(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The turn's first spell. `cast_from_hand` taps everything, so the two
    // Swamps are floating for the prepared cast that follows — a pool only
    // empties at the end of a step, and this all happens in one main phase.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        1,
        "the first spell of the turn is nobody's second"
    );

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 2);
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the tutor is the turn's second spell, so a copy stands beside it"
    );
    assert_eq!(
        spells_on_the_stack(&engine, p1),
        0,
        "the copy is the caster's, not the other seat's"
    );
}

/// The same prepared cast, all the way through: the linked spell resolves,
/// the tutor actually searches, and the Warlock is no longer prepared.
///
/// The two tests above both stop with the tutor still on the stack, which is
/// exactly where two further defects were hiding. A fresh object starts in
/// its owner's library and `Zones::insert` does not say otherwise, so the
/// spell resolved *out of the library*: its id stayed on the stack and in
/// `stack_projectable`, which the very next `refresh_characteristics` reports
/// as drift. And `resolve_stack_top` reads a spell's effects off
/// `GameObject::card`, which `new_bare` leaves `None`, so the tutor resolved
/// to nothing at all — both seats passed and no library was ever searched.
#[test]
fn a_prepared_cast_resolves_and_unprepares_the_warlock() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(107, forest())
        .battlefield(0, &[emeritus_of_woe(), swamp(), swamp(), swamp()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    tap_mana_except(&mut engine, p0, emeritus);
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .len();

    let cast = offered_ability(&engine, emeritus).expect("the prepared cast is offered");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: emeritus,
                ability_index: cast,
            },
        )
        .unwrap();

    // Both seats pass, the tutor resolves, and the search it asks for is
    // answered on the way through by `advance_until`.
    advance_until(&mut engine, |e| {
        e.state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len()
            > hand_before
    });

    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Library(p0))
            .len(),
        library_before - 1,
        "the tutored card left the library"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .is_empty(),
        "the spell left the stack rather than being destroyed underneath it"
    );
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        0,
        "and nothing of it is still standing there"
    );

    let riders = &engine.state().object(emeritus).expect("the Warlock").riders;
    assert!(
        !riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Prepared)),
        "casting the prepared spell is what unprepares the card that held it"
    );
    assert!(
        offered_ability(&engine, emeritus).is_none(),
        "so it is not offered a second time"
    );
    // CR 704.5e: a copy of a spell in a zone other than the stack ceases to
    // exist. The Warlock's spell is a copy of a card nobody put in a deck,
    // so a graveyard is the one place it must never reach — a Demonic Tutor
    // sitting there is a card that could be flashed back, delved away or
    // counted by a threshold, and it was never in the game.
    assert_eq!(
        in_graveyard(&engine, p0, demonic_tutor()),
        0,
        "the copy ceased to exist rather than becoming a card in a graveyard"
    );
}

/// The other half of CR 704.5e, at the other place a spell copy is made:
/// `Effect::CopyTargetSpell`.
///
/// Storm of Saruman copies the turn's second spell, so a Brainstorm cast
/// second resolves twice — and exactly *one* Brainstorm may be in the
/// graveyard afterwards. The copy going there instead of ceasing to exist
/// is the same defect the prepared cast had, at the site the prepared cast
/// was modelled on.
#[test]
fn a_copied_spell_leaves_one_card_in_the_graveyard_not_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, island())
        .battlefield(
            0,
            &[storm_of_saruman(), forest(), island(), island(), island()],
        )
        .hand(0, &[llanowar_elves(), brainstorm()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The turn's first spell, so the Brainstorm after it is the second.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    cast_from_hand(&mut engine, p0, brainstorm());
    // The copy is made by a trigger, so it is one resolution away.
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 2);
    assert_eq!(
        spells_on_the_stack(&engine, p0),
        2,
        "the copy stands beside the spell it was made from"
    );

    // Both resolve, each asking which two cards go back on the library.
    advance_until(&mut engine, |e| spells_on_the_stack(e, p0) == 0);
    assert_eq!(
        in_graveyard(&engine, p0, brainstorm()),
        1,
        "the copy ceased to exist; only the card that was cast is a card"
    );
}

/// CR 707.10: a copy of a **permanent** spell becomes a token as it
/// resolves — the sentence Storm of Saruman prints in its own reminder
/// text.
///
/// Two Elves cast in one turn puts three on the board, and until now all
/// three were cards: the copy resolved into a permanent still carrying the
/// card it was copied from, so `Filter::IsToken` said no, and a copy that
/// died left a second Llanowar Elves in the graveyard for anything reading
/// that zone to find. The board count is asserted beside the token count
/// on purpose — a copy that simply failed to arrive would pass the second
/// assertion on its own.
#[test]
fn the_copy_of_a_creature_spell_arrives_as_a_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(109, forest())
        .battlefield(0, &[storm_of_saruman(), forest(), forest()])
        .hand(0, &[llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        cardless_permanents(&engine, p0),
        0,
        "the first spell of the turn is copied by nothing"
    );

    cast_from_hand(&mut engine, p0, llanowar_elves());
    settle(&mut engine);
    assert_eq!(
        permanents_of(&engine, p0, llanowar_elves()),
        3,
        "the second spell arrives with a copy of itself beside it"
    );
    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "and exactly one of the three is the token the copy became"
    );
    assert_eq!(
        in_graveyard(&engine, p0, llanowar_elves()),
        0,
        "nothing was a card that should not have been one"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "the token is the caster's, not the other seat's"
    );
}

/// Emeritus of Woe's prepared cast, offered on the opponent's turn.
///
/// "You may cast a copy of its spell" is a *cast*, so it obeys the timing of
/// the spell it copies, and Emeritus of Woe's spell is Demonic Tutor — a
/// sorcery, castable during a main phase of that player's own turn with the
/// stack empty (CR 307.1, the special case of CR 117.1a's "a noninstant
/// spell during their main phase"). Nothing on the card lifts that: an
/// effect that meant a copy could be cast at any time would print the
/// permission, the way "as though it had flash" does.
///
/// The offer asked one question — can the linked spell's mana cost be paid —
/// and no others, so a prepared Warlock was a Demonic Tutor at instant speed
/// on anybody's turn. Only Emeritus of Woe reaches this today, and its spell
/// is a sorcery, so the fault is the whole of the mechanic in this pool.
///
/// The first half of the test is what keeps the second honest: a prepared
/// cast that was never offered at all would satisfy the assertion below on
/// its own.
#[test]
fn a_prepared_cast_copies_a_sorcery_and_waits_for_a_sorcery_moment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(207, forest())
        .battlefield(0, &[emeritus_of_woe(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let emeritus = on_battlefield(&engine, p0, emeritus_of_woe()).expect("the Warlock");
    // Demonic Tutor is {1}{B}; two Swamps pay it and two stay untapped for
    // the other half of the test, on the far side of an untap step that
    // will not come round to this seat.
    tap_mana_count(&mut engine, p0, 2);
    assert_eq!(
        offered_ability(&engine, emeritus),
        Some(crate::choice::PREPARED_CAST),
        "in their own main phase, with the stack empty, the prepared cast is theirs to make"
    );

    // Their turn, and this seat holding priority in it. The mana floated
    // above is gone (CR 500.4), so the two Swamps left standing are what
    // pays for the attempt.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    tap_mana_count(&mut engine, p0, 2);
    assert_eq!(
        offered_ability(&engine, emeritus),
        None,
        "a sorcery cannot be cast on their turn, and a copy of one is still a sorcery"
    );
    // And the other probe agrees. A client naming the action out of a stale
    // offer is refused rather than handed the tutor: the offer and the
    // activation asking different questions about the same permanent is the
    // shape this engine treats as worse than either answer alone.
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: emeritus,
                    ability_index: crate::choice::PREPARED_CAST,
                },
            )
            .is_err(),
        "naming the prepared cast anyway is refused"
    );
}
