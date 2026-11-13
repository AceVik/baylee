//! Casting the visible library top for life uses the engine's exact route.
use super::*;
use baylee_engine::choice::{CastModeDesc, CastModeKind, LegalActions};

const ME: PlayerId = PlayerId::new(0);
const TOP: u32 = 90;

fn top(name: &str) -> PublicObject {
    carded(permanent(obj(TOP), ME, 0), name, hand_card(TOP, name).types)
}

fn offer(
    view: &PlayerView,
    kind: CastModeKind,
    cost: baylee_core::mana::ManaCost,
    mana_sources: Vec<ObjectId>,
) -> Pending {
    let id = obj(TOP);
    let ready = view.seats[0].mana_pool.total() >= u64::from(cost.cmc());
    Pending::Priority {
        player: ME,
        legal: Box::new(LegalActions {
            can_pass: true,
            castable: ready.then_some(id).into_iter().collect(),
            payable: (!ready).then_some(id).into_iter().collect(),
            cast_ways: vec![(
                id,
                CastModeDesc {
                    index: 0,
                    kind,
                    cost,
                },
            )],
            mana_abilities: mana_sources,
            ..Default::default()
        }),
    }
}

fn life_route(face: usize, life: u32) -> CastModeKind {
    CastModeKind::LibraryTopLife {
        life,
        face,
        modes: 0,
        kicked: false,
    }
}

fn table(name: &str, life: i32, board: Vec<PublicObject>) -> PlayerView {
    let mut view = view(0, &[life, 20], board);
    view.phase = baylee_view::Phase::FirstMain;
    view.library_tops = vec![top(name)];
    view
}

#[test]
fn a_zero_mana_library_route_is_cast_without_tapping() {
    let view = table("Grizzly Bears", 20, vec![]);
    let pending = offer(
        &view,
        life_route(0, 2),
        baylee_core::mana::ManaCost::ZERO,
        vec![],
    );
    assert_eq!(
        agent().act(&view, &pending),
        PlayerAction::CastSpell { card: obj(TOP) }
    );
}

#[test]
fn the_library_life_floor_is_kept() {
    let view = table("Grizzly Bears", 7, vec![]);
    let pending = offer(
        &view,
        life_route(0, 2),
        baylee_core::mana::ManaCost::ZERO,
        vec![],
    );
    assert_eq!(agent().act(&view, &pending), PlayerAction::PassPriority);
}

#[test]
fn only_the_routes_remaining_mana_is_tapped() {
    let mut forest = carded(permanent(obj(1), ME, 0), "Forest", TypeSet::LAND);
    forest
        .subtypes
        .insert(baylee_core::generated::subtypes::land::FOREST);
    let view = table("Grizzly Bears", 20, vec![forest]);
    let pending = offer(
        &view,
        life_route(0, 2),
        baylee_core::mana::ManaCost::parse("{G}"),
        vec![obj(1)],
    );
    assert_eq!(
        agent().act(&view, &pending),
        PlayerAction::ActivateManaAbility { source: obj(1) },
        "the printed {{1}}{{G}} must not be charged again beside the route's {{G}}"
    );
}

#[test]
fn an_adventure_back_face_keeps_its_own_identity_and_targets() {
    let enemy = carded(
        permanent(obj(1), PlayerId::new(1), 4),
        "Baleful Strix",
        TypeSet::CREATURE,
    );
    let view = table("Brazen Borrower", 20, vec![enemy]);
    let pending = offer(
        &view,
        life_route(1, 2),
        baylee_core::mana::ManaCost::ZERO,
        vec![],
    );
    assert_eq!(
        agent().act(&view, &pending),
        PlayerAction::CastSpell { card: obj(TOP) }
    );
    let mode = Pending::ChooseCastMode {
        player: ME,
        object: obj(TOP),
        options: vec![CastModeDesc {
            index: 0,
            kind: life_route(1, 2),
            cost: baylee_core::mana::ManaCost::ZERO,
        }],
    };
    assert_eq!(agent().act(&view, &mode), PlayerAction::ChooseMode(0));
}

#[test]
fn mandatory_additional_life_counts_toward_the_floor() {
    let enemy_creature = carded(
        permanent(obj(1), PlayerId::new(1), 4),
        "Baleful Strix",
        TypeSet::CREATURE,
    );
    let enemy_land = carded(
        permanent(obj(2), PlayerId::new(1), 0),
        "Forest",
        TypeSet::LAND,
    );
    let board = vec![enemy_creature, enemy_land];
    let affordable = table("Fumarole", 14, board.clone());
    let pending = offer(
        &affordable,
        life_route(0, 5),
        baylee_core::mana::ManaCost::ZERO,
        vec![],
    );
    assert_eq!(
        agent().act(&affordable, &pending),
        PlayerAction::CastSpell { card: obj(TOP) },
        "five mana-value life plus three additional life leaves exactly six"
    );
    let too_risky = table("Fumarole", 13, board);
    let pending = offer(
        &too_risky,
        life_route(0, 5),
        baylee_core::mana::ManaCost::ZERO,
        vec![],
    );
    assert_eq!(
        agent().act(&too_risky, &pending),
        PlayerAction::PassPriority
    );
}

#[test]
fn forced_zero_x_does_not_cast_a_stillborn_creature_even_with_a_tax() {
    let mut view = table("Walking Ballista", 20, vec![]);
    view.seats[0].mana_pool.colorless = 2;
    let pending = offer(
        &view,
        life_route(0, 0),
        baylee_core::mana::ManaCost::ZERO.with_more_generic(2),
        vec![],
    );
    assert_eq!(agent().act(&view, &pending), PlayerAction::PassPriority);
}
