//! The untap step's determination (CR 502.3), when a permanent gives it a
//! second answer.
//!
//! Synthetic cards rather than pool cards, and that is the honest place for
//! this today: the five Fallen Empires storage lands and Ice Floe are the
//! only six printings of "you may choose not to untap" this pool has, and
//! all six are refused by the transcoder for reasons that have nothing to
//! do with the sentence — the lands on their intervening-if upkeep trigger
//! (CR 603.4), Ice Floe on a `withoutFlying` filter. So the mechanism is
//! played here, against a land built for it, the way `m2_tests` plays the
//! layer system against a lattice nobody printed.

use super::synthetic::{
    SyntheticLookup, forest, keep_mulligans, land, permanents, preset, tap_every_land, tapped,
    walk_past,
};
use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, Cost, Effect, Filter, Layer,
    ManaColor, Modifier, StaticAbility,
};

// ---------------------------------------------------------------- fixtures

/// A land that may be left tapped, and nothing else.
const STORAGE_BASIN: u32 = 1100;
/// The same land, also held tapped by an effect — the permanent with
/// nothing left to decide.
const FROZEN_BASIN: u32 = 1101;

static THIS_F: Filter = Filter::This;

static MANA: &[Effect] = &[Effect::mana(ManaColor::Colorless, 1)];

static BASIN_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: THIS_F,
        modifier: Modifier::MayChooseNotToUntap,
    }),
    AbilityDef::Activated {
        cost: Cost::TAP,
        effects: MANA,
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
    },
];

static FROZEN_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: THIS_F,
        modifier: Modifier::MayChooseNotToUntap,
    }),
    AbilityDef::Static(StaticAbility {
        layer: Layer::Text,
        filter: THIS_F,
        modifier: Modifier::DoesNotUntap,
    }),
    AbilityDef::Activated {
        cost: Cost::TAP,
        effects: MANA,
        target: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
    },
];

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        land(STORAGE_BASIN, "Storage Basin", BASIN_ABILITIES),
        land(FROZEN_BASIN, "Frozen Basin", FROZEN_ABILITIES),
    ])
}

/// Passes priority until the untap step asks its question, or until the
/// game has gone round too far for one to be coming.
///
/// Bounded rather than `loop`: a question that never arrives is the whole
/// failure this is testing for, and a hang reports it as a timeout with no
/// state to read.
fn pass_until_question(engine: &mut Engine<SyntheticLookup>) -> bool {
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::ChooseCards { prompt, .. } => {
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::LeaveTapped,
                    "the only card question this board can ask"
                );
                return true;
            }
            other => assert!(walk_past(engine, &other), "unexpected question: {other:?}"),
        }
    }
    false
}

// ------------------------------------------------------------------- tests

/// The whole sentence, in one turn cycle: the question is asked, it is
/// asked about the one permanent that prints it, the answer is obeyed, and
/// the lands beside it untap anyway.
///
/// The two Forests are the counter-check. A permanent still tapped after an
/// untap step proves nothing on its own — an untap step that never ran
/// leaves every permanent exactly as tapped as this one.
#[test]
fn the_permanent_named_in_the_answer_stays_tapped_and_the_rest_untap() {
    let f = forest();
    let mut engine = Engine::new(&preset(11, &[STORAGE_BASIN, f, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let basin = permanents(&engine, STORAGE_BASIN)[0];
    let forests = permanents(&engine, f);
    assert_eq!(forests.len(), 2, "two Forests to untap beside the basin");

    tap_every_land(&mut engine, p0);
    assert!(tapped(&engine, basin), "the basin is tapped");
    assert!(
        forests.iter().all(|id| tapped(&engine, *id)),
        "both Forests are tapped"
    );

    assert!(
        pass_until_question(&mut engine),
        "the untap step asks about the basin"
    );
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("checked above")
    };
    assert_eq!(player, p0, "the active player determines (CR 502.3)");
    assert_eq!(options, vec![basin], "only the basin has a second answer");
    assert_eq!((min, max), (0, 1), "leaving it tapped is optional");

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .expect("the engine takes the answer it asked for");
    assert!(tapped(&engine, basin), "the basin was named, so it stays");
    assert!(
        forests.iter().all(|id| !tapped(&engine, *id)),
        "the untap step ran: the Forests untapped"
    );
}

/// The other answer, on the turn after: naming nothing untaps everything.
#[test]
fn an_empty_answer_untaps_the_permanent_like_any_other() {
    let f = forest();
    let mut engine = Engine::new(&preset(12, &[STORAGE_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let basin = permanents(&engine, STORAGE_BASIN)[0];

    tap_every_land(&mut engine, p0);
    assert!(tapped(&engine, basin));

    assert!(pass_until_question(&mut engine), "first untap step asks");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![basin],
            },
        )
        .unwrap();
    assert!(tapped(&engine, basin), "kept tapped once");

    assert!(
        pass_until_question(&mut engine),
        "and asks again next turn, because it is still tapped"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert!(!tapped(&engine, basin), "nothing was named, so it untapped");
}

/// A permanent an effect already keeps from untapping is not on the menu,
/// and the untap step asks nothing at all.
///
/// The offer/apply rule at its narrowest: both answers to that question
/// leave the permanent exactly as tapped as it was, so asking it is asking
/// a player to make a decision the rules have already made.
#[test]
fn a_permanent_that_cannot_untap_anyway_is_never_asked_about() {
    let f = forest();
    let mut engine = Engine::new(&preset(13, &[FROZEN_BASIN, f]), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let basin = permanents(&engine, FROZEN_BASIN)[0];
    let forest_id = permanents(&engine, f)[0];

    tap_every_land(&mut engine, p0);
    assert!(tapped(&engine, basin) && tapped(&engine, forest_id));

    // Walk a whole turn cycle. Nothing but priority may come back.
    for _ in 0..200 {
        let pending = engine.pending().clone();
        assert!(
            walk_past(&mut engine, &pending),
            "the untap step asked something: {pending:?}"
        );
        if !tapped(&engine, forest_id) {
            break;
        }
    }
    assert!(
        !tapped(&engine, forest_id),
        "the untap step ran and the Forest untapped"
    );
    assert!(
        tapped(&engine, basin),
        "and the basin stayed tapped without anybody being asked"
    );
}
