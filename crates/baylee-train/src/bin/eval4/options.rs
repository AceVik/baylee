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
    /// head on outcomes; with `--tactician-imitate` it keeps the tactical
    /// layer's answers as the tactician's labels; with `--tactician` a trained
    /// head plays. Needs `--raw-policy` at temperature 0 (`--choice-guard
    /// tactical` keeps the tactical layer everywhere else), two seats against
    /// the house (an imitating collection also against a self-play copy), a
    /// graph exported with `--embeddings`, and `--decks-manifest`: the look's
    /// prior is the manifest's train split. Evaluation only.
    #[arg(long, conflicts_with = "legacy_v3")]
    tactician_windows: Option<String>,
    /// Explore at the windows: the net's answer with probability `1 − ε`.
    #[arg(long, requires = "tactician_windows", conflicts_with = "tactician")]
    tactician_explore: Option<f64>,
    /// The tactician's bootstrap (stage S1): the seat plays as the tactical
    /// layer with its rule groups (`--choice-guard tactical`, `--tactics-rules`)
    /// and at the windows keeps the layer's answer as the label, leaving it
    /// with probability ε for the net's answer or a neighbor of the layer's.
    /// Needs `--tactician-out`; also against a self-play copy, whose seat is
    /// kept too. Its records (`tactician_imitation_v1`) teach the tactician and
    /// nothing else: convert4 and every other trainer refuse them.
    #[arg(long, requires = "tactician_windows", conflicts_with_all = ["tactician", "tactician_explore"])]
    tactician_imitate: Option<f64>,
    /// A trained head plays the windows: a stage-1 head (`tactician_head_v1`)
    /// chooses among the answers it was offered; a `tactician_head_v2` net
    /// builds the answer from its own logits and plays the net's answer when
    /// unsure. The contract is read from `tactician.json`.
    #[arg(long, requires = "tactician_windows")]
    tactician: Option<PathBuf>,
    /// How much better a head must score an answer to leave the net's;
    /// default the head's own `margin` (`tactician_head_v1`).
    #[arg(long, requires = "tactician")]
    tactician_margin: Option<f32>,
    /// Below this confidence a `tactician_head_v2` net plays the net's own
    /// answer. Default none: the bare tactician plays (stage S1 measures it
    /// so; the net's `confidence_threshold` is reported, not used).
    #[arg(long, requires = "tactician")]
    tactician_confidence: Option<f32>,
    /// `DAgger`: a `tactician_head_v2` net plays its windows, and the tactical
    /// layer's answer under these rule groups (comma list), read on copies at
    /// the same view and logits, is kept as the label in `--tactician-out`
    /// (`tactician_imitation_v1`); nothing of the layer is played at the
    /// windows. Also against a self-play copy, whose windows are kept too.
    #[arg(long, requires_all = ["tactician", "tactician_out"])]
    tactician_relabel: Option<String>,
    /// Whole-net distillation labels (`bundle_distillation_v1`): at every
    /// answer of the policy, the net's raw greedy answer `N` and the tactical
    /// layer's answer `R` under these rule groups (comma list), read on copies
    /// at the same view and logits, with the groups that changed it; kept in
    /// `games.jsonl` (`distill_sidecar`, one entry per `rollout_sidecar` step).
    /// Whoever plays: the layer (`--choice-guard tactical`) or the net itself
    /// (`DAgger`). Needs `--record-out` and temperature 0; also against a
    /// self-play copy. Only `convert4 --bundle-distill` reads such a run.
    #[arg(long, requires = "record_out")]
    distill_labels: Option<String>,
    /// House-teacher labels (`house_distillation_v1`): at every answer of the
    /// policy, the net's raw greedy answer `N` and the answer `H` of a fair
    /// house level (`house:<level>`, as `--baseline` names one; never one
    /// that peeks), asked at the same question before the net answers, with
    /// its foresight in forks dealt from the seat's public prior. Kept as
    /// `--distill-labels` keeps its labels; the net's answer is played. A
    /// collection on train decks only: `--decks-manifest`, `--deck-split
    /// train`, `--generated 0`. Only `convert4 --bundle-distill` reads such a
    /// run.
    #[arg(long, requires = "record_out", conflicts_with = "distill_labels")]
    distill_teacher: Option<String>,
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
    /// Opponent house profile (`novice`, `casual`, `steady`, `sharp`, `expert`),
    /// with knobs for an ablation after a colon (`steady:hold_up=none`,
    /// `profile_spec.rs`).
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
    /// On-policy collection only: rule proposals as recorded exploration
    /// (`netplay4::explore`). At each learner decision the tactical layer with
    /// these groups (a comma list as `--tactics-rules` reads it, `all`, or
    /// `measured` for the bundle `BayleeAI` is measured with) answers on the
    /// same logits; where it differs from the net's sample, its answer is
    /// played with probability `--explore-epsilon`. Each step records π, μ
    /// and the proposer; the records are exploration records, outcomes their
    /// only labels.
    #[arg(long, requires = "explore_epsilon")]
    explore_rules: Option<String>,
    /// With `--explore-rules`: the probability ε, `0 < ε < 1`, of playing
    /// the layer's answer where it differs from the net's.
    #[arg(long, requires = "explore_rules")]
    explore_epsilon: Option<f64>,
    /// Plays only these deals, a JSON list: the deals a shadow run says a group
    /// reaches. The rest of `--deals` are skipped, and the same deal is the same game.
    #[arg(long)]
    only_deals: Option<std::path::PathBuf>,
    /// In a team format, plays one deal per game: the deal whose focal seat is
    /// its team's first seat. The other focal seats of a team replay the same
    /// game (same seed, decks and seats under the net), so a team run otherwise
    /// plays each game once per seat of the net's team. Without teams every
    /// focal seat is a game of its own and nothing is skipped.
    #[arg(long)]
    one_deal_per_game: bool,
    /// How the decks sit at the tables (`baylee_train::deals4`). `fixed` seats
    /// deck `(table · seats + seat) mod n`, so with two seats and an even
    /// number of decks a deck only ever meets one opponent; `round-robin` (two
    /// seats) moves the opponent on every `n` tables, so that over `2n(n−1)`
    /// deals every deck pilots against every other once from each seat.
    #[arg(long, value_enum, default_value_t = DeckSchedule::Fixed)]
    deck_schedule: DeckSchedule,
    /// Writes the deals of `--deal-offset`.. `+ --deals` whose net pilots one
    /// of `--pilot-decks`, as the JSON list `--only-deals` reads, out of the
    /// decks these same flags select and seat, and exits without playing.
    #[arg(long, requires = "pilot_decks")]
    write_deals: Option<PathBuf>,
    /// `--write-deals`: the decks the net pilots, as their keys separated by
    /// commas, or `@FILE` holding a JSON list of keys.
    #[arg(long, requires = "write_deals")]
    pilot_decks: Option<String>,
    /// Also records every baseline game, the house against the house, into
    /// this new directory, laid out as a `selfplay` run is (`games.jsonl`,
    /// `records/`), for `gamelog4 show --seat`: how to read the deals one
    /// house level lost from both sides. Numbers only; never training data.
    #[arg(long)]
    record_baseline: Option<std::path::PathBuf>,
    /// Dev only: a raw net may sample at any temperature, below sampler 2's
    /// boundary too, and `--choice-guard` may run above zero. Never for training.
    #[arg(long)]
    dev_free_temperature: bool,
    /// Dev only (`netplay4::menu`): the spells of `legal.payable` the planner
    /// can pay join the priority menu, and the planner taps for the one chosen.
    /// The net never saw such a candidate. Never for training.
    #[arg(long)]
    menu_payable: bool,
    /// The search student (`netplay4::student`, `search_advantage_head_v1`):
    /// the directory of its `advantage.onnx` and `advantage.json`. At each
    /// macro root of the seat under test it reads the payable menu in a
    /// second pass and plays another answer where its head scores it above
    /// the seat's own by more than the head's margin. The net must export
    /// embeddings and be the checkpoint the head was fit on.
    #[arg(long)]
    search_student: Option<PathBuf>,
    /// `--search-student`: the macro candidates a root compares, the seat's
    /// own included (the search's `K` in the collections it learnt from).
    #[arg(long, default_value_t = baylee_train::netplay4::student::DEFAULT_CANDIDATES, requires = "search_student")]
    search_student_candidates: usize,
    /// At priority, decide between pass and act by the policy's summed
    /// probability per group (coarse classes `coarse_class_v1`), then take
    /// the best answer inside the winning group. Evaluation only; the
    /// summary counts the answers it changed.
    #[arg(long)]
    grouped_decode: bool,
    /// `DEEP,THRESHOLD`: a decision whose first pass (at `--thinking-steps`)
    /// states a confidence below THRESHOLD is thought through again at DEEP
    /// deliberation steps. Needs a model with the deliberation block
    /// (`docs/proposals/baylee-ai-thinking.md`). Evaluation only.
    #[arg(long)]
    think_adaptive: Option<String>,
    /// `--menu-payable`: added to each added option's logit.
    #[arg(long, default_value_t = 0.0, requires = "menu_payable")]
    menu_payable_bias: f32,
    /// One JSON line per net decision of the seat under test: its value heads
    /// and where the game stands, for `value_report4`. The outcome is in `--out`.
    #[arg(long)]
    value_log: Option<PathBuf>,
    /// A measurement of the recurrent memory H (`netplay4::MemoryReset4`):
    /// the seat under test forgets it before every view's observation update
    /// (`decision`: each answer reads only its own view), or before the first
    /// view of each turn (`turn`). Its explicit observation ledger is kept.
    /// Evaluation only; `run.json` and the summary name it.
    #[arg(long, value_enum)]
    memory_reset: Option<MemoryResetArg>,
    /// Writes `RECORD_OUT/decisions.jsonl`: one line per decision of the seat
    /// under test, with its kind, head, answer and logit margin, for
    /// `phase_a4 diverge`.
    #[arg(long, requires = "record_out")]
    decision_trace: bool,
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
    /// Baseline house profile for duplicate comparison (`steady`, `sharp`, `expert`),
    /// with knobs as `--against` takes them.
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
    /// A JSON object of manifest deck id to a whole weight: each selected
    /// deck plays as often as its weight (absent: 1, 0: not at all), the
    /// copies in rounds through the deck list. With `--decks-manifest`, not
    /// with the expanded training decks.
    #[arg(long)]
    deck_weights: Option<PathBuf>,
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
    /// Hold every house chair to its own decklist, the level that peeks
    /// (`seer`) too; every other level is fair at any table.
    #[arg(long, default_value_t = true)]
    fair_scouting: bool,
    /// Let a level that peeks (`seer`) be told what it peeks at, despite
    /// `--fair-scouting`: for measuring `seer` against the fair levels, never
    /// for a benchmark of the net. Every other level stays fair either way.
    #[arg(long)]
    seer_peeks: bool,
    /// The house's chairs try their options in forks of the game before
    /// they answer, where their level has a foresight tier
    /// (`AIProfile::FORESIGHT`). With `--random-policy` only the
    /// house-against-house game searches. The summary's `foresight` holds
    /// what it cost.
    #[arg(long)]
    house_foresight: bool,
    /// `--house-foresight`: bounds one searched answer at this much work on
    /// its meter (`Session::set_foresight_work`: a node of the forks' combat
    /// searches is one unit, a fork and a house answer in a fork are priced
    /// in the same unit), after which the search stops where it stands; `0`
    /// leaves the level's whole budget.
    #[arg(long, default_value_t = 0, requires = "house_foresight")]
    foresight_work: u64,
    /// `--foresight-work`: the most nodes one combat search in a fork may
    /// visit (`Session::set_foresight_nodes`); `0` leaves it the bound's
    /// usual share.
    #[arg(long, default_value_t = 0, requires = "foresight_work")]
    foresight_nodes: u32,
    /// `--house-foresight`: writes the searched questions here, a JSON line
    /// each: the seat's view and question, every answer tried with its
    /// value in each world, and the one given
    /// (`Session::set_foresight_questions`). `foresight_mine` reads the file.
    #[arg(long, requires = "house_foresight")]
    foresight_questions: Option<PathBuf>,
    /// `--foresight-questions`: the kinds of question written, of
    /// `priority`, `attack` and `block`.
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "block",
        requires = "foresight_questions"
    )]
    foresight_question_kinds: Vec<String>,
    /// `--foresight-questions`: also the questions where the level's own
    /// answer stood, not only those the search answered otherwise.
    #[arg(long, requires = "foresight_questions")]
    foresight_questions_all: bool,
    /// Writes, for every house-against-house game, each house chair's
    /// position at the start of every turn (`HeuristicAgent::foresight_features`)
    /// with the game's result, a JSON line each: what `fit_leaf` fits the
    /// search's leaf value on.
    #[arg(long)]
    leaf_log: Option<PathBuf>,
    /// `--house-foresight`: judges the searches by the games played on from
    /// them. Of the questions searched, one in `--foresight-probe-every`
    /// has every world played on past its leaf to the end of the game, and
    /// each such world is written here as a JSON line: the question and the
    /// world, the answer tried, what the seat's view showed at the leaf,
    /// the leaf's value and how the game came out
    /// (`Session::set_foresight_probe`). Slower, and no answer changes.
    /// `foresight_probe` reads the file; `fit_leaf --pairs` fits on it.
    #[arg(long, requires = "house_foresight")]
    foresight_probe: Option<PathBuf>,
    /// `--foresight-probe`: one searched question in this many is probed.
    #[arg(long, default_value_t = 8, requires = "foresight_probe")]
    foresight_probe_every: u64,
    /// Skip running the House baseline duplicate game (useful for pure RL rollouts).
    #[arg(long)]
    no_baseline: bool,
    /// The house chairs answer as a seat's `--mind house` does over the
    /// gateway (`baylee_seat::HouseMind`, the ladder's `house:<level>`): from
    /// the socket's view, the note beside its question and its own deck list
    /// (`baylee_ai::noted::answer_as_seated`), not as an in-process chair. Compares the seat path's opponent
    /// with the benchmark's; the summary's `against` reads `seat:<level>`.
    #[arg(long)]
    house_as_seat: bool,
    /// Optional output directory for recorded games (`run.json`, `games.jsonl`, `records/`).
    #[arg(long)]
    record_out: Option<PathBuf>,
    /// Output JSON summary file path.
    #[arg(long, required_unless_present = "write_deals")]
    out: Option<PathBuf>,
    /// Worker threads (`0` = all cores).
    #[arg(long, default_value_t = 0)]
    threads: usize,
    /// Run the net's steps of every worker in shared batched calls of at most
    /// this many rows, on one copy of each graph (`0` = every worker owns its
    /// sessions and runs one step per call). Batched runs are not bitwise
    /// reproducible.
    #[arg(long, default_value_t = 0)]
    inference_batch: usize,
    /// Sessions of each graph that answer the `--inference-batch` calls at
    /// once, each on its own thread (`1` = one copy of each graph). The CPU's
    /// way to run a search's rollouts on several cores: there a call of many
    /// rows costs about as much per row as one row, so cores, not batches,
    /// make steps cheaper (`docs/trained-ai.md` §"Lookahead search"). With
    /// `--inference-batch 1` every step is the unbatched one, bit for bit.
    #[arg(long, default_value_t = 1)]
    inference_servers: usize,
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
    /// Offline technique experiment. The seat always uses the model's FP32
    /// CPU export. With fair macro search, its separate rollout backend
    /// gets five seconds from the original question.
    #[arg(long)]
    offline_deadline_seat: bool,
    /// How a search chooses: `prior` (the default) adds
    /// `--search-prior-c · prior / (1 + worlds)` to each mean, so the
    /// policy's answer stays unless the worlds outweigh the policy's
    /// confidence; `paired` keeps the policy's answer unless another's mean
    /// improvement over it, world by world, exceeds
    /// `max(--search-margin, --search-z · se)`; `mean` takes the best mean
    /// (v1); `measure` never plays another answer: the policy's own is
    /// played with its own provenance and the worlds are only reported. Why
    /// `prior` is the default: `docs/trained-ai.md` §"Lookahead search".
    #[arg(long, value_enum, default_value_t = SearchSelect::Prior)]
    search_select: SearchSelect,
    /// What a root compares the policy's answer with: `ranking` (the
    /// default) its next answers by raw logit; `distill` the tactical
    /// layer's answer `R` of `--distill-labels`, alone, at every priority
    /// decision (any phase) where it differs from the net's `N`, nothing is
    /// owed and the two are not both mana abilities. With `distill` the
    /// search measures (`--search-select measure`) in fair worlds
    /// (`--search-fair`), with two candidates, and each such decision's
    /// label keeps the worlds as its `probe` (`distill_probe_v1`).
    #[arg(long, value_enum, default_value_t = SearchRival::Ranking, requires = "search")]
    search_rival: SearchRival,
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
    /// not bitwise reproducible, unless every call holds one row
    /// (`--inference-batch 1`, the CPU's way with `--inference-servers`).
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
    /// Fair worlds: every card the searching seat cannot see is dealt from the
    /// public prior its own view and deck list give (the house foresight's
    /// builder), never from another player's list (contract
    /// `belief_lookahead_v5`). A measurement of what a seat could do: it may
    /// run with `--choice-guard` and `--tactics-rules`.
    #[arg(long, requires = "search")]
    search_fair: bool,
    /// Rollouts are played to this many turn boundaries past the root (2:
    /// through the next player's turn in a duel) instead of to the seat's
    /// first settled pass; H still caps the seat's decisions. `0` (the
    /// default) keeps the settled stop.
    #[arg(long, default_value_t = 0, requires = "search")]
    search_turns: usize,
    /// Continue beyond ordinary turn and settled-priority stops. A search
    /// guard or refusal still censors a fork. Only terminal leaves are game
    /// outcomes. For paired offline measurements of the distillation rival.
    #[arg(long, requires = "search", conflicts_with = "search_turns")]
    search_until_game_over: bool,
    /// Earliest public turn number on which a measurement can sample a root.
    #[arg(long, default_value_t = 0, requires = "search")]
    search_min_turn: u32,
    /// Measure the raw policy and two distinct currently payable spells,
    /// including the complete mana plans, in paired fair worlds.
    #[arg(long, requires = "search_macros")]
    search_payable_pair: bool,
    /// Macro candidates at priority: the policy's answer, a pass, each spell
    /// the planner can pay for (its taps and the cast as one candidate),
    /// each ability and each land, read off the engine's offer, K in all.
    #[arg(long, requires = "search")]
    search_macros: bool,
    /// No house answer in a world: a rollout stops where its next answer
    /// would be the house's (a fallback of the net, or the fork's house for
    /// a refused answer) and the position before it is scored. Without it
    /// such answers are played and counted (`house_answers`).
    #[arg(long, requires = "search")]
    search_house_free: bool,
    /// A stall guard at the search's roots, as the house's: an answer the
    /// search gave that the table took back (the seat asked again from the
    /// same position) is not chosen there again; if it was the policy's
    /// own, the best mean of the others is played.
    #[arg(long, requires = "search")]
    search_stall_guard: bool,
    /// Every rollout of a searched decision at once (whatever
    /// `--search-threads`), stepped in lockstep: their net steps wait until
    /// every rollout still playing waits on one, and go to the
    /// `--inference-batch` lanes as one group, so a call holds up to K × P
    /// rows, the batches an accelerator needs. Needs `--inference-batch`;
    /// the summary's `inference_batch.lockstep_groups` gives the group
    /// sizes (`docs/trained-ai.md` §"Lookahead search").
    #[arg(long, requires = "search")]
    search_lockstep: bool,
    /// Another, cheaper net plays the search's rollouts: every chair in
    /// every world, from a memory of its own that watches the game beside
    /// the net's (`netplay4::search::RolloutNet4`). The net still decides
    /// at the root and gives the prior. Fair searches with
    /// `--inference-batch` only; the rollout net's calls run in batches
    /// of their own, as many servers as the net's.
    #[arg(long, requires = "search_fair")]
    rollout_net: Option<PathBuf>,
    /// How a rollout of `--rollout-net` is scored where it ends:
    /// `student` (the rollout net's own value), `jump` (the net's value of
    /// the seat's last frame, seen once from its memory at the root) or
    /// `follow` (the net watches every frame of the rollout, as when it
    /// plays the rollouts itself).
    #[arg(long, default_value = "jump", requires = "rollout_net")]
    rollout_leaf: baylee_train::netplay4::search::RolloutLeaf4,
    /// The net's own search answers; at each of its roots the same worlds
    /// are also searched with `--rollout-net` under each of these leaves
    /// (comma-separated), recorded in the search event as `variants`.
    #[arg(long, value_delimiter = ',', requires = "rollout_net")]
    rollout_compare: Vec<baylee_train::netplay4::search::RolloutLeaf4>,
}

/// `--search-select`: which [`Selection4`] a search uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum SearchSelect {
    Mean,
    Paired,
    Prior,
    Measure,
}

/// `--search-rival`: what a root compares the policy's answer with
/// ([`baylee_train::netplay4::search::Rival4`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum SearchRival {
    Ranking,
    Distill,
}

/// Why `--search-rival distill` cannot run with `args`, if it cannot: the
/// layer's answer comes from `--distill-labels` on the raw net's own play,
/// and the probe is a measurement in fair worlds of exactly two candidates.
fn rival_refusal(args: &Args) -> Option<&'static str> {
    if args.search_rival != SearchRival::Distill {
        return None;
    }
    let search = args.search?;
    if args.distill_labels.is_none() {
        return Some("--search-rival distill reads the layer's answer from --distill-labels");
    }
    if args.search_select != SearchSelect::Measure {
        return Some("--search-rival distill measures: --search-select measure");
    }
    if !args.search_fair {
        return Some("--search-rival distill plays fair worlds: --search-fair");
    }
    if search.candidates() != 2 {
        return Some("--search-rival distill compares two answers: --search 2,P,H");
    }
    if args.search_targets > 0
        || args.search_combat > 0
        || args.search_macros
        || args.search_stall_guard
    {
        return Some(
            "--search-rival distill takes no targets, combat roots, macros or stall guard",
        );
    }
    if args.choice_guard != "off" || !args.tactics_rules.is_empty() {
        return Some(
            "--search-rival distill probes the raw net's own answers: no --choice-guard or \
             --tactics-rules",
        );
    }
    None
}

/// Whether `--distill-labels` may run beside `--search`: only a measuring
/// search whose rival is the layer's answer, which plays nothing the net did
/// not and keeps its provenance.
fn distill_probe(args: &Args) -> bool {
    args.search.is_some()
        && args.search_rival == SearchRival::Distill
        && args.search_select == SearchSelect::Measure
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
        SearchSelect::Measure => Selection4::Measure,
    };
    if let Some(refusal) = rival_refusal(args) {
        anyhow::bail!(refusal);
    }
    if args.search_payable_pair {
        anyhow::ensure!(
            args.search_fair
                && args.search_house_free
                && args.search_select == SearchSelect::Measure
                && args.search_rival == SearchRival::Ranking
                && args.search_macros
                && config.candidates() == 3
                && args.raw_policy
                && args.tactics_rules.is_empty()
                && args.choice_guard == "off"
                && args.search_targets == 0
                && args.search_combat == 0
                && !args.search_stall_guard
                && args.distill_labels.is_none(),
            "payable pairs require three candidates, fair house-free measurement of the raw net, and ordinary macro roots"
        );
    }
    anyhow::ensure!(
        !(args.search_until_game_over || args.search_min_turn > 0)
            || distill_probe(args)
            || args.search_payable_pair,
        "terminal or turn-filtered measurements require a fair distillation rival or a payable spell pair"
    );
    let rival = match args.search_rival {
        SearchRival::Ranking => baylee_train::netplay4::search::Rival4::Ranking,
        SearchRival::Distill => baylee_train::netplay4::search::Rival4::Distill,
    };
    let config = config
        .with_rival(rival)
        .with_selection(selection)?
        .with_threads(args.search_threads)?
        .with_targets(args.search_targets)?
        .with_combat(args.search_combat)?
        .with_combat_only(args.search_combat_only)
        .with_neural_opponent(args.search_neural_opponent)
        .with_fair(args.search_fair)
        .with_turns(args.search_turns)?
        .with_game_end(args.search_until_game_over)
        .with_payable_pair(args.search_payable_pair)
        .with_macros(args.search_macros)
        .with_house_free(args.search_house_free)
        .with_stall_guard(args.search_stall_guard)
        .with_lockstep(args.search_lockstep);
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
    /// `--search-student`: what the student did over the run.
    student: Option<baylee_train::netplay4::student::StudentStats4>,
    /// `--grouped-decode`: what the grouping decided over the run.
    grouped: baylee_train::netplay4::GroupedUse4,
    /// `--think-adaptive`: second passes over the run.
    thinking: baylee_train::netplay4::ThinkingUse4,
    tactician:
        std::collections::BTreeMap<String, baylee_train::netplay4::tactician::TacticianStats4>,
    /// `--explore-rules`: decisions explored and layer answers played.
    exploration_draws: u64,
    exploration_played: u64,
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

/// Why `--rollout-net` cannot run with `args`, if it cannot: the seats it has
/// no part in would play on while the run said it played their worlds, and
/// `identity` is the rollout net's own.
fn rollout_refusal(
    args: &Args,
    collection: &collector::Contract,
    identity: Option<&ModelIdentity>,
) -> Option<&'static str> {
    args.rollout_net.as_ref()?;
    if collection.uses_seat_pool() || args.offline_deadline_seat {
        return Some(
            "--rollout-net plays the worlds of one net seat: no neural opponent and no \
             --offline-deadline-seat",
        );
    }
    if args.search_rival == SearchRival::Distill {
        return Some("--search-rival distill probes the net's own rollouts: no --rollout-net");
    }
    if matches!(
        identity.and_then(|identity| identity.critic_semantics.as_ref()),
        Some(CriticSemantics4::FailureTerminalResearchUtilityV1 { .. })
    ) {
        return Some(
            "--rollout-net scores leaves with an MTG outcome critic, not a research utility",
        );
    }
    None
}

fn deadline_refusal(args: &Args, collection: &collector::Contract) -> Option<&'static str> {
    if !args.offline_deadline_seat {
        return None;
    }
    if args.threads != 1
        || args.seats != 2
        || args.deck_split != "train"
        || args.holdout_tier != "train_active"
        || args.decks_manifest.is_none()
        || args.generated != 0
        || !args.no_baseline
        || !args.numbers_only
        || args.record_out.is_none()
        || args.legacy_v3
        || args.random_policy
        || args.opponent_model.is_some()
        || collection.uses_seat_pool()
        || args.shadow_teacher.is_some()
        || args.seer_peeks
        || args.collection_purpose != collector::Purpose::Evaluation
        || args.temperature != 0.0
    {
        return Some(
            "offline deadline needs one recorded, numbers-only, greedy 1v1 TRAIN table worker against the house",
        );
    }
    if args.search.is_some()
        && (!args.search_fair
            || !args.search_macros
            || !args.search_neural_opponent
            || !args.search_lockstep
            || args.search_targets != 0
            || args.search_stall_guard
            || args.inference_batch == 0
            || args.inference_servers != 1)
    {
        return Some(
            "offline deadline needs a separate batched backend with fair lockstep macro search and no target or stall extensions",
        );
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

/// `--memory-reset`'s words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum MemoryResetArg {
    Decision,
    Turn,
}

/// The memory reset the seat under test plays with.
fn memory_reset(args: &Args) -> baylee_train::netplay4::MemoryReset4 {
    use baylee_train::netplay4::MemoryReset4;
    match args.memory_reset {
        None => MemoryReset4::Off,
        Some(MemoryResetArg::Decision) => MemoryReset4::Decision,
        Some(MemoryResetArg::Turn) => MemoryReset4::Turn,
    }
}

/// Why `--memory-reset` or `--decision-trace` cannot run with `args`, if
/// they cannot: a reset plays another policy than the one recorded
/// probabilities and searches assume, so it measures an evaluation only.
fn memory_reset_refusal(args: &Args, collection: &collector::Contract) -> Option<&'static str> {
    if args.memory_reset.is_some() {
        if collection.purpose != collector::Purpose::Evaluation {
            return Some("--memory-reset is a measurement: evaluation purpose only");
        }
        if args.legacy_v3 || args.random_policy {
            return Some("--memory-reset needs a v4/v5 net under test");
        }
        if args.search.is_some()
            || args.shadow_teacher.is_some()
            || args.tactician_windows.is_some()
        {
            return Some("--memory-reset plays no search, shadow teacher or tactician");
        }
    }
    if args.decision_trace && (args.legacy_v3 || args.search.is_some()) {
        return Some("--decision-trace reads a v4/v5 net's own menus: no legacy net or search");
    }
    None
}

/// `--explore-rules`/`--explore-epsilon`: the learner's rule groups and the
/// run's exploration contract, or `None` without them. Only an on-policy
/// collection explores, and only its raw learner: an evaluation plays the
/// policy it measures.
fn exploration_setup(
    args: &Args,
    collection: &collector::Contract,
) -> anyhow::Result<
    Option<(
        baylee_train::netplay4::TacticRules4,
        baylee_train_ops::exploration::RunExploration,
    )>,
> {
    let (Some(list), Some(epsilon)) = (args.explore_rules.as_deref(), args.explore_epsilon) else {
        return Ok(None);
    };
    anyhow::ensure!(
        collection.purpose == collector::Purpose::OnPolicyTraining,
        "--explore-rules explores in --collection-purpose on-policy-training only"
    );
    anyhow::ensure!(
        !args.legacy_v3
            && args.tactician_windows.is_none()
            && args.search.is_none()
            && !args.random_policy
            && args.choice_guard == "off"
            && args.tactics_rules.is_empty()
            && !args.tactics_shadow,
        "--explore-rules explores the raw learner alone: no tactician, search, random seat, \
         guard or tactics rules"
    );
    let list = if list == "measured" {
        baylee_train_ops::measured::RULE_GROUPS
    } else {
        list
    };
    let rules = baylee_train::netplay4::TacticRules4::parse(list)?;
    let run = baylee_train_ops::exploration::RunExploration::new(epsilon, list, &rules.names());
    run.validate()?;
    Ok(Some((rules, run)))
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
    rollout_identity: Option<&ModelIdentity>,
) -> Option<Value> {
    let mut metadata = config?.metadata();
    metadata["max_decisions_per_game"] = json!(args.search_max_decisions_per_game);
    if args.search_min_turn > 0 {
        metadata["min_turn"] = json!(args.search_min_turn);
    }
    // A row's outputs in a batch differ from its outputs alone, and which
    // rows share a batch depends on timing: a batched search is not
    // reproducible bit for bit (`docs/trained-ai.md` §"Lookahead search").
    // Calls of one row are the unbatched steps, whichever server runs them.
    // A search stopped by the wall clock is no more reproducible.
    metadata["bitwise_reproducible"] =
        json!(args.inference_batch <= 1 && !args.offline_deadline_seat);
    metadata["critic_semantics"] = json!(identity.and_then(|i| i.critic_semantics.as_ref()));
    if let Some(rollout) = &args.rollout_net {
        // Only a search whose rollouts another net plays names it. Where the
        // leaves are only compared, the net's own search answers, and its
        // rollouts and leaf are the ones above.
        let plays = args.rollout_compare.is_empty();
        if plays {
            metadata["rollout"] = json!(if args.search_neural_opponent {
                "rollout_net_for_every_chair_other_seats_from_world_view_only"
            } else {
                "rollout_net_for_the_searching_seat_house_for_every_other_chair"
            });
        }
        metadata["rollout_net"] = json!({
            "model": rollout,
            "model_identity": rollout_identity,
            "leaf": plays.then(|| args.rollout_leaf.name()),
            "compare": args.rollout_compare.iter().map(|leaf| leaf.name()).collect::<Vec<_>>(),
            "memory": "the_rollout_net_watches_every_view_of_the_seat_at_the_table",
        });
    }
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

fn parse_profile(spec: &str) -> anyhow::Result<AIProfile> {
    profile_spec::parse(spec)
}
