//! Played engine rule tests for Kamigawa Flip (`CR 710`), Tarkir Omen
//! (`CR 720`), `Effect::SetState` state transitions (`CR 701.27`, `CR 708`,
//! `CR 701.40g`, `CR 712.16`), and `MeldPair` (`CR 701.42`, `CR 712.1`).
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::{CastModeKind, SpellEnding, TURN_FACE_UP, YesNoPrompt};
use crate::event::GameEvent;
use crate::object::{Rider, Status};
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    Amount, CounterKind, EnterModifier, FaceDownSpec, Filter, Modifier, PlayerRel, PtCount,
    StateSubject, StateTransition, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes;
use baylee_core::mana::ManaColor;
use baylee_core::types::{SupertypeSet, TypeSet};

const FLIP_GOBLIN: u32 = 9800;
const FLIP_ASCENDANT: u32 = 9801;
const CREATURE_AURA: u32 = 9802;
const COPY_CLONE: u32 = 9803;
const DESTROY_INSTANT: u32 = 9804;
const OMEN_DRAGON: u32 = 9805;
const OMEN_TARGET_DRAGON: u32 = 9806;
const COUNTERSPELL: u32 = 9807;
const COPY_SPELL_INSTANT: u32 = 9808;
const RIP_ENCHANTMENT: u32 = 9809;
const DFC_WEREWOLF: u32 = 9811;
const MDFC_CARD: u32 = 9812;
const TRANSFORM_PROBE: u32 = 9813;
const INCUBATOR_MAKER: u32 = 9814;
const MORPH_BEAR: u32 = 9815;
const DISGUISE_BEAR: u32 = 9816;
const OBSCURING_ENCHANT: u32 = 9817;
const BACKSLIDE_INSTANT: u32 = 9818;
const CYBER_INSTANT: u32 = 9819;
const IXIDRON_CARD: u32 = 9820;
const VANILLA_BEAR: u32 = 9821;
const TOKEN_MAKER: u32 = 9822;
const SHRIEKER_PROBE: u32 = 9823;
const ETRATA_PROBE: u32 = 9824;
const LINKED_EXILE_PROBE: u32 = 9825;
const BOLT_INSTANT: u32 = 9826;
const MANIFEST_PROBE: u32 = 9827;
const URZA_PRIMARY: u32 = 9828;
const MIGHTSTONE_PARTNER: u32 = 9829;
const TITANIA_PRIMARY: u32 = 9830;
const ARGOTH_PARTNER: u32 = 9831;
const MISHRA_PRIMARY: u32 = 9832;
const DRAGON_ENGINE_PARTNER: u32 = 9833;
const ATTACK_WATCHER: u32 = 9834;
const BIG_DRAGON: u32 = 9835;

static FILTER_CREATURE_AND_MORPH: Filter = Filter::And(&[Filter::CREATURE, Filter::HasMorph]);
static FILTER_OTHER_NONTOKEN_CREATURE: Filter = Filter::And(&[
    Filter::Another,
    Filter::NONTOKEN_CREATURE,
    Filter::Not(&Filter::FaceDown),
]);
static FILTER_FACE_DOWN_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::FaceDown]);
static FILTER_DFC_TRANSFORM: Filter = Filter::And(&[Filter::CREATURE, Filter::CanTransform]);
static FILTER_ANY_ARTIFACT_TOKEN: Filter = Filter::And(&[Filter::ARTIFACT, Filter::IsToken]);

static MELDER_URZA_PW: TokenDef = TokenDef {
    name: "Urza, Planeswalker",
    colors: ColorSet::from_slice(&[Color::White, Color::Blue]),
    types: TypeSet::PLANESWALKER,
    supertypes: SupertypeSet::LEGENDARY,
    subtypes: &[subtypes::planeswalker::URZA],
    loyalty: Some(7),
    ..TokenDef::DEFAULT
};

static MELDER_TITANIA: TokenDef = TokenDef {
    name: "Titania, Gaea Incarnate",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    supertypes: SupertypeSet::LEGENDARY,
    power: Some(5),
    toughness: Some(5),
    ..TokenDef::DEFAULT
};

static MELDER_MISHRA_ETB: &[AbilityDef] = &[
    triggered!(
        Trigger::EntersBattlefield(&Filter::This),
        &[Effect::gain_life(4)]
    ),
    triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::LoseLife {
            amount: Amount::Fixed(3),
            target: PlayerRel::You,
        }]
    ),
];

static MELDER_MISHRA: TokenDef = TokenDef {
    name: "Mishra, Lost to Phyrexia",
    colors: ColorSet::from_slice(&[Color::Black, Color::Red]),
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    supertypes: SupertypeSet::LEGENDARY,
    power: Some(9),
    toughness: Some(9),
    abilities: MELDER_MISHRA_ETB,
    ..TokenDef::DEFAULT
};

static PHYREXIAN_TOKEN_BACK: TokenDef = TokenDef {
    name: "Phyrexian",
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    power: Some(0),
    toughness: Some(0),
    ..TokenDef::DEFAULT
};

static INCUBATOR_TOKEN: TokenDef = TokenDef {
    name: "Incubator",
    types: TypeSet::ARTIFACT,
    back_face: Some(&PHYREXIAN_TOKEN_BACK),
    ..TokenDef::DEFAULT
};

static BEAR_TOKEN: TokenDef = TokenDef {
    name: "Bear",
    colors: ColorSet::from_slice(&[Color::Green]),
    types: TypeSet::CREATURE,
    power: Some(2),
    toughness: Some(2),
    ..TokenDef::DEFAULT
};

static CYBERMAN_SPEC: FaceDownSpec = FaceDownSpec {
    types: TypeSet::ARTIFACT.union(TypeSet::CREATURE),
    subtypes: &[subtypes::creature::CYBERMAN],
    power: Some(2),
    toughness: Some(2),
};

static EXILE_FAILED_TARGET: &[Effect] = &[Effect::Exile {
    target: TargetSpec::Object(&Filter::Any),
}];
static DRAW_IF_CANT: &[Effect] = &[Effect::draw(1)];

// Static ability tables for synthetic test cards.
static FLIP_ABILITIES_FRONT: &[AbilityDef] = &[free(
    &[Effect::SetState {
        subject: StateSubject::Source,
        transition: StateTransition::Flip,
        if_cant: &[],
    }],
    None,
)];
static FLIP_ABILITIES_BACK: &[AbilityDef] = &[free(&[Effect::draw(1)], None)];
static ASCENDANT_FRONT_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::SetState {
            subject: StateSubject::Source,
            transition: StateTransition::Flip,
            if_cant: &[],
        }],
        None,
    ),
    static_ability!(Filter::CREATURE, Modifier::ModifyPT(0, 1)),
];
static ENCHANTMENT_PUMP_STATIC: &[AbilityDef] =
    &[static_ability!(Filter::CREATURE, Modifier::ModifyPT(1, 1))];
static CREATURE_AURA_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::AttachSelf {
        target: TargetSpec::Object(&Filter::CREATURE),
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
)];
static CLONE_ABILITIES: &[AbilityDef] = &[AbilityDef::CopyOnEnter {
    target: TargetSpec::Object(&Filter::CREATURE),
    mods: &[],
}];
static DESTROY_CREATURE_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
)];
static DESTROY_ANY_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::destroy(TargetSpec::Object(&Filter::Any))],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any)))
)];
static OMEN_GAIN3_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::gain_life(3)])];
static OMEN_TARGET_GAIN2_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::gain_life(2)],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
)];
static COUNTERSPELL_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::CounterTargetSpell],
    targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
)];
static COPY_SPELL_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::CopyTargetSpell { mods: &[] }],
    targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
)];
static RIP_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::Any,
    Modifier::ExileInsteadOfYourGraveyard
)];
static TRANSFORM_PROBE_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::SetState {
            subject: StateSubject::Targets,
            transition: StateTransition::Transform,
            if_cant: DRAW_IF_CANT,
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::SetState {
            subject: StateSubject::BattlefieldFilter(&FILTER_ANY_ARTIFACT_TOKEN),
            transition: StateTransition::Transform,
            if_cant: &[],
        }],
        None,
    ),
];
static INCUBATOR_MAKER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::CreateToken {
        token: &INCUBATOR_TOKEN,
    }],
    None,
)];
static BEAR_MAKER_ABILITIES: &[AbilityDef] =
    &[free(&[Effect::CreateToken { token: &BEAR_TOKEN }], None)];
static OBSCURING_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::SetState {
        subject: StateSubject::Source,
        transition: StateTransition::TurnFaceDown(FaceDownSpec::DEFAULT),
        if_cant: &[],
    }],
    None,
)];
static BACKSLIDE_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::SetState {
        subject: StateSubject::Targets,
        transition: StateTransition::TurnFaceDown(FaceDownSpec::DEFAULT),
        if_cant: &[],
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &FILTER_CREATURE_AND_MORPH
    )))
)];
static CYBER_ABILITIES: &[AbilityDef] = &[spell!(
    &[Effect::SetState {
        subject: StateSubject::Targets,
        transition: StateTransition::TurnFaceDown(CYBERMAN_SPEC),
        if_cant: &[],
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
)];
static IXIDRON_ENTER_MODS: &[EnterModifier] = &[EnterModifier::TurnMatchingFaceDown(
    &FILTER_OTHER_NONTOKEN_CREATURE,
)];
static IXIDRON_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CharacteristicPT {
        count: PtCount::OnBattlefield(&FILTER_FACE_DOWN_CREATURE),
        toughness_plus: 0,
    }
)];
static BOLT_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::gain_life(1)])];
static MANIFEST_PROBE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::ManifestTop {
        who: PlayerRel::You,
        count: Amount::Fixed(1),
    }],
    None,
)];
static SHRIEKER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::SetState {
        subject: StateSubject::Targets,
        transition: StateTransition::RevealAndTurnFaceUpIf {
            filter: &Filter::CREATURE,
            optional: true,
        },
        if_cant: &[],
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::FaceDown))),
)];
static ETRATA_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::SetState {
        subject: StateSubject::Targets,
        transition: StateTransition::TurnFaceUp,
        if_cant: EXILE_FAILED_TARGET,
    }],
    Some(TargetReq::one(TargetSpec::Object(&Filter::FaceDown))),
)];
static LINKED_EXILE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::SetState {
        subject: StateSubject::LinkedExile,
        transition: StateTransition::TurnFaceUp,
        if_cant: &[],
    }],
    None,
)];
static URZA_MELD_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::MeldPair {
        source_name: "Urza, Lord Protector",
        partner_name: "The Mightstone and Weakstone",
        partner_types: TypeSet::ARTIFACT,
        require_attacking: false,
        enters_tapped: false,
        enters_attacking: false,
        melded: &MELDER_URZA_PW,
    }],
    None,
)];
static TITANIA_MELD_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::MeldPair {
        source_name: "Titania, Voice of Gaea",
        partner_name: "Argoth, Sanctum of Nature",
        partner_types: TypeSet::LAND,
        require_attacking: false,
        enters_tapped: false,
        enters_attacking: false,
        melded: &MELDER_TITANIA,
    }],
    None,
)];
static MISHRA_MELD_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::MeldPair {
        source_name: "Mishra, Claimed by Gix",
        partner_name: "Phyrexian Dragon Engine",
        partner_types: TypeSet::CREATURE,
        require_attacking: true,
        enters_tapped: true,
        enters_attacking: true,
        melded: &MELDER_MISHRA,
    }]
)];
static ATTACK_WATCHER_ABILITIES: &[AbilityDef] = &[triggered!(
    Trigger::Attacks(&Filter::ControlledByYou),
    &[Effect::draw(1)]
)];

fn two_face_card(index: u32, front: FaceDef, back: FaceDef) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([front, back])),
        color_identity: ColorSet::EMPTY,
        keywords: KeywordSet::EMPTY,
        commander: CommanderRule::NotEligible,
        partner: PartnerKind::None,
        fuse: false,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

fn add_mana(engine: &mut Bench, seat: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(seat.get())]
        .mana_pool
        .add(color, amount);
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn cast_face(engine: &mut Bench, seat: PlayerId, card_id: ObjectId, mode_face: Option<usize>) {
    engine
        .apply(seat, PlayerAction::CastSpell { card: card_id })
        .expect("start cast");
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let idx = match mode_face {
            None => options
                .iter()
                .position(|o| matches!(o.kind, CastModeKind::Normal))
                .expect("normal cast mode"),
            Some(face_idx) => options
                .iter()
                .position(|o| matches!(o.kind, CastModeKind::Face(f) if f == face_idx))
                .expect("requested face cast mode"),
        };
        engine
            .apply(seat, PlayerAction::ChooseMode(idx))
            .expect("choose cast mode");
    }
}

fn flip_goblin_def() -> &'static CardDef {
    two_face_card(
        FLIP_GOBLIN,
        FaceDef {
            abilities: FLIP_ABILITIES_FRONT,
            subtypes: &[subtypes::creature::GOBLIN],
            ..creature_face("Akki Lavarunner", "{1}{R}", 1, 1)
        },
        FaceDef {
            name: "Tok-Tok, Volcano Born",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::CREATURE,
            supertypes: SupertypeSet::LEGENDARY,
            subtypes: &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
            power: Some(2),
            toughness: Some(2),
            abilities: FLIP_ABILITIES_BACK,
            castable_from_hand: false,
            flip: true,
            ..land_face("Tok-Tok, Volcano Born")
        },
    )
}

fn omen_dragon_def() -> &'static CardDef {
    two_face_card(
        OMEN_DRAGON,
        FaceDef {
            keywords: KeywordSet::FLYING,
            ..creature_face("Bloomvine Regent", "{3}{G}{G}", 4, 5)
        },
        FaceDef {
            name: "Claim Territory",
            mana_cost: ManaCost::parse("{1}{G}"),
            types: TypeSet::SORCERY,
            subtypes: &[subtypes::spell::OMEN],
            abilities: OMEN_GAIN3_ABILITIES,
            castable_from_hand: true,
            omen: true,
            ..land_face("Claim Territory")
        },
    )
}

#[test]
fn flip_replaces_characteristics_preserves_mana_cost_color_counters_and_is_one_way() {
    let goblin = flip_goblin_def();
    let transform_probe = card(
        TRANSFORM_PROBE,
        face("Transform Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TRANSFORM_PROBE_ABILITIES,
    );
    let destroy = card(
        DESTROY_INSTANT,
        face("Doom", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DESTROY_CREATURE_ABILITIES,
    );

    let mut engine = bench(
        1,
        vec![goblin, transform_probe, destroy],
        [
            Seat::with(&[TRANSFORM_PROBE]).holding(&[FLIP_GOBLIN, DESTROY_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    // Only front face is castable from hand; flip back face is NEVER offered.
    let hand_goblin = engine.state().zones.list(ZoneLocation::Hand(me()))[0];
    add_mana(&mut engine, me(), ManaColor::Red, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: hand_goblin })
        .expect("cast flip card front");
    assert!(
        !matches!(engine.pending(), Pending::ChooseCastMode { .. }),
        "flip back face must never be offered as a cast mode from hand"
    );
    settle(&mut engine, me());

    let perm = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(FLIP_GOBLIN)
        })
        .expect("flip permanent entered");
    let version_before = engine.state().object(perm).unwrap().version;
    let controlled_before = engine.state().object(perm).unwrap().controlled_since;
    let ts_before = engine.state().object(perm).unwrap().timestamp;
    engine
        .state
        .object_mut(perm)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 1);
    engine.state.object_mut(perm).unwrap().damage = 1;
    engine.state.invalidate_projections();

    // Activate flip ability on face 0.
    activate(&mut engine, me(), perm, 0);
    settle(&mut engine, me());

    let obj = engine.state().object(perm).expect("still same object");
    assert_eq!(obj.version, version_before);
    assert_eq!(obj.face_index, 1);
    assert!(obj.status.contains(Status::FLIPPED));
    assert!(crate::eval::matches(
        &Filter::Flipped,
        engine.state(),
        obj,
        me(),
        perm
    ));
    assert!(!crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        obj,
        me(),
        perm
    ));
    assert!(!crate::eval::matches(
        &Filter::CanTransform,
        engine.state(),
        obj,
        me(),
        perm
    ));
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Tok-Tok, Volcano Born"
    );
    assert!(
        obj.characteristics()
            .supertypes
            .contains(SupertypeSet::LEGENDARY)
    );
    assert_eq!(obj.characteristics().power, Some(3));
    assert_eq!(obj.characteristics().toughness, Some(3));
    assert_eq!(obj.counters.get(CounterKind::P1P1), 1);
    assert_eq!(obj.damage, 1);
    // CR 710.2: mana cost, color, and mana value remain face 0's ({1}{R}, Red, MV 2).
    assert_eq!(obj.characteristics().mana_cost, ManaCost::parse("{1}{R}"));
    assert_eq!(
        obj.characteristics().colors,
        ColorSet::from_slice(&[Color::Red])
    );
    assert_eq!(obj.characteristics().mana_value(), 2);
    assert_eq!(obj.base.front_mana_value, None);
    // CR 613.7g & CR 302.6: new timestamp, unchanged controlled_since.
    assert!(obj.timestamp > ts_before);
    assert_eq!(obj.controlled_since, controlled_before);

    // CR 710.4: one-way flip — flipping again returns false and leaves face 1.
    assert!(!engine.state.flip_permanent(perm, goblin));
    assert_eq!(engine.state().object(perm).unwrap().face_index, 1);

    // Flipped permanent's new ability (draw 1) works.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), perm, 0);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1
    );

    // CR 701.27d: trying to Transform a flip permanent does nothing and triggers if_cant (draw 1).
    let probe = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(TRANSFORM_PROBE)
        })
        .unwrap();
    let hand_before_transform = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), probe, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![perm],
            },
        )
        .expect("target flipped goblin");
    settle(&mut engine, me());
    assert_eq!(engine.state().object(perm).unwrap().face_index, 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_transform + 1,
        "if_cant should run because flip cards cannot transform"
    );

    // CR 710.4 / CR 400.7: leaving battlefield resets to face 0 and clears Status::FLIPPED.
    let doom = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(DESTROY_INSTANT)
        })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: doom })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![perm],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let gy_obj = engine.state().object(perm).unwrap();
    assert_eq!(gy_obj.zone, Zone::Graveyard);
    assert_eq!(gy_obj.face_index, 0);
    assert!(!gy_obj.status.contains(Status::FLIPPED));
    assert_eq!(
        engine.state().names.get(gy_obj.characteristics().name),
        "Akki Lavarunner"
    );
}

#[test]
fn flip_into_enchantment_removes_from_combat_and_detaches_creature_aura() {
    let ascendant = two_face_card(
        FLIP_ASCENDANT,
        FaceDef {
            abilities: ASCENDANT_FRONT_ABILITIES,
            ..creature_face("Kuon, Ogre Ascendant", "{B}{B}{B}", 2, 2)
        },
        FaceDef {
            name: "Kuon's Essence",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::ENCHANTMENT,
            supertypes: SupertypeSet::LEGENDARY,
            abilities: ENCHANTMENT_PUMP_STATIC,
            castable_from_hand: false,
            flip: true,
            ..land_face("Kuon's Essence")
        },
    );
    let aura = card(
        CREATURE_AURA,
        FaceDef {
            subtypes: &[subtypes::enchantment::AURA],
            ..face("Creature Aura", "{0}", TypeSet::ENCHANTMENT)
        },
        KeywordSet::EMPTY,
        CREATURE_AURA_ABILITIES,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Grizzly", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        2,
        vec![ascendant, aura, bear],
        [
            Seat::with(&[FLIP_ASCENDANT, VANILLA_BEAR]).holding(&[CREATURE_AURA]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let kuon = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(FLIP_ASCENDANT)
        })
        .unwrap();
    let grizzly = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(VANILLA_BEAR)
        })
        .unwrap();
    let aura_card = engine.state().zones.list(ZoneLocation::Hand(me()))[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: aura_card })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![kuon],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(aura_card).unwrap().attached_to,
        Some(kuon)
    );
    assert_eq!(
        engine
            .state()
            .object(grizzly)
            .unwrap()
            .characteristics()
            .toughness,
        Some(3)
    );

    walk_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(kuon, baylee_core::ids::Defender::Player(them()))],
            },
        )
        .expect("declare Kuon attacking");
    assert!(engine.state().combat.is_attacking(kuon));

    activate(&mut engine, me(), kuon, 0);
    settle(&mut engine, me());

    let k_obj = engine.state().object(kuon).unwrap();
    assert_eq!(k_obj.characteristics().types, TypeSet::ENCHANTMENT);
    assert!(
        !engine.state().combat.is_attacking(kuon),
        "CR 506.4: noncreature removed from combat"
    );
    assert_eq!(
        engine.state().object(aura_card).unwrap().zone,
        Zone::Graveyard,
        "CR 704.5m: creature aura detaches and goes to graveyard"
    );
    let g_chars = engine.state().object(grizzly).unwrap().characteristics();
    assert_eq!((g_chars.power, g_chars.toughness), (Some(3), Some(3)));
}

#[test]
fn clone_of_flipped_and_unflipped_flip_cards_and_turn_face_down_round_trip() {
    let goblin = flip_goblin_def();
    let clone_def = card(
        COPY_CLONE,
        creature_face("Clone", "{0}", 0, 0),
        KeywordSet::EMPTY,
        CLONE_ABILITIES,
    );
    let mut engine = bench(
        3,
        vec![goblin, clone_def],
        [
            Seat::with(&[FLIP_GOBLIN]).holding(&[COPY_CLONE, COPY_CLONE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let orig = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(FLIP_GOBLIN)
        })
        .unwrap();

    // 1. Clone an UNFLIPPED flip card (CR 707.2 / CR 710.2): acquires the flip
    //    card's alternative characteristics, flips to face 1 keeping Clone's
    //    `original_base`, survives turn-face-down round-trip, and reverts to
    //    Clone upon leaving the battlefield.
    let clone1 = engine.state().zones.list(ZoneLocation::Hand(me()))[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone1 })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![orig],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(clone1)
                .unwrap()
                .characteristics()
                .name
        ),
        "Akki Lavarunner"
    );
    activate(&mut engine, me(), clone1, 0);
    settle(&mut engine, me());
    {
        let c1 = engine.state().object(clone1).unwrap();
        assert!(
            c1.status.contains(Status::FLIPPED),
            "CR 707.2/710.2: Clone of unflipped flip card flips to face 1"
        );
        assert_eq!(
            engine.state().names.get(c1.characteristics().name),
            "Tok-Tok, Volcano Born"
        );
        assert_eq!(
            c1.characteristics().colors,
            ColorSet::from_slice(&[Color::Red])
        );
        assert_eq!(c1.characteristics().mana_value(), 2);
        assert_eq!(c1.characteristics().power, Some(2));
        assert_eq!(c1.characteristics().toughness, Some(2));
        assert!(c1.original_base.is_some());
    }
    // CR 710.4: one-way flip on the flipped Clone too.
    assert!(!engine.state.flip_permanent(clone1, goblin));
    // Flipped Clone's face-1 ability (draw 1) works.
    let hand_before_c1_draw = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), clone1, 0);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_c1_draw + 1
    );
    // Turning the flipped Clone face down and back face up restores Tok-Tok + Status::FLIPPED.
    assert!(
        engine
            .state
            .turn_permanent_face_down(clone1, FaceDownSpec::DEFAULT)
    );
    assert!(engine.state.turn_permanent_face_up(clone1));
    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(clone1)
                .unwrap()
                .characteristics()
                .name
        ),
        "Tok-Tok, Volcano Born"
    );
    assert!(
        engine
            .state()
            .object(clone1)
            .unwrap()
            .status
            .contains(Status::FLIPPED)
    );
    // Moving clone1 to graveyard restores printed Clone and clears Status::FLIPPED.
    engine
        .state
        .move_object(
            clone1,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    {
        let c1_gy = engine.state().object(clone1).unwrap();
        assert_eq!(engine.state().names.get(c1_gy.base.name), "Clone");
        assert!(!c1_gy.status.contains(Status::FLIPPED));
        assert!(c1_gy.original_base.is_none());
    }

    // 2. Flip the original, then Clone the FLIPPED permanent.
    activate(&mut engine, me(), orig, 0);
    settle(&mut engine, me());
    let clone2 = engine.state().zones.list(ZoneLocation::Hand(me()))[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone2 })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![orig],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::LegendChoice { .. } | Pending::Priority { .. }
        )
    });
    if matches!(engine.pending(), Pending::LegendChoice { .. }) {
        engine
            .apply(
                me(),
                PlayerAction::ChooseObjects {
                    objects: vec![clone2],
                },
            )
            .unwrap();
    }
    settle(&mut engine, me());

    let c2 = engine.state().object(clone2).unwrap();
    assert_eq!(
        engine.state().names.get(c2.characteristics().name),
        "Tok-Tok, Volcano Born"
    );
    assert_eq!(c2.characteristics().mana_cost, ManaCost::parse("{1}{R}"));
    assert_eq!(
        c2.characteristics().colors,
        ColorSet::from_slice(&[Color::Red])
    );
    assert_eq!(c2.characteristics().mana_value(), 2);
    assert!(
        !c2.status.contains(Status::FLIPPED),
        "status is not copiable (CR 707.2)"
    );

    // 3. Single-faced flip permanent (flipped) can turn face down and back face up keeping Status::FLIPPED.
    let mut e2 = bench(
        4,
        vec![goblin],
        [Seat::with(&[FLIP_GOBLIN]), Seat::default()],
    );
    to_main(&mut e2, me());
    let orig2 = e2.state().battlefield_seen().next().unwrap();
    activate(&mut e2, me(), orig2, 0);
    settle(&mut e2, me());
    assert!(
        e2.state
            .turn_permanent_face_down(orig2, FaceDownSpec::DEFAULT)
    );
    let fd = e2.state().object(orig2).unwrap();
    assert!(fd.status.contains(Status::FACE_DOWN));
    assert!(fd.status.contains(Status::FLIPPED));
    assert_eq!(
        fd.printed_abilities(&*e2.state.bases).len(),
        0,
        "no Ward {{2}}"
    );
    assert!(e2.state.turn_permanent_face_up(orig2));
    let fu = e2.state().object(orig2).unwrap();
    assert!(!fu.status.contains(Status::FACE_DOWN));
    assert!(fu.status.contains(Status::FLIPPED));
    assert_eq!(
        e2.state().names.get(fu.characteristics().name),
        "Tok-Tok, Volcano Born"
    );
}

#[test]
fn omen_casts_either_face_shuffles_into_library_on_resolution_and_graveyards_on_counter_or_fizzle()
{
    let dragon = omen_dragon_def();
    let targeted_omen = two_face_card(
        OMEN_TARGET_DRAGON,
        creature_face("Storm Dragon", "{4}{R}", 4, 4),
        FaceDef {
            name: "Fiery Omen",
            mana_cost: ManaCost::parse("{R}"),
            types: TypeSet::INSTANT,
            subtypes: &[subtypes::spell::OMEN],
            abilities: OMEN_TARGET_GAIN2_ABILITIES,
            castable_from_hand: true,
            omen: true,
            ..land_face("Fiery Omen")
        },
    );
    let counterspell = card(
        COUNTERSPELL,
        face("Counterspell", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COUNTERSPELL_ABILITIES,
    );
    let kill = card(
        DESTROY_INSTANT,
        face("Murder", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DESTROY_CREATURE_ABILITIES,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Grizzly", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    assert!(dragon.is_omen_card());
    assert!(!dragon.is_split_spell_card());
    assert!(!dragon.is_double_faced());

    let mut engine = bench(
        5,
        vec![dragon, targeted_omen, counterspell, kill, bear],
        [
            Seat::with(&[VANILLA_BEAR]).holding(&[OMEN_DRAGON, OMEN_DRAGON, OMEN_TARGET_DRAGON]),
            Seat::default().holding(&[COUNTERSPELL, DESTROY_INSTANT]),
        ],
    );
    to_main(&mut engine, me());

    let hand = engine.state().zones.list(ZoneLocation::Hand(me())).clone();
    let omen1 = hand[0];
    let omen2 = hand[1];
    let omen_target = hand[2];
    assert!(crate::eval::matches(
        &Filter::HasOmen,
        engine.state(),
        engine.state().object(omen1).unwrap(),
        me(),
        omen1
    ));

    // 1. Cast Omen face with only 2 mana (cannot afford 5-mana Dragon!).
    let lib_before = engine.state().zones.list(ZoneLocation::Library(me())).len();
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    cast_face(&mut engine, me(), omen1, Some(1));
    settle(&mut engine, me());
    assert_eq!(engine.state().players[0].life, 23);
    assert_eq!(engine.state().object(omen1).unwrap().zone, Zone::Library);
    assert_eq!(engine.state().object(omen1).unwrap().face_index, 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(me())).len(),
        lib_before + 1
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(me()))
            .is_empty()
    );

    // 2. Cast Omen face and counter it -> goes to Graveyard (NOT library!), face 0.
    add_mana(&mut engine, me(), ManaColor::Green, 2);
    cast_face(&mut engine, me(), omen2, Some(1));
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let cs = engine.state().zones.list(ZoneLocation::Hand(them()))[0];
    engine
        .apply(them(), PlayerAction::CastSpell { card: cs })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![omen2],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(omen2).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(omen2).unwrap().face_index, 0);

    // 3. Cast targeted Omen face and kill target in response -> fizzles to Graveyard (NOT library!).
    let bear_id = engine.state().battlefield_seen().next().unwrap();
    add_mana(&mut engine, me(), ManaColor::Red, 1);
    cast_face(&mut engine, me(), omen_target, Some(1));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let murder = engine.state().zones.list(ZoneLocation::Hand(them()))[0];
    engine
        .apply(them(), PlayerAction::CastSpell { card: murder })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseObjects {
                objects: vec![bear_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(omen_target).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(engine.state().object(omen_target).unwrap().face_index, 0);

    // 4. Cast Dragon front face -> enters battlefield as 4/5 Flying Dragon!
    let mut e2 = bench(
        6,
        vec![dragon],
        [Seat::default().holding(&[OMEN_DRAGON]), Seat::default()],
    );
    to_main(&mut e2, me());
    let d_card = e2.state().zones.list(ZoneLocation::Hand(me()))[0];
    add_mana(&mut e2, me(), ManaColor::Green, 5);
    cast_face(&mut e2, me(), d_card, None);
    settle(&mut e2, me());
    let d_obj = e2.state().object(d_card).unwrap();
    assert_eq!(d_obj.zone, Zone::Battlefield);
    assert_eq!(d_obj.face_index, 0);
    assert_eq!(d_obj.characteristics().power, Some(4));
    assert!(
        d_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}

#[test]
fn omen_spell_copy_ceases_to_exist_and_competes_with_exile_and_flashback() {
    let dragon = omen_dragon_def();
    let copy_spell = card(
        COPY_SPELL_INSTANT,
        face("Twincast", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        COPY_SPELL_ABILITIES,
    );
    let rip = card(
        RIP_ENCHANTMENT,
        face("Rest in Peace Proxy", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        RIP_ABILITIES,
    );

    // 1. Copy of an Omen spell resolves for effect and ceases to exist (only original shuffles in).
    let mut e1 = bench(
        7,
        vec![dragon, copy_spell],
        [
            Seat::default().holding(&[OMEN_DRAGON, COPY_SPELL_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut e1, me());
    let hand = e1.state().zones.list(ZoneLocation::Hand(me())).clone();
    let omen_card = hand[0];
    let twincast = hand[1];
    let lib_before = e1.state().zones.list(ZoneLocation::Library(me())).len();
    add_mana(&mut e1, me(), ManaColor::Green, 2);
    cast_face(&mut e1, me(), omen_card, Some(1));
    e1.apply(me(), PlayerAction::CastSpell { card: twincast })
        .unwrap();
    e1.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![omen_card],
        },
    )
    .unwrap();
    settle(&mut e1, me());
    assert_eq!(e1.state().players[0].life, 26);
    assert_eq!(
        e1.state().zones.list(ZoneLocation::Library(me())).len(),
        lib_before + 1
    );
    assert_eq!(e1.state().object(omen_card).unwrap().zone, Zone::Library);

    // 2. Competing SpellEnding::Omen vs SpellEnding::Exile under Rest in Peace (CR 616.1e).
    for choose_omen in [true, false] {
        let mut e2 = bench(
            8,
            vec![dragon, rip],
            [
                Seat::with(&[RIP_ENCHANTMENT]).holding(&[OMEN_DRAGON]),
                Seat::default(),
            ],
        );
        to_main(&mut e2, me());
        let card_id = e2.state().zones.list(ZoneLocation::Hand(me()))[0];
        add_mana(&mut e2, me(), ManaColor::Green, 2);
        cast_face(&mut e2, me(), card_id, Some(1));
        walk_until(&mut e2, |e| {
            matches!(e.pending(), Pending::ChooseCastMode { .. })
        });
        let Pending::ChooseCastMode { options, .. } = e2.pending().clone() else {
            panic!("expected SpellEnding choice");
        };
        let endings: Vec<SpellEnding> = options
            .iter()
            .filter_map(|o| match o.kind {
                CastModeKind::SpellEnding(se) => Some(se),
                _ => None,
            })
            .collect();
        assert_eq!(endings, vec![SpellEnding::Omen, SpellEnding::Exile]);
        let pick = usize::from(!choose_omen);
        e2.apply(me(), PlayerAction::ChooseMode(pick)).unwrap();
        settle(&mut e2, me());
        let expected_zone = if choose_omen {
            Zone::Library
        } else {
            Zone::Exile
        };
        assert_eq!(e2.state().object(card_id).unwrap().zone, expected_zone);
        assert_eq!(e2.state().object(card_id).unwrap().face_index, 0);
    }

    // 3. Flashback on an Omen exiles unconditionally (CR 702.34a).
    let mut e3 = bench(
        9,
        vec![dragon],
        [Seat::default().holding(&[OMEN_DRAGON]), Seat::default()],
    );
    to_main(&mut e3, me());
    let card_id = e3.state().zones.list(ZoneLocation::Hand(me()))[0];
    e3.state
        .object_mut(card_id)
        .unwrap()
        .riders
        .push(Rider::Flashback);
    e3.state.switch_face(card_id, dragon, 1);
    e3.state.object_mut(card_id).unwrap().kind = crate::object::ObjectKind::Spell;
    e3.state
        .move_object(
            card_id,
            ZoneLocation::Stack,
            ZonePosition::Top,
            crate::event::Cause::Spell,
        )
        .unwrap();
    e3.finalize_spell(card_id, None);
    assert_eq!(
        e3.state().object(card_id).unwrap().zone,
        Zone::Exile,
        "CR 702.34a: Flashback exiles an Omen instead of shuffling into library"
    );
    assert_eq!(e3.state().object(card_id).unwrap().face_index, 0);
}

#[test]
fn set_state_transform_targets_filter_tokens_and_rejects_non_transforming_or_facedown() {
    let werewolf = two_face_card(
        DFC_WEREWOLF,
        creature_face("Human Scout", "{1}{G}", 2, 2),
        FaceDef {
            name: "Howling Wolf",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::CREATURE,
            power: Some(4),
            toughness: Some(4),
            castable_from_hand: false,
            ..land_face("Howling Wolf")
        },
    );
    let mdfc = two_face_card(
        MDFC_CARD,
        creature_face("Modal Front", "{1}{W}", 2, 2),
        FaceDef {
            name: "Modal Back",
            mana_cost: ManaCost::parse("{2}{W}"),
            types: TypeSet::CREATURE,
            power: Some(3),
            toughness: Some(3),
            castable_from_hand: true,
            ..land_face("Modal Back")
        },
    );
    let incubator_maker = card(
        INCUBATOR_MAKER,
        face("Incubator Maker", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        INCUBATOR_MAKER_ABILITIES,
    );
    let probe = card(
        TRANSFORM_PROBE,
        face("Transform Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        TRANSFORM_PROBE_ABILITIES,
    );

    let mut engine = bench(
        10,
        vec![werewolf, mdfc, incubator_maker, probe],
        [
            Seat::with(&[DFC_WEREWOLF, MDFC_CARD, INCUBATOR_MAKER, TRANSFORM_PROBE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let find = |e: &Bench, idx: u32| {
        e.state()
            .battlefield_seen()
            .find(|&id| e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(idx))
            .unwrap()
    };
    let ww = find(&engine, DFC_WEREWOLF);
    let md = find(&engine, MDFC_CARD);
    let im = find(&engine, INCUBATOR_MAKER);
    let pr = find(&engine, TRANSFORM_PROBE);

    assert!(crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        engine.state().object(ww).unwrap(),
        me(),
        pr
    ));
    assert!(crate::eval::matches(
        &Filter::CanTransform,
        engine.state(),
        engine.state().object(ww).unwrap(),
        me(),
        pr
    ));
    assert!(crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        engine.state().object(md).unwrap(),
        me(),
        pr
    ));
    assert!(!crate::eval::matches(
        &Filter::CanTransform,
        engine.state(),
        engine.state().object(md).unwrap(),
        me(),
        pr
    ));
    assert!(crate::eval::matches(
        &FILTER_DFC_TRANSFORM,
        engine.state(),
        engine.state().object(ww).unwrap(),
        me(),
        pr
    ));

    activate(&mut engine, me(), pr, 0);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![ww] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(ww).unwrap().face_index, 1);
    assert_eq!(
        engine.state().object(ww).unwrap().characteristics().power,
        Some(4)
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), pr, 0);
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![md] })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(md).unwrap().face_index, 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 1
    );

    activate(&mut engine, me(), im, 0);
    settle(&mut engine, me());
    let inc = engine
        .state()
        .battlefield_seen()
        .find(|&id| engine.state().object(id).is_some_and(|o| o.token.is_some()))
        .unwrap();
    engine
        .state
        .object_mut(inc)
        .unwrap()
        .counters
        .add(CounterKind::P1P1, 3);
    assert!(crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        engine.state().object(inc).unwrap(),
        me(),
        pr
    ));
    assert!(crate::eval::matches(
        &Filter::CanTransform,
        engine.state(),
        engine.state().object(inc).unwrap(),
        me(),
        pr
    ));
    activate(&mut engine, me(), pr, 1);
    settle(&mut engine, me());
    let inc_obj = engine.state().object(inc).unwrap();
    assert_eq!(inc_obj.face_index, 1);
    assert!(inc_obj.characteristics().types.contains(TypeSet::CREATURE));
    assert_eq!(inc_obj.characteristics().power, Some(3));
}

#[test]
fn set_state_turn_face_down_default_custom_cyberman_ixidron_clone_and_cr712_16_guards() {
    let morph_bear = card(
        MORPH_BEAR,
        FaceDef {
            morph: Some(ManaCost::parse("{G}")),
            ..creature_face("Morph Bear", "{1}{G}", 3, 3)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let disguise_bear = card(
        DISGUISE_BEAR,
        FaceDef {
            disguise: Some(ManaCost::parse("{1}{G}")),
            ..creature_face("Disguise Bear", "{1}{G}", 3, 3)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let obscuring = card(
        OBSCURING_ENCHANT,
        face("Obscuring Aether", "{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        OBSCURING_ABILITIES,
    );
    let backslide = card(
        BACKSLIDE_INSTANT,
        face("Backslide", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BACKSLIDE_ABILITIES,
    );
    let cyber = card(
        CYBER_INSTANT,
        face("Cyber Conversion", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        CYBER_ABILITIES,
    );
    let ixidron = card(
        IXIDRON_CARD,
        FaceDef {
            enter_modifiers: IXIDRON_ENTER_MODS,
            ..creature_face("Ixidron", "{0}", 0, 0)
        },
        KeywordSet::EMPTY,
        IXIDRON_ABILITIES,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let token_maker = card(
        TOKEN_MAKER,
        face("Token Maker", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        BEAR_MAKER_ABILITIES,
    );
    let big_dragon = card(
        BIG_DRAGON,
        creature_face("Shivan", "{4}{R}{R}", 5, 5),
        KeywordSet::FLYING,
        &[],
    );
    let clone_def = card(
        COPY_CLONE,
        creature_face("Clone", "{0}", 0, 0),
        KeywordSet::EMPTY,
        CLONE_ABILITIES,
    );
    let werewolf = two_face_card(
        DFC_WEREWOLF,
        creature_face("DFC Wolf", "{1}{G}", 2, 2),
        FaceDef {
            name: "DFC Back",
            mana_cost: ManaCost::ZERO,
            types: TypeSet::CREATURE,
            power: Some(4),
            toughness: Some(4),
            castable_from_hand: false,
            ..land_face("DFC Back")
        },
    );
    let mdfc = two_face_card(
        MDFC_CARD,
        creature_face("MDFC Front", "{1}{W}", 2, 2),
        FaceDef {
            name: "MDFC Back",
            mana_cost: ManaCost::parse("{2}{W}"),
            types: TypeSet::CREATURE,
            power: Some(3),
            toughness: Some(3),
            castable_from_hand: true,
            ..land_face("MDFC Back")
        },
    );

    let mut engine = bench(
        11,
        vec![
            morph_bear,
            disguise_bear,
            obscuring,
            backslide,
            cyber,
            ixidron,
            bear,
            token_maker,
            big_dragon,
            clone_def,
            werewolf,
            mdfc,
        ],
        [
            Seat::with(&[
                MORPH_BEAR,
                DISGUISE_BEAR,
                OBSCURING_ENCHANT,
                VANILLA_BEAR,
                TOKEN_MAKER,
                BIG_DRAGON,
                DFC_WEREWOLF,
                MDFC_CARD,
            ])
            .holding(&[BACKSLIDE_INSTANT, CYBER_INSTANT, IXIDRON_CARD, COPY_CLONE]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let find = |e: &Bench, idx: u32| {
        e.state()
            .battlefield_seen()
            .find(|&id| e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(idx))
            .unwrap()
    };
    let mb = find(&engine, MORPH_BEAR);
    let db = find(&engine, DISGUISE_BEAR);
    let oa = find(&engine, OBSCURING_ENCHANT);
    let vb = find(&engine, VANILLA_BEAR);
    let tm = find(&engine, TOKEN_MAKER);
    let shivan = find(&engine, BIG_DRAGON);
    let ww = find(&engine, DFC_WEREWOLF);
    let md = find(&engine, MDFC_CARD);

    // 1. Self TurnFaceDown (Obscuring Aether -> 2/2 creature with NO Ward {2}).
    activate(&mut engine, me(), oa, 0);
    settle(&mut engine, me());
    let oa_obj = engine.state().object(oa).unwrap();
    assert!(oa_obj.status.contains(Status::FACE_DOWN));
    assert!(oa_obj.riders.contains(&Rider::TurnedFaceDown));
    assert_eq!(oa_obj.characteristics().types, TypeSet::CREATURE);
    assert_eq!(
        (
            oa_obj.characteristics().power,
            oa_obj.characteristics().toughness
        ),
        (Some(2), Some(2))
    );
    assert_eq!(oa_obj.printed_abilities(&*engine.state.bases).len(), 0);

    // 2. Backslide targets creature with Morph -> turns face down, loses HasMorph while face down,
    //    and can still turn face up via Morph special action!
    assert!(crate::eval::matches(
        &Filter::HasMorph,
        engine.state(),
        engine.state().object(mb).unwrap(),
        me(),
        mb
    ));
    assert!(!crate::eval::matches(
        &Filter::HasMorph,
        engine.state(),
        engine.state().object(vb).unwrap(),
        me(),
        vb
    ));
    let bs = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(BACKSLIDE_INSTANT)
        })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: bs })
        .unwrap();
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![mb] })
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(mb)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        !crate::eval::matches(
            &Filter::HasMorph,
            engine.state(),
            engine.state().object(mb).unwrap(),
            me(),
            mb
        ),
        "CR 708.2: face-down creature does not have morph"
    );
    add_mana(&mut engine, me(), ManaColor::Green, 1);
    engine
        .apply(
            me(),
            PlayerAction::ActivateAbility {
                source: mb,
                ability_index: TURN_FACE_UP,
            },
        )
        .expect("morph turn-face-up special action works after effect turn-face-down");
    assert!(
        !engine
            .state()
            .object(mb)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        !engine
            .state()
            .object(mb)
            .unwrap()
            .riders
            .contains(&Rider::TurnedFaceDown)
    );
    assert_eq!(
        engine.state().object(mb).unwrap().characteristics().power,
        Some(3)
    );

    // 3. Disguise creature turned face down by effect does NOT gain Ward {2}!
    assert!(
        engine
            .state
            .turn_permanent_face_down(db, FaceDownSpec::DEFAULT)
    );
    assert_eq!(
        engine
            .state()
            .object(db)
            .unwrap()
            .printed_abilities(&*engine.state.bases)
            .len(),
        0,
        "Disguise creature turned face down by an effect must NOT gain Ward {{2}}"
    );

    // 4. Cyber Conversion custom FaceDownSpec (2/2 Artifact Creature — Cyberman, no Ward {2}).
    let cc = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(CYBER_INSTANT)
        })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: cc })
        .unwrap();
    engine
        .apply(me(), PlayerAction::ChooseObjects { objects: vec![vb] })
        .unwrap();
    settle(&mut engine, me());
    let vb_obj = engine.state().object(vb).unwrap();
    assert!(vb_obj.status.contains(Status::FACE_DOWN));
    assert_eq!(
        vb_obj.characteristics().types,
        TypeSet::ARTIFACT.union(TypeSet::CREATURE)
    );
    assert!(
        vb_obj
            .characteristics()
            .subtypes
            .contains(subtypes::creature::CYBERMAN)
    );
    assert_eq!(
        (
            vb_obj.characteristics().power,
            vb_obj.characteristics().toughness
        ),
        (Some(2), Some(2))
    );
    assert_eq!(vb_obj.printed_abilities(&*engine.state.bases).len(), 0);

    // 5. CR 712.16: Double-faced permanents (transforming DFC, MDFC) CANNOT be turned face down.
    assert!(!engine.state.can_turn_face_down(ww));
    assert!(
        !engine
            .state
            .turn_permanent_face_down(ww, FaceDownSpec::DEFAULT)
    );
    assert!(!engine.state.can_turn_face_down(md));
    assert!(
        !engine
            .state
            .turn_permanent_face_down(md, FaceDownSpec::DEFAULT)
    );

    // 6. Clone of Shivan Dragon turned face down & back face up retains copied 5/5 Dragon base!
    let clone_id = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(COPY_CLONE)
        })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: clone_id })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![shivan],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine
            .state()
            .object(clone_id)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    let hash_before = engine.state().snapshot_hash();
    assert!(
        engine
            .state
            .turn_permanent_face_down(clone_id, FaceDownSpec::DEFAULT)
    );
    assert!(!engine.state.face_down_copied_bases.is_empty());
    assert_ne!(engine.state().snapshot_hash(), hash_before);
    assert_eq!(
        engine
            .state()
            .object(clone_id)
            .unwrap()
            .characteristics()
            .power,
        Some(2)
    );
    assert!(engine.state.turn_permanent_face_up(clone_id));
    assert!(engine.state.face_down_copied_bases.is_empty());
    assert_eq!(
        engine
            .state()
            .object(clone_id)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );
    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(clone_id)
                .unwrap()
                .characteristics()
                .name
        ),
        "Shivan"
    );

    // 7. Create a token Bear, then cast Ixidron: turns all other nontoken face-up creatures face down,
    //    leaves token face up, leaves DFC/MDFC face up, and survives SBA with P/T == face-down count!
    activate(&mut engine, me(), tm, 0);
    settle(&mut engine, me());
    let bear_tok = engine
        .state()
        .battlefield_seen()
        .find(|&id| engine.state().object(id).is_some_and(|o| o.token.is_some()))
        .unwrap();
    let ix_id = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .iter()
        .copied()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(IXIDRON_CARD)
        })
        .unwrap();
    engine
        .apply(me(), PlayerAction::CastSpell { card: ix_id })
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(ix_id).unwrap().zone,
        Zone::Battlefield,
        "Ixidron survives SBA"
    );
    assert!(
        !engine
            .state()
            .object(ix_id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        !engine
            .state()
            .object(bear_tok)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "tokens stay face up"
    );
    assert!(
        engine
            .state()
            .object(mb)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .object(shivan)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .object(clone_id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    let ix_chars = engine.state().object(ix_id).unwrap().characteristics();
    assert_eq!((ix_chars.power, ix_chars.toughness), (Some(6), Some(6)));
}

#[test]
fn set_state_reveal_turn_face_up_if_etrata_if_cant_and_linked_exile() {
    let bear = card(
        VANILLA_BEAR,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bolt = card(
        BOLT_INSTANT,
        face("Bolt", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BOLT_ABILITIES,
    );
    let manifest_probe = card(
        MANIFEST_PROBE,
        face("Manifest Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MANIFEST_PROBE_ABILITIES,
    );
    let shrieker = card(
        SHRIEKER_PROBE,
        creature_face("Hauntwoods Shrieker", "{1}{G}{G}", 3, 3),
        KeywordSet::EMPTY,
        SHRIEKER_ABILITIES,
    );
    let etrata = card(
        ETRATA_PROBE,
        creature_face("Etrata Probe", "{1}{U}{B}", 1, 4),
        KeywordSet::EMPTY,
        ETRATA_ABILITIES,
    );
    let linked_probe = card(
        LINKED_EXILE_PROBE,
        face("Pyxis Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        LINKED_EXILE_ABILITIES,
    );

    let mut engine = bench(
        12,
        vec![bear, bolt, manifest_probe, shrieker, etrata, linked_probe],
        [
            Seat::with(&[
                MANIFEST_PROBE,
                SHRIEKER_PROBE,
                ETRATA_PROBE,
                LINKED_EXILE_PROBE,
            ])
            .holding(&[VANILLA_BEAR, BOLT_INSTANT, VANILLA_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let find = |e: &Bench, idx: u32| {
        e.state()
            .battlefield_seen()
            .find(|&id| e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(idx))
            .unwrap()
    };
    let mp = find(&engine, MANIFEST_PROBE);
    let sh = find(&engine, SHRIEKER_PROBE);
    let et = find(&engine, ETRATA_PROBE);
    let lp = find(&engine, LINKED_EXILE_PROBE);

    let hand = engine.state().zones.list(ZoneLocation::Hand(me())).clone();
    let bear1 = hand[0];
    let bolt1 = hand[1];
    let bear2 = hand[2];
    for id in [bear1, bolt1] {
        engine
            .state
            .move_object(
                id,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    activate(&mut engine, me(), mp, 0);
    settle(&mut engine, me());
    activate(&mut engine, me(), mp, 0);
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(bolt1)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .object(bear1)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    // 1. Hauntwoods Shrieker on manifested Instant: reveals, does NOT prompt YesNo, stays face down.
    let seq_before = engine.state().journal.last_seq();
    activate(&mut engine, me(), sh, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bolt1],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(bolt1)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .filter(|e| e.seq > seq_before)
            .any(
                |e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&bolt1))
            )
    );

    // 2. Hauntwoods Shrieker on manifested Creature: reveals, asks YesNo(MayDo), declining leaves face down,
    //    accepting turns face up!
    activate(&mut engine, me(), sh, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear1],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(bear1)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    activate(&mut engine, me(), sh, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bear1],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert!(
        !engine
            .state()
            .object(bear1)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );

    // 3. Etrata `if_cant` on manifested Instant: fails to turn face up (CR 701.40g) and exiles it!
    activate(&mut engine, me(), et, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![bolt1],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(bolt1).unwrap().zone, Zone::Exile);

    // 4. LinkedExile TurnFaceUp reveals a face-down card exiled linked to Pyxis Probe.
    engine
        .state
        .move_object(
            bear2,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let src_ref = engine.state().source_identity(lp).unwrap();
    let src_acq = engine.state().object(lp).unwrap().rules_acquisition();
    engine.state.hide_exiled_card(
        bear2,
        src_ref,
        src_acq,
        baylee_core::ids::SeatSet::new(),
        false,
    );
    assert!(
        engine
            .state()
            .object(bear2)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    activate(&mut engine, me(), lp, 0);
    settle(&mut engine, me());
    assert!(
        !engine
            .state()
            .object(bear2)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
}

#[test]
fn meld_pair_artifact_planeswalker_and_land_partners_and_departure() {
    let urza = card(
        URZA_PRIMARY,
        FaceDef {
            meld_partner: Some("The Mightstone and Weakstone"),
            ..creature_face("Urza, Lord Protector", "{1}{W}{U}", 2, 4)
        },
        KeywordSet::EMPTY,
        URZA_MELD_ABILITIES,
    );
    let stone = card(
        MIGHTSTONE_PARTNER,
        FaceDef {
            meld_partner: Some("Urza, Lord Protector"),
            ..face("The Mightstone and Weakstone", "{5}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let titania = card(
        TITANIA_PRIMARY,
        FaceDef {
            meld_partner: Some("Argoth, Sanctum of Nature"),
            ..creature_face("Titania, Voice of Gaea", "{1}{G}{G}", 3, 4)
        },
        KeywordSet::EMPTY,
        TITANIA_MELD_ABILITIES,
    );
    let argoth = card(
        ARGOTH_PARTNER,
        FaceDef {
            meld_partner: Some("Titania, Voice of Gaea"),
            ..land_face("Argoth, Sanctum of Nature")
        },
        KeywordSet::EMPTY,
        &[],
    );
    let destroy = card(
        DESTROY_INSTANT,
        face("Hero's Downfall", "{0}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        DESTROY_ANY_ABILITIES,
    );

    assert!(urza.is_meld_card());
    assert!(urza.is_double_faced());
    assert!(!urza.can_transform());

    let mut engine = bench(
        13,
        vec![urza, stone, titania, argoth, destroy],
        [
            Seat::with(&[
                URZA_PRIMARY,
                MIGHTSTONE_PARTNER,
                TITANIA_PRIMARY,
                ARGOTH_PARTNER,
            ])
            .holding(&[DESTROY_INSTANT]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let find = |e: &Bench, idx: u32| {
        e.state()
            .battlefield_seen()
            .find(|&id| e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(idx))
            .unwrap()
    };
    let u_id = find(&engine, URZA_PRIMARY);
    let s_id = find(&engine, MIGHTSTONE_PARTNER);
    let t_id = find(&engine, TITANIA_PRIMARY);
    let a_id = find(&engine, ARGOTH_PARTNER);

    assert!(!engine.state.can_turn_face_down(u_id));
    assert!(!engine.state.can_turn_face_down(s_id));

    activate(&mut engine, me(), u_id, 0);
    settle(&mut engine, me());
    let u_melded = engine.state().object(u_id).unwrap();
    assert_eq!(u_melded.zone, Zone::Battlefield);
    assert_eq!(
        engine.state().names.get(u_melded.characteristics().name),
        "Urza, Planeswalker"
    );
    assert_eq!(u_melded.characteristics().types, TypeSet::PLANESWALKER);
    assert_eq!(
        u_melded.counters.get(CounterKind::Loyalty),
        7,
        "melded planeswalker enters with starting loyalty"
    );
    assert_eq!(
        u_melded.characteristics().mana_value(),
        8,
        "combined MV = 3 + 5"
    );
    assert!(
        !engine.state.can_turn_face_down(u_id),
        "melded permanent cannot turn face down"
    );

    // Destroy melded Urza -> BOTH Urza and Mightstone arrive in graveyard!
    let downfall = engine.state().zones.list(ZoneLocation::Hand(me()))[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: downfall })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![u_id],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(u_id).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(s_id).unwrap().zone, Zone::Graveyard);

    // Meld Titania + Land partner -> Titania, Gaea Incarnate!
    activate(&mut engine, me(), t_id, 0);
    settle(&mut engine, me());
    let t_melded = engine.state().object(t_id).unwrap();
    assert_eq!(t_melded.zone, Zone::Battlefield);
    assert_eq!(
        engine.state().names.get(t_melded.characteristics().name),
        "Titania, Gaea Incarnate"
    );
    assert_eq!(engine.state().object(a_id).unwrap().zone, Zone::Command);
}

#[test]
fn meld_pair_mishra_attacking_requirement_enters_tapped_attacking_and_clone_stays_exiled() {
    let mishra = card(
        MISHRA_PRIMARY,
        FaceDef {
            meld_partner: Some("Phyrexian Dragon Engine"),
            ..creature_face("Mishra, Claimed by Gix", "{2}{B}{R}", 3, 5)
        },
        KeywordSet::EMPTY,
        MISHRA_MELD_ABILITIES,
    );
    let dragon_engine = card(
        DRAGON_ENGINE_PARTNER,
        FaceDef {
            meld_partner: Some("Mishra, Claimed by Gix"),
            ..creature_face("Phyrexian Dragon Engine", "{3}", 2, 2)
        },
        KeywordSet::EMPTY,
        &[],
    );
    let attack_watcher = card(
        ATTACK_WATCHER,
        face("Attack Watcher", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ATTACK_WATCHER_ABILITIES,
    );
    let clone_def = card(
        COPY_CLONE,
        creature_face("Clone", "{0}", 0, 0),
        KeywordSet::EMPTY,
        CLONE_ABILITIES,
    );

    // 1. Only Mishra attacks (Dragon Engine stays back) -> does NOT meld!
    let mut e1 = bench(
        14,
        vec![mishra, dragon_engine],
        [
            Seat::with(&[MISHRA_PRIMARY, DRAGON_ENGINE_PARTNER]),
            Seat::default(),
        ],
    );
    walk_until(&mut e1, |e| {
        e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
    });
    let m1 = e1
        .state()
        .battlefield_seen()
        .find(|&id| {
            e1.state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(MISHRA_PRIMARY)
        })
        .unwrap();
    e1.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![(m1, baylee_core::ids::Defender::Player(them()))],
        },
    )
    .unwrap();
    settle(&mut e1, me());
    assert_eq!(
        e1.state()
            .names
            .get(e1.state().object(m1).unwrap().characteristics().name),
        "Mishra, Claimed by Gix"
    );

    // 2. Both Mishra and Dragon Engine attack -> melds into tapped & attacking Mishra, Lost to Phyrexia!
    let mut e2 = bench(
        15,
        vec![mishra, dragon_engine, attack_watcher],
        [
            Seat::with(&[MISHRA_PRIMARY, DRAGON_ENGINE_PARTNER, ATTACK_WATCHER]),
            Seat::default(),
        ],
    );
    walk_until(&mut e2, |e| {
        e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
    });
    let m2 = e2
        .state()
        .battlefield_seen()
        .find(|&id| {
            e2.state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(MISHRA_PRIMARY)
        })
        .unwrap();
    let d2 = e2
        .state()
        .battlefield_seen()
        .find(|&id| {
            e2.state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(DRAGON_ENGINE_PARTNER)
        })
        .unwrap();
    let hand_before = e2.state().zones.list(ZoneLocation::Hand(me())).len();
    e2.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (m2, baylee_core::ids::Defender::Player(them())),
                (d2, baylee_core::ids::Defender::Player(them())),
            ],
        },
    )
    .unwrap();
    settle(&mut e2, me());

    let melded = e2.state().object(m2).unwrap();
    assert_eq!(melded.zone, Zone::Battlefield);
    assert_eq!(
        e2.state().names.get(melded.characteristics().name),
        "Mishra, Lost to Phyrexia"
    );
    assert!(melded.status.contains(Status::TAPPED));
    assert!(
        e2.state().combat.is_attacking(m2),
        "melded Mishra enters attacking"
    );
    assert_eq!(
        e2.state().players[0].life,
        24,
        "ETB trigger (+4 life) fired; ThisAttacks (-3 life) did NOT"
    );
    assert_eq!(
        e2.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before + 2,
        "only the 2 declared attackers fired CreatureAttacks (CR 508.4: entering attacking did not)"
    );

    // 3. CR 701.42b: Clone copying Dragon Engine exiles both without melding!
    let mut e3 = bench(
        16,
        vec![mishra, dragon_engine, clone_def],
        [
            Seat::with(&[MISHRA_PRIMARY]).holding(&[COPY_CLONE]),
            Seat::with(&[DRAGON_ENGINE_PARTNER]),
        ],
    );
    to_main(&mut e3, me());
    let opp_engine = e3
        .state()
        .battlefield_seen()
        .find(|&id| {
            e3.state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(DRAGON_ENGINE_PARTNER)
        })
        .unwrap();
    let clone_id = e3.state().zones.list(ZoneLocation::Hand(me()))[0];
    e3.apply(me(), PlayerAction::CastSpell { card: clone_id })
        .unwrap();
    walk_until(&mut e3, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e3.apply(
        me(),
        PlayerAction::ChooseObjects {
            objects: vec![opp_engine],
        },
    )
    .unwrap();
    settle(&mut e3, me());

    walk_until(&mut e3, |e| {
        e.state().turn.number >= 3
            && matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me())
    });
    let m3 = e3
        .state()
        .battlefield_seen()
        .find(|&id| {
            e3.state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index.0)
                == Some(MISHRA_PRIMARY)
        })
        .unwrap();
    e3.apply(
        me(),
        PlayerAction::DeclareAttackers {
            attackers: vec![
                (m3, baylee_core::ids::Defender::Player(them())),
                (clone_id, baylee_core::ids::Defender::Player(them())),
            ],
        },
    )
    .unwrap();
    settle(&mut e3, me());
    assert_eq!(
        e3.state().object(m3).unwrap().zone,
        Zone::Exile,
        "CR 701.42b: Mishra stays exiled"
    );
    assert_eq!(
        e3.state().object(clone_id).unwrap().zone,
        Zone::Exile,
        "CR 701.42b: Clone stays exiled"
    );
}

const CAPTURE_SET_STATE_PROBE: u32 = 9836;

static CAPTURE_MANIFEST_TURN_UP_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::CaptureAffected {
        first: &[Effect::ManifestTop {
            who: PlayerRel::You,
            count: Amount::Fixed(1),
        }],
        then: &[Effect::SetState {
            subject: StateSubject::Remembered,
            transition: StateTransition::TurnFaceUp,
            if_cant: &[Effect::draw(1)],
        }],
    }],
    None,
)];

/// Item 1 regression (`CR 727.2` / `CR 712.16`):
/// Mutate and Meld both record `Rider::MergedComponent`, so a mutated
/// single-faced creature must NOT match `Filter::DoubleFaced` and MUST be able
/// to turn face down (restoring its mutated base on turn-face-up), whereas a
/// mutated permanent containing a DFC component and a melded permanent MUST
/// match `Filter::DoubleFaced` and MUST be blocked from turning face down.
#[test]
fn mutate_single_faced_vs_dfc_and_meld_cr727_2_and_cr712_16() {
    let bear = card(
        VANILLA_BEAR,
        creature_face("Grizzly Bears", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let dragon = card(
        BIG_DRAGON,
        creature_face("Mutate Beast", "{2}{G}", 4, 4),
        KeywordSet::TRAMPLE,
        &[],
    );
    let werewolf = two_face_card(
        DFC_WEREWOLF,
        creature_face("Huntmaster", "{2}{R}{G}", 2, 2),
        FaceDef {
            castable_from_hand: false,
            ..creature_face("Ravager", "", 4, 4)
        },
    );
    let urza = card(
        URZA_PRIMARY,
        FaceDef {
            meld_partner: Some("The Mightstone and Weakstone"),
            ..creature_face("Urza, Lord Protector", "{1}{W}{U}", 2, 4)
        },
        KeywordSet::EMPTY,
        URZA_MELD_ABILITIES,
    );
    let mightstone = card(
        MIGHTSTONE_PARTNER,
        FaceDef {
            meld_partner: Some("Urza, Lord Protector"),
            ..face("The Mightstone and Weakstone", "{5}", TypeSet::ARTIFACT)
        },
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        91,
        vec![bear, dragon, werewolf, urza, mightstone],
        [
            Seat::with(&[
                VANILLA_BEAR,
                BIG_DRAGON,
                DFC_WEREWOLF,
                URZA_PRIMARY,
                MIGHTSTONE_PARTNER,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let find_card = |e: &Bench, idx: u32| {
        e.state()
            .battlefield_seen()
            .find(|&id| e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(idx))
            .unwrap()
    };
    let bear_id = find_card(&engine, VANILLA_BEAR);
    let dragon_id = find_card(&engine, BIG_DRAGON);
    let dfc_id = find_card(&engine, DFC_WEREWOLF);
    let urza_id = find_card(&engine, URZA_PRIMARY);

    // Merge two single-faced creatures as a mutated permanent (matching resolve/mutate.rs).
    let top_base = std::sync::Arc::clone(&engine.state().object(dragon_id).unwrap().base);
    {
        let host = engine.state.object_mut(bear_id).unwrap();
        host.original_base = Some(std::sync::Arc::clone(&host.base));
        host.base = top_base;
        host.riders.push(Rider::MergedTop(dragon_id));
        host.riders.push(Rider::MergedComponent(dragon_id));
        host.cache.clear();
    }
    engine
        .state
        .move_object(
            dragon_id,
            ZoneLocation::Command(me()),
            ZonePosition::Top,
            crate::event::Cause::Spell,
        )
        .unwrap();
    engine
        .state
        .object_mut(dragon_id)
        .unwrap()
        .riders
        .push(Rider::MergedInto(bear_id));
    engine.state.invalidate_projections();

    assert!(
        !engine.state().is_melded_permanent(bear_id),
        "mutated single-faced creature is not melded"
    );
    assert!(
        !crate::eval::matches(
            &Filter::DoubleFaced,
            engine.state(),
            engine.state().object(bear_id).unwrap(),
            me(),
            ObjectId::NO_SOURCE,
        ),
        "CR 727.2: mutated single-faced creature must not match Filter::DoubleFaced"
    );
    assert!(
        engine.state().can_turn_face_down(bear_id),
        "CR 727.2: mutated single-faced creature can turn face down"
    );
    assert!(
        engine
            .state
            .turn_permanent_face_down(bear_id, FaceDownSpec::DEFAULT)
    );
    assert_eq!(
        engine
            .state()
            .object(bear_id)
            .unwrap()
            .characteristics()
            .power,
        Some(2)
    );
    assert!(engine.state.turn_permanent_face_up(bear_id));
    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(bear_id)
                .unwrap()
                .characteristics()
                .name
        ),
        "Mutate Beast",
        "turning mutated permanent back face up restores its mutated base"
    );

    // Merging a DFC component into a permanent makes it double-faced (CR 727.2 / 712.16).
    engine
        .state
        .object_mut(bear_id)
        .unwrap()
        .riders
        .push(Rider::MergedComponent(dfc_id));
    assert!(crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        engine.state().object(bear_id).unwrap(),
        me(),
        ObjectId::NO_SOURCE,
    ));
    assert!(
        !engine.state().can_turn_face_down(bear_id),
        "CR 727.2/712.16: merged permanent with a DFC component cannot turn face down"
    );

    // Melded permanent is DoubleFaced, cannot transform, and cannot turn face down.
    activate(&mut engine, me(), urza_id, 0);
    settle(&mut engine, me());
    assert!(engine.state().is_melded_permanent(urza_id));
    assert!(crate::eval::matches(
        &Filter::DoubleFaced,
        engine.state(),
        engine.state().object(urza_id).unwrap(),
        me(),
        ObjectId::NO_SOURCE,
    ));
    assert!(!crate::eval::matches(
        &Filter::CanTransform,
        engine.state(),
        engine.state().object(urza_id).unwrap(),
        me(),
        ObjectId::NO_SOURCE,
    ));
    assert!(!engine.state().can_turn_face_down(urza_id));
}

/// Item 2 regression (`CR 400.7`):
/// `StateSubject::LinkedExile` must enforce `source.version` on both
/// `Rider::ExiledWith` and `Rider::LinkedExile`, so blinking the host
/// permanent prevents it from turning face-up cards exiled by its prior
/// incarnation.
#[test]
fn linked_exile_set_state_enforces_cr400_7_version_after_blink() {
    let probe = card(
        LINKED_EXILE_PROBE,
        face("Exile Host", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        LINKED_EXILE_ABILITIES,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        92,
        vec![probe, bear],
        [
            Seat::with(&[LINKED_EXILE_PROBE]).holding(&[
                VANILLA_BEAR,
                VANILLA_BEAR,
                VANILLA_BEAR,
                VANILLA_BEAR,
            ]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());

    let host = engine.state().battlefield_seen().next().unwrap();
    let v1 = engine.state().object(host).unwrap().version;
    let acq1 = engine.state().object(host).unwrap().rules_acquisition();
    let cards: Vec<_> = engine.state().zones.list(ZoneLocation::Hand(me())).clone();
    let (stale_ew, stale_le, fresh_ew, fresh_le) = (cards[0], cards[1], cards[2], cards[3]);

    // Exile two cards linked to host's v1 incarnation (one ExiledWith, one LinkedExile).
    for &c in &[stale_ew, stale_le] {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        let o = engine.state.object_mut(c).unwrap();
        o.status.insert(Status::FACE_DOWN);
        if c == stale_ew {
            o.riders.push(Rider::ExiledWith {
                host,
                version: u16::try_from(v1).unwrap(),
            });
        } else {
            o.riders.push(Rider::LinkedExile {
                source: baylee_core::ids::DamageSourceRef {
                    object: host,
                    version: v1,
                },
                acquisition: acq1,
            });
        }
    }

    // Blink host -> new version v2 > v1 (CR 400.7).
    engine
        .state
        .move_object(
            host,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine
        .state
        .move_object(
            host,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let v2 = engine.state().object(host).unwrap().version;
    let acq2 = engine.state().object(host).unwrap().rules_acquisition();
    assert_ne!(v1, v2);

    // Activating host v2 must NOT turn v1's exiled cards face up.
    activate(&mut engine, me(), host, 0);
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .object(stale_ew)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "CR 400.7: stale ExiledWith version must remain face down"
    );
    assert!(
        engine
            .state()
            .object(stale_le)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN),
        "CR 400.7: stale LinkedExile version must remain face down"
    );

    // Exile two cards linked to host v2 -> ONLY those two turn face up.
    for &c in &[fresh_ew, fresh_le] {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Exile(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        let o = engine.state.object_mut(c).unwrap();
        o.status.insert(Status::FACE_DOWN);
        if c == fresh_ew {
            o.riders.push(Rider::ExiledWith {
                host,
                version: u16::try_from(v2).unwrap(),
            });
        } else {
            o.riders.push(Rider::LinkedExile {
                source: baylee_core::ids::DamageSourceRef {
                    object: host,
                    version: v2,
                },
                acquisition: acq2,
            });
        }
    }
    activate(&mut engine, me(), host, 0);
    settle(&mut engine, me());
    assert!(
        !engine
            .state()
            .object(fresh_ew)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        !engine
            .state()
            .object(fresh_le)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .object(stale_ew)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert!(
        engine
            .state()
            .object(stale_le)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
}

/// Item 3 regression (`Effect::CaptureAffected` -> `StateSubject::Remembered`):
/// `StateSubject::Remembered` reads `res.subject.remembered_objects` populated
/// by `CaptureAffected` (e.g. `RememberChanged$ True` -> `SetState`), turning a
/// manifested creature face up immediately and triggering `if_cant` for an
/// instant that cannot turn face up (`CR 701.40g`).
#[test]
fn capture_affected_remembered_drives_set_state_transition() {
    let probe = card(
        CAPTURE_SET_STATE_PROBE,
        face("Capture Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CAPTURE_MANIFEST_TURN_UP_ABILITIES,
    );
    let bear = card(
        VANILLA_BEAR,
        creature_face("Grizzly Bears", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bolt = card(
        BOLT_INSTANT,
        face("Bolt", "{R}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BOLT_ABILITIES,
    );

    let mut engine = bench(
        93,
        vec![probe, bear, bolt],
        [
            Seat::with(&[CAPTURE_SET_STATE_PROBE]).holding(&[BOLT_INSTANT, VANILLA_BEAR]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let probe_id = engine.state().battlefield_seen().next().unwrap();
    // Place BOLT_INSTANT then VANILLA_BEAR on top of library so VANILLA_BEAR manifests first.
    let hand_cards: Vec<_> = engine.state().zones.list(ZoneLocation::Hand(me())).clone();
    for c in hand_cards {
        engine
            .state
            .move_object(
                c,
                ZoneLocation::Library(me()),
                ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }

    // 1. Manifest top (VANILLA_BEAR) -> CaptureAffected remembers it -> TurnFaceUp flips it face up.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), probe_id, 0);
    settle(&mut engine, me());
    let face_up_bear = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            id != probe_id
                && engine
                    .state()
                    .object(id)
                    .is_some_and(|o| !o.status.contains(Status::FACE_DOWN))
        })
        .expect("captured manifested creature turned face up via StateSubject::Remembered");
    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(face_up_bear)
                .unwrap()
                .characteristics()
                .name
        ),
        "Grizzly Bears"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before,
        "if_cant must not run when TurnFaceUp succeeds"
    );

    // 2. Manifest next top (BOLT_INSTANT) -> cannot turn face up (CR 701.40g) -> stays face down and runs if_cant (draw 1).
    let hand_before_bolt = engine.state().zones.list(ZoneLocation::Hand(me())).len();
    activate(&mut engine, me(), probe_id, 0);
    settle(&mut engine, me());
    let face_down_bolt = engine
        .state()
        .battlefield_seen()
        .find(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.status.contains(Status::FACE_DOWN))
        })
        .expect("instant stays face down");
    assert_eq!(
        engine
            .state()
            .object(face_down_bolt)
            .unwrap()
            .card
            .map(|c| c.index.0),
        Some(BOLT_INSTANT)
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me())).len(),
        hand_before_bolt + 1,
        "if_cant ran when captured instant failed to turn face up"
    );
}

/// Item 6 hash/fingerprint sensitivity verification for reserved tags `520..=527`:
/// - `520..=524`: `Filter::{HasMorph, DoubleFaced, CanTransform, Flipped, HasOmen}`
/// - `525`: `Rider::TurnedFaceDown` (`snapshot_hash`, `loop_signature`, `Engine::fingerprint`)
/// - `526`: `face_down_copied_bases` (`snapshot_hash`, `loop_signature`, `Engine::fingerprint`)
/// - `527`: `AwaitingOp::SetStateOptionalTurnFaceUp` (`Engine::fingerprint`)
#[test]
fn hash_and_fingerprint_sensitivity_for_tags_520_through_527() {
    static FIVE_FILTERS: [Filter; 5] = [
        Filter::HasMorph,
        Filter::DoubleFaced,
        Filter::CanTransform,
        Filter::Flipped,
        Filter::HasOmen,
    ];
    let bear = card(
        VANILLA_BEAR,
        creature_face("Grizzly Bears", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let dragon = card(
        BIG_DRAGON,
        creature_face("Shivan Dragon", "{4}{R}{R}", 5, 5),
        KeywordSet::FLYING,
        &[],
    );
    let shrieker = card(
        SHRIEKER_PROBE,
        face("Shrieker", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SHRIEKER_ABILITIES,
    );

    // 1. Tags 520..=524: each new Filter hashes distinctly in snapshot_hash & loop_signature.
    let mut filter_snaps = std::collections::BTreeSet::new();
    let mut filter_fps = Vec::new();
    for filter in &FIVE_FILTERS {
        let mut e = bench(
            94,
            vec![bear],
            [Seat::with(&[VANILLA_BEAR]), Seat::default()],
        );
        to_main(&mut e, me());
        let ts = e.state.next_timestamp();
        e.state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Modifier::ModifyPT(1, 1).layer(),
            timestamp: ts,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::Dsl(filter),
            modifier: Modifier::ModifyPT(1, 1),
        });
        filter_snaps.insert(e.state().snapshot_hash());
        let fp = e.fingerprint();
        if !filter_fps.contains(&fp) {
            filter_fps.push(fp);
        }
    }
    assert_eq!(
        filter_snaps.len(),
        5,
        "tags 520..=524 distinct in snapshot_hash"
    );
    assert_eq!(
        filter_fps.len(),
        5,
        "tags 520..=524 distinct in Engine::fingerprint"
    );

    // 2. Tag 525 (Rider::TurnedFaceDown) & Tag 526 (face_down_copied_bases).
    let make_fd_bench = || {
        let mut e = bench(
            95,
            vec![bear, dragon],
            [Seat::with(&[VANILLA_BEAR, BIG_DRAGON]), Seat::default()],
        );
        to_main(&mut e, me());
        let b_id = e
            .state()
            .battlefield_seen()
            .find(|&id| {
                e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(VANILLA_BEAR)
            })
            .unwrap();
        let d_id = e
            .state()
            .battlefield_seen()
            .find(|&id| {
                e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(BIG_DRAGON)
            })
            .unwrap();
        assert!(
            e.state
                .turn_permanent_face_down(b_id, FaceDownSpec::DEFAULT)
        );
        (e, b_id, d_id)
    };
    let (mut e_disguise, bear_id, _) = make_fd_bench();
    let (e_turned_down, _, _) = make_fd_bench();
    // Strip Rider::TurnedFaceDown on e_disguise so the two states differ ONLY in tag 525.
    e_disguise
        .state
        .object_mut(bear_id)
        .unwrap()
        .riders
        .retain(|r| *r != Rider::TurnedFaceDown);
    e_disguise.state.invalidate_projections();

    assert_ne!(
        e_disguise.state().snapshot_hash(),
        e_turned_down.state().snapshot_hash(),
        "tag 525 must distinguish snapshot_hash"
    );
    assert_ne!(
        e_disguise.state().loop_signature(),
        e_turned_down.state().loop_signature(),
        "tag 525 must distinguish loop_signature"
    );
    assert_ne!(
        e_disguise.fingerprint(),
        e_turned_down.fingerprint(),
        "tag 525 must distinguish Engine::fingerprint"
    );

    // Now differ ONLY in tag 526 (`face_down_copied_bases`).
    let (mut e_copied_base, bear_id_c, dragon_id_c) = make_fd_bench();
    let version = e_copied_base.state().object(bear_id_c).unwrap().version;
    let dragon_base =
        std::sync::Arc::clone(&e_copied_base.state().object(dragon_id_c).unwrap().base);
    e_copied_base.state.face_down_copied_bases.push((
        baylee_core::ids::DamageSourceRef {
            object: bear_id_c,
            version,
        },
        dragon_base,
    ));
    assert_ne!(
        e_turned_down.state().snapshot_hash(),
        e_copied_base.state().snapshot_hash(),
        "tag 526 must distinguish snapshot_hash"
    );
    assert_ne!(
        e_turned_down.state().loop_signature(),
        e_copied_base.state().loop_signature(),
        "tag 526 must distinguish loop_signature"
    );
    assert_ne!(
        e_turned_down.fingerprint(),
        e_copied_base.fingerprint(),
        "tag 526 must distinguish Engine::fingerprint"
    );

    // 3. Tag 527 (`AwaitingOp::SetStateOptionalTurnFaceUp`) in Engine::fingerprint.
    let make_await_bench = |pick_dragon: bool| {
        let mut e = bench(
            96,
            vec![shrieker, bear, dragon],
            [
                Seat::with(&[SHRIEKER_PROBE, VANILLA_BEAR, BIG_DRAGON]),
                Seat::default(),
            ],
        );
        to_main(&mut e, me());
        let probe_id = e
            .state()
            .battlefield_seen()
            .find(|&id| {
                e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(SHRIEKER_PROBE)
            })
            .unwrap();
        let fd_bear = e
            .state()
            .battlefield_seen()
            .find(|&id| {
                e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(VANILLA_BEAR)
            })
            .unwrap();
        let fd_dragon = e
            .state()
            .battlefield_seen()
            .find(|&id| {
                e.state().object(id).and_then(|o| o.card).map(|c| c.index.0) == Some(BIG_DRAGON)
            })
            .unwrap();
        assert!(
            e.state
                .turn_permanent_face_down(fd_bear, FaceDownSpec::DEFAULT)
        );
        assert!(
            e.state
                .turn_permanent_face_down(fd_dragon, FaceDownSpec::DEFAULT)
        );
        e.pending = Pending::Priority {
            player: me(),
            legal: Box::new(e.compute_legal(me())),
        };
        let chosen = if pick_dragon { fd_dragon } else { fd_bear };
        activate(&mut e, me(), probe_id, 0);
        e.apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .unwrap();
        walk_until(&mut e, |eng| {
            matches!(
                eng.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            )
        });
        e
    };
    let e_await_a = make_await_bench(false);
    let e_await_b = make_await_bench(true);
    assert_ne!(
        e_await_a.fingerprint(),
        e_await_b.fingerprint(),
        "tag 527 must distinguish Engine::fingerprint for SetStateOptionalTurnFaceUp"
    );
}
