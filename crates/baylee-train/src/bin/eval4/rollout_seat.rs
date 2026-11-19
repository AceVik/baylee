//! The net's seat when another net plays its search's rollouts
//! (`--rollout-net`): the net plays the table as without it, and the rollout
//! net watches every view beside it, so that at a searched root its memory
//! is the game's (`netplay4::search::RolloutNet4`). With `--rollout-compare`
//! the net's own search answers, and at the same root the same worlds are
//! searched again with the rollout net under each leaf named, for the
//! search event (`variants`).

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_train::features4::CoverageMatrix4;
use baylee_train::netplay4::NetPlayer4;
use baylee_train::netplay4::search::{
    PriorDecks, RolloutLeaf4, RolloutNet4, ScriptFollow4, SearchConfig4, SearchOutcome4,
    SearchPlayer4, SearchedAnswer4, search_decision_avoiding,
};
use serde_json::{Value, json};

use super::EvaluationPlayer;

/// The searching net and the net that plays its rollouts.
pub(super) struct RolloutSeat {
    net: NetPlayer4,
    rollout: NetPlayer4,
    leaf: RolloutLeaf4,
    /// `--rollout-compare`: the leaves whose searches are recorded beside
    /// the net's own, which then answers.
    compare: Vec<RolloutLeaf4>,
    /// The compared searches of the latest searched decision.
    variants: Option<Value>,
}

impl RolloutSeat {
    pub(super) fn new(
        net: NetPlayer4,
        rollout: NetPlayer4,
        leaf: RolloutLeaf4,
        compare: Vec<RolloutLeaf4>,
    ) -> Self {
        Self {
            net,
            rollout,
            leaf,
            compare,
            variants: None,
        }
    }

    /// The searching net and its rollout net at this root, as one player.
    fn searcher(&mut self, leaf: RolloutLeaf4) -> anyhow::Result<RolloutNet4<'_>> {
        RolloutNet4::new(&mut self.net, &mut self.rollout, leaf)
    }

    /// Each compared leaf's search of this root, from the same state and
    /// worlds; both nets are back at the root afterwards.
    #[allow(
        clippy::too_many_arguments,
        reason = "search_decision_avoiding's inputs, handed on whole"
    )]
    fn compared(
        &mut self,
        table: &Session,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        config: SearchConfig4,
        seed: u64,
        taken_back: &[PlayerAction],
    ) -> anyhow::Result<Value> {
        let leaves = self.compare.clone();
        let root = self.searcher(self.leaf)?.snapshot();
        let mut variants = serde_json::Map::new();
        for leaf in leaves {
            let mut searcher = self.searcher(leaf)?;
            let searched = search_decision_avoiding(
                &mut searcher,
                &PriorDecks::new(table, view),
                view,
                pending,
                teams,
                config,
                seed,
                taken_back,
            )?;
            searcher.restore(&root);
            let variant = match searched.outcome {
                SearchOutcome4::Searched(report) => json!({
                    "candidates": report.actions,
                    "chosen": report.chosen,
                    "means": report.means,
                    "worlds": report.worlds,
                    "rounds": report.rounds,
                    "rollouts": report.rollouts,
                    "rollout_steps": report.rollout_steps,
                    "leaves": report.leaves,
                }),
                other => json!({"unsearched": format!("{other:?}")}),
            };
            variants.insert(leaf.name().to_owned(), variant);
        }
        Ok(Value::Object(variants))
    }
}

impl EvaluationPlayer for RolloutSeat {
    fn configure_seats(&mut self, focal: usize, teams: &[Option<u8>]) -> anyhow::Result<()> {
        EvaluationPlayer::configure_seats(&mut self.net, focal, teams)
    }
    fn controls_seat(&self, seat: usize, focal: usize, teams: &[Option<u8>]) -> bool {
        EvaluationPlayer::controls_seat(&self.net, seat, focal, teams)
    }
    fn reseed(&mut self, seed: u64) {
        EvaluationPlayer::reseed(&mut self.net, seed);
        EvaluationPlayer::reseed(&mut self.rollout, seed);
    }
    fn init_game(&mut self, preset: &baylee_core::preset::GamePreset) {
        EvaluationPlayer::init_game(&mut self.net, preset);
        EvaluationPlayer::init_game(&mut self.rollout, preset);
    }
    fn uses_context(&self) -> bool {
        EvaluationPlayer::uses_context(&self.net)
    }
    fn uses_payment_context(&self) -> bool {
        EvaluationPlayer::uses_payment_context(&self.net)
    }
    fn uses_announcement_context(&self) -> bool {
        EvaluationPlayer::uses_announcement_context(&self.net)
    }
    fn observe(&mut self, view: &baylee_view::PlayerView) -> anyhow::Result<()> {
        EvaluationPlayer::observe(&mut self.net, view)?;
        EvaluationPlayer::observe(&mut self.rollout, view)
    }
    fn answer(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<baylee_train::netplay4::Answer4>> {
        EvaluationPlayer::answer(&mut self.net, view, pending, teams)
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        EvaluationPlayer::coverage(&self.net)
    }
    fn fallback_diagnostic(&self) -> Option<&baylee_train::netplay4::FallbackDiagnostic4> {
        EvaluationPlayer::fallback_diagnostic(&self.net)
    }
    fn rule_refs_left_out(&self) -> u64 {
        EvaluationPlayer::rule_refs_left_out(&self.net)
    }
    fn set_grouped_decode(&mut self) -> bool {
        EvaluationPlayer::set_grouped_decode(&mut self.rollout)
            && EvaluationPlayer::set_grouped_decode(&mut self.net)
    }
    fn take_grouped_use(&mut self) -> baylee_train::netplay4::GroupedUse4 {
        EvaluationPlayer::take_grouped_use(&mut self.net)
    }
    fn set_adaptive_thinking(
        &mut self,
        adaptive: baylee_train::netplay4::AdaptiveThinking4,
    ) -> anyhow::Result<bool> {
        Ok(
            EvaluationPlayer::set_adaptive_thinking(&mut self.rollout, adaptive)?
                && EvaluationPlayer::set_adaptive_thinking(&mut self.net, adaptive)?,
        )
    }
    fn take_thinking_use(&mut self) -> baylee_train::netplay4::ThinkingUse4 {
        EvaluationPlayer::take_thinking_use(&mut self.net)
    }
    fn set_choice_guard(&mut self, guard: &baylee_train::netplay4::ChoiceGuard4) -> bool {
        // The rollouts play the seat as the table does: the bundle too.
        EvaluationPlayer::set_choice_guard(&mut self.rollout, guard)
            && EvaluationPlayer::set_choice_guard(&mut self.net, guard)
    }
    fn take_choice_guard_fired(&mut self) -> Vec<u64> {
        EvaluationPlayer::take_choice_guard_fired(&mut self.net)
    }
    fn set_tactic_rules(&mut self, rules: baylee_train::netplay4::TacticRules4) -> bool {
        EvaluationPlayer::set_tactic_rules(&mut self.rollout, rules)
            && EvaluationPlayer::set_tactic_rules(&mut self.net, rules)
    }
    fn take_tactic_rules_fired(&mut self) -> Vec<(&'static str, u64)> {
        EvaluationPlayer::take_tactic_rules_fired(&mut self.net)
    }
    fn set_payable_menu(&mut self, bias: f32) -> bool {
        EvaluationPlayer::set_payable_menu(&mut self.net, bias)
    }
    fn set_tactician(&mut self, tactician: baylee_train::netplay4::tactician::Tactician4) -> bool {
        EvaluationPlayer::set_tactician(&mut self.net, tactician)
    }
    fn take_tactician_records(
        &mut self,
    ) -> Vec<baylee_train::netplay4::tactician::TacticianRecord4> {
        EvaluationPlayer::take_tactician_records(&mut self.net)
    }
    fn take_menu_stats(&mut self) -> Option<baylee_train::netplay4::MenuStats4> {
        EvaluationPlayer::take_menu_stats(&mut self.net)
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        EvaluationPlayer::observed_value(&self.net, seat)
    }
    fn search(
        &mut self,
        table: &Session,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        config: SearchConfig4,
        seed: u64,
        taken_back: &[PlayerAction],
    ) -> Option<anyhow::Result<SearchedAnswer4>> {
        if !config.fair() {
            return Some(Err(anyhow::anyhow!(
                "--rollout-net searches fair worlds only (--search-fair)"
            )));
        }
        if self.compare.is_empty() {
            let searched = self.searcher(self.leaf).and_then(|mut searcher| {
                search_decision_avoiding(
                    &mut searcher,
                    &PriorDecks::new(table, view),
                    view,
                    pending,
                    teams,
                    config,
                    seed,
                    taken_back,
                )
            });
            return Some(searched);
        }
        // The compared searches first, each from the root; then the net's
        // own, which answers.
        let variants = self.compared(table, view, pending, teams, config, seed, taken_back);
        let variants = match variants {
            Ok(variants) => variants,
            Err(error) => return Some(Err(error)),
        };
        let searched = EvaluationPlayer::search(
            &mut self.net,
            table,
            view,
            pending,
            teams,
            config,
            seed,
            taken_back,
        );
        self.variants = Some(variants);
        searched
    }
    fn take_search_variants(&mut self) -> Option<Value> {
        self.variants.take()
    }
    fn follow(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        target: &PlayerAction,
    ) -> Option<(baylee_train::netplay4::Answer4, bool)> {
        EvaluationPlayer::follow(&mut self.net, view, pending, teams, target)
    }
    fn follow_script(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
        script: &mut baylee_train::netplay4::Script4,
    ) -> Option<ScriptFollow4> {
        EvaluationPlayer::follow_script(&mut self.net, view, pending, teams, script)
    }
    fn policy_encoding(
        &self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> Option<(
        baylee_train::features4::EncodedState4,
        baylee_train::features4::EncodedCandidates4,
    )> {
        EvaluationPlayer::policy_encoding(&self.net, view, pending, teams)
    }
}

/// The seat of `--rollout-net`: the net on `learner`'s batches, configured as
/// `load_player` configures it, and the rollout net on `rollout`'s, with the
/// same depth, temperature and policy mode.
#[allow(
    clippy::too_many_arguments,
    reason = "load_player's settings, both nets and the leaves"
)]
pub(super) fn load(
    learner: &std::sync::Arc<baylee_train::netplay4::BatchedNet4>,
    rollout: &std::sync::Arc<baylee_train::netplay4::BatchedNet4>,
    depth: u8,
    temperature: f32,
    policy_mode: baylee_train::netplay4::PolicyMode4,
    expected_identity: Option<&baylee_train::netplay4::ModelIdentity>,
    leaf: RolloutLeaf4,
    compare: Vec<RolloutLeaf4>,
) -> anyhow::Result<RolloutSeat> {
    let configure = |net: NetPlayer4| {
        let mut net = net
            .with_thinking_steps(depth)
            .with_temperature(temperature)
            .with_policy_mode(policy_mode);
        if depth == 0 {
            net.set_no_thinking();
        }
        net
    };
    let net = configure(NetPlayer4::shared(learner));
    anyhow::ensure!(
        expected_identity == Some(net.model_identity()),
        "worker model identity differs from evaluation/run metadata"
    );
    let rollout = configure(NetPlayer4::shared(rollout));
    anyhow::ensure!(
        rollout.is_encoder7() == net.is_encoder7()
            && rollout.uses_payment_context() == net.uses_payment_context()
            && rollout.uses_announcement_context() == net.uses_announcement_context(),
        "--rollout-net reads other views than the model"
    );
    Ok(RolloutSeat::new(net, rollout, leaf, compare))
}
