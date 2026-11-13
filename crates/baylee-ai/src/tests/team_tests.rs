//! The house plays as a team (owner, 10.11.2026, `AIProfile::TEAM_PLAY`):
//! an ally's answer on the stack is the side's, an ally's board is the
//! side's, and an ally who holds an answer up lets this seat develop. The
//! frozen reference plays as it was frozen, so each test asks it too.
use super::*;

/// Seats 0 and 1 on one side, seat 2 on the other.
const SIDES: [Option<u8>; 3] = [Some(1), Some(1), Some(2)];

fn on_a_team(profile: AIProfile) -> HeuristicAgent {
    HeuristicAgent::new(profile).with_teams(SIDES.to_vec())
}

fn priority(me: PlayerId, castable: Vec<ObjectId>) -> Pending {
    Pending::Priority {
        player: me,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable,
            ..Default::default()
        }),
    }
}

/// **An ally's removal on its way is the side's.** The same position as
/// `removal_is_not_cast_at_what_this_seats_own_removal_is_taking`, with the
/// Path an ally's: every level lets it take the creature and keeps its
/// Swords, and the frozen reference casts the Swords as it always did.
#[test]
fn an_allys_removal_on_the_stack_answers_the_target_for_the_side() {
    let (me, ally, them) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut v = view(0, &[20, 20, 20], vec![permanent(obj(7), them, 5)]);
    v.phase = baylee_view::Phase::FirstMain;
    v.seats[0].mana_pool.white = 1;
    v.hand = vec![hand_card(1, "Swords to Plowshares")];
    v.stack = vec![stack_spell(6, "Path to Exile", ally, obj(7))];
    baylee_client_core::test_support::project_current_targets(&mut v);
    let pending = priority(me, vec![obj(1)]);
    for (name, profile) in AIProfile::NAMED {
        assert_eq!(
            on_a_team(profile).act(&v, &pending),
            PlayerAction::PassPriority,
            "{name}: its ally's Path is already taking the 5/5"
        );
    }
    assert_eq!(
        on_a_team(AIProfile::SHARP_2026_11).act(&v, &pending),
        PlayerAction::CastSpell { card: obj(1) },
        "the reference plays as it was frozen"
    );
}

/// **An ally's board is the side's.** Wrath of God against one opposing
/// creature, with three of the ally's on the table: a fair trade for a
/// seat that counts only its own board, and three for one against the
/// side for a seat that plays as a team.
#[test]
fn a_sweeper_counts_what_it_takes_from_an_ally() {
    let (me, ally, them) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let board = vec![
        permanent(obj(7), them, 4),
        permanent(obj(8), ally, 3),
        permanent(obj(9), ally, 3),
        permanent(obj(10), ally, 3),
    ];
    let mut v = view(0, &[20, 20, 20], board);
    v.phase = baylee_view::Phase::FirstMain;
    v.seats[0].mana_pool.white = 4;
    v.hand = vec![hand_card(1, "Wrath of God")];
    let pending = priority(me, vec![obj(1)]);
    for (name, profile) in AIProfile::NAMED {
        assert_eq!(
            on_a_team(profile).act(&v, &pending),
            PlayerAction::PassPriority,
            "{name} swept its ally's three creatures to take one"
        );
    }
    assert_eq!(
        on_a_team(AIProfile::SHARP_2026_11).act(&v, &pending),
        PlayerAction::CastSpell { card: obj(1) },
        "the reference counts its own board alone, as it was frozen"
    );
}

/// **An ally who holds the answer up lets this seat develop.** Seat 1
/// keeps a creature in play and a Counterspell in hand with one land: on
/// its own it keeps that land up for the counter rather than play a
/// Raging Goblin with it (a counterspell is what a level that times its
/// instants keeps mana up for, `AIProfile::INSTANT_TIMING`; before it, a
/// Lightning Bolt was kept up too). Shown that its ally in
/// the lower seat holds a Counterspell over two untapped Islands, it plays
/// its second creature instead. The ally in the lower seat is the one who
/// holds, so two allies who see each other never both tap out.
#[test]
fn an_ally_holding_the_answer_lets_this_seat_develop() {
    let (me, ally, them) = (PlayerId::new(1), PlayerId::new(0), PlayerId::new(2));
    let island = |id: u32, controller: PlayerId| {
        let mut land = carded(permanent(obj(id), controller, 0), "Island", TypeSet::LAND);
        land.power = None;
        land.toughness = None;
        land
    };
    let board = vec![
        permanent(obj(20), me, 2),
        carded(permanent(obj(21), me, 0), "Mountain", TypeSet::LAND),
        island(30, ally),
        island(31, ally),
        permanent(obj(40), them, 3),
    ];
    let mut v = view(1, &[20, 20, 20], board);
    v.phase = baylee_view::Phase::FirstMain;
    v.active = me;
    v.awaiting = Some(me);
    v.hand = vec![hand_card(1, "Counterspell"), hand_card(2, "Raging Goblin")];
    v.seats[2].hand_count = 3;
    let pending = Pending::Priority {
        player: me,
        legal: Box::new(baylee_engine::choice::LegalActions {
            can_pass: true,
            castable: vec![obj(2)],
            ..Default::default()
        }),
    };
    let alone = v.clone();
    v.shared_hands.push(baylee_view::SharedHand {
        player: ally,
        public: false,
        cards: vec![hand_card(5, "Counterspell")],
    });
    for (name, profile) in AIProfile::NAMED {
        if profile.hold_up == baylee_core::preset::HoldUp::None {
            continue;
        }
        let with = on_a_team(profile).act(&v, &pending);
        let without = on_a_team(profile).act(&alone, &pending);
        assert_ne!(
            with, without,
            "{name}: its ally's Counterspell changed nothing"
        );
    }
    let reference = on_a_team(AIProfile::SHARP_2026_11);
    assert_eq!(
        reference.act(&v, &pending),
        reference.act(&alone, &pending),
        "the reference reads no ally's hand"
    );
}
