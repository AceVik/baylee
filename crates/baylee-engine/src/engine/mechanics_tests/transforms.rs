//! `Effect::TransformSource`, "transform this" (CR 701.27a), and the two
//! clocks a permanent that transforms is read against.
//!
//! A double-faced permanent receives a new timestamp each time it transforms
//! (CR 613.7g), and the continuous effects of its static abilities take that
//! timestamp (CR 613.7a). It stays the same object (CR 712.18), and nothing
//! about who controls it changes, so it is no more summoning-sick than it
//! was (CR 302.6). One field used to hold both facts, which is why the flip
//! got no timestamp at all: a new one would have put the permanent to sleep.
//!
//! The card is one nobody printed, so that the order of two effects in one
//! sublayer is all the board shows: each face says "{0}: this creature has
//! base power and toughness N/N" (layer 7b, CR 613.4b), the front face also
//! "{0}: transform this", and the back face sets its own base power and
//! toughness to 5/5 as a static ability.

use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier};

const FLIPPER: u32 = 7300;

static TRANSFORM: &[Effect] = &[Effect::TransformSource];
static BECOME_ONE: &[Effect] = &[Effect::continuous(
    &Filter::This,
    Modifier::SetPT(1, 1),
    Duration::Indefinitely,
)];
static BECOME_TWO: &[Effect] = &[Effect::continuous(
    &Filter::This,
    Modifier::SetPT(2, 2),
    Duration::Indefinitely,
)];
/// 0: "{0}: transform this"; 1: "{0}: … base power and toughness 1/1".
static FRONT: &[AbilityDef] = &[free(TRANSFORM, None), free(BECOME_ONE, None)];
/// 0: "This creature has base power and toughness 5/5", whose effect has
/// the permanent's timestamp (CR 613.7a); 1: "{0}: … 2/2".
static BACK: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(Filter::This, Modifier::SetPT(5, 5)),
    free(BECOME_TWO, None),
];

fn cards() -> Vec<&'static CardDef> {
    let front = FaceDef {
        abilities: FRONT,
        ..creature_face("Flipper", "{0}", 2, 2)
    };
    let back = FaceDef {
        abilities: BACK,
        castable_from_hand: false,
        ..creature_face("Flipped", "{0}", 3, 3)
    };
    vec![Box::leak(Box::new(CardDef {
        index: CardIndex::new(FLIPPER),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([front, back])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))]
}

/// The permanent's projected power and toughness.
fn pt(engine: &Bench, id: ObjectId) -> (Option<i16>, Option<i16>) {
    let chars = engine
        .state()
        .object(id)
        .expect("on the battlefield")
        .characteristics();
    (chars.power, chars.toughness)
}

/// Activates ability `index` of the face showing and lets it resolve.
fn play(engine: &mut Bench, id: ObjectId, index: u32) {
    activate(engine, me(), id, index);
    settle(engine, me());
}

/// CR 613.7g, read in the one place a timestamp shows: which of two
/// layer-7b effects applies last.
///
/// The 1/1 is created after the permanent entered and before it transforms,
/// so once it has transformed the back face's 5/5 is the newer of the two
/// and applies over it. Without the new timestamp the back face's static
/// took the timestamp the permanent entered with, sorted first, and the 1/1
/// won. An effect created after the flip is newer still and wins in its
/// turn: the timestamp is a moment, not a rank.
#[test]
fn a_permanent_that_transforms_gets_a_new_timestamp() {
    let mut engine = bench(7300, cards(), [Seat::with(&[FLIPPER]), Seat::default()]);
    to_main(&mut engine, me());
    let id = the(&engine, ZoneLocation::Battlefield, FLIPPER);

    play(&mut engine, id, 1);
    assert_eq!(
        pt(&engine, id),
        (Some(1), Some(1)),
        "the front face, made 1/1"
    );

    play(&mut engine, id, 0);
    assert_eq!(
        engine
            .state()
            .object(id)
            .expect("the same object")
            .face_index,
        1,
        "it transformed"
    );
    assert_eq!(
        pt(&engine, id),
        (Some(5), Some(5)),
        "the back face's static is newer than the 1/1, so it applies last"
    );

    play(&mut engine, id, 1);
    assert_eq!(
        pt(&engine, id),
        (Some(2), Some(2)),
        "and an effect created after the flip is newer than the static"
    );
}

/// CR 302.6 asks whether a creature has been under its controller's control
/// continuously since their most recent turn began, and a transform changes
/// no controller: the new timestamp CR 613.7g gives it is not a new
/// controller. Asked through the offer, where a player meets it.
#[test]
fn a_permanent_that_transforms_is_not_summoning_sick_again() {
    let mut engine = bench(7301, cards(), [Seat::with(&[FLIPPER]), Seat::default()]);
    to_main(&mut engine, me());
    let id = the(&engine, ZoneLocation::Battlefield, FLIPPER);
    play(&mut engine, id, 0);
    assert_eq!(
        engine
            .state()
            .object(id)
            .expect("the same object")
            .face_index,
        1,
        "it transformed"
    );

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
            || !matches!(e.state().turn.phase, Phase::FirstMain | Phase::Combat)
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        panic!(
            "the turn went past combat without asking for attackers: {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        attackers,
        vec![id],
        "on the battlefield since before the turn began, and transformed since: it may attack"
    );
}
