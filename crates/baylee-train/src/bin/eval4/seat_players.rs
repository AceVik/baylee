//! Independent policy instances and RNG streams for the bounded 1v1 collector.
use super::collector::Contract;
use super::{
    CatalogPolicy4, CoverageMatrix4, EvaluationPlayer, ModelFormat, Pending, PolicyMode4,
    SharedNets, load_player,
};
use anyhow::{Context as _, ensure};
use baylee_core::preset::GamePreset;
use baylee_train::features4::PendingKind4;
use baylee_train::netplay4::{Answer4, FallbackDiagnostic4};
use baylee_view::PlayerView;
use std::path::Path;

pub(super) struct SeatPlayers {
    learner: Box<dyn EvaluationPlayer>,
    opponent: Option<Box<dyn EvaluationPlayer>>,
    focal: usize,
    last_was_learner: bool,
    coverage: CoverageMatrix4,
}

impl SeatPlayers {
    pub fn load(
        path: &Path,
        contract: &Contract,
        mode: PolicyMode4,
        shared: &SharedNets,
        catalog: CatalogPolicy4,
    ) -> anyhow::Result<Self> {
        let learner = load_player(
            path,
            ModelFormat::Net(catalog),
            contract.depth,
            contract.temperature,
            mode,
            contract.learner_identity.as_ref(),
            shared.learner.as_ref(),
        )?;
        let opponent = contract
            .opponent_path
            .as_deref()
            .map(|path| {
                load_player(
                    path,
                    ModelFormat::Net(catalog),
                    contract.depth,
                    contract.temperature,
                    mode,
                    contract.opponent_identity.as_ref(),
                    shared.opponent.as_ref(),
                )
            })
            .transpose()?;
        Ok(Self {
            learner,
            opponent,
            focal: 0,
            last_was_learner: true,
            coverage: CoverageMatrix4::default(),
        })
    }

    fn player(&self, seat: usize) -> Option<&dyn EvaluationPlayer> {
        if seat == self.focal {
            Some(self.learner.as_ref())
        } else {
            self.opponent.as_deref()
        }
    }

    fn player_mut(&mut self, seat: usize) -> Option<&mut (dyn EvaluationPlayer + 'static)> {
        if seat == self.focal {
            Some(self.learner.as_mut())
        } else {
            self.opponent.as_deref_mut()
        }
    }
}

impl EvaluationPlayer for SeatPlayers {
    fn configure_seats(&mut self, focal: usize, teams: &[Option<u8>]) -> anyhow::Result<()> {
        ensure!(
            teams.len() == 2 && teams.iter().all(Option::is_none) && focal < 2,
            "seat collector is 1v1 only"
        );
        self.focal = focal;
        Ok(())
    }
    fn controls_seat(&self, seat: usize, _focal: usize, _teams: &[Option<u8>]) -> bool {
        seat == self.focal || self.opponent.is_some()
    }
    fn reseed(&mut self, seed: u64) {
        self.learner
            .reseed(seed ^ ((self.focal as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93)));
        if let Some(opponent) = &mut self.opponent {
            opponent
                .reseed(seed ^ (((1 - self.focal) as u64 + 1).wrapping_mul(0xd6e8_feb8_6659_fd93)));
        }
    }
    fn init_game(&mut self, preset: &GamePreset) {
        self.learner.init_game(preset);
        if let Some(opponent) = &mut self.opponent {
            opponent.init_game(preset);
        }
    }
    fn uses_context(&self) -> bool {
        self.learner.uses_context()
    }
    fn uses_payment_context(&self) -> bool {
        self.learner.uses_payment_context()
    }
    fn uses_announcement_context(&self) -> bool {
        self.learner.uses_announcement_context()
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        self.player_mut(usize::from(view.seat.get()))
            .context("observing uncontrolled seat")?
            .observe(view)
    }
    fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<Answer4>> {
        self.last_was_learner = usize::from(view.seat.get()) == self.focal;
        let answer = self
            .player_mut(usize::from(view.seat.get()))
            .context("answering uncontrolled seat")?
            .answer(view, pending, teams)?;
        if let Some(answer) = &answer {
            self.coverage
                .record(PendingKind4::of(pending), answer.provenance.source);
        }
        Ok(answer)
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
    fn rule_refs_left_out(&self) -> u64 {
        self.learner.rule_refs_left_out()
            + self
                .opponent
                .as_ref()
                .map_or(0, |opponent| opponent.rule_refs_left_out())
    }
    fn fallback_diagnostic(&self) -> Option<&FallbackDiagnostic4> {
        if self.last_was_learner {
            self.learner.fallback_diagnostic()
        } else {
            self.opponent
                .as_deref()
                .and_then(EvaluationPlayer::fallback_diagnostic)
        }
    }
    fn policy_encoding(
        &self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Option<(
        baylee_train::features4::EncodedState4,
        baylee_train::features4::EncodedCandidates4,
    )> {
        self.player(usize::from(view.seat.get()))?
            .policy_encoding(view, pending, teams)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::choice::PlayerAction;
    use baylee_train::features4::{SourceKind4, StepProvenance4, TeacherTarget4};
    use std::{cell::RefCell, rc::Rc};
    type Calls = Rc<RefCell<Vec<(u8, String)>>>;
    struct Fake {
        id: u8,
        calls: Calls,
        coverage: CoverageMatrix4,
    }
    impl EvaluationPlayer for Fake {
        fn reseed(&mut self, seed: u64) {
            self.calls
                .borrow_mut()
                .push((self.id, format!("seed:{seed}")));
        }
        fn init_game(&mut self, _: &GamePreset) {
            self.calls.borrow_mut().push((self.id, "reset".into()));
        }
        fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
            self.calls
                .borrow_mut()
                .push((self.id, format!("observe:{}", view.seat.get())));
            Ok(())
        }
        fn answer(
            &mut self,
            view: &PlayerView,
            _: &Pending,
            _: &[Option<u8>],
        ) -> anyhow::Result<Option<Answer4>> {
            self.calls
                .borrow_mut()
                .push((self.id, format!("answer:{}", view.seat.get())));
            Ok(Some(Answer4 {
                action: PlayerAction::PassPriority,
                value: 0.5,
                provenance: StepProvenance4 {
                    source: SourceKind4::Net,
                    ..StepProvenance4::default()
                },
                target: TeacherTarget4::default(),
            }))
        }
        fn coverage(&self) -> &CoverageMatrix4 {
            &self.coverage
        }
    }
    #[test]
    fn physical_seats_route_to_distinct_policies_with_independent_seeds_and_resets() {
        let calls = Calls::default();
        let fake = |id| {
            Box::new(Fake {
                id,
                calls: calls.clone(),
                coverage: CoverageMatrix4::default(),
            }) as Box<dyn EvaluationPlayer>
        };
        let mut players = SeatPlayers {
            learner: fake(0),
            opponent: Some(fake(1)),
            focal: 0,
            last_was_learner: true,
            coverage: CoverageMatrix4::default(),
        };
        let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
        let preset = baylee_train::selfplay::table_for(
            31,
            &[&deck, &deck],
            &[baylee_core::preset::AIProfile::STEADY; 2],
            &[],
        );
        let session = baylee_gamehost::Session::new(&preset).unwrap();
        for focal in [0, 1] {
            calls.borrow_mut().clear();
            players.configure_seats(focal, &[None, None]).unwrap();
            players.reseed(91);
            players.init_game(&preset);
            for seat in [0, 1] {
                let view = session.observe_view_with_context(baylee_core::ids::PlayerId::new(seat));
                players.observe(&view).unwrap();
                players
                    .answer(&view, session.pending(), &[None, None])
                    .unwrap();
            }
            let log = calls.borrow();
            assert_eq!(log.iter().filter(|(_, event)| event == "reset").count(), 2);
            let seeds: Vec<_> = log
                .iter()
                .filter(|(_, event)| event.starts_with("seed:"))
                .collect();
            assert_ne!(seeds[0].1, seeds[1].1);
            assert!(log.contains(&(0, format!("observe:{focal}"))));
            assert!(log.contains(&(1, format!("observe:{}", 1 - focal))));
            assert!(log.contains(&(0, format!("answer:{focal}"))));
            assert!(log.contains(&(1, format!("answer:{}", 1 - focal))));
        }
        assert!(players.configure_seats(0, &[Some(1), Some(1)]).is_err());
    }
}
