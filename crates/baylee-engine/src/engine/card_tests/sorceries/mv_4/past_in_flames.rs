//! `cards/sorceries/mv_4/past_in_flames.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Past in Flames — {3}{R} sorcery: "Each instant and sorcery card in your
/// graveyard gains flashback until end of turn. The flashback cost is equal
/// to its mana cost." (Its own flashback {4}{R} is the `Coverage::Partial`
/// gap, and nothing here presses it.)
///
/// The scenario is one card in one zone read twice. A Dark Ritual is cast so
/// that it lies in its owner's graveyard, and the offer — the only way this
/// engine ever says a card may be cast — holds nothing for it; the sorcery
/// resolves, and the same card in the same zone is offered as castable for
/// its own {B}. Casting it there is the second half: a flashed-back card is
/// exiled rather than handed back to the graveyard, and its `{B}{B}{B}`
/// arrives, so the spell resolved rather than merely being offered.
///
/// The control is a Llanowar Elves in the same graveyard, put there by
/// `seed_graveyard`. "Each instant and sorcery card" is a filter, and a
/// creature card in that zone is what says so — a grant that had lost its
/// type filter would offer it too, and every assertion about the Ritual
/// would still pass.
///
/// The rule's own test is [`super::super::flashback_tests`], which injects
/// both shapes of grant directly; this one is the card, and it is here
/// because a filtered grant is the only kind Past in Flames makes.
#[test]
fn past_in_flames_grants_a_graveyard_ritual_flashback_and_that_cast_exiles_it() {
    let p0 = PlayerId::new(0);
    // The filler deck is Llanowar Elves so that the graveyard seeding below
    // has a *creature* card to offer the filter's other half.
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain(), mountain()])
        .hand(0, &[past_in_flames(), dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, llanowar_elves()).expect("the seeded Elf");

    // A Dark Ritual is what the sorcery's sentence is about, and it gets
    // there by being cast: the Swamps pay its {B} while every Mountain is
    // held back for the sorcery itself.
    tap_all_mana_but(&mut engine, p0, Some(mountain()));
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("{B} is in the pool for it");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        in_graveyard(&engine, p0, dark_ritual()),
        Some(ritual),
        "it resolved into the graveyard, which is the zone the sentence names"
    );

    // Before the sorcery: the same card in the same zone is offered nowhere.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&ritual),
        "a card in a graveyard is not castable until something says it is: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "and a creature card there is never a spell at all"
    );

    // Past in Flames {3}{R}: the four black already in the pool are still
    // inside their phase (CR 500.4) and pay the {3}, and a Mountain pays the
    // {R}.
    tap_all_mana(&mut engine, p0);
    cast_from_hand(&mut engine, p0, past_in_flames());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "\"each instant and sorcery card in your graveyard gains flashback\": {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "\"each instant and sorcery card\" is a filter and not the whole \
         graveyard: {:?}",
        legal.castable
    );

    // Cast it from the graveyard for its own mana cost. "Then exile it" is
    // the half that tells a flashback cast from an ordinary one, and the
    // Ritual's own three black say the spell resolved on the way.
    let black_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Black);
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("the offer named it, so it is castable");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black)
            >= black_before + 2,
        "its own \"{{B}}{{B}}{{B}}\" resolved: three black arrived and at most \
         one paid the cost"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_none(),
        "\"then exile it\" — a flashed-back card does not come back to the \
         graveyard to be cast again"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&ritual),
        "it is in exile, where a card cast out of a graveyard goes"
    );
}
