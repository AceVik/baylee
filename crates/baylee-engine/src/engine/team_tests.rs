//! Teams: who is an opponent, and who has won.
//!
//! Team Free-for-all changes exactly two things in the rules kernel and
//! nothing else. "Opponent" (CR 102.3) stops meaning "another seat" and
//! starts meaning "a seat on another side", which is why every rule that
//! says opponent goes through [`GameState::is_opponent`]. And the game ends
//! when one *side* is left rather than one player (CR 104.2b), which is why
//! a lone survivor with two dead teammates still wins for the team.
//!
//! Turns stay individual and life totals stay separate — that is what makes
//! this format the free-for-all and not Two-Headed Giant.

use super::*;
use crate::engine::testkit::RegistryLookup;
use crate::state::Side;
use crate::win::{EndReason, Victor};
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};

fn forest() -> CardIndex {
    crate::engine::testkit::card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

/// Halimar Excavator: a plain ground creature, which is all a test about
/// who may be attacked needs standing on the battlefield.
fn creature() -> CardIndex {
    crate::engine::testkit::card_index("fd3e37c9-93bf-4f3e-a279-22afbffd8d43")
}

/// A table whose seats carry the given teams, each on a plain deck.
fn table(teams: &[Option<u8>], battlefield: &[CardIndex], seed: u64) -> Engine<RegistryLookup> {
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: forest(),
            print: PrintRef::new(0),
        })
        .collect();
    let preset = GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: teams
            .iter()
            .map(|team| SeatSpec {
                controller: SeatController::Ai(AIProfile::default()),
                capabilities: baylee_core::preset::SeatCapabilities::default(),
                deck: deck.clone(),
                sideboard: vec![],
                starting_life: None,
                starting_hand: None,
                starting_battlefield: battlefield
                    .iter()
                    .map(|card| DeckEntry {
                        card: *card,
                        print: PrintRef::new(0),
                    })
                    .collect(),
                emblems: vec![],
                team: *team,
            })
            .collect(),
    };
    let mut engine = Engine::new(&preset, RegistryLookup).expect("table starts");
    for _ in 0..teams.len() {
        let Pending::Mulligan { player, .. } = engine.pending().clone() else {
            panic!("expected a mulligan, got {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    engine
}

/// Passes priority (declaring nothing in combat) until the game is over.
#[track_caller]
fn play_out(engine: &mut Engine<RegistryLookup>) -> crate::win::GameResult {
    for _ in 0..200 {
        match engine.pending().clone() {
            Pending::GameOver(result) => return result,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while passing: {other:?}"),
        }
    }
    panic!("the game never ended");
}

#[test]
fn a_seat_with_no_team_is_a_side_of_its_own() {
    let engine = table(&[None, None, None], &[], 7);
    let state = engine.state();
    assert_eq!(
        state.side_of(PlayerId::new(0)),
        Side::Solo(PlayerId::new(0))
    );
    // Two teamless seats must not compare equal — the reason `Side` is an
    // enum over the seat rather than an `Option<u8>`.
    assert_ne!(
        state.side_of(PlayerId::new(0)),
        state.side_of(PlayerId::new(1))
    );
    assert!(state.is_opponent(PlayerId::new(1), PlayerId::new(0)));
    assert!(!state.is_opponent(PlayerId::new(0), PlayerId::new(0)));
}

#[test]
fn a_teammate_is_not_an_opponent() {
    let engine = table(&[Some(1), Some(1), None], &[], 11);
    let state = engine.state();
    let (a, b, c) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    assert!(!state.is_opponent(b, a), "teammates are not opponents");
    assert!(state.is_opponent(c, a));
    assert!(state.is_opponent(a, c));

    // "Each opponent" is the set the rule names, so it is two seats at this
    // table and not three minus yourself.
    let opponents = crate::eval::players(baylee_cards_dsl::PlayerRel::EachOpponent, state, a);
    assert_eq!(opponents, vec![c]);
    let everyone = crate::eval::players(baylee_cards_dsl::PlayerRel::EachPlayer, state, a);
    assert_eq!(everyone, vec![a, b, c]);
}

#[test]
fn a_teammate_and_their_planeswalkers_cannot_be_attacked() {
    let mut engine = table(&[Some(1), Some(1), None], &[creature()], 13);
    // Walk to the first declaration of attackers; the active seat is 0.
    for _ in 0..40 {
        if let Pending::ChooseAttackers {
            player, defenders, ..
        } = engine.pending().clone()
        {
            assert_eq!(player, PlayerId::new(0));
            // Only the seat on the other side, never the teammate.
            assert!(
                defenders.contains(&baylee_core::ids::Defender::Player(PlayerId::new(2))),
                "the opponent is attackable: {defenders:?}"
            );
            assert!(
                !defenders.contains(&baylee_core::ids::Defender::Player(PlayerId::new(1))),
                "a teammate must not be attackable: {defenders:?}"
            );
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected pending: {:?}", engine.pending())
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("never reached a declaration of attackers");
}

#[test]
fn the_game_ends_when_one_team_is_left_standing() {
    let mut engine = table(&[Some(1), Some(1), Some(2), Some(2)], &[], 17);
    // Both seats on team 2 are dealt out at once; a state-based-action pass
    // runs before the next priority grant and finds them.
    engine.state.players[2].life = 0;
    engine.state.players[3].life = 0;
    let result = play_out(&mut engine);
    assert_eq!(result.winner, Some(Victor::Team(1)));
    assert_eq!(result.reason, EndReason::LastTeamStanding);
}

#[test]
fn a_sole_survivor_wins_for_the_whole_team() {
    // CR 104.2b: the team wins, not the seat that happened to survive. Seat
    // 1 is on the winning team and dead, and the result still names the team.
    let mut engine = table(&[Some(1), Some(1), None], &[], 19);
    engine.state.players[1].life = 0;
    engine.state.players[2].life = 0;
    let result = play_out(&mut engine);
    assert_eq!(result.winner, Some(Victor::Team(1)));
    assert!(
        result
            .winner
            .expect("a winner")
            .includes(PlayerId::new(1), Some(1)),
        "a dead teammate is still on the winning team"
    );
}

#[test]
fn a_table_with_no_teams_ends_exactly_as_it_did_before() {
    let mut engine = table(&[None, None, None], &[], 23);
    engine.state.players[1].life = 0;
    engine.state.players[2].life = 0;
    let result = play_out(&mut engine);
    assert_eq!(result.winner, Some(Victor::Player(PlayerId::new(0))));
    assert_eq!(result.reason, EndReason::LastPlayerStanding);
}

/// "Target opponent" is the same enumeration wherever it is asked â the cast
/// wizard's player choice included, which used to build its own list and so
/// offered the caster their own face.
#[test]
fn target_opponent_offers_neither_you_nor_your_teammate() {
    use baylee_cards_dsl::TargetSpec;

    let engine = table(&[Some(1), Some(1), Some(2)], &[], 23);
    let state = engine.state();
    let (a, b, c) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));

    assert_eq!(
        crate::eval::target_player_options(state, &TargetSpec::AnyOpponent, a),
        vec![c],
        "target opponent offered a seat that is not one"
    );
    // "Any player" is every player, teammate and self included (CR 115.1).
    assert_eq!(
        crate::eval::target_player_options(state, &TargetSpec::AnyPlayer, a),
        vec![a, b, c]
    );
}
