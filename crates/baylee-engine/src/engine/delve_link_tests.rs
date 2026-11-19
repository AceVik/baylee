//! Delve's cards are exiled with the spell (CR 607.2q), and "whenever a
//! [filter] card leaves your graveyard" (`Trigger::LeavesGraveyard`), against
//! cards nobody printed: Murktide Regent's two engine pieces.
//!
//! The made-up delver is {3}{U} with delve, enters with a +1/+1 counter for
//! each instant and sorcery card exiled with it, and gets a +1/+1 counter
//! whenever an instant or sorcery card leaves your graveyard. The watcher is
//! an enchantment that gains its controller 1 life for each instant or
//! sorcery card leaving that player's graveyard.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{
    AbilityDef, Amount, CardDef, CommanderRule, CounterKind, Coverage, Effect, EnterModifier,
    FaceDef, Filter, KeywordSet, Trigger, ZoneSel,
};
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

const DELVER: u32 = 990_301;
const WATCHER: u32 = 990_302;
const INSTANT: u32 = 990_303;

/// An instant or sorcery card you own.
static YOUR_INSTANT_OR_SORCERY: Filter =
    Filter::And(&[Filter::INSTANT_OR_SORCERY, Filter::OwnedByYou]);

/// "Enters with a +1/+1 counter on it for each instant and sorcery card
/// exiled with it."
const DELVER_ENTERS: &[EnterModifier] = &[EnterModifier::WithCounters {
    kind: CounterKind::P1P1,
    amount: Amount::CountOf {
        filter: &Filter::INSTANT_OR_SORCERY,
        zone: ZoneSel::LinkedExile,
    },
}];

/// "Whenever an instant or sorcery card leaves your graveyard, put a +1/+1
/// counter on this creature."
const DELVER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::LeavesGraveyard(&YOUR_INSTANT_OR_SORCERY),
    &[Effect::AddCounter {
        kind: CounterKind::P1P1,
        amount: Amount::Fixed(1),
    }]
)];

/// "Whenever an instant or sorcery card leaves your graveyard, you gain 1
/// life."
const WATCHER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::LeavesGraveyard(&YOUR_INSTANT_OR_SORCERY),
    &[Effect::gain_life(1)]
)];

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
    let delver = FaceDef {
        mana_cost: ManaCost::parse("{3}{U}"),
        castable_from_hand: true,
        delve: true,
        ..synthetic::creature_face("Delver", 2, 2, DELVER_ENTERS)
    };
    let watcher = FaceDef {
        types: TypeSet::ENCHANTMENT,
        ..synthetic::land_face("Watcher")
    };
    let instant = FaceDef {
        mana_cost: ManaCost::parse("{U}"),
        types: TypeSet::INSTANT,
        castable_from_hand: true,
        ..synthetic::land_face("Spent Instant")
    };
    SyntheticLookup::new(vec![
        defined(DELVER, delver, DELVER_ABILITIES),
        defined(WATCHER, watcher, WATCHER_ABILITIES),
        defined(INSTANT, instant, &[]),
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

fn island() -> u32 {
    baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get()
}

/// Seat 0's first main phase with `islands` Islands and `mine` beside them,
/// `theirs` on seat 1's battlefield, the delver and two instants in hand,
/// and the instants and the top card of the library (a Forest) put into the
/// graveyard. Returns the engine and the three graveyard cards.
fn board(islands: usize, mine: &[u32], theirs: &[u32]) -> (Engine<SyntheticLookup>, Vec<ObjectId>) {
    let mut battlefield = vec![island(); islands];
    battlefield.extend_from_slice(mine);
    let mut preset: GamePreset = synthetic::preset_both(23, &battlefield, theirs);
    preset.seats[0].starting_hand = Some(vec![entry(DELVER), entry(INSTANT), entry(INSTANT)]);
    let mut engine = Engine::new(&preset, cards()).expect("a made-up board");
    synthetic::keep_mulligans(&mut engine);
    for _ in 0..30 {
        if engine.state().turn.phase == Phase::FirstMain
            && matches!(engine.pending(), Pending::Priority { player: ME, .. })
        {
            break;
        }
        let question = engine.pending().clone();
        assert!(synthetic::walk_past(&mut engine, &question));
    }
    assert_eq!(engine.state().turn.phase, Phase::FirstMain);
    let state = engine.dev_state_mut(ME).expect("dev commands");
    let mut buried: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Hand(ME))
        .iter()
        .copied()
        .filter(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index.get() == INSTANT)
        })
        .collect();
    buried.extend(state.zones.list(ZoneLocation::Library(ME)).last().copied());
    let buried = buried
        .into_iter()
        .map(|id| {
            state
                .move_object(
                    id,
                    ZoneLocation::Graveyard(ME),
                    ZonePosition::Top,
                    crate::event::Cause::DevCommand,
                )
                .expect("a card moves to the graveyard")
        })
        .collect();
    (engine, buried)
}

fn delver_in(engine: &Engine<SyntheticLookup>, zone: ZoneLocation) -> Option<ObjectId> {
    engine.state().zones.list(zone).iter().copied().find(|id| {
        engine
            .state()
            .object(*id)
            .and_then(|o| o.card)
            .is_some_and(|c| c.index.get() == DELVER)
    })
}

/// Seat 0 taps every Island, casts the delver from hand exiling `delved`
/// with delve, and both players pass until the stack is empty.
fn cast_delver(engine: &mut Engine<SyntheticLookup>, delved: &[ObjectId]) {
    for _ in 0..12 {
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("priority expected, got {:?}", engine.pending());
        };
        let Some(&source) = legal.mana_abilities.first() else {
            break;
        };
        engine
            .apply(ME, PlayerAction::ActivateManaAbility { source })
            .expect("an Island taps for mana");
    }
    let card = delver_in(engine, ZoneLocation::Hand(ME)).expect("the delver is in hand");
    engine
        .apply(ME, PlayerAction::CastSpell { card })
        .expect("the delver is castable");
    if let Pending::ChooseCards { .. } = engine.pending() {
        engine
            .apply(
                ME,
                PlayerAction::ChooseObjects {
                    objects: delved.to_vec(),
                },
            )
            .expect("the delve answer is legal");
    } else {
        assert!(delved.is_empty(), "no delve question: {:?}", engine.pending());
    }
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

fn counters(engine: &Engine<SyntheticLookup>) -> u16 {
    let delver =
        delver_in(engine, ZoneLocation::Battlefield).expect("the delver is on the battlefield");
    engine
        .state()
        .object(delver)
        .expect("the delver")
        .counters
        .get(CounterKind::P1P1)
}

/// Two instants and a land delved: two cards exiled with it are instants,
/// so it enters with two counters. Its own leaves-the-graveyard ability did
/// not see them leave, because it was a spell on the stack then.
#[test]
fn delved_instants_are_exiled_with_the_spell_that_delved_them() {
    let (mut engine, buried) = board(1, &[], &[]);
    cast_delver(&mut engine, &buried);
    assert_eq!(counters(&engine), 2);
}

/// Paid in full, nothing is exiled with it.
#[test]
fn without_delve_nothing_is_exiled_with_it() {
    let (mut engine, _) = board(4, &[], &[]);
    cast_delver(&mut engine, &[]);
    assert_eq!(counters(&engine), 0);
}

/// The cards are exiled with the spell that became this permanent, and with
/// no later object (CR 607.2q, 400.7): returned to hand and cast again
/// without delve, the delver enters with none.
#[test]
fn a_delver_cast_again_is_a_new_object_with_nothing_exiled_with_it() {
    let (mut engine, buried) = board(5, &[], &[]);
    cast_delver(&mut engine, &buried);
    assert_eq!(counters(&engine), 2);
    let delver = delver_in(&engine, ZoneLocation::Battlefield).expect("on the battlefield");
    engine
        .dev_state_mut(ME)
        .expect("dev commands")
        .move_object(
            delver,
            ZoneLocation::Hand(ME),
            ZonePosition::Top,
            crate::event::Cause::DevCommand,
        )
        .expect("back to hand");
    cast_delver(&mut engine, &[]);
    assert_eq!(counters(&engine), 0, "the four floating mana paid in full");
}

/// Another permanent's "leaves your graveyard" sees each delved instant
/// leave, once per card, and not the land; the same ability on the
/// opponent's side watches only that player's graveyard.
#[test]
fn each_instant_leaving_your_graveyard_triggers_once() {
    let (mut engine, buried) = board(1, &[WATCHER], &[WATCHER]);
    let life = |engine: &Engine<SyntheticLookup>, seat: PlayerId| {
        engine.state().players[usize::from(seat.get())].life
    };
    let (mine, theirs) = (life(&engine, ME), life(&engine, THEM));
    cast_delver(&mut engine, &buried);
    assert_eq!(life(&engine, ME) - mine, 2, "two instants left your graveyard");
    assert_eq!(life(&engine, THEM) - theirs, 0, "none left theirs");
}
