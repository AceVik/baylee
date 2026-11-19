//! The kicker shapes the card reader writes and no hand-written card had
//! played yet (CR 702.33), against cards nobody printed.
//!
//! The reader learned `K:Kicker` on 19 Nov 2026 and writes two shapes the
//! pool's hand-written kicker cards never used, so nothing had played them:
//!
//! - a target that exists only when kicked (Probe's "If this spell was
//!   kicked, target player discards two cards"): no `targets` on the spell,
//!   `FaceDef::kicked_targets` for the kicked cast, and the effect under
//!   `Effect::IfKicked`;
//! - "When this creature enters, if it was kicked, …" (the Invasion
//!   Emissaries, Waterspout Elemental) as an enters trigger whose filter
//!   asks `Filter::Kicked` of the entering permanent itself, beside
//!   "If this creature was kicked, it enters with a +1/+1 counter"
//!   (`EnterModifier::WithCounters` over `Amount::TimesKicked`) on a card
//!   with a single kicker.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::synthetic::{self, SyntheticLookup};
use crate::choice::CastModeKind;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, EnterModifier,
    FaceDef, Filter, KeywordSet, PlayerRel, TargetReq, TargetSpec, Trigger,
};
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);

const PROBE_LIKE: u32 = 990_101;
const EMISSARY_LIKE: u32 = 990_102;

/// "Draw a card. If this spell was kicked, target player discards a card."
const PROBE_EFFECTS: &[Effect] = &[
    Effect::draw(1),
    Effect::IfKicked {
        then: &[Effect::DiscardForPlayers {
            who: PlayerRel::Chosen,
            count: 1,
        }],
        otherwise: &[],
    },
];

const PROBE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(PROBE_EFFECTS)];

/// The entering permanent itself, and only when it was kicked.
static KICKED_SELF: Filter = Filter::And(&[Filter::This, Filter::Kicked]);

/// "When this creature enters, if it was kicked, you gain 1 life."
const EMISSARY_EFFECTS: &[Effect] = &[Effect::GainLife {
    amount: Amount::Fixed(1),
}];

const EMISSARY_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::EntersBattlefield(&KICKED_SELF),
    EMISSARY_EFFECTS
)];

/// "If this creature was kicked, it enters with a +1/+1 counter on it."
const EMISSARY_ENTERS: &[EnterModifier] = &[EnterModifier::WithCounters {
    kind: CounterKind::P1P1,
    amount: Amount::TimesKicked,
}];

fn defined(index: u32, face: FaceDef, abilities: &'static [AbilityDef]) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn cards() -> SyntheticLookup {
    let probe = FaceDef {
        mana_cost: ManaCost::parse("{U}"),
        types: TypeSet::SORCERY,
        castable_from_hand: true,
        additional_costs: &[baylee_cards_dsl::cost!("{1}")],
        kicked_targets: Some(TargetReq::one(TargetSpec::AnyPlayer)),
        ..synthetic::land_face("Kicked Probe")
    };
    let emissary = FaceDef {
        mana_cost: ManaCost::parse("{1}"),
        castable_from_hand: true,
        additional_costs: &[baylee_cards_dsl::cost!("{1}")],
        ..synthetic::creature_face("Kicked Emissary", 1, 1, EMISSARY_ENTERS)
    };
    SyntheticLookup::new(vec![
        defined(PROBE_LIKE, probe, PROBE_ABILITIES),
        defined(EMISSARY_LIKE, emissary, EMISSARY_ABILITIES),
    ])
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    }
}

/// `card` in seat 0's hand over three Islands, at seat 0's first main.
fn board(card: u32) -> Engine<SyntheticLookup> {
    let island = baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get();
    let mut preset: GamePreset = synthetic::preset(91, &[island; 3]);
    preset.seats[0].starting_hand = Some(vec![entry(card)]);
    let mut engine = Engine::new(&preset, cards()).expect("a made-up board");
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..30 {
        if engine.state().turn.phase == Phase::FirstMain
            && matches!(engine.pending(), Pending::Priority { player: ME, .. })
        {
            return engine;
        }
        let question = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &question));
    }
    panic!("no main phase");
}

fn in_hand(engine: &Engine<SyntheticLookup>, card: u32) -> ObjectId {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|object| object.card)
                .is_some_and(|c| c.index.get() == card)
        })
        .expect("the card is in hand")
}

/// Casts `card` with every land tapped and answers the kicker question: a
/// face with `kicked_targets` offers the ordinary and the kicked cast as two
/// modes (their targets differ), a plain kicker asks yes or no.
fn cast(card: u32, mode: CastModeKind) -> Engine<SyntheticLookup> {
    let mut engine = board(card);
    synthetic::tap_every_land(&mut engine, ME);
    let spell = in_hand(&engine, card);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spell })
        .unwrap();
    match engine.pending().clone() {
        Pending::ChooseCastMode { options, .. } => {
            let index = options
                .iter()
                .position(|option| option.kind == mode)
                .expect("both casts are offered over three Islands");
            engine.apply(ME, PlayerAction::ChooseMode(index)).unwrap();
        }
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Kicker,
            ..
        } => {
            engine
                .apply(ME, PlayerAction::YesNo(mode == CastModeKind::Kicked))
                .unwrap();
        }
        other => panic!("the kicker is offered, got {other:?}"),
    }
    engine
}

/// Passes until the stack is empty, answering a discard as it is asked and
/// returning how many discards were asked of seat 0.
fn resolve(engine: &mut Engine<SyntheticLookup>) -> usize {
    let mut discards = 0;
    for _ in 0..12 {
        if engine.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return discards;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCards {
                player: ME,
                options,
                min: 1,
                max: 1,
                ..
            } => {
                discards += 1;
                engine
                    .apply(
                        ME,
                        PlayerAction::ChooseObjects {
                            objects: vec![options[0]],
                        },
                    )
                    .unwrap();
            }
            other => panic!("got {other:?}"),
        }
    }
    panic!("the stack did not empty");
}

fn hand_size(engine: &Engine<SyntheticLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(ME)).len()
}

/// Unkicked, a spell whose only target is the kicked one targets nothing:
/// the cast asks no target, the spell resolves (CR 608.2b does not apply to
/// a spell with no targets), and the kicked half does nothing.
#[test]
fn an_unkicked_cast_asks_no_target_and_resolves_without_the_kicked_half() {
    let mut engine = cast(PROBE_LIKE, CastModeKind::Normal);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no target is asked of the ordinary cast, got {:?}",
        engine.pending()
    );
    let before = hand_size(&engine);
    assert_eq!(resolve(&mut engine), 0, "no discard is asked");
    assert_eq!(hand_size(&engine), before + 1, "drew, discarded nothing");
}

/// Kicked, the same spell asks its target as it is cast (CR 601.2c) and the
/// kicked half reaches the chosen player.
#[test]
fn a_kicked_cast_asks_the_kicked_target_and_the_chosen_player_discards() {
    let mut engine = cast(PROBE_LIKE, CastModeKind::Kicked);
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!(
            "the kicked cast asks its target, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ME) && options.contains(&PlayerId::new(1)),
        "either player: {options:?}"
    );
    engine.apply(ME, PlayerAction::ChoosePlayer(ME)).unwrap();
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    assert!(engine.state().object(spell).unwrap().kicked());
    let before = hand_size(&engine);
    assert_eq!(resolve(&mut engine), 1, "the chosen player is asked once");
    assert_eq!(hand_size(&engine), before, "drew one, then discarded one");
}

/// Kicked, the creature enters with its counter and its "if it was kicked"
/// enters trigger fires: `Filter::Kicked` reads the permanent the kicked
/// spell became (CR 400.7d, 702.33d).
#[test]
fn a_kicked_creature_enters_with_its_counter_and_its_kicked_trigger_fires() {
    let mut engine = cast(EMISSARY_LIKE, CastModeKind::Kicked);
    assert_eq!(resolve(&mut engine), 0, "nothing to discard");
    let body = synthetic::permanents(&engine, EMISSARY_LIKE);
    assert_eq!(body.len(), 1);
    let object = engine.state().object(body[0]).unwrap();
    assert!(object.kicked());
    assert_eq!(object.counters.get(CounterKind::P1P1), 1);
    assert_eq!(engine.state().players[0].life, 21, "the trigger resolved");
}

/// Unkicked, neither happens: no counter, and the trigger never fires.
#[test]
fn an_unkicked_creature_has_no_counter_and_no_kicked_trigger() {
    let mut engine = cast(EMISSARY_LIKE, CastModeKind::Normal);
    assert_eq!(resolve(&mut engine), 0, "nothing to discard");
    let body = synthetic::permanents(&engine, EMISSARY_LIKE);
    assert_eq!(body.len(), 1);
    let object = engine.state().object(body[0]).unwrap();
    assert!(!object.kicked());
    assert_eq!(object.counters.get(CounterKind::P1P1), 0);
    assert_eq!(engine.state().players[0].life, 20, "nothing triggered");
}
