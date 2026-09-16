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

use super::*;
use baylee_cards_dsl::{
    AbilityDef, ActivationLimit, ActivationTiming, ActivationZone, CardDef, CommanderRule, Cost,
    Coverage, Effect, FaceDef, Filter, KeywordSet, Layer, ManaColor, Modifier, StaticAbility,
};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};
use baylee_core::types::{SupertypeSet, TypeSet};

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

fn face(name: &'static str) -> FaceDef {
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
        cost_reduction: None,
        disturb: false,
        adventure: false,
    }
}

fn def(index: u32, name: &'static str, abilities: &'static [AbilityDef]) -> CardDef {
    CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([face(name)])),
        color_identity: baylee_core::color::ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }
}

struct TestLookup {
    storage: &'static CardDef,
    frozen: &'static CardDef,
}

impl TestLookup {
    fn new() -> Self {
        Self {
            storage: Box::leak(Box::new(def(
                STORAGE_BASIN,
                "Storage Basin",
                BASIN_ABILITIES,
            ))),
            frozen: Box::leak(Box::new(def(
                FROZEN_BASIN,
                "Frozen Basin",
                FROZEN_ABILITIES,
            ))),
        }
    }
}

impl CardLookup for TestLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        match index.get() {
            STORAGE_BASIN => Some(self.storage),
            FROZEN_BASIN => Some(self.frozen),
            _ => baylee_cards::by_index(index),
        }
    }
}

fn forest() -> u32 {
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

fn preset(seed: u64, battlefield: &[u32]) -> GamePreset {
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

fn permanents(engine: &Engine<TestLookup>, index: u32) -> Vec<ObjectId> {
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

fn tapped(engine: &Engine<TestLookup>, id: ObjectId) -> bool {
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
fn tap_every_land(engine: &mut Engine<TestLookup>, seat: PlayerId) {
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

/// Passes priority until the untap step asks its question, or until the
/// game has gone round too far for one to be coming.
///
/// Bounded rather than `loop`: a question that never arrives is the whole
/// failure this is testing for, and a hang reports it as a timeout with no
/// state to read.
fn pass_until_question(engine: &mut Engine<TestLookup>) -> bool {
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

/// Answers the questions a turn asks on the way round, and nothing else.
///
/// Priority is passed and combat is declined; anything else is a question
/// this board should not be able to ask, and the caller says so. Listing
/// them rather than answering whatever arrives is what keeps the untap
/// question from being swallowed by a driver that answers everything.
fn walk_past(engine: &mut Engine<TestLookup>, pending: &Pending) -> bool {
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
    let mut engine = Engine::new(&preset(11, &[STORAGE_BASIN, f, f]), TestLookup::new()).unwrap();
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
    let mut engine = Engine::new(&preset(12, &[STORAGE_BASIN, f]), TestLookup::new()).unwrap();
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
    let mut engine = Engine::new(&preset(13, &[FROZEN_BASIN, f]), TestLookup::new()).unwrap();
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
