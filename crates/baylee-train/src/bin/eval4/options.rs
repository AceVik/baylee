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
    /// Which export of the models runs. `auto` is `MODEL/fp16` for an evaluation
    /// on the GPU, when it exists; a collection always keeps FP32.
    #[arg(long, value_enum, default_value_t = precision::Precision::Auto)]
    precision: precision::Precision,
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
    /// A measurement of what the model reads: leaves out every identity
    /// reference (`i/<card>/<face>`, the card's index as bytes) of the seat
    /// under test, as an unknown rule's. Its other references, its DSL
    /// summary and its printed features stay. Evaluation only; recorded in
    /// run.json and the summary (`catalog`).
    #[arg(long, conflicts_with = "legacy_v3")]
    leave_out_identity: bool,
    /// Accept an export of the seat under test whose rule table was ablated
    /// on purpose (`export_onnx4.py --blind-card-names`): a measurement, never
    /// a model to play live or to collect with. Evaluation only; the export's
    /// `rule_table_ablation` is recorded in run.json and the summary.
    #[arg(long, conflicts_with = "legacy_v3")]
    ablated_rule_table: bool,
    /// The tactician (`netplay4::tactician`) answers these questions of the
    /// seat under test: `attack`, `block`, `target` (one target), or `all`.
    /// With `--tactician-explore` it explores and keeps records for training a
    /// head on outcomes; with `--tactician` a trained head plays. Needs
    /// `--raw-policy` at temperature 0 (`--choice-guard tactical` keeps the
    /// tactical layer everywhere else), two seats against the house, a graph
    /// exported with `--embeddings`, and `--decks-manifest`: the look's prior is
    /// the manifest's train split. Evaluation only.
    #[arg(long, conflicts_with = "legacy_v3")]
    tactician_windows: Option<String>,
    /// Explore at the windows: the net's answer with probability `1 − ε`.
    #[arg(long, requires = "tactician_windows", conflicts_with = "tactician")]
    tactician_explore: Option<f64>,
    /// A trained head (`tactician.onnx`, `tactician.json`) plays the windows.
    #[arg(long, requires = "tactician_windows")]
    tactician: Option<PathBuf>,
    /// How much better a head must score an answer to leave the net's;
    /// default the head's own `margin`.
    #[arg(long, requires = "tactician")]
    tactician_margin: Option<f32>,
    /// Where the window decisions are kept (`wNN.jsonl`, `wNN.f32`).
    #[arg(long, requires = "tactician_windows")]
    tactician_out: Option<PathBuf>,
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
    /// Dev-only ceilings (`netplay4::tactics::guard`), comma separated: `cast`,
    /// `tap`, `spend`, `mulligan`, `react`, `tactical`. `off` is none. Takes a value so
    /// that `paired_eval4` can read the command. Never for training.
    #[arg(long, default_value = "off")]
    choice_guard: String,
    /// Rule groups over the tactical layer (`netplay4::tactics::rules`), comma
    /// separated, or `all`. Empty is none. Takes a value so that `paired_eval4`
    /// can read the command.
    #[arg(long, default_value = "")]
    tactics_rules: String,
    /// Plays as the layer plays and counts, per deal, what each rule group alone
    /// would have changed: a deal no group touches is the same game with it on.
    #[arg(long)]
    tactics_shadow: bool,
    /// Plays only these deals, a JSON list: the deals a shadow run says a group
    /// reaches. The rest of `--deals` are skipped, and the same deal is the same game.
    #[arg(long)]
    only_deals: Option<std::path::PathBuf>,
    /// Dev only: a raw net may sample at any temperature, below sampler 2's
    /// boundary too, and `--choice-guard` may run above zero. Never for training.
    #[arg(long)]
    dev_free_temperature: bool,
    /// Dev only (`netplay4::menu`): the spells of `legal.payable` the planner
    /// can pay join the priority menu, and the planner taps for the one chosen.
    /// The net never saw such a candidate. Never for training.
    #[arg(long)]
    menu_payable: bool,
    /// `--menu-payable`: added to each added option's logit.
    #[arg(long, default_value_t = 0.0, requires = "menu_payable")]
    menu_payable_bias: f32,
    /// One JSON line per net decision of the seat under test: its value heads
    /// and where the game stands, for `value_report4`. The outcome is in `--out`.
    #[arg(long)]
    value_log: Option<PathBuf>,
    /// Plays `--opponent-model` under this policy (`raw`, `tactical`) instead
    /// of the seat under test's. Only for evaluation.
    #[arg(long, requires = "opponent_model")]
    opponent_policy: Option<String>,
    /// `--opponent-model` is an archived v3 net (`--legacy-v3`'s player), for
    /// numbers only: it lives on its own sessions.
    #[arg(long, requires = "opponent_model")]
    opponent_legacy_v3: bool,
    /// These games are for numbers only: `run.json` says so, and they may never
    /// enter training, a league or a dataset.
    #[arg(long)]
    numbers_only: bool,
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
    /// The seat under test answers uniformly at random among the answers each
    /// question offers (`random_seat.rs`), seeded by the deal: the floor a
    /// trained policy has to clear. `--model` still names the run; it does not
    /// play. Numbers only.
    #[arg(long)]
    random_policy: bool,
    /// Write separate BC-only teacher labels on the neural seat's views; requires
    /// raw recording. `sharp` or `expert` asks the house afresh on each view;
    /// `llm:<profile>` seats a language model of `--llm-profiles` that watches
    /// the seat through its own frames (feature `llm`).
    #[arg(long)]
    shadow_teacher: Option<String>,
    /// The seat settings file an `llm:<profile>` shadow teacher is read from.
    #[arg(long)]
    llm_profiles: Option<std::path::PathBuf>,
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
    /// The house chairs answer as a seat's `--mind house` does over the
    /// gateway (`baylee_seat::HouseMind`, the ladder's `house:<level>`): from
    /// the socket's view, with no decision context and no scouting report,
    /// not as an in-process house chair. Compares the seat path's opponent
    /// with the benchmark's; the summary's `against` reads `seat:<level>`.
    #[arg(long)]
    house_as_seat: bool,
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
    /// Tries a candidate that casts a spell or activates an ability at each
    /// of up to this many of its targets, when the seat's next question
    /// after it is a single target: the policy's own target first, then by
    /// its raw logits (contract `belief_lookahead_v3`). The chosen target
    /// answers that question at the table. `0` (the default) leaves targets
    /// to the policy.
    #[arg(long, default_value_t = 0, requires = "search")]
    search_targets: usize,
    /// In search rollouts, keep non-searching seats under the searching
    /// player's neural policy from their simulated world views instead of
    /// releasing them to the house AI.
    #[arg(long, requires = "search")]
    search_neural_opponent: bool,
    /// Also take a declaration of attackers or blockers as a root: the
    /// policy's own declaration and up to this many minus one others built
    /// from it (none, all in, each creature's part changed alone), each
    /// played in the worlds (contract `belief_lookahead_v4`). Offline only:
    /// its worlds are dealt from the real decks. `0` (the default) leaves
    /// declarations to the policy.
    #[arg(long, default_value_t = 0, requires = "search")]
    search_combat: usize,
    /// With `--search-combat`: combat declarations are the only roots;
    /// priority decisions are answered by the policy (skipped as `roots`).
    #[arg(long, requires = "search_combat")]
    search_combat_only: bool,
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
        .with_threads(args.search_threads)?
        .with_targets(args.search_targets)?
        .with_combat(args.search_combat)?
        .with_combat_only(args.search_combat_only)
        .with_neural_opponent(args.search_neural_opponent);
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
    menu: Option<baylee_train::netplay4::MenuStats4>,
    tactician:
        std::collections::BTreeMap<String, baylee_train::netplay4::tactician::TacticianStats4>,
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
    if collection.purpose != collector::Purpose::Evaluation {
        return Some("--search requires evaluation collection purpose");
    }
    if collection.opponent_legacy_v3 {
        return Some("--search does not support a legacy v3 opponent");
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

/// Why `--house-as-seat` cannot run with `args`, if it cannot: it is a
/// measurement of the seat under test against seated house chairs, and every
/// other house answer of the run (the baseline game, a fallback) still comes
/// from an in-process chair.
fn house_as_seat_refusal(args: &Args, collection: &collector::Contract) -> Option<&'static str> {
    if !args.house_as_seat {
        return None;
    }
    if collection.purpose != collector::Purpose::Evaluation {
        return Some("--house-as-seat requires evaluation collection purpose");
    }
    if !args.no_baseline {
        return Some("--house-as-seat needs --no-baseline: the baseline game seats no net");
    }
    if args.opponent_model.is_some() {
        return Some("--house-as-seat seats the house: no neural opponent");
    }
    if args.record_out.is_some() || args.shadow_teacher.is_some() || args.search.is_some() {
        return Some("--house-as-seat is for numbers only: no records, shadow labels or search");
    }
    None
}

/// The opponent the summary names: a seated house reads `seat:<level>`, so
/// a table never pairs it with the in-process house of the same level.
fn against_label(args: &Args) -> String {
    if args.house_as_seat {
        format!("seat:{}", args.against)
    } else {
        args.against.clone()
    }
}

/// What `run.json` and the summary say about the search, when there is one.
fn search_metadata(
    args: &Args,
    config: Option<SearchConfig4>,
    identity: Option<&ModelIdentity>,
) -> Option<Value> {
    let mut metadata = config?.metadata();
    metadata["max_decisions_per_game"] = json!(args.search_max_decisions_per_game);
    // A row's outputs in a batch differ from its outputs alone, and which
    // rows share a batch depends on timing: a batched search is not
    // reproducible bit for bit (`docs/trained-ai.md` §"Lookahead search").
    metadata["bitwise_reproducible"] = json!(args.inference_batch == 0);
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

/// How the seat under test reads its card catalog: the run's policy and
/// the measurement's ablations. Any other model of the run reads with the
/// policy alone.
const fn catalog_read(args: &Args) -> CatalogRead4 {
    CatalogRead4 {
        policy: catalog_policy(args),
        leave_out_identity: args.leave_out_identity,
        ablated_table: args.ablated_rule_table,
    }
}

/// Why `--catalog-superset`/`--catalog-drift`, `--leave-out-identity` or
/// `--ablated-rule-table` cannot run with `args`, if they cannot: only an
/// evaluation may bind another catalog or read it ablated. `convert4`
/// re-encodes every record with this build's whole catalog, so a recorded
/// probability would belong to another observation than the one converted.
fn catalog_refusal(args: &Args) -> Option<&'static str> {
    let read = catalog_read(args);
    if read != CatalogRead4::default() && args.collection_purpose != collector::Purpose::Evaluation
    {
        return Some(
            "--catalog-superset, --catalog-drift, --leave-out-identity and --ablated-rule-table \
             are for evaluation only: convert4 re-encodes with this build's catalog",
        );
    }
    // A self-play copy shares the seat under test's graphs and rule roots.
    ((read.leave_out_identity || read.ablated_table) && args.opponent_model.is_some()).then_some(
        "--leave-out-identity and --ablated-rule-table measure the seat under test against the \
         house: no neural opponent",
    )
}

/// What `run.json` and the summary say about the card catalog when a run
/// accepts another than this build's, or reads it ablated: each model's
/// binding.
fn catalog_metadata(
    args: &Args,
    collection: &collector::Contract,
) -> anyhow::Result<Option<Value>> {
    let read = catalog_read(args);
    if read == CatalogRead4::default() {
        return Ok(None);
    }
    let opponent = collection
        .opponent_path
        .as_deref()
        .map(|path| inspect_catalog(path, read.policy))
        .transpose()?
        .flatten();
    Ok(Some(json!({
        "policy": read.policy,
        "learner": inspect_catalog(&args.model, read)?,
        "opponent": opponent,
    })))
}

/// How a run's models are opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModelFormat {
    /// An archived v3 net (`--legacy-v3`).
    LegacyV3,
    /// A v4/v5 net, its card catalog read as given.
    Net(CatalogRead4),
}

/// Reads `--choice-guard`.
fn parse_choice_guard(text: &str) -> anyhow::Result<Option<ChoiceGuard4>> {
    if text == "off" {
        return Ok(None);
    }
    let mut guard = ChoiceGuard4::default();
    for arm in text.split(',') {
        match arm {
            "cast" => guard.enable(GuardArm::Cast),
            "tap" => guard.enable(GuardArm::Tap),
            "spend" => guard.enable(GuardArm::Spend),
            "mulligan" => guard.enable(GuardArm::Mulligan),
            "react" => guard.enable(GuardArm::React),
            "probe" => guard.probing = true,
            "tactical" => {
                guard.tactical = baylee_train::netplay4::TacticalParts {
                    priority: true,
                    single: true,
                    subset: true,
                    pair: true,
                    scalar: true,
                };
            }
            "tactical-priority" => guard.tactical.priority = true,
            "tactical-single" => guard.tactical.single = true,
            "tactical-subset" => guard.tactical.subset = true,
            "tactical-pair" => guard.tactical.pair = true,
            "tactical-scalar" => guard.tactical.scalar = true,
            other => bail!("unknown --choice-guard arm {other}"),
        }
    }
    Ok(Some(guard))
}

fn parse_profile(name: &str) -> anyhow::Result<AIProfile> {
    AIProfile::named(name).with_context(|| format!("unknown AIProfile {name}"))
}
