#[allow(clippy::wildcard_imports)] // shared integration fixture vocabulary
use super::*;

/// Seat 0's permanents after its first turn with `board` and nothing in
/// hand, by name.
fn board_after_turn_one(board: &[&str]) -> Vec<String> {
    let mut engine = Engine::new(&position(&[], board), RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::STEADY);
    for seq in 0..200 {
        let pending = engine.pending();
        let seat = pending_player(pending).expect("nobody has lost on turn 1");
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn > 1 {
            return view
                .battlefield_of(PlayerId::new(0))
                .map(|o| o.name.clone())
                .collect();
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            // The opponent holds nothing and passes; a creature on this
            // board may attack, and the opponent is asked to block it.
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        engine
            .apply(seat, action)
            .expect("every planned action is legal");
    }
    panic!("turn 1 never ended");
}

/// #223. A card is not given up for a small gain.
///
/// `activate::useful` took any ability whose cost changes the board and
/// whose effect it recognises as a gain, and a sacrifice is a cost that
/// changes the board. Measured through the engine before the fix, with
/// nothing to cast on turn 1: Zuran Orb was fed all four Forests for 8 life,
/// and Viscera Seer sacrificed itself to scry 1. Every gain the whitelist
/// knew — a card, a scry, a few life, a counter, a token — was worth no more
/// than the permanent or the card it would cost. Since `worth` the refusal
/// is a price rather than a list: a land on turn 1 and a creature are each
/// worth more than what they buy here.
#[test]
fn a_card_is_not_given_up_for_a_small_gain() {
    let forests = ["Zuran Orb", "Forest", "Forest", "Forest", "Forest"];
    assert_eq!(
        board_after_turn_one(&forests),
        forests.map(String::from),
        "the Orb ate the lands"
    );
    let elves = ["Viscera Seer", "Llanowar Elves", "Llanowar Elves", "Forest"];
    assert_eq!(
        board_after_turn_one(&elves),
        elves.map(String::from),
        "the Seer ate a creature"
    );
}

#[test]
fn x_cannot_demand_more_graveyard_targets_than_exist() {
    let preset = position(
        &["Entreat the Dead"],
        &["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"],
    );
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::EXPERT);
    for seq in 0..80 {
        let pending = engine.pending();
        let seat = pending_player(pending).unwrap();
        let view = asked_view(engine.state(), seat, seq, pending);
        if view.turn > 1 {
            return;
        }
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, pending, &engine.decision_context()),
        };
        if matches!(pending, Pending::ChooseNumber { .. }) {
            assert_eq!(action, PlayerAction::ChooseNumber(0));
        }
        engine
            .apply(seat, action)
            .expect("no under-targeted X cast may roll back");
    }
    panic!("the decision must make progress");
}

/// Beta.6: the house held Camouflage with mana open all game. It tapped a
/// Forest for it in the main phase, where its "declare attackers step only"
/// keeps the engine from taking the cast, and the mana emptied. Now it is
/// cast after the attack is declared into an untapped blocker, and only then.
#[test]
fn the_ai_casts_camouflage_once_it_attacks_into_a_blocker() {
    let mut preset = position(
        &["Camouflage"],
        &["Forest", "Grizzly Bears", "Grizzly Bears"],
    );
    preset.seats[1].starting_battlefield = vec![entry("Grizzly Bears")];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    let agent = HeuristicAgent::new(AIProfile::default());
    let mut cast_in = None;
    for seq in 0..300 {
        let pending = engine.pending().clone();
        let Some(seat) = pending_player(&pending) else {
            break;
        };
        let view = asked_view(engine.state(), seat, seq, &pending);
        if view.turn > 1 {
            break;
        }
        let action = match &pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } if seat == PlayerId::new(1) => PlayerAction::PassPriority,
            _ => agent.act_with_context(&view, &pending, &engine.decision_context()),
        };
        if seat == PlayerId::new(0) {
            if matches!(action, PlayerAction::ActivateManaAbility { .. }) {
                assert_eq!(
                    view.step,
                    baylee_view::Step::DeclareAttackers,
                    "mana tapped early"
                );
            }
            if matches!(action, PlayerAction::CastSpell { .. }) {
                cast_in = Some(view.step);
            }
        }
        engine.apply(seat, action).expect("legal");
    }
    assert_eq!(cast_in, Some(baylee_view::Step::DeclareAttackers));
}
