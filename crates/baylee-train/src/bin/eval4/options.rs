#[derive(Parser, Debug)]
#[command(about = "Multi-format duplicate evaluator and RL rollout generator for Playernet v4")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Independent command-line switches"
)]
struct Args {
    /// Path to Playernet v4 `model.onnx` or checkpoint directory.
    #[arg(long)]
    model: PathBuf,
    /// Explicitly separate new on-policy collection from ordinary evaluation records.
    #[arg(long, value_enum, default_value_t = collector::Purpose::Evaluation)]
    collection_purpose: collector::Purpose,
    /// Frozen neural opponent; omitted means the ordinary house opponent.
    #[arg(long)]
    opponent_model: Option<PathBuf>,
    /// Historical policy or exact frozen-learner self-play opponent.
    #[arg(long, value_enum, default_value_t = collector::OpponentRole::Historical)]
    opponent_role: collector::OpponentRole,
    /// Evaluate an archived v3 net (expert profile) under the same table protocol.
    /// Greedy evaluation only; never write v4 PPO rollout labels for v3 actions.
    #[arg(long)]
    legacy_v3: bool,
    /// Number of deals to play (each deal plays Net + optional House baseline).
    #[arg(long, default_value_t = 40)]
    deals: u64,
    /// First deal index, preserving table/deck/seat rotation for a reproducer.
    #[arg(long, default_value_t = 0)]
    deal_offset: u64,
    /// Seats at the table (`2..=8`).
    #[arg(long, default_value_t = 2)]
    seats: usize,
    /// Team assignment per seat (e.g. `1,2,1,2` for 2v2); empty = FFA.
    #[arg(long, value_delimiter = ',')]
    teams: Vec<u8>,
    /// Opponent house profile (`novice`, `casual`, `steady`, `sharp`, `expert`).
    #[arg(long, default_value = "steady")]
    against: String,
    /// Baseline house profile for duplicate comparison (`steady`, `sharp`, `expert`).
    #[arg(long, default_value = "steady")]
    baseline: String,
    /// Latent thinking steps `K` (`1..=8`).
    #[arg(long, default_value_t = 2)]
    thinking_steps: u8,
    /// Sampling temperature `tau` (`0.0` = greedy argmax).
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    /// Use unmodified neural outputs; stochastic version-2 sampling requires tau > 0.35.
    #[arg(long)]
    raw_policy: bool,
    /// Write separate BC-only view-only teacher labels (sharp or expert); requires raw recording.
    #[arg(long, value_parser = ["sharp", "expert"])]
    shadow_teacher: Option<String>,
    /// Number of generated decks to build (`0` = use `--decks-manifest` only).
    #[arg(long, default_value_t = 32)]
    generated: usize,
    /// Shape for generated/manifest decks (`constructed`, `highlander`, `commander`).
    #[arg(long, default_value = "constructed")]
    shape: String,
    /// Optional path to `data/decks/manifest.v4.json` to load real-world decks.
    #[arg(long)]
    decks_manifest: Option<PathBuf>,
    /// Split to filter when `--decks-manifest` is given (`train`, `val`, `test`).
    #[arg(long, default_value = "test")]
    deck_split: String,
    /// Optional `holdout_tier` filter when `--decks-manifest` is given
    /// (`eval_benchmark`, `unseen_deck_known_cards`, `unseen_cards`).
    #[arg(long, default_value = "")]
    holdout_tier: String,
    /// Build/filter decks including the held-out cards excluded from training.
    #[arg(long)]
    held_out_cards: bool,
    /// Clamp AI scouting so House opponents have no omniscient hand/deck peek.
    #[arg(long, default_value_t = true)]
    fair_scouting: bool,
    /// Skip running the House baseline duplicate game (useful for pure RL rollouts).
    #[arg(long)]
    no_baseline: bool,
    /// Optional output directory for recorded games (`run.json`, `games.jsonl`, `records/`).
    #[arg(long)]
    record_out: Option<PathBuf>,
    /// Output JSON summary file path.
    #[arg(long)]
    out: PathBuf,
    /// Worker threads (`0` = all cores).
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// First deal seed (shuffle/opening hand RNG).
    #[arg(long, default_value_t = 10_000)]
    first_seed: u64,
    /// Independent policy sampling seed (never mixed with deal `seed`, `P0-7`).
    #[arg(long, default_value_t = 0x517c_c1b7_2722_0a95)]
    policy_seed: u64,
    /// Max decisions per two seats.
    #[arg(long, default_value_t = 5000)]
    max_decisions: u64,
}

#[derive(Default)]
struct EvalStats {
    deals_completed: u64,
    net_utility_sum: f64,
    base_utility_sum: f64,
    net_wins: u64,
    base_wins: u64,
    net_draws: u64,
    base_draws: u64,
    net_answer_caps: u64,
    base_answer_caps: u64,
    net_answers: u64,
    house_fallbacks: u64,
    refused: u64,
    stalls: u64,
    latencies_ms: Vec<f64>,
    coverage: CoverageMatrix4,
    deal_details: Vec<Value>,
}

fn parse_profile(name: &str) -> anyhow::Result<AIProfile> {
    AIProfile::named(name).with_context(|| format!("unknown AIProfile {name}"))
}
