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
    /// Accept a model exported with an older card catalog when this build's
    /// is a superset: every rule the model knows is unchanged, and the rules
    /// of cards and tokens added since are left out as an unknown card's
    /// are (`docs/trained-ai.md` §"Card catalog compatibility"). Recorded in
    /// run.json and the summary.
    #[arg(long, conflicts_with = "legacy_v3")]
    catalog_superset: bool,
    /// Indices stable, structures may drift: `--catalog-superset`, and also
    /// accept cards and tokens whose rules changed since the export,
    /// withdrawing each card's rule trees (left out as an added card's are;
    /// its DSL summary stays current). run.json and the summary carry the
    /// drift report: appended cards, changed entries, the withdrawn cards.
    #[arg(long, conflicts_with = "legacy_v3")]
    catalog_drift: bool,
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
    /// Run the net's steps of every worker in shared batched calls of at most
    /// this many rows, on one copy of each graph (`0` = every worker owns its
    /// sessions and runs one step per call). Batched runs are not bitwise
    /// reproducible.
    #[arg(long, default_value_t = 0)]
    inference_batch: usize,
    /// First deal seed (shuffle/opening hand RNG).
    #[arg(long, default_value_t = 10_000)]
    first_seed: u64,
    /// Independent policy sampling seed (never mixed with deal `seed`, `P0-7`).
    #[arg(long, default_value_t = 0x517c_c1b7_2722_0a95)]
    policy_seed: u64,
    /// Max decisions per two seats.
    #[arg(long, default_value_t = 5000)]
    max_decisions: u64,
    /// Belief-consistent lookahead at the net's priority decisions in main
    /// phases and combat, as `K,P,H`: the policy's answer and its next K-1,
    /// each tried in P worlds dealt again from the policy seed, rolled out
    /// until the seat holds priority in a settled main phase, its next turn
    /// ends, or H of its own decisions are spent (mana abilities and forced
    /// answers do not count; `0`: score the answer's own position). Off by
    /// default; evaluation against the house only. Budgets are counts.
    #[arg(long)]
    search: Option<SearchConfig4>,
    /// How a search chooses: `prior` (the default) adds
    /// `--search-prior-c · prior / (1 + worlds)` to each mean, so the
    /// policy's answer stays unless the worlds outweigh the policy's
    /// confidence; `paired` keeps the policy's answer unless another's mean
    /// improvement over it, world by world, exceeds
    /// `max(--search-margin, --search-z · se)`; `mean` takes the best mean
    /// (v1). Why `prior` is the default: `docs/trained-ai.md` §"Lookahead
    /// search".
    #[arg(long, value_enum, default_value_t = SearchSelect::Prior)]
    search_select: SearchSelect,
    /// `paired`: the smallest mean improvement that overrides the policy.
    #[arg(long, default_value_t = 0.05)]
    search_margin: f64,
    /// `paired`: standard errors the mean improvement must also exceed.
    #[arg(long, default_value_t = 1.0)]
    search_z: f64,
    /// `prior`: the weight of the policy's prior.
    #[arg(long, default_value_t = 1.0)]
    search_prior_c: f64,
    /// Searched decisions per game at most (`0` = no limit); later ones are
    /// answered without searching.
    #[arg(long, default_value_t = 0)]
    search_max_decisions_per_game: u64,
    /// Rollouts one searched decision runs at once, each on its own thread
    /// and a twin of the net whose steps join the `--inference-batch`
    /// batches; without `--inference-batch` they run one by one. A batch
    /// then holds up to `--threads × (this + 1)` rows. Batched searches are
    /// not bitwise reproducible.
    #[arg(long, default_value_t = SearchConfig4::DEFAULT_THREADS)]
    search_threads: usize,
}

/// `--search-select`: which [`Selection4`] a search uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum SearchSelect {
    Mean,
    Paired,
    Prior,
}

/// The run's search, its budgets and selection together; `None` without
/// `--search`.
fn search_config(args: &Args) -> anyhow::Result<Option<SearchConfig4>> {
    let Some(config) = args.search else {
        return Ok(None);
    };
    // Every selection parameter is checked, the chosen rule's or not: a
    // value given wrong is refused, not ignored.
    let paired = Selection4::Paired {
        margin: args.search_margin,
        z: args.search_z,
    }
    .validate()?;
    let prior = Selection4::Prior {
        c: args.search_prior_c,
    }
    .validate()?;
    let selection = match args.search_select {
        SearchSelect::Mean => Selection4::Mean,
        SearchSelect::Paired => paired,
        SearchSelect::Prior => prior,
    };
    let config = config
        .with_selection(selection)?
        .with_threads(args.search_threads)?;
    // Only a net on shared batches lends rollout players; without them the
    // rollouts run one by one, and the run's metadata says so.
    if args.inference_batch > 0 {
        Ok(Some(config))
    } else {
        config.with_threads(1).map(Some)
    }
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
    search: lookahead::SearchTally,
    rule_refs_left_out: u64,
}

/// Why `--search` cannot run with `args`, if it cannot.
fn search_refusal(
    args: &Args,
    collection: &collector::Contract,
    identity: Option<&ModelIdentity>,
) -> Option<&'static str> {
    args.search?;
    if args.legacy_v3 {
        return Some("--search needs a v4/v5 model");
    }
    if collection.uses_seat_pool() {
        return Some("--search is evaluation against the house only");
    }
    if args.shadow_teacher.is_some() {
        return Some("--search writes no shadow labels");
    }
    if matches!(
        identity.and_then(|identity| identity.critic_semantics.as_ref()),
        Some(CriticSemantics4::FailureTerminalResearchUtilityV1 { .. })
    ) {
        return Some("--search scores leaves with an MTG outcome critic, not a research utility");
    }
    None
}

/// What `run.json` and the summary say about the search, when there is one.
fn search_metadata(
    args: &Args,
    config: Option<SearchConfig4>,
    identity: Option<&ModelIdentity>,
) -> Option<Value> {
    let mut metadata = config?.metadata();
    metadata["max_decisions_per_game"] = json!(args.search_max_decisions_per_game);
    metadata["critic_semantics"] = json!(identity.and_then(|i| i.critic_semantics.as_ref()));
    Some(metadata)
}

/// How every model of the run binds its card catalog to this build's.
const fn catalog_policy(args: &Args) -> CatalogPolicy4 {
    match (args.catalog_superset, args.catalog_drift) {
        (_, true) => CatalogPolicy4::Drift,
        (true, false) => CatalogPolicy4::Superset,
        (false, false) => CatalogPolicy4::Exact,
    }
}

/// What `run.json` and the summary say about the card catalog when a run
/// accepts another than this build's: each model's binding.
fn catalog_metadata(
    args: &Args,
    collection: &collector::Contract,
) -> anyhow::Result<Option<Value>> {
    let policy = catalog_policy(args);
    if policy == CatalogPolicy4::Exact {
        return Ok(None);
    }
    let opponent = collection
        .opponent_path
        .as_deref()
        .map(|path| inspect_catalog(path, policy))
        .transpose()?
        .flatten();
    Ok(Some(json!({
        "policy": policy,
        "learner": inspect_catalog(&args.model, policy)?,
        "opponent": opponent,
    })))
}

/// How a run's models are opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFormat {
    /// An archived v3 net (`--legacy-v3`).
    LegacyV3,
    /// A v4/v5 net, its card catalog bound under the policy.
    Net(CatalogPolicy4),
}

fn parse_profile(name: &str) -> anyhow::Result<AIProfile> {
    AIProfile::named(name).with_context(|| format!("unknown AIProfile {name}"))
}
