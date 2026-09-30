//! An effect that sets a land's subtype to a basic land type (CR 305.7):
//! Evil Presence, Conversion, Phantasmal Terrain.
//!
//! Played against a land built for it, because the rule is about what the
//! land loses, and a printed land that loses something visible carries other
//! sentences a test would have to hold still. The marsh here is a Forest by
//! its type line, prints a keyword, a mana ability and a static ability, and
//! each of the three is what the rule takes away; a second land grants it a
//! keyword the rule must leave alone.

use super::synthetic::{SyntheticLookup, keep_mulligans, land_face, permanents, preset, walk_past};
use super::*;
use crate::object::Characteristics;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost,
    Coverage, Effect, FaceDef, Filter, KeywordSet, Layer, ManaColor, Modifier, StaticAbility,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::CardIndex;
use baylee_core::types::SubtypeSet;

// ---------------------------------------------------------------- fixtures

/// A Forest by its type line that prints shroud, "{T}: Add {C}" and "this
/// land is red".
const MARSH: u32 = 1110;
/// "Each Forest is a Swamp": the effect under test.
const SETTER: u32 = 1111;
/// "Lands have hexproof": an ability another effect grants.
const GRANTER: u32 = 1112;

static MANA_C: &[Effect] = &[Effect::mana(ManaColor::Colorless, 1)];

static MARSH_ABILITIES: &[AbilityDef] = &[
    AbilityDef::Activated {
        cost: Cost::TAP,
        effects: MANA_C,
        targets: None,
        second_targets: None,
        timing: ActivationTiming::InstantSpeed,
        mana_ability: true,
        zone: ActivationZone::Battlefield,
        limit: ActivationLimit::Unlimited,
        cost_reduction: None,
    },
    AbilityDef::Static(StaticAbility {
        layer: Layer::Color,
        filter: Filter::This,
        modifier: Modifier::AddColor(ColorSet::of(Color::Red)),
        condition: None,
    }),
];

static SETTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::Type,
    filter: Filter::HasSubtype(land::FOREST),
    modifier: Modifier::SetLandType(land::SWAMP),
    condition: None,
})];

static GRANTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::Ability,
    filter: Filter::LAND,
    modifier: Modifier::AddKeyword(KeywordSet::HEXPROOF),
    condition: None,
})];

static MARSH_TYPES: &[baylee_core::ids::SubtypeId] = &[land::FOREST];

/// "As this enters, choose a basic land type. Each Forest is the chosen
/// type": Phantasmal Terrain's two sentences on a land, so it can be played.
const CHOOSER: u32 = 1113;

static CHOOSER_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::Type,
    filter: Filter::HasSubtype(land::FOREST),
    modifier: Modifier::SetLandTypeToChosen,
    condition: None,
})];

static CHOOSER_ENTERS: &[baylee_cards_dsl::EnterModifier] =
    &[baylee_cards_dsl::EnterModifier::ChooseBasicLandType];

fn card(
    index: u32,
    face: FaceDef,
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: ColorSet::EMPTY,
        keywords,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        card(
            MARSH,
            FaceDef {
                subtypes: MARSH_TYPES,
                keywords: KeywordSet::SHROUD,
                ..land_face("Marsh Basin")
            },
            KeywordSet::SHROUD,
            MARSH_ABILITIES,
        ),
        card(
            SETTER,
            land_face("Setter Basin"),
            KeywordSet::EMPTY,
            SETTER_ABILITIES,
        ),
        card(
            GRANTER,
            land_face("Granter Basin"),
            KeywordSet::EMPTY,
            GRANTER_ABILITIES,
        ),
        card(
            CHOOSER,
            FaceDef {
                enter_modifiers: CHOOSER_ENTERS,
                ..land_face("Chooser Basin")
            },
            KeywordSet::EMPTY,
            CHOOSER_ABILITIES,
        ),
    ])
}

/// A board with `cards` on seat 0's battlefield, both hands kept, and the
/// marsh's id.
fn board(cards: &[u32]) -> (Engine<SyntheticLookup>, ObjectId) {
    let mut engine = Engine::new(&preset(21, cards), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let marsh = permanents(&engine, MARSH)[0];
    (engine, marsh)
}

fn characteristics(engine: &Engine<SyntheticLookup>, id: ObjectId) -> Characteristics {
    engine.state().object(id).unwrap().characteristics().clone()
}

/// What seat 0 is offered on the marsh: whether its CR 305.6 shortcut is
/// open, and whether its printed "{T}: Add {C}" is.
fn marsh_offer(engine: &Engine<SyntheticLookup>, marsh: ObjectId) -> (bool, bool) {
    let Pending::Priority { player, legal } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert_eq!(*player, PlayerId::new(0), "seat 0 holds priority");
    (
        legal.mana_abilities.contains(&marsh),
        legal.abilities.contains(&(marsh, 0)),
    )
}

/// Without the effect the marsh is what it prints: a red Forest with shroud
/// that taps for {G} by its type and {C} by its text. The counter-board for
/// the test below.
#[test]
fn a_land_nothing_sets_is_what_it_prints() {
    let (engine, marsh) = board(&[MARSH, GRANTER]);
    let c = characteristics(&engine, marsh);
    assert!(
        c.subtypes.contains(land::FOREST),
        "a Forest by its type line"
    );
    assert!(!c.subtypes.contains(land::SWAMP));
    assert!(
        c.keywords.contains(KeywordSet::SHROUD),
        "its printed keyword"
    );
    assert!(
        c.keywords.contains(KeywordSet::HEXPROOF),
        "the granted keyword"
    );
    assert!(
        c.colors.contains(Color::Red),
        "its own static ability applies"
    );
    assert!(!c.rules_text_lost);
    assert_eq!(
        marsh_offer(&engine, marsh),
        (true, true),
        "{{G}} and {{C}} are offered"
    );
}

/// CR 305.7: set to a Swamp, the land has no other land type, loses the
/// keyword, mana ability and static ability its rules text gives it, taps
/// for {B} through the Swamp's mana ability, and keeps hexproof, which
/// another effect grants it.
#[test]
fn a_land_set_to_a_basic_land_type_loses_its_rules_text_and_keeps_its_grants() {
    let (mut engine, marsh) = board(&[MARSH, SETTER, GRANTER]);
    let c = characteristics(&engine, marsh);
    let mut swamp = SubtypeSet::EMPTY;
    swamp.insert(land::SWAMP);
    assert_eq!(c.subtypes, swamp, "a Swamp and no longer a Forest");
    assert!(
        !c.keywords.contains(KeywordSet::SHROUD),
        "its printed keyword is gone"
    );
    assert!(
        c.keywords.contains(KeywordSet::HEXPROOF),
        "a granted keyword stays"
    );
    assert!(
        !c.colors.contains(Color::Red),
        "its own static ability lapsed"
    );
    assert_eq!(c.produced_colors, ColorSet::of(Color::Black));
    assert!(
        engine
            .state()
            .object(marsh)
            .unwrap()
            .abilities(&lookup())
            .is_empty(),
        "no ability its text prints is left"
    );
    assert_eq!(
        marsh_offer(&engine, marsh),
        (true, false),
        "the Swamp's mana is offered and the printed {{C}} is not"
    );

    engine
        .apply(
            PlayerId::new(0),
            PlayerAction::ActivateManaAbility { source: marsh },
        )
        .expect("the engine takes the land mana it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "it tapped for {{B}}");
    assert_eq!(pool.available(ManaColor::Green), 0, "not for {{G}}");
    assert_eq!(pool.available(ManaColor::Colorless), 0, "not for {{C}}");
}

/// "As this enters, choose a basic land type" asks for one of the five
/// (CR 205.3i) and for nothing else, and "is the chosen type" sets the
/// land's subtype to the answer (CR 305.7): the marsh becomes an Island, taps
/// for {U}, and has lost its printed {C}.
#[test]
fn a_land_set_to_the_chosen_basic_land_type_is_that_type() {
    let me = PlayerId::new(0);
    let mut preset = preset(21, &[MARSH]);
    preset.seats[0].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
        card: CardIndex::new(CHOOSER),
        print: baylee_core::ids::PrintRef::new(0),
    }]);
    let mut engine = Engine::new(&preset, lookup()).unwrap();
    keep_mulligans(&mut engine);
    let marsh = permanents(&engine, MARSH)[0];
    for _ in 0..200 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        }
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending), "walked past {pending:?}");
    }
    assert!(
        characteristics(&engine, marsh)
            .subtypes
            .contains(land::FOREST),
        "a Forest until the type is chosen"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    let card = *legal
        .lands
        .first()
        .expect("the chooser is offered as a land");
    engine
        .apply(me, PlayerAction::PlayLand { card })
        .expect("an offered land is accepted");
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected the type question, got {:?}", engine.pending());
    };
    assert_eq!(player, me, "its controller chooses");
    assert_eq!(
        options,
        [
            land::PLAINS,
            land::ISLAND,
            land::SWAMP,
            land::MOUNTAIN,
            land::FOREST
        ],
        "the five basic land types and nothing else"
    );
    assert!(
        engine
            .apply(
                me,
                PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::land::DESERT)
            )
            .is_err(),
        "a nonbasic land type is refused"
    );
    engine
        .apply(me, PlayerAction::ChooseSubtype(land::ISLAND))
        .expect("an offered type is accepted");

    let c = characteristics(&engine, marsh);
    let mut island = SubtypeSet::EMPTY;
    island.insert(land::ISLAND);
    assert_eq!(c.subtypes, island, "an Island and no longer a Forest");
    assert_eq!(
        marsh_offer(&engine, marsh),
        (true, false),
        "the Island's mana is offered and the printed {{C}} is not"
    );
    engine
        .apply(me, PlayerAction::ActivateManaAbility { source: marsh })
        .expect("the engine takes the land mana it offered");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "it tapped for {{U}}"
    );
}
