//! `cards/enchantments/mv_1/mystic_remora.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mystic Remora's tax trigger, fought on the *opponent's* turn so its
/// controller's cumulative upkeep never comes up — age counters go on at the
/// Remora's own controller's upkeep, and this game ends before that. The
/// upkeep is `mystic_remora_s_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid`.
///
/// Both words of the filter are struck as well as the sentence read: the Sol
/// Ring its own controller casts is a noncreature spell that costs nobody a
/// card, so "an opponent casts" is doing work, and the Dark Ritual it does
/// react to is that opponent's noncreature spell.
///
/// What is asked is asserted down to the number — `{4}` of the player who
/// cast, never the `{1}` the upkeep clause would have charged — and with the
/// tax declined the payment is a card: one off the top of, and one into the
/// hand of, the seat that controls the Remora.
#[test]
fn mystic_remora_taxes_an_opponents_noncreature_spell_and_draws_when_they_decline() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // Three Islands pay {U} for the Remora and leave the Ring's {1} behind
    // it; p1's five Swamps and a Forest cast the Ritual with {4} still
    // floating, and the Elves stay back as the creature spell the trigger
    // must not notice.
    let mut engine = Duel::new(87, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[mystic_remora(), quiet_artifact()])
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp(), swamp(), forest()])
        .hand(1, &[dark_ritual(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches a main phase");

    cast_from_hand(&mut engine, p0, mystic_remora());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, mystic_remora()).is_some(),
        "the Remora resolved and stands on p0's battlefield"
    );

    // "an opponent casts": a noncreature spell of the Remora's own
    // controller's is nothing to it, so the Ring costs a card out of hand
    // and nothing comes back.
    let hand_before_own_spell = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_own_spell - 1,
        "the Ring left p0's hand and nothing came back: the trigger watches \
         the spells of the Remora's opponents, not its controller's"
    );

    reach_their_main_phase(&mut engine, p1);
    // p1's lands are tapped *before* the Ritual is cast, and there are five
    // of them so that the pool still covers the tax when it is asked. The
    // question exists either way now that CR 605.3a opens a payment window
    // against an empty pool; floating the mana first keeps this test on the
    // card rather than on the window.
    tap_all_mana(&mut engine, p1);
    let library_before = library_size(&engine, p0);
    let hand_before_tax = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    cast_from_hand(&mut engine, p1, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p1,
        "the tax is asked of the player who cast the spell it taxes"
    );
    assert_eq!(
        prompt,
        YesNoPrompt::PayTax { mana: 4 },
        "\"unless that player pays {{4}}\" — and not the {{1}} the upkeep \
         clause charges"
    );

    engine
        .apply(p1, PlayerAction::YesNo(false))
        .expect("declining is one of the two answers the question offered");
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the declined tax pays the Remora's controller a card off the top"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_tax + 1,
        "and that card arrives: one that left the library without being \
         drawn would satisfy the count above"
    );

    // The creature spell is not what the trigger is written for, and p1 has
    // the Ritual's black mana to pay the Elves' {G} with beside the green
    // already floating.
    let hand_before_elf = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    cast_from_hand(&mut engine, p1, llanowar_elves());
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before_elf,
        "\"a noncreature spell\": an Elf cast across the table asks for no tax \
         and hands out no card"
    );
}

/// Mystic Remora's cumulative upkeep {1} (CR 702.24a): "At the beginning of
/// your upkeep, … put an age counter on this permanent. Then you may pay
/// [cost] for each age counter on it. If you don't, sacrifice it."
///
/// The first upkeep asks {1} for one counter, and p0 pays it with an Island
/// through the payment window; the Remora stays. The next asks {2} for two,
/// and declined, the Remora is sacrificed.
#[test]
fn mystic_remora_s_cumulative_upkeep_grows_and_is_sacrificed_when_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(88, island())
        .battlefield(0, &[mystic_remora(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let remora = on_battlefield(&engine, p0, mystic_remora()).expect("the Remora is seated");
    let islands = all_on_battlefield(&engine, p0, island());

    let mana = remora_upkeep_question(&mut engine, p0);
    assert_eq!(
        age_counters(&engine, remora),
        1,
        "one age counter went on first"
    );
    assert_eq!(mana, 1, "{{1}} for each age counter: one");
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("paying is one of the two answers");
    tap_mana_where(&mut engine, p0, |id| id == islands[0]);
    if engine.payment_window().is_some() {
        engine
            .apply(p0, PlayerAction::PassPriority)
            .expect("the window closes on a pool that covers the cost");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&remora),
        "paid, the Remora stays"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the Island's {{U}} went into the payment"
    );

    let mana = remora_upkeep_question(&mut engine, p0);
    assert_eq!(age_counters(&engine, remora), 2, "a second age counter");
    assert_eq!(mana, 2, "and the cost is {{1}} for each of the two");
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is the other answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, mystic_remora()).is_some(),
        "unpaid, the Remora is sacrificed"
    );
}

/// "…unless **that player** pays {4}" is the player who cast the spell. At a
/// table of three, the seat after the Remora's controller casts nothing and
/// the one after it casts Dark Ritual: the tax is asked of the caster, not of
/// whichever opponent comes first.
#[test]
fn mystic_remora_taxes_the_player_who_cast_the_spell_at_a_table_of_three() {
    let (p0, p2) = (PlayerId::new(0), PlayerId::new(2));
    let mut engine = Duel::table(89, island(), 3)
        .battlefield(0, &[island()])
        .hand(0, &[mystic_remora()])
        .battlefield(2, &[swamp()])
        .hand(2, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches a main phase");
    cast_from_hand(&mut engine, p0, mystic_remora());
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(on_battlefield(&engine, p0, mystic_remora()).is_some());

    reach_their_main_phase(&mut engine, p2);
    cast_from_hand(&mut engine, p2, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 4 });
    assert_eq!(
        player, p2,
        "the tax is asked of the player who cast the Ritual, not of p1"
    );
}
