//! `Filter::InZone(ZoneRef::Hand)`: "a card in a hand" (CR 402.1).
//!
//! The pool prints it once, inside Drannith Magistrate's "your opponents
//! can't cast spells from anywhere other than their hands" — a `Not` around
//! it, so a matcher that answered the hand wrong would lock the one zone the
//! card leaves open and open every zone it locks. Both halves are played
//! here: the filter asked directly of an object in each zone, and the lock
//! asked through the offer, where a player meets it.

use super::*;
use crate::effects::{ContinuousEffect, EffectFilter};
use crate::zone::ZonePosition;
use baylee_cards_dsl::{Duration, Filter, Layer, Modifier, ZoneRef};
use baylee_core::ids::EffectId;

const MAGISTRATE: u32 = 7200;
const SPARK: u32 = 7201;

static IN_A_HAND: Filter = Filter::InZone(ZoneRef::Hand);
static NOT_FROM_A_HAND: Filter = Filter::Not(&IN_A_HAND);

static MAGISTRATE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::Any,
    Modifier::OpponentsCantCast(&NOT_FROM_A_HAND)
)];

static GAIN_ONE: &[Effect] = &[Effect::gain_life(1)];
static SPARK_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: GAIN_ONE,
    targets: None,
    second_targets: None,
    condition: None,
}];

fn cards() -> Vec<&'static CardDef> {
    vec![
        super::super::synthetic::land(MAGISTRATE, "Magistrate", MAGISTRATE_ABILITIES),
        card(
            SPARK,
            face("Spark", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPARK_ABILITIES,
        ),
    ]
}

/// Moves `card` into its owner's graveyard, behind the engine's back.
fn bury(engine: &mut Bench, card: ObjectId) {
    let owner = engine.state().object(card).expect("the card").owner;
    engine
        .dev_state_mut(owner)
        .expect("the harness may set boards up")
        .move_object(
            card,
            ZoneLocation::Graveyard(owner),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
}

/// Gives `card` flashback until end of turn, the shape `Effect::GrantFlashback`
/// registers, so a graveyard is a second zone the card can be cast from.
fn flash_back(engine: &mut Bench, card: ObjectId) {
    let owner = engine.state().object(card).expect("the card").owner;
    let state = engine
        .dev_state_mut(owner)
        .expect("the harness may set boards up");
    let timestamp = state.next_timestamp();
    let filter = EffectFilter::object(state, card);
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: owner,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: Layer::Text,
        timestamp,
        duration: Duration::UntilEndOfTurn,
        filter,
        modifier: Modifier::GrantsFlashback,
    });
}

/// The filter asked of one object in each zone a card can be in, the hand
/// the only one it may answer yes for.
#[test]
fn a_hand_filter_matches_a_card_in_a_hand_and_nowhere_else() {
    let mut engine = bench(
        7200,
        cards(),
        [
            Seat::with(&[MAGISTRATE]).holding(&[SPARK, SPARK]),
            Seat::default(),
        ],
    );
    let [held, buried] = objects(&engine, ZoneLocation::Hand(me()), SPARK)[..] else {
        panic!("two Sparks in hand")
    };
    bury(&mut engine, buried);
    let exiled = engine.state().zones.list(ZoneLocation::Library(me()))[0];
    engine
        .dev_state_mut(me())
        .expect("the harness may set boards up")
        .move_object(
            exiled,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    let in_library = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .expect("a library");
    let on_battlefield = the(&engine, ZoneLocation::Battlefield, MAGISTRATE);

    let state = engine.state();
    let asks = |id: ObjectId| {
        let obj = state.object(id).expect("the object");
        crate::eval::matches(&IN_A_HAND, state, obj, me(), id)
    };
    assert!(asks(held), "a card in its owner's hand is in a hand");
    for (id, zone) in [
        (buried, "graveyard"),
        (in_library, "library"),
        (exiled, "exile"),
        (on_battlefield, "battlefield"),
    ] {
        assert!(!asks(id), "a card in the {zone} is not in a hand");
    }
}

/// Drannith Magistrate's lock: the opponent may cast the card in their hand
/// and not the same card from their graveyard, though it has flashback.
///
/// The control board without the lock is what makes the refusal mean
/// something: there the graveyard copy *is* offered, so its absence under
/// the lock is the filter's answer and not a grant that never applied.
#[test]
fn a_lock_on_every_zone_but_the_hand_leaves_the_hand_open() {
    for locked in [true, false] {
        let board: &[u32] = if locked { &[MAGISTRATE] } else { &[] };
        let mut engine = bench(
            7201,
            cards(),
            [Seat::with(board), Seat::default().holding(&[SPARK, SPARK])],
        );
        let [held, buried] = objects(&engine, ZoneLocation::Hand(them()), SPARK)[..] else {
            panic!("two Sparks in hand")
        };
        bury(&mut engine, buried);
        flash_back(&mut engine, buried);
        to_main(&mut engine, me());
        engine.apply(me(), PlayerAction::PassPriority).unwrap();

        let (player, legal) = offer(&engine);
        assert_eq!(player, them(), "the opponent holds priority on my turn");
        assert!(
            legal.castable.contains(&held),
            "the hand is open, locked or not: {:?}",
            legal.castable
        );
        assert_eq!(
            legal.castable.contains(&buried),
            !locked,
            "the graveyard is locked exactly while the Magistrate is out \
             (locked: {locked}): {:?}",
            legal.castable
        );
    }
}
