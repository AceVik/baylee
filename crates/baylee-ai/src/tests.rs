use super::*;
use baylee_cards_dsl::AbilityDef;
use baylee_core::color::ColorSet;
use baylee_core::ids::SeatSet;
use baylee_core::types::{SubtypeSet, SupertypeSet, TypeSet};
use baylee_engine::choice::{ArrangePile, ArrangePlace};
use baylee_view::{
    CombatView, CounterEntry, CounterKind, ObjectStatus, PlayerView, PublicObject, SeatView,
};

mod constrained_tests;
mod controlled_tests;
mod foresight_tests;
/// `worth`'s takes and declines, on this module's boards: a child of
/// it, so the helpers below serve both.
mod worth_tests;

mod activations_tests;
mod announced_cost_tests;
mod attacks_tests;
mod blocks_tests;
mod choices_tests;
mod combat_tests;
mod counters_tests;
mod ice_consumers_tests;
mod land_tests;
mod library_top_tests;
mod mana_tests;
mod modes_tests;
mod optional_tests;
mod payment_tests;
mod planning_tests;
mod spell_ending_tests;
mod stack_tests;
mod targets_tests;
mod team_tests;
mod timing_tests;
mod tower_supplies_tests;
mod x_target_tests;

fn obj(slot: u32) -> ObjectId {
    ObjectId::new(slot, 0)
}

fn hand_card(slot: u32, name: &str) -> baylee_view::HandObject {
    let index = baylee_cards::decks::by_name(name).unwrap();
    let face = &baylee_cards::by_index(index).unwrap().faces[0];
    baylee_view::HandObject {
        id: obj(slot),
        card: baylee_view::CardIdentity {
            index,
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        },
        name: name.into(),
        mana_value: face.mana_cost.cmc(),
        colors: face.mana_cost.colors(),
        types: face.types,
        commander: false,
    }
}

/// An instant on the stack, as the seat sees it, aimed at `at`.
fn stack_spell(id: u32, name: &str, controller: PlayerId, at: ObjectId) -> PublicObject {
    let mut o = permanent(obj(id), controller, 0);
    o.card = Some(hand_card(id, name).card);
    o.types = TypeSet::INSTANT;
    o.power = None;
    o.toughness = None;
    o.stack_item = Some(baylee_view::StackItem::Spell);
    o.targets = vec![baylee_client_core::test_support::target(at)];
    o
}

/// The five shipped profiles, so a rule is not proved on one of them.
const PROFILES: [(&str, AIProfile); 5] = [
    ("NOVICE", AIProfile::NOVICE),
    ("CASUAL", AIProfile::CASUAL),
    ("STEADY", AIProfile::STEADY),
    ("SHARP", AIProfile::SHARP),
    ("EXPERT", AIProfile::EXPERT),
];

const EVERY_PROFILE: [(&str, AIProfile); 5] = [
    ("NOVICE", AIProfile::NOVICE),
    ("CASUAL", AIProfile::CASUAL),
    ("STEADY", AIProfile::STEADY),
    ("SHARP", AIProfile::SHARP),
    ("EXPERT", AIProfile::EXPERT),
];

/// A default-profile agent at a table with no teams.
fn agent() -> HeuristicAgent {
    HeuristicAgent::new(AIProfile::default())
}

/// One permanent on the battlefield, as the seat sees it.
fn permanent(id: ObjectId, controller: PlayerId, power: i16) -> PublicObject {
    PublicObject {
        battle_protector: None,
        id,
        card: None,
        rules: None,
        prepared_spell: None,
        name: "Creature".into(),
        controller,
        owner: controller,
        commander: false,
        status: ObjectStatus::default(),
        types: TypeSet::CREATURE,
        supertypes: SupertypeSet::EMPTY,
        subtypes: SubtypeSet::EMPTY,
        chosen_subtype: None,
        second_chosen_subtype: None,
        chosen_card_type: None,
        chosen_number: None,
        chosen_name: None,
        chosen_opponent: None,
        chosen_color: None,
        chosen_x: None,
        linked_exile: Vec::new(),
        unlocked_doors: None,
        suspended: false,
        plotted: false,
        warped: false,
        suspected: false,
        token: None,
        colors: ColorSet::EMPTY,
        mana_value: 1,
        keywords: 0,
        power: Some(power),
        toughness: Some(power),
        base_power: None,
        base_toughness: None,
        loyalty: None,
        damage: 0,
        counters: vec![],
        attached_to: None,
        targets: vec![],
        stack_item: None,
        times_kicked: 0,
        bought_back: false,
        summoning_sick: false,
        ability_mana_spending: baylee_core::mana::ManaSpending::EXACT,
        granted_mana: None,
        board_mana: None,
        flashback: None,
        grants: Vec::new(),
        ability_sources: Vec::new(),
        word_changes: Vec::new(),
    }
}

/// A planeswalker with `loyalty` counters on it.
fn walker(id: ObjectId, controller: PlayerId, loyalty: u16) -> PublicObject {
    PublicObject {
        name: "Walker".into(),
        types: TypeSet::PLANESWALKER,
        power: None,
        toughness: None,
        base_power: None,
        base_toughness: None,
        loyalty: Some(loyalty),
        counters: vec![CounterEntry {
            kind: CounterKind::Loyalty,
            count: loyalty,
        }],
        ..permanent(id, controller, 0)
    }
}

/// Urza's Saga with `lore` lore counters on it, which is chapter `lore`.
///
/// A real registry card, for the reason [`carded`] gives and one more:
/// how many chapters a Saga has is printed on the card and nowhere in
/// the view, so the agent reads it back out of the pool.
fn saga(id: ObjectId, controller: PlayerId, lore: u16) -> PublicObject {
    let mut object = carded(
        permanent(id, controller, 0),
        "Urza's Saga",
        TypeSet::LAND.union(TypeSet::ENCHANTMENT),
    );
    object.name = "Urza's Saga".into();
    object.power = None;
    object.toughness = None;
    object.counters = vec![CounterEntry {
        kind: CounterKind::Lore,
        count: lore,
    }];
    object
}

/// A creature of `name` on `controller`'s side, `power`/`power`, wearing
/// the keyword bits a view would project onto it.
///
/// Written against the view's `keywords` and not the card's, because that
/// is what the agent reads and because a granted undying has to reach the
/// same rule as a printed one.
fn keyworded(
    id: ObjectId,
    controller: PlayerId,
    power: i16,
    name: &str,
    keywords: baylee_cards_dsl::KeywordSet,
) -> PublicObject {
    let mut object = carded(permanent(id, controller, power), name, TypeSet::CREATURE);
    object.name = name.into();
    object.keywords = keywords.bits();
    object
}

/// Ancestral Vision in exile with `time` time counters left on it —
/// suspend 4, and a free three-card draw when the last one comes off.
fn suspended(id: ObjectId, owner: PlayerId, time: u16) -> PublicObject {
    let mut object = carded(
        permanent(id, owner, 0),
        "Ancestral Vision",
        TypeSet::SORCERY,
    );
    object.name = "Ancestral Vision".into();
    object.power = None;
    object.toughness = None;
    object.counters = vec![CounterEntry {
        kind: CounterKind::Time,
        count: time,
    }];
    object
}

/// A view of `seats` (life totals) with `battlefield` on the table.
fn view(seat: u8, lives: &[i32], battlefield: Vec<PublicObject>) -> PlayerView {
    let seats: Vec<SeatView> = lives
        .iter()
        .enumerate()
        .map(|(i, life)| SeatView {
            player: PlayerId::new(i as u8),
            life: *life,
            poison: 0,
            energy: 0,
            speed: 0,
            rad: 0,
            hand_count: 0,
            no_max_hand_size: false,
            library_count: 40,
            graveyard_count: 0,
            loss: None,
            house_answered: None,
            mana_pool: baylee_view::ManaPoolView::default(),
            commanders: vec![],
            commander_damage: vec![],
        })
        .collect();
    PlayerView {
        divided: Vec::new(),
        damage_sources: Vec::new(),
        target_objects: battlefield
            .iter()
            .map(|o| {
                baylee_client_core::test_support::target_snapshot(
                    o,
                    baylee_view::LogZone::Battlefield,
                )
            })
            .collect(),
        seq: 7,
        seat: PlayerId::new(seat),
        turn: 3,
        phase: baylee_view::Phase::Combat,
        step: baylee_view::Step::DeclareAttackers,
        active: PlayerId::new(seat),
        awaiting: None,
        deciding: SeatSet::new(),
        decision_remaining_ms: None,
        clocks: Vec::new(),
        lost: Vec::new(),
        priority_held: false,
        policy_acts: Vec::new(),
        monarch: None,
        day_night: None,
        seats,
        hand: vec![],
        shared_hands: vec![],
        controlled_hands: vec![],
        decision_player: None,
        hand_shared_with: SeatSet::new(),
        hand_requests: SeatSet::new(),
        hand_requested: SeatSet::new(),
        battlefield,
        stack: vec![],
        graveyards: vec![vec![]; lives.len()],
        exile: vec![vec![]; lives.len()],
        command: vec![vec![]; lives.len()],
        combat: CombatView::default(),
        looking_at: Vec::new(),
        trigger_order: Vec::new(),
        library_tops: Vec::new(),
        owed: None,
        targeting: None,
        casting: None,
        announced_cast: None,
        announced_x: None,
        sorcery_lock: None,
        sorceries_have_flash: false,
        flash_types: baylee_core::types::TypeSet::EMPTY,
        controls: Vec::new(),
        foreign_targets: Vec::new(),
        ballots: Vec::new(),
        held_choices: Vec::new(),
    }
}

/// The same permanent, but backed by a real registry card so the
/// activation policy can read what its abilities cost and do.
///
/// Named, not numbered. An index is assigned over the whole card corpus
/// rather than over this pool, so the literal that was Arid Mesa is now
/// some other card — and a test that reads the abilities off whatever
/// landed there fails for a reason that has nothing to do with the agent.
///
/// Its abilities are printed on the same card, as the view says of every
/// object that is not a copy; [`copying`] is the one that is.
fn carded(mut object: PublicObject, name: &str, types: TypeSet) -> PublicObject {
    object.card = Some(baylee_view::CardIdentity {
        index: baylee_cards::decks::by_name(name).expect("a card of that name in the pool"),
        print: baylee_core::ids::PrintRef::new(0),
        face: 0,
    });
    object.rules = object.card.map(baylee_view::RulesFace::from);
    object.types = types;
    object
}

/// A carded object that has become a copy of `original` (CR 707.2): the
/// card underneath is still its own, and the abilities are the other's.
fn copying(object: PublicObject, original: &str) -> PublicObject {
    PublicObject {
        rules: Some(baylee_view::RulesFace {
            card: baylee_cards::decks::by_name(original).expect("a card of that name"),
            face: 0,
        }),
        ..object
    }
}

/// Priority with exactly these abilities on offer and nothing else.
fn offering(abilities: Vec<(ObjectId, u32)>) -> Pending {
    Pending::Priority {
        player: PlayerId::new(0),
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            abilities,
            ..Default::default()
        }),
    }
}

#[test]
fn chosen_and_linked_filters_are_read_from_generation_19_and_refused_before() {
    use baylee_cards_dsl::{Filter, ZoneRef};
    use baylee_core::color::{Color, ColorSet};
    use baylee_core::mana::ManaColor;

    let me = PlayerId::new(0);
    let mut source = permanent(obj(1), me, 2);
    source.chosen_card_type = Some(TypeSet::CREATURE);
    source.chosen_color = Some(ManaColor::Blue);
    source.linked_exile = vec![obj(10)];

    let mut exiled = permanent(obj(10), me, 0);
    exiled.name = "Island".into();
    exiled.types = TypeSet::LAND;
    exiled.colors = ColorSet::from_slice(&[Color::Blue]);

    let mut matching = permanent(obj(2), me, 2);
    matching.name = "Island".into();
    matching.colors = ColorSet::from_slice(&[Color::Blue]);

    let mut v = view(0, &[20, 20], vec![source, matching.clone()]);
    v.exile[0].push(exiled);

    let current = HeuristicAgent::new(AIProfile::EXPERT);
    let prior =
        HeuristicAgent::new(AIProfile::EXPERT.at_generation(AIProfile::TOKEN_AND_EXILE_CHOICES));

    for filter in [
        Filter::OfSourceChosenCardType,
        Filter::OfSourceChosenColor,
        Filter::ChosenColor,
        Filter::SameNameAsLinkedExile,
        Filter::SharesColorWithExiledWithSource,
    ] {
        assert_eq!(
            current.filter_matches(&filter, &v, &matching, ZoneRef::Battlefield, Some(obj(1))),
            Some(true),
            "{filter:?} at generation 19"
        );
        assert_eq!(
            prior.filter_matches(&filter, &v, &matching, ZoneRef::Battlefield, Some(obj(1))),
            None,
            "{filter:?} before generation 19"
        );
    }
}

#[test]
fn wave9c_strive_cap_either_cost_preference_and_ring_bearer_selection() {
    use baylee_cards_dsl::{Cost, CostPart, Filter, KeywordSet};
    use baylee_core::mana::ManaCost;
    use baylee_engine::choice::ChoicePrompt;
    use baylee_engine::engine::DecisionContext;

    let me = PlayerId::new(0);
    let agent = HeuristicAgent::new(AIProfile::EXPERT);

    // 1) Strive cap: base `{R}` + `{2}{R}` per extra target with `{R}{R}{C}{C}` floating -> 2 targets.
    let mut v = view(0, &[20, 20], vec![]);
    v.seats[0].mana_pool.red = 2;
    v.seats[0].mana_pool.colorless = 2;
    let base = ManaCost::parse("{R}");
    let each = ManaCost::parse("{2}{R}");
    assert_eq!(
        policy::strive_affordable_targets(&v, obj(90), (1, 4), &base, &each),
        2
    );

    // 2) Either cost branch preference: `{3}{B}` beats sacrificing a 5/5 creature,
    // while sacrificing a 1/1 token beats `{3}{B}`.
    let sac_creature = Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::Sacrifice(&Filter::CREATURE)],
    };
    let pay_3b = Cost {
        mana: ManaCost::parse("{3}{B}"),
        parts: &[],
    };
    let mut big = carded(permanent(obj(1), me, 5), "Serra Angel", TypeSet::CREATURE);
    big.mana_value = 5;
    let mut v_big = view(0, &[20, 20], vec![big]);
    v_big.seats[0].mana_pool.black = 5;
    let cast_ctx = DecisionContext {
        actor: Some(me),
        source: Some(obj(91)),
        cost: Some(ManaCost::parse("{B}")),
        ..DecisionContext::default()
    };
    assert!(
        !agent.prefers_cost_branch_pair(&v_big, &cast_ctx, &sac_creature, &pay_3b),
        "paying {{3}}{{B}} beats sacrificing a 5/5 non-token creature"
    );

    let mut token = permanent(obj(2), me, 0);
    token.toughness = Some(1);
    let mut v_tok = view(0, &[20, 20], vec![token]);
    v_tok.hand.push(hand_card(50, "Counterspell"));
    v_tok.seats[0].mana_pool.black = 5;
    assert!(
        agent.prefers_cost_branch_pair(&v_tok, &cast_ctx, &sac_creature, &pay_3b),
        "sacrificing a 0/1 token beats spending {{3}}{{B}} when holding up interaction"
    );

    // 3) ChooseRingBearer prefers a 2/2 non-legendary attacker over a Defender wall.
    let mut wall = permanent(obj(3), me, 0);
    wall.toughness = Some(4);
    wall.keywords = KeywordSet::DEFENDER.bits();
    let scout = permanent(obj(4), me, 2);
    let v_ring = view(0, &[20, 20], vec![wall, scout]);
    let ans = agent.act(
        &v_ring,
        &Pending::ChooseCards {
            player: me,
            options: vec![obj(3), obj(4)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::ChooseRingBearer,
            total: None,
        },
    );
    assert_eq!(
        ans,
        PlayerAction::ChooseObjects {
            objects: vec![obj(4)]
        }
    );
}
