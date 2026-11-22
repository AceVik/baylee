//! Engine Wave 11b mechanics tests:
//! 1. `CostPart::RemoveAnyCounter` & `CostPart::RemoveAnyCounterX` (`CR 118.3`, `CR 601.2h`)
//! 2. `Effect::Vote`, `Effect::VoteObjects`, & `Modifier::AdditionalVotes` (`CR 701.38a–d`)
//! 3. `Effect::ReduceNextSpellCost` (`CR 601.2f`, `CR 118.7`)
//! 4. `Effect::Play` (`CR 608.2g`, `CR 707.12`, trailing `SubAbility` continuation)
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, ChoicePrompt, NumberPrompt, YesNoPrompt};
use crate::event::GameEvent;
use crate::resolve::OptionBallot;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::counters::QUEST;
use baylee_cards_dsl::prelude::static_ability;
use baylee_cards_dsl::{
    Amount, CostPart, CounterKind, Duration, Filter, Modifier, PlayCostMode, PlayCount, PlaySource,
    PlayerRel, TargetChooser, TargetSpec, ThisWay, VoteObjectAction, VoteOption, VoteOutcome,
    ZoneRef,
};
use baylee_core::mana::ManaColor;

const SELF_ANY_COUNTER_RELIC: u32 = 9600;
const TAYAM_SHRINE: u32 = 9601;
const SYLEX_BOMB: u32 = 9602;
const X_COUNTER_CANNON: u32 = 9603;
const COUNTER_COST_SORCERY: u32 = 9604;
const VANILLA_ALLY_A: u32 = 9605;
const VANILLA_ALLY_B: u32 = 9606;
const COUNCIL_VERDICT: u32 = 9607;
const DILEMMA_PLEA: u32 = 9608;
const BRAGO_REP: u32 = 9609;
const BALLOT_BROKER_CREATURE: u32 = 9610;
const SECRET_EXPROPRIATE: u32 = 9611;
const OBJECT_VOTE_JUDGMENT: u32 = 9612;
const NEXT_SPELL_TUTOR: u32 = 9613;
const TURN_RED_DISCOUNTER: u32 = 9614;
const EXPENSIVE_INSTANT: u32 = 9615;
const RED_BOLT_SORCERY_A: u32 = 9616;
const RED_BOLT_SORCERY_B: u32 = 9617;
const PLAY_HAND_SUBABILITY: u32 = 9618;
const BIG_PAYOFF_SORCERY: u32 = 9619;
const COPY_GY_MASTERY: u32 = 9620;
const ALT_COST_GRANTOR: u32 = 9621;

static INSTANT_OR_SORCERY_FILTER: Filter = Filter::Or(&[Filter::INSTANT, Filter::SORCERY]);
static NONLAND_CARD_FILTER: Filter = Filter::Not(&Filter::LAND);

fn refresh_offer(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn first_of(engine: &Bench, owner: PlayerId, idx: u32, zone: Zone) -> ObjectId {
    let loc = match zone {
        Zone::Battlefield => ZoneLocation::Battlefield,
        Zone::Hand => ZoneLocation::Hand(owner),
        Zone::Graveyard => ZoneLocation::Graveyard(owner),
        Zone::Exile => ZoneLocation::Exile(owner),
        Zone::Library => ZoneLocation::Library(owner),
        Zone::Stack => ZoneLocation::Stack,
        Zone::Command => ZoneLocation::Command(owner),
        Zone::OutsideGame => unreachable!("no OutsideGame zone in wave11b tests"),
    };
    engine
        .state()
        .zones
        .list(loc)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.card.is_some_and(|c| c.index.get() == idx)
                    && (zone != Zone::Battlefield || o.controller == owner)
            })
        })
        .unwrap_or_else(|| panic!("missing card {idx} in {zone:?} for {owner:?}"))
}

fn resolve_all_stack(engine: &mut Bench) {
    let mut safety = 0;
    while !engine.state().zones.list(ZoneLocation::Stack).is_empty() {
        safety += 1;
        assert!(
            safety < 64,
            "stack resolution did not settle: {:?}",
            engine.pending()
        );
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected non-priority pending while resolving stack: {other:?}"),
        }
    }
}

// ============================================================================
// 1. `CostPart::RemoveAnyCounter` & `CostPart::RemoveAnyCounterX`
// ============================================================================

#[test]
fn remove_any_counter_self_auto_picks_single_kind_and_disambiguates_multiple_kinds() {
    static PARTS: &[CostPart] = &[CostPart::RemoveAnyCounter {
        count: 1,
        kind: None,
        filter: &Filter::This,
    }];
    static EFFECTS: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }];
    static ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: PARTS,
        },
        EFFECTS,
        None,
    )];

    let defs = vec![card(
        SELF_ANY_COUNTER_RELIC,
        face("Counter Conduit", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ABILITIES,
    )];
    let mut e = bench(
        9600,
        defs,
        [Seat::with(&[SELF_ANY_COUNTER_RELIC]), Seat::default()],
    );
    to_main(&mut e, me());
    let relic = first_of(&e, me(), SELF_ANY_COUNTER_RELIC, Zone::Battlefield);

    // Negative boundary: 0 counters on `relic` -> ability is NOT legal.
    let (_, legal) = offer(&e);
    assert!(
        !legal.abilities.contains(&(relic, 0)),
        "RemoveAnyCounter<1/Any/This> must not be offered with 0 counters"
    );

    // Put 2 +1/+1 counters on `relic`: single counter kind auto-picks with zero extra prompts.
    e.state
        .object_mut(relic)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    refresh_offer(&mut e);
    e.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: relic,
            ability_index: 0,
        },
    )
    .expect("single-kind self counter removal activates without extra prompts");
    assert_eq!(
        e.state()
            .object(relic)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 22);

    // Add 1 Charge counter alongside the remaining +1/+1 counter: must ask `CastModeKind::Counter`.
    e.state
        .object_mut(relic)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 1);
    refresh_offer(&mut e);
    e.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: relic,
            ability_index: 0,
        },
    )
    .expect("begins activation");
    let Pending::ChooseCastMode {
        player,
        object,
        options,
    } = e.pending().clone()
    else {
        panic!(
            "expected counter-kind disambiguation, got {:?}",
            e.pending()
        );
    };
    assert_eq!((player, object), (me(), relic));
    let charge_pos = options
        .iter()
        .position(|o| o.kind == CastModeKind::Counter(CounterKind::Charge))
        .expect("Charge counter offered");
    assert!(
        options
            .iter()
            .any(|o| o.kind == CastModeKind::Counter(CounterKind::P1P1)),
        "+1/+1 counter also offered"
    );
    e.apply(me(), PlayerAction::ChooseMode(charge_pos))
        .expect("choose Charge counter");
    assert_eq!(
        e.state()
            .object(relic)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        0
    );
    assert_eq!(
        e.state()
            .object(relic)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
}

#[test]
fn remove_any_counter_distributes_across_permanents_and_allows_self_sacrifice() {
    static TAYAM_PARTS: &[CostPart] = &[CostPart::RemoveAnyCounter {
        count: 3,
        kind: None,
        filter: &Filter::CREATURE,
    }];
    static SYLEX_PARTS: &[CostPart] = &[
        CostPart::TapSelf,
        CostPart::RemoveAnyCounter {
            count: 2,
            kind: Some(QUEST),
            filter: &Filter::Any,
        },
        CostPart::SacrificeSelf,
    ];
    static GAIN_THREE: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(3),
    }];
    static GAIN_FIVE: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(5),
    }];
    static TAYAM_ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: TAYAM_PARTS,
        },
        GAIN_THREE,
        None,
    )];
    static SYLEX_ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: SYLEX_PARTS,
        },
        GAIN_FIVE,
        None,
    )];

    let defs = vec![
        card(
            TAYAM_SHRINE,
            face("Enigma Altar", "{3}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            TAYAM_ABILITIES,
        ),
        card(
            SYLEX_BOMB,
            face("Quest Sylex", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            SYLEX_ABILITIES,
        ),
        card(
            VANILLA_ALLY_A,
            creature_face("Ally Alpha", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            VANILLA_ALLY_B,
            creature_face("Ally Beta", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut e = bench(
        9601,
        defs,
        [
            Seat::with(&[TAYAM_SHRINE, SYLEX_BOMB, VANILLA_ALLY_A, VANILLA_ALLY_B]),
            Seat::with(&[VANILLA_ALLY_A]),
        ],
    );
    to_main(&mut e, me());

    let altar = first_of(&e, me(), TAYAM_SHRINE, Zone::Battlefield);
    let sylex = first_of(&e, me(), SYLEX_BOMB, Zone::Battlefield);
    let ally_a = first_of(&e, me(), VANILLA_ALLY_A, Zone::Battlefield);
    let ally_b = first_of(&e, me(), VANILLA_ALLY_B, Zone::Battlefield);
    let enemy_ally = first_of(&e, them(), VANILLA_ALLY_A, Zone::Battlefield);

    // Opponent's counters never count toward our 3-counter requirement.
    e.state
        .object_mut(enemy_ally)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 5);
    e.state
        .object_mut(ally_a)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    refresh_offer(&mut e);

    // Only 2 counters on our creatures -> Tayam ability is NOT affordable yet.
    assert!(
        !offer(&e).1.abilities.contains(&(altar, 0)),
        "2 counters cannot pay RemoveAnyCounter(3)"
    );

    // Give `ally_b` 1 Shield + 1 Charge counter -> now 4 counters total across our creatures.
    e.state
        .object_mut(ally_b)
        .unwrap()
        .counters
        .set(CounterKind::Shield, 1);
    e.state
        .object_mut(ally_b)
        .unwrap()
        .counters
        .set(CounterKind::Charge, 1);
    refresh_offer(&mut e);

    e.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: altar,
            ability_index: 0,
        },
    )
    .expect("3-counter activation starts");

    // Pick 1: `ally_a` (auto-picks P1P1).
    let Pending::ChooseCards {
        options, prompt, ..
    } = e.pending().clone()
    else {
        panic!("expected CostRemoveCounter ChooseCards");
    };
    assert_eq!(prompt, ChoicePrompt::CostRemoveCounter);
    assert_eq!(options, vec![ally_a, ally_b]);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_a],
        },
    )
    .unwrap();

    // Pick 2: `ally_a` still has 1 counter left, so both `ally_a` and `ally_b` remain on the menu!
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected second CostRemoveCounter ChooseCards");
    };
    assert_eq!(options, vec![ally_a, ally_b]);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_a],
        },
    )
    .unwrap();

    // Pick 3: `ally_a` is exhausted, so only `ally_b` is offered; `ally_b` has 2 counter kinds!
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected third CostRemoveCounter ChooseCards");
    };
    assert_eq!(options, vec![ally_b]);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_b],
        },
    )
    .unwrap();

    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("expected counter kind disambiguation on ally_b");
    };
    let shield_idx = options
        .iter()
        .position(|o| o.kind == CastModeKind::Counter(CounterKind::Shield))
        .expect("Shield offered");
    e.apply(me(), PlayerAction::ChooseMode(shield_idx)).unwrap();

    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 23);
    assert_eq!(
        e.state()
            .object(ally_a)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        e.state()
            .object(ally_b)
            .unwrap()
            .counters
            .get(CounterKind::Shield),
        0
    );
    assert_eq!(
        e.state()
            .object(ally_b)
            .unwrap()
            .counters
            .get(CounterKind::Charge),
        1
    );
    assert_eq!(
        e.state()
            .object(enemy_ally)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        5
    );

    // Sylex pattern: `{T}, Remove two quest counters from among permanents you control, Sacrifice ~`.
    e.state.object_mut(sylex).unwrap().counters.set(QUEST, 2);
    refresh_offer(&mut e);
    e.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: sylex,
            ability_index: 0,
        },
    )
    .expect("Sylex can spend its own counters before sacrificing itself");
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![sylex],
        },
    )
    .unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![sylex],
        },
    )
    .unwrap();
    assert_eq!(e.state().object(sylex).unwrap().zone, Zone::Graveyard);
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 28);
}

#[test]
fn remove_any_counter_x_bounds_activation_x_and_spell_cost_rolls_back_on_cancel() {
    static CANNON_PARTS: &[CostPart] = &[CostPart::RemoveAnyCounterX {
        kind: Some(CounterKind::P1P1),
        filter: &Filter::CREATURE,
    }];
    static CANNON_FX: &[Effect] = &[Effect::GainLife { amount: Amount::X }];
    static CANNON_ABILITIES: &[AbilityDef] = &[paid(
        Cost {
            mana: ManaCost::ZERO,
            parts: CANNON_PARTS,
        },
        CANNON_FX,
        None,
    )];
    static SPELL_MANDATORY: &[CostPart] = &[CostPart::RemoveAnyCounter {
        count: 2,
        kind: Some(CounterKind::P1P1),
        filter: &Filter::CREATURE,
    }];
    static SPELL_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(4),
    }];
    static SPELL_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: SPELL_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            X_COUNTER_CANNON,
            face("Counter Cannon", "{2}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            CANNON_ABILITIES,
        ),
        card(
            COUNTER_COST_SORCERY,
            FaceDef {
                mandatory_additional_costs: SPELL_MANDATORY,
                ..face("Harvest Growth", "{G}", TypeSet::SORCERY)
            },
            KeywordSet::EMPTY,
            SPELL_ABILITIES,
        ),
        card(
            VANILLA_ALLY_A,
            creature_face("Ally Alpha", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            VANILLA_ALLY_B,
            creature_face("Ally Beta", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut e = bench(
        9602,
        defs,
        [
            Seat::with(&[X_COUNTER_CANNON, VANILLA_ALLY_A, VANILLA_ALLY_B, forest()])
                .holding(&[COUNTER_COST_SORCERY]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());

    let cannon = first_of(&e, me(), X_COUNTER_CANNON, Zone::Battlefield);
    let ally_a = first_of(&e, me(), VANILLA_ALLY_A, Zone::Battlefield);
    let ally_b = first_of(&e, me(), VANILLA_ALLY_B, Zone::Battlefield);
    let sorcery = first_of(&e, me(), COUNTER_COST_SORCERY, Zone::Hand);
    let land = first_of(&e, me(), forest(), Zone::Battlefield);

    e.state
        .object_mut(ally_a)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 2);
    e.state
        .object_mut(ally_b)
        .unwrap()
        .counters
        .set(CounterKind::P1P1, 1);
    refresh_offer(&mut e);

    // Activate `RemoveAnyCounterX`: max X must be bounded at 3!
    e.apply(
        me(),
        PlayerAction::ActivateAbility {
            source: cannon,
            ability_index: 0,
        },
    )
    .unwrap();
    let Pending::ChooseNumber {
        min, max, reason, ..
    } = e.pending().clone()
    else {
        panic!("expected ChooseNumber for X");
    };
    assert_eq!((min, max, reason), (0, 3, NumberPrompt::X));
    e.apply(me(), PlayerAction::ChooseNumber(1)).unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_a],
        },
    )
    .unwrap();
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 21);
    assert_eq!(
        e.state()
            .object(ally_a)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );

    // Now cast `COUNTER_COST_SORCERY` (`{G}` + remove 2 +1/+1 counters) before tapping `land`,
    // choose both creatures in `WizardStage::RemoveCounters`, then cancel in the mana window!
    e.apply(me(), PlayerAction::CastSpell { card: sorcery })
        .expect("spell is payable via untapped Forest + 2 counters");
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_a],
        },
    )
    .unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_b],
        },
    )
    .unwrap();
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "entered mana payment window"
    );
    e.apply(me(), PlayerAction::CancelCast)
        .expect("CancelCast rewinds cleanly");
    assert_eq!(e.state().object(sorcery).unwrap().zone, Zone::Hand);
    assert_eq!(
        e.state()
            .object(ally_a)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(
        e.state()
            .object(ally_b)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );

    // Tap `land` and cast `sorcery` to completion.
    e.apply(me(), PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    e.apply(me(), PlayerAction::CastSpell { card: sorcery })
        .unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_a],
        },
    )
    .unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![ally_b],
        },
    )
    .unwrap();
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 25);
    assert_eq!(
        e.state()
            .object(ally_a)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
    assert_eq!(
        e.state()
            .object(ally_b)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0
    );
}

// ============================================================================
// 2. `Effect::Vote`, `Effect::VoteObjects`, & `Modifier::AdditionalVotes`
// ============================================================================

#[test]
fn vote_will_of_the_council_majority_vs_tie_and_public_turn_order() {
    static GRACE_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(5),
    }];
    static CONDEMN_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    static OPTIONS: &[VoteOption] = &[
        VoteOption {
            name: "grace",
            effects: GRACE_FX,
        },
        VoteOption {
            name: "condemnation",
            effects: CONDEMN_FX,
        },
    ];
    static VOTE_FX: &[Effect] = &[Effect::Vote {
        options: OPTIONS,
        outcome: VoteOutcome::MajorityOrTied { tied_option: 1 },
        secret: false,
    }];
    static ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: VOTE_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![card(
        COUNCIL_VERDICT,
        face("Council Verdict", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        ABILITIES,
    )];
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));

    // 3-seat vote: P0 and P2 vote `grace` (0), P1 votes `condemnation` (1) -> `grace` wins 2–1.
    let mut e = bench(
        9603,
        defs.clone(),
        [
            Seat::default().holding(&[COUNCIL_VERDICT]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut e, p0);
    let spell = first_of(&e, p0, COUNCIL_VERDICT, Zone::Hand);
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(p2, PlayerAction::PassPriority).unwrap();

    assert!(e.option_ballots().is_empty());
    let mut expected_ballots = Vec::new();
    for (voter, choice) in [(p0, 0usize), (p1, 1usize), (p2, 0usize)] {
        let Pending::ChooseCastMode {
            player, options, ..
        } = e.pending().clone()
        else {
            panic!("expected VoteOption ChooseCastMode, got {:?}", e.pending());
        };
        assert_eq!(player, voter, "APNAP turn order starting with controller");
        assert_eq!(options[0].kind, CastModeKind::VoteOption(0));
        assert_eq!(options[1].kind, CastModeKind::VoteOption(1));
        assert_eq!(e.option_ballots(), expected_ballots.as_slice());
        e.apply(voter, PlayerAction::ChooseMode(choice)).unwrap();
        expected_ballots.push(OptionBallot {
            voter,
            option: u8::try_from(choice).unwrap(),
        });
    }
    assert!(e.option_ballots().is_empty(), "vote finished");
    assert_eq!(e.state().players[0].life, 25, "majority option 0 ran");

    // 2-seat tie: P0 votes `grace` (0), P1 votes `condemnation` (1) -> tie selects `tied_option: 1`.
    let mut tie_game = bench(
        9604,
        defs,
        [Seat::default().holding(&[COUNCIL_VERDICT]), Seat::default()],
    );
    to_main(&mut tie_game, p0);
    let spell2 = first_of(&tie_game, p0, COUNCIL_VERDICT, Zone::Hand);
    tie_game
        .apply(p0, PlayerAction::CastSpell { card: spell2 })
        .unwrap();
    tie_game.apply(p0, PlayerAction::PassPriority).unwrap();
    tie_game.apply(p1, PlayerAction::PassPriority).unwrap();
    tie_game.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    tie_game.apply(p1, PlayerAction::ChooseMode(1)).unwrap();
    assert_eq!(
        tie_game.state().players[0].life,
        21,
        "1-1 tie selected tied_option: 1"
    );
}

#[test]
fn vote_councils_dilemma_this_way_vote_count_and_additional_votes() {
    static FEAST_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::ThisWay(ThisWay::VoteCount(0)),
    }];
    static LORE_FX: &[Effect] = &[Effect::DrawCards {
        amount: Amount::ThisWay(ThisWay::VoteCount(1)),
    }];
    static OPTIONS: &[VoteOption] = &[
        VoteOption {
            name: "feast",
            effects: FEAST_FX,
        },
        VoteOption {
            name: "lore",
            effects: LORE_FX,
        },
    ];
    static PLEA_FX: &[Effect] = &[Effect::Vote {
        options: OPTIONS,
        outcome: VoteOutcome::PerOptionCount,
        secret: false,
    }];
    static PLEA_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: PLEA_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static REP_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::AdditionalVotes {
            count: 1,
            optional: false,
        }
    )];
    static BROKER_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::This,
        Modifier::AdditionalVotes {
            count: 1,
            optional: true,
        }
    )];

    let defs = vec![
        card(
            DILEMMA_PLEA,
            face("Dilemma Plea", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            PLEA_ABILITIES,
        ),
        card(
            BRAGO_REP,
            creature_face("Council Rep", "{2}{W}", 1, 4),
            KeywordSet::EMPTY,
            REP_ABILITIES,
        ),
        card(
            BALLOT_BROKER_CREATURE,
            creature_face("Ballot Broker", "{3}{W}", 2, 3),
            KeywordSet::EMPTY,
            BROKER_ABILITIES,
        ),
    ];
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = bench(
        9605,
        defs,
        [
            Seat::with(&[BRAGO_REP, BALLOT_BROKER_CREATURE]).holding(&[DILEMMA_PLEA]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut e, p0);
    let hand_before = e.state().zones.list(ZoneLocation::Hand(p0)).len() - 1;
    let spell = first_of(&e, p0, DILEMMA_PLEA, Zone::Hand);
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(p2, PlayerAction::PassPriority).unwrap();

    // P0 gets 2 mandatory votes, then is asked `YesNoPrompt::AdditionalVote`, accepts, and casts a 3rd!
    e.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    e.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    assert!(matches!(
        e.pending(),
        Pending::YesNo {
            player,
            prompt: YesNoPrompt::AdditionalVote,
            ..
        } if *player == p0
    ));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    e.apply(p0, PlayerAction::ChooseMode(1)).unwrap();

    // P1 votes 0 (`feast`), P2 votes 1 (`lore`) -> total: 3 `feast` votes, 2 `lore` votes.
    e.apply(p1, PlayerAction::ChooseMode(0)).unwrap();
    e.apply(p2, PlayerAction::ChooseMode(1)).unwrap();

    assert_eq!(e.state().players[0].life, 23);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
}

#[test]
fn vote_repeat_per_vote_binds_voter_as_event_player_and_secret_vote_hides_ballots_until_done() {
    static TRIBUTE_FX: &[Effect] = &[Effect::LoseLife {
        target: PlayerRel::EventPlayer,
        amount: Amount::Fixed(3),
    }];
    static WEALTH_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(4),
    }];
    static OPTIONS: &[VoteOption] = &[
        VoteOption {
            name: "tribute",
            effects: TRIBUTE_FX,
        },
        VoteOption {
            name: "wealth",
            effects: WEALTH_FX,
        },
    ];
    static SECRET_FX: &[Effect] = &[Effect::Vote {
        options: OPTIONS,
        outcome: VoteOutcome::RepeatPerVote,
        secret: true,
    }];
    static ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: SECRET_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![card(
        SECRET_EXPROPRIATE,
        face("Secret Levy", "{0}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        ABILITIES,
    )];
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut e = bench(
        9606,
        defs,
        [
            Seat::default().holding(&[SECRET_EXPROPRIATE]),
            Seat::default(),
            Seat::default(),
        ],
    );
    to_main(&mut e, p0);
    let spell = first_of(&e, p0, SECRET_EXPROPRIATE, Zone::Hand);
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(p2, PlayerAction::PassPriority).unwrap();

    let journal_before = e.state().journal.entries().len();
    e.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    assert!(
        e.option_ballots().is_empty(),
        "secret vote must not expose ballots while voting is in progress"
    );
    assert!(
        !e.state().journal.entries()[journal_before..]
            .iter()
            .any(|en| matches!(en.event, GameEvent::VotedOption { .. })),
        "secret vote must not journal VotedOption before all voters finish"
    );

    e.apply(p1, PlayerAction::ChooseMode(0)).unwrap();
    assert!(e.option_ballots().is_empty());

    e.apply(p2, PlayerAction::ChooseMode(0)).unwrap();

    let revealed: Vec<_> = e.state().journal.entries()[journal_before..]
        .iter()
        .filter_map(|en| match en.event {
            GameEvent::VotedOption { player, option, .. } => Some((player, option)),
            _ => None,
        })
        .collect();
    assert_eq!(revealed, vec![(p0, 1), (p1, 0), (p2, 0)]);
    assert_eq!(e.state().players[0].life, 24, "P0 gained 4 from wealth");
    assert_eq!(e.state().players[1].life, 17, "P1 lost 3 as EventPlayer");
    assert_eq!(e.state().players[2].life, 17, "P2 lost 3 as EventPlayer");
}

#[test]
fn vote_objects_supports_destroy_ties_and_if_no_votes_fallback() {
    static FALLBACK: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(5),
    }];
    static VOTE_OBJ_FX: &[Effect] = &[Effect::VoteObjects {
        filter: &Filter::CREATURE,
        zone: ZoneRef::Battlefield,
        optional: true,
        secret: false,
        action: VoteObjectAction::Destroy { no_regen: true },
        if_no_votes: FALLBACK,
    }];
    static ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: VOTE_OBJ_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            OBJECT_VOTE_JUDGMENT,
            face("Trial by Jury", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            ABILITIES,
        ),
        card(
            VANILLA_ALLY_A,
            creature_face("Suspect Alpha", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            VANILLA_ALLY_B,
            creature_face("Suspect Beta", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];

    // Case 1: Both players decline (`optional: true`) -> `if_no_votes` runs (+5 life), nobody dies.
    let mut e = bench(
        9607,
        defs,
        [
            Seat::with(&[VANILLA_ALLY_A]).holding(&[OBJECT_VOTE_JUDGMENT, OBJECT_VOTE_JUDGMENT]),
            Seat::with(&[VANILLA_ALLY_B]),
        ],
    );
    to_main(&mut e, me());
    let suspect_a = first_of(&e, me(), VANILLA_ALLY_A, Zone::Battlefield);
    let suspect_b = first_of(&e, them(), VANILLA_ALLY_B, Zone::Battlefield);
    let trial_1 = objects(&e, ZoneLocation::Hand(me()), OBJECT_VOTE_JUDGMENT)[0];
    e.apply(me(), PlayerAction::CastSpell { card: trial_1 })
        .unwrap();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();
    e.apply(me(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    e.apply(them(), PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    assert_eq!(e.state().players[0].life, 25, "if_no_votes fallback ran");
    assert_eq!(e.state().object(suspect_a).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(suspect_b).unwrap().zone, Zone::Battlefield);

    // Case 2: P0 votes for `suspect_b`, P1 votes for `suspect_a` -> 1-1 tie destroys BOTH!
    let trial_2 = first_of(&e, me(), OBJECT_VOTE_JUDGMENT, Zone::Hand);
    e.apply(me(), PlayerAction::CastSpell { card: trial_2 })
        .unwrap();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![suspect_b],
        },
    )
    .unwrap();
    e.apply(
        them(),
        PlayerAction::ChooseObjects {
            objects: vec![suspect_a],
        },
    )
    .unwrap();
    assert_eq!(e.state().object(suspect_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(suspect_b).unwrap().zone, Zone::Graveyard);
}

// ============================================================================
// 3. `Effect::ReduceNextSpellCost`
// ============================================================================

#[test]
fn reduce_next_spell_cost_next_only_skips_nonmatching_survives_cancel_and_consumes_once() {
    static TUTOR_FX: &[Effect] = &[Effect::ReduceNextSpellCost {
        who: PlayerRel::You,
        filter: &INSTANT_OR_SORCERY_FILTER,
        generic: Amount::Fixed(2),
        mana: ManaCost::ZERO,
        colored_only: false,
        duration: Duration::UntilEndOfTurn,
        next_only: true,
    }];
    static TUTOR_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: TUTOR_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static GAIN_ONE: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(1),
    }];
    static INSTANT_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: GAIN_ONE,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            NEXT_SPELL_TUTOR,
            face("Insight Prep", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            TUTOR_ABILITIES,
        ),
        card(
            EXPENSIVE_INSTANT,
            face("Arcane Bolt", "{2}{G}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            INSTANT_ABILITIES,
        ),
        card(
            VANILLA_ALLY_A,
            creature_face("Moss Bear", "{1}{G}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
    ];
    let mut e = bench(
        9608,
        defs,
        [
            Seat::with(&[forest(), forest(), forest()]).holding(&[
                NEXT_SPELL_TUTOR,
                VANILLA_ALLY_A,
                EXPENSIVE_INSTANT,
                EXPENSIVE_INSTANT,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());

    let prep = first_of(&e, me(), NEXT_SPELL_TUTOR, Zone::Hand);
    e.apply(me(), PlayerAction::CastSpell { card: prep })
        .unwrap();
    resolve_all_stack(&mut e);

    let bear = first_of(&e, me(), VANILLA_ALLY_A, Zone::Hand);
    let instants = objects(&e, ZoneLocation::Hand(me()), EXPENSIVE_INSTANT);
    let (inst_1, inst_2) = (instants[0], instants[1]);

    // `LegalActions::spell_reductions` reports 2 on `inst_1`, 0 on `bear`.
    let (_, legal) = offer(&e);
    assert_eq!(legal.spell_reduction(inst_1, CastModeKind::Normal), 2);
    assert_eq!(legal.spell_reduction(bear, CastModeKind::Normal), 0);

    // Casting the non-matching creature pays full `{1}{G}` (2 Forests) and does NOT consume the discount!
    let forests = objects(&e, ZoneLocation::Battlefield, forest());
    e.apply(
        me(),
        PlayerAction::ActivateManaAbility { source: forests[0] },
    )
    .unwrap();
    e.apply(
        me(),
        PlayerAction::ActivateManaAbility { source: forests[1] },
    )
    .unwrap();
    e.apply(me(), PlayerAction::CastSpell { card: bear })
        .unwrap();
    resolve_all_stack(&mut e);

    let (_, legal) = offer(&e);
    assert_eq!(
        legal.spell_reduction(inst_1, CastModeKind::Normal),
        2,
        "non-matching creature spell must not consume next_only instant/sorcery discount"
    );

    // Start casting `inst_1` (`{2}{G}` discounted to `{G}`) with `forests[2]` untapped, then `CancelCast`!
    e.apply(me(), PlayerAction::CastSpell { card: inst_1 })
        .unwrap();
    e.apply(me(), PlayerAction::CancelCast).unwrap();
    let (_, legal) = offer(&e);
    assert_eq!(
        legal.spell_reduction(inst_1, CastModeKind::Normal),
        2,
        "CancelCast must not consume next_only discount"
    );

    // Tap `forests[2]` for `{G}` and cast `inst_1` (`{2}{G}` -> `{G}`)!
    e.apply(
        me(),
        PlayerAction::ActivateManaAbility { source: forests[2] },
    )
    .unwrap();
    e.apply(me(), PlayerAction::CastSpell { card: inst_1 })
        .unwrap();
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 21);

    // Discount was consumed: `inst_2` now has 0 reduction!
    let (_, legal) = offer(&e);
    assert_eq!(
        legal.spell_reduction(inst_2, CastModeKind::Normal),
        0,
        "next_only discount consumed after one matching spell cast"
    );
}

#[test]
fn reduce_next_spell_cost_duration_mode_and_colored_pips_discount_multiple_spells() {
    static DISCOUNTER_FX: &[Effect] = &[Effect::ReduceNextSpellCost {
        who: PlayerRel::You,
        filter: &INSTANT_OR_SORCERY_FILTER,
        generic: Amount::Fixed(1),
        mana: ManaCost::parse("{R}"),
        colored_only: false,
        duration: Duration::UntilEndOfTurn,
        next_only: false,
    }];
    static DISCOUNTER_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: DISCOUNTER_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PING_SELF_GAIN: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(2),
    }];
    static RED_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: PING_SELF_GAIN,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            TURN_RED_DISCOUNTER,
            face("Pyretic Ritualist", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            DISCOUNTER_ABILITIES,
        ),
        card(
            RED_BOLT_SORCERY_A,
            face("Crimson Spark A", "{1}{R}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            RED_ABILITIES,
        ),
        card(
            RED_BOLT_SORCERY_B,
            face("Crimson Spark B", "{1}{R}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            RED_ABILITIES,
        ),
    ];
    let mut e = bench(
        9609,
        defs,
        [
            Seat::default().holding(&[TURN_RED_DISCOUNTER, RED_BOLT_SORCERY_A, RED_BOLT_SORCERY_B]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());

    let ritual = first_of(&e, me(), TURN_RED_DISCOUNTER, Zone::Hand);
    let spark_a = first_of(&e, me(), RED_BOLT_SORCERY_A, Zone::Hand);
    let spark_b = first_of(&e, me(), RED_BOLT_SORCERY_B, Zone::Hand);

    e.apply(me(), PlayerAction::CastSpell { card: ritual })
        .unwrap();
    resolve_all_stack(&mut e);

    // Both `{1}{R}` sorceries are discounted by `{1}` + `{R}` = `{0}` in the same turn!
    e.apply(me(), PlayerAction::CastSpell { card: spark_a })
        .expect("first {1}{R} spell costs {0}");
    resolve_all_stack(&mut e);
    e.apply(me(), PlayerAction::CastSpell { card: spark_b })
        .expect("second {1}{R} spell also costs {0} when next_only: false");
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 24);
}

// ============================================================================
// 4. `Effect::Play` (SubAbility continuation, Copy, Controller, Alternative)
// ============================================================================

#[test]
fn effect_play_free_cast_resumes_trailing_subability_and_applies_exile_rider() {
    static PLAY_CHAIN: &[Effect] = &[
        Effect::Play {
            source: PlaySource::Zone {
                zone: ZoneRef::Hand,
                owner: PlayerRel::You,
            },
            filter: &NONLAND_CARD_FILTER,
            controller: PlayerRel::You,
            count: PlayCount::One,
            cost: PlayCostMode::WithoutPayingManaCost,
            cast_only: true,
            copy: false,
            exile_instead_of_graveyard: true,
            optional: true,
        },
        Effect::GainLife {
            amount: Amount::Fixed(7),
        },
    ];
    static PLAY_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: PLAY_CHAIN,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static PAYOFF_FX: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(10),
    }];
    static PAYOFF_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: PAYOFF_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            PLAY_HAND_SUBABILITY,
            face("Fires of Discovery", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            PLAY_ABILITIES,
        ),
        card(
            BIG_PAYOFF_SORCERY,
            face("Apex Blessing", "{5}{G}{G}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            PAYOFF_ABILITIES,
        ),
    ];
    let mut e = bench(
        9610,
        defs,
        [
            Seat::default().holding(&[PLAY_HAND_SUBABILITY, BIG_PAYOFF_SORCERY]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());

    let enabler = first_of(&e, me(), PLAY_HAND_SUBABILITY, Zone::Hand);
    let payoff = first_of(&e, me(), BIG_PAYOFF_SORCERY, Zone::Hand);

    e.apply(me(), PlayerAction::CastSpell { card: enabler })
        .unwrap();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();

    // `Effect::Play` suspends mid-resolution offering `payoff` (and NOT any Forests in hand!).
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected PlayCards ChooseCards, got {:?}", e.pending());
    };
    assert_eq!(options, vec![payoff]);
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![payoff],
        },
    )
    .unwrap();

    // Immediately after `payoff` is put onto the stack, the parent spell's trailing `SubAbility`
    // (`GainLife(7)`) MUST have executed while `payoff` is still on the stack!
    assert_eq!(
        e.state().object(payoff).unwrap().zone,
        Zone::Stack,
        "nested free spell is now on the stack"
    );
    assert_eq!(
        e.state().players[0].life,
        27,
        "trailing SubAbility GainLife(7) ran after nested cast completed"
    );

    // Now resolve `payoff` from the stack: life becomes 37 and `payoff` exiles itself!
    resolve_all_stack(&mut e);
    assert_eq!(e.state().players[0].life, 37);
    assert_eq!(
        e.state().object(payoff).unwrap().zone,
        Zone::Exile,
        "exile_instead_of_graveyard moved resolved spell to Exile"
    );
}

#[test]
fn effect_play_copies_from_graveyard_and_supports_opponent_alternative_cost() {
    static MASTERY_FX: &[Effect] = &[Effect::Play {
        source: PlaySource::Zone {
            zone: ZoneRef::Graveyard,
            owner: PlayerRel::You,
        },
        filter: &INSTANT_OR_SORCERY_FILTER,
        controller: PlayerRel::You,
        count: PlayCount::All,
        cost: PlayCostMode::WithoutPayingManaCost,
        cast_only: true,
        copy: true,
        exile_instead_of_graveyard: false,
        optional: true,
    }];
    static MASTERY_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: MASTERY_FX,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    static GRANT_OPP_FX: &[Effect] = &[Effect::Play {
        source: PlaySource::Zone {
            zone: ZoneRef::Exile,
            owner: PlayerRel::TargetPlayerOrController,
        },
        filter: &NONLAND_CARD_FILTER,
        controller: PlayerRel::TargetPlayerOrController,
        count: PlayCount::One,
        cost: PlayCostMode::Alternative(ManaCost::parse("{1}")),
        cast_only: true,
        copy: false,
        exile_instead_of_graveyard: false,
        optional: false,
    }];
    static GRANT_OPP_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: GRANT_OPP_FX,
        targets: Some(TargetReq {
            spec: TargetSpec::AnyOpponent,
            min: 1,
            max: 1,
            count_is_x: false,
            distinct_mana_values: false,
            chooser: TargetChooser::Controller,
        }),
        second_targets: None,
        third_targets: None,
        condition: None,
    }];
    static GAIN_THREE: &[Effect] = &[Effect::GainLife {
        amount: Amount::Fixed(3),
    }];
    static SPARK_ABILITIES: &[AbilityDef] = &[AbilityDef::Spell {
        effects: GAIN_THREE,
        targets: None,
        second_targets: None,
        third_targets: None,
        condition: None,
    }];

    let defs = vec![
        card(
            COPY_GY_MASTERY,
            face("Archive Mastery", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            MASTERY_ABILITIES,
        ),
        card(
            ALT_COST_GRANTOR,
            face("Opponent Offer", "{0}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            GRANT_OPP_ABILITIES,
        ),
        card(
            RED_BOLT_SORCERY_A,
            face("Grave Spark A", "{2}{R}", TypeSet::SORCERY),
            KeywordSet::EMPTY,
            SPARK_ABILITIES,
        ),
        card(
            RED_BOLT_SORCERY_B,
            face("Grave Spark B", "{3}{R}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SPARK_ABILITIES,
        ),
    ];
    let mut e = bench(
        9611,
        defs,
        [
            Seat::default().holding(&[
                COPY_GY_MASTERY,
                ALT_COST_GRANTOR,
                RED_BOLT_SORCERY_A,
                RED_BOLT_SORCERY_B,
            ]),
            Seat::default().holding(&[RED_BOLT_SORCERY_A]),
        ],
    );
    to_main(&mut e, me());

    // Move P0's two sparks into P0's graveyard, and P1's spark into P1's exile.
    let gy_a = first_of(&e, me(), RED_BOLT_SORCERY_A, Zone::Hand);
    let gy_b = first_of(&e, me(), RED_BOLT_SORCERY_B, Zone::Hand);
    let opp_exiled = first_of(&e, them(), RED_BOLT_SORCERY_A, Zone::Hand);
    e.state
        .move_object(
            gy_a,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            gy_b,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            opp_exiled,
            ZoneLocation::Exile(them()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    refresh_offer(&mut e);

    // Part A: Cast `COPY_GY_MASTERY` -> copies and casts BOTH graveyard cards while originals stay in GY!
    let mastery = first_of(&e, me(), COPY_GY_MASTERY, Zone::Hand);
    e.apply(me(), PlayerAction::CastSpell { card: mastery })
        .unwrap();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();

    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![gy_a],
        },
    )
    .unwrap();
    e.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![gy_b],
        },
    )
    .unwrap();
    resolve_all_stack(&mut e);

    assert_eq!(
        e.state().players[0].life,
        26,
        "both copied spells resolved (+6 life)"
    );
    assert_eq!(
        e.state().object(gy_a).unwrap().zone,
        Zone::Graveyard,
        "copy: true left original A in graveyard"
    );
    assert_eq!(
        e.state().object(gy_b).unwrap().zone,
        Zone::Graveyard,
        "copy: true left original B in graveyard"
    );

    // Part B: Give P1 `{G}` floating mana; cast `ALT_COST_GRANTOR` targeting P1 -> P1 casts `{2}{R}` card
    // from exile for `PlayCostMode::Alternative("{1}")`!
    e.state.players[1].mana_pool.add(ManaColor::Green, 1);
    refresh_offer(&mut e);
    let grantor = first_of(&e, me(), ALT_COST_GRANTOR, Zone::Hand);
    e.apply(me(), PlayerAction::CastSpell { card: grantor })
        .unwrap();
    e.apply(me(), PlayerAction::ChoosePlayer(them())).unwrap();
    e.apply(me(), PlayerAction::PassPriority).unwrap();
    e.apply(them(), PlayerAction::PassPriority).unwrap();

    let Pending::ChooseCards {
        player,
        options,
        min,
        ..
    } = e.pending().clone()
    else {
        panic!("expected ChooseCards for opponent Play");
    };
    assert_eq!((player, options, min), (them(), vec![opp_exiled], 1));
    e.apply(
        them(),
        PlayerAction::ChooseObjects {
            objects: vec![opp_exiled],
        },
    )
    .unwrap();
    resolve_all_stack(&mut e);
    assert_eq!(
        e.state().players[1].life,
        23,
        "opponent cast exiled {{2}}{{R}} spell for alternative cost {{1}}"
    );
    assert_eq!(e.state().players[1].mana_pool.total(), 0);
}
