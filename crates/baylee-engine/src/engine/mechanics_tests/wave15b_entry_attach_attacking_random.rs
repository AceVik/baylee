//! Engine Wave 15b deterministic rule tests:
//! 1. Zone-entry attachments (`Effect::PutOntoBattlefieldAttached`,
//!    `Effect::CreateTokenAttached` — CR 303.4f–g, CR 301.5c, CR 301.6,
//!    CR 704.5y Role uniqueness).
//! 2. Entering battlefield tapped & attacking (`Effect::PutOntoBattlefieldAttacking` —
//!    CR 508.4, CR 506.3b, CR 508.3a, Alesha-style defender selection).
//! 3. Deterministic random card choice (`Effect::ChooseCardConstrained` with
//!    `at_random: true`, reproducible under identical `state.rng` seeds).
//! 4. Distinct-names enforcement (`Effect::SearchLibraryDistinctNames`,
//!    `Effect::ChooseCardConstrained` with `distinct_names: true`).
//! 5. Optional & mandatory attach chosen (`Effect::AttachChosen`,
//!    `Effect::AttachSlot { optional: true }`) and discarding remembered/found
//!    cards (`Effect::DiscardDefined` — CR 701.9a).

#![allow(clippy::too_many_lines)] // comprehensive multi-case rule tests

use super::*;
use crate::choice::{ChoicePrompt, YesNoPrompt};
use crate::event::{Cause, GameEvent};
use crate::object::Status;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::prelude::{spell, static_ability, triggered};
use baylee_cards_dsl::{
    ActivationLimit, ActivationTiming, ActivationZone, Amount, AttachHost, ChooseMove, ChooseZone,
    DefinedObjects, Filter, Find, Modifier, PlayerRel, TargetSlot, TargetSpec, TokenDef, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::{artifact, enchantment};
use baylee_core::ids::{CardIndex, Defender, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

const C0: u32 = 99_970;
const C1: u32 = 99_971;
const C2: u32 = 99_972;
const C3: u32 = 99_973;
const C4: u32 = 99_974;
const C5: u32 = 99_975;

static AURA_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[enchantment::AURA];
static ROLE_AURA_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[enchantment::AURA, enchantment::ROLE];
static EQUIPMENT_SUBTYPES: &[baylee_core::ids::SubtypeId] = &[artifact::EQUIPMENT];

static WHITE_AURA_ABILITIES: &[AbilityDef] = &[
    spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),
    static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2)),
];

static MONSTER_ROLE_ABILITIES: &[AbilityDef] = &[
    spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),
    static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
    static_ability!(
        Filter::AttachedToBySource,
        Modifier::AddKeyword(KeywordSet::TRAMPLE)
    ),
];

static MONSTER_ROLE_TOKEN: TokenDef = TokenDef {
    name: "Monster",
    colors: ColorSet::EMPTY,
    types: TypeSet::ENCHANTMENT,
    subtypes: ROLE_AURA_SUBTYPES,
    abilities: MONSTER_ROLE_ABILITIES,
    ..TokenDef::DEFAULT
};

static WICKED_ROLE_ABILITIES: &[AbilityDef] = &[
    spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),
    static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
];

static WICKED_ROLE_TOKEN: TokenDef = TokenDef {
    name: "Wicked",
    colors: ColorSet::EMPTY,
    types: TypeSet::ENCHANTMENT,
    subtypes: ROLE_AURA_SUBTYPES,
    abilities: WICKED_ROLE_ABILITIES,
    ..TokenDef::DEFAULT
};

static SWORD_TOKEN_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::AttachedToBySource,
    Modifier::ModifyPT(2, 0)
)];

static SWORD_EQUIPMENT_TOKEN: TokenDef = TokenDef {
    name: "Blade",
    colors: ColorSet::EMPTY,
    types: TypeSet::ARTIFACT,
    subtypes: EQUIPMENT_SUBTYPES,
    abilities: SWORD_TOKEN_ABILITIES,
    ..TokenDef::DEFAULT
};

static EQUIP_PLUS_3_0: &[AbilityDef] = &[static_ability!(
    Filter::AttachedToBySource,
    Modifier::ModifyPT(3, 0)
)];

static PRO_WHITE_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::White])))
)];

static PRO_ENCHANTMENT_ABILITIES: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::ProtectionFrom(&Filter::HasType(TypeSet::ENCHANTMENT))
)];

fn aim_object(engine: &mut Bench, seat: PlayerId, obj: ObjectId) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![obj],
                players: vec![],
            },
        )
        .expect("object target accepted");
}

fn choose_objects(objects: Vec<ObjectId>) -> PlayerAction {
    PlayerAction::ChooseObjects { objects }
}

fn refresh_legal(engine: &mut Bench) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn white_aura_card(index: u32) -> &'static CardDef {
    let f = FaceDef {
        subtypes: AURA_SUBTYPES,
        ..face("Blessed Mantle", "{1}{W}", TypeSet::ENCHANTMENT)
    };
    card(index, f, KeywordSet::EMPTY, WHITE_AURA_ABILITIES)
}

fn equipment_card(index: u32, name: &'static str) -> &'static CardDef {
    let f = FaceDef {
        subtypes: EQUIPMENT_SUBTYPES,
        ..face(name, "{2}", TypeSet::ARTIFACT)
    };
    card(index, f, KeywordSet::EMPTY, EQUIP_PLUS_3_0)
}

fn planeswalker_card(index: u32, name: &'static str, loyalty: u16) -> &'static CardDef {
    let f = FaceDef {
        loyalty: Some(loyalty),
        ..face(name, "{3}", TypeSet::PLANESWALKER)
    };
    card(index, f, KeywordSet::EMPTY, &[])
}

fn bench_with_deck(
    seed: u64,
    cards: Vec<&'static CardDef>,
    my_deck: Vec<u32>,
    my_bf: &[u32],
    my_hand: &[u32],
) -> Bench {
    let to_entry = |c: u32| DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(c),
        print: PrintRef::new(0),
    };
    let opp_deck: Vec<DeckEntry> = (0..60)
        .map(|_| to_entry(super::synthetic::forest()))
        .collect();
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules {
            starting_player: Some(0),
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![
            SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities {
                    dev_commands: true,
                    see_hidden: false,
                },
                deck: my_deck.into_iter().map(to_entry).collect(),
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: Some(my_hand.iter().copied().map(to_entry).collect()),
                starting_battlefield: my_bf.iter().copied().map(to_entry).collect(),
                emblems: vec![],
                team: None,
            },
            SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities {
                    dev_commands: true,
                    see_hidden: false,
                },
                deck: opp_deck,
                sideboard: vec![],
                commanders: vec![],
                starting_life: None,
                starting_hand: Some(vec![]),
                starting_battlefield: vec![],
                emblems: vec![],
                team: None,
            },
        ],
    };
    let mut engine =
        Engine::new(&preset, super::synthetic::SyntheticLookup::new(cards)).expect("bench builds");
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

// -----------------------------------------------------------------------------
// 1. CR 303.4f / 303.4g Aura ETB attached (legal + illegal stays in prior zone)
//    and Equipment ETB attached (legal + unattached fallback)
// -----------------------------------------------------------------------------

#[test]
fn cr_303_4f_aura_enters_attached_without_targeting_and_illegal_stays_in_prior_zone() {
    static PUT_GY_AURA_FX: &[Effect] = &[Effect::PutOntoBattlefieldAttached {
        subject: DefinedObjects::Spec(TargetSpec::CardInGraveyard(
            &Filter::HasType(TypeSet::ENCHANTMENT),
            PlayerRel::You,
        )),
        host: AttachHost::ChosenPermanent(&Filter::CREATURE),
        owner_control: false,
        tapped: false,
    }];
    static PUT_GY_EQUIP_FX: &[Effect] = &[Effect::PutOntoBattlefieldAttached {
        subject: DefinedObjects::Spec(TargetSpec::CardInGraveyard(
            &Filter::HasType(TypeSet::ARTIFACT),
            PlayerRel::You,
        )),
        host: AttachHost::ChosenPermanent(&Filter::CREATURE),
        owner_control: false,
        tapped: false,
    }];
    static ENGINE_AB: &[AbilityDef] = &[
        free(
            PUT_GY_AURA_FX,
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::HasType(TypeSet::ENCHANTMENT),
                PlayerRel::You,
            ))),
        ),
        free(
            PUT_GY_EQUIP_FX,
            Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::HasType(TypeSet::ARTIFACT),
                PlayerRel::You,
            ))),
        ),
    ];

    let relocator = card(
        C0,
        face("Relocator Engine", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        ENGINE_AB,
    );
    let aura_def = white_aura_card(C1);
    let hex_creature = card(
        C2,
        creature_face("Slippery Bogle", "{1}", 1, 1),
        KeywordSet::HEXPROOF,
        &[],
    );
    let pro_white = card(
        C3,
        creature_face("White Shield", "{2}", 2, 2),
        KeywordSet::EMPTY,
        PRO_WHITE_ABILITIES,
    );
    let eq_def = equipment_card(C4, "Iron Greatsword");

    // Case A: Legal Hexproof creature (owned by opponent!) — CR 303.4f ignores hexproof!
    let mut engine = bench(
        15_101,
        vec![relocator, aura_def, hex_creature, eq_def],
        [Seat::with(&[C0]).holding(&[C1, C4]), Seat::with(&[C2])],
    );
    to_main(&mut engine, me());
    let eng_id = the(&engine, ZoneLocation::Battlefield, C0);
    let aura_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let eq_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let bogle_id = the(&engine, ZoneLocation::Battlefield, C2);

    // Move aura and equipment to graveyard so we test graveyard -> battlefield entry attached.
    let _ = engine.state.move_object(
        aura_id,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    let _ = engine.state.move_object(
        eq_id,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    refresh_legal(&mut engine);

    activate(&mut engine, me(), eng_id, 0);
    aim_object(&mut engine, me(), aura_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    // Must offer opponent's Hexproof Bogle via ChoicePrompt::NewHost (untargeted CR 303.4f).
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            min,
            max,
            prompt: ChoicePrompt::NewHost { aura },
            ..
        } => {
            assert_eq!(*player, me());
            assert_eq!(*min, 1);
            assert_eq!(*max, 1);
            assert_eq!(*aura, aura_id);
            assert_eq!(options, &vec![bogle_id]);
        }
        other => panic!("expected NewHost choice for CR 303.4f Aura entry, got {other:?}"),
    }
    engine.apply(me(), choose_objects(vec![bogle_id])).unwrap();
    settle(&mut engine, me());

    let aura_obj = engine.state().object(aura_id).unwrap();
    assert_eq!(aura_obj.zone, Zone::Battlefield);
    assert_eq!(aura_obj.attached_to, Some(bogle_id));
    let bogle = engine.state().object(bogle_id).unwrap();
    assert_eq!(bogle.characteristics().power, Some(3));
    assert_eq!(bogle.characteristics().toughness, Some(3));

    // Equipment enters attached to Bogle as well (untargeted -> ignores hexproof).
    activate(&mut engine, me(), eng_id, 1);
    aim_object(&mut engine, me(), eq_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), choose_objects(vec![bogle_id])).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(eq_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(eq_id).unwrap().attached_to,
        Some(bogle_id)
    );
    assert_eq!(
        engine
            .state()
            .object(bogle_id)
            .unwrap()
            .characteristics()
            .power,
        Some(6)
    );

    // Case B: NO legal host (only a creature with Protection from White) —
    // CR 303.4g: White Aura MUST stay in graveyard and NEVER enter the battlefield!
    // Meanwhile an Equipment with no legal host STILL enters unattached (CR 301.5c).
    let mut blocked = bench(
        15_102,
        vec![relocator, aura_def, pro_white, eq_def],
        [Seat::with(&[C0]).holding(&[C1, C4]), Seat::with(&[C3])],
    );
    to_main(&mut blocked, me());
    let eng2 = the(&blocked, ZoneLocation::Battlefield, C0);
    let aura2 = the(&blocked, ZoneLocation::Hand(me()), C1);
    let eq2 = the(&blocked, ZoneLocation::Hand(me()), C4);
    let _ = blocked.state.move_object(
        aura2,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    let _ = blocked.state.move_object(
        eq2,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    refresh_legal(&mut blocked);

    let journal_before = blocked.journal().entries().len();
    activate(&mut blocked, me(), eng2, 0);
    aim_object(&mut blocked, me(), aura2);
    settle(&mut blocked, me());

    assert_eq!(
        blocked.state().object(aura2).unwrap().zone,
        Zone::Graveyard,
        "CR 303.4g: Aura with no legal host must remain in its prior zone"
    );
    let aura_entered = blocked.journal().entries()[journal_before..]
        .iter()
        .any(|e| {
            matches!(
                e.event,
                GameEvent::ZoneChanged {
                    object,
                    to: Zone::Battlefield,
                    ..
                } if object == aura2
            )
        });
    assert!(
        !aura_entered,
        "CR 303.4g: Aura with no legal host must never enter the battlefield"
    );

    // Remove the Pro-White creature and put Equipment onto battlefield with no creatures at all.
    let pro_id = the(&blocked, ZoneLocation::Battlefield, C3);
    let _ = blocked.state.move_object(
        pro_id,
        ZoneLocation::Exile(them()),
        ZonePosition::Top,
        Cause::Effect,
    );
    refresh_legal(&mut blocked);

    activate(&mut blocked, me(), eng2, 1);
    aim_object(&mut blocked, me(), eq2);
    settle(&mut blocked, me());
    let eq2_obj = blocked.state().object(eq2).unwrap();
    assert_eq!(
        eq2_obj.zone,
        Zone::Battlefield,
        "CR 301.5c: Equipment with no legal creature still enters the battlefield unattached"
    );
    assert_eq!(eq2_obj.attached_to, None);
}

// -----------------------------------------------------------------------------
// 2. Token created attached (Role Aura + CR 704.5y + CR 303.4g + Equipment token)
// -----------------------------------------------------------------------------

#[test]
fn create_token_attached_role_aura_and_equipment_enforce_cr_303_4g_and_704_5y() {
    static CREATE_MONSTER_ROLE: &[Effect] = &[Effect::CreateTokenAttached {
        token: &MONSTER_ROLE_TOKEN,
        amount: Amount::Fixed(1),
        host: AttachHost::Spec(TargetSpec::Object(&Filter::CREATURE)),
    }];
    static CREATE_WICKED_ROLE: &[Effect] = &[Effect::CreateTokenAttached {
        token: &WICKED_ROLE_TOKEN,
        amount: Amount::Fixed(1),
        host: AttachHost::Spec(TargetSpec::Object(&Filter::CREATURE)),
    }];
    static CREATE_EQUIP_TOKEN: &[Effect] = &[Effect::CreateTokenAttached {
        token: &SWORD_EQUIPMENT_TOKEN,
        amount: Amount::Fixed(1),
        host: AttachHost::Spec(TargetSpec::Object(&Filter::CREATURE)),
    }];
    static MAKER_AB: &[AbilityDef] = &[
        free(
            CREATE_MONSTER_ROLE,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            CREATE_WICKED_ROLE,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            CREATE_EQUIP_TOKEN,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
    ];

    let maker = card(
        C0,
        face("Role Smith", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MAKER_AB,
    );
    let bear = card(
        C1,
        creature_face("Grizzly Bears", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let pro_ench = card(
        C2,
        creature_face("Aegis Knight", "{2}", 2, 2),
        KeywordSet::EMPTY,
        PRO_ENCHANTMENT_ABILITIES,
    );

    let mut engine = bench(
        15_201,
        vec![maker, bear, pro_ench],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let maker_id = the(&engine, ZoneLocation::Battlefield, C0);
    let bear_id = the(&engine, ZoneLocation::Battlefield, C1);
    let pro_id = the(&engine, ZoneLocation::Battlefield, C2);

    // 1) Create Monster Role attached to Grizzly Bears -> 3/3 with trample.
    activate(&mut engine, me(), maker_id, 0);
    aim_object(&mut engine, me(), bear_id);
    settle(&mut engine, me());

    let roles: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.characteristics().subtypes.contains(enchantment::ROLE))
        })
        .collect();
    assert_eq!(roles.len(), 1);
    let monster_role_id = roles[0];
    assert_eq!(
        engine.state().object(monster_role_id).unwrap().attached_to,
        Some(bear_id)
    );
    let bear_chars = engine.state().object(bear_id).unwrap().characteristics();
    assert_eq!(bear_chars.power, Some(3));
    assert_eq!(bear_chars.toughness, Some(3));
    assert!(bear_chars.keywords.contains(KeywordSet::TRAMPLE));

    // 2) Create Wicked Role controlled by SAME player attached to SAME Grizzly Bears:
    // CR 704.5y puts the older Monster Role into graveyard (ceases to exist), leaving only Wicked!
    activate(&mut engine, me(), maker_id, 1);
    aim_object(&mut engine, me(), bear_id);
    settle(&mut engine, me());

    let roles_after: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.characteristics().subtypes.contains(enchantment::ROLE))
        })
        .collect();
    assert_eq!(
        roles_after.len(),
        1,
        "CR 704.5y: only the newest Role controlled by the same player survives"
    );
    assert_ne!(roles_after[0], monster_role_id);
    assert!(
        !engine
            .state()
            .object(bear_id)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::TRAMPLE),
        "older Monster Role's trample is gone after CR 704.5y"
    );

    // 3) Attempting to create a Monster Role attached to a creature with Protection from
    // Enchantments -> CR 303.4g: NO token is created!
    let bf_count_before = engine.state().battlefield_seen().count();
    activate(&mut engine, me(), maker_id, 0);
    aim_object(&mut engine, me(), pro_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().battlefield_seen().count(),
        bf_count_before,
        "CR 303.4g: Aura token with illegal host must not be created"
    );

    // 4) Create Equipment token attached to Grizzly Bears -> enters attached, grants +2/+0.
    activate(&mut engine, me(), maker_id, 2);
    aim_object(&mut engine, me(), bear_id);
    settle(&mut engine, me());

    let sword_tokens: Vec<ObjectId> = engine
        .state()
        .battlefield_seen()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.characteristics().subtypes.contains(artifact::EQUIPMENT))
        })
        .collect();
    assert_eq!(sword_tokens.len(), 1);
    assert_eq!(
        engine.state().object(sword_tokens[0]).unwrap().attached_to,
        Some(bear_id)
    );
    assert_eq!(
        engine
            .state()
            .object(bear_id)
            .unwrap()
            .characteristics()
            .power,
        Some(5), // 2 base + 1 Wicked Role + 2 Blade
    );
}

// -----------------------------------------------------------------------------
// 3. Alesha-style return tapped & attacking (CR 508.4, CR 506.3b, defender choice)
// -----------------------------------------------------------------------------

#[test]
fn alesha_return_tapped_and_attacking_selects_defender_and_never_counts_as_attacked() {
    static ALESHA_FX: &[Effect] = &[Effect::PutOntoBattlefieldAttacking {
        subject: DefinedObjects::Spec(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        )),
        owner_control: false,
        tapped: true,
    }];
    static ALESHA_AB: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::This),
        ALESHA_FX,
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        )))
    )];
    static ATTACK_WATCHER_AB: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::YOUR_CREATURE),
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }]
    )];
    static MAIN_REANIMATE_AB: &[AbilityDef] = &[free(
        ALESHA_FX,
        Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
    )];

    let alesha = card(
        C0,
        creature_face("Alesha Proxy", "{2}{R}", 3, 2),
        KeywordSet::HASTE,
        ALESHA_AB,
    );
    let watcher = card(
        C1,
        face("Attack Watcher", "{1}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        ATTACK_WATCHER_AB,
    );
    let reanimated = card(
        C2,
        creature_face("Goblin Raider", "{1}{R}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let walker = planeswalker_card(C3, "Jace Proxy", 4);
    let main_probe = card(
        C4,
        face("Main Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        MAIN_REANIMATE_AB,
    );

    let mut engine = bench(
        15_301,
        vec![alesha, watcher, reanimated, walker, main_probe],
        [Seat::with(&[C0, C1, C4]).holding(&[C2]), Seat::with(&[C3])],
    );
    to_main(&mut engine, me());
    let alesha_id = the(&engine, ZoneLocation::Battlefield, C0);
    let watcher_id = the(&engine, ZoneLocation::Battlefield, C1);
    let goblin_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let walker_id = the(&engine, ZoneLocation::Battlefield, C3);
    let probe_id = the(&engine, ZoneLocation::Battlefield, C4);

    let _ = engine.state.move_object(
        goblin_id,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    settle(&mut engine, me());

    // Advance to Declare Attackers.
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == me()),
    );

    // Declare ONLY Alesha attacking opponent directly.
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![(alesha_id, Defender::Player(them()))],
            },
        )
        .unwrap();

    // Order simultaneous attack triggers if asked, then pick target for Alesha's trigger.
    loop {
        match engine.pending().clone() {
            Pending::Arrange { player, cards, .. } => {
                engine
                    .apply(player, PlayerAction::Arrange { piles: vec![cards] })
                    .unwrap();
            }
            Pending::ChooseTargets { player, .. } => {
                aim_object(&mut engine, player, goblin_id);
                break;
            }
            other => panic!("unexpected pending before Alesha targets: {other:?}"),
        }
    }

    // Resolve triggers until Alesha's effect asks which planeswalker (if any) Goblin attacks!
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == me()),
    );
    match engine.pending() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(*min, 0);
            assert_eq!(*max, 1);
            assert_eq!(options, &vec![walker_id]);
        }
        other => panic!("expected defender planeswalker choice, got {other:?}"),
    }
    // Choose the opponent's planeswalker!
    engine.apply(me(), choose_objects(vec![walker_id])).unwrap();
    settle(&mut engine, me());

    let goblin_obj = engine.state().object(goblin_id).unwrap();
    assert_eq!(goblin_obj.zone, Zone::Battlefield);
    assert!(
        goblin_obj.status.contains(Status::TAPPED),
        "Goblin entered tapped"
    );
    assert!(
        engine.state().combat.is_attacking(goblin_id),
        "Goblin entered attacking"
    );
    let goblin_atk = engine
        .state()
        .combat
        .attackers()
        .iter()
        .find(|a| a.creature == goblin_id)
        .expect("Goblin is in combat.attackers()");
    assert_eq!(goblin_atk.defending, Defender::Planeswalker(walker_id));

    // CR 508.4 / CR 508.3a: Goblin NEVER declared an attack — must NOT be in per_turn.attacked,
    // and Attack Watcher must have triggered ONLY ONCE (for Alesha, never for Goblin)!
    assert!(
        engine
            .state()
            .per_turn
            .attacked
            .iter()
            .any(|&(id, _, _)| id == alesha_id),
        "Alesha declared an attack"
    );
    assert!(
        !engine
            .state()
            .per_turn
            .attacked
            .iter()
            .any(|&(id, _, _)| id == goblin_id),
        "CR 508.4: creature put onto battlefield attacking must NEVER count as having attacked"
    );
    assert_eq!(
        triggered(&engine, watcher_id),
        1,
        "CR 508.3a: 'whenever a creature you control attacks' must NOT trigger for entering attacking"
    );

    // Outside combat (next main phase): PutOntoBattlefieldAttacking enters tapped, NOT attacking (CR 506.3b).
    let _ = engine.state.move_object(
        goblin_id,
        ZoneLocation::Graveyard(me()),
        ZonePosition::Top,
        Cause::Effect,
    );
    to_main(&mut engine, me());
    activate(&mut engine, me(), probe_id, 0);
    aim_object(&mut engine, me(), goblin_id);
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(goblin_id).unwrap().zone,
        Zone::Battlefield
    );
    assert!(
        engine
            .state()
            .object(goblin_id)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !engine.state().combat.is_attacking(goblin_id),
        "CR 506.3b: outside combat, enters tapped and NOT attacking"
    );
}

// -----------------------------------------------------------------------------
// 4. Deterministic random card choice (reproducible under identical seed)
// -----------------------------------------------------------------------------

#[test]
fn deterministic_random_card_choice_reproducible_under_same_seed() {
    static RANDOM_EXILE_TWO: &[Effect] = &[Effect::ChooseCardConstrained {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::You),
        filter: &Filter::Any,
        min: 2,
        max: Amount::Fixed(2),
        at_random: true,
        distinct_names: false,
        reveal_chosen: false,
        movement: ChooseMove::Exile,
        then: &[],
    }];
    static RANDOM_DISTINCT_TWO: &[Effect] = &[Effect::ChooseCardConstrained {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::You),
        filter: &Filter::Any,
        min: 2,
        max: Amount::Fixed(2),
        at_random: true,
        distinct_names: true,
        reveal_chosen: false,
        movement: ChooseMove::Exile,
        then: &[],
    }];
    static PROBE_AB: &[AbilityDef] = &[
        free(RANDOM_EXILE_TWO, None),
        free(RANDOM_DISTINCT_TWO, None),
    ];

    let probe = card(
        C0,
        face("Random Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_AB,
    );
    let c_a = card(
        C1,
        creature_face("Card A", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let c_b = card(
        C2,
        creature_face("Card B", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let c_c = card(
        C3,
        creature_face("Card C", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let c_d = card(
        C4,
        creature_face("Card D", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let c_e = card(
        C5,
        creature_face("Card E", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );

    let run_once = |seed: u64, ab_index: u32, hand: &[u32]| {
        let mut e = bench(
            seed,
            vec![probe, c_a, c_b, c_c, c_d, c_e],
            [Seat::with(&[C0]).holding(hand), Seat::default()],
        );
        to_main(&mut e, me());
        let p = the(&e, ZoneLocation::Battlefield, C0);
        activate(&mut e, me(), p, ab_index);
        settle(&mut e, me());
        let exiled_names: Vec<String> = e
            .state()
            .zones
            .list(ZoneLocation::Exile(me()))
            .iter()
            .map(|&id| {
                e.state()
                    .names
                    .get(e.state().object(id).unwrap().characteristics().name)
                    .to_owned()
            })
            .collect();
        (exiled_names, e.state().snapshot_hash())
    };

    // Identical seed -> bit-for-bit identical random selection and state hash!
    let (names1, hash1) = run_once(15_401, 0, &[C1, C2, C3, C4, C5]);
    let (names2, hash2) = run_once(15_401, 0, &[C1, C2, C3, C4, C5]);
    assert_eq!(names1.len(), 2);
    assert_eq!(
        names1, names2,
        "same seed must yield identical random picks"
    );
    assert_eq!(hash1, hash2, "same seed must yield identical snapshot_hash");

    // Distinct-names + at_random over duplicate hand [A, A, A, B, C]:
    // across 20 seeds, every single run must exile 2 cards with strictly distinct names,
    // and different seeds must vary (using state.rng, not a fixed prefix).
    let mut distinct_outcomes = std::collections::BTreeSet::new();
    for seed in 15_410..15_430 {
        let (picked, _) = run_once(seed, 1, &[C1, C1, C1, C2, C3]);
        assert_eq!(picked.len(), 2);
        assert_ne!(
            picked[0], picked[1],
            "seed {seed}: distinct_names + at_random must never choose two copies with the same name"
        );
        distinct_outcomes.insert(picked);
    }
    assert!(
        distinct_outcomes.len() >= 2,
        "state.rng should vary random selections across seeds: {distinct_outcomes:?}"
    );
}

// -----------------------------------------------------------------------------
// 5. Distinct-names enforcement on library search & choose-up-to-N from zone
// -----------------------------------------------------------------------------

#[test]
fn distinct_names_search_and_choose_up_to_n_enforce_unique_names() {
    static SEARCH_DISTINCT: &[Effect] = &[Effect::SearchLibraryDistinctNames {
        filter: &Filter::CREATURE,
        count: Amount::Fixed(3),
        find: &Find::HAND,
    }];
    static CHOOSE_GY_DISTINCT: &[Effect] = &[Effect::ChooseCardConstrained {
        chooser: PlayerRel::You,
        zone: ChooseZone::Graveyard(PlayerRel::You),
        filter: &Filter::CREATURE,
        min: 1,
        max: Amount::Fixed(3),
        at_random: false,
        distinct_names: true,
        reveal_chosen: false,
        movement: ChooseMove::Hand,
        then: &[],
    }];
    static PROBE_AB: &[AbilityDef] = &[free(SEARCH_DISTINCT, None), free(CHOOSE_GY_DISTINCT, None)];

    let probe = card(
        C0,
        face("Tutor Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_AB,
    );
    let alpha = card(
        C1,
        creature_face("Alpha", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let beta = card(
        C2,
        creature_face("Beta", "{2}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let gamma = card(
        C3,
        creature_face("Gamma", "{3}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    // Part A: SearchLibraryDistinctNames over library containing 2x Alpha, 2x Beta, 1x Gamma.
    let mut deck = vec![C1, C1, C2, C2, C3];
    deck.resize(60, super::synthetic::forest());
    let mut engine = bench_with_deck(15_501, vec![probe, alpha, beta, gamma], deck, &[C0], &[]);
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);

    activate(&mut engine, me(), probe_id, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();

    let (search_options, min, max) = match engine.pending().clone() {
        Pending::ChooseCards {
            options, min, max, ..
        } => (options, min, max),
        other => panic!("expected SearchLibraryDistinctNames prompt, got {other:?}"),
    };
    assert_eq!(min, 0);
    assert_eq!(max, 3);
    // Only 3 distinct names exist among the 5 creature cards in the library!
    assert_eq!(
        search_options.len(),
        3,
        "distinct_names search must offer only one option per distinct card name"
    );
    let offered_names: std::collections::BTreeSet<&str> = search_options
        .iter()
        .map(|&id| {
            engine
                .state()
                .names
                .get(engine.state().object(id).unwrap().characteristics().name)
        })
        .collect();
    assert_eq!(
        offered_names,
        std::collections::BTreeSet::from(["Alpha", "Beta", "Gamma"])
    );
    engine
        .apply(
            me(),
            choose_objects(vec![search_options[0], search_options[1]]),
        )
        .unwrap();
    settle(&mut engine, me());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&search_options[0])
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(me()))
            .contains(&search_options[1])
    );

    // Part B: ChooseCardConstrained { distinct_names: true, min: 1, max: 3 } over a graveyard
    // containing 2x Alpha, 2x Beta, 1x Gamma — steps one-by-one so player can pick specific
    // copy Alpha #2 and all Alphas are then excluded on step 2, then player can stop early!
    let mut gy_bench = bench(
        15_502,
        vec![probe, alpha, beta, gamma],
        [
            Seat::with(&[C0]).holding(&[C1, C1, C2, C2, C3]),
            Seat::default(),
        ],
    );
    to_main(&mut gy_bench, me());
    let p2 = the(&gy_bench, ZoneLocation::Battlefield, C0);
    let hand_ids: Vec<ObjectId> = gy_bench
        .state()
        .zones
        .list(ZoneLocation::Hand(me()))
        .clone();
    for id in hand_ids {
        let _ = gy_bench.state.move_object(
            id,
            ZoneLocation::Graveyard(me()),
            ZonePosition::Top,
            Cause::Effect,
        );
    }
    settle(&mut gy_bench, me());
    let alphas = objects(&gy_bench, ZoneLocation::Graveyard(me()), C1);
    let betas = objects(&gy_bench, ZoneLocation::Graveyard(me()), C2);
    assert_eq!(alphas.len(), 2);
    assert_eq!(betas.len(), 2);

    activate(&mut gy_bench, me(), p2, 1);
    gy_bench.apply(me(), PlayerAction::PassPriority).unwrap();
    gy_bench.apply(them(), PlayerAction::PassPriority).unwrap();

    // Step 1: all 5 copies offered, min=1, max=1. Pick specifically alphas[1]!
    match gy_bench.pending() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(options.len(), 5);
            assert_eq!(*min, 1);
            assert_eq!(*max, 1);
        }
        other => panic!("expected step 1 distinct-name choice, got {other:?}"),
    }
    gy_bench
        .apply(me(), choose_objects(vec![alphas[1]]))
        .unwrap();

    // Step 2: NEITHER Alpha may appear in options! Only 2x Beta + 1x Gamma remain, and min=0!
    match gy_bench.pending() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(
                *min, 0,
                "minimum 1 already satisfied; player may stop early"
            );
            assert_eq!(*max, 1);
            assert_eq!(options.len(), 3);
            assert!(!options.contains(&alphas[0]));
            assert!(!options.contains(&alphas[1]));
            assert!(options.contains(&betas[0]));
        }
        other => panic!("expected step 2 distinct-name choice, got {other:?}"),
    }
    gy_bench
        .apply(me(), choose_objects(vec![betas[0]]))
        .unwrap();

    // Step 3: only Gamma remains with min=0 -> stop early with empty vec!
    match gy_bench.pending() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(*min, 0);
            assert_eq!(*max, 1);
            assert_eq!(options.len(), 1);
        }
        other => panic!("expected step 3 distinct-name choice, got {other:?}"),
    }
    gy_bench.apply(me(), choose_objects(vec![])).unwrap();
    settle(&mut gy_bench, me());

    assert_eq!(gy_bench.state().object(alphas[1]).unwrap().zone, Zone::Hand);
    assert_eq!(gy_bench.state().object(betas[0]).unwrap().zone, Zone::Hand);
    assert_eq!(
        gy_bench.state().object(alphas[0]).unwrap().zone,
        Zone::Graveyard
    );
}

// -----------------------------------------------------------------------------
// 6. Optional & mandatory AttachChosen + AttachSlot { optional: true }
// -----------------------------------------------------------------------------

#[test]
fn optional_attach_chosen_and_attach_slot_accept_and_decline() {
    static OPT_ATTACH_CHOSEN: &[Effect] = &[Effect::AttachChosen {
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::Object(&Filter::CREATURE),
        max: Amount::Fixed(1),
        optional: true,
    }];
    static MANDATORY_ATTACH_CHOSEN: &[Effect] = &[Effect::AttachChosen {
        filter: &Filter::HasSubtype(artifact::EQUIPMENT),
        to: TargetSpec::Object(&Filter::CREATURE),
        max: Amount::Fixed(1),
        optional: false,
    }];
    static OPT_ATTACH_SLOT: &[Effect] = &[Effect::AttachSlot {
        object: TargetSlot::First,
        to: TargetSlot::Second,
        optional: true,
    }];
    static PROBE_AB: &[AbilityDef] = &[
        free(
            OPT_ATTACH_CHOSEN,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        free(
            MANDATORY_ATTACH_CHOSEN,
            Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        AbilityDef::Activated {
            cost: Cost::FREE,
            effects: OPT_ATTACH_SLOT,
            targets: Some(TargetReq::one(TargetSpec::Object(&Filter::HasSubtype(
                artifact::EQUIPMENT,
            )))),
            second_targets: Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            third_targets: None,
            timing: ActivationTiming::InstantSpeed,
            mana_ability: false,
            zone: ActivationZone::Battlefield,
            limit: ActivationLimit::Unlimited,
            x_value: baylee_cards_dsl::ActivationX::Any,
            cost_reduction: None,
        },
    ];

    let probe = card(
        C0,
        face("Armory Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_AB,
    );
    let sword = equipment_card(C1, "Steel Blade");
    let knight = card(
        C2,
        creature_face("Silver Knight", "{2}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15_601,
        vec![probe, sword, knight],
        [Seat::with(&[C0, C1, C2]), Seat::default()],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);
    let sword_id = the(&engine, ZoneLocation::Battlefield, C1);
    let knight_id = the(&engine, ZoneLocation::Battlefield, C2);

    // 1) Optional AttachChosen: decline (empty vec) leaves sword unattached.
    activate(&mut engine, me(), probe_id, 0);
    aim_object(&mut engine, me(), knight_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    match engine.pending() {
        Pending::ChooseCards {
            options, min, max, ..
        } => {
            assert_eq!(*min, 0);
            assert_eq!(*max, 1);
            assert_eq!(options, &vec![sword_id]);
        }
        other => panic!("expected optional AttachChosen prompt, got {other:?}"),
    }
    engine.apply(me(), choose_objects(vec![])).unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(sword_id).unwrap().attached_to, None);

    // 2) Mandatory AttachChosen: min=1 rejects empty answer, accepting attaches sword (+3/+0).
    activate(&mut engine, me(), probe_id, 1);
    aim_object(&mut engine, me(), knight_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(engine.apply(me(), choose_objects(vec![])).is_err());
    engine.apply(me(), choose_objects(vec![sword_id])).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_id).unwrap().attached_to,
        Some(knight_id)
    );
    assert_eq!(
        engine
            .state()
            .object(knight_id)
            .unwrap()
            .characteristics()
            .power,
        Some(5)
    );

    // 3) Detach sword, test AttachSlot { optional: true } decline then accept.
    engine.state.object_mut(sword_id).unwrap().attached_to = None;
    engine.state.invalidate_projections();
    engine.state.refresh_characteristics();

    activate(&mut engine, me(), probe_id, 2);
    aim_object(&mut engine, me(), sword_id);
    aim_object(&mut engine, me(), knight_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    engine.apply(me(), PlayerAction::YesNo(false)).unwrap();
    settle(&mut engine, me());
    assert_eq!(engine.state().object(sword_id).unwrap().attached_to, None);

    activate(&mut engine, me(), probe_id, 2);
    aim_object(&mut engine, me(), sword_id);
    aim_object(&mut engine, me(), knight_id);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine.apply(me(), PlayerAction::YesNo(true)).unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(sword_id).unwrap().attached_to,
        Some(knight_id)
    );
}

// -----------------------------------------------------------------------------
// 7. DiscardDefined for found/remembered cards (CR 701.9a)
// -----------------------------------------------------------------------------

#[test]
fn discard_defined_discards_exact_found_and_remembered_cards_and_skips_stale() {
    static CHOOSE_AND_DISCARD_FOUND: &[Effect] = &[Effect::ChooseCardConstrained {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::Opponent),
        filter: &Filter::Any,
        min: 1,
        max: Amount::Fixed(1),
        at_random: true,
        distinct_names: false,
        reveal_chosen: true,
        movement: ChooseMove::None,
        then: &[Effect::DiscardDefined {
            cards: DefinedObjects::Found,
        }],
    }];
    static EXILE_THEN_STALE_DISCARD: &[Effect] = &[Effect::ChooseCardConstrained {
        chooser: PlayerRel::You,
        zone: ChooseZone::Hand(PlayerRel::Opponent),
        filter: &Filter::Any,
        min: 1,
        max: Amount::Fixed(1),
        at_random: false,
        distinct_names: false,
        reveal_chosen: false,
        movement: ChooseMove::Exile,
        then: &[Effect::DiscardDefined {
            cards: DefinedObjects::Found,
        }],
    }];
    static PROBE_AB: &[AbilityDef] = &[
        free(CHOOSE_AND_DISCARD_FOUND, None),
        free(EXILE_THEN_STALE_DISCARD, None),
    ];

    let probe = card(
        C0,
        face("Thought Probe", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        PROBE_AB,
    );
    let c_a = card(
        C1,
        creature_face("Keep A", "{1}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let c_b = card(
        C2,
        creature_face("Keep B", "{2}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let c_c = card(
        C3,
        creature_face("Keep C", "{3}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        15_701,
        vec![probe, c_a, c_b, c_c],
        [Seat::with(&[C0]), Seat::default().holding(&[C1, C2, C3])],
    );
    to_main(&mut engine, me());
    let probe_id = the(&engine, ZoneLocation::Battlefield, C0);

    let journal_before = engine.journal().entries().len();
    activate(&mut engine, me(), probe_id, 0);
    settle(&mut engine, me());

    // Exactly 1 revealed card in opponent's hand was discarded to opponent's graveyard.
    let opp_gy = engine.state().zones.list(ZoneLocation::Graveyard(them()));
    let opp_hand = engine.state().zones.list(ZoneLocation::Hand(them()));
    assert_eq!(opp_gy.len(), 1);
    assert_eq!(opp_hand.len(), 2);
    let discarded_card = opp_gy[0];
    let revealed_same = engine.journal().entries()[journal_before..]
        .iter()
        .any(|e| matches!(&e.event, GameEvent::Revealed { cards, .. } if cards.contains(&discarded_card)));
    let discarded_event = engine.journal().entries()[journal_before..]
        .iter()
        .any(|e| {
            matches!(
                e.event,
                GameEvent::Discarded { object, player, .. }
                    if object == discarded_card && player == them()
            )
        });
    assert!(revealed_same, "the exact found/revealed card was discarded");
    assert!(discarded_event, "Discarded event emitted for opponent");

    // If the found card already moved to Exile before DiscardDefined runs, it must NOT
    // emit a spurious Discarded event (CR 701.9a — only cards in hand can be discarded).
    let journal_before_stale = engine.journal().entries().len();
    let remaining_card = opp_hand[0];
    activate(&mut engine, me(), probe_id, 1);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    engine
        .apply(me(), choose_objects(vec![remaining_card]))
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(
        engine.state().object(remaining_card).unwrap().zone,
        Zone::Exile
    );
    let spurious_discard = engine.journal().entries()[journal_before_stale..]
        .iter()
        .any(|e| matches!(e.event, GameEvent::Discarded { .. }));
    assert!(
        !spurious_discard,
        "DiscardDefined must skip cards not in a hand"
    );
}
