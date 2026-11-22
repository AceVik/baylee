//! Beat 18 / Engine Wave 14a deterministic rule tests:
//! 1. Zone-aware normal-cost static play permissions (`AltCastPrice::Normal`,
//!    `AltCastPrice::NormalPlus(Cost)`) on `Modifier::MayCastWithAltCost` across
//!    `TopOfLibrary` (*Elven Chorus*, *Elsha of the Infinite*) and
//!    `Zones(&[ZoneRef::Graveyard])` (*Lurrus*, *Demonic Embrace*), plus private
//!    top-of-library visibility (`Modifier::MayLookAtTopOfLibrary { who }`) with
//!    zero hidden-info leaks in `library_top_visible_to` and simulation `fork_for`,
//!    and filtered `Modifier::PlayLandsFromGraveyard`.
//! 2. Generalized Flash grants (`Modifier::GrantFlash { who, filter }`) across
//!    historic spells (*Raff Capashen*), green creatures + Prototype/Disguise forms
//!    (*Yeva*), `EachPlayer` grants, and `CR 101.2` `OpponentsCastAsSorcery` override.
//! 3. Dynamic filter comparisons (`Filter::{CmcAtMostAmount, CmcEqualsAmount,
//!    PowerAtMostAmount, PowerAtLeastAmount}`) with LKI `Amount::SourcePower`
//!    (*Alesha*, *Arcane Proxy*), `Stack -> Battlefield` `Amount::ManaSpentToCast`
//!    (*Astelli Reclaimer*, *Bortuk Bonerattle*), resolution `Amount::EventAmount`
//!    (*Buster Sword*), `Amount::{GreatestPowerAmong, GreatestManaValueAmong}`,
//!    and cycle-safe re-entrancy depth capping.
#![allow(clippy::too_many_lines)]

use super::*;
use crate::choice::CastModeKind;
use crate::event::Cause;
use crate::state::CardLookup;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_cards_dsl::{
    AltCastPrice, AltCastZones, Amount, ChooseMove, ChooseZone, Duration, Filter, Layer, Modifier,
    MoveSubject, PlayerRel, Prototype, StaticAbility, SupertypeSet, TargetReq, TargetSpec, Trigger,
    ZoneRef, ZoneSel, cost, spell, static_ability, triggered,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land;
use baylee_core::mana::ManaColor;

const C0: u32 = 99_940;
const C1: u32 = 99_941;
const C2: u32 = 99_942;
const C3: u32 = 99_943;
const C4: u32 = 99_944;
const C5: u32 = 99_945;
const C6: u32 = 99_946;
const C7: u32 = 99_947;
const C8: u32 = 99_948;
const C9: u32 = 99_949;

#[derive(Clone, Copy)]
struct CloneLookup(&'static [&'static CardDef]);

impl CardLookup for CloneLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        self.0
            .iter()
            .find(|def| def.index == index)
            .copied()
            .or_else(|| baylee_cards::by_index(index))
    }
}

fn refresh_offer(engine: &mut Bench) {
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.state.refresh_characteristics();
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
    }
}

fn add_pool(engine: &mut Bench, player: PlayerId, color: ManaColor, amount: u32) {
    engine.state.players[usize::from(player.get())]
        .mana_pool
        .add(color, amount);
    refresh_offer(engine);
}

fn move_to_zone(engine: &mut Bench, id: ObjectId, loc: ZoneLocation) {
    engine
        .state
        .move_object(id, loc, ZonePosition::Top, Cause::Effect)
        .expect("move test object");
    refresh_offer(engine);
}

fn resolve_top(engine: &mut Bench) {
    let (first, _) = offer(engine);
    engine
        .apply(first, PlayerAction::PassPriority)
        .expect("first player passes priority");
    let (second, _) = offer(engine);
    engine
        .apply(second, PlayerAction::PassPriority)
        .expect("second player passes priority");
}

fn to_second_main(engine: &mut Bench, seat: PlayerId) {
    walk_until(engine, |e| {
        holds_priority_in(e, seat, crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
}

// =========================================================================
// 1. TopOfLibrary Normal-Cost Permissions & Private MayLookAtTopOfLibrary
// =========================================================================

#[test]
fn top_of_library_normal_cost_casts_creatures_x_spells_lands_and_preserves_private_top() {
    static CHORUS_ABILITIES: &[AbilityDef] = &[
        static_ability!(
            Filter::Any,
            Modifier::MayLookAtTopOfLibrary {
                who: PlayerRel::You,
            }
        ),
        static_ability!(
            Filter::CREATURE,
            Modifier::MayCastWithAltCost {
                who: PlayerRel::You,
                zones: AltCastZones::TopOfLibrary,
                grant_zone_permission: true,
                allow_lands: false,
                with_flash: false,
                max_mana_value: None,
                once_per_turn: false,
                price: AltCastPrice::Normal,
            }
        ),
    ];
    static ELSHA_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::NONCREATURE,
        Modifier::MayCastWithAltCost {
            who: PlayerRel::You,
            zones: AltCastZones::TopOfLibrary,
            grant_zone_permission: true,
            allow_lands: true,
            with_flash: true,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::Normal,
        }
    )];
    static X_BURN_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::DealDamage {
        target: TargetSpec::Player(PlayerRel::EachOpponent),
        amount: Amount::X,
    }])];
    static COSTLESS_ABILITIES: &[AbilityDef] = &[spell!(&[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }])];

    let chorus = card(
        C0,
        face("Chorus Probe", "{3}{G}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CHORUS_ABILITIES,
    );
    let bear = card(
        C1,
        creature_face("Top Bear", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let divination = card(
        C2,
        face("Top Sorcery", "{2}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        COSTLESS_ABILITIES,
    );
    let elsha = card(
        C3,
        creature_face("Elsha Probe", "{2}{U}{R}{W}", 3, 3),
        KeywordSet::EMPTY,
        ELSHA_ABILITIES,
    );
    let x_blaze = card(
        C4,
        face("Top Blaze", "{X}{R}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        X_BURN_ABILITIES,
    );
    let mut costless_face = face("Top Vision", "{0}", TypeSet::SORCERY);
    costless_face.mana_cost = ManaCost::ZERO;
    let costless = card(C5, costless_face, KeywordSet::EMPTY, COSTLESS_ABILITIES);
    let top_land = card(C6, land_face("Top Wastes"), KeywordSet::EMPTY, &[]);

    let mut engine = bench(
        1401,
        vec![chorus, bear, divination, elsha, x_blaze, costless, top_land],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C3, C4, C5, C6]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let sorc_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let elsha_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let blaze_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let costless_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let land_id = the(&engine, ZoneLocation::Hand(me()), C6);

    // Put Top Bear on top of me()'s library.
    move_to_zone(&mut engine, bear_id, ZoneLocation::Library(me()));

    // 1. Private top-of-library visibility: me() sees own top, them() does NOT,
    //    and library_top_revealed remains false (no public reveal).
    assert!(crate::casting::library_top_visible_to(
        &engine.state,
        me(),
        me()
    ));
    assert!(!crate::casting::library_top_visible_to(
        &engine.state,
        them(),
        me()
    ));
    assert!(!engine.state.library_top_revealed(me()));

    // Also verify simulation `fork_for` preserves private top for `me()` and
    // redeals it for `them()`!
    let leaked_defs: &'static [&'static CardDef] = Box::leak(
        vec![chorus, bear, divination, elsha, x_blaze, costless, top_land].into_boxed_slice(),
    );
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card: CardIndex::new(forest()),
            print: PrintRef::new(0),
        })
        .collect();
    let fork_seat = |bf: &[u32], hand: &[u32]| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities {
            dev_commands: true,
            see_hidden: false,
        },
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(
            hand.iter()
                .map(|&idx| DeckEntry {
                    rule_print: None,
                    choices: baylee_core::card_metadata::DeckChoices::default(),
                    card: CardIndex::new(idx),
                    print: PrintRef::new(0),
                })
                .collect(),
        ),
        starting_battlefield: bf
            .iter()
            .map(|&idx| DeckEntry {
                rule_print: None,
                choices: baylee_core::card_metadata::DeckChoices::default(),
                card: CardIndex::new(idx),
                print: PrintRef::new(0),
            })
            .collect(),
        emblems: vec![],
        team: None,
    };
    let fork_preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 14010,
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
        seats: vec![fork_seat(&[C0], &[]), fork_seat(&[], &[])],
    };
    let mut fork_engine =
        Engine::new(&fork_preset, CloneLookup(leaked_defs)).expect("cloneable engine");
    while let Pending::Mulligan { player, .. } = fork_engine.pending().clone() {
        fork_engine
            .apply(player, PlayerAction::MulliganKeep)
            .expect("keep");
    }
    let fork_bear = *fork_engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .expect("library top exists");

    let fork_me = fork_engine
        .fork_for(me(), crate::engine::simulation::DeckKnowledge::Known, 77)
        .expect("fork_for(me) succeeds with private library top");
    assert_eq!(
        fork_me
            .state()
            .zones
            .list(ZoneLocation::Library(me()))
            .last(),
        Some(&fork_bear),
        "me()'s own simulation fork preserves the privately visible top card"
    );
    let fork_them = fork_engine
        .fork_for(them(), crate::engine::simulation::DeckKnowledge::Known, 77)
        .expect("fork_for(them) succeeds without leaking me()'s library top");
    assert_ne!(
        fork_them
            .state()
            .zones
            .list(ZoneLocation::Library(me()))
            .last(),
        Some(&fork_bear),
        "opponent's simulation fork redeals me()'s unrevealed top card"
    );

    // 2. Top Bear is castable for its normal {1}{G} cost with alt_cast == false;
    //    Top Sorcery on top of library is NOT castable under Chorus alone.
    move_to_zone(&mut engine, sorc_id, ZoneLocation::Library(me()));
    add_pool(&mut engine, me(), ManaColor::Blue, 3);
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    let (_, legal_sorc) = offer(&engine);
    assert!(
        !legal_sorc.castable.contains(&sorc_id),
        "Chorus only permits creature spells from top of library"
    );
    move_to_zone(&mut engine, sorc_id, ZoneLocation::Graveyard(me()));
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Library(me())).last(),
        Some(&bear_id)
    );
    let (_, legal_bear) = offer(&engine);
    assert!(legal_bear.castable.contains(&bear_id));

    engine
        .apply(me(), PlayerAction::CastSpell { card: bear_id })
        .expect("cast Top Bear from top of library");
    let on_stack = engine.state.object(bear_id).expect("bear on stack");
    assert_eq!(on_stack.zone, Zone::Stack);
    assert!(
        !on_stack.alt_cast,
        "AltCastPrice::Normal is not an alternative cost (CR 118.9)"
    );
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(bear_id).unwrap().zone,
        Zone::Battlefield
    );

    // 3. Put Elsha Probe onto battlefield:
    //    - Costless card on top of library is refused (CR 118.6);
    //    - Land on top of library is playable via allow_lands: true;
    //    - {X}{R} sorcery on top of library is castable on opponent's turn
    //      (with_flash: true) and prompts for X!
    move_to_zone(&mut engine, elsha_id, ZoneLocation::Battlefield);
    move_to_zone(&mut engine, costless_id, ZoneLocation::Library(me()));
    let (_, legal_costless) = offer(&engine);
    assert!(
        !legal_costless.castable.contains(&costless_id),
        "unpayable costless spell cannot be cast via AltCastPrice::Normal (CR 118.6)"
    );
    move_to_zone(&mut engine, costless_id, ZoneLocation::Graveyard(me()));

    move_to_zone(&mut engine, land_id, ZoneLocation::Library(me()));
    let (_, legal_land) = offer(&engine);
    assert!(
        legal_land.lands.contains(&land_id),
        "allow_lands: true opens top-of-library land drop"
    );
    engine
        .apply(me(), PlayerAction::PlayLand { card: land_id })
        .expect("play land from top of library");
    assert_eq!(
        engine.state.object(land_id).unwrap().zone,
        Zone::Battlefield
    );

    // Advance to opponent's main phase and cast {X}{R} from top of library for X = 3!
    to_main(&mut engine, them());
    move_to_zone(&mut engine, blaze_id, ZoneLocation::Library(me()));
    add_pool(&mut engine, me(), ManaColor::Red, 4);
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("opponent passes priority to me()");
    let (who, legal_flash) = offer(&engine);
    assert_eq!(who, me());
    assert!(
        legal_flash.castable.contains(&blaze_id),
        "Elsha with_flash opens sorcery on top of library during opponent's turn"
    );
    let opp_life_before = life(&engine, them());
    engine
        .apply(me(), PlayerAction::CastSpell { card: blaze_id })
        .expect("begin casting {X}{R} from top of library");
    assert!(
        matches!(engine.pending(), Pending::ChooseNumber { max: 3, .. }),
        "AltCastPrice::Normal preserves variable X selection (expected max 3, got {:?})",
        engine.pending()
    );
    engine
        .apply(me(), PlayerAction::ChooseNumber(3))
        .expect("choose X = 3");
    assert!(!engine.state.object(blaze_id).unwrap().alt_cast);
    resolve_top(&mut engine);
    assert_eq!(
        life(&engine, them()),
        opp_life_before - 3,
        "Blaze resolved for X = 3 off the top of the library"
    );
}

// =========================================================================
// 2. Graveyard Normal / NormalPlus & Filtered PlayLandsFromGraveyard
// =========================================================================

#[test]
fn graveyard_normal_and_normal_plus_permissions_and_filtered_graveyard_lands() {
    static GY_ZONES: &[ZoneRef] = &[ZoneRef::Graveyard];
    static LURRUS_FILTER: Filter = Filter::And(&[
        Filter::Or(&[
            Filter::CREATURE,
            Filter::ARTIFACT,
            Filter::ENCHANTMENT,
            Filter::LAND,
        ]),
        Filter::OwnedByYou,
    ]);
    static LURRUS_ABILITIES: &[AbilityDef] = &[
        static_ability!(Filter::Any, Modifier::ExtraLandDrops(1)),
        static_ability!(
            LURRUS_FILTER,
            Modifier::MayCastWithAltCost {
                who: PlayerRel::You,
                zones: AltCastZones::Zones(GY_ZONES),
                grant_zone_permission: true,
                allow_lands: true,
                with_flash: false,
                max_mana_value: Some(Amount::Fixed(2)),
                once_per_turn: true,
                price: AltCastPrice::Normal,
            }
        ),
    ];
    static EMBRACE_EXTRA: Cost = cost!(PayLife(3), Discard(&Filter::Any));
    static EMBRACE_ABILITIES: &[AbilityDef] = &[AbilityDef::Static(StaticAbility {
        source_zone: ZoneRef::Graveyard,
        layer: Layer::Text,
        filter: Filter::This,
        modifier: Modifier::MayCastWithAltCost {
            who: PlayerRel::You,
            zones: AltCastZones::Zones(GY_ZONES),
            grant_zone_permission: true,
            allow_lands: false,
            with_flash: false,
            max_mana_value: None,
            once_per_turn: false,
            price: AltCastPrice::NormalPlus(EMBRACE_EXTRA),
        },
        condition: None,
    })];
    static DESERT_CRUCIBLE_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::HasSubtype(land::DESERT),
        Modifier::PlayLandsFromGraveyard
    )];

    let lurrus = card(
        C0,
        creature_face("Lurrus Probe", "{1}{W}{B}", 3, 2),
        KeywordSet::EMPTY,
        LURRUS_ABILITIES,
    );
    let bear_a = card(
        C1,
        creature_face("Grave Bear A", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let bear_b = card(
        C2,
        creature_face("Grave Bear B", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let embrace = card(
        C3,
        creature_face("Demonic Shade", "{1}{B}{B}", 3, 1),
        KeywordSet::EMPTY,
        EMBRACE_ABILITIES,
    );
    let fodder = card(
        C4,
        creature_face("Hand Fodder", "{G}", 1, 1),
        KeywordSet::EMPTY,
        &[],
    );
    let desert_crucible = card(
        C5,
        face("Desert Crucible", "{2}{G}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        DESERT_CRUCIBLE_ABILITIES,
    );
    let mut desert_face = land_face("Sunscorched Probe");
    desert_face.subtypes = &[land::DESERT];
    let desert_land = card(C6, desert_face, KeywordSet::EMPTY, &[]);
    let plain_land = card(C7, land_face("Plain Wastes"), KeywordSet::EMPTY, &[]);

    let mut engine = bench(
        1402,
        vec![
            lurrus,
            bear_a,
            bear_b,
            embrace,
            fodder,
            desert_crucible,
            desert_land,
            plain_land,
        ],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C3, C4, C5, C6, C7]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let lurrus_id = the(&engine, ZoneLocation::Battlefield, C0);
    let bear_a_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let bear_b_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let embrace_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let fodder_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let crucible_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let desert_id = the(&engine, ZoneLocation::Hand(me()), C6);
    let plain_id = the(&engine, ZoneLocation::Hand(me()), C7);

    move_to_zone(&mut engine, bear_a_id, ZoneLocation::Graveyard(me()));
    move_to_zone(&mut engine, bear_b_id, ZoneLocation::Graveyard(me()));
    move_to_zone(&mut engine, desert_id, ZoneLocation::Graveyard(me()));
    move_to_zone(&mut engine, plain_id, ZoneLocation::Graveyard(me()));

    // 1. Lurrus once-per-turn: playing plain_id as a land from graveyard consumes
    //    Lurrus's once-per-turn allowance, blocking bear_a_id and desert_id this turn!
    let (_, legal1) = offer(&engine);
    assert!(legal1.lands.contains(&plain_id));
    engine
        .apply(me(), PlayerAction::PlayLand { card: plain_id })
        .expect("play land from graveyard via Lurrus");
    add_pool(&mut engine, me(), ManaColor::Green, 4);
    let (_, legal_after_land) = offer(&engine);
    assert!(
        !legal_after_land.castable.contains(&bear_a_id),
        "Lurrus once_per_turn land play consumed the turn's allowance"
    );
    assert!(
        !legal_after_land.lands.contains(&desert_id),
        "second graveyard land is blocked once Lurrus's once_per_turn allowance is spent"
    );

    // 2. Filtered PlayLandsFromGraveyard: put Desert Crucible onto battlefield!
    //    With ExtraLandDrops(1) still open, Desert IS playable from graveyard.
    move_to_zone(&mut engine, crucible_id, ZoneLocation::Battlefield);
    let (_, legal_desert) = offer(&engine);
    assert!(
        legal_desert.lands.contains(&desert_id),
        "Desert Crucible opens Desert land in graveyard"
    );
    engine
        .apply(me(), PlayerAction::PlayLand { card: desert_id })
        .expect("play Desert from graveyard via filtered PlayLandsFromGraveyard");
    assert_eq!(
        engine.state.object(desert_id).unwrap().zone,
        Zone::Battlefield
    );

    // Remove Lurrus, advance to me()'s next turn, put both lands in graveyard:
    // Desert is playable, Plain Wastes is NOT!
    move_to_zone(&mut engine, lurrus_id, ZoneLocation::Exile(me()));
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    move_to_zone(&mut engine, desert_id, ZoneLocation::Graveyard(me()));
    move_to_zone(&mut engine, plain_id, ZoneLocation::Graveyard(me()));
    let (_, legal_filtered) = offer(&engine);
    assert!(legal_filtered.lands.contains(&desert_id));
    assert!(
        !legal_filtered.lands.contains(&plain_id),
        "filtered PlayLandsFromGraveyard must reject non-Desert lands"
    );
    assert!(
        crate::casting::play_land(&mut engine.state, me(), plain_id).is_err(),
        "play_land directly refuses non-matching graveyard land"
    );

    // 3. AltCastPrice::NormalPlus (Demonic Embrace self-static in graveyard):
    move_to_zone(&mut engine, embrace_id, ZoneLocation::Graveyard(me()));
    add_pool(&mut engine, me(), ManaColor::Black, 3);

    // At 2 life, cannot pay 3 life -> NOT castable!
    engine.state.players[usize::from(me().get())].life = 2;
    refresh_offer(&mut engine);
    let (_, legal_low_life) = offer(&engine);
    assert!(
        !legal_low_life.castable.contains(&embrace_id),
        "NormalPlus(PayLife(3), Discard) refused at 2 life"
    );

    // Restore to 10 life, empty hand -> NOT castable!
    engine.state.players[usize::from(me().get())].life = 10;
    let hand_snapshot: Vec<ObjectId> = engine.state.zones.list(ZoneLocation::Hand(me())).clone();
    for cid in hand_snapshot {
        move_to_zone(&mut engine, cid, ZoneLocation::Exile(me()));
    }
    let (_, legal_empty_hand) = offer(&engine);
    assert!(
        !legal_empty_hand.castable.contains(&embrace_id),
        "NormalPlus(PayLife(3), Discard) refused with empty hand"
    );

    // Return fodder_id to hand -> NOW Demonic Shade IS castable!
    move_to_zone(&mut engine, fodder_id, ZoneLocation::Hand(me()));
    let (_, legal_embrace) = offer(&engine);
    assert!(legal_embrace.castable.contains(&embrace_id));

    engine
        .apply(me(), PlayerAction::CastSpell { card: embrace_id })
        .expect("start casting Demonic Shade from graveyard");
    assert!(matches!(engine.pending(), Pending::ChooseCards { .. }));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![fodder_id],
            },
        )
        .expect("discard fodder_id for NormalPlus");
    assert_eq!(life(&engine, me()), 7);
    assert_eq!(
        engine.state.object(fodder_id).unwrap().zone,
        Zone::Graveyard
    );
    assert!(!engine.state.object(embrace_id).unwrap().alt_cast);
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(embrace_id).unwrap().zone,
        Zone::Battlefield,
        "Demonic Shade entered battlefield without flashback exile rider"
    );
}

// =========================================================================
// 3. Generalized Flash Grants (Modifier::GrantFlash)
// =========================================================================

#[test]
fn grant_flash_filters_historic_prototype_forms_each_player_and_respects_teferi_lock() {
    static GREEN_CREATURE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::HasColor(ColorSet::of(Color::Green)),
    ]);
    static RAFF_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::GrantFlash {
            who: PlayerRel::You,
            filter: &Filter::Historic,
        }
    )];
    static YEVA_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::GrantFlash {
            who: PlayerRel::You,
            filter: &GREEN_CREATURE,
        }
    )];
    static SHARED_FLASH_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::GrantFlash {
            who: PlayerRel::EachPlayer,
            filter: &Filter::SORCERY,
        }
    )];
    static TEFERI_ABILITIES: &[AbilityDef] = &[static_ability!(
        Filter::Any,
        Modifier::OpponentsCastAsSorcery
    )];
    static DRAW_ONE_SPELL: &[AbilityDef] = &[spell!(&[Effect::DrawCards {
        amount: Amount::Fixed(1),
    }])];

    let raff = card(
        C0,
        creature_face("Raff Probe", "{2}{W}{U}", 3, 3),
        KeywordSet::EMPTY,
        RAFF_ABILITIES,
    );
    let yeva = card(
        C1,
        creature_face("Yeva Probe", "{2}{G}{G}", 4, 4),
        KeywordSet::EMPTY,
        YEVA_ABILITIES,
    );
    let shared_flash = card(
        C2,
        face("Shared Orrery", "{3}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SHARED_FLASH_ABILITIES,
    );
    let teferi = card(
        C3,
        creature_face("Teferi Probe", "{1}{W}{U}", 1, 4),
        KeywordSet::EMPTY,
        TEFERI_ABILITIES,
    );
    let mut legend_face = creature_face("Legend Squire", "{1}{W}", 2, 2);
    legend_face.supertypes = SupertypeSet::LEGENDARY;
    let legend_crea = card(C4, legend_face, KeywordSet::EMPTY, &[]);
    let plain_bear = card(
        C5,
        creature_face("Plain Bear", "{1}{W}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let mut proto_face = creature_face("Rootwire Probe", "{5}", 5, 5);
    proto_face.types = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
    proto_face.prototype = Some(Prototype::new(ManaCost::parse("{1}{G}"), 2, 3));
    let proto_golem = card(C6, proto_face, KeywordSet::EMPTY, &[]);

    let mut disg_face = creature_face("Green Stalker", "{1}{G}", 2, 2);
    disg_face.disguise = Some(ManaCost::parse("{1}{G}"));
    let disg_crea = card(C7, disg_face, KeywordSet::EMPTY, &[]);

    let opp_sorc = card(
        C8,
        face("Quick Counsel", "{1}{U}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        DRAW_ONE_SPELL,
    );

    let mut engine = bench(
        1403,
        vec![
            raff,
            yeva,
            shared_flash,
            teferi,
            legend_crea,
            plain_bear,
            proto_golem,
            disg_crea,
            opp_sorc,
        ],
        [
            Seat::with(&[C0]).holding(&[C1, C2, C4, C5, C6, C7]),
            Seat::default().holding(&[C3, C8]),
        ],
    );
    to_main(&mut engine, them());
    let raff_id = the(&engine, ZoneLocation::Battlefield, C0);
    let yeva_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let shared_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let teferi_id = the(&engine, ZoneLocation::Hand(them()), C3);
    let legend_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let bear_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let proto_id = the(&engine, ZoneLocation::Hand(me()), C6);
    let disg_id = the(&engine, ZoneLocation::Hand(me()), C7);
    let opp_sorc_id = the(&engine, ZoneLocation::Hand(them()), C8);

    add_pool(&mut engine, me(), ManaColor::White, 4);
    add_pool(&mut engine, me(), ManaColor::Green, 6);
    engine
        .apply(them(), PlayerAction::PassPriority)
        .expect("them passes priority to me()");

    // 1. Under Raff: Legendary creature and Artifact creature ARE castable at
    //    instant speed; non-legendary Plain Bear is NOT!
    let (_, legal_raff) = offer(&engine);
    assert!(legal_raff.castable.contains(&legend_id));
    assert!(legal_raff.castable.contains(&proto_id));
    assert!(
        !legal_raff.castable.contains(&bear_id),
        "non-historic Plain Bear must not gain flash from Raff"
    );

    // 2. Replace Raff with Yeva (green creatures have flash):
    //    - proto_golem ({5} colorless front, {1}{G} green Prototype) is castable
    //      ONLY as Prototype (Normal colorless cast is filtered out)!
    //    - disg_crea ({1}{G} green front, {3} colorless Disguise) is castable
    //      ONLY face-up as Normal (colorless Disguise does NOT gain flash)!
    move_to_zone(&mut engine, raff_id, ZoneLocation::Exile(me()));
    move_to_zone(&mut engine, yeva_id, ZoneLocation::Battlefield);
    let (_, legal_yeva) = offer(&engine);
    assert!(
        legal_yeva.castable.contains(&proto_id),
        "green Prototype form gains flash from Yeva"
    );
    let proto_opts = engine
        .cast_options(me(), proto_id, false)
        .expect("prototype options under Yeva");
    assert_eq!(proto_opts.len(), 1);
    assert_eq!(proto_opts[0].kind, CastModeKind::Prototype);

    let disg_opts = engine
        .cast_options(me(), disg_id, false)
        .expect("disguise card options under Yeva");
    assert_eq!(disg_opts.len(), 1);
    assert_eq!(
        disg_opts[0].kind,
        CastModeKind::Normal,
        "colorless face-down Disguise must not inherit green front face's flash"
    );

    // 3. Opponent controls Teferi (OpponentsCastAsSorcery): CR 101.2 restriction
    //    beats GrantFlash!
    move_to_zone(&mut engine, teferi_id, ZoneLocation::Battlefield);
    let (_, legal_teferi) = offer(&engine);
    assert!(
        !legal_teferi.castable.contains(&proto_id) && !legal_teferi.castable.contains(&disg_id),
        "OpponentsCastAsSorcery overrides GrantFlash (CR 101.2)"
    );
    move_to_zone(&mut engine, teferi_id, ZoneLocation::Exile(them()));

    // 4. Shared Orrery (GrantFlash { who: EachPlayer, filter: &Filter::SORCERY }):
    //    During me()'s turn, opponent can cast Quick Counsel at instant speed!
    move_to_zone(&mut engine, shared_id, ZoneLocation::Battlefield);
    to_main(&mut engine, me());
    add_pool(&mut engine, them(), ManaColor::Blue, 2);
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("me() passes priority to them()");
    let (who_opp, legal_opp) = offer(&engine);
    assert_eq!(who_opp, them());
    assert!(
        legal_opp.castable.contains(&opp_sorc_id),
        "EachPlayer GrantFlash allows opponent to cast sorcery on me()'s turn"
    );
}

// =========================================================================
// 4. Dynamic Filter Comparisons + ManaSpentToCast / SourcePower LKI + EventAmount
// =========================================================================

#[test]
fn dynamic_filters_evaluate_source_power_lki_mana_spent_to_cast_and_event_amount() {
    static POWER_LE_SOURCE: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::PowerAtMostAmount(&Amount::SourcePower),
    ]);
    static ALESHA_TRIGGER: &[AbilityDef] = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::MoveAndModify {
            subject: MoveSubject::Target(TargetSpec::CardInGraveyard(
                &POWER_LE_SOURCE,
                PlayerRel::You,
            )),
            expected_origin: Some(ZoneRef::Graveyard),
            blink: false,
            owner_control: false,
            tapped: false,
            face: None,
            counters: None,
            modifiers: &[],
            duration: Duration::Indefinitely,
            attach_source: false,
            when_source_leaves: None,
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &POWER_LE_SOURCE,
            PlayerRel::You,
        )))
    )];

    static CMC_LE_MANA_SPENT: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::CmcAtMostAmount(&Amount::ManaSpentToCast),
    ]);
    static RECLAIMER_ETB: &[AbilityDef] = &[triggered!(
        Trigger::ETB,
        &[Effect::MoveAndModify {
            subject: MoveSubject::Target(TargetSpec::CardInGraveyard(
                &CMC_LE_MANA_SPENT,
                PlayerRel::You,
            )),
            expected_origin: Some(ZoneRef::Graveyard),
            blink: false,
            owner_control: false,
            tapped: false,
            face: None,
            counters: None,
            modifiers: &[],
            duration: Duration::Indefinitely,
            attach_source: false,
            when_source_leaves: None,
        }],
        targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
            &CMC_LE_MANA_SPENT,
            PlayerRel::You,
        )))
    )];

    static CMC_EQ_LANDS: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::CmcEqualsAmount(&Amount::CountOf {
            filter: &Filter::YOUR_LAND,
            zone: ZoneSel::Battlefield,
        }),
    ]);
    static CMC_LE_EVENT: Filter = Filter::CmcAtMostAmount(&Amount::EventAmount);
    static SWORD_TRIGGER: &[AbilityDef] = &[triggered!(
        Trigger::DealsCombatDamageToPlayer(&Filter::This),
        &[Effect::ChooseCardEx {
            chooser: PlayerRel::You,
            zone: ChooseZone::Hand(PlayerRel::You),
            filter: &CMC_LE_EVENT,
            min: 0,
            max: Amount::Fixed(1),
            reveal_chosen: true,
            movement: ChooseMove::Battlefield { tapped: false },
            not_chosen_movement: ChooseMove::None,
            then: &[],
            for_unchosen: &[],
        }]
    )];
    static EXPEL_FILTER: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::PowerAtLeastAmount(&Amount::Fixed(3)),
    ]);
    static EXPEL_SPELL: &[AbilityDef] = &[spell!(&[Effect::DestroyAll {
        filter: &EXPEL_FILTER,
        no_regen: false,
    }])];
    static INSTANT_KILL: &[AbilityDef] = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];
    static BLINK_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::MoveAndModify {
            subject: MoveSubject::Target(TargetSpec::Object(&Filter::CREATURE)),
            expected_origin: Some(ZoneRef::Battlefield),
            blink: true,
            owner_control: false,
            tapped: false,
            face: None,
            counters: None,
            modifiers: &[],
            duration: Duration::Indefinitely,
            attach_source: false,
            when_source_leaves: None,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )];

    let alesha = card(
        C0,
        creature_face("Alesha Probe", "{1}{B}{R}", 3, 2),
        KeywordSet::HASTE,
        ALESHA_TRIGGER,
    );
    let pow3_gy = card(
        C1,
        creature_face("Three Power Ally", "{2}{R}", 3, 3),
        KeywordSet::EMPTY,
        &[],
    );
    let pow5_gy = card(
        C2,
        creature_face("Five Power Titan", "{4}{R}", 5, 5),
        KeywordSet::EMPTY,
        &[],
    );
    let reclaimer = card(
        C3,
        creature_face("Astelli Probe", "{2}{W}{W}", 2, 3),
        KeywordSet::EMPTY,
        RECLAIMER_ETB,
    );
    let sword_runner = card(
        C4,
        creature_face("Buster Runner", "{2}", 2, 2),
        KeywordSet::HASTE,
        SWORD_TRIGGER,
    );
    let expel = card(
        C5,
        face("Expel Probe", "{2}{W}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        EXPEL_SPELL,
    );
    let kill_instant = card(
        C6,
        face("Instant Doom", "{B}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        INSTANT_KILL,
    );
    let blink_instant = card(
        C7,
        face("Instant Blink", "{W}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BLINK_SPELL,
    );
    let probe_land = card(C8, land_face("Probe Plains"), KeywordSet::EMPTY, &[]);
    let mv2_hand_crea = card(
        C9,
        creature_face("Two MV Squire", "{1}{W}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );

    let mut engine = bench(
        1404,
        vec![
            alesha,
            pow3_gy,
            pow5_gy,
            reclaimer,
            sword_runner,
            expel,
            kill_instant,
            blink_instant,
            probe_land,
            mv2_hand_crea,
        ],
        [
            Seat::with(&[C3, C4, C5, C7, C8]).holding(&[C0, C1, C2, C6, C9]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let reclaimer_id = the(&engine, ZoneLocation::Battlefield, C3);
    let runner_id = the(&engine, ZoneLocation::Battlefield, C4);
    let expel_id = the(&engine, ZoneLocation::Battlefield, C5);
    let blink_id = the(&engine, ZoneLocation::Battlefield, C7);
    let land_bf = the(&engine, ZoneLocation::Battlefield, C8);
    let alesha_id = the(&engine, ZoneLocation::Hand(me()), C0);
    let pow3_id = the(&engine, ZoneLocation::Hand(me()), C1);
    let pow5_id = the(&engine, ZoneLocation::Hand(me()), C2);
    let kill_id = the(&engine, ZoneLocation::Hand(me()), C6);
    let mv2_hand_id = the(&engine, ZoneLocation::Hand(me()), C9);

    move_to_zone(&mut engine, reclaimer_id, ZoneLocation::Hand(me()));
    move_to_zone(&mut engine, runner_id, ZoneLocation::Exile(me()));
    move_to_zone(&mut engine, expel_id, ZoneLocation::Hand(me()));
    move_to_zone(&mut engine, blink_id, ZoneLocation::Hand(me()));
    move_to_zone(&mut engine, pow3_id, ZoneLocation::Graveyard(me()));
    move_to_zone(&mut engine, pow5_id, ZoneLocation::Graveyard(me()));

    // 1. CmcEqualsAmount(&CountOf(YOUR_LAND)): with 1 land on battlefield,
    //    mv2_hand_id (MV 2) does NOT match; put one Forest onto battlefield
    //    (now 2 lands) -> mv2_hand_id (MV 2) matches and pow3_id (MV 3) does not!
    assert!(!crate::eval::matches(
        &CMC_EQ_LANDS,
        &engine.state,
        engine.state.object(mv2_hand_id).unwrap(),
        me(),
        land_bf
    ));
    let extra_forest = *engine
        .state
        .zones
        .list(ZoneLocation::Library(me()))
        .last()
        .expect("library has Forests");
    move_to_zone(&mut engine, extra_forest, ZoneLocation::Battlefield);
    assert!(crate::eval::matches(
        &CMC_EQ_LANDS,
        &engine.state,
        engine.state.object(mv2_hand_id).unwrap(),
        me(),
        land_bf
    ));
    assert!(!crate::eval::matches(
        &CMC_EQ_LANDS,
        &engine.state,
        engine.state.object(pow3_id).unwrap(),
        me(),
        land_bf
    ));

    // 2. Cast Astelli Probe for {2}{W}{W} (4 mana spent) -> ETB trigger targets
    //    pow3_id (MV 3 <= 4) and refuses pow5_id (MV 5 > 4)!
    add_pool(&mut engine, me(), ManaColor::White, 5);
    engine
        .apply(me(), PlayerAction::CastSpell { card: reclaimer_id })
        .expect("cast Astelli Probe for 4 mana");
    resolve_top(&mut engine);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ETB target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&pow3_id) && !options.contains(&pow5_id),
        "ManaSpentToCast = 4 allows MV 3 and excludes MV 5 (got {options:?})"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![pow3_id],
            },
        )
        .expect("target MV 3 creature");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(pow3_id).unwrap().zone,
        Zone::Battlefield,
        "Astelli Probe ETB returned MV 3 creature"
    );

    // Move pow3_id back to graveyard and blink Astelli Probe without casting it:
    // ManaSpentToCast is now 0 -> pow3_id (MV 3) is NOT a legal target!
    move_to_zone(&mut engine, pow3_id, ZoneLocation::Graveyard(me()));
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink_id })
        .expect("cast Instant Blink");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![reclaimer_id],
            },
        )
        .expect("target Astelli Probe");
    settle(&mut engine, me());
    assert_eq!(
        engine.state.object(pow3_id).unwrap().zone,
        Zone::Graveyard,
        "blinked permanent was not cast (ManaSpentToCast == 0), so MV 3 stays in graveyard"
    );

    // 3. Alesha (3/2) + Buster Runner (2/2) attack!
    move_to_zone(&mut engine, alesha_id, ZoneLocation::Battlefield);
    move_to_zone(&mut engine, runner_id, ZoneLocation::Battlefield);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (alesha_id, baylee_core::ids::Defender::Player(them())),
                    (runner_id, baylee_core::ids::Defender::Player(them())),
                ],
            },
        )
        .expect("declare Alesha and Buster Runner attacking");
    let Pending::ChooseTargets {
        options: alesha_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected Alesha target prompt, got {:?}", engine.pending());
    };
    assert!(alesha_opts.contains(&pow3_id) && !alesha_opts.contains(&pow5_id));
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![pow3_id],
            },
        )
        .expect("choose pow3_id for Alesha");

    // Kill Alesha while her trigger is still on the stack!
    add_pool(&mut engine, me(), ManaColor::Black, 1);
    engine
        .apply(me(), PlayerAction::CastSpell { card: kill_id })
        .expect("cast Instant Doom on Alesha in response");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![alesha_id],
            },
        )
        .expect("target Alesha");
    resolve_top(&mut engine); // resolve Instant Doom -> Alesha dies
    assert_eq!(
        engine.state.object(alesha_id).unwrap().zone,
        Zone::Graveyard
    );
    resolve_top(&mut engine); // resolve Alesha's trigger using LKI SourcePower == 3!
    assert_eq!(
        engine.state.object(pow3_id).unwrap().zone,
        Zone::Battlefield,
        "LKI SourcePower == 3 preserved pow3_id target legality after Alesha died"
    );

    // Buster Runner deals 2 combat damage -> ChooseCardEx with CmcAtMostAmount(&EventAmount)
    // offers mv2_hand_id (MV 2), while excluding expel_id (MV 3)!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options: sword_opts,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected Buster Sword ChooseCards, got {:?}",
            engine.pending()
        );
    };
    assert!(
        sword_opts.contains(&mv2_hand_id) && !sword_opts.contains(&expel_id),
        "CmcAtMostAmount(&EventAmount) = 2 includes MV 2 card and excludes MV 3 card"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![mv2_hand_id],
            },
        )
        .expect("put Two MV Squire onto battlefield");
    to_second_main(&mut engine, me());
    assert_eq!(
        engine.state.object(mv2_hand_id).unwrap().zone,
        Zone::Battlefield,
        "Buster Runner combat trigger put MV 2 card onto battlefield"
    );
    assert_eq!(
        engine.state.object(expel_id).unwrap().zone,
        Zone::Hand,
        "MV 3 card remained in hand"
    );

    // 4. Cast Expel Probe (DestroyAll PowerAtLeastAmount(3)):
    //    destroys pow3_id (3/3), spares runner_id (2/2), mv2_hand_id (2/2),
    //    and land_bf (no power, CR 208.3)!
    add_pool(&mut engine, me(), ManaColor::White, 3);
    engine
        .apply(me(), PlayerAction::CastSpell { card: expel_id })
        .expect("cast Expel Probe");
    settle(&mut engine, me());
    assert_eq!(engine.state.object(pow3_id).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state.object(runner_id).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state.object(land_bf).unwrap().zone,
        Zone::Battlefield,
        "noncreature land has no power (CR 208.3) and is unaffected by PowerAtLeastAmount"
    );
}

// =========================================================================
// 5. GreatestPowerAmong, GreatestManaValueAmong & Cycle-Safe Depth Cap
// =========================================================================

#[test]
fn greatest_power_and_mana_value_amounts_and_cyclic_filter_depth_cap_terminate() {
    static CYCLIC_FILTER: Filter = Filter::PowerAtMostAmount(&CYCLIC_AMOUNT);
    static CYCLIC_AMOUNT: Amount = Amount::GreatestPowerAmong(&CYCLIC_FILTER);

    static GY_CREATURE_CARDS: Filter = Filter::And(&[
        Filter::CREATURE,
        Filter::OwnedByYou,
        Filter::InZone(ZoneRef::Graveyard),
    ]);
    static BEATDOWN_SPELL: &[AbilityDef] = &[spell!(
        &[Effect::DealDamage {
            target: TargetSpec::AnyOpponent,
            amount: Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent))
    )];
    static GREATEST_MV_SPELL: &[AbilityDef] = &[spell!(&[
        Effect::GainLife {
            amount: Amount::GreatestManaValueAmong(&Filter::ControlledByYou),
        },
        Effect::DrawCards {
            amount: Amount::GreatestPowerAmong(&GY_CREATURE_CARDS),
        },
    ])];
    static CYCLIC_ANTHEM_ABILITIES: &[AbilityDef] =
        &[static_ability!(CYCLIC_FILTER, Modifier::ModifyPT(1, 1))];

    let bear = card(
        C0,
        creature_face("Two Drop", "{1}{G}", 2, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let beast = card(
        C1,
        creature_face("Four Power Five MV", "{4}{G}", 4, 4),
        KeywordSet::EMPTY,
        &[],
    );
    let relic6 = card(
        C2,
        face("Six MV Relic", "{6}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        &[],
    );
    let gy_ogre = card(
        C3,
        creature_face("Grave Three Power", "{2}{R}", 3, 2),
        KeywordSet::EMPTY,
        &[],
    );
    let beatdown = card(
        C4,
        face("Beatdown Probe", "{1}{G}", TypeSet::INSTANT),
        KeywordSet::EMPTY,
        BEATDOWN_SPELL,
    );
    let mv_life = card(
        C5,
        face("Insight Probe", "{1}{W}", TypeSet::SORCERY),
        KeywordSet::EMPTY,
        GREATEST_MV_SPELL,
    );
    let cyclic_totem = card(
        C6,
        face("Cyclic Totem", "{2}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CYCLIC_ANTHEM_ABILITIES,
    );

    let mut engine = bench(
        1405,
        vec![
            bear,
            beast,
            relic6,
            gy_ogre,
            beatdown,
            mv_life,
            cyclic_totem,
        ],
        [
            Seat::with(&[C0, C1, C2]).holding(&[C3, C4, C5, C6]),
            Seat::default(),
        ],
    );
    to_main(&mut engine, me());
    let bear_id = the(&engine, ZoneLocation::Battlefield, C0);
    let beast_id = the(&engine, ZoneLocation::Battlefield, C1);
    let gy_ogre_id = the(&engine, ZoneLocation::Hand(me()), C3);
    let beatdown_id = the(&engine, ZoneLocation::Hand(me()), C4);
    let mv_life_id = the(&engine, ZoneLocation::Hand(me()), C5);
    let cyclic_id = the(&engine, ZoneLocation::Hand(me()), C6);

    move_to_zone(&mut engine, gy_ogre_id, ZoneLocation::Graveyard(me()));

    // 1. GreatestPowerAmong(&YOUR_CREATURE) == 4 (between 2/2 and 4/4).
    let opp_life_start = life(&engine, them());
    add_pool(&mut engine, me(), ManaColor::Green, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: beatdown_id })
        .expect("cast Beatdown Probe");
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .expect("target opponent");
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), opp_life_start - 4);

    // 2. GreatestManaValueAmong(&ControlledByYou) == 6 (Six MV Relic),
    //    and cross-zone GreatestPowerAmong(&GY_CREATURE_CARDS) == 3 (Grave Three Power)!
    let my_life_before = life(&engine, me());
    let hand_before = engine.state.zones.list(ZoneLocation::Hand(me())).len();
    add_pool(&mut engine, me(), ManaColor::White, 2);
    engine
        .apply(me(), PlayerAction::CastSpell { card: mv_life_id })
        .expect("cast Insight Probe");
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        my_life_before + 6,
        "GreatestManaValueAmong counted Six MV Relic (6)"
    );
    assert_eq!(
        engine.state.zones.list(ZoneLocation::Hand(me())).len(),
        hand_before - 1 + 3,
        "cross-zone GreatestPowerAmong counted 3-power creature card in graveyard"
    );

    // 3. Negative power floor (CR 107.1b): remove beast_id and debuff bear_id to -2/2 ->
    //    GreatestPowerAmong returns 0!
    move_to_zone(&mut engine, beast_id, ZoneLocation::Exile(me()));
    let ts = engine.state.next_timestamp();
    let bear_filter = crate::effects::EffectFilter::object(&engine.state, bear_id);
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: me(),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::PtModify,
            timestamp: ts,
            duration: Duration::UntilEndOfTurn,
            filter: bear_filter,
            modifier: Modifier::ModifyPT(-4, 0),
        });
    refresh_offer(&mut engine);
    assert_eq!(
        engine
            .state
            .object(bear_id)
            .unwrap()
            .characteristics()
            .power,
        Some(-2)
    );
    assert_eq!(
        crate::eval::amount(
            &Amount::GreatestPowerAmong(&Filter::YOUR_CREATURE),
            &engine.state,
            me(),
            bear_id,
            None,
        ),
        0,
        "negative greatest power floors at 0 (CR 107.1b)"
    );

    // 4. Cycle-safe depth cap: put Cyclic Totem (CYCLIC_FILTER <-> CYCLIC_AMOUNT)
    //    onto battlefield, refresh characteristics, evaluate matches, and take
    //    snapshot_hash — must terminate cleanly and deterministically!
    move_to_zone(&mut engine, cyclic_id, ZoneLocation::Battlefield);
    let bear_obj = engine.state.object(bear_id).unwrap();
    let cycle_res_1 =
        crate::eval::matches(&CYCLIC_FILTER, &engine.state, bear_obj, me(), cyclic_id);
    let cycle_res_2 =
        crate::eval::matches(&CYCLIC_FILTER, &engine.state, bear_obj, me(), cyclic_id);
    assert_eq!(cycle_res_1, cycle_res_2);
    let h1 = engine.state.snapshot_hash();
    let h2 = engine.state.snapshot_hash();
    assert_eq!(
        h1, h2,
        "snapshot_hash on cyclic filter/amount is deterministic"
    );
}
