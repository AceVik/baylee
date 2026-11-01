use super::*;
use crate::state::{CardLookup, ReplacementEntry};
use crate::text_changes::TextChangeMap;
use baylee_cards_dsl::{Filter, ReplacementRule};
use baylee_core::ids::CardIndex;
use baylee_core::preset::{
    AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
    SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn me() -> PlayerId {
    PlayerId::new(0)
}
fn them() -> PlayerId {
    PlayerId::new(1)
}

fn treasure() -> &'static baylee_cards_dsl::TokenDef {
    &baylee_cards::tokens::TREASURE
}

/// Seat 0 starts with one Forest on the battlefield: a token copy of a
/// **card** is the one branch that needs a card to copy.
fn state() -> GameState {
    let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
        .expect("registry contains Forest")
        .index;
    let entry = DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card: forest,
        print: baylee_core::ids::PrintRef::new(0),
    };
    let seat = |bf: Vec<DeckEntry>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: SeatCapabilities::default(),
        deck: (0..60).map(|_| entry).collect(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: bf,
        emblems: vec![],
        team: None,
    };
    let preset = GamePreset {
        rules: baylee_core::card_metadata::RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 6,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: baylee_core::preset::Finish::Normal,
        }],
        seats: vec![seat(vec![entry]), seat(vec![])],
    };
    GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
}

fn the_forest(state: &GameState) -> ObjectId {
    *state
        .zones
        .list(ZoneLocation::Battlefield)
        .first()
        .expect("seat 0 started with one")
}

/// "If one or more tokens would be created under your control" — a
/// Doubling Season controlled by seat 0.
fn doubles_tokens(state: &mut GameState, controller: PlayerId) {
    let name = state.names.intern("Doubling Season");
    let source = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    state.replacement_rules.push(ReplacementEntry {
        source,
        controller,
        rule: ReplacementRule::DoubleTokenCreation {
            controller_filter: &Filter::ControlledByYou,
        },
    });
}

#[test]
fn typed_multiplier_distinguishes_creatures_copies_recipients_and_overflow() {
    use baylee_core::types::TypeSet;
    let mut state = state();
    let source = the_forest(&state);
    state.replacement_rules.push(ReplacementEntry {
        source,
        controller: me(),
        rule: ReplacementRule::MultiplyTypedTokens {
            controller_filter: &Filter::ControlledByYou,
            types: TypeSet::CREATURE,
            factor: 3,
        },
    });
    doubles_tokens(&mut state, me());
    assert_eq!(
        create_tokens(&mut state, me(), treasure(), None, 1).len(),
        2
    );
    let token = &baylee_cards::tokens::PHYREXIAN_MITE_1_1;
    assert_eq!(create_tokens(&mut state, me(), token, None, 1).len(), 6);
    assert_eq!(create_tokens(&mut state, them(), token, None, 1).len(), 1);
    let mut base = (*state.object(source).unwrap().base).clone();
    base.types = TypeSet::ARTIFACT.union(TypeSet::CREATURE);
    base.power = Some(4);
    base.toughness = Some(4);
    assert_eq!(
        create_token_copies(&mut state, me(), source, &std::sync::Arc::new(base), 1).len(),
        6
    );
    for _ in 0..32 {
        doubles_tokens(&mut state, me());
    }
    let before = state.zones.list(ZoneLocation::Battlefield).len();
    assert!(create_tokens(&mut state, me(), token, None, 1).is_empty());
    assert_eq!(state.zones.list(ZoneLocation::Battlefield).len(), before);
    assert!(state.numeric_failure.is_some());
}

#[test]
fn token_copy_exceptions_remove_defining_abilities_and_keep_other_subtypes() {
    use baylee_cards_dsl::{AbilityDef, CopyMod, KeywordSet, Modifier, PtCount, StaticAbility};
    use baylee_core::{
        generated::subtypes,
        types::{SubtypeSet, TypeSet},
    };
    let mut state = state();
    let source = the_forest(&state);
    let object = state.object_mut(source).unwrap();
    let base = object.base_mut();
    base.types = TypeSet::CREATURE.union(TypeSet::LAND);
    base.subtypes =
        SubtypeSet::ALL_CREATURE.union(SubtypeSet::from_slice(&[subtypes::land::DESERT]));
    base.keywords = KeywordSet::CHANGELING;
    object.take_abilities(crate::object::AbilityList::from_static(
        &[AbilityDef::Static(StaticAbility {
            source_zone: baylee_cards_dsl::ZoneRef::Battlefield,
            layer: baylee_cards_dsl::Layer::PtCda,
            filter: Filter::This,
            condition: None,
            modifier: Modifier::CharacteristicPT {
                count: PtCount::YouControl(&Filter::LAND),
                toughness_plus: 0,
            },
        })],
        None,
        None,
    ));
    resolve(
        &mut state,
        &[source],
        vec![Effect::CreateTokenCopyOfTarget {
            mods: &[
                CopyMod::SetPT(4, 4),
                CopyMod::SetCreatureTypes(&[subtypes::creature::ZOMBIE]),
            ],
            sacrifice_at_next_end_step: false,
        }],
    );
    let copy = *state.zones.list(ZoneLocation::Battlefield).last().unwrap();
    assert_ne!(copy, source);
    let object = state.object(copy).unwrap();
    assert_eq!(
        (object.base.power, object.base.toughness),
        (Some(4), Some(4))
    );
    assert!(!object.base.keywords.contains(KeywordSet::CHANGELING));
    assert_eq!(
        object.base.subtypes,
        SubtypeSet::from_slice(&[subtypes::creature::ZOMBIE, subtypes::land::DESERT])
    );
    assert!(
        state
            .printed_ability_list(copy)
            .unwrap()
            .abilities
            .is_empty()
    );
}

/// The factory is a **door** (CR 614.1): a token made beside it is
/// invisible to every replacement that multiplies tokens. And the rule
/// is read off whoever ends up with them — Crib Swap hands its
/// Shapeshifter to the player whose creature it answered, so my
/// Doubling Season must not double that one.
#[test]
fn the_token_factory_is_where_the_doubling_replacement_applies() {
    let mut state = state();
    doubles_tokens(&mut state, me());

    assert_eq!(
        create_tokens(&mut state, me(), treasure(), None, 1).len(),
        2
    );
    assert_eq!(
        create_tokens(&mut state, them(), treasure(), None, 1).len(),
        1,
        "the tokens are created under their control, not under mine"
    );
    assert_eq!(
        create_tokens(&mut state, me(), treasure(), None, 3).len(),
        6,
        "the multiplier applies to the whole creation"
    );
}

/// A token copy is a token, so the same replacement applies: Rite of
/// Replication under a Doubling Season makes two copies. That the three
/// copy branches read no replacement at all is why this is a second
/// door rather than one more argument to the first.
#[test]
fn a_token_copy_goes_through_the_same_replacement() {
    let mut state = state();
    doubles_tokens(&mut state, me());
    let forest = the_forest(&state);
    let base = std::sync::Arc::new(
        state
            .object(forest)
            .expect("the Forest")
            .characteristics()
            .clone(),
    );

    let copies = create_token_copies(&mut state, me(), forest, &base, 1);
    assert_eq!(copies.len(), 2);
    assert_eq!(
        create_token_copies(&mut state, them(), forest, &base, 1).len(),
        1
    );
}

/// CR 707.2 copies the original's abilities with its characteristics,
/// and a copy of a *card* owes the face whose printed abilities it still
/// carries — this crate has no card registry, so the face is queued for
/// whoever does.
#[test]
fn a_copy_of_a_card_queues_the_face_its_abilities_are_behind() {
    let mut state = state();
    let forest = the_forest(&state);
    let card = state
        .object(forest)
        .expect("the Forest")
        .card
        .expect("a card");
    let base = std::sync::Arc::new(
        state
            .object(forest)
            .expect("the Forest")
            .characteristics()
            .clone(),
    );
    state.pending_copied_faces.clear();

    let copies = create_token_copies(&mut state, me(), forest, &base, 2);

    assert_eq!(copies.len(), 2);
    assert_eq!(
        state.pending_copied_faces,
        vec![(copies[0], card.index, 0), (copies[1], card.index, 0)],
        "one entry per copy — a shared queue with one entry would leave \
         the second copy a blank permanent"
    );
    // A copy of a *token* has its definition instead, and owes no face.
    let treasures = create_tokens(&mut state, me(), treasure(), None, 1);
    state.pending_copied_faces.clear();
    let base = std::sync::Arc::new(
        state
            .object(treasures[0])
            .expect("a Treasure")
            .characteristics()
            .clone(),
    );
    let copy = create_token_copies(&mut state, me(), treasures[0], &base, 1);
    assert!(state.pending_copied_faces.is_empty());
    assert!(
        std::ptr::eq(
            state
                .object(copy[0])
                .expect("the copy")
                .token
                .expect("a definition"),
            treasure()
        ),
        "the copy of a Treasure is a Treasure: the definition is the \
         only record of which token this is once the characteristics \
         have been copied out of it"
    );
}

/// A token is not moved onto the battlefield — it is created there. But
/// CR 603.6a asks whether a permanent *entered* and not how it got
/// there, and the engine reads that off this one event, so a token that
/// arrived in silence was a board Nesting Dovehawk never saw.
///
/// `from` is `OutsideGame`, the only variant meaning no zone at all
/// (CR 400.1): the leaves-, dies- and exiled-from-battlefield triggers
/// all read that side and every one of them wants `Battlefield` there,
/// so naming a zone the token was never in would be both a lie and a
/// trigger.
#[test]
fn a_token_enters_the_battlefield_and_says_so() {
    let mut state = state();
    state.characteristics_generation = 0;
    let entries = state.journal.len();

    let tokens = create_tokens(&mut state, me(), treasure(), None, 1);
    let id = tokens[0];

    let obj = state.object(id).expect("just made it");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!(obj.controller, me());
    assert!(
        std::ptr::eq(obj.token.expect("a definition"), treasure()),
        "what makes it a Treasure rather than a blank artifact"
    );
    assert!(
        state.zones.list(ZoneLocation::Battlefield).contains(&id),
        "and the zone list agrees with the object"
    );
    assert_eq!(
        state.characteristics_generation,
        u64::MAX,
        "a permanent that just arrived has never been projected"
    );
    assert_eq!(state.journal.len(), entries + 1);
    assert!(
        matches!(
            state.journal.entries().last().expect("an entry").event,
            GameEvent::ZoneChanged {
                object,
                from: crate::zone::Zone::OutsideGame,
                to: crate::zone::Zone::Battlefield,
                ..
            } if object == id
        ),
        "recorded {:?}",
        state.journal.entries().last().expect("an entry").event
    );
}

/// An ability of seat 0's with `targets`, resolving `effects` in order.
fn resolve(state: &mut GameState, targets: &[ObjectId], effects: Vec<Effect>) {
    let mut res = Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: me(),
        effects,
        pc: 0,
        targets: SmallVec::from_slice(targets),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        text: TextChangeMap::IDENTITY,
        this_way_from: None,
        coin_stake: None,
    };
    assert!(matches!(run(state, &mut res), Flow::Complete));
    state.refresh_characteristics();
}

/// Crib Swap: "Exile target creature. Its controller creates a 1/1
/// colorless Shapeshifter creature token with changeling."
///
/// By the time the second sentence is read the creature is in exile, so
/// "its controller" is the controller it had as it last existed on the
/// battlefield (CR 608.2h), here the player who had taken it. The field
/// on the exiled card is no answer: it holds whatever the last refresh
/// left there, and a refresh that reaches into exile settles it to the
/// card's default, which after a steal is the player it was stolen from.
/// Any effect whose filter names another zone makes every refresh reach
/// every zone; Past in Flames' is the one used here.
#[test]
fn its_controller_is_the_one_the_creature_had_on_the_battlefield() {
    static FLASHBACK_REACH: Filter = Filter::And(&[
        Filter::INSTANT_OR_SORCERY,
        Filter::InZone(baylee_cards_dsl::ZoneRef::Graveyard),
        Filter::OwnedByYou,
    ]);
    let mut state = state();
    let name = state.names.intern("Creature");
    let it = state.create_bare(
        them(),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let obj = state.object_mut(it).expect("fresh");
    let mut base = (*obj.base).clone();
    base.types = baylee_core::types::TypeSet::CREATURE;
    obj.base = std::sync::Arc::new(base);
    state.invalidate_projections();
    crate::resolve::gain_control(&mut state, &[(it, me())]);
    resolve(
        &mut state,
        &[],
        vec![Effect::continuous(
            &FLASHBACK_REACH,
            baylee_cards_dsl::Modifier::GrantsFlashback,
            baylee_cards_dsl::Duration::UntilEndOfTurn,
        )],
    );
    let obj = state.object(it).expect("still there");
    assert_eq!((obj.owner, obj.controller), (them(), me()), "taken");

    let shapeshifter = &baylee_cards::tokens::SHAPESHIFTER_1_1_CHANGELING;
    resolve(
        &mut state,
        &[it],
        vec![
            Effect::exile(baylee_cards_dsl::TargetSpec::Object(&Filter::CREATURE)),
            Effect::CreateTokenForTargetController {
                token: shapeshifter,
            },
        ],
    );
    assert_eq!(
        state.object(it).map(|o| o.zone),
        Some(crate::zone::Zone::Exile)
    );
    let tokens: Vec<PlayerId> = state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter_map(|id| state.object(*id))
        .filter(|o| o.token.is_some_and(|t| std::ptr::eq(t, shapeshifter)))
        .map(|o| o.controller)
        .collect();
    assert_eq!(
        tokens,
        vec![me()],
        "one Shapeshifter, for the player who controlled the creature when it was exiled"
    );
}

/// A token is owned by the player it is created under (CR 111.2), and
/// nothing is created for a player who has left the game (CR 800.4b,
/// 800.4d): neither door makes one.
#[test]
fn no_token_is_created_for_a_player_who_has_left() {
    let mut state = state();
    let forest = the_forest(&state);
    let base = state.object(forest).expect("a Forest").base.clone();
    crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);

    assert!(create_tokens(&mut state, them(), treasure(), None, 1).is_empty());
    assert!(create_token_copies(&mut state, them(), forest, &base, 1).is_empty());
    assert_eq!(
        state.zones.list(ZoneLocation::Battlefield)[..],
        [forest],
        "the Forest alone"
    );
}

#[test]
fn per_count_token_records_its_ability_for_copying_and_later_text_changes() {
    use crate::text_changes::TextReplacement;
    use baylee_cards_dsl::TextWordKind;
    use baylee_core::color::{Color, ColorSet};
    static GREEN: Filter = Filter::HasColor(ColorSet::of(Color::Green));
    let mut state = state();
    let green = create_tokens(
        &mut state,
        me(),
        &baylee_cards::tokens::BOAR_2_2_GREEN,
        None,
        1,
    )[0];
    // The token's characteristic is a value. The green word in its
    // quoted static ability remains text and can change afterwards.
    resolve(
        &mut state,
        &[],
        vec![Effect::CreateTokenPtPerCount {
            token: &baylee_cards::tokens::CONSTRUCT_ARTIFACT_0_0,
            filter: &GREEN,
            p: 1,
            t: 1,
        }],
    );
    let construct = state
        .battlefield_seen()
        .find(|id| {
            state
                .printed_ability_list(*id)
                .is_some_and(|rules| !rules.runtime_statics().is_empty())
        })
        .unwrap();
    assert_eq!(
        state.object(construct).unwrap().characteristics().power,
        Some(1)
    );
    let identity = state.source_identity(construct).unwrap();
    state.text_changes.replace(
        identity,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 4,
            to: 1,
        },
    );
    state.invalidate_projections();
    state.refresh_characteristics();
    assert_eq!(
        state.object(construct).unwrap().characteristics().power,
        Some(0)
    );
    assert_eq!(
        state.object(green).unwrap().characteristics().colors,
        ColorSet::of(Color::Green)
    );
    let base = crate::layers::copiable_values(&state, construct).unwrap();
    let first = create_token_copies(&mut state, me(), construct, &base, 1)[0];
    let second = create_token_copies(&mut state, me(), first, &base, 1)[0];
    let rules = state.printed_ability_list(second).unwrap();
    assert_eq!(rules.runtime_statics().len(), 1);
    assert_eq!(
        rules.runtime_statics()[0].base_text,
        TextChangeMap::IDENTITY,
        "later Sleight on the original token is not copiable"
    );
}

#[test]
fn token_instruction_words_are_copiable_but_later_text_changes_are_not() {
    use crate::text_changes::{RuleContext, TextReplacement};
    use baylee_cards_dsl::{AbilityDef, CopyMod, TextWordKind, TokenDef};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::mana::ManaColor;
    use baylee_core::types::TypeSet;
    static TOKEN: TokenDef = TokenDef {
        name: "Green token",
        colors: ColorSet::of(Color::Green),
        types: TypeSet::CREATURE,
        power: Some(1),
        toughness: Some(1),
        abilities: &[baylee_cards_dsl::mana_ability!(&[Effect::mana(
            ManaColor::Green,
            1
        )])],
        ..TokenDef::DEFAULT
    };
    let mut state = state();
    let mut text = TextChangeMap::IDENTITY;
    text.replace(TextReplacement {
        kind: TextWordKind::Color,
        from: 4,
        to: 1,
    });
    let made = create_tokens_with_text(&mut state, me(), &TOKEN, None, 1, text)[0];
    let base = crate::layers::copiable_values(&state, made).unwrap();
    assert_eq!(base.colors, ColorSet::of(Color::Blue));
    assert_eq!(state.names.get(base.name), "Green token");
    let rules = state.printed_ability_list(made).unwrap();
    let AbilityDef::Activated { effects, .. } = rules.abilities[0] else {
        panic!("mana ability")
    };
    assert_eq!(
        effects,
        &[Effect::mana(ManaColor::Green, 1)],
        "a green mana symbol is not the color word green"
    );
    assert!(crate::eval::matches_with_context(
        &Filter::HasColor(ColorSet::of(Color::Green)),
        &state,
        state.object(made).unwrap(),
        me(),
        RuleContext {
            source: made,
            text: rules.base_text(0),
            resolved_name: None,
            reference: None,
        },
    ));
    let identity = state.source_identity(made).unwrap();
    state.text_changes.replace(
        identity,
        TextReplacement {
            kind: TextWordKind::Color,
            from: 1,
            to: 3,
        },
    );
    let copy = create_token_copies(&mut state, me(), made, &base, 1)[0];
    let copied_rules = state.printed_ability_list(copy).unwrap();
    assert_eq!(
        copied_rules.base_text(0).color_word(Color::Green),
        Color::Blue
    );
    assert_eq!(
        state.object(copy).unwrap().base.colors,
        ColorSet::of(Color::Blue)
    );
    let mut exception = (*base).clone();
    apply_copy_mod_with_text(
        &mut exception,
        &CopyMod::SetColor(ColorSet::of(Color::Green)),
        text,
    );
    assert_eq!(exception.colors, ColorSet::of(Color::Blue));
    apply_copy_mod_with_text(&mut exception, &CopyMod::KeepColor, text);
    assert_eq!(
        exception.colors,
        ColorSet::of(Color::Blue),
        "a retained color is a value"
    );
}

#[test]
fn populate_does_not_copy_a_created_tokens_entry_status_or_temporary_haste() {
    let mut state = state();
    let source = the_forest(&state);
    let created = create_tokens_entering(
        &mut state,
        me(),
        &baylee_cards::tokens::SPIRIT_1_1_COLORLESS,
        None,
        1,
        TextChangeMap::IDENTITY,
        TokenEntry {
            tapped: true,
            temporary_haste_source: Some(source),
        },
    );
    let original = created[0];
    assert!(
        state
            .object(original)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    populate(&mut state, me(), original);
    let copy = *state.zones.list(ZoneLocation::Battlefield).last().unwrap();
    assert_ne!(original, copy);
    assert!(
        !state
            .object(copy)
            .unwrap()
            .status
            .contains(crate::object::Status::TAPPED)
    );
    let copied = crate::layers::copiable_values(&state, copy).unwrap();
    assert!(
        !copied
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::HASTE)
    );
    assert!(
        !crate::layers::recompute(&state, state.object(copy).unwrap())
            .characteristics
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::HASTE)
    );
}
