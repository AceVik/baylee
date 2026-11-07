//! Multikicker, against cards nobody printed (CR 702.33c, 702.33d).
//!
//! "You may pay an additional [cost] any number of times as you cast this
//! spell." The pool holds no multikicker card, so the witnesses here are
//! made up, in the way `synthetic` provides for: a sorcery that counts its
//! kicks as it resolves, and a 0/0 that counts them as it enters. What the
//! printed cards do with the count is one of these two shapes: a replacement
//! as it enters (Joraga Warcaller) or an enters trigger (Wolfbriar Elemental).

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::synthetic::{self, SyntheticLookup};
use crate::choice::NumberPrompt;
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, EnterModifier,
    FaceDef, KeywordSet, Trigger,
};
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);

const SPELL: u32 = 990_001;
const BODY: u32 = 990_002;

/// "Draw a card for each time this spell was kicked. If it was kicked, you
/// gain 1 life. Otherwise, you lose 1 life."
const SPELL_EFFECTS: &[Effect] = &[
    Effect::DrawCards {
        amount: Amount::TimesKicked,
    },
    Effect::IfKicked {
        then: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        otherwise: &[Effect::LoseLife {
            amount: Amount::Fixed(1),
            target: baylee_cards_dsl::PlayerRel::You,
        }],
    },
];

const SPELL_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(SPELL_EFFECTS)];

/// "This creature enters with a +1/+1 counter on it for each time it was
/// kicked."
const BODY_ENTERS: &[EnterModifier] = &[EnterModifier::WithCounters {
    kind: CounterKind::P1P1,
    amount: Amount::TimesKicked,
}];

/// "When this creature enters, draw a card for each time it was kicked. If
/// it was kicked, you gain 1 life."
const BODY_EFFECTS: &[Effect] = &[
    Effect::DrawCards {
        amount: Amount::TimesKicked,
    },
    Effect::IfKicked {
        then: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        otherwise: &[],
    },
];

const BODY_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(Trigger::ETB, BODY_EFFECTS)];

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
    let spell = FaceDef {
        mana_cost: ManaCost::parse("{U}"),
        types: TypeSet::SORCERY,
        castable_from_hand: true,
        multikicker: Some(ManaCost::parse("{1}{U}")),
        ..synthetic::land_face("Counted Sorcery")
    };
    let body = FaceDef {
        mana_cost: ManaCost::parse("{1}"),
        castable_from_hand: true,
        multikicker: Some(ManaCost::parse("{1}")),
        ..synthetic::creature_face("Counted Body", 0, 0, BODY_ENTERS)
    };
    SyntheticLookup::new(vec![
        defined(SPELL, spell, SPELL_ABILITIES),
        defined(BODY, body, BODY_ABILITIES),
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

/// `card` in seat 0's hand over `lands` Islands, at seat 0's first main.
fn board(card: u32, lands: usize) -> Engine<SyntheticLookup> {
    let island = baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get();
    let mut preset: GamePreset = synthetic::preset(77, &vec![island; lands]);
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

fn cast_over(card: u32, lands: usize) -> Engine<SyntheticLookup> {
    let mut engine = board(card, lands);
    synthetic::tap_every_land(&mut engine, ME);
    let spell = in_hand(&engine, card);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
}

fn the_question(engine: &Engine<SyntheticLookup>) -> (u32, u32, ManaCost) {
    match engine.pending() {
        Pending::ChooseNumber {
            min,
            max,
            reason: NumberPrompt::Multikicker { cost },
            ..
        } => (*min, *max, *cost),
        other => panic!("the multikicker question, got {other:?}"),
    }
}

fn resolve(engine: &mut Engine<SyntheticLookup>) {
    for _ in 0..12 {
        if engine.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("got {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the stack did not empty");
}

fn hand_size(engine: &Engine<SyntheticLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(ME)).len()
}

/// The pool pays what it pays: with five floating and `{U}` spent, `{1}{U}`
/// fits twice. It is what the question says, and what it charges.
#[test]
fn the_count_is_asked_up_to_what_the_pool_pays_and_each_is_charged() {
    let mut engine = cast_over(SPELL, 5);
    assert_eq!(the_question(&engine), (0, 2, ManaCost::parse("{1}{U}")));
    let before = hand_size(&engine);
    engine.apply(ME, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    resolve(&mut engine);
    assert_eq!(hand_size(&engine), before - 1 + 2, "a card for each kick");
    assert_eq!(engine.state().players[0].life, 21, "it was kicked");
}

/// One is enough to have kicked it (CR 702.33d), and a smaller count than
/// the mana allows is a count like any other.
#[test]
fn a_smaller_count_is_taken_and_one_is_kicked() {
    let mut engine = cast_over(SPELL, 5);
    engine.apply(ME, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the third and fourth of five were spent, not the rest"
    );
    resolve(&mut engine);
    assert_eq!(engine.state().players[0].life, 21);
}

/// Zero times is not kicked: "if it was kicked" is false.
#[test]
fn no_payment_is_not_kicked() {
    let mut engine = cast_over(SPELL, 5);
    engine.apply(ME, PlayerAction::ChooseNumber(0)).unwrap();
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    assert!(!engine.state().object(spell).unwrap().kicked());
    resolve(&mut engine);
    assert_eq!(engine.state().players[0].life, 19);
}

/// A cost nothing can pay once is not asked.
#[test]
fn a_pool_that_pays_none_asks_nothing() {
    let engine = cast_over(SPELL, 2);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the spell went straight to the stack, got {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
}

/// With a land still to tap the question stands open, as an X's does, and a
/// count the lands cannot pay reverses the cast (CR 601.2h, 734.1).
#[test]
fn a_count_the_lands_cannot_reach_is_reversed() {
    let mut engine = board(SPELL, 3);
    let spell = in_hand(&engine, SPELL);
    engine
        .apply(ME, PlayerAction::CastSpell { card: spell })
        .unwrap();
    let (_, max, _) = the_question(&engine);
    assert!(max > 1, "the lands are not counted for it");
    engine.apply(ME, PlayerAction::ChooseNumber(5)).unwrap();
    synthetic::tap_every_land(&mut engine, ME);
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    assert!(engine.state().zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(
        engine.state().object(spell).map(|object| object.zone),
        Some(crate::zone::Zone::Hand)
    );
}

/// Both of what the permanent asks: the count at the door, as a replacement
/// (CR 614.1c), and the count on its enters trigger (CR 400.7d, 607.2i).
#[test]
fn a_permanent_reads_the_count_as_it_enters_and_when_it_has() {
    let mut engine = cast_over(BODY, 4);
    assert_eq!(the_question(&engine), (0, 3, ManaCost::parse("{1}")));
    let before = hand_size(&engine);
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    resolve(&mut engine);
    let body = synthetic::permanents(&engine, BODY);
    assert_eq!(body.len(), 1, "three counters kept the 0/0 alive");
    let object = engine.state().object(body[0]).unwrap();
    assert_eq!(object.counters.get(CounterKind::P1P1), 3);
    assert_eq!(hand_size(&engine), before - 1 + 3, "a card for each kick");
    assert_eq!(
        engine.state().players[0].life,
        21,
        "the trigger saw it kicked"
    );
}

/// A permanent nobody kicked has no count: left unkicked the 0/0 has no
/// counter and dies, and its trigger counts nothing.
#[test]
fn a_permanent_not_kicked_counts_none() {
    let mut engine = cast_over(BODY, 4);
    let before = hand_size(&engine);
    engine.apply(ME, PlayerAction::ChooseNumber(0)).unwrap();
    resolve(&mut engine);
    assert!(synthetic::permanents(&engine, BODY).is_empty());
    assert_eq!(hand_size(&engine), before - 1);
    assert_eq!(engine.state().players[0].life, 20);
}

/// Arriving from anywhere but the stack is arriving unkicked (CR 400.7): no
/// spell became this permanent, so there is no cost to ask about.
#[test]
fn a_permanent_that_comes_back_was_never_kicked() {
    let mut engine = cast_over(BODY, 4);
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    resolve(&mut engine);
    let body = synthetic::permanents(&engine, BODY)[0];
    let state = engine.dev_state_mut(ME).unwrap();
    state
        .move_object(
            body,
            ZoneLocation::Exile(ME),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            body,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let _ = engine.apply(ME, PlayerAction::PassPriority);
    let object = engine.state().object(body).unwrap();
    assert!(!object.kicked());
    assert_eq!(object.times_kicked, 0);
}

/// A copy has the count (CR 707.2, 707.10): it is a choice made for the
/// spell, and "whether it was kicked" is named among them.
#[test]
fn a_targeted_spell_keeps_its_count_on_the_stack() {
    let mut engine = cast_over(SPELL, 5);
    engine.apply(ME, PlayerAction::ChooseNumber(2)).unwrap();
    let spell = *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap();
    let object = engine.state().object(spell).unwrap();
    assert!(object.kicked());
    assert_eq!(object.times_kicked, 2);
    assert_eq!(
        object.characteristics().mana_value(),
        1,
        "additional costs don't change a spell's mana cost (CR 118.8d)"
    );
}
