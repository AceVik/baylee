//! Engine rule unit tests for resolution-time and as-enters creature-type
//! choices (CR 205.1b, CR 205.3m, CR 608.2d, CR 611.2c, CR 612.1, CR 614.12a).

use super::*;
use crate::choice::{Pending, PlayerAction};
use crate::engine::Engine;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::object::Characteristics;
use baylee_cards_dsl::{
    CardDef, Condition, Coverage, Duration, Effect, EnterModifier, FaceDef, Filter, Modifier,
    PlayerRel, SearchDest, TextWordKind, static_ability,
};
use baylee_core::generated::subtypes::{ALL_CREATURE_TYPES, creature, land};
use baylee_core::ids::{CardIndex, ObjectId, PlayerId, SeatSet};
use baylee_core::types::{SubtypeSet, TypeSet};
use smallvec::SmallVec;

fn two_player_state() -> GameState {
    let mut game = preset(911, &[]);
    game.seats.push(game.seats[0].clone());
    GameState::from_preset(&game, &SyntheticLookup::new(vec![])).unwrap()
}

fn make_creature(
    state: &mut GameState,
    controller: PlayerId,
    name: &str,
    subtypes: &[baylee_core::ids::SubtypeId],
) -> ObjectId {
    let name = state.names.intern(name);
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let obj = state.object_mut(id).unwrap();
    let base = obj.base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(2);
    base.toughness = Some(2);
    for &st in subtypes {
        base.subtypes.insert(st);
    }
    id
}

fn make_resolution(
    state: &GameState,
    source: ObjectId,
    controller: PlayerId,
    effects: Vec<Effect>,
) -> Resolution {
    let version = state.object(source).map(|o| o.version);
    Resolution {
        source,
        on_stack: source,
        controller,
        effects,
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: SubjectContext::default().with_source_version(version),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    }
}

#[test]
fn choose_creature_type_offers_all_creature_types_and_honors_exclusion() {
    let mut state = two_player_state();
    let p0 = PlayerId::new(0);
    let src = make_creature(&mut state, p0, "Shapeshifter", &[creature::ILLUSION]);

    let mut res = make_resolution(
        &state,
        src,
        p0,
        vec![Effect::ChooseCreatureType { exclude: None }],
    );
    let Flow::Wait(Pending::ChooseSubtype { player, options }) = run(&mut state, &mut res) else {
        panic!("expected Pending::ChooseSubtype");
    };
    assert_eq!(player, p0);
    assert_eq!(options, ALL_CREATURE_TYPES.iter().collect::<Vec<_>>());
    assert!(options.contains(&creature::WALL));
    assert!(options.contains(&creature::TALAXIAN));
    assert!(!options.contains(&land::ISLAND));

    let mut res_excl = make_resolution(
        &state,
        src,
        p0,
        vec![Effect::ChooseCreatureType {
            exclude: Some(creature::WALL),
        }],
    );
    let Flow::Wait(Pending::ChooseSubtype { options, .. }) = run(&mut state, &mut res_excl) else {
        panic!("expected Pending::ChooseSubtype");
    };
    assert!(!options.contains(&creature::WALL));
    assert_eq!(options.len(), ALL_CREATURE_TYPES.len() as usize - 1);

    // Rejecting a forced excluded answer leaves chosen_subtype unset.
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res_excl, creature::WALL),
        Flow::Complete
    ));
    assert_eq!(res_excl.subject.chosen_subtype, None);

    // Choosing a legal creature type records it on the resolution.
    let mut res_ok = make_resolution(
        &state,
        src,
        p0,
        vec![Effect::ChooseCreatureType {
            exclude: Some(creature::WALL),
        }],
    );
    assert!(matches!(run(&mut state, &mut res_ok), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res_ok, creature::ELF),
        Flow::Complete
    ));
    assert_eq!(res_ok.subject.chosen_subtype, Some(creature::ELF));
}

#[test]
fn become_chosen_creature_type_replaces_and_ignores_text_changes() {
    let mut state = two_player_state();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bear = make_creature(&mut state, p0, "Grizzly Bears", &[creature::BEAR]);
    let goblin = make_creature(&mut state, p1, "Goblin", &[creature::GOBLIN]);

    let mut res = make_resolution(
        &state,
        bear,
        p0,
        vec![
            Effect::ChooseCreatureType {
                exclude: Some(creature::WALL),
            },
            Effect::BecomeChosenCreatureType {
                filter: &Filter::This,
                additive: false,
                duration: Duration::UntilEndOfTurn,
            },
        ],
    );
    res.targeted = true;
    res.targets.push(goblin);
    let mut text = crate::text_changes::TextChangeMap::IDENTITY;
    assert!(text.replace(crate::text_changes::TextReplacement {
        kind: TextWordKind::BasicLandType,
        from: 0,
        to: 1,
    }));
    res.text = text;
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res, creature::DRAGON),
        Flow::Complete
    ));
    state.refresh_characteristics();
    let g_subs = state.object(goblin).unwrap().characteristics().subtypes;
    assert!(g_subs.contains(creature::DRAGON));
    assert!(!g_subs.contains(creature::GOBLIN));
    let fx_id = state.effects.iter().last().unwrap().id;
    assert!(state.effect_text_overrides.iter().any(|(id, origin)| {
        *id == fx_id
            && *origin
                == crate::text_changes::TextOrigin::Frozen(
                    crate::text_changes::TextChangeMap::IDENTITY,
                )
    }));

    state
        .effects
        .remove_where(|fx| fx.duration == Duration::UntilEndOfTurn);
    state.refresh_characteristics();
    let g_subs_after = state.object(goblin).unwrap().characteristics().subtypes;
    assert!(g_subs_after.contains(creature::GOBLIN));
    assert!(!g_subs_after.contains(creature::DRAGON));
}

#[test]
fn become_chosen_creature_type_additive_indefinite_and_sweep_locks_set() {
    let mut state = two_player_state();
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bear = make_creature(&mut state, p0, "Grizzly Bears", &[creature::BEAR]);
    let goblin = make_creature(&mut state, p1, "Goblin", &[creature::GOBLIN]);

    let mut res_add = make_resolution(
        &state,
        bear,
        p0,
        vec![
            Effect::ChooseCreatureType { exclude: None },
            Effect::BecomeChosenCreatureType {
                filter: &Filter::This,
                additive: true,
                duration: Duration::Indefinitely,
            },
        ],
    );
    assert!(matches!(run(&mut state, &mut res_add), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res_add, creature::SLIVER),
        Flow::Complete
    ));
    state
        .effects
        .remove_where(|fx| fx.duration == Duration::UntilEndOfTurn);
    state.refresh_characteristics();
    let b_subs = state.object(bear).unwrap().characteristics().subtypes;
    assert!(b_subs.contains(creature::BEAR));
    assert!(b_subs.contains(creature::SLIVER));

    // Sweep filter locks affected set at resolution (CR 611.2c).
    let mut res_all = make_resolution(
        &state,
        bear,
        p0,
        vec![
            Effect::ChooseCreatureType {
                exclude: Some(creature::WALL),
            },
            Effect::BecomeChosenCreatureType {
                filter: &Filter::CREATURE,
                additive: false,
                duration: Duration::UntilEndOfTurn,
            },
        ],
    );
    assert!(matches!(run(&mut state, &mut res_all), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res_all, creature::ZOMBIE),
        Flow::Complete
    ));
    let late_elf = make_creature(&mut state, p0, "Late Elf", &[creature::ELF]);
    state.refresh_characteristics();
    for id in [bear, goblin] {
        assert!(
            state
                .object(id)
                .unwrap()
                .characteristics()
                .subtypes
                .contains(creature::ZOMBIE)
        );
    }
    let late_subs = state.object(late_elf).unwrap().characteristics().subtypes;
    assert!(late_subs.contains(creature::ELF));
    assert!(!late_subs.contains(creature::ZOMBIE));
}

#[test]
fn become_chosen_creature_type_follows_attached_host_including_departed_aura_lki() {
    let mut state = two_player_state();
    let p0 = PlayerId::new(0);
    let host = make_creature(&mut state, p0, "Host", &[creature::SOLDIER]);
    let host_version = state.object(host).unwrap().version;
    let aura_name = state.names.intern("Mask Aura");
    let aura = state.create_bare(
        p0,
        ObjectKind::Permanent,
        aura_name,
        ZoneLocation::Battlefield,
    );
    state.object_mut(aura).unwrap().attached_to = Some(host);

    // Move aura to graveyard and record its attachment LKI on the stack object.
    state
        .move_object(
            aura,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let stack_name = state.names.intern("Mask Ability");
    let stack_obj = state.create_bare(
        p0,
        ObjectKind::AbilityOnStack,
        stack_name,
        ZoneLocation::Stack,
    );
    state
        .object_mut(stack_obj)
        .unwrap()
        .riders
        .push(crate::object::Rider::SourceAttachmentLki(host));
    state
        .object_mut(stack_obj)
        .unwrap()
        .riders
        .push(crate::object::Rider::SourceAttachmentVersion(host_version));

    let mut res = make_resolution(
        &state,
        aura,
        p0,
        vec![
            Effect::ChooseCreatureType { exclude: None },
            Effect::BecomeChosenCreatureType {
                filter: &Filter::AttachedToBySource,
                additive: false,
                duration: Duration::UntilEndOfTurn,
            },
        ],
    );
    res.on_stack = stack_obj;
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res, creature::WIZARD),
        Flow::Complete
    ));
    state.refresh_characteristics();
    let h_subs = state.object(host).unwrap().characteristics().subtypes;
    assert!(h_subs.contains(creature::WIZARD));
    assert!(!h_subs.contains(creature::SOLDIER));
}

fn move_top(state: &mut GameState, id: ObjectId, to: ZoneLocation) {
    state
        .move_object(id, to, ZonePosition::Top, Cause::Effect)
        .unwrap();
}

#[test]
fn resolution_chosen_type_drives_condition_reveal_and_graveyard_shuffle_after_source_leaves() {
    static CHOSEN_CREATURE: Filter =
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);
    static GAIN_ALL_CHOSEN: &[Effect] = &[Effect::CreateContinuousEffect {
        filter: &CHOSEN_CREATURE,
        modifier: Modifier::GainControl,
        duration: Duration::Indefinitely,
        layer: baylee_cards_dsl::Layer::Control,
    }];

    let mut state = two_player_state();
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    // Source has a DIFFERENT persistent chosen_subtype (Goblin) and leaves the
    // battlefield (CR 113.7a): resolution choice (Elf) must override it cleanly.
    let source = make_creature(&mut state, p0, "Sacrificed Source", &[creature::WIZARD]);
    state.object_mut(source).unwrap().chosen_subtype = Some(creature::GOBLIN);
    let _p0_elf_1 = make_creature(&mut state, p0, "Elf 1", &[creature::ELF]);
    let _p0_elf_2 = make_creature(&mut state, p0, "Elf 2", &[creature::ELF]);
    let p1_elf = make_creature(&mut state, p1, "Opp Elf", &[creature::ELF]);
    let p1_goblin = make_creature(&mut state, p1, "Opp Goblin", &[creature::GOBLIN]);

    let gy_elf = make_creature(&mut state, p0, "GY Elf", &[creature::ELF]);
    let gy_gob = make_creature(&mut state, p0, "GY Goblin", &[creature::GOBLIN]);
    move_top(&mut state, gy_elf, ZoneLocation::Graveyard(p0));
    move_top(&mut state, gy_gob, ZoneLocation::Graveyard(p0));

    let lib_elf = make_creature(&mut state, p0, "Lib Elf", &[creature::ELF]);
    let lib_top = make_creature(&mut state, p0, "Lib Goblin", &[creature::GOBLIN]);
    move_top(&mut state, lib_elf, ZoneLocation::Library(p0));
    move_top(&mut state, lib_top, ZoneLocation::Library(p0));
    move_top(&mut state, source, ZoneLocation::Graveyard(p0));

    let mut res = make_resolution(
        &state,
        source,
        p0,
        vec![
            Effect::ChooseCreatureType { exclude: None },
            Effect::IfCondition {
                condition: Condition::ControlMoreThanEachOtherPlayer(&CHOSEN_CREATURE),
                then: GAIN_ALL_CHOSEN,
                otherwise: &[],
            },
            Effect::RevealUntilMatching {
                player: PlayerRel::You,
                filter: &CHOSEN_CREATURE,
                matched: SearchDest::Battlefield,
                otherwise: SearchDest::TopOfLibrary,
            },
            Effect::ShuffleGraveyardMatchingIntoLibrary {
                who: PlayerRel::You,
                filter: &CHOSEN_CREATURE,
            },
        ],
    );
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res, creature::ELF),
        Flow::Complete
    ));
    state.refresh_characteristics();

    assert_eq!(state.object(p1_elf).unwrap().controller, p0);
    assert_eq!(state.object(p1_goblin).unwrap().controller, p1);
    assert_eq!(
        state.object(lib_elf).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        state.object(lib_top).unwrap().zone,
        crate::zone::Zone::Library
    );
    assert_eq!(
        state.object(gy_elf).unwrap().zone,
        crate::zone::Zone::Library
    );
    assert_eq!(
        state.object(gy_gob).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        state.object(source).unwrap().chosen_subtype,
        Some(creature::GOBLIN)
    );
}

#[test]
fn destroy_all_of_sweeps_only_chosen_players_matching_creatures() {
    static CHOSEN_CREATURE: Filter =
        Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);
    let mut state = two_player_state();
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let source = make_creature(&mut state, p0, "Source", &[creature::WIZARD]);
    let p0_gob = make_creature(&mut state, p0, "My Goblin", &[creature::GOBLIN]);
    let p1_gob = make_creature(&mut state, p1, "Opp Goblin", &[creature::GOBLIN]);
    let p1_elf = make_creature(&mut state, p1, "Opp Elf", &[creature::ELF]);

    let mut res = make_resolution(
        &state,
        source,
        p0,
        vec![
            Effect::ChooseCreatureType { exclude: None },
            Effect::DestroyAllOf {
                who: PlayerRel::Chosen,
                filter: &CHOSEN_CREATURE,
                no_regen: true,
            },
        ],
    );
    res.chosen_player = Some(p1);
    assert!(matches!(run(&mut state, &mut res), Flow::Wait(_)));
    assert!(matches!(
        resume_with_subtype(&mut state, &mut res, creature::GOBLIN),
        Flow::Complete
    ));
    assert_eq!(
        state.object(p1_gob).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    assert_eq!(
        state.object(p1_elf).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(
        state.object(p0_gob).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

fn cast_and_wait_subtype<L: crate::state::CardLookup>(
    engine: &mut Engine<L>,
    caster: PlayerId,
    card: ObjectId,
) -> (PlayerId, Vec<baylee_core::ids::SubtypeId>) {
    engine
        .apply(caster, PlayerAction::CastSpell { card })
        .unwrap();
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseSubtype { player, options } => return (player, options),
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
    panic!("did not reach Pending::ChooseSubtype");
}

const PLAIN_IDX: u32 = 900_001;
const THRONE_IDX: u32 = 900_002;

static PLAIN_CHOOSER: CardDef = CardDef {
    index: CardIndex::new(PLAIN_IDX),
    oracle_id: "00000000-0000-0000-0000-000000000001",
    scryfall_id: "00000000-0000-0000-0000-000000000001",
    faces: &[FaceDef {
        name: "Plain Herald",
        mana_cost: baylee_core::mana!("{0}"),
        castable_from_hand: true,
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::GOLEM],
        power: Some(2),
        toughness: Some(2),
        enter_modifiers: &[EnterModifier::ChooseSubtype],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[],
    ..CardDef::DEFAULT
};

static THRONE_CHOOSER: CardDef = CardDef {
    index: CardIndex::new(THRONE_IDX),
    oracle_id: "00000000-0000-0000-0000-000000000002",
    scryfall_id: "00000000-0000-0000-0000-000000000002",
    faces: &[FaceDef {
        name: "Throne Clone",
        mana_cost: baylee_core::mana!("{0}"),
        castable_from_hand: true,
        types: TypeSet::CREATURE.union(TypeSet::ARTIFACT),
        subtypes: &[creature::GOLEM],
        power: Some(4),
        toughness: Some(4),
        enter_modifiers: &[EnterModifier::ChooseSubtype],
        ..FaceDef::DEFAULT
    }],
    coverage: Coverage::Implemented,
    abilities: &[static_ability!(Filter::This, Modifier::AddChosenSubtype)],
    ..CardDef::DEFAULT
};

#[test]
fn enter_choose_subtype_offers_all_creature_types_and_only_adds_subtype_with_modifier() {
    let deck_entry = |card: u32| baylee_core::preset::DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: CardIndex::new(card),
        print: baylee_core::ids::PrintRef::new(0),
    };
    let mut game = preset(912, &[]);
    game.seats[0].starting_hand = Some(vec![deck_entry(PLAIN_IDX), deck_entry(THRONE_IDX)]);
    let mut engine = Engine::new(
        &game,
        SyntheticLookup::new(vec![&PLAIN_CHOOSER, &THRONE_CHOOSER]),
    )
    .unwrap();
    let p0 = PlayerId::new(0);
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            Pending::Priority { player, .. }
                if engine.state().turn.phase == crate::turn::Phase::FirstMain
                    && engine.state().turn.active == p0
                    && player == p0 =>
            {
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending before main phase: {other:?}"),
        }
    }
    let find_in_hand = |engine: &Engine<SyntheticLookup>, idx: u32| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .find(|&id| {
                engine
                    .state()
                    .object(id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == CardIndex::new(idx)))
            })
            .unwrap()
    };
    let (plain_id, throne_id) = (
        find_in_hand(&engine, PLAIN_IDX),
        find_in_hand(&engine, THRONE_IDX),
    );

    let (player, options) = cast_and_wait_subtype(&mut engine, p0, plain_id);
    assert_eq!(player, p0);
    assert_eq!(options, ALL_CREATURE_TYPES.iter().collect::<Vec<_>>());
    assert!(options.contains(&creature::TALAXIAN));
    assert!(!options.contains(&land::ISLAND));
    engine
        .apply(p0, PlayerAction::ChooseSubtype(creature::TALAXIAN))
        .unwrap();
    let plain_subs = engine
        .state()
        .object(plain_id)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(plain_subs.contains(creature::GOLEM));
    assert!(!plain_subs.contains(creature::TALAXIAN));

    cast_and_wait_subtype(&mut engine, p0, throne_id);
    engine
        .apply(p0, PlayerAction::ChooseSubtype(creature::TALAXIAN))
        .unwrap();
    let throne_obj = engine.state().object(throne_id).unwrap();
    assert!(
        throne_obj
            .characteristics()
            .subtypes
            .contains(creature::GOLEM)
    );
    assert!(
        throne_obj
            .characteristics()
            .subtypes
            .contains(creature::TALAXIAN)
    );
    let base_subtypes: SubtypeSet =
        Characteristics::from_face(&THRONE_CHOOSER, 0, throne_obj.base.name).subtypes;
    assert_eq!(throne_obj.base.subtypes, base_subtypes);
}
