//! Reach on positions a real engine dealt.
//!
//! The rest of this file's siblings build the view and the offer by hand,
//! which proves the orders read them right and nothing about what the engine
//! puts in them. What a cast costs under a reducer and whether a spell has
//! anything to point at are the engine's to say (`LegalActions`), so these
//! tables are dealt by a `Session` and read exactly as the bridge reads one.

use baylee_core::ids::PlayerId;
use baylee_core::preset::{DeckEntry, GamePreset, SeatController};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::{Phase, PlayerView};

use super::super::{Reach, reachable};

const ME: PlayerId = PlayerId::new(0);

fn entry(name: &str) -> DeckEntry {
    DeckEntry {
        card: baylee_cards::decks::by_name(name)
            .unwrap_or_else(|| panic!("{name} is not in the pool")),
        print: baylee_core::ids::PrintRef::new(0),
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
    }
}

/// Seat 0's first main phase with `mine` on its battlefield and `hand` in its
/// hand, `theirs` across the table, and the view and offer it is asked with.
pub(crate) fn position(
    mine: &[&str],
    hand: &[&str],
    theirs: &[&str],
) -> (Session, PlayerView, LegalActions) {
    position_with(mine, hand, theirs, Vec::new())
}

/// [`position`] with counters on seat 0's battlefield before the game begins.
fn position_with(
    mine: &[&str],
    hand: &[&str],
    theirs: &[&str],
    starting_counters: Vec<baylee_core::preset::StartingCounters>,
) -> (Session, PlayerView, LegalActions) {
    let mut preset: GamePreset =
        baylee_cards::decks::probe_preset(41, entry("Mountain").card).unwrap();
    preset.house_rules.starting_player = Some(0);
    preset.house_rules.starting_counters = starting_counters;
    for seat in &mut preset.seats {
        seat.controller = SeatController::Human { user_id: 0 };
        seat.starting_hand = Some(Vec::new());
        seat.starting_battlefield.clear();
    }
    preset.seats[0].starting_hand = Some(hand.iter().map(|name| entry(name)).collect());
    preset.seats[0].starting_battlefield = mine.iter().map(|name| entry(name)).collect();
    preset.seats[1].starting_battlefield = theirs.iter().map(|name| entry(name)).collect();
    let mut session = Session::new(&preset).unwrap();
    for _ in 0..100 {
        let pending = session.pending().clone();
        let player = pending.asked().expect("the table stays playable");
        if let Pending::Priority { player, .. } = pending
            && player == ME
            && session.observe_view(ME).phase == Phase::FirstMain
        {
            let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
            let Pending::Priority { legal, .. } = pending else {
                unreachable!("seat 0 holds priority");
            };
            return (session, view, *legal);
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            other => panic!("unexpected setup question: {other:?}"),
        };
        session.act_slice(player, action, 0).unwrap();
    }
    panic!("seat 0's first main phase was not reached");
}

/// The reach for the card named `name`, if the orders found one.
pub(super) fn reach_of<'a>(reach: &'a [Reach], name: &str) -> Option<&'a Reach> {
    let card = entry(name).card;
    reach.iter().find(|r| r.card.index == card)
}

/// Goblin Warchief: "Goblin spells you cast cost {1} less to cast." The
/// engine charges the Sky Raider `{1}{R}` under it, and the seat priced the
/// printed `{2}{R}`: with two Mountains the Goblin was never offered, and with
/// three it was offered tapping all three, leaving a mana floating that
/// nothing could spend (NEW-03, 3.11.2026; CR 601.2f).
#[test]
fn a_goblin_under_the_warchief_is_reached_for_its_reduced_price() {
    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider")
        .expect("two Mountains pay the Sky Raider's {1}{R} under the Warchief");
    assert_eq!(raider.plan.steps.len(), 2);

    let (_, view, legal) = position(
        &["Goblin Warchief", "Mountain", "Mountain", "Mountain"],
        &["Goblin Sky Raider"],
        &[],
    );
    let reach = reachable(&view, &legal);
    let raider = reach_of(&reach, "Goblin Sky Raider").expect("three Mountains pay it");
    assert_eq!(
        raider.plan.steps.len(),
        2,
        "the third Mountain is not tapped for a price the engine does not charge"
    );
}

/// A spell that must target and has no legal target cannot be cast
/// (CR 601.2c), and the orders tapped the lands for it anyway: Terror over
/// empty battlefields, Terror at a black creature, Fear (an Aura targets
/// what it will enchant, CR 303.4a) with no creature, Disenchant over a board
/// of creatures, Righteousness with nothing blocking. The engine refused the
/// cast with the mana already floating (NEW-04, 3.11.2026). What a spell may
/// target is the engine's to say: it lists the card in neither `castable`
/// nor `payable`.
#[test]
fn a_spell_with_no_legal_target_is_not_reached_for() {
    for (spell, lands, theirs) in [
        ("Terror", &["Swamp", "Swamp"][..], &[][..]),
        ("Terror", &["Swamp", "Swamp"][..], &["Vampire Bats"][..]),
        ("Fear", &["Swamp", "Swamp"][..], &[][..]),
        (
            "Disenchant",
            &["Plains", "Plains"][..],
            &["Grizzly Bears"][..],
        ),
        // "Target blocking creature": in a main phase nothing blocks.
        ("Righteousness", &["Plains"][..], &["Grizzly Bears"][..]),
    ] {
        let (_, view, legal) = position(lands, &[spell], theirs);
        assert!(
            reach_of(&reachable(&view, &legal), spell).is_none(),
            "{spell} reached for with nothing to target among {theirs:?}"
        );
    }
    // And with a target it is reached for: the engine's answer, not a blanket no.
    let (_, view, legal) = position(&["Swamp", "Swamp"], &["Terror"], &["Grizzly Bears"]);
    let reach = reachable(&view, &legal);
    let terror = reach_of(&reach, "Terror").expect("the Bears are a target");
    assert_eq!(terror.plan.steps.len(), 2);
}

/// Mana that may be spent only to cast a spell pays for no ability, whatever
/// the ability's source is (CR 106.6). The engine refuses it
/// (`mana_provenance::admits`), and the seat planned it: Delighted
/// Halfling's coloured mana ("only to cast a legendary spell") for the
/// legendary Yawgmoth's `{B}{B}` proliferate, and Mishra's Workshop's ("only
/// to cast artifact spells") for Rod of Ruin's `{3}`. The filter named the
/// source, so the plan tapped it, and the payment refused the mana
/// (NEW-05, 3.11.2026).
#[test]
fn mana_only_for_spells_is_never_planned_for_an_ability() {
    let ability_of = |reach: &[super::super::AbilityReach], view: &PlayerView, name: &str| {
        let source = view
            .battlefield
            .iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} is on the battlefield"))
            .id;
        reach.iter().any(|r| r.source == source)
    };
    for (mine, source, reached) in [
        (
            &["Yawgmoth, Thran Physician", "Delighted Halfling", "Swamp"][..],
            "Yawgmoth, Thran Physician",
            false,
        ),
        (
            &["Yawgmoth, Thran Physician", "Swamp", "Swamp"][..],
            "Yawgmoth, Thran Physician",
            true,
        ),
        (
            &["Rod of Ruin", "Mishra's Workshop"][..],
            "Rod of Ruin",
            false,
        ),
        (
            &[
                "Rod of Ruin",
                "Mishra's Workshop",
                "Mountain",
                "Mountain",
                "Mountain",
            ][..],
            "Rod of Ruin",
            true,
        ),
    ] {
        // A card in hand, for the proliferate's discard.
        let (_, view, legal) = position(mine, &["Mountain"], &[]);
        let reach = super::super::reachable_abilities(&view, &legal);
        assert_eq!(
            ability_of(&reach, &view, source),
            reached,
            "{source} over {mine:?}"
        );
        if reached {
            let plan = &reach
                .iter()
                .find(|r| view.object(r.source).is_some_and(|o| o.name == source))
                .unwrap()
                .plan;
            for step in &plan.steps {
                let tapped = view.object(step.source).map(|o| o.name.as_str());
                assert!(
                    !matches!(tapped, Some("Mishra's Workshop" | "Delighted Halfling")),
                    "{source}'s plan tapped {tapped:?}"
                );
            }
        }
    }
}

/// Urborg, Tomb of Yawgmoth makes each land a Swamp and Yavimaya, Cradle of
/// Growth each a Forest, so together each is a Swamp Forest that prints no
/// mana ability and taps for `{B}` or `{G}` (CR 305.6). The engine offers the
/// shortcut with a colour to choose; the planner read the shortcut only for a
/// land of one basic type and dropped both, so a Grizzly Bears beside them was
/// never offered (NEW-09, 3.11.2026). The report's Blooming Marsh, made a Swamp
/// by Evil Presence and a Forest by Yavimaya, is the same shape.
#[test]
fn a_land_with_two_basic_types_is_a_source_of_either_colour() {
    let (_, view, legal) = position(
        &["Urborg, Tomb of Yawgmoth", "Yavimaya, Cradle of Growth"],
        &["Grizzly Bears"],
        &[],
    );
    assert_eq!(
        legal.mana_abilities.len(),
        2,
        "the premise: the engine offers both lands' CR 305.6 shortcut"
    );
    let reach = reachable(&view, &legal);
    let bears = reach_of(&reach, "Grizzly Bears").expect("two Swamp Forests pay {1}{G}");
    assert_eq!(bears.plan.steps.len(), 2);
}

/// The board of the report: Meteor Storm's `{R}{G}` beside Canyon Slough,
/// Crucible of the Spirit Dragon and Shimmering Grotto. The engine lists the
/// card (`payable`) and the Crucible pays the Grotto's `{1}`, so the seat
/// reaches it, the conversion first and the Slough's `{R}` after it, and the
/// taps, played out at this table, cast the spell.
#[test]
fn a_spell_only_a_filter_land_pays_for_is_reached_and_cast() {
    use baylee_client_core::manaplan::Tap;
    use baylee_core::mana::ManaColor;
    let (mut session, view, legal) = position(
        &[
            "Canyon Slough",
            "Crucible of the Spirit Dragon",
            "Shimmering Grotto",
        ],
        &["Meteor Storm"],
        &[],
    );
    let hand = view.hand.first().expect("the Storm in hand").id;
    assert!(legal.payable.contains(&hand), "the engine lists it");
    let reach = reachable(&view, &legal);
    let storm = reach_of(&reach, "Meteor Storm").expect("the three lands pay it");
    let name = |id| {
        view.object(id)
            .map(|object| object.name.clone())
            .expect("a land of the seat's")
    };
    let order: Vec<String> = storm.plan.steps.iter().map(|s| name(s.source)).collect();
    assert_eq!(
        order,
        [
            "Crucible of the Spirit Dragon",
            "Shimmering Grotto",
            "Canyon Slough"
        ]
    );
    assert_eq!(storm.plan.steps[1].tap, Tap::Ability(1), "the paid ability");
    assert_eq!(storm.plan.steps[1].color, Some(ManaColor::Green));
    assert_eq!(storm.plan.steps[2].color, Some(ManaColor::Red));
    for step in &storm.plan.steps {
        session
            .act_slice(ME, crate::narrator::tap(step), 0)
            .expect("each step is offered as it comes");
        if let Some(color) = step.color {
            session
                .act_slice(ME, PlayerAction::ChooseColor(color), 0)
                .expect("the colour it asks");
        }
    }
    session
        .act_slice(ME, PlayerAction::CastSpell { card: hand }, 0)
        .expect("the mana pays the spell");
    let on_stack = session.observe_view(ME);
    assert!(
        on_stack.stack.iter().any(|object| object.id == hand),
        "the Storm is on the stack"
    );
}

/// The Grotto alone pays nothing coloured: it cannot pay for itself.
#[test]
fn a_filter_land_alone_reaches_no_coloured_spell() {
    let (_, view, legal) = position(&["Shimmering Grotto"], &["Llanowar Elves"], &[]);
    assert!(reach_of(&reachable(&view, &legal), "Llanowar Elves").is_none());
}

/// Seat 0's question now, read as the bridge reads it.
fn asked(session: &Session) -> (PlayerView, LegalActions) {
    let (pending, view) = session.view_for(ME).expect("seat 0 is asked");
    let Pending::Priority { legal, .. } = pending else {
        panic!("seat 0 holds priority, not {pending:?}");
    };
    (view, *legal)
}

/// Sends a plan's taps as the bridge does, each with its colour, and `then`.
fn run(session: &mut Session, plan: &baylee_client_core::manaplan::Plan, then: PlayerAction) {
    for step in &plan.steps {
        session
            .act_slice(ME, crate::narrator::tap(step), 0)
            .expect("each step is offered as it comes");
        if let Some(color) = step.color {
            session
                .act_slice(ME, PlayerAction::ChooseColor(color), 0)
                .expect("the colour it asks");
        }
    }
    session.act_slice(ME, then, 0).expect("the mana pays it");
}

/// Plays Cavern of Souls from seat 0's hand, naming `subtype` as it enters.
fn cavern_naming(
    session: &mut Session,
    view: &PlayerView,
    subtype: baylee_core::ids::SubtypeId,
) -> (PlayerView, LegalActions) {
    let card = view
        .hand
        .iter()
        .find(|card| card.name == "Cavern of Souls")
        .expect("the Cavern in hand")
        .id;
    session
        .act_slice(ME, PlayerAction::PlayLand { card }, 0)
        .expect("the land drop is free");
    session
        .act_slice(ME, PlayerAction::ChooseSubtype(subtype), 0)
        .expect("the type it names");
    asked(session)
}

/// Cavern of Souls: "Spend this mana only to cast a creature spell of the
/// chosen type". The view carries the chosen type, and the reader of a
/// restriction called it unreadable, so the coloured mana counted for no
/// spell: Dismissive Pyromancer ({1}{R}, a Human Wizard) was never offered
/// beside a Forest and a Cavern naming Human (codex attempt 32, q49). Naming
/// another type leaves it uncastable, as the engine would.
#[test]
fn a_cavern_pays_for_a_creature_of_the_type_it_names() {
    use baylee_core::generated::subtypes::creature;
    use baylee_core::mana::ManaColor;
    let hand = [
        "Cavern of Souls",
        "Dismissive Pyromancer",
        "Fire-Belly Changeling",
    ];

    let (mut session, view, _) = position(&["Forest"], &hand, &[]);
    let (view, legal) = cavern_naming(&mut session, &view, creature::GOBLIN);
    let reach = reachable(&view, &legal);
    assert!(
        reach_of(&reach, "Dismissive Pyromancer").is_none(),
        "a Cavern naming Goblin makes no red for a Human"
    );
    assert!(
        reach_of(&reach, "Fire-Belly Changeling").is_some(),
        "a changeling is a Goblin in hand too (CR 702.73a)"
    );

    let (mut session, view, _) = position(&["Forest"], &hand, &[]);
    let (view, legal) = cavern_naming(&mut session, &view, creature::HUMAN);
    let reach = reachable(&view, &legal);
    let pyromancer = reach_of(&reach, "Dismissive Pyromancer")
        .expect("a Forest and a Cavern naming Human pay {1}{R}");
    let last = pyromancer.plan.steps.last().expect("two taps");
    assert_eq!(
        last.color,
        Some(ManaColor::Red),
        "the restricted tap is last"
    );
    let card = pyromancer.object;
    let plan = pyromancer.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(
        session.observe_view(ME).stack.iter().any(|o| o.id == card),
        "the Pyromancer is on the stack"
    );
}

/// Gemstone Caverns: "{T}: Add {C}. If Gemstone Caverns has a luck counter on
/// it, instead add one mana of any color." Written as an `if`, which no reader
/// of a mana ability read, so the land was no source at all: Assault Griffin
/// ({3}{W}) was never offered beside it and three other lands (codex attempt
/// 25, q57). Which branch runs is the board's to say.
#[test]
fn gemstone_caverns_taps_for_what_its_counter_says() {
    use baylee_core::mana::ManaColor;
    use baylee_core::preset::{StartingCounter, StartingCounters};
    let luck = || {
        vec![StartingCounters {
            seat: 0,
            permanent: 0,
            counters: vec![StartingCounter {
                // `counters::LUCK`.
                kind: "custom:30".into(),
                amount: 1,
            }],
        }]
    };

    // The report: {C} or a colour, it pays the generic part either way.
    for counters in [Vec::new(), luck()] {
        let (_, view, legal) = position_with(
            &["Gemstone Caverns", "Plains", "Swamp", "Swamp"],
            &["Assault Griffin"],
            &[],
            counters,
        );
        let reach = reachable(&view, &legal);
        let griffin = reach_of(&reach, "Assault Griffin").expect("four lands pay {3}{W}");
        assert_eq!(griffin.plan.steps.len(), 4);
    }

    // With no counter it is {C} and no colour.
    let (_, view, legal) = position(&["Gemstone Caverns"], &["Savannah Lions"], &[]);
    assert!(reach_of(&reachable(&view, &legal), "Savannah Lions").is_none());

    // With one it is any colour, asked as the tap resolves, and the house
    // counts it as the seat does.
    let (mut session, view, legal) =
        position_with(&["Gemstone Caverns"], &["Savannah Lions"], &[], luck());
    assert_eq!(baylee_ai::mana_sources(&view, &legal).len(), 1);
    let reach = reachable(&view, &legal);
    let lions = reach_of(&reach, "Savannah Lions").expect("a luck counter makes white");
    assert_eq!(lions.plan.steps[0].color, Some(ManaColor::White));
    let card = lions.object;
    let plan = lions.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));
}

/// River of Tears: "{T}: Add {U}. If you played a land this turn, add {B}
/// instead." The same shape, and the answer moves within a turn.
#[test]
fn river_of_tears_taps_for_black_once_a_land_was_played() {
    let hand = ["Dark Ritual", "Opt", "Island"];
    let (mut session, view, legal) = position(&["River of Tears"], &hand, &[]);
    let reach = reachable(&view, &legal);
    assert!(reach_of(&reach, "Opt").is_some(), "{{U}} before the land");
    assert!(reach_of(&reach, "Dark Ritual").is_none());

    let card = view
        .hand
        .iter()
        .find(|card| card.name == "Island")
        .expect("the Island in hand")
        .id;
    session
        .act_slice(ME, PlayerAction::PlayLand { card }, 0)
        .expect("the land drop is free");
    let (view, legal) = asked(&session);
    let reach = reachable(&view, &legal);
    let ritual = reach_of(&reach, "Dark Ritual").expect("{B} once a land was played");
    let river = view
        .battlefield
        .iter()
        .find(|o| o.name == "River of Tears")
        .expect("the River")
        .id;
    assert_eq!(ritual.plan.steps[0].source, river);
}

/// What the table asks of whoever is asked while nothing is being decided:
/// a pass, and no attackers or blockers.
fn pass_on(session: &mut Session) {
    let pending = session.pending().clone();
    let player = pending.asked().expect("the table stays playable");
    let action = match pending {
        Pending::Priority { .. } => PlayerAction::PassPriority,
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        },
        other => panic!("unexpected question: {other:?}"),
    };
    session.act_slice(player, action, 0).unwrap();
}

/// Passes until the stack is empty and seat 0 holds priority.
fn resolve_the_stack(session: &mut Session) -> (PlayerView, LegalActions) {
    for _ in 0..20 {
        if session.observe_view(ME).stack.is_empty()
            && matches!(session.pending(), Pending::Priority { player, .. } if *player == ME)
        {
            return asked(session);
        }
        pass_on(session);
    }
    panic!("the stack did not empty");
}

/// Passes to seat 0's next first main phase, its Saga's chapters resolved.
fn my_next_main(session: &mut Session) -> (PlayerView, LegalActions) {
    let turn = session.observe_view(ME).turn;
    for _ in 0..200 {
        let view = session.observe_view(ME);
        if view.turn > turn && view.active == ME && view.phase == Phase::FirstMain {
            return resolve_the_stack(session);
        }
        pass_on(session);
    }
    panic!("seat 0's next turn was not reached");
}

/// Plays the land `name` from seat 0's hand and resolves what it set off.
fn play(session: &mut Session, name: &str) -> (PlayerView, LegalActions) {
    let (view, _) = asked(session);
    let card = view
        .hand
        .iter()
        .find(|card| card.name == name)
        .unwrap_or_else(|| panic!("{name} in hand"))
        .id;
    session
        .act_slice(ME, PlayerAction::PlayLand { card }, 0)
        .expect("the land drop is free");
    resolve_the_stack(session)
}

/// The permanent of that name on the battlefield.
fn named(view: &PlayerView, name: &str) -> baylee_core::ids::ObjectId {
    view.battlefield
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("{name} is on the battlefield"))
        .id
}

/// Urza's Saga gains "{2}, {T}: Create a Construct": the `{T}` is the Saga's,
/// so its own mana pays none of the `{2}` (CR 601.2g, 601.2h). A printed cost
/// was read for that and a granted one was not, so with two Sagas the seat
/// offered the Construct tapping both, and the Saga then had no tap to pay
/// with (codex attempt 27, q42).
#[test]
fn a_saga_cannot_tap_itself_for_its_own_construct() {
    let (mut session, _, _) = position(&[], &["Urza's Saga", "Urza's Saga"], &[]);
    play(&mut session, "Urza's Saga");
    my_next_main(&mut session);
    let (view, legal) = play(&mut session, "Urza's Saga");
    assert!(
        legal
            .unpaid_abilities
            .iter()
            .any(|&(_, index, _)| { baylee_engine::choice::granted_slot(index).is_some() }),
        "chapter II gave one Saga the ability, and only mana is missing"
    );
    assert_eq!(
        super::super::reachable_abilities(&view, &legal),
        Vec::new(),
        "one Saga's {{C}} does not pay {{2}}"
    );
}

/// The payment the report asked for, which leaves the Saga its tap (codex
/// attempt 27, q89): a Swamp and Bronze Walrus pay, and the Construct comes.
#[test]
fn a_saga_makes_its_construct_with_other_mana() {
    let (mut session, _, _) = position(&["Swamp", "Bronze Walrus"], &["Urza's Saga"], &[]);
    play(&mut session, "Urza's Saga");
    let (view, legal) = my_next_main(&mut session);
    let saga = named(&view, "Urza's Saga");
    let reach = super::super::reachable_abilities(&view, &legal);
    let construct = reach
        .iter()
        .find(|r| r.source == saga)
        .expect("the Swamp and the Walrus pay {2}");
    let tapped: Vec<_> = construct.plan.steps.iter().map(|s| s.source).collect();
    assert_eq!(tapped.len(), 2);
    assert!(!tapped.contains(&saga), "the Saga keeps its tap");
    let ability_index = construct.ability_index;
    let plan = construct.plan.clone();
    run(
        &mut session,
        &plan,
        PlayerAction::ActivateAbility {
            source: saga,
            ability_index,
        },
    );
    let (view, _) = resolve_the_stack(&mut session);
    assert!(
        view.battlefield.iter().any(|o| o.name == "Construct"),
        "the token is made"
    );
}

/// "{1}{W}: This land becomes a 1/5 white Soldier creature until end of
/// turn." Nothing stops the Watchtower paying for itself, and the plan had it
/// do that ahead of a Swarmyard kept for its regeneration, so the only offer
/// beside untapped Plains and Swarmyard made a creature that could not block
/// (codex attempt 32, q173). An activation's source is tapped for it last.
#[test]
fn a_land_that_animates_itself_is_tapped_for_it_last() {
    let (mut session, view, legal) =
        position(&["Forbidding Watchtower", "Plains", "Swarmyard"], &[], &[]);
    let tower = named(&view, "Forbidding Watchtower");
    let reach = super::super::reachable_abilities(&view, &legal);
    let animate = reach
        .iter()
        .find(|r| r.source == tower)
        .expect("the Plains and the Swarmyard pay {1}{W}");
    assert!(
        animate.plan.steps.iter().all(|s| s.source != tower),
        "the land stays untapped"
    );
    let ability_index = animate.ability_index;
    let plan = animate.plan.clone();
    run(
        &mut session,
        &plan,
        PlayerAction::ActivateAbility {
            source: tower,
            ability_index,
        },
    );
    let (view, _) = resolve_the_stack(&mut session);
    let soldier = view.object(tower).expect("the Watchtower");
    assert!(
        soldier
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
    );
    assert!(!soldier.status.is_tapped());

    // Where nothing else pays it is still a legal activation, and offered.
    let (_, view, legal) = position(&["Forbidding Watchtower", "Plains"], &[], &[]);
    let tower = named(&view, "Forbidding Watchtower");
    let reach = super::super::reachable_abilities(&view, &legal);
    let animate = reach.iter().find(|r| r.source == tower);
    assert!(
        animate.is_some_and(|r| r.plan.steps.iter().any(|s| s.source == tower)),
        "the Watchtower may pay for itself when it has to"
    );
}

/// A bounce land's "{T}: Add {U}{R}" says "add" twice. The client read that as
/// a package and the house's reader did not, so for the seat and the house
/// the land was no source at all: Wee Dragonauts ({1}{U}{R}) was not offered
/// beside Izzet Boilerworks and an Island (codex attempt 54, q42), nor
/// Eladamri's Call ({W}{G}) beside Selesnya Sanctuary alone (attempt 66, q77).
/// One tap pays one pip of each, and never two of one.
#[test]
fn a_bounce_land_pays_one_of_each_of_its_colours() {
    let (mut session, view, legal) =
        position(&["Izzet Boilerworks", "Island"], &["Wee Dragonauts"], &[]);
    assert_eq!(baylee_ai::mana_sources(&view, &legal).len(), 2);
    let reach = reachable(&view, &legal);
    let dragonauts = reach_of(&reach, "Wee Dragonauts").expect("two lands make three mana");
    assert_eq!(dragonauts.plan.steps.len(), 2);
    let card = dragonauts.object;
    let plan = dragonauts.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));

    let (_, view, legal) = position(&["Selesnya Sanctuary"], &["Eladamri's Call"], &[]);
    let reach = reachable(&view, &legal);
    let call = reach_of(&reach, "Eladamri's Call").expect("one tap makes {G}{W}");
    assert_eq!(call.plan.steps.len(), 1);

    let (_, view, legal) = position(&["Izzet Boilerworks"], &["Counterspell"], &[]);
    assert!(
        reach_of(&reachable(&view, &legal), "Counterspell").is_none(),
        "{{U}}{{R}} pays no {{U}}{{U}}"
    );
}

/// Pelargir Survivor: "{T}: Add one mana of any color. Spend this mana only
/// to cast an instant or sorcery spell." The filter was read, and the plan
/// stopped at one restricted tap, so two Survivors beside an Island and a
/// Swamp never paid Tamiyo's Epiphany ({3}{U}) (codex attempt 38, q231). The
/// seat sends its taps whole, so it may spend every source that admits the
/// spell.
#[test]
fn two_restricted_taps_pay_one_spell() {
    let survivors = ["Pelargir Survivor", "Pelargir Survivor", "Island", "Swamp"];
    let (mut session, view, legal) = position(&survivors, &["Tamiyo's Epiphany"], &[]);
    let reach = reachable(&view, &legal);
    let epiphany = reach_of(&reach, "Tamiyo's Epiphany").expect("four sources pay {3}{U}");
    assert_eq!(epiphany.plan.steps.len(), 4);
    let card = epiphany.object;
    let plan = epiphany.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));

    let (_, view, legal) = position(&survivors[1..], &["Tamiyo's Epiphany"], &[]);
    assert!(
        reach_of(&reachable(&view, &legal), "Tamiyo's Epiphany").is_none(),
        "three sources do not"
    );

    let (_, view, legal) = position(&survivors, &["Wind Drake"], &[]);
    assert!(
        reach_of(&reachable(&view, &legal), "Wind Drake").is_none(),
        "their mana pays for no creature, and the lands alone make two"
    );
}

/// Unstable Frontier on Hidden Grotto, choosing Swamp, as played in argon
/// attempt 34. The type is chosen as the ability resolves (CR 608.2d): the
/// ability is on the stack when it is asked. The land then has the Swamp's
/// {T}: Add {B} (CR 305.6) and has lost what it printed (CR 305.7), so
/// Hidden Grotto's own abilities are rightly gone. The Frontier tapped to do
/// this, which leaves three mana: {1}{B}{B} is reached, and the report's
/// Kezzerdrix ({2}{B}{B}) is not.
#[test]
fn a_land_made_a_swamp_taps_for_black_and_nothing_it_printed() {
    let (mut session, view, _) = position(
        &["Unstable Frontier", "Hidden Grotto", "Swamp", "Island"],
        &["Hypnotic Specter", "Kezzerdrix"],
        &[],
    );
    let (frontier, grotto) = (
        named(&view, "Unstable Frontier"),
        named(&view, "Hidden Grotto"),
    );
    session
        .act_slice(
            ME,
            PlayerAction::ActivateAbility {
                source: frontier,
                ability_index: 1,
            },
            0,
        )
        .expect("the Frontier's tap is its whole cost");
    session
        .act_slice(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![grotto],
            },
            0,
        )
        .expect("its target");
    for _ in 0..4 {
        if matches!(session.pending(), Pending::ChooseCastMode { .. }) {
            break;
        }
        pass_on(&mut session);
    }
    let on_the_stack = session.observe_view(ME).stack.len();
    assert_eq!(on_the_stack, 1, "the choice comes as the ability resolves");
    session
        .act_slice(ME, PlayerAction::ChooseMode(2), 0)
        .expect("Swamp, the third of the five");

    let (view, legal) = resolve_the_stack(&mut session);
    assert!(legal.mana_abilities.contains(&grotto), "CR 305.6");
    assert!(
        legal.abilities.iter().all(|&(source, _)| source != grotto),
        "CR 305.7: what it printed is gone"
    );
    let reach = reachable(&view, &legal);
    let specter = reach_of(&reach, "Hypnotic Specter").expect("two Swamps and an Island");
    assert!(specter.plan.steps.iter().any(|s| s.source == grotto));
    assert!(
        reach_of(&reach, "Kezzerdrix").is_none(),
        "three mana, not four"
    );
}
