//! "Whenever a player cycles a card" (`Trigger::Cycled`, CR 702.29c–d),
//! against cards nobody printed.
//!
//! The trigger reads `GameEvent::Cycled`, the event the activation records
//! when a cycling ability's cost discards its card, and the relation names
//! whose cycling counts: `EachPlayer` for "a player" (Lightning Rift, Astral
//! Slide), `You` for "you". A card discarded to pay any other ability is not
//! cycled, so the same discard under another ability triggers nothing; a
//! typecycled card is cycled (CR 702.29f).

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use super::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{
    AbilityDef, ActivationZone, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, Find,
    KeywordSet, PlayerRel, Trigger,
};
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::mana::ManaCost;
use baylee_core::preset::{DeckEntry, GamePreset};
use baylee_core::types::TypeSet;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

const ANY_CYCLES: u32 = 990_201;
const YOU_CYCLE: u32 = 990_202;
const CYCLER: u32 = 990_203;
const DISCARDER: u32 = 990_204;
const LANDCYCLER: u32 = 990_205;

/// "Whenever a player cycles a card, you gain 1 life."
const ANY_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Cycled {
        by: PlayerRel::EachPlayer,
        filter: &Filter::Any
    },
    &[Effect::gain_life(1)]
)];

/// "Whenever you cycle a card, you gain 1 life."
const YOU_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Cycled {
        by: PlayerRel::You,
        filter: &Filter::Any
    },
    &[Effect::gain_life(1)]
)];

/// "Cycling {1}" (CR 702.29a).
const CYCLER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    baylee_cards_dsl::cost!("{1}", DiscardSelf),
    &[Effect::draw(1)],
    zone = ActivationZone::Hand
)];

/// "{1}, Discard this card: Draw two cards." A discard from hand that is not
/// cycling: cycling draws exactly one (CR 702.29a).
const DISCARDER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    baylee_cards_dsl::cost!("{1}", DiscardSelf),
    &[Effect::draw(2)],
    zone = ActivationZone::Hand
)];

/// "Basic landcycling {1}" (CR 702.29e), as the reader writes typecycling.
const LANDCYCLER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
    baylee_cards_dsl::cost!("{1}", DiscardSelf),
    &[Effect::SearchLibrary {
        filter: &Filter::BASIC_LAND,
        finds: &[Find::HAND],
        optional: false,
    }],
    zone = ActivationZone::Hand
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
        fuse: false,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn cards() -> SyntheticLookup {
    let enchantment = |name| FaceDef {
        types: TypeSet::ENCHANTMENT,
        ..synthetic::land_face(name)
    };
    let instant = |name| FaceDef {
        mana_cost: ManaCost::parse("{5}"),
        types: TypeSet::INSTANT,
        castable_from_hand: true,
        ..synthetic::land_face(name)
    };
    SyntheticLookup::new(vec![
        defined(ANY_CYCLES, enchantment("Any Cycles"), ANY_ABILITIES),
        defined(YOU_CYCLE, enchantment("You Cycle"), YOU_ABILITIES),
        defined(CYCLER, instant("Cycler"), CYCLER_ABILITIES),
        defined(DISCARDER, instant("Discarder"), DISCARDER_ABILITIES),
        defined(LANDCYCLER, instant("Landcycler"), LANDCYCLER_ABILITIES),
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

/// `watcher` on seat 0's battlefield beside two Islands, two Islands on seat
/// 1's, and `in_hand` in each hand, at seat 0's first main phase.
fn board(watcher: u32, in_hand: u32) -> Engine<SyntheticLookup> {
    let island = baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .expect("Island exists")
        .index
        .get();
    let mut preset: GamePreset =
        synthetic::preset_both(17, &[island, island, watcher], &[island, island]);
    preset.seats[0].starting_hand = Some(vec![entry(in_hand)]);
    preset.seats[1].starting_hand = Some(vec![entry(in_hand)]);
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

fn in_hand(engine: &Engine<SyntheticLookup>, seat: PlayerId, card: u32) -> ObjectId {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat))
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

/// `seat` taps its Islands and activates the hand ability of `card`, then
/// both players pass until the stack is empty. Returns seat 0's life gained.
fn activate_from_hand(mut engine: Engine<SyntheticLookup>, seat: PlayerId, card: u32) -> i32 {
    let before = engine.state().players[0].life;
    tap_islands(&mut engine, seat);
    let source = in_hand(&engine, seat, card);
    engine
        .apply(
            seat,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .expect("the hand ability is offered off two Islands");
    assert!(
        in_hand_count(&engine, seat, card) == 0,
        "the cost discarded the card"
    );
    for _ in 0..12 {
        if engine.state().zones.list(ZoneLocation::Stack).is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return engine.state().players[0].life - before;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            // The landcycler's search: take the first Forest offered.
            Pending::ChooseCards {
                player, options, ..
            } if !options.is_empty() => {
                engine
                    .apply(
                        player,
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

fn in_hand_count(engine: &Engine<SyntheticLookup>, seat: PlayerId, card: u32) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|object| object.card)
                .is_some_and(|c| c.index.get() == card)
        })
        .count()
}

#[test]
fn whenever_a_player_cycles_reads_your_cycling_and_theirs() {
    assert_eq!(
        activate_from_hand(board(ANY_CYCLES, CYCLER), ME, CYCLER),
        1,
        "you cycled a card"
    );
    assert_eq!(
        activate_from_hand(board(ANY_CYCLES, CYCLER), THEM, CYCLER),
        1,
        "an opponent cycled a card"
    );
}

#[test]
fn whenever_you_cycle_reads_only_your_cycling() {
    assert_eq!(
        activate_from_hand(board(YOU_CYCLE, CYCLER), ME, CYCLER),
        1,
        "you cycled a card"
    );
    assert_eq!(
        activate_from_hand(board(YOU_CYCLE, CYCLER), THEM, CYCLER),
        0,
        "an opponent's cycling is not yours"
    );
}

/// CR 702.29c: a card is cycled when it is discarded to pay a *cycling*
/// ability's cost. Discarding it to pay another ability's cost is not
/// cycling it.
#[test]
fn a_discard_to_pay_another_ability_is_not_cycling() {
    assert_eq!(
        activate_from_hand(board(ANY_CYCLES, DISCARDER), ME, DISCARDER),
        0,
        "a discard that draws two is not cycling"
    );
}

/// CR 702.29f: typecycling is cycling, so a landcycled card is a card
/// cycled (the shape `AbilityDef::is_cycling` reads, as the reader writes
/// `K:TypeCycling`).
#[test]
fn typecycling_is_cycling() {
    assert_eq!(
        activate_from_hand(board(ANY_CYCLES, LANDCYCLER), ME, LANDCYCLER),
        1,
        "you landcycled a card"
    );
}

/// Gives `seat` priority and taps its Islands for their intrinsic mana.
///
/// Not `synthetic::tap_every_land`, which presses whatever untapped ability
/// is offered first, and the card in hand under test is offering one.
fn tap_islands(engine: &mut Engine<SyntheticLookup>, seat: PlayerId) {
    for _ in 0..8 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            panic!("priority expected, got {:?}", engine.pending());
        };
        if player != seat {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
            continue;
        }
        let Some(&source) = legal.mana_abilities.first() else {
            return;
        };
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .expect("the engine takes the land mana it offered");
    }
}
