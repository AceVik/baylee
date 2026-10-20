# A query language, and the dialog that builds one

Nothing of this is implemented. One rule stands over all of it: **the typed
text is the only truth.** There is no printer that re-sets the query — every
term carries its own byte span, and a control in the dialog replaces exactly
that span. What the dialog does not understand it therefore cannot touch,
structurally.

It is written down because the design was decided — three were drawn and one
was chosen — and a design that lives only in a conversation is a design that
has to be had again. What gets built is the **Sentence** design (a chain of
chips over the typed text, splicing instead of printing), with three grafts:
the count per value and the permanently visible field column from **Facets**,
the two separate refusal sentences and the count per condition from **Rows**.

---

## 1. Where it lives

**`crates/baylee-client-core/src/deckbuilder/query/`** — four modules
(`mod.rs`, `lex.rs`, `parse.rs`, `eval.rs`) beside `builder.rs` and `tests.rs`
(`crates/baylee-client-core/src/deckbuilder/`, 1215 + 972 lines today).

**No entry in any `Cargo.toml`.** `crates/baylee-client-core/Cargo.toml:13-19`
lists exactly `baylee-core`, `baylee-engine`, `baylee-view`, `glam`, `serde`,
`serde_json`; a hand-written lexer and parser needs none of them except
`core`. The dependency graph does not move.

Why there and not in a crate of its own:

- **wasm32.** `baylee-client-core` is in the list of five crates that must
  compile for `wasm32-unknown-unknown` (CLAUDE.md, §one-way data flow). A new
  crate would have to redeem the same obligation from scratch; here it already
  holds.
- **The second user lives here.** The trait from §4 has two implementations,
  and both sit in this crate: `deckbuilder::PoolCard`
  (`crates/baylee-client-core/src/deckbuilder.rs:67`) and `browser::BrowseRow`
  (`crates/baylee-client-core/src/browser.rs:126`) — the row of the zone
  browser, the client's second card list. A crate of its own would bring
  nothing but a third file.
- **Whoever links the crate decides.** The server-side copy
  `baylee_cards::pool::PoolCard` (`crates/baylee-cards/src/pool.rs:107`) gets
  three new fields (§4) and **no** trait implementation. Otherwise
  `baylee-cards` would have to link the parser crate — and `baylee-cards`
  hangs in every engine build. The server does not filter: `GET /pool`
  (`crates/baylee-gateway/src/pool.rs:58`) sends the whole pool, every filter
  runs locally.
- **No regex crate.** See §10.

The drawing happens in **`crates/baylee-client/src/queryui.rs`**, new, called
from `buildui::builder` (`crates/baylee-client/src/buildui.rs:64`) as the last
child after the printing picker (`buildui.rs:149-153` — "Last, so it sits over
both halves whatever the frame is").

---

## 2. The grammar

### AST

```rust
/// A byte span in the query. Every span indexes `Query::text`.
pub type Span = core::ops::Range<usize>;

/// A parsed query. It owns its text; without it no span is worth
/// anything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    text: String,
    root: Disjunction,
    faults: Vec<Fault>,
}

/// `or` binds loosest, so the root is always a disjunction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Disjunction {
    pub runs: Vec<Conjunction>,
    /// The spans of the `or` marks between them; exactly `runs.len() - 1`.
    /// Kept separate, because the dialog's `·` target replaces only those.
    pub joins: Vec<Span>,
    pub span: Span,
}

/// Juxtaposition is AND (`and_expr := unary (AND? unary)*`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conjunction {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Term(Term),
    Group(Group),
    /// Bytes the parser reached and could not read.
    Rubble(Rubble),
}

/// `-( … )` or `( … )`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub negated: bool,
    pub inner: Disjunction,
    /// From `-` or `(` up to and including `)`.
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    pub negated: bool,
    pub field: FieldRef,
    pub op: Op,
    pub value: Value,
    /// The whole term, the leading `-` included. What `splice` replaces.
    pub span: Span,
    /// The value alone. What the editor selects in the text line.
    pub value_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldRef {
    /// A keyword this client knows, together with the spelling the player
    /// used — `t` and `type` are the same field and not the same bytes.
    Known { field: Field, spelling: Span },
    /// Lexes as `word:value`, but no such keyword.
    Unknown { spelling: Span },
    /// A bare word or a quoted string.
    ///
    /// **Not** the same as `name:`. It runs over all five haystacks the box
    /// has today (§4) — deliberately wider than Scryfall, so that step 5 in
    /// §9 moves no result.
    Bare,
    /// The prefix `!` — exact name, binds exactly one token.
    Exact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op { Colon, Eq, Ne, Lt, Le, Gt, Ge }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// Unquoted.
    Word(Span),
    /// In `"` or `'`; `text` is the content, apostrophes normalised.
    Quoted { span: Span, text: String },
    /// `/…/` including the slashes. Never evaluated (§10).
    Regex(Span),
}

/// A remainder the parser leaves standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rubble { pub span: Span, pub why: FaultKind }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault { pub span: Span, pub kind: FaultKind }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// `o:"draw a` — quote left open.
    UnclosedQuote,
    /// `t:goblin (c:r` — paren left open.
    UnclosedParen,
    /// `o:/dra` — pattern left open.
    UnclosedRegex,
    /// `)` with no `(`.
    StrayParen,
    /// `t:` — keyword with no value.
    EmptyValue,
    /// `mv>` — operator with no value.
    DanglingOperator,
    /// `-` at the end.
    DanglingNot,
}
```

### Tokens

`lex.rs` produces: `Ident` (**only** ASCII letters, matching
`keyword := [A-Za-z]+`), `Op`, `Word`, `Quoted`, `Regex`, `LParen`, `RParen`,
`Minus`, `Bang`, `Or`, `And` (the last two out of the bare words `or` / `and`,
case irrelevant).

Quirks of the original, written down, each one line in the lexer:

- `=<` is read as `<=`.
- `<>` is **not** an operator: `mv<>3` falls apart into a name word.
- `!` binds exactly one token: `!lightning bolt` is `!lightning` AND the loose
  word `bolt`.
- `'…'` and `"…"` are equivalent; `’` (U+2019) is normalised to `'`, on both
  sides of the comparison.
- `-` negates a term or a parenthesised group.
- On text fields `:` and `=` are the same thing.

### Precedence

```
query    := or_expr
or_expr  := and_expr (("or"|"OR") and_expr)*
and_expr := unary (("and"|"AND")? unary)*
unary    := "-"? atom
atom     := "(" or_expr ")" | "!" name_token | term
term     := keyword op value | bare_word | quoted
```

`t:elf t:goblin or t:creature` is therefore
`(t:elf AND t:goblin) OR t:creature` — two runs at the top level, without a
single paren. That is exactly the drawing of the dialog (§5): runs are
stripes, terms are beads.

### Faults

**The parser never fails.** It always returns a `Query`, plus
`faults: Vec<Fault>`; unreadable bytes become `Item::Rubble` and let every
card through. That is not convenience but the hard boundary condition of the
search field: the query runs live on every keystroke, and Enter is taken — in
the search field it adds the first result
(`crates/baylee-client/src/lobby/systems.rs:325-334`, "the fastest way to type
a deck is name, return, name, return"). A half-typed `t:` that empties the
list breaks that flow.

`Fault` carries the span so the field can underline the place: `queryui` draws
a `DANGER` underline under `&text[fault.span]`, and `Query::faults()` is
public.

API:

```rust
impl Query {
    pub fn parse(text: &str) -> Self;
    pub fn text(&self) -> &str;
    pub fn root(&self) -> &Disjunction;
    pub fn faults(&self) -> &[Fault];
    pub fn is_empty(&self) -> bool;
    /// The term at a path (§5).
    pub fn at(&self, path: &Path) -> Option<&Item>;
    /// Replaces a span and re-parses. The only way to write.
    pub fn splice(&self, span: Span, with: &str) -> String;
}
```

---

## 3. The fields

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name, Type, Oracle, Color, Identity, ManaValue, ManaCost,
    Power, Toughness, PowTou, Loyalty, Devotion,
    Is, Keyword, Subtype, Supertype,
}

impl Field {
    /// The canonical token and every alias, in one table.
    pub fn of(word: &str) -> Option<Self>;
    pub fn token(self) -> &'static str;
    pub fn label(self) -> Phrase;
}
```

### Answerable from the data that is already on the wire today

| Field | Aliases | out of what |
|---|---|---|
| `name` | — (plus `!`) | `name` / `english_name` / `alt_names` (`deckbuilder.rs:70,72,120`), folded with `prose::sort_key` (`crates/baylee-client-core/src/prose.rs:100`) |
| *bare word* (`FieldRef::Bare`) | — | the same three **plus** `type_line` and `oracle_text` — the five haystacks `matches` has today (`builder.rs:305-310`) |
| `type` | `t` | `kinds` (English words, `deckbuilder.rs:85`) **and** the folded `type_line` (`deckbuilder.rs:83`) |
| `oracle` | `o` | `oracle_text` (`deckbuilder.rs:90`) |
| `color` | `c` | `colors` (`deckbuilder.rs:79`) |
| `identity` | `id`, `ci` | `identity` (`deckbuilder.rs:81`) |
| `manavalue` | `mv`, `cmc` | `cmc` (`deckbuilder.rs:77`) |
| `mana` | `m` | `mana_cost` (`deckbuilder.rs:75`) |
| `power` | `pow` | `stats` (`deckbuilder.rs:87`), split at the `/` |
| `toughness` | `tou` | the same, right half |
| `powtou` | `pt` | the sum of both halves |
| `loyalty` | `loy` | `stats` without a `/` (`crates/baylee-cards/src/pool.rs:178-181`: P/T wins, otherwise loyalty) |
| `devotion` | — | purely out of `mana_cost`, no new field |
| `is` | `has`, `not` | `commander` (`:99`), `basic_land` (`:102`), `two_faced` (`:106`), `coverage` (`:93`) |

Three traps that have to be named in the dialog and cannot be programmed
away:

- **`colors` is front-face only.** `crates/baylee-cards/src/pool.rs:110-111`
  takes `def.faces.first()`, `:124` builds `letters(cost.colors())` out of the
  cost alone. `FaceDef::color_indicator`
  (`crates/baylee-cards-dsl/src/lib.rs:228-231`) does not flow into it.
  `identity` is the one colour field that is whole-card (`pool.rs:125`, out of
  `def.color_identity`).
- **`cmc`, `kinds`, `type_line`, `mana_cost`, `stats` are front-face only**
  (`pool.rs:121-128`, all through the same `face`).
- **`type_line` is localised** as soon as a catalog is there:
  `crates/baylee-gateway/src/pool.rs:122` overwrites it with the catalog line.
  `t:` therefore checks three ways — English `kinds`, the new English
  `subtypes` / `supertypes` from §4, and the folded localised line — so that
  `t:kreatur` and `t:creature` both hit.
- **`oracle_text` is empty without a catalog** (`pool.rs:129` writes
  `String::new()`, filled only in `gateway/pool.rs:126-131`, `has_text` there
  too at `:69`) and otherwise stands in the **served** language. `o:` is
  therefore not a field that always answers — it falls out of reach at
  `has_text == false` (§4) and refuses with the existing `Phrase::NoRulesText`
  (`crates/baylee-client-core/src/i18n.rs:765`).

### Derivable — three new wire fields

| Field | Aliases | out of what |
|---|---|---|
| `keyword` | `kw` | `CardDef::all_keywords()` (`crates/baylee-cards-dsl/src/lib.rs:141-147`) through a **new** `KeywordSet::words()`. There is none: `fn words` exists only three times in the workspace, all in `crates/baylee-core/src/types.rs:185,290,460`. 34 bits, `FLYING = 0` (`dsl/lib.rs:342`) through `NIGHTBOUND = 33` (`:375`). |
| `subtype` | — (and `t:goblin`) | `FaceDef::subtypes` (`dsl/lib.rs:194`) through `baylee_core::generated::subtypes::name` (`crates/baylee-core/src/generated/subtypes.rs:1596`) — exactly the call `pool::type_line` already makes (`crates/baylee-cards/src/pool.rs:162-166`). What goes on the wire are the English **words**, never the `SubtypeId`. |
| `supertype` | — (and `is:legendary`, `t:legendary`) | `FaceDef::supertypes` (`dsl/lib.rs:193`) through `SupertypeSet::words` (`crates/baylee-core/src/types.rs:290`) |

### Not answerable — a named refusal

Two sentences, and the difference between them is the answer to whether
retyping helps:

- **`Phrase::QueryNoSuchField`** — "This card collection does not know it".
  The field does not exist here at all:
  `rarity`/`r`, `set`/`s`/`e`/`edition`, `settype`/`st`, `number`/`cn`,
  `block`/`b`, `group`/`g`, `in`, `cube`, `format`/`f`/`legal`, `banned`,
  `restricted`, `edhrecrank`/`edhrec`, `usd`, `eur`, `tix`, `cheapest`,
  `artist`/`a`, `artists`, `illustrations`, `flavor`/`ft`, `watermark`/`wm`,
  `border`, `frame`, `stamp`, `game`, `language`/`lang`, `year`, `date`,
  `layout`, `art`/`atag`/`arttag`, `function`/`otag`/`oracletag`, `prints`,
  `sets`, `new`, `include`, `produces`, `fulloracle`/`fo`.
  `produces:` stands here and not in the second list because **no** field of
  `PoolCard` (`deckbuilder.rs:67-121`) carries produced mana. `fo:` likewise:
  the pool carries exactly one text, and nobody can promise it is the one with
  reminder text.
- **`Phrase::QueryTextOnly`** — "Read as text only".
  The field exists, this form does not: `c:2` (colour count instead of colour
  set), `mv:even` / `mv:odd`, field comparisons (`pow>tou`), `pow=*`, every
  `/…/`, `!` with a pattern, hybrid and Phyrexian symbols in `m:` (`m:{R/G}`),
  `is:split` / `is:mdfc` / `is:transform` / `is:adventure`.

**The honesty rule and what it means here.** Scryfall drops an unknown term
and writes a warning into a JSON nobody reads. We drop it from the evaluation
too — but **by name**: the chip is grey, carries the prohibition sign
`glyph::EXILE` (`crates/baylee-client/src/hud.rs:293`, U+F05E, already in the
font), its editor offers nothing but Remove, and the foot counts it
(`{0} condition is not evaluated` / `{0} conditions are not evaluated`,
through `Phrase::counted`,
`crates/baylee-client-core/src/i18n.rs:1460`). The alternative — a refused
term yields zero hits — is rejected, because the same rule would then apply to
`Rubble` and the list would be empty while `t:` is being typed. Not evaluated
**and** counted is the refusal; letting it through in silence would not be.

---

## 4. The evaluation

### The trait

```rust
/// What a row says about a field.
pub enum Slot<'a> {
    Text(&'a str),
    /// Several strings, each one a haystack (names, type words).
    Texts(&'a [String]),
    Number(i32),
    /// WUBRG as five bits, plus the sixth for colourless.
    Colors(ColorMask),
    /// Mana cost in `{1}{W}` spelling.
    Mana(&'a str),
    Flag(bool),
    /// The card does not have this characteristic — a land has no power.
    /// Never hits, and that is an answer, not a refusal: on Scryfall a
    /// card with no power matches no `pow:` at all.
    None,
}

/// A row a query can run over.
pub trait Queryable {
    /// What this row says about a field.
    ///
    /// Called only for fields that lie in the caller's `Reach`.
    /// A refusal is a property of the table and never of the row:
    /// half a list that answers and half that refuses would be exactly
    /// the filter that lets things through in silence.
    fn slot(&self, field: Field) -> Slot<'_>;
}

/// Which fields a table can answer. Asked once before the run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reach(u32);

impl Reach {
    pub fn has(self, field: Field) -> bool;
    pub fn with(self, field: Field) -> Self;
}
```

### The plan

```rust
/// A query, checked against a reach and ready to run.
pub struct Plan { /* … */ }

/// A term that is not evaluated, and why.
pub struct Refusal { pub span: Span, pub why: Phrase }

impl Query {
    pub fn plan(&self, reach: Reach) -> Plan;
}

impl Plan {
    /// Whether a row survives the whole query.
    pub fn admits<C: Queryable>(&self, row: &C) -> bool;
    /// Whether it survives this one term — the count at the chip (from `Rows`).
    pub fn term_admits<C: Queryable>(&self, at: &Path, row: &C) -> bool;
    /// Whether it would survive the query if this term had this value —
    /// the count at the value (from `Facets`).
    pub fn with_value_admits<C: Queryable>(&self, at: &Path, value: &str, row: &C) -> bool;
    pub fn refusals(&self) -> &[Refusal];
}
```

`admits` walks the tree: disjunction = one of the `runs`, conjunction = all
`items`, `Group` recursively with `negated` as XOR, `Rubble` = `true`, a
refused term = `true` (§3). A term asks `row.slot(field)` exactly once and
compares by kind of field:

- **Bare word** (`FieldRef::Bare`): folded substring over **five** haystacks —
  `name`, `english_name`, every entry in `alt_names`, `type_line`,
  `oracle_text` — so line for line what `matches` does today
  (`builder.rs:305-310`). That is wider than Scryfall, where a bare word hits
  the name only, and it stays that way: otherwise `set_text("instant")` stops
  finding instants (`deckbuilder/tests.rs:70-71`) and the regression search in
  §8 would no longer be green. Whoever wants Scryfall's narrower meaning types
  `name:instant`; whoever wants the type line alone, `t:instant`.
- **Text** (`name`, `type`, `oracle`): folded substring over
  `prose::sort_key` (`prose.rs:100`), folded on both sides — a folded needle
  against a merely lower-cased haystack stops matching at exactly the accents
  the folding exists for (`builder.rs:300-310` says so today already).
  `!` / `Exact` compares the whole folded string.
- **Colour set** (`color`, `identity`): `:` is superset on `color` and subset
  on `identity` — the colon's change of direction. `=` is equality, `<` / `>`
  proper subset and superset, `<=` / `>=` the same with equality, `!=`
  inequality. Today's `color_match` (`builder.rs:318-320`) **is** `id:` with
  `<=`; nothing says so today, and in the dialog it becomes the label.
- **Number** (`manavalue`, `power`, `toughness`, `powtou`, `loyalty`): `:` is
  `=`. `Slot::None` never hits.
- **Mana cost** (`mana`): multiset containment, generic numeric, colour
  symbols counted, split symbols atomic.
- **Word lists** (`keyword`, `subtype`, `supertype`, and `type` through
  `kinds`): word equality, not substring.
- **Flag** (`is`): `commander`, `basic_land`, `two_faced` → "double-sided",
  plus the three of our own `is:playable` / `is:partial` / `is:implemented`
  out of `coverage` (`deckbuilder.rs:93`). Those three last are **typable and
  not in the editor** — see §5.

### The two implementations

**`deckbuilder::PoolCard`** (`crates/baylee-client-core/src/deckbuilder.rs:67`)
answers all sixteen fields. Its `Reach` is computed once in `set_pool`
(`crates/baylee-client-core/src/deckbuilder/builder.rs:153-164`), beside the
`keys`:

- `Oracle` only if `has_text` (`builder.rs:31`, set from `gateway/pool.rs:69`);
- `Keyword`, `Subtype`, `Supertype` only if **some** row of the pool carries a
  non-empty such field. That is the one place where an old gateway decides the
  difference between "no card has flying" and "this wire carries no keywords"
  — and reading an empty column as data would mean answering `kw:flying` with
  0 instead of refusing.

**`browser::BrowseRow`** (`crates/baylee-client-core/src/browser.rs:126`) —
the row of the zone browser. Its `Reach` is small and honest: `Name` from
`row.name` (`browser.rs:138`), `ManaValue` from `row.mana_value`
(`browser.rs:152`), `Type` / `Supertype` from `row.types: TypeSet`
(`browser.rs:154`) through `TypeSet::words`
(`crates/baylee-core/src/types.rs:185`), `Is` for `is:token` from `row.token`
(`browser.rs:161`). Everything else refuses with `QueryNoSuchField`. That is
exactly what the trait is for: the same language over the client's second card
list, with a different reach and the same named refusals.

### Which fields `PoolCard` gains — both copies

**Server side, `crates/baylee-cards/src/pool.rs`** (struct up to `:93`,
written in `row`, `:107-141`), three fields with
`#[serde(skip_serializing_if = "Vec::is_empty")]` like `alt_names` (`:92`):

```rust
pub subtypes: Vec<&'static str>,    // face.subtypes through subtypes::name, as at :162-166
pub supertypes: Vec<&'static str>,  // face.supertypes.words()
pub keywords: Vec<&'static str>,    // def.all_keywords().words()  ← new in baylee-cards-dsl
```

**Client side, `crates/baylee-client-core/src/deckbuilder.rs`** (struct
`:67-121`), the same three as `Vec<String>` with `#[serde(default)]` like
`alt_names` (`:117-120`).

Plus **one** new function in `crates/baylee-cards-dsl/src/lib.rs`:
`KeywordSet::words(self) -> impl Iterator<Item = &'static str>`, built out of
the same `keywords!` macro (`dsl/lib.rs:333-339`) that generates the
constants, so that bit and word cannot drift apart.

---

## 5. The dialog

### The model, in `baylee-client-core`

```rust
/// The way to an item: run, then item, then onwards into groups.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Path(Vec<u16>);

/// What is open on the dialog. No renderer, so testable in client-core.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueryPane {
    /// Which chip has its editor open.
    editing: Option<Path>,
    /// Whether the field menu is open, and where a new term would go.
    adding: Option<Path>,
    /// The pages the phone has opened (a group from depth 2 on).
    pages: Vec<Path>,
}
```

`DeckBuilder` (`deckbuilder.rs:606-662`) gains
`query_pane: Option<QueryPane>` beside `picker: Option<Picker>` (`:662`) — the
same pattern, so that the same tidy-up rule holds: nothing despawns the dialog
by hand, at the next rebuild it is simply no longer there.

```rust
impl DeckBuilder {
    pub fn query(&self) -> &Query;
    pub fn query_pane(&self) -> Option<&QueryPane>;
    pub fn open_query(&mut self);
    pub fn close_query(&mut self);

    /// Opens or closes a chip's editor.
    pub fn edit_term(&mut self, at: &Path);
    pub fn set_term_field(&mut self, at: &Path, field: Field);
    pub fn set_term_op(&mut self, at: &Path, op: Op);
    pub fn set_term_value(&mut self, at: &Path, value: &str);
    pub fn toggle_term_value(&mut self, at: &Path, value: &str);   // multiple choice
    pub fn toggle_term_negation(&mut self, at: &Path);
    pub fn remove_term(&mut self, at: &Path);
    pub fn add_term(&mut self, at: &Path, field: Field);

    /// The `·` target between two chips: splits the run in two with `or`.
    pub fn split_run(&mut self, at: &Path);
    /// The `or` chip: merges two runs back into one.
    pub fn merge_runs(&mut self, join: usize);

    /// The count at the chip: how many cards this term alone lets through.
    pub fn term_tally(&self, at: &Path) -> usize;
    /// The count at the value: how many there would be if it were ticked now.
    pub fn value_tally(&self, at: &Path, value: &str) -> usize;
}
```

**Every** one of those writers ends in the same line:
`let next = self.query.splice(span, &canonical); self.set_text(&next);` —
and `set_text` (`builder.rs:177-180`) calls `refilter` (`builder.rs:248`). One
way to write, one thing that computes.

The three quick filters of today become writers into that same sentence and
keep no field of their own: `toggle_color` (`builder.rs:195`) flips a letter
in the `id:` term, `set_kind` (`builder.rs:205`) sets the `t:` term, `set_cmc`
(`builder.rs:212`) writes **two** terms, `mv=N -t:land`, because the curve bar
today also throws out every land (`builder.rs:289-292`). Its switching itself
off (`builder.rs:213`) becomes "are these two terms already in the tree, then
out". And because the player may also have typed `mv=3` themselves: the bar
reads its "on" from the presence of an `mv=N` term alone, and the writer
appends only the **missing** half — otherwise the box says
`mv=3 mv=3 -t:land` after one click. `playable_only` stays **outside** the
query: it is on by default (`builder.rs:16`), is not in `filtered()`
(`builder.rs:240-245`) and is not touched by `clear_filters`
(`builder.rs:230-236`) — an honesty policy of the builder, not a narrowing by
the player.

From that follows the one omission in the `is:` editor: it offers exactly
`Commander`, `Basic land` and `double-sided` and **none** of the three
coverage flags. Two controls on the same `coverage` field
(`deckbuilder.rs:93`) — the foot switch "Playable only" and a checkbox beside
it — would be two switches that overrule each other. The switch stays;
`is:partial` stays typable and is drawn correctly as a bead.

`filtered()` becomes `!self.query.is_empty()`.

### Drawing, in `baylee-client`

`crates/baylee-client/src/queryui.rs`, called from `buildui::builder`
(`buildui.rs:64`) as the last child, behind the printing picker
(`buildui.rs:149-153`).

**Desktop (≥ 1180 px, `Frame::of`, `crates/baylee-client/src/lobby/ui.rs:52`):**
no full-screen scrim. A column docked on the right, `PositionType::Absolute`,
`ZIndex(20)` like the picker (`buildui.rs:612`) — the results list on the left
stays visible and re-orders itself at every chip. On the left inside the panel
is the permanent field column (the graft from *Facets*): without a single tap
the player sees which sixteen questions the dialog answers and which it does
not.

```
┌ ‹ Decks   Editing "Goblins"                                     [Save] ┐
├─────────────────────────────────┬──────────────────────────────────────┤
│ SEARCH                          │┌ Build a search ────────────────  × ┐│
│ ┌───────────────────────────⚙┐  ││ ┌──────────────────────────────┐   ││
│ │ t:goblin c:r mv<=2 -o:hast │  ││ │ t:goblin c:r mv<=2 -o:haste| │   ││
│ └────────────────────────────┘  ││ └──────────────────────────────┘   ││
│ 184 of 1365 cards               ││ ╭──────────────────────────────╮   ││
│ ┌────────────────────────────┐  ││ │[Type·Goblin 61][Color·●R 402]│   ││
│ │ Goblin Guide         {R}   │  ││ │[MV ≤2 489] · [not Text 12]   │   ││
│ │ Goblin Lackey        {R}   │  ││ ╰──────────────────────────────╯   ││
│ │ Goblin Bushwhacker   {R}   │  ││ [+ or]                             ││
│ │ …                          │  │├─────────────┬──────────────────────┤│
│ │                            │  ││ Name        │ MANA VALUE           ││
│ │                            │  ││ Type        │ [=][≠][<][≤●][>][≥]  ││
│ │                            │  ││ Subtype     │ [−]    2    [+]      ││
│ │                            │  ││ Color       │ 0:80 1:143 2:489 …   ││
│ │                            │  ││▌Mana value  │ [even] [odd]         ││
│ │                            │  ││ Mana cost   │ against: [Pow] [Tou] ││
│ │                            │  ││ Power       │ [not]       [Remove] ││
│ │                            │  ││ …           │                      ││
│ │                            │  ││ ─────────   │                      ││
│ │                            │  ││ Not here:   │                      ││
│ │                            │  ││ r set a usd │                      ││
│ │                            │  ││ f lang ft   │                      ││
│ │                            │  │├─────────────┴──────────────────────┤│
│ │                            │  ││ 184 of 1365 · 1 not evaluated      ││
│ │                            │  ││                       [Done]       ││
│ └────────────────────────────┘  │└────────────────────────────────────┘│
└─────────────────────────────────┴──────────────────────────────────────┘
```

The count at the chip (`term_tally`) says what this term lets through
**alone** — it shows which of the four conditions emptied the list. The count
at the value (`value_tally`) says what it **would be** if it were ticked now.
Both are possible because the whole pool lies in the client and is already
filtered per keystroke today (`builder.rs:248`); the computing happens only
for the **one** open value list.

The fields that cannot be answered stand at the bottom of the column as grey,
unpressable mono tokens (`r set a usd f lang ft …`) under a single footnote.
Tokens on purpose and not translated words: whoever types them knows them that
way, and it saves about thirty phrases.

**Phone (< 760 px; `Metrics::tap` = 48, `ui.rs:89`):** full-screen sheet, one
column, the field column becomes an opened page (push/pop).

```
┌ Build a search                × ┐  Head, 48 tall
│ ┌─────────────────────────────┐ │
│ │ t:goblin c:r mv<=2 -o:hast| │ │  real search box with caret
│ └─────────────────────────────┘ │
│ ╭─────────────────────────────╮ │
│ │ [Type · Goblin         61]  │ │  string of beads = one AND run
│ │ [Color · ●R           402]  │ │  each bead 48 tall (bead, not chip)
│ │ [MV ≤ 2               489]  │ │
│ │ [⊘ r:mythic             –]  │ │  grey, prohibition sign, no count
│ │ [not Text · haste      12]  │ │
│ │                       [+]   │ │
│ ╰─────────────────────────────╯ │
│ [+ or]                          │
├─────────────────────────────────┤  ← the body scrolls from here down
│ MANA VALUE                      │
│ [=] [≠] [<] [≤●] [>] [≥]        │
│ [−]         2         [+]       │
│ 0:80  1:143  2:489  3:…         │
│ [even] [odd]                    │
│ [not]               [Remove]    │
├─────────────────────────────────┤
│ 184 of 1365 · 1 not evaluated   │
│                        [Done]   │  Foot, sticks
└─────────────────────────────────┘
```

Two drawing rules that break in silence otherwise:

- The beads are **not** built with `chip` (`ui.rs:1376`): its `min_height` is
  `metrics.tap * 0.8` (`ui.rs:1397`), so 38 px on the phone — below the 44
  that the same file names as the lower bound. A `bead` variant of the same
  code with `min_height: px(metrics.tap)`. That holds for the `·` target and
  the `or` chip too, otherwise splitting a run is a mis-tap.
- The body is a `scroller` (`ui.rs:1351`), so `List`
  (`crates/baylee-client/src/lobby/systems.rs:808-815`) needs a fourth variant
  `Query` and `Scrolled` (`systems.rs:825-830`) a fourth field — otherwise the
  dialog jumps back to the top at every chip.

"Done", `×` and (on a tablet) the scrim all do the same thing: close. There is
no "Apply": every change is already in the search field when it is pressed,
and the list behind it filters live.

---

## 6. The gear

### Where it goes

**Yes, two fields, both filter cards:**

1. The deckbuilder's pool search — `crates/baylee-client/src/buildui.rs:265`,
   `text_field(..., &FieldLook { buffer, focused, mask: None, press: Press::FocusBuild(BuildField::Search) })`
   (`buildui.rs:265-276`). Step 6 in §9.
2. The zone browser's filter — `crates/baylee-client/src/hud/tray.rs:1074`.
   That is **not** a `text_field`: it is `TrayFilter + Button` with runs of
   its own, and the click routing is
   `find_in_lineage(entity, &tray.filter, …)` in
   `crates/baylee-client/src/input.rs:2385` — there is no `Press` there at
   all. So a second implementation, not the same one. Step 10 in §9.

**No, with a reason:**

- E-mail (`crates/baylee-client/src/lobby/ui.rs:402-405`), display name
  (`:416-419`), account password (`:430-436`), room password (`:875-883`) —
  credentials, they filter nothing.
- Table search (`ui.rs:820-822`) — looks like a search but filters **tables**:
  it goes as a `GameQuery` to `GET /lobby/games`, never against a card.
- Deck name (`buildui.rs:1183-1190`) — it names the deck.
- The drawer's creature-type filter
  (`crates/baylee-client/src/hud/ledge/drawer.rs:655`) — it filters about 350
  subtype **names**, not cards.

### How it hangs in `text_field`

`FieldLook` (`crates/baylee-client/src/lobby/ui.rs:1518-1533`) gets a third
optional field beside `mask` (`:1530`) and `press` (`:1532`):

```rust
/// What a gear on the right edge means, when one hangs there.
pub(crate) gear: Option<Press>,
```

In `text_field` (`ui.rs:1611`) the branch at `:1663` becomes a branch for
both: the `flex_grow: 1.0` spacer (`ui.rs:1666-1676`) is spawned **once**, as
soon as `mask` **or** `gear` is set, then the gear, then the eye — a box with
both keeps the eye outermost. `gear_button` is `eye_button`
(`ui.rs:1705-1747`) line for line: a `Text` with `crate::hud::icon_tf`,
`Pickable::IGNORE`, wrapped in a node with
`min_width: px(metrics.tap * 0.7)` (`ui.rs:1735`), `align_self: Stretch`, and
the `Press` on the **wrapper** (`ui.rs:1741`) — so that the ancestor walk
`in_lineage` (`systems.rs:1188`) finds the gear before `Press::FocusBuild` and
the tap does not fall through.

New variant: `Press::OpenQuery` beside `Press::FocusBuild(BuildField)`
(`systems.rs:1126`), with no payload — in the builder there is exactly one
such box. An arm in `clicks` (`systems.rs:454`) beside `Press::PickerClose`
(`systems.rs:747`): `state.lobby.builder_mut().open_query()`.

**The glyph is missing.** `hud::glyph`
(`crates/baylee-client/src/hud.rs:283-345`) has sixteen constants and no
slider or filter sign. `SLIDERS` (U+F1DE) is read out of the shipped font's
cmap, the way `CHECK` was (`hud.rs:321-324`: "Read out of the shipped font's
own cmap rather than looked up: a codepoint a search agrees about is not the
same claim as a glyph this file has"). Until that is proven, the word
`Phrase::ShowFilters` stands there ("Filters" / "Filter",
`crates/baylee-client-core/src/i18n.rs:718`), which does exist.

### Keyboard

**Native.** The build branch of `keyboard` (`systems.rs:310-345`) matches
exactly `Key::Backspace`, `Key::Tab`, `Key::Enter` and `_` today — **no
Escape**. Two arms are added:

- `Key::Escape`: two-stage — first `QueryPane::editing` to `None`, the second
  time `close_query()`. The same arm incidentally repairs the printing picker,
  which today closes only through `Press::PickerClose` (`systems.rs:747`).
- The chord that **opens** the dialog. It fires only while
  `builder.focus() == BuildField::Search`. The build branch today reads
  `key.logical_key` and no modifiers at all; the precedent for a modifier
  comparison in the same file is `text_field_keys` (`systems.rs:376`), the
  tool is `modifiers_match` (`crates/baylee-client/src/keys.rs:127`), which
  compares **exactly** — Shift+X is never also X. Proposed: `Ctrl`/`Cmd` +
  `K`. **Step 0 before the commit: check the keymap against this chord**
  (`Fired::of`, `keys.rs:191`), because a collision here is exactly the fault
  the developer's desktop never shows.

**wasm.** `keyboard` bails out **completely** (`systems.rs:306-308`), not only
on field focus. So there is no chord — the gear is a tap there and nothing
else. The keyboard arrives solely through the one `<input>` and its `keydown`
listener (`crates/baylee-client/src/softkeys.rs:251`), which knows two keys.
So a new rule holds in the build branch of `softkeys` (`systems.rs:180-206`):
today it says `SoftKey::Submit | SoftKey::Dismiss => keys.close()`
(`systems.rs:204`), and `close()` **clears and blurs** the element
(`softkeys.rs:302-314`) — a dialog that allows that has no keyboard at all in
the browser. While `query_pane` is open:

- `SoftKey::Dismiss` closes the editor first, then the dialog, and calls
  `keys.close()` only once both are shut.
- `SoftKey::Submit` means "editor closed" here and **not** "add the first
  result" — that is Enter in the search field (`systems.rs:325-334`).
- When a value editor with text of its own is opened, the element is re-hung
  with `keys.open(kind, value)` (`softkeys.rs:262`), behind an epoch of its
  own like `build_epoch` (`systems.rs:176-186`).
- **The `Caret` arm stops being empty.** Today it says
  `SoftKey::Caret { .. } => {}` (`systems.rs:201`), and the comment above it
  (`systems.rs:197-200`) justifies that with exactly "The builder's boxes are
  strings and the browser draws no caret in them". After step 4 that is no
  longer true: the arm writes `cursor` / `anchor` back into the `TextBuffer`
  (`textbuf.rs:170` `place`), otherwise the caret in the browser is always at
  the end and the editor selects a span nobody sees. The other direction —
  pushing a selection from the model **to** the `<input>` — does not exist:
  `softkeys.rs:262-300` knows no `setSelectionRange`. See §10.

**Phone.** The gear is `tap * 0.7` like the eye (`ui.rs:1735`) — 33.6 px wide
at `tap` = 48 (`ui.rs:89`), over the full height of the box, so a target and
not a dot. It does not replace the `ToggleFilters` chip: that stays where it
is (`buildui.rs:282-326`). The dialog there is the full-screen sheet from §5.

---

## 7. The round trip

### Text → dialog

Every keystroke parses anew: `set_text` (`builder.rs:177`) calls
`Query::parse` and puts the result beside the text. The dialog reads **only**
that tree; it has no second state. Opening therefore means nothing more than
`query_pane = Some(default)`.

One precondition, and it is not a bonus: `DeckBuilder.text`
(`deckbuilder.rs:619`) is changed from `String` to `TextBuffer`
(`crates/baylee-client-core/src/textbuf.rs:74`). Today `buildui.rs:264` builds
a throwaway buffer on **every** rebuild, and the comment above it
(`buildui.rs:259-262`) says why itself: "The builder's boxes are plain
strings, so the caret is always after what is in them". Without a caret the
editor cannot select the value span. The precedent stands in the same crate:
`Browser::filter` has long been a `TextBuffer`
(`crates/baylee-client-core/src/browser.rs:689`), and the comment there names
exactly this reason.

### Dialog → text

**There is no printer.** Every control is `query.splice(span, &canonical)` and
replaces exactly one span:

- Change a value → `value_span`.
- Change an operator → the operator's span.
- Toggle negation → put a `-` in front of `term.span` or strike it out.
- Remove a term → `term.span` plus **one** separator.
- Append a term → at the end of the `Conjunction::span` (before the closing
  paren, if it is a group).
- `·` → the separator between two items becomes ` or `; the `or` chip → it
  becomes a single space.

**Only the term that was touched** is written canonically. Three things follow
from that, and they are tests (§8): `c:gruul` stays `c:gruul` until somebody
touches that bead; `mv=<3` is not turned into `mv<=3` unasked;
`type:creature` never becomes `t:creature`.

### What cannot be drawn

Four classes, each with a bead of its own and a sentence of its own:

1. **The field does not exist here** (`r:mythic`, `set:lea`, `usd<1`,
   `artist:`, `xyzzy:3`): grey bead, `glyph::EXILE` (`hud.rs:293`), the
   sentence `Phrase::QueryNoSuchField` — "This card collection does not know
   it". The editor offers nothing but Remove. Not evaluated, counted in the
   foot.
2. **The field exists, this form does not** (`c:2`, `mv:even`, `pow>tou`,
   `m:{R/G}`, `o:/…/`, `!/…/`, `is:split`): the same grey bead, the sentence
   `Phrase::QueryTextOnly` — "Read as text only". The difference from (1) is
   the one piece of information the player needs: whether retyping helps.
3. **`o:` without a catalog**: the existing `Phrase::NoRulesText`
   (`i18n.rs:765`). No new sentence, because it is not a new case.
4. **Incomplete** (an open quote, an open paren, `t:` while typing): a dashed
   bead, `Phrase::QueryIncomplete`, no editor but Remove, **lets every card
   through** and carries no count. On top of that `queryui` underlines
   `&text[fault.span]` in the text line.

In all four cases the bytes stay standing in the text, and the chip controls
on the parsed prefix keep working, because they splice spans and do not touch
the rest.

**One half of the round trip is missing in the browser.** A chip's text editor
natively selects the `value_span` in the text line (`TextBuffer::place`,
`textbuf.rs:170`) and replaces the selection while typing. On wasm the focus
lies in the one `<input>` (`softkeys.rs:203-224`), and `open`
(`softkeys.rs:262-300`) sets type, `inputmode`, value and focus — but no
selection; there is no `setSelectionRange` there. In the browser the value
editor therefore types **at the caret**, not into the selection, and the chip
writes its value through `set_term_value` instead of through the box. That is
v1 (§10), not a gap that may come as a surprise during the building.

---

## 8. The tests

Every test stands in `crates/baylee-client-core/src/deckbuilder/tests.rs`
(972 lines today) unless said otherwise. For each of them, the **one line**
whose deletion drops it is named.

**Lexer and precedence** — `or_binds_looser_than_juxtaposition`:
`Query::parse("t:elf t:goblin or t:creature")` has two `runs`, the first of
them two `items`. *Killing line:* the `Tok::Or` arm in `parse::disjunction`
that begins a new run — without it `or` is a name word and there is one run
with four items.

**Quirks** — `the_operator_quirks_are_the_ones_scryfall_has`: `mv=<3` parses
to `Op::Le`, `mv<>3` to a `FieldRef::Bare`, `!lightning bolt` to two items.
*Killing line:* the `'='` case in the lexer that looks ahead for `'<'`.

**Spans** — `every_span_reparses_to_the_term_it_came_from`: for every term of
a collection of queries, `Query::parse(&text[term.span.clone()])` is exactly
that one term. *Killing line:* the `+ 1` in `lex::quoted` that includes the
closing quote — without it the span ends before it and the re-parse leaves an
`UnclosedQuote` behind.

**Faults with a span** — `an_unclosed_quote_names_its_own_bytes`:
`Query::parse("o:\"draw a").faults()` is one, `kind == UnclosedQuote`, and
`&text[fault.span]` begins with `"`. *Killing line:* the `faults.push(...)` in
the quote branch.

**The field count, bounded on both sides** —
`every_field_is_either_answered_or_refused_by_name`: a `const` in the test
lists every canonical field and every alias the original has; for each of them
the classification must be either `Field::of(..).is_some()` or an entry in one
of the two refusal lists — **and** `answered.len() >= 16` and
`refused.len() >= 38`. The floors are the point: a spelling that recognises
nothing any more would otherwise report a smaller population as a success.
That is the lesson from `cross-read` (CLAUDE.md: "a reader that reports a
population is worth more than one that is merely correct today"). *Killing
line:* any alias line at all in `Field::of` — the alias falls to `Unknown`,
`answered` shrinks, the floor breaks.

**Honesty** — `a_refused_term_narrows_nothing_and_says_so`:
`plan("t:creature r:mythic").admits(..)` yields exactly the same thing for
every card as `plan("t:creature")`, **and** `refusals()` has length 1 with
`Phrase::QueryNoSuchField`. *Killing line:* the `refusals.push(..)` in
`Query::plan` — the first half would stay green, the second falls. That is
exactly why it is two assertions in one test.

**An empty column ≠ the state of the data** —
`an_empty_keyword_column_refuses_instead_of_answering_none`: a pool whose
cards all carry `keywords: vec![]` makes `kw:flying` a refusal and not zero
hits. *Killing line:* the `cards.iter().any(|c| !c.keywords.is_empty())` in
the `Reach` computation in `set_pool`.

**No characteristic ≠ a refusal** — `a_land_never_matches_any_power_term`: a
land matches neither `pow=0` nor `pow>=0` nor `pow<9`, and `refusals()` is
empty. *Killing line:* the arm `Slot::None => false` in the numeric
comparison.

**The second user** — in
`crates/baylee-client-core/src/browser/tests/filtering.rs`:
`the_browser_answers_four_fields_and_refuses_the_rest` — `mv<=2` filters rows,
`o:draw` refuses with `QueryNoSuchField`. *Killing line:* the
`Field::ManaValue` entry in `BrowseRow`'s `Reach`.

**Regression over today's behaviour** —
`every_recorded_search_gives_the_results_it_gave_before`: the search strings
`tests.rs` uses today yield byte-identical `results()`.
`a_stub_is_hidden_until_it_is_asked_for` (`tests.rs:49`) stays **unchanged**,
because `playable_only` keeps a field of its own. *Killing line:* the
`folded(&card.type_line)` line in the name comparison — the bare word hits the
type line today too (`builder.rs:309`), and whoever loses that in the rebuild
loses it here.

### The counter-check to "the round trip loses nothing"

The claim is: a control in the dialog leaves alone every byte that does not
belong to the term that was touched. It is **trivially true** if `splice` does
nothing at all. Hence two tests that only together prove something:

1. `touching_one_term_leaves_every_other_byte_alone` — out of
   `t:goblin c:gruul mv=<2 -o:"haste"`, `set_term_value` on the first term
   makes `t:elf c:gruul mv=<2 -o:"haste"`. What is checked is not only the
   whole string but **every untouched span individually**: `c:gruul` has not
   become `c:rg`, `mv=<2` has not become `mv<=2`.
2. `the_touched_term_really_changed` — the same call, and `&next[term.span]`
   is **not** `&before[term.span]`.

A `splice` that returns its input unchanged leaves (1) green and drops (2). A
`splice` that re-sets the whole query leaves (2) green and drops (1). That is
the injected counter-check that makes a zero report worth quoting in the first
place.

On top of that the third, coarser safeguard: `parse_splice_parse_is_a_fixpoint`
— for a collection of queries, `parse(query.splice(span, &text[span]))` equals
`parse(query)`. A splice with itself must do nothing.

---

## 9. The order

Every step is one commit, compiles on its own, and proves something.

**0. The two assumptions become tests.** A case in `hud::tests` that looks
U+F1DE up in the shipped font's cmap — that is exactly where the icon font is
read (`crates/baylee-client/src/hud.rs:300`, "Used by `hud::tests`, which is
where the icon face is read"), and that is exactly how `CHECK` (`:324`) was
evidenced. Plus a case beside `keys.rs` asserting that no chord of the shipped
keymap binds to `Ctrl`/`Cmd`+`K` (`Fired::of`, `keys.rs:191`;
`modifiers_match`, `keys.rs:127`). *Proof:* both tests fall if the glyph is
missing or the chord is taken — instead of it becoming a silent assumption in
step 6.

**1. The three wire fields.** `KeywordSet::words()` in
`crates/baylee-cards-dsl/src/lib.rs` out of the same `keywords!` macro
(`dsl/lib.rs:333-339`); `subtypes`, `supertypes`, `keywords` on
`baylee_cards::pool::PoolCard` (`crates/baylee-cards/src/pool.rs`, struct and
`row` from `:107` on) and on `deckbuilder::PoolCard` (`deckbuilder.rs:67-121`).
*Proof:* every card whose `all_keywords()` carries `FLYING` has "flying" in
the row; a card's `subtypes` list is exactly what `type_line` writes after the
em dash (`pool.rs:162-166`).

**2. Lexer, AST, spans.** No evaluation. *Proof:* the precedence, quirk, span
and fault tests from §8.

**3. `Queryable`, `Reach`, `Plan`.** The implementation for the client's
`PoolCard`. *Proof:* the field count with both floors, the honesty test, the
empty-column test, `Slot::None`.

**4. `DeckBuilder.text: String → TextBuffer`.** `set_text`, `type_focused`,
`backspace_focused`, `set_focused` go through it; `buildui.rs:264` stops
building a throwaway buffer per rebuild. *Proof:* caret and selection survive
a rebuild; arrow keys move it.

**5. `matches` becomes `plan.admits`.** `colors` / `kind` / `cmc` fall out of
`DeckBuilder` and become writers into the text; `filtered()` becomes
`!query.is_empty()`; `set_cmc` writes `mv=N -t:land`. *Proof:* the regression
search from §8, byte-identical `results()`; `tests.rs:49` untouched.

**6. The gear.** `FieldLook.gear`, `gear_button`, `Press::OpenQuery`, the
glyph. *Proof:* a client test that the wrapper carries the `Press` and its
`Text` is `Pickable::IGNORE` — that is, that the ancestor walk
(`systems.rs:1188`) finds it before `Press::FocusBuild`.

**7. `QueryPane` and the splice methods.** No drawing. *Proof:* the two
counter-check tests from §8 and the fixpoint.

**8. `queryui.rs`, first half.** The chain, one run, no `or`, five editors:
name, type, colour identity, mana value, rules text. `List::Query` and the
fourth field in `Scrolled`. *Proof:* a client test plus a screenshot through
`dev-control`.

**9. `queryui.rs`, second half.** `or` runs, the `·` target, negation, groups,
the remaining editors (colour, mana cost, power/toughness/loyalty, `is:`,
`kw:`, subtype), the count per value, the phone pages from depth 2 on.

**10. The zone browser.** A second gear at `tray.rs:1074`, an arm in
`input.rs:2385`, `Queryable` for `BrowseRow` with its small `Reach`. *Proof:*
the browser test from §8.

After step 5 the rebuild is finished and **nothing has changed for the
player** — that is the point at which the regression search counts. From step
6 on, surface is added.

---

## 10. What is not built

**No regex.** `crates/baylee-client-core/Cargo.toml:13-19` has six
dependencies, none of them a regex machine; adding one puts it into every wasm
build. The original's five regex fields (`t:`, `o:`, `fo:`, `ft:`, `name:`)
stay typable, `/…/` becomes a named refusal ("Read as text only"). If somebody
asks for it, it is one line in the manifest and one arm in the evaluator — the
parser already knows `Value::Regex`.

**No server-side query.** `baylee_cards::pool::PoolCard` gets the three fields
and **no** trait implementation, so that `baylee-cards` does not link the
language and no engine build grows a text parser. The whole pool goes over the
wire in one piece anyway.

**No parameterised keywords.** `kw:equip`, `kw:cycling`, `kw:ward`,
`kw:kicker` refuse. `KeywordSet`'s own documentation says so
(`crates/baylee-cards-dsl/src/lib.rs:325-329`): parameterised keywords are
`AbilityDef` data, not bits. The rubric lists exactly the 34 bits
(`dsl/lib.rs:342-375`) and promises nothing beyond them. A heuristic in the
pool builder guessing "this ability is cycling" would be the opposite of the
honesty rule.

**No `is:split` / `is:mdfc` / `is:transform` / `is:adventure`.** What stands
on the wire is a single bit, `two_faced` (`deckbuilder.rs:106`, out of
`faces.len() > 1`, `crates/baylee-cards/src/pool.rs:136`). It cannot tell the
four apart. What is offered is `is:doublesided` ("double-sided"); the other
four refuse with "Read as text only".

**No `produces:`.** Neither `PoolCard` copy carries produced mana. A
derivation from the abilities would be incomplete by construction, and a wrong
yes is worse than a named refusal.

**Back faces are not repaired.** `cmc`, `colors`, `kinds`, `type_line`,
`mana_cost`, `stats` all come out of `def.faces.first()`
(`crates/baylee-cards/src/pool.rs:110-111`, read at `:121-128`). Fire // Ice
answers `mv=2` instead of 4, and `color_indicator`
(`crates/baylee-cards-dsl/src/lib.rs:228-231`) flows in nowhere, so a
transformed back face is colourless. The dialog then draws correct chips over
wrong answers. The repair belongs in `pool::row` and is a project of its own;
here it is **named and not hidden**, and the dialog makes it more visible than
it is today.

**No `unique:` / `order:` / `sort:` / `direction:` / `prefer:` /
`display:` / `include:extras`.** Those are not filters. The builder has a
sorting of its own (`cycle_sort`, `builder.rs:225`), and "extras" do not exist
in a pool of 1365 implemented cards.

**No value selection in the browser.** Natively the value editor selects the
`value_span` in the text line and replaces the selection while typing
(`textbuf.rs:170`). On wasm the focus lies in the one `<input>`
(`softkeys.rs:203-224`), and `open` (`softkeys.rs:262-300`) can give it type,
value and focus, but no selection — `setSelectionRange` is not called there.
In the browser the editor types at the caret, and the chip writes its value
through `set_term_value`. That is one line in `softkeys.rs` if somebody wants
it; it is not in v1 so that it does not come as a surprise in step 7.

**No tree depth > 2 on the phone in one view.** From the second group on it
collapses to `(…)` and opens as a page of its own. That is the one place where
the tree comes back, and it is rare enough to pay for rather than to compute
away.

**No second state beside the text.** No "applied" query against an "edited"
one, no apply button, no saving of queries. The text in the search field is
all there is — and what the player typed stays what they typed.
