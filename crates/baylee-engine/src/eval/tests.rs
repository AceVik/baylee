//! eval's tests: filters, amounts, conditions, protection and target options on a bare state.

use super::*;
use crate::object::ObjectKind;
use crate::state::CardLookup;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::KeywordSet;
use baylee_cards_dsl::{Amount, Condition, PlayerRel, ZoneSel};
use baylee_core::ids::{CardIndex, SubtypeId};
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
use baylee_core::types::{SubtypeSet, TypeSet};

/// No cards: these tests are about keywords, not about whichever
/// printed card happens to carry one.
struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

fn empty_state() -> GameState {
    let seat = || SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![],
        sideboard: vec![],
        commanders: vec![],
        starting_life: Some(20),
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GameState::from_preset(
        &GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 1,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: vec![seat(), seat()],
        },
        &NoCards,
    )
    .expect("an empty two-seat board")
}

fn creature(state: &mut GameState, controller: PlayerId, keywords: KeywordSet) -> ObjectId {
    let name = state.names.intern("Test Creature");
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let b = state.object_mut(id).expect("just created").base_mut();
    b.types = TypeSet::CREATURE;
    b.power = Some(1);
    b.toughness = Some(1);
    b.keywords = keywords;
    id
}

/// "That player" of a step trigger is whoever's turn it is (CR 102.1),
/// read from the state alone; the enchanted permanent's controller needs
/// the source and is the resolution's to answer.
#[test]
fn the_active_player_is_whoever_s_turn_it_is() {
    let mut state = empty_state();
    state.turn.active = P1;
    assert_eq!(players(PlayerRel::ActivePlayer, &state, P0), Some(vec![P1]));
    state.turn.active = P0;
    assert_eq!(players(PlayerRel::ActivePlayer, &state, P1), Some(vec![P0]));
    assert_eq!(players(PlayerRel::ControllerOfAttached, &state, P0), None);

    let host = land(&mut state, P1, &[]);
    let aura = creature(&mut state, P0, KeywordSet::EMPTY);
    assert_eq!(
        controller_of_attached(&state, aura),
        None,
        "attached to nothing"
    );
    state.object_mut(aura).expect("just made").attached_to = Some(host);
    assert_eq!(
        controller_of_attached(&state, aura),
        Some(P1),
        "the host's controller, not the Aura's (CR 303.4e)"
    );
}

fn buried_card(state: &mut GameState, types: TypeSet) -> ObjectId {
    let name = state.names.intern("Graveyard fixture");
    let id = state.create_bare(P0, ObjectKind::Card, name, ZoneLocation::Graveyard(P0));
    let object = state.object_mut(id).expect("created card");
    object.card = Some(crate::object::CardRef {
        index: CardIndex::new(1),
        print: baylee_core::ids::PrintRef::new(0),
    });
    object.base_mut().types = types;
    id
}

#[test]
fn graveyard_cards_above_counts_only_matching_cards_above_the_source() {
    let mut state = empty_state();
    for _ in 0..3 {
        buried_card(&mut state, TypeSet::CREATURE);
    }
    let source = buried_card(&mut state, TypeSet::CREATURE);
    let condition = Condition::GraveyardCardsAbove(&ANY_CREATURE, 3);
    assert!(!condition_holds(&state, P0, source, condition));
    for _ in 0..2 {
        buried_card(&mut state, TypeSet::CREATURE);
    }
    buried_card(&mut state, TypeSet::LAND);
    let token = buried_card(&mut state, TypeSet::CREATURE);
    state.object_mut(token).expect("token fixture").card = None;
    let copy = buried_card(&mut state, TypeSet::CREATURE);
    state
        .object_mut(copy)
        .expect("copy fixture")
        .riders
        .push(crate::object::Rider::SpellCopy);
    assert!(!condition_holds(&state, P0, source, condition));
    buried_card(&mut state, TypeSet::CREATURE);
    assert!(condition_holds(&state, P0, source, condition));
    assert!(!condition_holds(&state, P1, source, condition));
    state
        .move_object(
            source,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(!condition_holds(
        &state,
        P0,
        source,
        Condition::GraveyardCardsAbove(&ANY_CREATURE, 0)
    ));
}

#[test]
fn graveyard_order_condition_cannot_find_a_later_incarnation() {
    let mut state = empty_state();
    let source = buried_card(&mut state, TypeSet::CREATURE);
    for _ in 0..3 {
        buried_card(&mut state, TypeSet::CREATURE);
    }
    let condition = Some(Condition::All(&[Condition::GraveyardCardsAbove(
        &Filter::CREATURE,
        3,
    )]));
    let version = state.object(source).expect("source").version;
    assert!(intervening_if_for_incarnation(
        &state,
        condition,
        P0,
        source,
        Some(version),
        None,
        None
    ));
    state
        .move_object(
            source,
            ZoneLocation::Exile(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Bottom,
            crate::event::Cause::Effect,
        )
        .unwrap();
    assert!(
        intervening_if(&state, condition, P0, source),
        "the new card independently satisfies the clause"
    );
    assert!(
        !intervening_if_for_incarnation(&state, condition, P0, source, Some(version), None, None),
        "the old trigger cannot find that new card"
    );
}

/// A land on `controller`'s battlefield carrying `subtypes`.
fn land(state: &mut GameState, controller: PlayerId, subtypes: &[SubtypeId]) -> ObjectId {
    let name = state.names.intern("Test Land");
    let id = state.create_bare(
        controller,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let b = state.object_mut(id).expect("just created").base_mut();
    b.types = TypeSet::LAND;
    b.subtypes = SubtypeSet::from_slice(subtypes);
    id
}

/// Domain counts **types**, and that is the whole reason it is a variant
/// of its own rather than either of the two amounts that were already
/// there. `Amount::CountOf` counts objects, so two Forests would answer
/// 2 where the card wants 1 and a Tundra 1 where it wants 2; a land is
/// colourless, so `Amount::DistinctColorsAmong` answers 0 for any board
/// of them. Both readings are asserted here beside the right one, so a
/// later simplification back onto either is a red test rather than a
/// card that quietly pumps by the wrong number.
#[test]
fn a_domain_count_is_over_basic_land_types_and_not_over_lands() {
    use baylee_core::generated::subtypes::land;
    static DOMAIN: Amount = Amount::BasicLandTypesAmong(&Filter::YOUR_LAND);
    static YOUR_LANDS: Filter = Filter::YOUR_LAND;

    let mut state = empty_state();
    let forest = land(&mut state, P0, &[land::FOREST]);
    let count = |state: &GameState| self::amount(&DOMAIN, state, P0, forest, None);

    assert_eq!(count(&state), 1, "one Forest is one basic land type");
    land(&mut state, P0, &[land::FOREST]);
    assert_eq!(
        count(&state),
        1,
        "a second Forest adds no type — this is where a count of objects parts company"
    );
    land(&mut state, P0, &[land::PLAINS, land::ISLAND]);
    assert_eq!(count(&state), 3, "a Tundra is two types on one land");
    land(&mut state, P1, &[land::MOUNTAIN, land::SWAMP]);
    assert_eq!(
        count(&state),
        3,
        "and the filter is `lands you control`: an opponent's Badlands is not yours"
    );

    // The two amounts this is not, over the same board.
    assert_eq!(
        self::amount(
            &Amount::CountOf {
                filter: &YOUR_LANDS,
                zone: ZoneSel::Battlefield
            },
            &state,
            P0,
            forest,
            None
        ),
        3,
        "three lands, four types — a count of objects is not domain"
    );
    assert_eq!(
        self::amount(
            &Amount::DistinctColorsAmong(&YOUR_LANDS),
            &state,
            P0,
            forest,
            None
        ),
        0,
        "and a land is colourless, whatever mana it makes"
    );

    // CR 305.6 names five and no other land type joins them.
    land(&mut state, P0, &[land::DESERT, land::GATE]);
    assert_eq!(
        count(&state),
        3,
        "Desert and Gate are land types (CR 205.3i) and not basic ones"
    );
}

/// `Filter::CmcAtMostX` reads its bound off the **source**, which is the
/// only place the announced number exists at match time: a filter is
/// evaluated with `this` in hand and with no `x` beside it. The pool
/// cards that print "mana value X or less" are searches, so the bound
/// being wrong is a tutor that finds a card it may not find.
///
/// Zero is asserted beside the two live bounds because that is the
/// answer for an ability that announced nothing, and it has to *mean*
/// zero: a filter that could not read its bound and matched everything
/// would be a tutor with no price at all.
#[test]
fn a_mana_value_bound_of_x_is_read_off_the_ability_source() {
    static WITHIN_X: Filter = Filter::CmcAtMostX;
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let free = creature(&mut state, P0, KeywordSet::EMPTY);
    let two = creature(&mut state, P0, KeywordSet::EMPTY);
    state
        .object_mut(two)
        .expect("just created")
        .base_mut()
        .mana_cost = baylee_core::mana::ManaCost::parse("{1}{G}");

    let reaches = |state: &GameState, id: ObjectId| {
        let obj = state.object(id).expect("on the battlefield");
        matches(&WITHIN_X, state, obj, P0, source)
    };

    assert!(
        reaches(&state, free),
        "X is 0 and a cost of nothing is 0 or less"
    );
    assert!(
        !reaches(&state, two),
        "and a two-drop is not — an unreadable bound that matched would              be a tutor with no price"
    );

    state.object_mut(source).expect("just created").x_value = 2;
    assert!(reaches(&state, two), "X = 2 reaches a two-drop");

    state.object_mut(source).expect("just created").x_value = 1;
    assert!(
        !reaches(&state, two),
        "and X = 1 does not: the bound is the announced number and not              merely whether one was announced"
    );
}

/// Who `chooser` may point a creature-targeting spell at.
fn targets(state: &GameState, chooser: PlayerId, source: ObjectId) -> Vec<ObjectId> {
    target_options(&TargetSpec::Object(&ANY_CREATURE), state, chooser, source)
}

/// Gives `object` protection from `filter`, as a continuous effect the
/// seat `controller` controls.
fn protect(state: &mut GameState, controller: PlayerId, object: ObjectId, filter: &'static Filter) {
    let filter_of = crate::effects::EffectFilter::object(state, object);
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(object),
        controller,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Modifier::ProtectionFrom(filter).layer(),
        timestamp: 1,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: filter_of,
        modifier: baylee_cards_dsl::Modifier::ProtectionFrom(filter),
    });
}

/// Protection (CR 702.16) is two questions and both are asked of a
/// different object: which permanent *has* it, and whether the thing
/// coming at it matches the filter. An effect naming one creature does
/// not protect the one beside it, and a creature with protection from
/// artifacts is not protected from a creature.
#[test]
fn protection_is_read_off_the_protected_object_and_the_thing_it_faces() {
    static ARTIFACTS: Filter = Filter::ARTIFACT;
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let beside = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

    assert!(
        !protected_from(&state, mine, theirs),
        "nothing is protected from anything to begin with"
    );
    protect(&mut state, P0, mine, &ANY_CREATURE);
    assert!(protected_from(&state, mine, theirs));
    assert!(
        protected_from(&state, mine, beside),
        "protection from creatures is from all of them, mine included"
    );
    assert!(
        !protected_from(&state, beside, theirs),
        "and the creature standing next to it has none"
    );

    let other = creature(&mut state, P0, KeywordSet::EMPTY);
    protect(&mut state, P0, other, &ARTIFACTS);
    assert!(
        !protected_from(&state, other, theirs),
        "protection from artifacts is not protection from a creature"
    );
}

/// A choice that does not target (CR 115.10a), such as what a Clone
/// enters as a copy of (CR 707.5), is offered what targeting refuses:
/// an opponent's hexproof creature, a creature with shroud and one with
/// protection from creatures are all off `target_options` for a
/// creature source, and all on `choice_options` (the copy question's
/// list, `copy_on_enter_question`). Red when the choice borrows the
/// targeting filter, as the copy question did.
#[test]
fn a_choice_that_does_not_target_ignores_hexproof_shroud_and_protection() {
    let mut state = empty_state();
    let clone = creature(&mut state, P0, KeywordSet::EMPTY);
    let plain = creature(&mut state, P1, KeywordSet::EMPTY);
    let hexproof = creature(&mut state, P1, KeywordSet::HEXPROOF);
    let shroud = creature(&mut state, P1, KeywordSet::SHROUD);
    let protected = creature(&mut state, P1, KeywordSet::EMPTY);
    protect(&mut state, P1, protected, &ANY_CREATURE);
    let spec = TargetSpec::Object(&ANY_CREATURE);

    let targeted = targets(&state, P0, clone);
    assert!(targeted.contains(&plain));
    for refused in [hexproof, shroud, protected] {
        assert!(!targeted.contains(&refused), "a target: {refused:?}");
    }
    let chosen = choice_options(&spec, &state, P0, clone);
    for offered in [plain, hexproof, shroud, protected] {
        assert!(chosen.contains(&offered), "not a target: {offered:?}");
    }
}

/// **Whose opponent** is read off the seat that controls the protection
/// effect and not off the permanent that carries it (CR 109.5: "you" is
/// the controller of the ability). The two are the same on every printed
/// card, which is exactly why a reader that used the permanent's
/// controller would look right — so the case that tells them apart is
/// the one worth writing down.
#[test]
fn protection_from_an_opponent_means_the_effects_controllers_opponent() {
    static OPPONENTS: Filter = Filter::ControlledByOpponent;
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let beside = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

    protect(&mut state, P0, mine, &OPPONENTS);
    assert!(protected_from(&state, mine, theirs));
    assert!(
        !protected_from(&state, mine, beside),
        "a creature I control is not one my opponent controls"
    );

    // The same protection, granted by the seat across the table: the
    // filter now reads from their side, so it is my own creature the
    // permanent is protected from.
    let odd = creature(&mut state, P0, KeywordSet::EMPTY);
    protect(&mut state, P1, odd, &OPPONENTS);
    assert!(
        protected_from(&state, odd, beside),
        "from their seat, a creature I control is an opponent's"
    );
    assert!(
        !protected_from(&state, odd, theirs),
        "and their own creature is not"
    );
}

/// An object that is not in the arena is protected from nothing and
/// protects against nothing: both halves answer `false` rather than
/// panicking, which is what lets a damage step ask about a creature that
/// a state-based action has already taken away.
#[test]
fn protection_asked_about_something_that_is_gone_is_simply_false() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
    protect(&mut state, P0, mine, &ANY_CREATURE);
    assert!(protected_from(&state, mine, theirs));

    let gone = ObjectId::new(9_999, 0);
    assert!(!protected_from(&state, gone, theirs));
    assert!(!protected_from(&state, mine, gone));
}

/// Hexproof (CR 702.11b) stops opponents and nobody else.
#[test]
fn hexproof_hides_a_creature_from_its_controllers_opponents() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::HEXPROOF);
    let plain = creature(&mut state, P0, KeywordSet::EMPTY);
    let source = creature(&mut state, P1, KeywordSet::EMPTY);

    let theirs = targets(&state, P1, source);
    assert!(
        !theirs.contains(&mine),
        "an opponent could target a hexproof creature"
    );
    assert!(
        theirs.contains(&plain),
        "the opponent lost sight of an ordinary creature too"
    );

    assert!(
        targets(&state, P0, mine).contains(&mine),
        "hexproof stopped its own controller"
    );
}

/// Shroud (CR 702.18a) stops everyone, controller included — that is
/// the whole difference between the two keywords.
#[test]
fn shroud_hides_a_creature_from_everyone() {
    let mut state = empty_state();
    let shrouded = creature(&mut state, P0, KeywordSet::SHROUD);
    let source = creature(&mut state, P1, KeywordSet::EMPTY);

    assert!(!targets(&state, P1, source).contains(&shrouded));
    assert!(
        !targets(&state, P0, shrouded).contains(&shrouded),
        "shroud let its own controller target it"
    );
}

/// Both keywords are battlefield-only (CR 702.11b): a card printed
/// with hexproof is an ordinary target in any other zone.
#[test]
fn hexproof_does_not_function_outside_the_battlefield() {
    let mut state = empty_state();
    let card = creature(&mut state, P0, KeywordSet::HEXPROOF);
    state
        .move_object(
            card,
            ZoneLocation::Graveyard(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::TurnBased,
        )
        .expect("the creature dies");
    assert!(
        !untargetable_by(&state, card, P1),
        "hexproof kept working in the graveyard"
    );
}

/// `LacksType(t)` and `Not(&HasType(t))` are the same predicate, which
/// is what lets the pool spell "not a creature" exactly one way.
///
/// The two arms of the `match` above are literal negations of each
/// other, so this could be read off the source — and that is precisely
/// why it is worth a test: the equivalence is an *invariant* the card
/// pool now depends on, not an accident of how those two lines happen
/// to be written today. Every type, and both answers, so a rule that
/// stopped negating would fail here rather than in a card.
#[test]
fn lacking_a_type_is_not_having_it() {
    // Written out in pairs rather than built in the loop: `Not` holds a
    // `&'static Filter`, and a reference taken to a value made from a
    // loop variable cannot be promoted (E0716). The pairing is the
    // assertion, so writing it is no loss.
    static NEGATIONS: &[(TypeSet, Filter, Filter)] = &[
        (
            TypeSet::ARTIFACT,
            Filter::LacksType(TypeSet::ARTIFACT),
            Filter::Not(&Filter::HasType(TypeSet::ARTIFACT)),
        ),
        (
            TypeSet::CREATURE,
            Filter::LacksType(TypeSet::CREATURE),
            Filter::Not(&Filter::HasType(TypeSet::CREATURE)),
        ),
        (
            TypeSet::ENCHANTMENT,
            Filter::LacksType(TypeSet::ENCHANTMENT),
            Filter::Not(&Filter::HasType(TypeSet::ENCHANTMENT)),
        ),
        (
            TypeSet::INSTANT,
            Filter::LacksType(TypeSet::INSTANT),
            Filter::Not(&Filter::HasType(TypeSet::INSTANT)),
        ),
        (
            TypeSet::KINDRED,
            Filter::LacksType(TypeSet::KINDRED),
            Filter::Not(&Filter::HasType(TypeSet::KINDRED)),
        ),
        (
            TypeSet::LAND,
            Filter::LacksType(TypeSet::LAND),
            Filter::Not(&Filter::HasType(TypeSet::LAND)),
        ),
        (
            TypeSet::PLANESWALKER,
            Filter::LacksType(TypeSet::PLANESWALKER),
            Filter::Not(&Filter::HasType(TypeSet::PLANESWALKER)),
        ),
        (
            TypeSet::SORCERY,
            Filter::LacksType(TypeSet::SORCERY),
            Filter::Not(&Filter::HasType(TypeSet::SORCERY)),
        ),
        (
            TypeSet::BATTLE,
            Filter::LacksType(TypeSet::BATTLE),
            Filter::Not(&Filter::HasType(TypeSet::BATTLE)),
        ),
    ];

    let mut state = empty_state();
    let obj = creature(&mut state, P0, KeywordSet::EMPTY);
    let object = state.object(obj).expect("just created");
    for (t, lacks, negated) in NEGATIONS {
        assert_eq!(
            matches(lacks, &state, object, P0, obj),
            matches(negated, &state, object, P0, obj),
            "LacksType and Not(HasType) disagreed about {t:?}"
        );
    }

    // The counter-test: the object really is a creature and really is
    // not a land, so the loop above compared both answers and not one
    // answer nine times.
    assert!(matches(&Filter::CREATURE, &state, object, P0, obj));
    assert!(matches(&Filter::NONLAND, &state, object, P0, obj));
    assert!(!matches(&Filter::NONCREATURE, &state, object, P0, obj));
}

/// The `None` is the whole point of the signature. Answering `vec![]`
/// for a relation this function cannot resolve reads exactly like "no
/// seats matched", and that is what shipped Abraded Bluffs as a land
/// that deals no damage and Bojuka Bog as a swamp: only a caller
/// holding a `Resolution` can answer these two, and it must not be able
/// to swallow them silently.
#[test]
fn a_relation_the_state_cannot_answer_is_none_rather_than_nobody() {
    let state = empty_state();
    assert_eq!(players(PlayerRel::ControllerOfTarget, &state, P0), None);
    assert_eq!(players(PlayerRel::Chosen, &state, P0), None);

    assert_eq!(players(PlayerRel::You, &state, P0), Some(vec![P0]));
    assert_eq!(players(PlayerRel::Opponent, &state, P0), Some(vec![P1]));
    assert_eq!(
        players(PlayerRel::EachPlayer, &state, P0),
        Some(vec![P0, P1])
    );
    assert_eq!(
        players(PlayerRel::EachOpponent, &state, P1),
        Some(vec![P0]),
        "the relation is read from whoever is asking"
    );
}

/// A player who has lost is not a player the game asks anything of —
/// "each opponent loses 1 life" resolving after somebody conceded must
/// not find them.
#[test]
fn a_player_who_has_lost_is_no_longer_each_player() {
    let mut state = empty_state();
    state.players[1].loss = Some(crate::event::LossReason::Conceded);
    assert_eq!(players(PlayerRel::EachPlayer, &state, P0), Some(vec![P0]));
    assert_eq!(players(PlayerRel::EachOpponent, &state, P0), Some(vec![]));
    assert_eq!(
        players(PlayerRel::You, &state, P0),
        Some(vec![P0]),
        "and you are still you"
    );
}

/// "If you control three or more creatures" is about the asker's own
/// board, whoever else has one.
#[test]
fn a_control_count_counts_the_askers_own_permanents() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    creature(&mut state, P0, KeywordSet::EMPTY);
    creature(&mut state, P1, KeywordSet::EMPTY);

    let two = Condition::ControlCount(&ANY_CREATURE, 2);
    let three = Condition::ControlCount(&ANY_CREATURE, 3);
    assert!(condition_holds(&state, P0, mine, two));
    assert!(
        !condition_holds(&state, P0, mine, three),
        "theirs is theirs"
    );
    assert!(!condition_holds(&state, P1, mine, two));
}

/// A power comparison reads the **projected** number, and an object
/// with no power at all is not "a creature with power 4 or greater".
///
/// Six cards in this pool print a power or toughness comparison and
/// every one of them had the ability that states it taken off the card
/// while `Filter` could not say it — Bonders' Enclave was a land with
/// no draw, Access Tunnel a land with one ability. The `is_some_and` is
/// the half that is easy to get wrong in the other direction: a land
/// answering `0 <= 3` would make Access Tunnel target one.
#[test]
fn a_power_comparison_reads_a_projected_number_and_an_object_that_has_none_is_not_small() {
    let mut state = empty_state();
    let c = creature(&mut state, P0, KeywordSet::EMPTY);
    let l = land(&mut state, P0, &[]);
    let ask = |state: &GameState, f: &Filter, id: ObjectId| {
        matches(f, state, state.object(id).expect("still here"), P0, id)
    };

    // A 1/1 to begin with.
    assert!(ask(&state, &Filter::PowerAtMost(3), c));
    assert!(!ask(&state, &Filter::PowerAtLeast(4), c));
    assert!(!ask(&state, &Filter::ToughnessAtLeast(4), c));

    // The land has neither number, so it is on **no** side of either
    // comparison — the bound is not a default of nought.
    assert!(!ask(&state, &Filter::PowerAtMost(3), l));
    assert!(!ask(&state, &Filter::PowerAtLeast(4), l));
    assert!(!ask(&state, &Filter::ToughnessAtLeast(4), l));
    assert!(
        !ask(&state, &Filter::ToughnessAtMost(3), l),
        "the predicate that was already here answers the same way, \
             which is what makes the three new ones its siblings"
    );

    // Grown to a 4/4 — through the base characteristics, because this
    // module tests the predicate and not the layer system. A card that
    // reached 4 power under an anthem reaches it here the same way,
    // since `matches` is handed the projected characteristics either
    // way.
    {
        let b = state.object_mut(c).expect("still here").base_mut();
        b.power = Some(4);
        b.toughness = Some(4);
    }
    state.invalidate_projections();
    assert!(ask(&state, &Filter::PowerAtLeast(4), c));
    assert!(ask(&state, &Filter::ToughnessAtLeast(4), c));
    assert!(
        !ask(&state, &Filter::PowerAtMost(3), c),
        "and the creature has grown out of the other card's restriction"
    );
}

/// "Activate only during combat" holds in each step of the combat phase
/// (CR 506.1), whoever's turn it is, and in no other phase.
#[test]
fn during_combat_is_the_combat_phase_of_any_turn() {
    use crate::turn::{Phase, Step};
    let mut state = empty_state();
    let this = creature(&mut state, P0, KeywordSet::EMPTY);
    let during = |state: &GameState| condition_holds(state, P0, this, Condition::DuringCombat);
    for (phase, step, holds) in [
        (Phase::FirstMain, Step::Main, false),
        (Phase::Combat, Step::CombatBegin, true),
        (Phase::Combat, Step::DeclareBlockers, true),
        (Phase::Combat, Step::CombatEnd, true),
        (Phase::SecondMain, Step::Main, false),
        (Phase::Ending, Step::End, false),
    ] {
        state.turn.phase = phase;
        state.turn.step = step;
        assert_eq!(during(&state), holds, "{step:?}");
    }
    state.turn.phase = Phase::Combat;
    state.turn.step = Step::DeclareAttackers;
    state.turn.active = P1;
    assert!(during(&state), "the opponent's combat too");
}

/// "For each creature that died this turn" reads the turn's tally, which
/// the move to a graveyard writes and the turn's end clears.
#[test]
fn the_turns_deaths_are_the_tally_the_moves_wrote() {
    let mut state = empty_state();
    let this = creature(&mut state, P0, KeywordSet::EMPTY);
    let died = Amount::CreaturesDiedThisTurn;
    assert_eq!(amount(&died, &state, P0, this, None), 0);
    for _ in 0..2 {
        let dying = creature(&mut state, P1, KeywordSet::EMPTY);
        state
            .move_object(
                dying,
                ZoneLocation::Graveyard(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("it moves");
    }
    assert_eq!(amount(&died, &state, P0, this, None), 2, "either player's");
}

/// "Toughness less than Stone Giant's power": compared with the
/// source's projected power, strictly, and a source with no power
/// bounds nothing in.
#[test]
fn a_comparison_with_the_sources_power_is_strict_and_reads_the_source() {
    let mut state = empty_state();
    let giant = creature(&mut state, P0, KeywordSet::EMPTY);
    let small = creature(&mut state, P0, KeywordSet::EMPTY);
    let land = land(&mut state, P0, &[]);
    {
        let b = state.object_mut(giant).expect("seated").base_mut();
        b.power = Some(3);
        b.toughness = Some(4);
    }
    {
        let b = state.object_mut(small).expect("seated").base_mut();
        b.power = Some(3);
        b.toughness = Some(2);
    }
    state.invalidate_projections();
    let ask = |state: &GameState, f: &Filter, id: ObjectId, source: ObjectId| {
        matches(f, state, state.object(id).expect("still here"), P0, source)
    };
    assert!(ask(
        &state,
        &Filter::ToughnessLessThanSourcePower,
        small,
        giant
    ));
    assert!(
        !ask(&state, &Filter::PowerLessThanSourcePower, small, giant),
        "3 is not less than 3"
    );
    assert!(
        !ask(&state, &Filter::ToughnessLessThanSourcePower, giant, giant),
        "the giant's own 4 is not less than its 3"
    );
    assert!(
        !ask(&state, &Filter::ToughnessLessThanSourcePower, small, land),
        "a source with no power bounds nothing in"
    );
    state.object_mut(giant).expect("seated").base_mut().power = Some(2);
    state.invalidate_projections();
    assert!(
        !ask(&state, &Filter::ToughnessLessThanSourcePower, small, giant),
        "the source's power as it is now"
    );
}

/// "If no creatures are on the battlefield" (Pestilence) counts both
/// sides of the table, where "you control" counts one.
#[test]
fn a_battlefield_count_is_everybodys() {
    let mut state = empty_state();
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

    let none = Condition::BattlefieldCountAtMost(&ANY_CREATURE, 0);
    let some = Condition::BattlefieldCount(&ANY_CREATURE, 1);
    assert!(!condition_holds(&state, P0, theirs, none), "theirs counts");
    assert!(condition_holds(&state, P0, theirs, some));
    assert!(
        condition_holds(
            &state,
            P0,
            theirs,
            Condition::ControlCountAtMost(&ANY_CREATURE, 0)
        ),
        "the control: P0 controls none of it"
    );
    let bare = empty_state();
    assert!(condition_holds(&bare, P0, theirs, none));
    assert!(!condition_holds(&bare, P0, theirs, some));
}

/// Blaze of Glory's "creature defending player controls": during the
/// combat phase, a creature an opponent of the active player controls
/// (CR 506.2, 802.2), whoever `you` is; outside it, nobody's. A
/// teammate of the active player defends nothing.
#[test]
fn a_defending_players_creatures_are_the_active_players_opponents_in_combat() {
    static DEFENDING: Filter = Filter::ControlledByDefendingPlayer;
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
    let asks = |state: &GameState, id| {
        state
            .object(id)
            .is_some_and(|o| matches(&DEFENDING, state, o, P1, id))
    };
    state.turn.active = P0;
    state.turn.phase = crate::turn::Phase::FirstMain;
    assert!(!asks(&state, theirs), "no defending player outside combat");
    state.turn.phase = crate::turn::Phase::Combat;
    assert!(asks(&state, theirs), "whoever `you` is");
    assert!(!asks(&state, mine), "the attacking player defends nothing");
    state.turn.active = P1;
    assert!(asks(&state, mine) && !asks(&state, theirs));
    for player in &mut state.players {
        player.team = Some(1);
    }
    assert!(!asks(&state, mine), "a teammate is not attacked");
}

/// Karma's "Swamps they control" is the active player's, and Spell
/// Blast's "mana value X" is the X its own source announced.
#[test]
fn the_active_players_permanents_and_a_mana_value_of_exactly_x() {
    static ACTIVES: Filter = Filter::ControlledByActivePlayer;
    static EXACTLY_X: Filter = Filter::CmcExactlyX;
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
    state.turn.active = P1;
    let asks = |state: &GameState, id, filter: &Filter| {
        state
            .object(id)
            .is_some_and(|o| matches(filter, state, o, P0, id))
    };
    assert!(asks(&state, theirs, &ACTIVES), "whoever `you` is");
    assert!(!asks(&state, mine, &ACTIVES));
    state.turn.active = P0;
    assert!(asks(&state, mine, &ACTIVES));

    let spell = creature(&mut state, P0, KeywordSet::EMPTY);
    let value = state
        .object(mine)
        .map_or(0, |o| o.characteristics().mana_value());
    let check = |state: &GameState| {
        state
            .object(mine)
            .is_some_and(|o| matches(&EXACTLY_X, state, o, P0, spell))
    };
    state.object_mut(spell).expect("here").x_value = value;
    assert!(check(&state));
    state.object_mut(spell).expect("here").x_value = value + 1;
    assert!(!check(&state), "at most X would have said yes");
}

/// "You control no artifacts" is not the negation of a minimum with the
/// number moved: it is a different comparison, and Glimmervoid and
/// Thran Quarry both shipped with the clause **off** while it could not
/// be said — which is a land that never sacrifices itself.
#[test]
fn a_control_count_downwards_is_a_different_question_from_one_upwards() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);

    let none = Condition::ControlCountAtMost(&ANY_CREATURE, 0);
    let one = Condition::ControlCountAtMost(&ANY_CREATURE, 1);
    assert!(!condition_holds(&state, P0, mine, none));
    assert!(condition_holds(&state, P0, mine, one));
    assert!(
        condition_holds(&state, P1, mine, none),
        "the other seat controls none of it"
    );

    // On a board with none of them the sentence turns over, and its
    // upward twin turns over with it — the two really are reading one
    // count and not two.
    let bare = empty_state();
    assert!(condition_holds(&bare, P0, mine, none));
    assert!(!condition_holds(
        &bare,
        P0,
        mine,
        Condition::ControlCount(&ANY_CREATURE, 1)
    ));
}

/// "An opponent controls four or more lands" is `any` over the seats
/// and never your own board: Tectonic Edge had been activating on a
/// table where nobody else had a land at all, which is a Wasteland.
#[test]
fn an_opponent_control_count_is_any_other_seat_and_never_your_own() {
    let mut state = empty_state();
    let mine = creature(&mut state, P0, KeywordSet::EMPTY);
    creature(&mut state, P0, KeywordSet::EMPTY);
    let two = Condition::OpponentControlCount(&ANY_CREATURE, 2);

    assert!(
        !condition_holds(&state, P0, mine, two),
        "two of your own are not two of an opponent's"
    );
    assert!(
        condition_holds(&state, P1, mine, two),
        "and they are, asked from the other side"
    );

    creature(&mut state, P1, KeywordSet::EMPTY);
    creature(&mut state, P1, KeywordSet::EMPTY);
    assert!(condition_holds(&state, P0, mine, two));
}

/// Hellbent and Library of Alexandria are one word apart and never the
/// same board: "at most nought" is true of an empty hand and "exactly
/// seven" stops being true the moment the seventh card is drawn on.
#[test]
fn a_hand_size_is_counted_on_the_asking_seats_own_hand() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let deal = |state: &mut GameState, who: PlayerId, n: usize| {
        for _ in 0..n {
            let name = state.names.intern("Test Card");
            state.create_bare(who, ObjectKind::Card, name, ZoneLocation::Hand(who));
        }
    };

    let empty = Condition::HandSizeAtMost(0);
    let seven = Condition::HandSizeExactly(7);
    assert!(condition_holds(&state, P0, source, empty));
    assert!(!condition_holds(&state, P0, source, seven));

    deal(&mut state, P1, 7);
    assert!(
        condition_holds(&state, P0, source, empty),
        "their hand is not yours"
    );
    assert!(!condition_holds(&state, P0, source, seven));
    assert!(condition_holds(&state, P1, source, seven));

    deal(&mut state, P0, 7);
    assert!(!condition_holds(&state, P0, source, empty));
    assert!(condition_holds(&state, P0, source, seven));

    deal(&mut state, P0, 1);
    assert!(
        !condition_holds(&state, P0, source, seven),
        "the eighth card is what makes this land stop working"
    );
}

/// Two sentences that look alike and are not: "if it has three or more"
/// against "if it has exactly three". A card that removes counters as a
/// cost sits on both sides of that difference.
#[test]
fn at_least_and_exactly_are_different_questions() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    state
        .object_mut(source)
        .expect("just made it")
        .counters
        .add(baylee_cards_dsl::CounterKind::P1P1, 3);

    let kind = baylee_cards_dsl::CounterKind::P1P1;
    assert!(condition_holds(
        &state,
        P0,
        source,
        Condition::CountersOnSelf(kind, 3)
    ));
    assert!(condition_holds(
        &state,
        P0,
        source,
        Condition::CountersOnSelf(kind, 2)
    ));
    assert!(!condition_holds(
        &state,
        P0,
        source,
        Condition::CountersOnSelf(kind, 4)
    ));
    assert!(condition_holds(
        &state,
        P0,
        source,
        Condition::CountersOnSelfExactly(kind, 3)
    ));
    assert!(!condition_holds(
        &state,
        P0,
        source,
        Condition::CountersOnSelfExactly(kind, 2)
    ));
}

/// A leveler's `{LEVEL N1-N2}` band (CR 711.2a) is closed at both ends:
/// Hexdrinker's "LEVEL 3-7" holds at three and at seven and at neither
/// two nor eight.
#[test]
fn a_level_band_holds_at_both_ends_and_not_beyond_them() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let kind = baylee_cards_dsl::CounterKind::Level;
    let band = Condition::CountersOnSelfBetween(kind, 3, 7);
    let mut held = Vec::new();
    for _ in 0..9 {
        held.push(condition_holds(&state, P0, source, band));
        state
            .object_mut(source)
            .expect("just made it")
            .counters
            .add(kind, 1);
    }
    assert_eq!(
        held,
        [false, false, false, true, true, true, true, true, false],
        "levels 0 to 8"
    );
}

/// CR 113.7a: an ability is a separate object from its source the
/// moment it goes on the stack, so "if this land is tapped" asked of a
/// land that has left the battlefield has nothing to be true of.
#[test]
fn a_condition_about_a_source_that_is_gone_is_false() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let about_itself = Condition::SourceMatches(&ANY_CREATURE);
    assert!(condition_holds(&state, P0, source, about_itself));

    state.arena.remove(source);
    assert!(!condition_holds(&state, P0, source, about_itself));
    assert!(
        !condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(baylee_cards_dsl::CounterKind::P1P1, 0)
        ),
        "every other sentence about the source fails the same way, \
             including the one a zero would otherwise make trivially true"
    );
}

/// CR 603.4 asks the clause twice — once where the ability would
/// trigger and once where it would resolve — so both readings go
/// through one function. An ability that prints no clause at all is the
/// trivially true one, and an absent clause answering `false` would
/// silence every trigger in the pool that does not print one.
#[test]
fn an_ability_with_no_intervening_if_has_a_true_one() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);

    assert!(intervening_if(&state, None, P0, source));
    assert!(intervening_if(
        &state,
        Some(Condition::ControlCount(&ANY_CREATURE, 1)),
        P0,
        source
    ));
    assert!(!intervening_if(
        &state,
        Some(Condition::ControlCount(&ANY_CREATURE, 2)),
        P0,
        source
    ));
}

/// "Target spell or permanent" is the spells on the stack and the
/// permanents on the battlefield. An ability on the stack is neither
/// (CR 113.9, CR 110.1), and offering one let every Venser trigger aim
/// at the triggers stacked under it until the menu outgrew a `u8` (l29
/// game 1930). The spell is asserted beside it, so a fix that dropped
/// the stack half altogether is red too.
#[test]
fn a_spell_or_permanent_is_never_an_ability_on_the_stack() {
    static SPELL_OR_PERMANENT: TargetSpec = TargetSpec::StackOrBattlefield(&Filter::Any);
    let mut state = empty_state();
    let permanent = creature(&mut state, P1, KeywordSet::EMPTY);
    let name = state.names.intern("Test Spell");
    let spell = state.create_bare(P1, ObjectKind::Spell, name, ZoneLocation::Stack);
    let name = state.names.intern("Test Ability");
    let ability = state.create_bare(P1, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);

    let options = target_options(&SPELL_OR_PERMANENT, &state, P0, permanent);
    assert!(
        options.contains(&spell),
        "a spell on the stack: {options:?}"
    );
    assert!(
        options.contains(&permanent),
        "a permanent on the battlefield: {options:?}"
    );
    assert!(
        !options.contains(&ability),
        "an ability on the stack is no spell: {options:?}"
    );
    assert_eq!(options.len(), 2, "and nothing else: {options:?}");
}
#[test]
fn combat_participation_survives_removal_but_not_a_new_incarnation_or_combat() {
    let mut state = empty_state();
    let id = creature(&mut state, P0, KeywordSet::EMPTY);
    let version = state.object(id).unwrap().version;
    state.combat.participants.push((id, version));
    let participated =
        |s: &GameState| condition_holds(s, P0, id, Condition::AttackedOrBlockedThisCombat);
    assert!(participated(&state));
    state.combat.remove_from_combat(id);
    assert!(
        participated(&state),
        "removal cannot undo an attack or block declaration"
    );
    state.object_mut(id).unwrap().version += 1;
    assert!(
        !participated(&state),
        "a later incarnation did not participate"
    );
    state.combat.participants.push((id, version + 1));
    assert!(participated(&state));
    state.combat = crate::combat::CombatState::default();
    assert!(!participated(&state), "an additional combat starts fresh");
}

#[test]
fn losing_all_abilities_removes_intrinsic_land_mana_as_well_as_printed_mana() {
    let mut state = empty_state();
    let id = land(
        &mut state,
        P0,
        &[baylee_core::generated::subtypes::land::FOREST],
    );
    assert!(!crate::casting::intrinsic_mana_colors(&state, id).is_empty());
    assert!(matches(
        &Filter::HasManaAbility,
        &state,
        state.object(id).unwrap(),
        P0,
        id
    ));
    state.object_mut(id).unwrap().base_mut().abilities_lost = std::num::NonZeroU32::new(1);
    assert!(crate::casting::intrinsic_mana_colors(&state, id).is_empty());
    assert!(!matches(
        &Filter::HasManaAbility,
        &state,
        state.object(id).unwrap(),
        P0,
        id
    ));
}

/// "If your life total is less than your starting life total": below the
/// starting total the format gave this player (CR 103.4), read per player,
/// whatever that total was. Twenty-five of a forty-life start is below it.
#[test]
fn life_below_starting_reads_each_players_own_starting_total() {
    let mut state = empty_state();
    let source = creature(&mut state, P0, KeywordSet::EMPTY);
    let condition = Condition::LifeBelowStarting;
    assert!(!condition_holds(&state, P0, source, condition), "20 of 20");
    state.players[0].life = 19;
    assert!(condition_holds(&state, P0, source, condition));
    assert!(!condition_holds(&state, P1, source, condition), "the other seat");
    state.players[0].life = 25;
    assert!(!condition_holds(&state, P0, source, condition), "above it");
    state.players[0].starting_life = 40;
    assert!(condition_holds(&state, P0, source, condition), "a 40-life start");
}
