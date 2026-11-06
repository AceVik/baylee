//! Each group on a real table: the engine sets the board, the seat's own view is
//! what the rule reads.
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::preset::{AIProfile, FormatId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use super::TacticRules4;

const ME: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

/// A table with these hands and boards, at the seat's first main phase.
fn table(
    my_hand: &[&str],
    my_board: &[&str],
    their_board: &[&str],
) -> (baylee_gamehost::Session, Pending, PlayerView) {
    let mut session = setup(my_hand, my_board, their_board, ME);
    for _ in 0..64 {
        if let Pending::Priority { player, .. } = session.pending()
            && *player == ME
        {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            if view.active == ME && view.phase == baylee_view::Phase::FirstMain {
                return (session, pending, view);
            }
        }
        step(&mut session);
    }
    panic!("first main not reached");
}

/// Answers the question up with the plainest answer.
fn step(session: &mut baylee_gamehost::Session) {
    let player = session.pending().asked().unwrap();
    let action = match session.pending() {
        Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
        Pending::Priority { .. } => PlayerAction::PassPriority,
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        },
        other => panic!("unexpected setup decision {other:?}"),
    };
    session.act(player, action).unwrap();
}

/// The seat's own declaration of attackers.
fn attack_question(
    my_board: &[&str],
    their_board: &[&str],
) -> (baylee_gamehost::Session, Pending, PlayerView) {
    let mut session = setup(&[], my_board, their_board, ME);
    for _ in 0..64 {
        if matches!(session.pending(), Pending::ChooseAttackers { player, .. } if *player == ME) {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            return (session, pending, view);
        }
        step(&mut session);
    }
    panic!("attack not reached");
}

/// The seat's block question, with every creature of the other side attacking.
fn block_question(
    my_board: &[&str],
    their_board: &[&str],
) -> (baylee_gamehost::Session, Pending, PlayerView) {
    let mut session = setup(&[], my_board, their_board, OTHER);
    for _ in 0..64 {
        if matches!(session.pending(), Pending::ChooseBlockers { player, .. } if *player == ME) {
            let (pending, view) = session.view_for_with_context(ME).unwrap();
            return (session, pending, view);
        }
        if let Pending::ChooseAttackers {
            player, attackers, ..
        } = session.pending()
            && *player == OTHER
        {
            let declared = attackers
                .iter()
                .map(|id| (*id, Defender::Player(ME)))
                .collect();
            session
                .act(
                    OTHER,
                    PlayerAction::DeclareAttackers {
                        attackers: declared,
                    },
                )
                .unwrap();
            continue;
        }
        step(&mut session);
    }
    panic!("block not reached");
}

fn setup(
    my_hand: &[&str],
    my_board: &[&str],
    their_board: &[&str],
    first: PlayerId,
) -> baylee_gamehost::Session {
    let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
    let mut preset = crate::selfplay::table_for(915, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
    preset.format = FormatId::Freeform;
    preset.modifiers.clear();
    preset.house_rules.starting_player = Some(first.get());
    let card = |name: &str| {
        let mut entry = preset.seats[0].deck[0];
        entry.card = baylee_cards::decks::by_name(name).unwrap();
        entry.rule_print = None;
        entry.choices = baylee_core::card_metadata::DeckChoices::default();
        entry
    };
    let hand: Vec<_> = my_hand.iter().map(|name| card(name)).collect();
    let mine: Vec<_> = my_board.iter().map(|name| card(name)).collect();
    let theirs: Vec<_> = their_board.iter().map(|name| card(name)).collect();
    for (seat, spec) in preset.seats.iter_mut().enumerate() {
        spec.commanders.clear();
        spec.starting_hand = Some(if seat == 0 { hand.clone() } else { vec![] });
        spec.starting_battlefield = if seat == 0 {
            mine.clone()
        } else {
            theirs.clone()
        };
    }
    let mut session = baylee_gamehost::Session::new_recorded(&preset, "rule-group-test").unwrap();
    session.take_over(ME);
    session.take_over(OTHER);
    session
}

fn card_in_hand(view: &PlayerView, name: &str) -> ObjectId {
    view.hand.iter().find(|card| card.name == name).unwrap().id
}

fn groups(rules: &str, view: &PlayerView, pending: &Pending, action: &PlayerAction) -> Vec<String> {
    TacticRules4::parse(rules)
        .unwrap()
        .audit(view, pending, action)
        .into_iter()
        .map(|hit| hit.0.to_owned())
        .collect()
}

#[test]
fn a_gate_no_permanent_opens_is_void() {
    let (_session, pending, view) = table(&["Pyroblast"], &["Mountain"], &["Plains"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pyroblast"),
    };
    assert_eq!(groups("void", &view, &pending, &cast), ["void"]);
    let (_session, pending, view) = table(&["Pyroblast"], &["Mountain"], &["Air Elemental"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pyroblast"),
    };
    assert!(groups("void", &view, &pending, &cast).is_empty());
}

#[test]
fn damage_counted_on_a_board_without_it_is_void() {
    let (_session, pending, view) = table(&["Typhoon"], &["Forest"], &["Plains"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Typhoon"),
    };
    assert_eq!(groups("void", &view, &pending, &cast), ["void"]);
    let (_session, pending, view) = table(&["Typhoon"], &["Forest"], &["Island"]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Typhoon"),
    };
    assert!(groups("void", &view, &pending, &cast).is_empty());
}

#[test]
fn a_sweep_of_the_seats_own_board_backfires() {
    let (_session, pending, view) = table(
        &["Pest Control"],
        &["Llanowar Elves", "Llanowar Elves"],
        &["Grizzly Bears"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Pest Control"),
    };
    assert_eq!(groups("mass", &view, &pending, &cast), ["mass"]);
}

#[test]
fn a_bounce_with_only_the_seats_own_creature_to_name_waits() {
    let (_session, pending, view) = table(&["Terror"], &["Swamp", "Swamp", "Grizzly Bears"], &[]);
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Terror"),
    };
    assert_eq!(groups("side", &view, &pending, &cast), ["side"]);
    let (_session, pending, view) = table(
        &["Terror"],
        &["Swamp", "Swamp", "Grizzly Bears"],
        &["Grizzly Bears"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Terror"),
    };
    assert!(groups("side", &view, &pending, &cast).is_empty());
}

#[test]
fn a_lord_is_no_gift_on_a_board_of_the_seats_own() {
    let (_session, pending, view) = table(
        &["Goblin King"],
        &["Mountain", "Goblin Piker", "Goblin Piker"],
        &["Plains"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Goblin King"),
    };
    assert!(groups("gift", &view, &pending, &cast).is_empty());
}

#[test]
fn a_lord_that_walks_their_goblins_through_my_mountain_is_a_gift() {
    let (_session, pending, view) = table(
        &["Goblin King"],
        &["Mountain", "Grizzly Bears", "Grizzly Bears"],
        &["Goblin Piker", "Goblin Piker", "Goblin Piker"],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Goblin King"),
    };
    assert_eq!(groups("gift", &view, &pending, &cast), ["gift"]);
}

fn every_attacker(pending: &Pending) -> PlayerAction {
    let Pending::ChooseAttackers { attackers, .. } = pending else {
        panic!("not an attack");
    };
    PlayerAction::DeclareAttackers {
        attackers: attackers
            .iter()
            .map(|id| (*id, Defender::Player(OTHER)))
            .collect(),
    }
}

#[test]
fn an_attack_that_loses_the_race_stays_home() {
    let wurms = ["Craw Wurm", "Craw Wurm", "Craw Wurm"];
    let (_session, pending, view) = attack_question(&["Grizzly Bears", "Grizzly Bears"], &wurms);
    let attack = every_attacker(&pending);
    assert_eq!(groups("race", &view, &pending, &attack), ["race"]);
}

#[test]
fn a_race_with_nothing_coming_back_is_left_alone() {
    let (_session, pending, view) =
        attack_question(&["Grizzly Bears", "Grizzly Bears"], &["Llanowar Elves"]);
    let attack = every_attacker(&pending);
    assert!(groups("race", &view, &pending, &attack).is_empty());
}

#[test]
fn a_free_swing_left_at_home_is_pressed_unless_it_opens_the_seat() {
    let none = PlayerAction::DeclareAttackers {
        attackers: Vec::new(),
    };
    let (_session, pending, view) = attack_question(&["Grizzly Bears"], &[]);
    assert_eq!(groups("press", &view, &pending, &none), ["press"]);
    let (_session, pending, view) = attack_question(&["Grizzly Bears"], &["Air Elemental"]);
    assert!(groups("press", &view, &pending, &none).is_empty());
    let wurms = ["Craw Wurm", "Craw Wurm", "Craw Wurm", "Craw Wurm"];
    let (_session, pending, view) = attack_question(&["Grizzly Bears"], &wurms);
    assert!(groups("press", &view, &pending, &none).is_empty());
}

#[test]
fn an_attacker_that_connects_for_value_is_blocked() {
    let none = PlayerAction::DeclareBlockers {
        blockers: Vec::new(),
    };
    let (_session, pending, view) = block_question(&["Grizzly Bears"], &["Goblin Lackey"]);
    assert_eq!(groups("trip", &view, &pending, &none), ["trip"]);
    let (_session, pending, view) = block_question(&["Grizzly Bears"], &["Grizzly Bears"]);
    assert!(groups("trip", &view, &pending, &none).is_empty());
}

#[test]
fn an_even_trade_is_taken_and_a_bad_one_is_not() {
    let none = PlayerAction::DeclareBlockers {
        blockers: Vec::new(),
    };
    let (_session, pending, view) = block_question(&["Grizzly Bears"], &["Grizzly Bears"]);
    assert_eq!(groups("trade", &view, &pending, &none), ["trade"]);
    let (_session, pending, view) = block_question(&["Serra Angel"], &["Llanowar Elves"]);
    assert!(groups("trade", &view, &pending, &none).is_empty());
}

#[test]
fn a_blocker_that_lives_stands_in_the_way() {
    let none = PlayerAction::DeclareBlockers {
        blockers: Vec::new(),
    };
    let (_session, pending, view) = block_question(&["Wall of Stone"], &["Grizzly Bears"]);
    assert_eq!(groups("wall", &view, &pending, &none), ["wall"]);
    let (_session, pending, view) = block_question(&["Llanowar Elves"], &["Grizzly Bears"]);
    assert!(groups("wall", &view, &pending, &none).is_empty());
}

#[test]
fn a_hand_without_lands_goes_back() {
    let mut session = setup(
        &["Forest", "Grizzly Bears", "Grizzly Bears", "Grizzly Bears"],
        &[],
        &[],
        ME,
    );
    for _ in 0..8 {
        if matches!(session.pending(), Pending::Mulligan { player, .. } if *player == ME) {
            break;
        }
        step(&mut session);
    }
    let (pending, view) = session.view_for_with_context(ME).unwrap();
    assert!(matches!(pending, Pending::Mulligan { .. }));
    assert_eq!(
        groups("keep", &view, &pending, &PlayerAction::MulliganKeep),
        ["keep"]
    );
}

#[test]
fn the_drop_that_pays_this_turn_goes_first() {
    let (_session, pending, view) =
        table(&["Forest", "Thornwood Falls", "Llanowar Elves"], &[], &[]);
    let tapped = PlayerAction::PlayLand {
        card: card_in_hand(&view, "Thornwood Falls"),
    };
    let forest = PlayerAction::PlayLand {
        card: card_in_hand(&view, "Forest"),
    };
    assert_eq!(groups("land", &view, &pending, &tapped), ["land"]);
    assert!(groups("land", &view, &pending, &forest).is_empty());
}

#[test]
fn with_nothing_to_cast_the_tapped_land_uses_the_turn() {
    let (_session, pending, view) = table(&["Forest", "Thornwood Falls", "Craw Wurm"], &[], &[]);
    let forest = PlayerAction::PlayLand {
        card: card_in_hand(&view, "Forest"),
    };
    assert_eq!(groups("land", &view, &pending, &forest), ["land"]);
}

#[test]
fn a_free_shot_waits_for_something_to_kill() {
    let shot = |view: &PlayerView| PlayerAction::ActivateAbility {
        source: view
            .battlefield
            .iter()
            .find(|object| object.name == "Mogg Fanatic")
            .unwrap()
            .id,
        ability_index: 0,
    };
    let (_session, pending, view) = table(&[], &["Mogg Fanatic"], &[]);
    assert_eq!(groups("ping", &view, &pending, &shot(&view)), ["ping"]);
    let (_session, pending, view) = table(&[], &["Mogg Fanatic"], &["Llanowar Elves"]);
    assert!(groups("ping", &view, &pending, &shot(&view)).is_empty());
}

#[test]
fn a_cheap_spell_does_not_strand_the_turn() {
    let forests = ["Forest", "Forest", "Forest"];
    let (_session, pending, view) = table(&["Grizzly Bears", "Trained Armodon"], &forests, &[]);
    let cheap = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Grizzly Bears"),
    };
    let dear = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Trained Armodon"),
    };
    assert_eq!(groups("curve", &view, &pending, &cheap), ["curve"]);
    assert!(groups("curve", &view, &pending, &dear).is_empty());
}

#[test]
fn a_second_legend_does_nothing() {
    let (_session, pending, view) = table(
        &["Isamaru, Hound of Konda"],
        &["Plains", "Isamaru, Hound of Konda"],
        &[],
    );
    let cast = PlayerAction::CastSpell {
        card: card_in_hand(&view, "Isamaru, Hound of Konda"),
    };
    assert_eq!(groups("void", &view, &pending, &cast), ["void"]);
}

#[test]
fn most_mana_spent_is_a_subset_sum() {
    assert_eq!(super::opening::best_spend(&[2, 3, 7], 6), 5);
    assert_eq!(super::opening::best_spend(&[4, 4], 7), 4);
    assert_eq!(super::opening::best_spend(&[], 3), 0);
}

#[test]
fn no_group_on_changes_nothing() {
    let rules = TacticRules4::parse("").unwrap();
    assert!(!rules.any());
    assert!(TacticRules4::parse("all").unwrap().any());
    assert!(TacticRules4::parse("nonsense").is_err());
}
