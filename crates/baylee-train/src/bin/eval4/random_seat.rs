//! `--random-policy`: the seat under test answers uniformly at random among
//! the answers each question offers, pick by pick
//! ([`baylee_train::policy::random_answer`]). It is the floor a trained
//! policy has to clear: a net that wins no more often than this seat plays
//! no better than chance.
//!
//! Seeded by the deal and the seat, so a run repeats move for move. A
//! question kind the net does not answer, or a pick that makes no answer, is
//! left to the house, as for the net, and counted as a fallback. Its answers
//! count as the seat's own (`SourceKind4::Net`) in the summary; no record of
//! it is written (`--record-out` is refused), so no dataset can take them
//! for a model's.
use super::{CoverageMatrix4, EvaluationPlayer, Pending};
use baylee_core::ids::ObjectId;
use baylee_core::preset::GamePreset;
use baylee_train::deckgen::Rng;
use baylee_train::features4::{SourceKind4, StepProvenance4, TeacherTarget4};
use baylee_train::netplay4::Answer4;
use baylee_view::PlayerView;

/// Keeps the random seat's stream apart from the house's, which seeds from
/// the same deal.
const STREAM: u64 = 0x5241_4E44_4F4D_0001;

pub(super) struct RandomPlayer {
    rng: Rng,
    seed: u64,
    coverage: CoverageMatrix4,
}

impl RandomPlayer {
    pub fn new() -> Self {
        Self {
            rng: Rng::new(STREAM),
            seed: 0,
            coverage: CoverageMatrix4::default(),
        }
    }
}

impl EvaluationPlayer for RandomPlayer {
    fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = Rng::new(seed ^ STREAM);
    }

    fn init_game(&mut self, _: &GamePreset) {
        self.rng = Rng::new(self.seed ^ STREAM);
    }

    fn observe(&mut self, _: &PlayerView) -> anyhow::Result<()> {
        Ok(())
    }

    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        _: &[Option<u8>],
    ) -> anyhow::Result<Option<Answer4>> {
        let hand: Vec<ObjectId> = view.hand.iter().map(|card| card.id).collect();
        let Ok(Some(action)) = baylee_train::policy::random_answer(pending, &hand, &mut self.rng)
        else {
            return Ok(None);
        };
        Ok(Some(Answer4 {
            action,
            value: 0.5,
            provenance: StepProvenance4 {
                source: SourceKind4::Net,
                has_logprob: false,
                ..StepProvenance4::default()
            },
            target: TeacherTarget4::default(),
        }))
    }

    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    /// Plays `steps` answers of the random seat for whoever is asked, the
    /// house where it leaves a question, and lists what it chose.
    fn play(seed: u64, steps: usize) -> Vec<String> {
        let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
        let preset = baylee_train::selfplay::table_for(
            31,
            &[&deck, &deck],
            &[baylee_core::preset::AIProfile::STEADY; 2],
            &[],
        );
        let mut session = baylee_gamehost::Session::new(&preset).unwrap();
        let seats: Vec<PlayerId> = (0..2).map(PlayerId::new).collect();
        for s in &seats {
            session.take_over(*s);
        }
        let mut seat = RandomPlayer::new();
        seat.reseed(seed);
        seat.init_game(&preset);
        let mut chosen = Vec::new();
        while chosen.len() < steps && !matches!(session.pending(), Pending::GameOver(_)) {
            session.pump_at_most(32);
            for me in &seats {
                let Some((pending, view)) = session.view_for(*me) else {
                    continue;
                };
                let action = match seat.answer(&view, &pending, &[None, None]).unwrap() {
                    Some(answer) => answer.action,
                    None => session.house_action(*me).unwrap(),
                };
                chosen.push(format!("{action:?}"));
                session
                    .act(*me, action)
                    .expect("an offered answer is accepted");
            }
        }
        chosen
    }

    /// The same deal plays the same answers, another seed others, and every
    /// answer is one the engine accepts.
    #[test]
    fn the_random_seat_repeats_by_seed_and_plays_legally() {
        let first = play(7, 60);
        assert!(first.len() >= 60, "{first:?}");
        assert_eq!(play(7, 60), first);
        assert_ne!(play(8, 60), first);
    }
}
