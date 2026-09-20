//! CR 305.2 and the permissions that widen it: how many lands, and from
//! where.
//!
//! Both halves are asked from **both ends** — what `legal.lands` offers and
//! what `casting::play_land` accepts — because that pair is the whole reason
//! the rule is one predicate rather than two. An offer and a refusal that
//! disagreed would be a land the interface lists and the engine then will
//! not let go of, which is not a fault a player can tell from a bug in their
//! own reading of the card.
//!
//! The cards that print these sentences — Crucible of Worlds, Ramunap
//! Excavator, Exploration — are not in the pool yet; they are three of the
//! cards the rule is being written *for*. So the abilities are built here
//! against [`super::synthetic`]'s bench, which is what it is for: a printed
//! card would carry four other sentences that have to be true for the test
//! to mean anything.

use super::synthetic::{SyntheticLookup, keep_mulligans, land, preset, walk_past};
use super::*;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{AbilityDef, Filter, Layer, Modifier, StaticAbility};

const CRUCIBLE: u32 = 1_301;
const EXPLORATION: u32 = 1_302;
const SECOND_EXPLORATION: u32 = 1_303;

/// `static_ability!` lives in the DSL and takes no layer; a `static` here
/// cannot call it, so the one field it derives is written out — and
/// `lints::every_layer_in_the_pool_is_the_one_its_modifier_derives` is what
/// keeps a card from disagreeing with it.
const fn rules_static(modifier: Modifier) -> AbilityDef {
    AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: Filter::Any,
        modifier,
    })
}

static CRUCIBLE_ABILITIES: &[AbilityDef] = &[rules_static(Modifier::PlayLandsFromGraveyard)];
static EXPLORATION_ABILITIES: &[AbilityDef] = &[rules_static(Modifier::ExtraLandDrops(1))];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        land(CRUCIBLE, "Crucible of Worlds", CRUCIBLE_ABILITIES),
        land(EXPLORATION, "Exploration", EXPLORATION_ABILITIES),
        land(
            SECOND_EXPLORATION,
            "Exploration Again",
            EXPLORATION_ABILITIES,
        ),
    ])
}

/// A game with `battlefield` already on seat 0's side, walked to that seat's
/// first main phase with at least `in_hand` cards drawn.
///
/// The bench deals empty hands, so the cards come from the draw step and
/// that is what sets which turn this stops on — seat 0 skips its first draw
/// (CR 103.8), so one card is turn 3 and three cards is turn 7. Counted
/// rather than a turn number, because the number is a consequence of two
/// rules and would be the thing that broke if either changed.
///
/// Every caller asks for **one more card than it plays**, and that is the
/// difference between a test and a test that passes. A hand emptied by the
/// last land makes the "and nothing more is offered" line true whatever the
/// limit says: an engine with no land-drop rule at all read exactly like a
/// correct one, and an injected `has_a_land_drop_left` that always answered
/// yes left two of these green.
fn seated(seed: u64, battlefield: &[u32], in_hand: usize) -> Engine<SyntheticLookup> {
    let me = PlayerId::new(0);
    let mut engine = Engine::new(&preset(seed, battlefield), lookup()).expect("the game starts");
    keep_mulligans(&mut engine);
    for _ in 0..400 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == me
            && engine.state().zones.list(ZoneLocation::Hand(me)).len() >= in_hand
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            return engine;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    panic!("never reached seat 0's main phase holding {in_hand} cards");
}

/// Every land seat 0 is currently offered.
fn offered(engine: &Engine<SyntheticLookup>) -> Vec<ObjectId> {
    match engine.pending() {
        Pending::Priority { legal, .. } => legal.lands.clone(),
        other => panic!("expected priority, got {other:?}"),
    }
}

/// Plays one offered land and returns to seat 0's priority.
///
/// Asserted to be offered *and* accepted, rather than one or the other: the
/// pair is what this file is about.
#[track_caller]
fn play_one(engine: &mut Engine<SyntheticLookup>) -> ObjectId {
    let me = PlayerId::new(0);
    let card = *offered(engine).first().expect("a land is offered");
    engine
        .apply(me, PlayerAction::PlayLand { card })
        .expect("an offered land is accepted");
    card
}

/// Moves `card` into seat 0's graveyard behind the engine's back and asks
/// the offer again, which is what a dev command owes (`refresh_offer`).
fn into_graveyard(engine: &mut Engine<SyntheticLookup>, card: ObjectId) {
    let me = PlayerId::new(0);
    engine
        .dev_state_mut(me)
        .expect("the bench grants dev commands")
        .move_object(
            card,
            ZoneLocation::Graveyard(me),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("a card moves to the graveyard");
    engine.refresh_offer();
}

/// A land in hand, and the second one refused: the rule the other tests are
/// measured against (CR 305.2).
///
/// Without this the extra-drop test below would pass against an engine that
/// had no limit at all.
#[test]
fn one_land_a_turn_is_the_rule_the_rest_of_this_file_bends() {
    let mut engine = seated(1, &[], 2);
    let played = play_one(&mut engine);
    assert!(
        offered(&engine).is_empty(),
        "a second land is offered after the first"
    );
    // And the refusal is not merely an omission from the list: an answer
    // nobody offered is refused on its own.
    let still_in_hand = *engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .first()
        .expect("the draw left a card in hand");
    assert!(
        engine
            .apply(
                PlayerId::new(0),
                PlayerAction::PlayLand {
                    card: still_in_hand
                }
            )
            .is_err(),
        "a second land was played anyway"
    );
    assert_eq!(
        engine
            .state()
            .object(played)
            .expect("the land is still an object")
            .zone,
        crate::zone::Zone::Battlefield
    );
}

/// CR 305.2: "continuous effects may increase this number."
#[test]
fn an_extra_land_drop_is_a_second_land_and_not_a_third() {
    let mut engine = seated(2, &[EXPLORATION], 3);
    play_one(&mut engine);
    assert!(
        !offered(&engine).is_empty(),
        "the second land drop was not offered"
    );
    play_one(&mut engine);
    assert!(
        offered(&engine).is_empty(),
        "a third land is offered on one additional drop"
    );
}

/// Two such effects add up rather than one of them winning.
///
/// The half that tells the fold from a `max`, and from a flag: on either of
/// those the board below plays two lands and this test reads exactly like
/// the one above it.
#[test]
fn two_extra_land_drops_are_two_extra_lands() {
    let mut engine = seated(3, &[EXPLORATION, SECOND_EXPLORATION], 4);
    for n in 1..=3 {
        assert!(
            !offered(&engine).is_empty(),
            "land {n} of three was not offered"
        );
        play_one(&mut engine);
    }
    assert!(
        offered(&engine).is_empty(),
        "a fourth land is offered on two additional drops"
    );
}

/// Crucible of Worlds: a land in the graveyard is playable, and is really
/// played — it ends up on the battlefield and spends the land drop.
#[test]
fn a_land_in_the_graveyard_is_playable_only_while_something_says_so() {
    // Without the permission first, so the assertion below is about the
    // permission and not about whether the graveyard is walked at all.
    let mut plain = seated(4, &[], 1);
    let in_hand = *plain
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .first()
        .expect("the draw left a card in hand");
    into_graveyard(&mut plain, in_hand);
    assert!(
        offered(&plain).is_empty(),
        "a graveyard land is playable with nothing granting it"
    );
    assert!(
        plain
            .apply(PlayerId::new(0), PlayerAction::PlayLand { card: in_hand })
            .is_err(),
        "and it was played anyway"
    );

    let mut engine = seated(4, &[CRUCIBLE], 1);
    let card = *engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .first()
        .expect("the draw left a card in hand");
    into_graveyard(&mut engine, card);
    assert_eq!(offered(&engine), [card], "the graveyard land is offered");
    engine
        .apply(PlayerId::new(0), PlayerAction::PlayLand { card })
        .expect("the graveyard land is accepted");
    assert_eq!(
        engine.state().object(card).expect("still an object").zone,
        crate::zone::Zone::Battlefield,
        "it was offered, accepted, and went nowhere"
    );
}

/// CR 305.2b: the permission is not a land drop.
///
/// The two modifiers are separate rules and this is where a reading that
/// merged them would show. A player with Crucible who has already played
/// their land may play nothing at all — "for any reason" — and the
/// graveyard is not an exception to it.
#[test]
fn a_permission_to_play_from_the_graveyard_is_not_permission_to_play_more() {
    let mut engine = seated(5, &[CRUCIBLE], 2);
    let spare = *engine
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(0)))
        .first()
        .expect("the draw left a card in hand");
    into_graveyard(&mut engine, spare);
    play_one(&mut engine);
    assert!(
        offered(&engine).is_empty(),
        "the graveyard land is offered after the land drop was spent"
    );
    assert!(
        engine
            .apply(PlayerId::new(0), PlayerAction::PlayLand { card: spare })
            .is_err(),
        "and it was played anyway"
    );
}
