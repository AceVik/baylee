//! Plays the planner missed on Carbon's boards (chunks b and c, 11.11.2026),
//! dealt by a real engine and paid through it as the bridge pays: a second
//! Rishadan Port, a Druids' Repository counted once, and spells dropped while
//! restricted mana floated (CR 106.6).

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaColor;
use baylee_core::preset::{StartingCounter, StartingCounters};
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_view::PlayerView;

use super::super::{reachable, reachable_abilities};
use super::played::{asked, position, position_with, reach_of, run};

const ME: PlayerId = PlayerId::new(0);

/// Seat 0's permanents named `name`, in battlefield order.
fn mine(view: &PlayerView, name: &str) -> Vec<ObjectId> {
    view.battlefield
        .iter()
        .filter(|o| o.controller == ME && o.name.as_ref() == name)
        .map(|o| o.id)
        .collect()
}

/// Rishadan Port's `{1}, {T}: Tap target land`.
const PORT_TAPS_A_LAND: u32 = 1;

/// Activates `port`'s land tapping after `plan`'s taps, aimed at the first
/// land seat 1 controls, and hands back the next priority.
fn port_at_their_land(
    session: &mut Session,
    view: &PlayerView,
    port: ObjectId,
    plan: &baylee_client_core::manaplan::Plan,
) -> (PlayerView, LegalActions) {
    let land = view
        .battlefield
        .iter()
        .find(|o| o.controller != ME)
        .expect("a land of theirs")
        .id;
    run(
        session,
        plan,
        PlayerAction::ActivateAbility {
            source: port,
            ability_index: PORT_TAPS_A_LAND,
        },
    );
    let Pending::ChooseTargets { options, .. } = session.pending().clone() else {
        panic!("the Port asks its target, not {:?}", session.pending());
    };
    assert!(options.contains(&land), "{options:?}");
    session
        .act_slice(
            ME,
            PlayerAction::ChooseTargets {
                objects: vec![land],
                players: Vec::new(),
            },
            0,
        )
        .expect("their land is a legal target");
    asked(session)
}

/// Two Rishadan Ports beside two Swamps and a Mountain, Terminate in hand:
/// each Port's `{1}` was paid by the other Port, the Port being worth least
/// to a hand that wants black and red, so the seat offered one payment per
/// Port and could never use both (Carbon b4, hc-expert-b game-0010 q160).
/// Each Port is paid by a land while a land is left to pay the other one.
#[test]
fn a_port_is_paid_by_a_land_so_the_other_port_stays_usable() {
    let (mut session, view, legal) = position(
        &[
            "Rishadan Port",
            "Rishadan Port",
            "Swamp",
            "Swamp",
            "Mountain",
        ],
        &["Terminate"],
        &["Mountain"],
    );
    let ports = mine(&view, "Rishadan Port");
    assert_eq!(ports.len(), 2);
    let reach = reachable_abilities(&view, &legal);
    let paid = |reach: &[super::super::AbilityReach], port: ObjectId| {
        reach
            .iter()
            .find(|r| r.source == port && r.ability_index == PORT_TAPS_A_LAND)
            .unwrap_or_else(|| panic!("the Port's {{1}} is planned: {reach:?}"))
            .plan
            .clone()
    };
    for &port in &ports {
        let plan = paid(&reach, port);
        assert!(
            plan.steps.iter().all(|step| !ports.contains(&step.source)),
            "a Port pays for a Port while lands are untapped: {plan:?}"
        );
    }
    // The first Port, then the second, both through the engine.
    let plan = paid(&reach, ports[0]);
    let (view, legal) = port_at_their_land(&mut session, &view, ports[0], &plan);
    let reach = reachable_abilities(&view, &legal);
    let plan = paid(&reach, ports[1]);
    port_at_their_land(&mut session, &view, ports[1], &plan);
}

/// With nothing but the two Ports, one pays for the other: keeping a Port
/// untapped for an ability nothing is left to pay is no reason to lose the
/// first activation.
#[test]
fn a_port_still_pays_for_a_port_when_nothing_else_does() {
    let (_, view, legal) = position(
        &["Rishadan Port", "Rishadan Port"],
        &["Terminate"],
        &["Mountain"],
    );
    let ports = mine(&view, "Rishadan Port");
    let reach = reachable_abilities(&view, &legal);
    for &port in &ports {
        let found = reach
            .iter()
            .find(|r| r.source == port && r.ability_index == PORT_TAPS_A_LAND)
            .unwrap_or_else(|| panic!("the other Port pays its {{1}}: {reach:?}"));
        assert_eq!(found.plan.steps.len(), 1);
        assert_ne!(found.plan.steps[0].source, port);
    }
}

/// Two charge counters on Druids' Repository, "Remove a charge counter from
/// this enchantment: Add one mana of any color", are two mana: with a Swamp
/// and a Plains they pay Prowling Nightstalker's `{3}{B}`. The planner read
/// one mana per permanent, so the cast was offered only after a counter was
/// removed by hand (Carbon c1, hc-expert-c game-0001 q143).
#[test]
fn a_repository_with_two_counters_pays_two_mana() {
    let charge = |amount| {
        vec![StartingCounters {
            seat: 0,
            permanent: 0,
            counters: vec![StartingCounter {
                kind: "charge".into(),
                amount,
            }],
        }]
    };
    let lands = ["Druids' Repository", "Swamp", "Plains"];
    let (_, view, legal) = position_with(&lands, &["Prowling Nightstalker"], &[], charge(1));
    assert!(
        reach_of(&reachable(&view, &legal), "Prowling Nightstalker").is_none(),
        "one counter and two lands are three mana"
    );
    let (mut session, view, legal) =
        position_with(&lands, &["Prowling Nightstalker"], &[], charge(2));
    let reach = reachable(&view, &legal);
    let stalker = reach_of(&reach, "Prowling Nightstalker")
        .expect("two counters, a Swamp and a Plains pay {3}{B}");
    let repository = mine(&view, "Druids' Repository")[0];
    assert_eq!(
        stalker
            .plan
            .steps
            .iter()
            .filter(|step| step.source == repository)
            .count(),
        2,
        "{:?}",
        stalker.plan
    );
    let card = stalker.object;
    let plan = stalker.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));
}

/// Great Hall of the Biblioplex's restricted ability: "{T}, Pay 1 life: Add
/// one mana of any color. Spend this mana only to cast an instant or sorcery
/// spell."
const GREAT_HALL_RESTRICTED: u32 = 1;

/// Seat 0's first main phase over `lands`, Great Hall of the Biblioplex
/// first, with `hand`, after the Hall made `color` for instants and
/// sorceries.
fn with_great_hall_mana(
    lands: &[&str],
    hand: &[&str],
    color: ManaColor,
) -> (Session, PlayerView, LegalActions) {
    let mut all = vec!["Great Hall of the Biblioplex"];
    all.extend_from_slice(lands);
    let (mut session, view, _) = position(&all, hand, &[]);
    let hall = mine(&view, "Great Hall of the Biblioplex")[0];
    session
        .act_slice(
            ME,
            PlayerAction::ActivateAbility {
                source: hall,
                ability_index: GREAT_HALL_RESTRICTED,
            },
            0,
        )
        .expect("the Hall's restricted mana");
    session
        .act_slice(ME, PlayerAction::ChooseColor(color), 0)
        .expect("the colour it asks");
    let (view, legal) = asked(&session);
    assert_eq!(view.seats[0].mana_pool.restricted_total(), 1);
    (session, view, legal)
}

/// Esper Charm with the Hall's `{B}` floating beside a Plains, a Study Hall
/// and Islands: the floating black pays the instant (CR 106.6), and the
/// seat dropped the Charm until the pool emptied (Carbon b9, hc-expert-b
/// game-0020 q163). The engine names the floating mana for the Charm since
/// protocol 43.
#[test]
fn an_instant_is_reached_with_the_restricted_mana_floating_for_it() {
    let (mut session, view, legal) = with_great_hall_mana(
        &["Plains", "Study Hall", "Island", "Island"],
        &["Esper Charm"],
        ManaColor::Black,
    );
    let reach = reachable(&view, &legal);
    let charm = reach_of(&reach, "Esper Charm").expect("{W}{U} tapped and {B} floating");
    assert_eq!(charm.plan.steps.len(), 2, "{:?}", charm.plan);
    let card = charm.object;
    let plan = charm.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    let (pending, _) = session.view_for(ME).expect("seat 0 is asked");
    assert!(
        matches!(pending, Pending::ChooseCastMode { .. }),
        "the Charm's mode, as it is cast: {pending:?}"
    );
}

/// Hymn to Tourach's `{B}{B}` with the Hall's `{B}` floating, a Study Hall
/// and an Island: the floating black pays one, and Study Hall's `{1}, {T}:
/// Add one mana of any color`, paid by the Island, the other. The planner
/// read no paid mana ability while any restricted mana floated.
#[test]
fn a_conversion_is_planned_beside_restricted_mana_floating() {
    let (mut session, view, legal) = with_great_hall_mana(
        &["Study Hall", "Island"],
        &["Hymn to Tourach"],
        ManaColor::Black,
    );
    let reach = reachable(&view, &legal);
    let hymn = reach_of(&reach, "Hymn to Tourach")
        .expect("the Island pays Study Hall's {1}, and Study Hall makes the second {B}");
    let card = hymn.object;
    let plan = hymn.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    let Pending::ChoosePlayer { options, .. } = session.pending().clone() else {
        panic!("the Hymn asks its player, not {:?}", session.pending());
    };
    let them = options
        .into_iter()
        .find(|p| *p != ME)
        .expect("the opponent may be targeted");
    session
        .act_slice(ME, PlayerAction::ChoosePlayer(them), 0)
        .expect("the opponent is a legal target, and the mana pays the Hymn");
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));
}

/// The Hall's `{B}` floats and may not pay for a creature, nor for Study
/// Hall's `{1}`: Grizzly Bears' `{1}{G}` is the Island's mana through Study
/// Hall and a Plains, and the floating black is left alone.
#[test]
fn restricted_mana_floating_for_other_spells_leaves_a_conversion_plannable() {
    let (mut session, view, legal) = with_great_hall_mana(
        &["Study Hall", "Island", "Plains"],
        &["Grizzly Bears"],
        ManaColor::Black,
    );
    let reach = reachable(&view, &legal);
    let bears = reach_of(&reach, "Grizzly Bears")
        .expect("Study Hall's {G}, paid by the Island, and the Plains");
    assert_eq!(bears.plan.steps.len(), 3, "{:?}", bears.plan);
    let card = bears.object;
    let plan = bears.plan.clone();
    run(&mut session, &plan, PlayerAction::CastSpell { card });
    assert!(session.observe_view(ME).stack.iter().any(|o| o.id == card));
}

/// The Hall's `{B}` may pay for an instant, never for Study Hall's `{1}`
/// (the restriction admits a spell, not an ability's cost): with nothing
/// else to pay that `{1}`, Brainstorm's `{U}` is out of reach.
#[test]
fn restricted_mana_never_pays_a_conversion() {
    let (_, view, legal) = with_great_hall_mana(&["Study Hall"], &["Brainstorm"], ManaColor::Black);
    let reach = reachable(&view, &legal);
    assert!(reach_of(&reach, "Brainstorm").is_none(), "{reach:?}");
}
