//! A decision this seat makes for the player whose turn it controls
//! (Mindslaver, CR 723.5), judged from that player's chair: that player's
//! hand, lands and pool, that player's turn on the rail, that player's
//! opponents as the other side. The seat's own cards stand beside them in
//! every view here, and none of them is ever used.

use super::*;
use baylee_view::SharedHand;

/// The seat's own Forest, which a decision for P2 never taps.
const MY_FOREST: u32 = 90;
/// The seat's own Giant Growth, which a decision for P2 never casts.
const MY_GROWTH: u32 = 91;

/// P2's precombat main phase, with this seat deciding for P2: P2's
/// `permanents` from slot 1 (each offering its tap, as [`table`] offers
/// them), P2's `hand` from slot 20 in the hand the seat controls, and the
/// seat's own Forest and Giant Growth beside them.
fn controlling(permanents: &[&str], hand: &[&str]) -> (PlayerView, LegalActions) {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, [permanent(MY_FOREST, "Forest")])
        .with_battlefield(
            1,
            permanents
                .iter()
                .zip(1..)
                .map(|(name, slot)| permanent(slot, name)),
        )
        .build();
    view.hand = vec![in_hand(MY_GROWTH, "Giant Growth")];
    view.controlled_hands = vec![SharedHand {
        player: THEM,
        public: false,
        cards: hand
            .iter()
            .zip(20..)
            .map(|(name, slot)| in_hand(slot, name))
            .collect(),
    }];
    view.active = THEM;
    view.awaiting = Some(ME);
    view.decision_player = Some(THEM);
    let basic = |name: &&str| ["Forest", "Island", "Mountain", "Plains", "Swamp"].contains(name);
    let mut legal = LegalActions {
        can_pass: true,
        payable: view.controlled_hands[0]
            .cards
            .iter()
            .filter(|card| !card.types.contains(TypeSet::LAND))
            .map(|card| card.id)
            .collect(),
        ..LegalActions::default()
    };
    for (name, slot) in permanents.iter().zip(1..) {
        if basic(name) {
            legal.mana_abilities.push(o(slot));
        } else {
            legal.abilities.push((o(slot), 0));
        }
    }
    (view, legal)
}

fn their_priority(legal: LegalActions) -> Pending {
    Pending::Priority {
        player: THEM,
        legal: Box::new(legal),
    }
}

fn judge_theirs(view: &PlayerView, legal: LegalActions) -> Verdict {
    WakeFilter::default().judge(view, &their_priority(legal), &[])
}

/// The reach planner casts the controlled player's card with the
/// controlled player's land, and never reaches the seat's own hand or
/// taps the seat's own Forest.
#[test]
fn the_reach_of_a_controlled_player_taps_that_players_lands() {
    let (view, legal) = controlling(&["Forest"], &["Llanowar Elves"]);
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 1, "{reach:?}");
    assert_eq!(reach[0].object, o(20), "P2's Elves, not the seat's Growth");
    assert_eq!(
        reach[0]
            .plan
            .steps
            .iter()
            .map(|step| step.source)
            .collect::<Vec<_>>(),
        vec![o(1)],
        "P2's Forest pays, not the seat's own #{MY_FOREST}"
    );
}

/// The pool read is the controlled player's: P2's floating {G} pays for
/// the Elves (nothing to tap, so no reach), the seat's own {G} does not.
#[test]
fn the_pool_of_a_controlled_decision_is_that_players() {
    let (mut view, legal) = controlling(&["Forest"], &["Llanowar Elves"]);
    view.seats[0].mana_pool.green = 1;
    assert_eq!(reachable(&view, &legal).len(), 1, "the seat's own pool");
    view.seats[1].mana_pool.green = 1;
    assert!(reachable(&view, &legal).is_empty(), "P2's pool pays it");
}

/// P2's main phase is P2's own turn: on the default rail it is a stop,
/// with P2's creature to cast. The same window decided by the seat for
/// itself (P2's turn, the seat's own cards) is quiet.
#[test]
fn the_controlled_players_main_phase_is_a_stop_on_its_rail() {
    let (view, legal) = controlling(&["Forest"], &["Llanowar Elves"]);
    assert_eq!(judge_theirs(&view, legal), Verdict::Wake(Why::RailStop));

    let (mut own, legal) = table(&["Forest"], &["Giant Growth"]);
    own.active = THEM;
    assert_eq!(
        judge(&own, legal),
        passes(Standing::QuietWindow),
        "the seat's own instant in P2's main phase waits for a stop"
    );
}

/// Nothing to do for the controlled player is passed for it, whatever the
/// seat itself could do: its own Forest and Giant Growth are not P2's.
#[test]
fn nothing_to_do_for_the_controlled_player_is_passed() {
    let (view, legal) = controlling(&["Forest"], &[]);
    assert_eq!(judge_theirs(&view, legal), passes(Standing::NothingToDo));
    let (view, legal) = controlling(&["Island"], &["Llanowar Elves"]);
    assert_eq!(judge_theirs(&view, legal), passes(Standing::NothingToDo));
}

/// The seat's own spell on the stack is the other side's for the player
/// it decides for, and with an instant to answer it that is a decision.
#[test]
fn the_seats_own_spell_is_the_other_side_for_the_controlled_player() {
    let (mut view, legal) = controlling(&["Forest"], &["Giant Growth"]);
    view.stack.push(token(80, 0, "TEST spell", 0, 0));
    assert_eq!(
        judge_theirs(&view, legal.clone()),
        Verdict::Wake(Why::OpposingStack)
    );
    // P2's own spell on the stack in P2's main phase is not.
    view.stack = vec![token(80, 1, "TEST spell", 0, 0)];
    assert_eq!(judge_theirs(&view, legal), Verdict::Wake(Why::RailStop));
}

/// A question for the controlled player is this seat's to answer: judged
/// as one, never "not asked". The same question while another seat decides
/// for P2 is not this seat's.
#[test]
fn a_question_for_the_controlled_player_is_this_seats() {
    let (mut view, _) = controlling(&["Forest"], &[]);
    let colour = Pending::ChooseColor {
        player: THEM,
        options: vec![ManaColor::Red, ManaColor::Green],
    };
    let mut filter = WakeFilter::default();
    assert_eq!(
        filter.judge(&view, &colour, &[]),
        Verdict::Wake(Why::Question)
    );
    view.awaiting = Some(THEM);
    assert_eq!(
        filter.judge(&view, &colour, &[]),
        Verdict::Wake(Why::NotAsked)
    );
}

/// The controlled player's attack is a declaration when it has a creature
/// to attack with, and declared empty for it when it has none.
#[test]
fn the_controlled_players_attack_is_declared_by_the_mind() {
    let (view, _) = controlling(&["Forest", "Grizzly Bears"], &[]);
    let attack = |attackers| Pending::ChooseAttackers {
        alone: Vec::new(),
        minimum_required: None,
        player: THEM,
        attackers,
        defenders: vec![Defender::Player(ME)],
        required: Vec::new(),
        limits: Vec::new(),
    };
    let mut filter = WakeFilter::default();
    assert_eq!(
        filter.judge(&view, &attack(vec![o(2)]), &[]),
        Verdict::Wake(Why::Declaration)
    );
    assert_eq!(
        filter.judge(&view, &attack(Vec::new()), &[]),
        standing(
            PlayerAction::DeclareAttackers {
                attackers: Vec::new()
            },
            Standing::NoAttackers
        )
    );
}

/// After blocks, the window is a stop for the controlled player when one
/// of its creatures attacks: its side fights in that combat.
#[test]
fn the_controlled_players_combat_is_its_own() {
    let (mut view, legal) = controlling(&["Forest", "Grizzly Bears"], &["Giant Growth"]);
    view.phase = Phase::Combat;
    view.step = Step::DeclareBlockers;
    assert_eq!(
        judge_theirs(&view, legal.clone()),
        passes(Standing::QuietWindow),
        "no combat"
    );
    view.combat.attackers.push(baylee_view::AttackerView {
        creature: o(2),
        defending: Defender::Player(ME),
        blocked: false,
    });
    assert_eq!(judge_theirs(&view, legal), Verdict::Wake(Why::RailStop));
}

/// An `until` held for the controlled player passes its windows and
/// leaves the seat's own filter alone: each has its own.
#[test]
fn an_until_for_the_controlled_player_is_held_in_its_own_filter() {
    let (view, legal) = controlling(&["Forest"], &["Llanowar Elves"]);
    let own = WakeFilter::default();
    let mut theirs = own.fresh();
    theirs.apply_orders(
        None,
        Some(Orders {
            until: Some(Until::EndOfTurn),
            react: React::All,
        }),
        0,
        &view,
        &[],
    );
    assert_eq!(
        theirs.judge(&view, &their_priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    assert_eq!(
        own.clone().judge(&view, &their_priority(legal), &[]),
        Verdict::Wake(Why::RailStop),
        "the seat's own filter holds no until"
    );
}
