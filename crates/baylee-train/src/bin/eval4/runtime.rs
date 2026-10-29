// Keep game setup, seeds, opponents and fallback handling identical when
// comparing an archived v3 player with the current v4 implementation.
trait EvaluationPlayer {
    fn configure_seats(&mut self, _focal: usize, _teams: &[Option<u8>]) -> anyhow::Result<()> {
        Ok(())
    }
    fn controls_seat(&self, seat: usize, focal: usize, teams: &[Option<u8>]) -> bool {
        is_net_seat(seat, focal, teams)
    }
    fn reseed(&mut self, seed: u64);
    fn init_game(&mut self, preset: &baylee_core::preset::GamePreset);
    fn uses_context(&self) -> bool {
        false
    }
    fn uses_payment_context(&self) -> bool {
        false
    }
    fn uses_announcement_context(&self) -> bool {
        false
    }
    fn observe(&mut self, view: &baylee_view::PlayerView) -> anyhow::Result<()>;
    fn answer(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<baylee_train::netplay4::Answer4>>;
    fn coverage(&self) -> &CoverageMatrix4;
    fn fallback_diagnostic(&self) -> Option<&baylee_train::netplay4::FallbackDiagnostic4> {
        None
    }
    fn policy_encoding(
        &self,
        _view: &baylee_view::PlayerView,
        _pending: &Pending,
        _teams: &[Option<u8>],
    ) -> Option<(
        baylee_train::features4::EncodedState4,
        baylee_train::features4::EncodedCandidates4,
    )> {
        None
    }
}

impl EvaluationPlayer for NetPlayer4 {
    fn fallback_diagnostic(&self) -> Option<&baylee_train::netplay4::FallbackDiagnostic4> {
        self.last_fallback()
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
        Some(Self::policy_encoding(self, view, pending, teams))
    }
    fn reseed(&mut self, seed: u64) {
        Self::reseed(self, seed);
    }
    fn init_game(&mut self, preset: &baylee_core::preset::GamePreset) {
        Self::init_game(self, preset);
    }
    fn uses_context(&self) -> bool {
        self.is_encoder7()
    }
    fn uses_payment_context(&self) -> bool {
        Self::uses_payment_context(self)
    }
    fn uses_announcement_context(&self) -> bool {
        Self::uses_announcement_context(self)
    }
    fn observe(&mut self, view: &baylee_view::PlayerView) -> anyhow::Result<()> {
        Self::observe(self, view)
    }
    fn answer(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<baylee_train::netplay4::Answer4>> {
        Ok(Some(self.act_detailed(view, pending, teams)))
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
}

struct LegacyPlayer {
    net: baylee_train::netplay3::NetPlayer3,
    own_decks: Vec<Vec<(baylee_core::ids::CardIndex, u32)>>,
    coverage: CoverageMatrix4,
}

impl EvaluationPlayer for LegacyPlayer {
    fn reseed(&mut self, seed: u64) {
        self.net.seed(seed);
    }
    fn init_game(&mut self, preset: &baylee_core::preset::GamePreset) {
        self.own_decks = preset
            .seats
            .iter()
            .map(baylee_train::features3::deck_list)
            .collect();
    }
    fn observe(&mut self, _view: &baylee_view::PlayerView) -> anyhow::Result<()> {
        Ok(())
    }
    fn answer(
        &mut self,
        view: &baylee_view::PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> anyhow::Result<Option<baylee_train::netplay4::Answer4>> {
        let table = baylee_train::features3::Table {
            teams,
            deck: &self.own_decks[usize::from(view.seat.get())],
        };
        let forced_before = self.net.forced;
        let answer = self.net.answer(view, pending, &table)?;
        let source = if answer.is_none() {
            SourceKind4::House
        } else if self.net.forced > forced_before {
            SourceKind4::Forced
        } else {
            SourceKind4::Net
        };
        self.coverage
            .record(baylee_train::features4::PendingKind4::of(pending), source);
        Ok(answer.map(|a| baylee_train::netplay4::Answer4 {
            action: a.action,
            value: a.value,
            provenance: StepProvenance4 {
                source,
                has_logprob: false,
                ..StepProvenance4::default()
            },
            target: baylee_train::features4::TeacherTarget4::default(),
        }))
    }
    fn coverage(&self) -> &CoverageMatrix4 {
        &self.coverage
    }
}

fn load_player(
    path: &std::path::Path,
    legacy: bool,
    depth: u8,
    temperature: f32,
    policy_mode: PolicyMode4,
    expected_identity: Option<&ModelIdentity>,
) -> anyhow::Result<Box<dyn EvaluationPlayer>> {
    if !legacy {
        let player = NetPlayer4::load(path)?
            .with_thinking_steps(depth)
            .with_temperature(temperature)
            .with_policy_mode(policy_mode);
        anyhow::ensure!(
            expected_identity == Some(player.model_identity()),
            "worker model identity differs from evaluation/run metadata"
        );
        return Ok(Box::new(player));
    }
    let model = if path.is_dir() {
        path.join("net.onnx")
    } else {
        path.to_path_buf()
    };
    let meta: Value = serde_json::from_slice(&fs::read(model.with_extension("onnx.json"))?)?;
    anyhow::ensure!(
        meta["encoder_version"] == 3,
        "--legacy-v3 requires encoder 3"
    );
    let entities = usize::try_from(
        meta["entities"]
            .as_u64()
            .context("missing legacy entity cap")?,
    )?;
    Ok(Box::new(LegacyPlayer {
        // Archived v3 was trained with a house-profile input; use its expert profile.
        net: baylee_train::netplay3::NetPlayer3::load(&model, entities, 4)?,
        own_decks: Vec::new(),
        coverage: CoverageMatrix4::default(),
    }))
}
