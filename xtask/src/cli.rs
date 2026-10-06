//! The command line: every subcommand and its flags.

use crate::{Parser, PathBuf, Subcommand, verify};

#[derive(Parser)]
#[command(name = "xtask", about = "baylee development tasks", version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) cmd: Cmd,
}

#[derive(Subcommand)]
pub(crate) enum Cmd {
    /// Regenerate subtype constants, card stubs, registry, and the script index.
    Codegen {
        /// Transfer an unfinished generated card to hand ownership, without
        /// claiming it is implemented. No other files are regenerated.
        #[arg(long, conflicts_with_all = ["check", "tables"])]
        adopt_stub: Option<String>,
        /// Verify generated files are up to date instead of writing (CI).
        #[arg(long)]
        check: bool,
        /// Write only the tables built from the **compiled** pool.
        ///
        /// `generated_lines.rs` and `generated_names.rs` are the two-phase
        /// halves: they read the pool as it is compiled right now, against
        /// the cached printings, and need no card-script reference at all.
        /// Everything before them does — and a full run on a machine with
        /// no corpus checked out rewrites every machine-owned card as an
        /// honest `// GENERATED STUB`, which is a correct answer to the
        /// question it was asked and a destroyed working tree.
        ///
        /// So: use this after editing a card by hand, or after changing
        /// what `baylee_cards_codegen::lines` reads, to bring the two
        /// tables back in step with the pool. A machine that has the corpus
        /// runs the whole thing and never needs it.
        ///
        /// It also files a token newly written by hand in `tokens.rs` in
        /// `generated_tokens.rs`: that half of the ledger reads only
        /// `tokens.rs` and the ledger itself, and may only append, so a card
        /// finished by hand can name a token of its own without the corpus.
        #[arg(long)]
        tables: bool,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Assign a `CardIndex` to every card in the corpus that has none.
    ///
    /// The only thing that writes the ledger, and the reason `codegen` never
    /// does: an index assigned as a side effect of a build takes its order
    /// from whatever happened to be fetched that day. This takes its order
    /// from the corpus — first appearance, over every card that exists — so
    /// adopting an old card inserts nothing.
    ///
    /// The ledger is `crates/baylee-cards-index/src/generated.rs`, a compiled
    /// table rather than a data file, because a data file is a second truth
    /// beside the code that nothing checks. So this reads the table it is
    /// about to write — one build old, which is safe because assignment only
    /// ever appends — and the build is what checks what came out.
    ///
    /// The corpus comes from `baylee-catalog corpus`, which needs a database.
    /// This half needs only the file it writes, so codegen stays offline.
    Ledger {
        /// The corpus, as `baylee-catalog corpus` writes it.
        #[arg(long, default_value = "data/card-corpus.tsv")]
        corpus: PathBuf,
        /// Report what would be assigned instead of writing it.
        #[arg(long)]
        check: bool,
        /// Throw every assignment away and number the corpus from zero.
        ///
        /// It reads no ledger at all — nothing is carried over, because a
        /// carried row is the one number the corpus did not produce. What
        /// protects a card this repo implements is the check at the end: the
        /// run asks the **registry** whether every compiled card got a row
        /// and refuses to write if one did not, and the answer to a refusal
        /// is `data/corpus-keep.tsv`. Every stored `CardIndex` anywhere
        /// becomes wrong, so this is a deliberate, one-off,
        /// migrate-or-discard operation and never a part of maintenance.
        #[arg(long)]
        reseed: bool,
    },
    /// Measure how well the compiled ability list lines up with the printed sentences.
    ///
    /// A report, not a check: it is the design input for the per-ability
    /// line index the stack panel needs.
    AbilityLines,
    /// Dump every compiled `CardDef` — the equivalence check for a refactor.
    ///
    /// A change that is meant to alter no rules (new macros, shared filters,
    /// a reshuffled literal) must leave this output byte-identical. Take a
    /// dump before, one after, and diff: anything that moved has the card's
    /// name on it. It is a tool rather than a test because there is nothing
    /// for it to assert on its own — the baseline lives outside the repo.
    /// Read a deck file and say what this pool can do with it.
    ///
    /// Two questions, and the first is the one a deck arriving from
    /// somewhere else needs answered. **Does every row round-trip?**
    /// `deckrow::Row` writes back what it parsed, so a line that does not
    /// survive `parse` then `to_string` is a line one of the two sides reads
    /// differently — which is how a printing silently becomes part of a card
    /// name. And **what can be played?** A name the pool does not carry is
    /// unplayable here whatever its printing says, and a name it carries as
    /// a stub is worse than that: the deckbuilder offers it.
    DeckCheck {
        /// The deck file, in `[deck:Name]` / `[sideboard]` / `[commander]`
        /// sections with one `deckrow` line each.
        file: PathBuf,
        /// Name every card that is not `Coverage::Implemented`.
        #[arg(long)]
        verbose: bool,
    },
    /// How far each card is verified, L1 (implemented) to L5 (mutation-
    /// killed), for the pool and every house deck (`xtask/src/verify.rs`).
    Verify(verify::Args),
    /// Import the retail precons from MTGJSON as Baylee text under
    /// `data/decks/precon/`, then write their status (`docs/precons.md`).
    ///
    /// Cards are matched on oracle id through the ledger and written under
    /// the ledger's name; a card the ledger does not know is written as the
    /// source names it and reported. Idempotent: the same archive writes the
    /// same files, and a list that did not change is not rewritten.
    DecksImport {
        /// A local `AllDeckFiles.tar.gz` to read instead of the cached or
        /// downloaded one.
        #[arg(long)]
        archive: Option<PathBuf>,
        /// Download the archive again even if a cached copy exists.
        #[arg(long)]
        refresh: bool,
    },
    /// Hold every precon file against the pool: write
    /// `data/decks/precon/STATUS.tsv` and the gateway's embedded list of the
    /// playable ones, or with `--check` only compare (`docs/precons.md`).
    DecksStatus {
        /// Compare instead of writing; fail if either file is stale.
        #[arg(long)]
        check: bool,
    },
    PoolDump {
        /// Where to write the dump.
        #[arg(long)]
        out: PathBuf,
    },
    /// Report how much of the card-script reference corpus the transcoder reads.
    TranscodeReport {
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Print this many refused scripts, for finding the next rule to add.
        #[arg(long, default_value_t = 0)]
        samples: usize,
        /// Show only samples whose refusal reason contains this text.
        ///
        /// The ranking names a reason; this is how you read the scripts
        /// behind one of them without grepping the corpus by hand and
        /// guessing which of them the transcoder actually stopped on.
        #[arg(long)]
        reason: Option<String>,
        /// Rank only this project's own unfinished cards.
        ///
        /// The corpus is 33666 scripts; the deckbuilder offers about a
        /// thousand, and most of those are already finished — the lands by
        /// `landgen`, the rest by hand. So the two rankings answer different
        /// questions: the corpus says what the transcoder is worth in
        /// general, and this says which of *our* stubs the next rule would
        /// finish, which is the one a player would notice.
        #[arg(long)]
        stubs: bool,
        /// Rank only the cards named in this file, one Scryfall name a line.
        ///
        /// A set is worked through before its cards are in the pool, so
        /// `--stubs` cannot see them yet; this is the ranking over a set's
        /// worklist (`#` lines and blanks are skipped). With `--stubs`, the
        /// two narrow together: a stub the file does not name is left out.
        #[arg(long)]
        names: Option<PathBuf>,
        /// How many refusal causes to rank. 0 prints every one of them.
        ///
        /// The ranking is long — a corpus run distinguishes several hundred
        /// causes — and the tail is where a mechanic this project is *about
        /// to* write sits, because a rule nothing needs yet blocks a handful
        /// of scripts rather than a thousand. The default is a screenful,
        /// and what it leaves out is counted rather than dropped in silence.
        #[arg(long, default_value_t = 30)]
        causes: usize,
    },
    /// Name the cards the reader would write in full and this pool does not
    /// have — the worklist §E8 step 1 is made of.
    ///
    /// `transcode-report` measures the corpus and ranks what is missing;
    /// this answers the other half of the same reading, which is which
    /// *cards* the transcoder could hand over today. The two differ in what
    /// they walk: the report walks scripts, and a script is not a card here
    /// until the ledger has a row for it and Scryfall has a name to fetch,
    /// so this walks the **ledger** and looks the script up, which is the
    /// order that cannot emit a name `codegen` would then fail loudly on.
    ///
    /// A name it writes is a claim about one reader and nothing else: that
    /// `scriptgen` claims every clause of the script. Whether the card comes
    /// out is still decided by the run — `stubgen::transcode_card` needs a
    /// printing this cache may not hold, and an honest stub is a correct
    /// outcome — so a batch is measured after `codegen`, never from here.
    /// Take the next cards the reader can already write, and run the batch
    /// procedure over them.
    ///
    /// `docs/mechanics-roadmap.md` §E8 writes this procedure out in prose,
    /// and #49 writes it out twice more. A procedure written three times is
    /// one that gets run wrong once — and the step that gets skipped is the
    /// **second** `codegen`, whose absence looks exactly like success: the
    /// cards are there, they compile, and the two tables built from the
    /// compiled pool have no rows for them, so a client draws a stack entry
    /// that says nothing. `codegen --check` is the only thing that sees it,
    /// and it is in here.
    ///
    /// The list is measured fresh every time rather than sliced off the last
    /// batch's file, because it moves when the reader moves: a refusal
    /// learned in between takes names off it that a stale slice would still
    /// propose.
    Batch {
        /// How many cards to take, in ledger order — oldest printing first,
        /// so two runs a week apart propose the same batch.
        #[arg(long, default_value_t = 100)]
        count: usize,
        /// Say which names would be appended, and stop before writing.
        #[arg(long)]
        dry_run: bool,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    ReachList {
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Write the names here, one per line, in the form
        /// `data/card-pool.txt` and `card-batch --cards` both take.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Write at most this many names. 0 is every one of them.
        ///
        /// The slice is taken in ledger order, which is oldest printing
        /// first, so two runs a week apart propose the same batch and a
        /// batch that was reverted is proposed again rather than skipped.
        #[arg(long, default_value_t = 0)]
        count: usize,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Read every hand-written card a second way and report the disagreements.
    ///
    /// A hand-written card is the one surface in the pool that no program
    /// checks against another program. `validate` pins its header to the
    /// printing and the engine sweeps pin its behaviour to the `CardDef` it
    /// builds — but the `CardDef` itself was typed by a person reading the
    /// card, and a person who read a clause wrong produces a card every one
    /// of those checks agrees with. The transcoder is a second reader of the
    /// same card from a different source, so where it reads a script **in
    /// full** and comes out with a different shape, one of the two is wrong.
    ///
    /// A disagreement never fails it. The transcoder writes what one rule
    /// can say, and a hand-written card is allowed to say more (a mode, a
    /// ward, a saga chapter). What the list is, is the shortest set of cards
    /// worth a second pair of eyes, and it is ranked by nothing — every line
    /// on it names a card and what the two readers disagreed about.
    ///
    /// The second reader is read at whichever of **two depths** the script
    /// reaches. Transcoding is the deep one and needs every clause claimed,
    /// which a hand-written card's script almost never offers — it is
    /// hand-written *because* a reader could not write it, so only 22 of the
    /// 207 overlap and that number shrinks as the transcoder grows. Counting
    /// the parser's own line kinds is the shallow one and reaches 141 of the
    /// 207 — those same 22 and 119 more: a script stopped on one unclaimed
    /// parameter still says plainly how many abilities it has and of what
    /// kind.
    ///
    /// Both depths obey the transcoder's honesty rule — **one unread clause
    /// and the script is not counted** — because the alternative is a tool
    /// that reports its own blind spots as defects in the cards. The skips
    /// are counted and *named* for the same reason, so the tail of the
    /// report is a worklist: thirteen cards are unreadable only because
    /// `K:ETBReplacement` hides an ability inside an `SVar` chain, and five
    /// only because `keyword_const` has no row for daybound.
    ///
    /// What it caught on its first honest run is the whole argument for it,
    /// and it is worth stating exactly, because the tool compares **shape**
    /// and nothing else. It pointed at *one* card: Raffine's Tower, which
    /// claimed `Coverage::Implemented` with no basic land types and no
    /// cycling ability at all. Reading the three neighbours in that cycle by
    /// hand — which is what a report is for — found Indatha, Raugrin and
    /// Zagoth cycling for `{2}` against the `{3}` their own `//! Oracle:`
    /// header prints. A wrong cost is invisible here by construction, so
    /// three of the four are this command's credit only in the sense that it
    /// put a person in front of the right file. Every other check in the repo
    /// agreed with all four, which is the hole this fills: `validate` pins a
    /// header to its printing and the sweeps pin behaviour to the `CardDef`,
    /// and neither can see a `CardDef` a person typed wrong.
    CrossRead {
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Print this many disagreeing cards with the script that caused it.
        #[arg(long, default_value_t = 0)]
        samples: usize,
    },
    /// Hold every `CR` citation in the tree against a local rules copy.
    ///
    /// A report and not a gate, for a reason that is not taste: the rules
    /// text is Wizards' and is deliberately not vendored (`docs/legal.md`
    /// §2), so CI has no copy and nothing in the test suite can read one.
    /// The citations rot anyway — an audit of all 1693 of them found 246
    /// naming the wrong rule, because Wizards renumbers a section whenever
    /// they insert a keyword action into it and sacrifice moved from 701.19a
    /// to 701.21a without anybody here touching a file.
    ///
    /// It checks two mechanical things. That the number exists at all. And
    /// that a line using one of the rules' **own** heading words — every
    /// `701.N` is a keyword action and every `702.N` a keyword ability —
    /// cites a number under that word. The second is the one worth having,
    /// because it is the exact shape the drift takes: the prose stays right
    /// and the number slides out from under it.
    ///
    /// `xtask/src/cr_check.rs` has the four rules that keep it quiet on the
    /// correct citations, each of which was added because it fired on one.
    CrCheck {
        /// A Comprehensive Rules text, relative to the repository or absolute.
        #[arg(long, default_value = "../mtg/MagicCompRules.txt")]
        rules: PathBuf,
        /// Print every citation beside the rule it names, not just the findings.
        #[arg(long)]
        all: bool,
    },
    /// Rank the land sentences `landgen` cannot read yet.
    ///
    /// The counterpart to `transcode-report`, and worth its own command for the
    /// reason that one exists: every card in the pool that is still a
    /// generated stub is a land — 792 of them — and land text is formulaic.
    /// A sentence shape taught to `landgen` is not one card; it is every land
    /// that prints that shape, generated with nobody reading the result.
    LandReport {
        /// Name up to this many example lands per shape.
        #[arg(long, default_value_t = 3)]
        samples: usize,
        /// Write the cards a parser should *not* be taught, one name per line.
        ///
        /// The split this command exists to make. A shape printed on eight
        /// lands repays a rule in `landgen`; a shape printed on one is a rule
        /// per card, which is what a person or a model does. This writes the
        /// second half — the tail, plus the lands that are not plain lands at
        /// all — in the form `card-batch --cards` takes.
        #[arg(long)]
        worklist: Option<PathBuf>,
        /// A shape printed on fewer than this many lands goes to the worklist.
        #[arg(long, default_value_t = 2)]
        tail: usize,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Choose the cards that would teach the engine the most, and say what
    /// each one asks for.
    ///
    /// Greedy set cover over the mechanics the corpus actually uses: every
    /// script contributes atoms (`api:Token`, `param:Pump.Duration`,
    /// `kw:Equip`, `line:S`), atoms already appearing in scripts the
    /// transcoder reads in full are struck off as known, and each card is
    /// scored by how many *cards elsewhere in the corpus* its remaining
    /// atoms would unblock. Picking by hand instead reliably picks famous
    /// cards, which are famous for their flavour, not their mechanics.
    CoverageSet {
        /// How many cards to choose.
        #[arg(long, default_value_t = 100)]
        count: usize,
        /// Skip a card that would need more than this many new mechanics —
        /// a planeswalker with three novel modes is a worse first card than
        /// three cards with one each.
        #[arg(long, default_value_t = 6)]
        max_new: usize,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
    },
    /// Show Scryfall + card-script reference data for a card side by side.
    Explain {
        /// Exact card name.
        #[arg(long)]
        name: String,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Prepare per-card task packages for LLM implementation batches.
    CardBatch {
        /// Only these cards (comma-separated names); default: every card in
        /// the pool that is still a generated stub.
        #[arg(long)]
        cards: Option<String>,
        /// Output directory for task packages.
        #[arg(long, default_value = "target/card-batch")]
        out: PathBuf,
        /// Path to the card-script reference cardsfolder.
        #[arg(long, default_value = "../mtg/card-scripts")]
        scripts: PathBuf,
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
    },
    /// Fill the Scryfall payload cache, and nothing else.
    ///
    /// What `codegen` does on its way to writing files, as a command of its
    /// own — so a scheduled job can warm the cache without building the
    /// generated tree, and without the card-script corpus it would need to
    /// generate anything. A cold cache is one bulk download and a handful of
    /// requests; a warm one is no network at all.
    ScryfallCache {
        /// Directory for cached Scryfall responses.
        #[arg(long, default_value = "data/scryfall-cache")]
        cache: PathBuf,
        /// Write every payload again, held or not.
        ///
        /// The cache is a **typed projection** of Scryfall's rows rather than
        /// a copy of them: what lands on disk is whatever
        /// `scryfall::ScryfallCard` declares, so teaching that struct a new
        /// field leaves every held file without it. A plain run cannot see
        /// that — it fills what is *missing*, and nothing is missing — so a
        /// generator that needs the new field would refuse over a cache no
        /// command could repair. One bulk download, the same as a cold start.
        #[arg(long)]
        refetch: bool,
    },
    /// Validate card-file conventions (header, coverage, tests).
    Validate,
    /// Take a generated card off the machine, so a person owns it from now on.
    ///
    /// A card one of the readers wrote in full is **machine-owned**: codegen
    /// rewrites it on every run, which is what makes "fix the reader, not the
    /// card" enforceable rather than a convention — a rule corrected in
    /// `landgen` or `scriptgen` reaches every card that rule wrote, at once.
    /// The cost is that a hand edit to such a file is reverted on the next
    /// run, silently as far as the editor is concerned.
    ///
    /// This is the way out, and it is deliberately a decision someone makes
    /// on purpose: it strips the ownership marker, and from then on the file
    /// is hand-owned like any card a person wrote from the stub. Reach for it
    /// when the card genuinely needs something the reader cannot say — not to
    /// get past a transcoding bug, which belongs in the reader where it fixes
    /// the other cards it also broke.
    Adopt {
        /// Printed card name, as the pool spells it.
        #[arg(long)]
        name: String,
    },
    /// Rewrite every card's `//! Oracle:` header from its cached printing.
    ///
    /// The header is *derived* data — Scryfall's own text, copied into the
    /// file so a person can read the card beside the code built from it — so
    /// a tool writes it and `validate` compares it. Hand-editing forty-eight
    /// of them by retyping rules text is exactly the transcription step this
    /// check exists to catch.
    ///
    /// It touches the header and nothing else, hand-owned files included:
    /// what a card *does* stays whoever's it was, and only the sentence it
    /// is measured against is refreshed. That is also how an errata is taken
    /// — refresh, then read the diff, because a changed printing usually
    /// means the implementation below it has to change too.
    RefreshOracle {
        /// Print what would change and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Seat a dev account at a table and print (or play) its ticket.
    ///
    /// Skips the lobby's sign-in and deck-picking screens and nothing else:
    /// the account, the deck, the room and the seat are all made through the
    /// gateway's own HTTP routes, and the game that comes out is played over
    /// the same engine ⇄ gateway ⇄ client sockets as any other.
    DevTable {
        /// Gateway base URL.
        #[arg(long, default_value = "http://127.0.0.1:28766")]
        gateway: String,
        /// How many chairs. Every other chair goes to the named AI profile.
        #[arg(long, default_value_t = 2)]
        seats: usize,
        /// Which difficulty the AI chairs play at.
        #[arg(long, default_value = "steady")]
        ai: String,
        /// Which acceptance deck to bring.
        #[arg(long, default_value = "Allytifact")]
        deck: String,
        /// Which side each chair plays for, in seat order — `1,1,2` is a
        /// 2v1. `0` leaves a chair on its own side. Needs three chairs or
        /// more, a duel already having exactly two sides.
        #[arg(long, value_delimiter = ',')]
        teams: Vec<u8>,
        /// Launch the client on the seat instead of printing its ticket.
        #[arg(long)]
        play: bool,
        /// Seat a bridge in chair 1 instead of the AI: `baylee-seat join`
        /// against the same gateway, playing this mind (`house`,
        /// `scripted`, `anthropic[:<model>]` or `openai:<model>`, whose key
        /// the bridge reads from this environment) under the name its mind
        /// discloses (`HOUSE-house`, `TEST-scripted`, `LLM-sonnet-5-5`), with
        /// the other acceptance deck. Its transcripts go to
        /// `target/seat-transcripts/`. `profile:<name>` plays that profile
        /// of the bridge's settings file (`llm-seat.json`, or the one
        /// `BAYLEE_SEAT_CONFIG` names; `docs/llm-seat.md`) under its daily
        /// and monthly caps. A model the bridge has no price for sits down
        /// here only as a profile that states its price or `game_tokens`.
        #[arg(long = "bridge", value_name = "MIND")]
        bridges: Vec<String>,
    },
    /// Make the key release archives are signed with, or show its public half.
    ///
    /// The seed goes only to the file (mode 600) and is never printed; an
    /// existing file is never overwritten (#326, `docs/releasing.md`).
    UpdateKey {
        /// Where the seed lives [default: ~/.config/baylee-release/update-signing.key].
        #[arg(long)]
        out: Option<PathBuf>,
    },
}
