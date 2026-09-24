//! Lands built for a test rather than printed, and the bench they sit on.
//!
//! Some rules are best played against a card nobody printed. The pool is
//! the wrong instrument for them twice over: a printed card carries four
//! other sentences that have to be true for the test to mean anything, and
//! the cards that *do* print the rule are often refused by the transcoder
//! for reasons that have nothing to do with it. `m2_tests` already plays
//! the layer system against a lattice nobody printed; this is the same
//! bargain for the ones that need a permanent on the battlefield.
//!
//! What lives here is the bench and not the card: a land face with no
//! text, a lookup that answers for a handful of made-up indices and defers
//! to the real pool for everything else, a two-seat preset, and the few
//! answers a turn asks on the way round. The cards themselves stay in the
//! test module that is about them, because a card here would be a fixture
//! nobody reading the test could see.

use super::*;
use baylee_cards_dsl::{AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, KeywordSet};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::types::{SupertypeSet, TypeSet};

/// A land face with a name and nothing else on it.
///
/// Every characteristic a land has no printed value for, rather than the
/// ones this test happens to read: a face built field by field per test
/// would be a second place for the DSL's defaults to be restated.
#[must_use]
pub fn land_face(name: &'static str) -> FaceDef {
    FaceDef {
        name,
        mana_cost: baylee_core::mana::ManaCost::ZERO,
        types: TypeSet::LAND,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: None,
        toughness: None,
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        keywords: KeywordSet::EMPTY,
        color_indicator: baylee_core::color::ColorSet::EMPTY,
        castable_from_hand: false,
        miracle: None,
        delve: false,
        convoke: false,
        waterbend: false,
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }
}

/// A one-faced land at a made-up index, leaked so the engine can hold it
/// for `'static` the way it holds a compiled card.
///
/// Leaking rather than a `static` per card because the index and the
/// ability list are what a test wants to write, and a `static` cannot take
/// either as an argument.
#[must_use]
pub fn land(index: u32, name: &'static str, abilities: &'static [AbilityDef]) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([land_face(name)])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

/// A creature face with a body, whatever it does as it enters, and nothing
/// else on it.
///
/// The sibling of [`land_face`] and here for the same reason: what a test
/// wants to write is the two or three characteristics it is about, not the
/// twenty a `FaceDef` has. `enter_modifiers` is one of the three because a
/// rule about *arriving* has nowhere else to put itself.
#[must_use]
pub fn creature_face(
    name: &'static str,
    power: i16,
    toughness: i16,
    enter_modifiers: &'static [baylee_cards_dsl::EnterModifier],
) -> FaceDef {
    FaceDef {
        power: Some(power),
        toughness: Some(toughness),
        types: TypeSet::CREATURE,
        enter_modifiers,
        ..land_face(name)
    }
}

/// A one-faced creature at a made-up index, leaked the way [`land`] leaks.
#[must_use]
pub fn creature(
    index: u32,
    name: &'static str,
    power: i16,
    toughness: i16,
    enter_modifiers: &'static [baylee_cards_dsl::EnterModifier],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([creature_face(
            name,
            power,
            toughness,
            enter_modifiers,
        )])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

/// A card lookup that knows a few made-up cards and the whole pool behind
/// them.
///
/// The fallback is what lets a synthetic board hold real Forests: a deck
/// has to be *something*, and a test that had to invent a basic land as
/// well would be proving its fixtures rather than the rule.
pub struct SyntheticLookup(Vec<&'static CardDef>);

impl SyntheticLookup {
    /// Answers for these cards first, then for the pool.
    #[must_use]
    pub fn new(defs: Vec<&'static CardDef>) -> Self {
        Self(defs)
    }
}

impl CardLookup for SyntheticLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        self.0
            .iter()
            .find(|def| def.index == index)
            .copied()
            .or_else(|| baylee_cards::by_index(index))
    }
}

/// The pool's Forest, by oracle id — a deck filler and an untap-step
/// counter-check in one.
#[must_use]
pub fn forest() -> u32 {
    baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("Forest exists")
        .index
        .get()
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    }
}

/// Two seats, empty hands, and the given cards already on seat 0's
/// battlefield.
///
/// Empty hands and a battlefield laid out by the preset because the rules
/// under test here happen on a permanent that is already in play: casting
/// one first would put a mulligan, a land drop and a summoning-sickness
/// turn between the test and what it is about.
#[must_use]
pub fn preset(seed: u64, battlefield: &[u32]) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
    let seat = |bf: &[u32]| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(vec![]),
        starting_battlefield: bf.iter().map(|c| entry(*c)).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(battlefield), seat(&[])],
    }
}

/// Keeps both opening hands.
pub fn keep_mulligans<L: CardLookup>(engine: &mut Engine<L>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

/// Every permanent on the battlefield that came from one card index.
#[must_use]
pub fn permanents<L: CardLookup>(engine: &Engine<L>, index: u32) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == index))
        })
        .collect()
}

/// Whether a permanent is tapped.
#[must_use]
pub fn tapped<L: CardLookup>(engine: &Engine<L>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .is_some_and(|o| o.status.contains(Status::TAPPED))
}

/// Taps every land on `seat`'s battlefield for mana, through the offer.
///
/// Through `legal.abilities` and not `legal.mana_abilities`: the second
/// carries the CR 305.6 shortcut for a land's *intrinsic* mana, which a
/// land whose mana comes from a printed ability does not have. Reading the
/// wrong list here is how a test once tapped two of three lands and
/// believed it had tapped all of them.
pub fn tap_every_land<L: CardLookup>(engine: &mut Engine<L>, seat: PlayerId) {
    for _ in 0..8 {
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            break;
        };
        if player != seat {
            engine.apply(player, PlayerAction::PassPriority).unwrap();
            continue;
        }
        let next = legal
            .abilities
            .iter()
            .copied()
            .find(|(id, _)| !tapped(engine, *id))
            .or_else(|| legal.mana_abilities.first().map(|id| (*id, 0)));
        let Some((source, ability_index)) = next else {
            break;
        };
        if legal.mana_abilities.contains(&source) {
            engine
                .apply(seat, PlayerAction::ActivateManaAbility { source })
                .expect("the engine takes the land mana it offered");
        } else {
            engine
                .apply(
                    seat,
                    PlayerAction::ActivateAbility {
                        source,
                        ability_index,
                    },
                )
                .expect("the engine takes the ability it offered");
        }
    }
}

/// Answers the questions a turn asks on the way round, and nothing else.
///
/// Priority is passed and combat is declined; anything else is a question
/// the board under test should not be able to ask, and the caller says so.
/// Listing them rather than answering whatever arrives is what keeps the
/// question a test is waiting for from being swallowed by a driver that
/// answers everything.
pub fn walk_past<L: CardLookup>(engine: &mut Engine<L>, pending: &Pending) -> bool {
    match pending {
        Pending::Priority { player, .. } => {
            engine.apply(*player, PlayerAction::PassPriority).unwrap();
            true
        }
        Pending::ChooseAttackers { player, .. } => {
            engine
                .apply(
                    *player,
                    PlayerAction::DeclareAttackers { attackers: vec![] },
                )
                .unwrap();
            true
        }
        Pending::ChooseBlockers { player, .. } => {
            engine
                .apply(*player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .unwrap();
            true
        }
        _ => false,
    }
}
