//! card-script reference card script → `CardDef` abilities.
//!
//! A card script is a line-oriented rules encoding:
//!
//! ```text
//! Name:Lightning Bolt
//! ManaCost:R
//! Types:Instant
//! A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ …
//! ```
//!
//! Names, costs, types and P/T are not read here — those come from Scryfall,
//! which is the identity this project already keys on. What this module reads
//! is the rules half: `K:` keywords, `A:`/`T:`/`S:`/`R:` abilities, and the
//! `SVar:` sub-abilities they chain into.
//!
//! # Refusal is the feature
//!
//! [`transcode`] returns `Some` only when **every line, every effect in every
//! `SubAbility$` chain, and every parameter key** was consumed. An unknown
//! API, an unknown parameter, a computed `SVar` — any one of them and the
//! card is refused and stays a stub. That rule is what makes a generated
//! `Coverage::Implemented` mean the same thing a hand-written one does: an
//! ability quietly dropped because its `NoRegen$ True` was ignored would be
//! worse than no card at all, because the deckbuilder would offer it.
//!
//! That key is the worked example of why *consuming* a parameter is not the
//! same as reading one. It was consumed as vacuous for as long as this
//! engine had no regeneration — true then, and a card the reader would have
//! written wrongly the moment a shield existed. It is read now.
//!
//! The corpus is read as an automated lookup only; no file of it is copied into
//! this repository.

use crate::body::CardBody;
use crate::catalog::SubtypeCatalogs;
use crate::tokengen::TokenLookup;
use std::collections::BTreeMap;

mod activated;
mod atoms;
mod chains;
mod constants;
mod effects;
mod keywords;
mod mana;
mod statics;
mod support;
mod targets;
mod triggers;

use atoms::{
    Bound, amount, color_atom, count_bound, hybrid_pair, keyword_atom, mana_color_const,
    named_atom, plain_number, pump_amount, stat_atom,
};
pub use atoms::{atoms, token_stems};
use constants::{
    card_type_const, color_const, color_word, cost_parts, counter_kind, keyword_const,
    keyword_enter_modifier, keyword_static, on_the_battlefield,
};
pub use constants::{keyword_const_of, keyword_enter_modifier_of, keyword_static_of};
use support::{CLUE_TOKEN_SCRIPT, asks_for_an_object, generic_mana, object_cost, printed_mana};
pub use support::{SUPPORTED_APIS, apis_used, is_supported_api, is_vanilla, refusal_reason};

/// One parsed card script.
#[derive(Debug, Default)]
pub struct CardScript {
    /// `K:` lines, verbatim.
    pub keywords: Vec<String>,
    /// Rules lines as `(kind, body)` — kind is `A`, `T`, `S` or `R`.
    pub rules: Vec<(char, String)>,
    /// `SVar:<name>:<body>` definitions.
    pub svars: BTreeMap<String, String>,
    /// A line whose prefix this module does not model at all.
    pub unknown_lines: Vec<String>,
}

/// Line prefixes whose content this project takes from Scryfall instead, or
/// that are the corpus's own deckbuilding/AI hints and carry no rules.
const IGNORED_PREFIXES: &[&str] = &[
    "Name",
    "ManaCost",
    "Types",
    "PT",
    "Loyalty",
    "Defense",
    "Colors",
    "Oracle",
    "DeckHas",
    "DeckHints",
    "DeckNeeds",
    "AI",
    "Draft",
    "HandLifeModifier",
];

/// Splits a card script into its rules-bearing lines.
#[must_use]
pub fn parse(text: &str) -> CardScript {
    let mut out = CardScript::default();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((prefix, rest)) = line.split_once(':') else {
            out.unknown_lines.push(line.to_string());
            continue;
        };
        match prefix {
            "K" => out.keywords.push(rest.to_string()),
            "A" | "T" | "S" | "R" => out
                .rules
                .push((prefix.chars().next().unwrap_or('A'), rest.to_string())),
            "SVar" => {
                if let Some((name, body)) = rest.split_once(':') {
                    out.svars.insert(name.to_string(), body.to_string());
                } else {
                    out.unknown_lines.push(line.to_string());
                }
            }
            _ if IGNORED_PREFIXES.contains(&prefix) => {}
            _ => out.unknown_lines.push(line.to_string()),
        }
    }
    out
}

/// Parameter keys that are pure prose or AI hints: they change no rule, so
/// consuming them silently is safe. Everything not on this list must be
/// claimed by a rule or the card is refused.
const PROSE_KEYS: &[&str] = &[
    "SpellDescription",
    "StackDescription",
    "Description",
    "TriggerDescription",
    "TgtPrompt",
    "AILogic",
    "AINoRecursiveCheck",
    "AICheckSVar",
    "AISVarCompare",
    "AIPreference",
    // Which mana the reference's own AI should rather spend — twelve
    // scripts, and every value it takes is a colour, `Treasure` or
    // `NotSameCard` (measured 2026-09-16). It changes no rule, which is
    // the only thing that puts a key on this list; it is here because
    // Basalt Monolith prints it beside `AILogic`, not to move a number.
    "AIManaPref",
    "AICurse",
    "PrecostDesc",
    "CostDesc",
    "References",
    "SpellDescriptionSVar",
];

/// An ordered `Key$ Value` list that records what has been read.
#[derive(Debug, Default)]
struct Params {
    entries: Vec<(String, String)>,
}

impl Params {
    /// Splits `"DealDamage | ValidTgts$ Any | NumDmg$ 3"` into the API name
    /// and its parameters.
    fn parse(spec: &str) -> Option<(String, Self)> {
        let mut parts = spec.split(" | ");
        let head = parts.next()?.trim();
        // `AB$ DealDamage`, `DB$ …`, `SP$ …`, `ST$ …`, or a bare `Mode$ …`.
        // Every one of those has a `$`, and a head without one is none of
        // them: an `SVar` body is very often a bare number (`SVar:SacMe:1`,
        // `SVar:X:3`), which this read as an API called "1" with no
        // parameters and then removed a leading entry that was never
        // pushed. It panicked rather than refusing, and held only because
        // nothing walked *every* `SVar` — every caller asked about a body a
        // rules line had named. `token_stems` walks all of them.
        let api = head.split_once('$')?.1.trim().to_string();
        let mut entries = Vec::new();
        for part in parts {
            let (key, value) = part.split_once('$')?;
            entries.push((key.trim().to_string(), value.trim().to_string()));
        }
        Some((api, Self { entries }))
    }

    fn take(&mut self, key: &str) -> Option<String> {
        let i = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(i).1)
    }

    /// Whether a parameter is present, without claiming it.
    fn has(&self, key: &str) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// A parameter's value, without claiming it: for a rule that has to
    /// know one key before it can read another.
    fn peek(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Claims a label that only restates the filter standing beside it.
    ///
    /// **Not a [`PROSE_KEYS`] entry, and the difference is the guard.** A key
    /// on that list is prose wherever it appears; this one is prose only
    /// while the parameter it describes is there to carry the meaning — so
    /// the same word would be refused on a line that wrote the label and no
    /// filter, where it would be the only thing said about what is being
    /// found.
    ///
    /// Measured before it was written, which is what the rule about that
    /// list asks for: `ChangeTypeDesc$` appears on 369 lines of the
    /// reference, every one of them a `ChangeZone`, and **not one** of them
    /// without a `ChangeType$` beside it. Its commonest value is `basic
    /// land` (257), which is `Land.Basic` spelled for a human.
    fn claim_label(&mut self, label: &str, filter: &str) {
        if self.has(filter) {
            self.take(label);
        }
    }

    fn drop_prose(&mut self) {
        self.entries
            .retain(|(k, _)| !PROSE_KEYS.contains(&k.as_str()));
    }

    /// The first parameter still unclaimed, for reporting.
    fn first_key(&self) -> Option<&str> {
        self.entries.first().map(|(k, _)| k.as_str())
    }

    /// True when every parameter has been claimed.
    fn exhausted(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A chain of `SubAbility$`-linked effects and the target they share.
#[derive(Debug, Default)]
struct Chain {
    effects: Vec<String>,
    target: Option<String>,
    could_add_mana: bool,
    moves_library: bool,
}

/// "… unless <a player> pays <a price>" (CR 118.12a), read off one line.
struct Unless {
    /// Who is asked, as a `PlayerRel`.
    payer: &'static str,
    price: Price,
    /// `UnlessSwitched$ True`: the line's effect is what paying *buys*
    /// ("you may pay {1}. If you do, …"), not what refusing costs.
    switched: bool,
}

/// A price a player may pay as an ability resolves, in the shape the effect
/// that charges it carries.
enum Price {
    /// Generic mana, as an `Amount` expression: `PlayerMayPayOr`/`Then`.
    Generic(String),
    /// A printed cost with colour in it, spelled `{G}{G}`:
    /// `PlayerMayPayManaOr`/`Then`.
    Printed(String),
    /// One cost part the player pays by naming an object:
    /// `PlayerMayPayCostOr`.
    Part(String),
}

/// Which side of a block a `T:Mode$ AttackerBlockedByCreature` line puts
/// its source on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BlockRole {
    /// `ValidBlocker$ Card.Self`: "whenever this creature blocks …".
    Blocks,
    /// `ValidCard$ Card.Self`: "whenever this creature becomes blocked by …".
    Blocked,
}

/// One half of a "blocks or becomes blocked by" trigger, read and waiting
/// for its mirror ([`Tx::block_trigger`]).
struct BlockHalf {
    role: BlockRole,
    /// The valid-string of the other creature.
    other: String,
    /// Whether this half carried `Secondary$ True`.
    secondary: bool,
    /// The ability this half wrote.
    ability: String,
}

struct Tx<'a> {
    svars: &'a BTreeMap<String, String>,
    cats: &'a SubtypeCatalogs,
    /// The reference's token scripts, when a checkout is at hand. `None` is
    /// a run with no token corpus and not a gap in the DSL, which is why the
    /// refusal it produces says so in those words — a report that ranked a
    /// missing directory as the top blocker would send somebody to write a
    /// rule that already exists.
    tokens: Option<&'a TokenLookup>,
    /// Whether the rules line being read announces an `X` of its own.
    ///
    /// True for `A:` — a spell's X is chosen on the stack (CR 601.2b) and an
    /// activated ability's on activation, and the engine reads both back as
    /// `Amount::X`. False for `T:`, `S:` and `R:`, where nothing announced a
    /// number and `Amount::X` would silently evaluate to nought. Set per
    /// line rather than per script, because one card writes both.
    has_x: bool,
    /// Whether the rules line being read is a spell (`A:SP$`) rather than an
    /// activated ability or a trigger. A spell has no "itself" to act on
    /// once it resolves, so a clause that defaults to the source means
    /// nothing on one.
    on_a_spell: bool,
    /// The mode of the `T:` line being read (`Phase`, `ChangesZone`, …), and
    /// `None` on every other kind of line: what "that player" means is the
    /// trigger's to say.
    trigger_mode: Option<String>,
    /// Which side of a block the `T:` line being read puts its source on,
    /// and the half it read ([`Tx::block_trigger`]); `None` on every other
    /// line.
    block_line: Option<(BlockRole, String, bool)>,
    /// The first half of a "blocks or becomes blocked by" trigger, while its
    /// mirror has not been read. A script that ends with one here printed a
    /// sentence this reader does not say, and is refused.
    block_half: Option<BlockHalf>,
    /// Whether the chain being read is a delayed trigger's `Execute$`, where
    /// `Defined$ DelayTriggerRememberedLKI` is the object it remembers.
    in_delayed: bool,
    body: CardBody,
    /// The first `Api.Key` no rule claimed, if that is why this
    /// script was refused. Recorded rather than derived, because a
    /// second list of each rule's keys would rot the first time a
    /// rule learned a new one.
    unclaimed: std::cell::RefCell<Option<String>>,
}

impl Tx<'_> {
    /// Records the first reason this script was refused.
    ///
    /// A diagnostic side channel, which is why it is behind a `RefCell`:
    /// the refusal points are `&self` readers, and threading `&mut` through
    /// them to carry a message would put the report in the way of the rules.
    fn note(&self, what: String) {
        let mut slot = self.unclaimed.borrow_mut();
        if slot.is_none() {
            *slot = Some(what);
        }
    }

    /// Refuses, saying why.
    ///
    /// The spelling that makes a silent `?` visible at the point it happens.
    /// Every refusal that reaches [`refusal_reason`] with nothing recorded is
    /// reported as "no reason recorded", which is a worklist entry naming no
    /// work — the same fault [`crate::scriptgen`]'s own report was written to
    /// fix one level up.
    fn deny<T>(&self, what: String) -> Option<T> {
        self.note(what);
        None
    }

    /// The named constants on `Filter`, against what this reader would
    /// otherwise write out.
    ///
    /// Every pair is the **same bytes** — clause order included — which is
    /// what makes the substitution invisible to `pool-dump` and is why the
    /// table is a lookup rather than a second way of building a filter. A
    /// constant this reader cannot reach byte for byte is therefore absent
    /// rather than approximated: `BASIC_LAND` is supertype first, and
    /// `YOUR_BASIC_LAND` nests it, where a script filter reading
    /// `Land.Basic.YouCtrl` builds three flat clauses. `YOUR_LAND` is here
    /// because it is noun first, which is the order this reader writes.
    ///
    /// A valid-string names its clauses in an order of its own, and for a
    /// filter of two or more the corpus prints more than one — so a row is
    /// worth having only where the constant's order is the one the corpus
    /// predominantly writes. Measured over the reference corpus, in files,
    /// with a restriction counted under either joiner (`Land.nonBasic` and
    /// `Land+nonBasic` are one order written two ways):
    /// `Artifact,Creature` 247 against `Creature,Artifact` 80,
    /// `Artifact.YouCtrl` 432 against **nought** the other way,
    /// `Creature,Planeswalker` 247 against nought,
    /// `Artifact,Creature,Enchantment` 31 against `Artifact,Enchantment,
    /// Creature` 13, and `Land.nonBasic` 78 against nought.
    ///
    /// The commit that added these rows reported the artifact one as "432
    /// against `YouCtrl.Artifact` 27", and the 27 was a measurement with an
    /// unescaped dot: what it found was `YouCtrl,Artifact`, where the comma
    /// is the corpus's *or* and the string is a different filter entirely.
    /// No script in the corpus writes the control clause before the noun.
    ///
    /// `ANOTHER_CREATURE_YOU_CONTROL` is the one constant that fails the
    /// test, and the reason the rule is written down: its order is `your`
    /// before `another`, where the corpus writes `Creature.Other+YouCtrl`
    /// 381 times against `Creature.YouCtrl+Other` 170, so a row would name
    /// the rarer spelling and write the commoner one out — the table
    /// disagreeing with itself on one filter.
    const NAMED: &'static [(&'static str, &'static str)] = &[
        (
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])",
            "Filter::YOUR_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByOpponent])",
            "Filter::OPPONENT_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Another])",
            "Filter::ANOTHER_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::IsToken)])",
            "Filter::NONTOKEN_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::HasSupertype(SupertypeSet::LEGENDARY)])",
            "Filter::LEGENDARY_CREATURE",
        ),
        (
            "Filter::And(&[Filter::CREATURE, Filter::Attacking])",
            "Filter::ATTACKING_CREATURE",
        ),
        (
            "Filter::And(&[Filter::LAND, Filter::ControlledByYou])",
            "Filter::YOUR_LAND",
        ),
        (
            "Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou])",
            "Filter::YOUR_ARTIFACT",
        ),
        (
            "Filter::And(&[Filter::LAND, Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC))])",
            "Filter::NONBASIC_LAND",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT])",
            "Filter::ARTIFACT_OR_ENCHANTMENT",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE])",
            "Filter::ARTIFACT_OR_CREATURE",
        ),
        (
            "Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ENCHANTMENT])",
            "Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT",
        ),
        (
            "Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER])",
            "Filter::CREATURE_OR_PLANESWALKER",
        ),
        (
            "Filter::Or(&[Filter::HasType(TypeSet::INSTANT), Filter::HasType(TypeSet::SORCERY)])",
            "Filter::INSTANT_OR_SORCERY",
        ),
    ];
}

/// Reads a whole card script, or refuses it.
///
/// # Errors
/// Never errors; an unreadable script is `None`, which is what keeps a
/// generated card honest.
#[must_use]
pub fn transcode(
    script: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<CardBody> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    let mut tx = Tx {
        svars: &script.svars,
        cats,
        tokens,
        has_x: false,
        on_a_spell: false,
        trigger_mode: None,
        block_line: None,
        block_half: None,
        in_delayed: false,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        tx.keyword(line)?;
    }
    for (kind, spec) in &script.rules {
        tx.rule(*kind, spec)?;
    }
    tx.block_halves_paired()?;
    if tx.body.is_empty() {
        return None;
    }
    tx.body
        .notes
        .push("transcoded from the card's rules".into());
    Some(tx.body)
}

#[cfg(test)]
mod tests;

/// A card-script reference checkout, resolved by card name.
///
/// Held by `codegen` so a stub can be transcoded from the rules reference
/// when one is available locally, and generated exactly as before when it is
/// not — the checkout is never part of the build.
#[derive(Debug)]
pub struct ScriptLookup {
    root: std::path::PathBuf,
    index: BTreeMap<String, String>,
}

impl ScriptLookup {
    /// Wraps a cardsfolder and the name → relative-path index `codegen`
    /// already builds.
    #[must_use]
    pub fn new(root: std::path::PathBuf, index: BTreeMap<String, String>) -> Self {
        Self { root, index }
    }

    /// The script for a card, by its Scryfall name.
    #[must_use]
    pub fn script(&self, name: &str) -> Option<CardScript> {
        // Multi-face Scryfall names ("A // B") are one reference script, filed
        // under the front face.
        let key = self.index.get(name).or_else(|| {
            let front = name.split(" // ").next()?;
            self.index.get(front)
        })?;
        let text = std::fs::read_to_string(self.root.join(key)).ok()?;
        Some(parse(&text))
    }
}
