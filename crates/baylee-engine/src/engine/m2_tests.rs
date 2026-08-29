use super::*;
use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, Effect, FaceDef, Filter, KeywordSet, Layer,
    Modifier, PartnerKind, StaticAbility, TargetReq, TargetSpec,
};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::types::{SupertypeSet, TypeSet};

// ---------------------------------------------------------------- fixtures
// Synthetic cards injected via the CardLookup seam (indexes 1000+).

const ANTHEM_LORD: u32 = 1000;
const FAKE_LATTICE: u32 = 1001;
const TEST_BEAR: u32 = 1002;
const PUMP_SPELL: u32 = 1003;

static CREATURE_F: Filter = Filter::HasType(TypeSet::CREATURE);
static CREATURE_YOU: Filter =
    Filter::And(&[Filter::HasType(TypeSet::CREATURE), Filter::ControlledByYou]);
static THIS_F: Filter = Filter::This;

fn face(name: &'static str, cost: &'static str, types: TypeSet, pt: Option<(i16, i16)>) -> FaceDef {
    FaceDef {
        name,
        mana_cost: baylee_core::mana::ManaCost::parse(cost),
        types,
        supertypes: SupertypeSet::EMPTY,
        subtypes: &[],
        power: pt.map(|(p, _)| p),
        toughness: pt.map(|(_, t)| t),
        loyalty: None,
        alternative_costs: &[],
        additional_costs: &[],
        mandatory_additional_costs: &[],
        enter_modifiers: &[],
        abilities: &[],
        castable_from_hand: true,
        miracle: None,
        delve: false,
        convoke: false,
    }
}

fn def(index: u32, face: FaceDef, abilities: &'static [AbilityDef]) -> CardDef {
    CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

struct TestLookup {
    anthem_lord: &'static CardDef,
    fake_lattice: &'static CardDef,
    test_bear: &'static CardDef,
    pump_spell: &'static CardDef,
}

static ANTHEM_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::PtModify,
    filter: CREATURE_YOU,
    modifier: Modifier::ModifyPT(1, 1),
    cross_zone: false,
})];

static LATTICE_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
    layer: Layer::Type,
    filter: Filter::Any,
    modifier: Modifier::AddType(TypeSet::ARTIFACT),
    cross_zone: false,
})];

static PUMP_EFFECTS: &[Effect] = &[Effect::CreateContinuousEffect {
    layer: Layer::PtModify,
    filter: &THIS_F,
    modifier: Modifier::ModifyPT(3, 3),
    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
}];

static PUMP_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
    effects: PUMP_EFFECTS,
    targets: Some(TargetReq::one(TargetSpec::Object(&CREATURE_F))),
}];

impl TestLookup {
    fn new() -> Self {
        let anthem_lord: &'static CardDef = Box::leak(Box::new(def(
            ANTHEM_LORD,
            face("Anthem Lord", "{1}{W}", TypeSet::CREATURE, Some((2, 2))),
            ANTHEM_ABILITIES,
        )));
        let fake_lattice: &'static CardDef = Box::leak(Box::new(def(
            FAKE_LATTICE,
            face("Fake Lattice", "{6}", TypeSet::ARTIFACT, None),
            LATTICE_ABILITIES,
        )));
        let test_bear: &'static CardDef = Box::leak(Box::new(def(
            TEST_BEAR,
            face("Test Bear", "{1}{G}", TypeSet::CREATURE, Some((2, 2))),
            &[],
        )));
        let pump_spell: &'static CardDef = Box::leak(Box::new(def(
            PUMP_SPELL,
            face("Pump Spell", "{G}", TypeSet::SORCERY, None),
            PUMP_ABILITIES,
        )));
        Self {
            anthem_lord,
            fake_lattice,
            test_bear,
            pump_spell,
        }
    }
}

impl CardLookup for TestLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        match index.get() {
            ANTHEM_LORD => Some(self.anthem_lord),
            FAKE_LATTICE => Some(self.fake_lattice),
            TEST_BEAR => Some(self.test_bear),
            PUMP_SPELL => Some(self.pump_spell),
            _ => baylee_cards::by_index(index),
        }
    }
}

fn entry(card: u32) -> DeckEntry {
    DeckEntry {
        card: CardIndex::new(card),
        print: PrintRef::new(0),
    }
}

fn preset_bf(seed: u64, bf0: &[u32], hand0: &[u32]) -> GamePreset {
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6").get();
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest)).collect();
    let mk = |bf: &[u32], hand: &[u32]| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        deck: deck.clone(),
        starting_life: None,
        starting_hand: Some(hand.iter().map(|c| entry(*c)).collect()),
        starting_battlefield: bf.iter().map(|c| entry(*c)).collect(),
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        dev_mode: false,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(bf0, hand0), mk(&[], &[])],
    }
}

fn keep_mulligans(engine: &mut Engine<TestLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

fn bear(engine: &Engine<TestLookup>) -> ObjectId {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == TEST_BEAR))
        })
        .expect("bear on battlefield")
}

fn power_toughness(engine: &Engine<TestLookup>, id: ObjectId) -> (i16, i16) {
    let c = engine.state().object(id).unwrap().characteristics();
    (c.power.unwrap(), c.toughness.unwrap())
}

#[test]
fn anthem_grants_and_structurally_removes() {
    let lookup = TestLookup::new();
    let mut engine = Engine::new(&preset_bf(7, &[ANTHEM_LORD, TEST_BEAR], &[]), lookup).unwrap();
    keep_mulligans(&mut engine);
    let bear = bear(&engine);
    assert_eq!(power_toughness(&engine, bear), (3, 3), "anthem applies");

    // Destroy the anthem lord → the effect deregisters structurally.
    let lord = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == ANTHEM_LORD))
        })
        .unwrap();
    sba::destroy(engine.state_mut_dev(), lord);
    // Force the machine to sync (any action works; concession is too
    // destructive, so pass priority).
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(power_toughness(&engine, bear), (2, 2), "anthem removed");
}

#[test]
fn lattice_turns_everything_into_artifacts() {
    let lookup = TestLookup::new();
    let mut engine = Engine::new(&preset_bf(8, &[FAKE_LATTICE, TEST_BEAR], &[]), lookup).unwrap();
    keep_mulligans(&mut engine);
    let bear = bear(&engine);
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ARTIFACT)
    );
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}

#[test]
fn nexus_makes_library_cards_allies() {
    let lookup = TestLookup::new();
    let nexus = card_index("9b2cdbed-c733-409b-b0e4-2c8960c25111").get();
    let chupacabra = card_index("7b459306-149b-4f43-abc1-2dd70c748c0e").get();
    // Deck: chupacabras (NOT Allies) mixed with forests.
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6").get();
    let deck: Vec<DeckEntry> = (0..60)
        .map(|i| entry(if i % 2 == 0 { chupacabra } else { forest }))
        .collect();
    let mut preset = preset_bf(11, &[nexus], &[]);
    preset.seats[0].deck = deck;
    preset.seats[1].deck = preset.seats[0].deck.clone();
    let mut engine = Engine::new(&preset, lookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);

    // Every chupacabra card in p0's library is an Ally now (cross-zone
    // projection), so a Tazri-style Ally search may offer it.
    let lib = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let chup = lib
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index.get() == chupacabra))
        })
        .expect("chupacabra in library");
    let c = engine.state().object(chup).unwrap().characteristics();
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ALLY),
        "library card must be an Ally under Maskwood Nexus"
    );

    // And the same search restricted to actual allies of the deck owner
    // would offer the card too (filter evaluated against projection).
    let matches_ally = crate::eval::matches(
        &Filter::And(&[
            Filter::HasSubtype(baylee_core::generated::subtypes::creature::ALLY),
            Filter::HasType(TypeSet::CREATURE),
        ]),
        engine.state(),
        engine.state().object(chup).unwrap(),
        p0,
        chup,
    );
    assert!(matches_ally);
}

#[test]
fn pump_lasts_until_end_of_turn() {
    let lookup = TestLookup::new();
    let forest = card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6").get();
    let mut engine =
        Engine::new(&preset_bf(9, &[TEST_BEAR, forest], &[PUMP_SPELL]), lookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bear = bear(&engine);

    // Walk to p0's main, tap the forest, cast the pump on the bear.
    let mut guard = 0;
    while !(matches!(engine.state().turn.phase, Phase::FirstMain)
        && engine.state().turn.active == p0)
    {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!()
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        guard += 1;
        assert!(guard < 20);
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!()
    };
    let pump = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    assert!(legal.castable.contains(&pump));
    engine
        .apply(p0, PlayerAction::CastSpell { card: pump })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected targets")
    };
    assert_eq!(options, vec![bear]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert_eq!(power_toughness(&engine, bear), (5, 5), "pump applied");

    // Walk until the turn ends (cleanup passes through instantly when no
    // discard is needed) — then the pump must be gone.
    let turn_before = engine.state().turn.number;
    let mut guard = 0;
    while engine.state().turn.number == turn_before {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
        guard += 1;
        assert!(guard < 100, "turn never ended");
    }
    assert_eq!(
        power_toughness(&engine, bear),
        (2, 2),
        "pump expired at cleanup"
    );
}
