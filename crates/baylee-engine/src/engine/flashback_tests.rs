//! Flashback's offer, read from the rule and not from one card (CR 702.34).
//!
//! A grant arrives in two shapes and the rule has to answer both.
//! `Effect::GrantFlashback` names one object and registers
//! [`EffectFilter::ObjectIs`]; a card whose sentence is about a *set* —
//! "each instant and sorcery card in your graveyard gains flashback" —
//! goes through `bound_now`, which cannot enumerate a filter reaching past
//! the battlefield and so registers [`EffectFilter::Dsl`] and re-reads it
//! every time.
//!
//! Both readers of the grant used to match `ObjectIs` alone, written out by
//! hand in two places, so the second shape granted flashback to nobody: the
//! effect sat in the table, `legal.castable` held no graveyard card, and
//! nothing anywhere said why. The tests here inject the effect directly
//! rather than casting the one card in the pool that makes one, because
//! the rule is the subject — a card-shaped version of this file would pass
//! the day that card is re-spelled and prove nothing about the next one.
//!
//! The card's own scenario is
//! [`super::card_tests::sorceries`](super::card_tests)'s Past in Flames,
//! and the `ObjectIs` half already has card tests there too; the second
//! test below is the control that says which of the two shapes was broken.

#[allow(clippy::wildcard_imports)] // the shared duel plumbing, as every card test takes it
use super::testkit::*;

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use crate::effects::{ContinuousEffect, EffectFilter};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{Duration, Filter, Layer, Modifier, ZoneRef};
use baylee_core::ids::{CardIndex, EffectId, ObjectId};
use baylee_core::mana::ManaColor;

// ---------------------------------------------------------------- fixtures

/// "Each instant and sorcery card in your graveyard" — the filter Past in
/// Flames prints, kept here in the rule's own test so the shape survives a
/// card being re-spelled.
static INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD: Filter = Filter::And(&[
    Filter::INSTANT_OR_SORCERY,
    Filter::InZone(ZoneRef::Graveyard),
    Filter::OwnedByYou,
]);

fn dark_ritual() -> CardIndex {
    card_index("53f7c868-b03e-4fc2-8dcf-a75bbfa3272b")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

/// A seat at its own main phase with `{B}` floating, a Dark Ritual in its
/// graveyard and a Llanowar Elves beside it.
///
/// The Elf is the filter's control and the reason the filler deck is a
/// creature: "each instant and sorcery card" is a filter, and only a card
/// in the same zone that the filter must *not* reach can say so. Every
/// assertion about the Ritual would pass just as well against a grant that
/// had lost its type half.
///
/// The Swamp is tapped *here* and not at the moment of casting, because
/// `legal.castable` is read off the mana pool rather than off tappable
/// lands: with nothing floating the offer is empty whatever the rule says,
/// and the "not castable yet" assertion below would pass against a grant
/// that worked perfectly.
#[track_caller]
fn bench() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[swamp()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    seed_graveyard(&mut engine, p0, 1);
    let elf = in_graveyard(&engine, p0, llanowar_elves()).expect("the seeded Elf");

    // Straight from hand to graveyard: casting it would spend the Swamp the
    // flashback cast below needs, and the sentence under test is about a
    // card lying in a graveyard however it got there.
    let ritual = in_hand(&engine, p0, dark_ritual()).expect("the Ritual is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            ritual,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    engine.refresh_offer();

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("the Swamp taps for {B}");
    }
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "the Ritual's own {{B}} is floating, so an empty offer below is about \
         the grant and not about the mana"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&ritual) && !legal.castable.contains(&elf),
        "a card in a graveyard is castable only once something says so: {:?}",
        legal.castable
    );
    (engine, ritual, elf)
}

/// Registers a flashback grant behind the engine's back and republishes the
/// offer, which is what a resolving spell would have done.
#[track_caller]
fn grant_flashback(engine: &mut Engine<RegistryLookup>, seat: PlayerId, filter: EffectFilter) {
    let state = engine
        .dev_state_mut(seat)
        .expect("the harness may set boards up");
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: seat,
        layer: Layer::Text,
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter,
        modifier: Modifier::GrantsFlashback,
    });
    engine.refresh_offer();
}

// ------------------------------------------------------------------ tests

/// The filtered shape: a grant that names a set rather than an object.
///
/// Three things have to agree and the defect broke the first two of them.
/// `legal.castable` is the only way this engine ever says a card may be
/// cast, `casting::can_cast` is the gate `apply` runs the answer through,
/// and the cast itself is what says the offer was not cosmetic — a
/// flashed-back spell resolves (its own `{B}{B}{B}` arrive) and is exiled
/// rather than handed back to the graveyard to be cast again.
#[test]
fn a_filtered_grant_offers_the_cards_its_filter_reaches() {
    let p0 = PlayerId::new(0);
    let (mut engine, ritual, elf) = bench();

    grant_flashback(
        &mut engine,
        p0,
        EffectFilter::Dsl(&INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD),
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "the grant reaches it, so the offer carries it: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "a creature card in the same graveyard is outside the filter: {:?}",
        legal.castable
    );
    assert!(
        casting::can_cast(engine.state(), &RegistryLookup, p0, ritual).is_ok(),
        "the gate `apply` runs the answer through agrees with the offer"
    );
    assert!(
        casting::can_cast(engine.state(), &RegistryLookup, p0, elf).is_err(),
        "and refuses the card the filter misses"
    );

    // The offer, taken. The Swamp's {B} has been floating since `bench`.
    engine
        .apply(p0, PlayerAction::CastSpell { card: ritual })
        .expect("the offer named it, so it is castable");
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black)
            >= 3,
        "its own \"{{B}}{{B}}{{B}}\" resolved rather than the offer being cosmetic"
    );
    assert!(
        in_graveyard(&engine, p0, dark_ritual()).is_none()
            && engine
                .state()
                .zones
                .list(ZoneLocation::Exile(p0))
                .contains(&ritual),
        "CR 702.34: a card cast from a graveyard this way is exiled, not \
         returned to be cast again"
    );
}

/// The named shape, which is the control.
///
/// `Effect::GrantFlashback` registers `EffectFilter::ObjectIs`, and that
/// half was never broken — it is here so the file says which of the two
/// shapes the rule used to answer, and so a later rewrite of
/// `effects::applies_to` cannot buy the filtered half at its expense.
#[test]
fn a_named_grant_offers_exactly_the_card_it_names() {
    let p0 = PlayerId::new(0);
    let (mut engine, ritual, elf) = bench();

    grant_flashback(&mut engine, p0, EffectFilter::ObjectIs(ritual));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&ritual),
        "the grant names it: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&elf),
        "and names nothing else: {:?}",
        legal.castable
    );
}
