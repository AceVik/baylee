//! `--house-as-seat`: the house chairs answer as a seat's `--mind house` does
//! over the gateway (`baylee_seat::HouseMind`, the ladder's `house:<level>`),
//! so the in-process benchmark and the seat path can be told apart by their
//! opponent alone. An in-process chair answers with the engine's decision
//! context and the host's report of its own deck (`Session::house_answer`);
//! a seat answers from what it holds: the view its socket is sent, the note
//! beside its question (`Session::question_note`) and its own deck list,
//! through the one path both use (`baylee_ai::noted::answer_as_seated`).

use baylee_ai::HeuristicAgent;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;

/// The house at `player`'s chair as `baylee_seat::HouseMind::answer` builds
/// it: at `profile`, told the sides, seeded as a house chair at a named
/// table. It answers through `baylee_ai::noted::answer_as_seated`.
pub(crate) fn agent(
    game_id: &str,
    player: PlayerId,
    profile: AIProfile,
    teams: &[Option<u8>],
) -> HeuristicAgent {
    HeuristicAgent::new(profile)
        .with_teams(teams.to_vec())
        .with_seed(baylee_ai::policy_seed(game_id, player.get()))
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "llm")]
    use super::agent;
    use crate::{Args, collector, house_as_seat_refusal};
    #[cfg(feature = "llm")]
    use baylee_core::ids::PlayerId;
    #[cfg(feature = "llm")]
    use baylee_core::preset::AIProfile;
    use clap::Parser as _;

    /// `--fair-scouting` stays on by default and cannot be turned off, so
    /// a benchmark never meets a peeking chair; `seer` is let peek by a
    /// switch of its own, named for what it does.
    #[test]
    fn only_a_named_switch_lets_the_seer_peek() {
        let base = ["eval4", "--model", "m", "--out", "o"];
        let plain = Args::try_parse_from(base).unwrap();
        assert!(plain.fair_scouting && !plain.seer_peeks);
        let mut argv = base.to_vec();
        argv.push("--seer-peeks");
        let peeking = Args::try_parse_from(argv).unwrap();
        assert!(peeking.fair_scouting && peeking.seer_peeks);
    }

    #[test]
    fn a_seated_house_is_a_measurement_against_the_house_only() {
        // An evaluation's contract; an opponent's would read its model.
        let base = ["eval4", "--model", "m", "--out", "o"];
        let contract =
            collector::Contract::new(&Args::try_parse_from(base).unwrap(), None).unwrap();
        let refusal = |extra: &[&str]| {
            let mut argv = base.to_vec();
            argv.extend_from_slice(extra);
            house_as_seat_refusal(&Args::try_parse_from(argv).unwrap(), &contract)
        };
        assert_eq!(refusal(&[]), None);
        assert_eq!(refusal(&["--house-as-seat", "--no-baseline"]), None);
        assert!(
            refusal(&["--house-as-seat"]).is_some(),
            "the baseline game's house would answer in-process"
        );
        assert!(refusal(&["--house-as-seat", "--no-baseline", "--record-out", "r"]).is_some());
        assert!(refusal(&["--house-as-seat", "--no-baseline", "--search", "4,2,0"]).is_some());
        assert!(refusal(&["--house-as-seat", "--no-baseline", "--opponent-model", "n"]).is_some());
    }

    /// The seated house answers every question of a deal exactly as the
    /// seat bridge's house mind does, on the view a socket is sent.
    #[cfg(feature = "llm")]
    #[test]
    fn a_seated_house_answers_as_the_seat_bridges_house_mind() {
        use baylee_core::preset::FormatId;
        use baylee_gamehost::Session;
        use baylee_seat::{DeckList, GameContext, HouseMind};

        let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
        let preset =
            baylee_train::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::SHARP; 2], &[]);
        let mut session = Session::new(&preset).unwrap();
        let game = "seated-house-test";
        let teams = [None, None];
        let players = [PlayerId::new(0), PlayerId::new(1)];
        for player in players {
            session.take_over(player);
        }
        let mind = HouseMind::new(AIProfile::SHARP);
        let mut answered = 0;
        while answered < 200 {
            let Some((player, (pending, view))) = players
                .iter()
                .find_map(|player| Some((*player, session.view_for_with_context(*player)?)))
            else {
                break;
            };
            let spec = &preset.seats[usize::from(player.get())];
            let list = DeckList {
                name: String::new(),
                main: spec
                    .deck
                    .iter()
                    .map(|e| baylee_seat::DeckCard {
                        card: e.card,
                        count: 1,
                    })
                    .collect(),
                sideboard: Vec::new(),
                commanders: Vec::new(),
            };
            let context = GameContext {
                game_id: game.to_owned(),
                seat: player,
                seats: 2,
                teams: teams.to_vec(),
                format: FormatId::Freeform,
                deck: list,
                decision_secs: None,
            };
            let note = session.question_note(player);
            let deck = baylee_ai::intelligence::DeckIntel::new(
                spec.deck.iter().map(|e| e.card).collect(),
                Vec::new(),
            );
            let action = baylee_ai::noted::answer_as_seated(
                &agent(game, player, AIProfile::SHARP, &teams),
                view.clone(),
                &pending,
                note.as_ref(),
                Some(&deck),
            );
            assert_eq!(
                action,
                mind.answer_noted(&context, &view, &pending, note.as_ref())
            );
            session.act_slice(player, action, 0).unwrap();
            answered += 1;
        }
        assert!(answered > 20, "the deal asked something: {answered}");
    }
}
