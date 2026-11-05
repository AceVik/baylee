//! The window after blocks (CR 509.4): the rail stops there in a combat
//! the seat fights in, and passes it in any other.

use super::*;
use baylee_view::{AttackerView, BlockerView};

/// Agy attempt 0, turn 25, q341-q342: P2 attacks with a Zombie token and
/// Nether Shadow; this seat (P1) has four untapped Swamps and Drudge
/// Skeletons, whose "{B}: Regenerate" the Swamps can pay. `blocking` is
/// whether the Skeletons block the Zombie.
fn after_blocks(blocking: bool) -> (PlayerView, LegalActions) {
    let (mut view, mut legal) = table(
        &["Swamp", "Swamp", "Swamp", "Swamp", "Drudge Skeletons"],
        &["Dusk Imp"],
    );
    let skeletons = o(5);
    legal.abilities.retain(|&(source, _)| source != skeletons);
    legal
        .unpaid_abilities
        .push((skeletons, 0, "{B}".parse().unwrap()));
    let mut zombie = token(138, 1, "Zombie", 2, 2);
    zombie.status = baylee_view::ObjectStatus::TAPPED;
    let mut shadow = permanent(87, "Nether Shadow");
    shadow.controller = THEM;
    view.battlefield.extend([zombie, shadow]);
    their(&mut view, Phase::Combat, Step::DeclareBlockers);
    view.combat.attackers = [o(138), o(87)]
        .into_iter()
        .map(|creature| AttackerView {
            creature,
            defending: Defender::Player(ME),
            blocked: blocking && creature == o(138),
        })
        .collect();
    if blocking {
        view.combat.blockers = vec![BlockerView {
            blocker: skeletons,
            attacker: o(138),
        }];
    }
    (view, legal)
}

/// NEW-10: with the Skeletons blocking and a Swamp to pay their
/// regeneration, the window after blocks is the seat's decision, and the
/// default rail wakes it there. Before, the rail had no blockers row and
/// the window was passed as quiet: games played by models had no
/// regeneration, pump or trick after blocks in them.
#[test]
fn the_window_after_blocks_wakes_a_seat_whose_creature_blocks() {
    let (view, legal) = after_blocks(true);
    assert!(offering(&view, &legal), "the regeneration is within reach");
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
}

/// The same window with nothing of the seat's in the combat is passed:
/// waking every combat for a seat with an instant in hand would cost a
/// call a turn for nothing.
#[test]
fn the_window_after_blocks_passes_a_combat_the_seat_has_no_creature_in() {
    let (view, legal) = after_blocks(false);
    assert!(offering(&view, &legal));
    assert_eq!(judge(&view, legal), passes(Standing::QuietWindow));
}

/// On the seat's own turn too: its attacker was blocked, and a pump or a
/// regeneration is a decision there.
#[test]
fn the_window_after_blocks_wakes_an_attacker_on_its_own_turn() {
    let (mut view, legal) = table(&["Swamp", "Swamp", "Drudge Skeletons"], &[]);
    let mut legal = legal;
    legal.abilities.retain(|&(source, _)| source != o(3));
    legal
        .unpaid_abilities
        .push((o(3), 0, "{B}".parse().unwrap()));
    view.battlefield.push(token(40, 1, "Wall", 0, 4));
    view.phase = Phase::Combat;
    view.step = Step::DeclareBlockers;
    view.combat.attackers = vec![AttackerView {
        creature: o(3),
        defending: Defender::Player(THEM),
        blocked: true,
    }];
    view.combat.blockers = vec![BlockerView {
        blocker: o(40),
        attacker: o(3),
    }];
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
}

/// A model's `stops` may name a step as the rules do. Codex attempt 62's
/// model asked for `declare_blockers` and was never woken there: only the
/// header's short names were read. Unknown names are left out, so the
/// header's `stops:` line says what the seat stops at.
#[test]
fn stops_read_the_rules_names_of_the_steps() {
    let decision = crate::narrator::Decision::from_decide(&serde_json::json!({
        "ask": "q191",
        "pick": ["p"],
        "stops": {
            "mine": ["main1", "main2", "declare_blockers", "my_upkeep"],
            "theirs": ["Declare Attackers", "combat-damage", "end step", "end_step"]
        }
    }))
    .expect("a decision");
    let stops = decision.stops.expect("stops");
    assert_eq!(stops.mine, ["main1", "main2", "blockers"]);
    assert_eq!(stops.theirs, ["attackers", "damage", "end_step"]);
    let (view, _) = table(&[], &[]);
    let mut filter = WakeFilter::default();
    filter.apply_orders(Some(&stops), None, 0, &view, &[]);
    assert!(filter.stops_at(RailSide::Mine, RailRow::Blockers));
    assert!(filter.stops_at(RailSide::Theirs, RailRow::Damage));
    assert!(!filter.stops_at(RailSide::Mine, RailRow::Upkeep));
}
