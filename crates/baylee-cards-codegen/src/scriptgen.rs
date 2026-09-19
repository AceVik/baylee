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
//! The corpus is read as an automated lookup only; no file of it is copied into
//! this repository.

use crate::body::CardBody;
use crate::catalog::SubtypeCatalogs;
use crate::tokengen::TokenLookup;
use std::collections::BTreeMap;

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
        let api = head
            .split_once('$')
            .map_or_else(|| head.to_string(), |(_, v)| v.trim().to_string());
        let mut entries = Vec::new();
        if let Some((key, value)) = head.split_once('$') {
            entries.push((key.trim().to_string(), value.trim().to_string()));
        }
        for part in parts {
            let (key, value) = part.split_once('$')?;
            entries.push((key.trim().to_string(), value.trim().to_string()));
        }
        // The leading `AB$ DealDamage` entry is the api, not a parameter.
        entries.remove(0);
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
    body: CardBody,
    /// The first `Api.Key` no rule claimed, if that is why this
    /// script was refused. Recorded rather than derived, because a
    /// second list of each rule's keys would rot the first time a
    /// rule learned a new one.
    unclaimed: std::cell::RefCell<Option<String>>,
}

/// A whole number, or an `SVar` that resolves to one.
fn amount(raw: &str, svars: &BTreeMap<String, String>, has_x: bool) -> Option<String> {
    let raw = raw.trim().trim_start_matches('+');
    if let Ok(n) = raw.parse::<i64>() {
        return Some(format!("Amount::Fixed({n})"));
    }
    // `X` is the number the player announced, and `Amount::X` is how the
    // engine reads it back — off the spell for a cast and off
    // `Engine::activation_x` for an activation. Two things have to be true
    // before that is the right reading, and both are checked because
    // neither is visible at the use site.
    //
    // The corpus's `X` is only *sometimes* that number: of the 356 scripts
    // that write `TokenAmount$ X`, 58 define `SVar:X:Count$xPaid` and the
    // rest count something — damage dealt, opponents, creatures in a
    // graveyard. So the definition is demanded rather than assumed.
    //
    // And `has_x` is the other half: a **triggered** ability announces no
    // number, so `Amount::X` would evaluate to `x.unwrap_or(0)` — a card
    // that compiles, claims `Implemented` and makes nothing at all, which
    // is exactly the outcome the honest-stub rule exists to prevent.
    if raw == "X" {
        return (has_x && svars.get("X").map(String::as_str) == Some("Count$xPaid"))
            .then(|| "Amount::X".to_string());
    }
    let resolved = svars.get(raw)?;
    let n = resolved.trim().parse::<i64>().ok()?;
    Some(format!("Amount::Fixed({n})"))
}

/// `W` as `ManaColor::White`, for a reader that is not inside a closure.
fn mana_color_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "W" => "ManaColor::White",
        "U" => "ManaColor::Blue",
        "B" => "ManaColor::Black",
        "R" => "ManaColor::Red",
        "G" => "ManaColor::Green",
        "C" => "ManaColor::Colorless",
        _ => return None,
    })
}

/// `UR` as `{U/R}`, or `None` when the token is not a hybrid pair.
///
/// The reference runs the two letters together and the printed card puts a
/// slash between them; `ColorPair` keeps the printed order — `{G/U}`, not
/// `{U/G}` — so the pair is written the way it is read.
fn hybrid_pair(token: &str) -> Option<String> {
    let mut letters = token.chars();
    let (Some(a), Some(b), None) = (letters.next(), letters.next(), letters.next()) else {
        return None;
    };
    let colored = |c: char| "WUBRG".contains(c);
    (colored(a) && colored(b) && a != b).then(|| format!("{{{a}/{b}}}"))
}

/// A `NumAtt`/`NumDef` value as an `Amount`.
///
/// Separate from [`amount`] because a pump is the one place a *negative*
/// constant is ordinary, and `Amount::Fixed` holds a `u32` — the sign
/// lives in the variant, not in the number. Everything else it asks is the
/// same, and it asked none of it until a batch of six hundred cards walked
/// seven finished pumps into the pool that pump by nothing at all.
fn pump_amount(raw: &str, svars: &BTreeMap<String, String>, has_x: bool) -> Option<String> {
    let raw = raw.trim();
    // `+X/+X` is the second commonest pump printed, and the two questions
    // [`amount`] asks of an `X` are asked here for the same two reasons.
    //
    // The letter is not the number. 207 scripts in the reference pump by
    // `X`, every one of them defines `SVar:X`, and only **44** define it as
    // `Count$xPaid` — the X the player announced, which `Amount::X` reads
    // straight back off the spell. The other 163 are *counts*:
    // `Count$Domain`, `Count$Valid Artifact.YouCtrl`, the greatest mana
    // value among permanents you control, and the DSL has no way to say any
    // of them. Written as `Amount::X` regardless, Gaea's Might came out
    // claiming `Implemented` and giving +0/+0, and six more with it.
    //
    // And `has_x` is the other half, because a triggered ability announces
    // no number at all: there `Amount::X` is `x.unwrap_or(0)`.
    match raw {
        "X" | "+X" | "-X" => {
            if !has_x || svars.get("X").map(String::as_str) != Some("Count$xPaid") {
                return None;
            }
            return Some(
                if raw == "-X" {
                    "Amount::NegX"
                } else {
                    "Amount::X"
                }
                .to_string(),
            );
        }
        _ => {}
    }
    let n = raw.parse::<i64>().ok().or_else(|| {
        svars
            .get(raw.trim_start_matches('+'))?
            .trim()
            .parse::<i64>()
            .ok()
    })?;
    Some(if n < 0 {
        format!("Amount::NegXFixed({})", n.unsigned_abs())
    } else {
        format!("Amount::Fixed({n})")
    })
}

fn plain_number(raw: &str, svars: &BTreeMap<String, String>) -> Option<i64> {
    let raw = raw.trim().trim_start_matches('+');
    raw.parse::<i64>()
        .ok()
        .or_else(|| svars.get(raw)?.trim().parse::<i64>().ok())
}

/// Which side of a count an enters-tapped clause is on, once the reference's
/// comparator has been read.
///
/// The reference states when the land comes down **tapped** and the card
/// prints when it does not, so every comparator [`Tx::enters_tapped_unless`]
/// sees is already the opposite of what a player reads. Naming the two
/// directions keeps that inversion in one place: the arithmetic happens where
/// the comparator is parsed, and the emitter picks its variant from a word
/// instead of re-deriving a direction from a number it was handed.
#[derive(Clone, Copy)]
enum Bound {
    /// "unless you control N or more" — a slow land, a battle land, and at
    /// one a checkland.
    AtLeast(u16),
    /// "unless you control N or fewer" — a fast land, and the same predicate
    /// the manlands print as "if you control N+1 or more, it enters tapped".
    AtMost(u16),
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

    /// A valid-string (`Creature.YouCtrl+nonToken`) as a `Filter`.
    fn filter_expr(&self, valid: &str) -> Option<String> {
        let mut alternatives = Vec::new();
        for alt in valid.split(',') {
            let mut clauses = Vec::new();
            let mut atoms = alt.split('.');
            let base = atoms.next()?.trim();
            match base {
                "Card" | "Permanent" => {}
                "Creature" => clauses.push("Filter::CREATURE".to_string()),
                "Artifact" => clauses.push("Filter::ARTIFACT".to_string()),
                "Enchantment" => clauses.push("Filter::ENCHANTMENT".to_string()),
                "Land" => clauses.push("Filter::LAND".to_string()),
                "Planeswalker" => clauses.push("Filter::PLANESWALKER".to_string()),
                "Instant" => clauses.push("Filter::HasType(TypeSet::INSTANT)".to_string()),
                "Sorcery" => clauses.push("Filter::HasType(TypeSet::SORCERY)".to_string()),
                _ => {
                    let Some(path) = self.cats.const_path(base) else {
                        self.note(format!("filter base `{base}`"));
                        return None;
                    };
                    clauses.push(format!("Filter::HasSubtype({path})"));
                }
            }
            for atom in atoms.flat_map(|a| a.split('+')) {
                clauses.push(match atom.trim() {
                    // "Enchanted creature gets +2/+1", "equipped creature has
                    // flying": one filter for both, because there is one
                    // question — what is this permanent attached to — and the
                    // corpus asks it in the two words the two card types
                    // print. An Aura and an Equipment differ in what happens
                    // when the answer is nothing (CR 704.5m against
                    // 704.5n–p), which is the state-based actions' business
                    // and not this clause's.
                    "EnchantedBy" | "EquippedBy" => "Filter::AttachedToBySource".to_string(),
                    "YouCtrl" => "Filter::ControlledByYou".to_string(),
                    "OppCtrl" => "Filter::ControlledByOpponent".to_string(),
                    "YouOwn" => "Filter::OwnedByYou".to_string(),
                    "Other" => "Filter::Another".to_string(),
                    "Self" => "Filter::This".to_string(),
                    "attacking" => "Filter::Attacking".to_string(),
                    "tapped" => "Filter::Tapped".to_string(),
                    "untapped" => "Filter::Untapped".to_string(),
                    "token" => "Filter::IsToken".to_string(),
                    "nonToken" | "!token" => "Filter::Not(&Filter::IsToken)".to_string(),
                    // `LacksType` and not `Not(&…)`: the two are the same
                    // predicate (`eval.rs` negates one line to get the
                    // other) and the DSL already carries a name for each,
                    // so writing the negation out would give "not a
                    // creature" a second spelling that only `filter_hash`
                    // can tell from the first.
                    "nonLand" => "Filter::NONLAND".to_string(),
                    "nonCreature" => "Filter::NONCREATURE".to_string(),
                    // Supertypes read like subtypes in a script filter but are
                    // a different set on the card (CR 205.4).
                    "Basic" => "Filter::HasSupertype(SupertypeSet::BASIC)".to_string(),
                    "nonBasic" => {
                        "Filter::Not(&Filter::HasSupertype(SupertypeSet::BASIC))".to_string()
                    }
                    "Legendary" => "Filter::HasSupertype(SupertypeSet::LEGENDARY)".to_string(),
                    "nonLegendary" => {
                        "Filter::Not(&Filter::HasSupertype(SupertypeSet::LEGENDARY))".to_string()
                    }
                    "Snow" => "Filter::HasSupertype(SupertypeSet::SNOW)".to_string(),
                    "" => continue,
                    // `Creature.Goblin` puts the subtype after the base, so
                    // an atom can name one too — and it is the commonest
                    // shape in the corpus, not a corner.
                    other => {
                        let (negated, name) = other
                            .strip_prefix("non")
                            .map_or((false, other), |rest| (true, rest));
                        let Some(path) = self.cats.const_path(name) else {
                            self.note(format!("filter atom `{other}`"));
                            return None;
                        };
                        if negated {
                            format!("Filter::Not(&Filter::HasSubtype({path}))")
                        } else {
                            format!("Filter::HasSubtype({path})")
                        }
                    }
                });
            }
            alternatives.push(match clauses.len() {
                0 => "Filter::Any".to_string(),
                1 => clauses.remove(0),
                _ => format!("Filter::And(&[{}])", clauses.join(", ")),
            });
        }
        let expr = match alternatives.len() {
            0 => return None,
            1 => alternatives.remove(0),
            _ => format!("Filter::Or(&[{}])", alternatives.join(", ")),
        };
        Some(Self::named_constant(&expr))
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

    /// The name for a filter the DSL already has one for, or the expression
    /// unchanged.
    fn named_constant(expr: &str) -> String {
        Self::NAMED
            .iter()
            .find(|(written, _)| *written == expr)
            .map_or_else(|| expr.to_string(), |(_, name)| (*name).to_string())
    }

    /// A `ValidTgts$` value as a `TargetSpec` expression.
    /// The zone a target lives in comes from the *effect*, not from the
    /// valid-string: `TargetSpec::Object` enumerates the battlefield and
    /// nothing else, so a counterspell built out of one offers permanents
    /// as targets and counters nothing.
    fn target_spec(&mut self, valid: &str, api: &str) -> Option<String> {
        // "Target player" and "target opponent" are both a *choice*, and
        // they are different choices: `Player(PlayerRel::Opponent)` would be
        // every opponent and no choice at all.
        if valid == "Player" {
            return Some("TargetSpec::AnyPlayer".to_string());
        }
        if valid == "Opponent" {
            return Some("TargetSpec::AnyOpponent".to_string());
        }
        // "Any target" (CR 115.4) spans objects and players, which is why it
        // is a spec and not a filter — there is nothing on a player for a
        // `Filter` to match.
        if valid == "Any" {
            return Some("TargetSpec::AnyTarget".to_string());
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("TARGET", &expr);
        Some(match api {
            "Counter" => format!("TargetSpec::Spell(&{name})"),
            _ => format!("TargetSpec::Object(&{name})"),
        })
    }

    /// `Defined$ You` and friends as a `PlayerRel`.
    fn player_rel(defined: Option<&str>) -> Option<&'static str> {
        Some(match defined.unwrap_or("You") {
            "You" => "PlayerRel::You",
            "Opponent" | "Player.Opponent" => "PlayerRel::Opponent",
            "Player" => "PlayerRel::EachPlayer",
            _ => return None,
        })
    }

    /// The same as [`Self::player_rel`], for an effect that sits in a chain
    /// which *targets a player*.
    ///
    /// The corpus leaves `Defined$` off when the effect means the target, and only
    /// the chain knows whether that target was a player. Reading the absent
    /// key as `You` there is how Piranha Marsh — "target player loses 1 life"
    /// — generated as a land that drains its own controller.
    fn player_rel_of(defined: Option<&str>, target: Option<&str>) -> Option<&'static str> {
        if defined.is_none() && target == Some("TargetSpec::Player(PlayerRel::Chosen)") {
            return Some("PlayerRel::Chosen");
        }
        Self::player_rel(defined)
    }

    /// One effect and everything its `SubAbility$` chain adds.
    fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an ability spec with no `$` in it".to_string());
        };
        p.drop_prose();
        if let Some(valid) = p.take("ValidTgts") {
            let Some(spec) = self.target_spec(&valid, &api) else {
                return self.deny(format!("target `{valid}`"));
            };
            if chain.target.get_or_insert(spec.clone()) != &spec {
                return self.deny("two different targets in one chain".to_string());
            }
        }
        let sub = p.take("SubAbility");
        // The requirement and the effect name the target differently when it
        // is a player: the wizard resolves `AnyPlayer`/`AnyOpponent` into the
        // spell's chosen player, and the effect then reads it back as
        // `PlayerRel::Chosen`. Handing the *requirement* to the effect
        // instead is how a burn spell ends up dealing damage to nothing at
        // all: `DealDamage` looks for an object target and finds none.
        let target: Option<String> = match chain.target.as_deref() {
            Some("TargetSpec::AnyPlayer" | "TargetSpec::AnyOpponent") => {
                Some("TargetSpec::Player(PlayerRel::Chosen)".to_string())
            }
            Some(other) => Some(other.to_string()),
            None => None,
        };

        let Some(effects) = self.effect_of(&api, &mut p, target.as_deref()) else {
            // An API with no rule at all is a different report than a rule
            // that met a value it cannot say — the first is a missing
            // effect, the second is a missing case in one that exists.
            if is_supported_api(&api) {
                self.note(format!("unreadable value in `{api}`"));
            } else {
                self.note(format!("effect `{api}`"));
            }
            return None;
        };
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `{api}.{key}`"));
            }
            return None;
        }
        chain.effects.extend(effects);
        match sub {
            Some(name) => {
                let Some(body) = self.svars.get(&name).cloned() else {
                    return self.deny(format!("`SubAbility$ {name}` names no SVar"));
                };
                self.chain(&body, chain)
            }
            None => Some(()),
        }
    }

    /// One effect API as the `Effect` expressions it stands for.
    ///
    /// Every parameter a rule reads is *taken* from `p`; the caller then
    /// refuses the card if anything is left, which is what stops an ignored
    /// `NoRegen$ True` from generating a card that does the wrong thing.
    fn effect_of(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
            "DealDamage" => {
                let n = amount(&p.take("NumDmg")?, self.svars, self.has_x)?;
                let to = match p.take("Defined").as_deref() {
                    None => aimed.to_string(),
                    Some("You") => "TargetSpec::Player(PlayerRel::You)".to_string(),
                    Some("Opponent") => "TargetSpec::Player(PlayerRel::Opponent)".to_string(),
                    Some(_) => return None,
                };
                vec![format!(
                    "Effect::DealDamage {{ amount: {n}, target: {to} }}"
                )]
            }
            "GainLife" => {
                let n = plain_number(&p.take("LifeAmount")?, self.svars)?;
                match Self::player_rel_of(p.take("Defined").as_deref(), target)? {
                    "PlayerRel::You" => vec![format!("Effect::gain_life({n})")],
                    who => vec![format!(
                        "Effect::GainLifeFor {{ amount: Amount::Fixed({n}), who: {who} }}"
                    )],
                }
            }
            "LoseLife" => {
                let n = amount(&p.take("LifeAmount")?, self.svars, self.has_x)?;
                let who = Self::player_rel_of(p.take("Defined").as_deref(), target)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Draw" => {
                let n = plain_number(p.take("NumCards").as_deref().unwrap_or("1"), self.svars)?;
                match Self::player_rel_of(p.take("Defined").as_deref(), target)? {
                    "PlayerRel::You" => vec![format!("Effect::draw({n})")],
                    who => vec![format!(
                        "Effect::DrawCardsFor {{ amount: Amount::Fixed({n}), who: {who} }}"
                    )],
                }
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars, self.has_x)?;
                let who = Self::player_rel_of(p.take("Defined").as_deref(), target)?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "PutCounter" => {
                let code = p.take("CounterType")?;
                let Some(kind) = counter_kind(&code) else {
                    return self.deny(format!("counter `{code}`"));
                };
                let n = amount(
                    p.take("CounterNum").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                // `AddCounter` puts them on the first target, or on the
                // source when the ability has none — which is exactly what
                // `Defined` means here.
                match p.take("Defined").as_deref() {
                    None | Some("Self") => {}
                    Some(_) => return None,
                }
                vec![format!(
                    "Effect::AddCounter {{ kind: {kind}, amount: {n} }}"
                )]
            }
            "Scry" => {
                let n = plain_number(p.take("ScryNum").as_deref().unwrap_or("1"), self.svars)?;
                vec![format!("Effect::scry({n})")]
            }
            "Surveil" => {
                // `Amount$` and never `Defined$`: not one of the reference
                // corpus's 228 surveil lines names a player, because a
                // surveil is the controller's own library by construction
                // (CR 701.25a). 224 of them write `Amount$` and 219 of those
                // write a plain number; the rest announce an `X`, and the
                // constructor only fits the first kind.
                let raw = p.take("Amount")?;
                if let Some(n) = plain_number(&raw, self.svars) {
                    vec![format!("Effect::surveil({n})")]
                } else {
                    let n = amount(&raw, self.svars, self.has_x)?;
                    vec![format!("Effect::Surveil {{ amount: {n} }}")]
                }
            }
            "Mana" => self.mana_effect(p)?,
            "Destroy" => {
                // `NoRegen$ True` is vacuous here and may be consumed:
                // `Effect::Destroy` already destroys unconditionally,
                // because the engine has no regeneration mechanic for a
                // shield to be worth anything against. Any other value
                // would be saying something about regeneration that this
                // engine cannot say, so it refuses.
                if p.take("NoRegen").is_some_and(|v| v != "True") {
                    return None;
                }
                vec![format!("Effect::destroy({aimed})")]
            }
            "Token" => self.token_effect(p, target)?,
            "Animate" => self.animate_effect(p, target)?,
            "Pump" => self.pump_effect(p, aimed)?,
            "ChangeZone" => self.change_zone(p, target)?,
            "Sacrifice" => self.sacrifice_effect(p)?,
            "Tap" => vec!["Effect::TapTarget".to_string()],
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match target {
                Some(_) => vec!["Effect::UntapTarget".to_string()],
                None => vec!["Effect::UntapSelf".to_string()],
            },
            "Counter" => {
                if p.take("TargetType").as_deref() != Some("Spell") {
                    return None;
                }
                vec!["Effect::CounterTargetSpell".to_string()]
            }
            _ => return None,
        })
    }

    /// `DB$ Sacrifice`: "sacrifice it", with or without a way out.
    ///
    /// Three sentences in one API, and only two of them are this rule.
    /// **Bare** is "sacrifice this" — 111 of the corpus's 895 sacrifice
    /// lines, and [`Effect::SacrificeSelf`] says it exactly. With an
    /// `UnlessCost$` it is the Karoo sentence, "sacrifice it unless you
    /// <pay>", which is 131 more and the largest single shape the API
    /// writes. What it is **not** is `Defined$`/`SacValid$`: those name
    /// somebody else's permanent ("each player sacrifices a creature"), a
    /// player choice this DSL has no effect for, and reading them as the
    /// source would be a card that sacrifices the wrong permanent under a
    /// `Coverage::Implemented`.
    ///
    /// The price is read by [`Tx::cost_pieces`], the same reader an
    /// activation cost goes through, and then held to what the two "unless"
    /// effects can carry: `PlayerMayPayOr` charges *generic* mana in an
    /// [`Amount`], so a colour is refused rather than silently spent as
    /// colourless, and `PlayerMayPayCostOr` charges exactly one part the
    /// player answers by naming an object. A part that needs no answer
    /// (`PayLife<2>`) is refused here even though `CostPart` can hold it:
    /// the engine asks that question by putting up the list of what may pay,
    /// and an empty list is how a player declines — so a price nobody names
    /// an object for would decline itself every time.
    fn sacrifice_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        for key in [
            "Defined",
            "SacValid",
            "Amount",
            "Optional",
            "RememberSacrificed",
        ] {
            if p.take(key).is_some() {
                return self.deny(format!("a sacrifice naming `{key}`"));
            }
        }
        let Some(cost) = p.take("UnlessCost") else {
            return Some(vec!["Effect::SacrificeSelf".to_string()]);
        };
        // 130 of the 131 say `You` and the one that does not says `Player`,
        // which is every player at once — a price this effect cannot put to
        // a table.
        match p.take("UnlessPayer").as_deref() {
            Some("You") => {}
            other => {
                return self.deny(format!(
                    "a sacrifice charging `{}`",
                    other.unwrap_or("nobody")
                ));
            }
        }
        let (mana, parts) = self.cost_pieces(&cost)?;
        match (mana.as_str(), parts.as_slice()) {
            (m, []) if !m.is_empty() => {
                // Generic only. `{U}` and `{W}{W}` are 40 of these lines and
                // are refused by name: the effect's price is an `Amount` of
                // generic mana with no colour to put a symbol in.
                let Some(n) = m.strip_prefix('{').and_then(|m| m.strip_suffix('}')) else {
                    return self.deny(format!("a sacrifice charging `{m}`"));
                };
                let Ok(n) = n.parse::<u16>() else {
                    return self.deny(format!("a sacrifice charging `{m}`"));
                };
                Some(vec![format!(
                    "Effect::PlayerMayPayOr {{ player: PlayerRel::You, \
                     mana: Amount::Fixed({n}), effect: &Effect::SacrificeSelf }}"
                )])
            }
            ("", [one]) if asks_for_an_object(one) => Some(vec![format!(
                "Effect::PlayerMayPayCostOr {{ player: PlayerRel::You, \
                 cost: &CostPart::{one}, effect: &Effect::SacrificeSelf }}"
            )]),
            _ => self.deny(format!("a sacrifice charging `{cost}`")),
        }
    }

    /// `Token`: "create a 1/1 white Soldier creature token".
    ///
    /// The effect names a `&'static TokenDef`, so what this writes is the
    /// constant the **ledger** filed that token under and never a definition
    /// of its own: the id a client keys token art off is a token's place in
    /// `generated_tokens::ALL`, and a literal written into a card file would
    /// have no place there at all. Which is also why this is one of the two
    /// rules that can be refused by something other than the script — a run
    /// with no token corpus has no constant to name.
    ///
    /// Everything the token *is* — its colours, its size, its keywords — is
    /// read by [`crate::tokengen`] from the token script and never from this
    /// line, which carries none of it.
    fn token_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        // Who gets it. The corpus writes `TokenOwner$ You` on 2220 of its
        // 3610 token lines and leaves the key off on 1154 — and absence is
        // **not** a synonym for you: Rootcast Apprenticeship says "target
        // player creates a 1/1 green Squirrel creature token" with no
        // `TokenOwner$` at all, leaning on the chain's own target instead.
        // That is the trap [`Self::player_rel_of`] was written for, so the
        // absent key is read the same way it reads one there: as the target
        // where the chain has a player to mean, and as you where it has not.
        let owner = match p.take("TokenOwner").as_deref() {
            Some("You") => "PlayerRel::You",
            None => Self::player_rel_of(None, target)?,
            Some(who) => return self.deny(format!("token owner `{who}`")),
        };
        if owner != "PlayerRel::You" {
            return self.deny("a token created under another player's control".to_string());
        }
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        let Some(body) = tokens.body(&stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        // `generated_tokens` and not `tokens`: the ledger is the one door,
        // and which half of it a constant is written in is the generator's
        // business — a hand-written token is re-exported from there under
        // the same name.
        let token = format!("&generated_tokens::{}", body.constant);
        let amount = match p.take("TokenAmount") {
            None => None,
            Some(raw) => match amount(&raw, self.svars, self.has_x) {
                // A refusal names the `SVar` the amount resolves *through*
                // and not merely the letter, because `X` is what 356 scripts
                // write and each of them means it by a different count —
                // the letter alone ranks one entry that is really thirty.
                None => {
                    return match self.svars.get(&raw) {
                        Some(how) => self.deny(format!("token amount `{raw}` = `{how}`")),
                        None => self.deny(format!("token amount `{raw}`")),
                    };
                }
                Some(a) => Some(a),
            },
        };
        Some(match amount.as_deref() {
            // One is the number `CreateToken` already means, and writing it
            // as `CreateTokenN { amount: Amount::Fixed(1) }` would give the
            // commonest token effect there is a second spelling.
            None | Some("Amount::Fixed(1)") => {
                vec![format!("Effect::CreateToken {{ token: {token} }}")]
            }
            Some(n) => vec![format!(
                "Effect::CreateTokenN {{ token: {token}, amount: {n} }}"
            )],
        })
    }

    /// `Animate`: the manland sentence — "until end of turn, this land
    /// becomes a 4/4 white and blue Elemental creature with flying and
    /// vigilance. It's still a land."
    ///
    /// One printed sentence, four continuous effects, because CR 613.1
    /// applies type, colour, ability and power/toughness in that order and
    /// each is its own layer. "It's still a land" is why the types are
    /// *added* rather than set — an animated Colonnade that stopped being a
    /// land would stop making mana.
    ///
    /// Only `Defined$ Self` is read, and only when the chain targets
    /// nothing: `Filter::This` binds to the first target when there is one
    /// and to the source when there is not, so a chain with both would
    /// animate the wrong permanent.
    fn animate_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        if p.take("Defined").as_deref() != Some("Self") || target.is_some() {
            self.note("`Animate` of something other than the source".to_string());
            return None;
        }
        let mut out = Vec::new();
        // Layer 4: the types it becomes. A word is either a card type or a
        // subtype, and the corpus writes both in one list.
        for word in p.take("Types")?.split(',') {
            let word = word.trim();
            let modifier = if let Some(types) = card_type_const(word) {
                format!("Modifier::AddType({types})")
            } else {
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("`Animate` into `{word}`"));
                    return None;
                };
                format!("Modifier::AddSubtype({path})")
            };
            out.push(Self::animate_expr(&modifier));
        }
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 105.3).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            let colors: Option<Vec<&str>> = raw.split(',').map(|c| color_const(c.trim())).collect();
            let Some(colors) = colors else {
                self.note(format!("`Animate` into colours `{raw}`"));
                return None;
            };
            let which = if overwrite { "SetColor" } else { "AddColor" };
            out.push(Self::animate_expr(&format!(
                "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                colors.join(", ")
            )));
        }
        // Layer 6: keywords it gains.
        if let Some(raw) = p.take("Keywords") {
            let each: Option<Vec<&str>> = raw.split('&').map(|k| keyword_const(k.trim())).collect();
            let Some(each) = each else {
                self.note(format!("`Animate` granting `{raw}`"));
                return None;
            };
            let joined =
                each.join(".union(") + &")".repeat(raw.split('&').count().saturating_sub(1));
            out.push(Self::animate_expr(&format!(
                "Modifier::AddKeyword({joined})"
            )));
        }
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr(&format!(
                    "Modifier::SetPT({power}, {toughness})"
                )));
            }
            (None, None) => {}
            _ => {
                self.note("`Animate` setting only one of power and toughness".to_string());
                return None;
            }
        }
        if out.is_empty() {
            self.note("`Animate` that changes nothing".to_string());
            return None;
        }
        Some(out)
    }

    /// One layer of an [`Self::animate_effect`], as the `Effect` expression.
    ///
    /// No layer is passed in because none is written out: `Effect::continuous`
    /// derives it from the modifier the way CR 613.1 does, so the emitter
    /// cannot name a layer that disagrees with what it is applying.
    fn animate_expr(modifier: &str) -> String {
        format!("Effect::continuous(&Filter::This, {modifier}, Duration::UntilEndOfTurn)")
    }

    /// One side of a pump, refused by what its value resolves *through*.
    ///
    /// The same rule as the token amount's: the letter is what 356 scripts
    /// write and each means it by a different count, so an entry naming `X`
    /// would rank thirty questions as one.
    fn pump_side(&mut self, raw: &str) -> Option<String> {
        match pump_amount(raw, self.svars, self.has_x) {
            Some(a) => Some(a),
            None => match self.svars.get(raw.trim().trim_start_matches(['+', '-'])) {
                Some(how) => self.deny(format!("pump amount `{raw}` = `{how}`")),
                None => self.deny(format!("pump amount `{raw}`")),
            },
        }
    }

    /// `Pump`: `NumAtt$ +2 | NumDef$ +2 | KW$ Trample`, the commonest
    /// effect in the whole script corpus.
    ///
    /// `Defined$ Self` and `Defined$ Targeted` are two different effects
    /// here, not one with a flag: `PumpFilter` binds `Filter::This` to the
    /// source, `PumpTarget` to what the spell targeted, and an ability can
    /// have both a target and a pump on itself.
    fn pump_effect(&mut self, p: &mut Params, target: &str) -> Option<Vec<String>> {
        let power = self.pump_side(p.take("NumAtt").as_deref().unwrap_or("0"))?;
        let toughness = self.pump_side(p.take("NumDef").as_deref().unwrap_or("0"))?;
        let keywords = match p.take("KW") {
            None => "KeywordSet::EMPTY".to_string(),
            Some(kw) => {
                let each: Option<Vec<&str>> =
                    kw.split('&').map(|k| keyword_const(k.trim())).collect();
                // A keyword the engine has no bit for is a whole sentence of
                // rules text ("can't block", "doesn't untap"), not a flag —
                // refuse rather than drop it.
                each?.join(".union(") + &")".repeat(kw.split('&').count().saturating_sub(1))
            }
        };
        // Every duration but the default is a lifetime the DSL spells
        // differently; none of them is "until end of turn" with a longer
        // name.
        if p.take("Duration").is_some() {
            return None;
        }
        // Purely an AI targeting hint (don't curse your own team); it moves
        // no rule, so reading it changes nothing.
        p.take("IsCurse");
        Some(match p.take("Defined").as_deref() {
            Some("Self") => vec![format!(
                "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                 power: {power}, toughness: {toughness}, keywords: {keywords}, \
                 duration: Duration::UntilEndOfTurn }}"
            )],
            None | Some("Targeted") => {
                // Without a target this would pump nothing at all.
                if target == "TargetSpec::AnyPlayer" {
                    return None;
                }
                vec![format!(
                    "Effect::PumpTarget {{ power: {power}, toughness: {toughness}, \
                     keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
                )]
            }
            Some(_) => return None,
        })
    }

    /// `ChangeZone` for the zone pairs the engine has an effect for.
    ///
    /// The corpus writes every zone change with one API and two zone names; the
    /// engine has a named effect per movement, because the movements differ
    /// in rules and not only in destination. So this is a table of pairs,
    /// not a translation of `Destination$` — and a pair with no effect
    /// refuses rather than reaching for the nearest one. Battlefield →
    /// Graveyard is the pair that makes the point: it is *not* `Destroy`,
    /// which checks indestructible (CR 702.12b), and generating one for the
    /// other would quietly kill creatures that survive.
    fn change_zone(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        let destination = p.take("Destination")?;
        p.claim_label("ChangeTypeDesc", "ChangeType");
        // A library search is a different effect, not a zone change with a
        // hidden target: a card in a library cannot be targeted at all
        // (CR 115.2 needs a visible object), so `Effect::SearchLibrary`
        // *finds* rather than moves, and carries the shuffle with it.
        if origin == "Library" {
            return self.search_library(p, &destination, target);
        }
        let target = target.unwrap_or("TargetSpec::AnyPlayer");
        let itself = match p.take("Defined").as_deref() {
            None => false,
            Some("Self") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` of `Defined$ {other}`"));
                return None;
            }
        };
        // "Return a land you control to its owner's hand": no target, no
        // `Defined$`, and a player picking one of their own permanents while
        // the ability resolves.
        if !itself
            && target == "TargetSpec::AnyPlayer"
            && origin == "Battlefield"
            && p.has("Hidden")
        {
            return self.return_chosen(p, &destination);
        }
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            self.note("`ChangeZone` with neither a target nor `Defined$`".to_string());
            return None;
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield", "Hand", false) => vec![format!("Effect::bounce({target})")],
            ("Battlefield", "Exile", false) => vec![format!("Effect::exile({target})")],
            ("Battlefield", "Exile", true) => vec!["Effect::ExileSource".to_string()],
            _ => {
                self.note(format!("`ChangeZone` {origin} to {destination}"));
                return None;
            }
        })
    }

    /// `Origin$ Battlefield` with a chooser rather than a target: the bounce
    /// land's "return a land you control to its owner's hand".
    ///
    /// **`Hidden$ True` is the discriminator, not decoration**, and that is
    /// a measurement rather than a reading of the word. Of the reference's
    /// 173 untargeted `Battlefield` → `Hand` lines, 96 carry a
    /// `ChangeType$` and **all 96** of those carry `Hidden$ True`; of the 77
    /// that name no filter — "return this land to its owner's hand", a
    /// different card — exactly one does. So the key separates "somebody
    /// chooses from the battlefield" from "this moves itself" cleanly, and
    /// the branch above is guarded on its presence so a line without it
    /// still gets the old, correct report.
    ///
    /// `Mandatory$ True` is required for the reason [`Self::search_library`]
    /// requires a word about its count: [`Effect::ReturnChosenToHand`] is an
    /// instruction and "you may return" is a different card, so a script
    /// that does not say which is refused rather than read as either. It
    /// separates the corpus 81 to 15 and every one of the twelve lands this
    /// was written for is on the mandatory side.
    ///
    /// Everything else is refused by not being claimed — `DefinedPlayer$`
    /// and `Chooser$` (14 and 9 lines that hand the choice to somebody
    /// else), `Optional$`, `UnlessCost$`, `RememberLKI$`. Each is a rule
    /// this does not have, and the generic unclaimed-parameter report names
    /// it better than a guess would.
    ///
    /// [`Effect::ReturnChosenToHand`]: baylee_cards_dsl::Effect::ReturnChosenToHand
    fn return_chosen(&mut self, p: &mut Params, destination: &str) -> Option<Vec<String>> {
        if destination != "Hand" {
            self.note(format!(
                "`ChangeZone` chosen off the battlefield into {destination}"
            ));
            return None;
        }
        for (key, expected) in [("Hidden", "True"), ("Mandatory", "True")] {
            match p.take(key).as_deref() {
                Some(value) if value == expected => {}
                Some(other) => {
                    self.note(format!("`ChangeZone` with `{key}$ {other}`"));
                    return None;
                }
                None => {
                    self.note(format!("`ChangeZone` chosen without `{key}$`"));
                    return None;
                }
            }
        }
        // One permanent, because that is the only count the effect states.
        // A sentence returning two is a different rule and says so here
        // rather than writing a card that returns one of them.
        let count = plain_number(p.take("ChangeNum").as_deref().unwrap_or("1"), self.svars)?;
        if count != 1 {
            self.note(format!("`ChangeZone` returning {count} chosen permanents"));
            return None;
        }
        let filter = self.filter_expr(&p.take("ChangeType")?)?;
        let name = self.body.filter_static("RETURN", &filter);
        Some(vec![format!(
            "Effect::ReturnChosenToHand {{ who: PlayerRel::You, filter: &{name} }}"
        )])
    }

    /// `Origin$ Library`: the fetchland sentence — "search your library for
    /// an Island or Swamp card, put it onto the battlefield, then shuffle".
    ///
    /// `finds` is positional and its length is how many cards may be found,
    /// so `ChangeNum$ 2` is the same `Find` twice rather than a count beside
    /// it. Nothing here targets, and a line that says it does is a different
    /// card.
    fn search_library(
        &mut self,
        p: &mut Params,
        destination: &str,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        if target.is_some() {
            self.note("`ChangeZone` searching a library *and* targeting".to_string());
            return None;
        }
        let tapped = match p.take("Tapped").as_deref() {
            None | Some("False") => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Tapped$ {other}`"));
                return None;
            }
        };
        let find = match (destination, tapped) {
            ("Battlefield", false) => "Find::BATTLEFIELD",
            ("Battlefield", true) => "Find::BATTLEFIELD_TAPPED",
            ("Hand", false) => "Find::HAND",
            _ => {
                self.note(format!("`ChangeZone` searching into {destination}"));
                return None;
            }
        };
        // "You may search" and "search for up to two" are both this flag:
        // the player may end up with fewer cards than `finds` allows.
        let optional = match p.take("Optional").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` found `Optional$ {other}`"));
                return None;
            }
        };
        // Claimed rather than read: `Mandatory$ True` is the default, every
        // search this emitter writes shuffles afterwards, and `Hidden$ True`
        // only says the library is a hidden zone, which it is.
        //
        // The shuffle is *this emitter's* convention and not a rule — no
        // sub-rule of CR 701.23 makes a search shuffle; the card's own "then
        // shuffle" is what does, as the separate action CR 701.24 names. So
        // `Shuffle$ False` has to be refused here rather than read, because
        // nothing downstream can express a search that leaves the library in
        // order.
        // Asked before the loop below consumes it, because it is the only
        // word that says a count above one is a requirement.
        let mandatory = p.has("Mandatory");
        for (key, expected) in [
            ("Mandatory", "True"),
            ("Shuffle", "True"),
            ("Hidden", "True"),
        ] {
            if let Some(value) = p.take(key)
                && value != expected
            {
                self.note(format!("`ChangeZone` with `{key}$ {value}`"));
                return None;
            }
        }
        let count = plain_number(p.take("ChangeNum").as_deref().unwrap_or("1"), self.svars)?;
        let count = usize::try_from(count).ok()?;
        if count == 0 || count > 4 {
            self.note(format!("`ChangeZone` finding {count} cards"));
            return None;
        }
        // "Search your library for **up to** two basic land cards" and
        // "search your library for three cards and reveal them" are two
        // different cards, and above a count of one the difference is what
        // the player is allowed to find — `optional` is exactly that flag.
        //
        // The script does not always say which it is. Measured over the
        // reference: 137 lines search a library for more than one card, and
        // of the 135 that carry no `Optional$`, 70 print "up to" and 65 do
        // not — identical fields, opposite cards. `Mandatory$ True` does
        // separate one side cleanly (36 lines, **none** of them "up to"),
        // so a script that says either word is read and a script that says
        // neither is refused. Reading the absence of a field as "up to"
        // would be an inference rather than a reading, and it would have
        // written Blighted Woodland — "up to two" — as a card that must
        // find both.
        if count > 1 && !optional && !mandatory {
            self.note(
                "`ChangeZone` finding several cards without saying whether that is a maximum"
                    .to_string(),
            );
            return None;
        }
        let filter = self.filter_expr(&p.take("ChangeType")?)?;
        let name = self.body.filter_static("SEARCH", &filter);
        let finds = vec![find; count].join(", ");
        Some(vec![format!(
            "Effect::SearchLibrary {{ filter: &{name}, finds: &[{finds}], optional: {optional} }}"
        )])
    }

    /// `Produced$ Combo W U | Amount$ 2` and friends.
    fn mana_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let produced = p.take("Produced")?;
        let restrict = match p.take("RestrictValid") {
            None => None,
            Some(valid) => Some(self.spend_restriction(&valid)?),
        };
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        // A literal first and always. The `Amount` this line can now carry is
        // the *dynamic* one, and reaching for it where a number would do
        // would rewrite half the lands in the pool on the next codegen run
        // while saying nothing new about any of them — `Effect::mana` and
        // `Effect::mana_dynamic` build the same `AddMana` and only one of
        // them is what the 508 machine-owned files already say.
        let fixed = plain_number(&raw, self.svars).and_then(|n| u32::try_from(n).ok());
        let color = |c: &str| {
            Some(match c {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                _ => return None,
            })
        };
        let effects = if produced == "Any" {
            match fixed {
                Some(1) => vec!["Effect::mana_of_any_color()".to_string()],
                // "Add three mana of any one color" — **one** pick for the
                // whole amount, which is the difference from `Combo` below
                // and the reason `mana_choice_dynamic` exists beside
                // `mana_combination`.
                _ => vec![format!(
                    "Effect::mana_choice_dynamic(ALL_MANA_COLORS, {})",
                    self.counted_amount(&raw)?
                )],
            }
        } else if produced == "Chosen" || produced == "ChosenColor" {
            // "Add one mana of the chosen color." The colour is on the
            // permanent, named as it entered — `EnterModifier::ChooseColor`
            // is the other half, and a card printing this line without it
            // would make nothing.
            (fixed == Some(1)).then(|| vec!["Effect::mana_chosen()".to_string()])?
        } else if let Some(list) = produced.strip_prefix("Combo ") {
            self.combo_mana(list, &raw, fixed)?
        } else {
            // `Produced$ W U` is "add {W}{U}" — two mana at once, not a
            // choice between them (that is `Combo`). One effect per colour,
            // which is how the bounce lands were already written by hand.
            let Some(colors) = produced
                .split_whitespace()
                .map(color)
                .collect::<Option<Vec<_>>>()
            else {
                // `Produced$ Special EachColorAmong_Valid …` is the whole of
                // what lands here, and it is one rule rather than an
                // unreadable word: a colour per colour among a filter.
                let head = produced.split_whitespace().next().unwrap_or(&produced);
                return self.deny(format!("mana source `{head}`"));
            };
            if let Some(n) = fixed {
                colors
                    .into_iter()
                    .map(|c| format!("Effect::mana({c}, {n})"))
                    .collect()
            } else {
                // "Add {B} for each Swamp you control." One colour only,
                // because a counted amount of *two* colours is a sentence no
                // card prints and `mana_dynamic` has no room for.
                let [only] = &colors[..] else {
                    return self.deny("a counted amount of more than one colour".to_string());
                };
                vec![format!(
                    "Effect::mana_dynamic({only}, {})",
                    self.counted_amount(&raw)?
                )]
            }
        };
        let Some(filter) = restrict else {
            return Some(effects);
        };
        // "Spend this mana only to cast a creature spell" is a rider on the
        // mana, so it can only hang on one effect — a line that made two
        // mana and restricted them would need the restriction twice, and
        // nothing in the corpus prints that.
        let [only] = &effects[..] else {
            self.note("`Mana.RestrictValid` on more than one mana".to_string());
            return None;
        };
        let name = self.body.filter_static("SPEND", &filter);
        Some(vec![format!(
            "{only}.restricted(&{name}, SpendRider::None)"
        )])
    }

    /// `Produced$ Combo …` — a choice among the colours listed, and how many.
    ///
    /// `Combo` is the corpus's word for "one of these", and the amount is
    /// what decides which of two different sentences it is: one mana picked
    /// from a list, or *n* mana each picked from it. "Add {G}{G}, {G}{W}, or
    /// {W}{W}" is the second — the filter cycle's whole point — and
    /// `combination: true` is where it lives (CR 608.2d: the choices are made
    /// as the effect is applied, and nothing in the rules numbers them).
    fn combo_mana(&mut self, list: &str, raw: &str, fixed: Option<u32>) -> Option<Vec<String>> {
        // `ColorIdentity` is a source of its own rather than a colour list:
        // what a Command Tower makes is read off its controller's commanders
        // (CR 903.4), so no card can name the colours.
        if list.trim() == "ColorIdentity" {
            return (fixed == Some(1))
                .then(|| vec!["Effect::mana_commander_identity()".to_string()]);
        }
        // The chosen colour is one of the options rather than a colour of
        // its own: "Add {W} or one mana of the chosen color."
        let (chosen, rest): (Vec<&str>, Vec<&str>) = list
            .split_whitespace()
            .partition(|w| *w == "Chosen" || *w == "ChosenColor");
        // `Combo Any` is the five colours written as one word, and here it is
        // a *list* rather than a source — which is what makes "add four mana
        // in any combination of colors" the same rule as the filter lands'
        // two.
        let colors: Vec<String> = if rest == ["Any"] {
            Vec::new()
        } else {
            let mut out = Vec::new();
            for word in rest {
                let Some(color) = mana_color_const(word) else {
                    // `AnyDifferent` (two mana of *different* colours) and
                    // `NotedColors` (what was chosen while drafting) are each
                    // a rule of their own, and a refusal naming the line
                    // rather than the word would send a reader back to the
                    // script to find out which.
                    return self.deny(format!("`Mana` combination over `{word}`"));
                };
                out.push(color.to_string());
            }
            out
        };
        let listed = if colors.is_empty() {
            "ALL_MANA_COLORS".to_string()
        } else {
            format!("&[{}]", colors.join(", "))
        };
        if !chosen.is_empty() {
            return (fixed == Some(1)).then(|| vec![format!("Effect::mana_chosen_or({listed})")]);
        }
        Some(match fixed {
            Some(1) if colors.is_empty() => vec!["Effect::mana_of_any_color()".to_string()],
            Some(1) => vec![format!("Effect::mana_choice({listed})")],
            // `combination: true`: one pick per mana, and the answers may
            // differ. One pick for the whole amount is `Produced$ Any`, a
            // sentence away and a different card.
            _ => vec![format!(
                "Effect::mana_combination({listed}, {})",
                self.counted_amount(raw)?
            )],
        })
    }

    /// An `Amount$` value as an `Amount`, counts included.
    ///
    /// [`amount`] reads what a number can be without a board: a literal, an
    /// `SVar` that resolves to one, and the `X` a player announced. A mana
    /// line is where the corpus's *counted* amounts are commonest — "Add {B}
    /// for each Swamp you control" — and the DSL has been able to say that
    /// since `Amount::CountOf`; Gaea's Cradle is written with it by hand.
    /// Nothing read it.
    ///
    /// Two differences from [`Self::filter_expr`]'s other counting caller are
    /// deliberate and pull the opposite way from
    /// `a_clause_that_counts_needs_the_filter_to_say_whose`. That rule refuses
    /// a filter naming no controller because `Condition::ControlCount` means
    /// "you control" and would silently narrow it; `CountOf` over
    /// `ZoneSel::Battlefield` counts everything, which is what Cloudpost
    /// prints ("each Locus on the battlefield"). And it refuses `Other`
    /// because a condition has no room for the card asking; `eval::amount`
    /// hands `matches` the source object, so Baldur's Gate's "each **other**
    /// Gate you control" is `Filter::Another` and says what it means.
    fn counted_amount(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = amount(raw, self.svars, self.has_x) {
            return Some(n);
        }
        let Some(def) = self.svars.get(raw.trim()).cloned() else {
            return self.deny(format!("mana amount `{}`", raw.trim()));
        };
        let Some(valid) = def.trim().strip_prefix("Count$Valid ") else {
            // Named by what it resolves *through* and never by its own
            // spelling: `Amount$ X` is one letter standing for thirty
            // different questions, and the definition is the one of them this
            // card is asking.
            return self.deny(format!("count `{}`", def.trim()));
        };
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}"
        ))
    }

    /// `RestrictValid$ Spell.Creature` — what produced mana may be spent on.
    ///
    /// Every alternative has to be a *spell*: `Activated.Hero` restricts an
    /// ability activation instead, which is a second kind of restriction the
    /// `ManaRestriction` filter cannot say, and a card printing both means
    /// both.
    fn spend_restriction(&self, valid: &str) -> Option<String> {
        let spells: Option<Vec<&str>> = valid
            .split(',')
            .map(|alt| alt.trim().strip_prefix("Spell."))
            .collect();
        let Some(spells) = spells else {
            self.note("`Mana.RestrictValid` beyond a spell".to_string());
            return None;
        };
        self.filter_expr(&spells.join(","))
    }

    /// A `Cost$` value as a `Cost` expression, plus whether it taps.
    ///
    /// An unreadable token names itself: a cost is where the widest variety
    /// of the corpus's syntax shows up — `Discard<…>`, `Exile<…>`,
    /// `tapXType<…>` — and a report saying "a cost" would send the reader
    /// back to the script to find out which.
    ///
    /// `&mut self` because one part carries a filter, and a filter is
    /// declared as a `static` above the card the way a target's is.
    fn cost_expr(&mut self, raw: &str) -> Option<String> {
        let (mana, parts) = self.cost_pieces(raw)?;
        Some(crate::body::cost_literal(&mana, &parts))
    }

    /// The same reading, before [`crate::body::cost_literal`] joins it.
    ///
    /// Split out because one caller wants a *part* rather than a cost:
    /// "unless you return an untapped Plains" is priced by a single
    /// [`CostPart`], and re-reading that syntax beside this one would be two
    /// places for `Return<1/Plains>` to mean two things.
    fn cost_pieces(&mut self, raw: &str) -> Option<(String, Vec<String>)> {
        let mut mana = String::new();
        let mut parts: Vec<String> = Vec::new();
        for token in cost_parts(raw) {
            let token = token.as_str();
            if token == "T" {
                parts.push("TapSelf".to_string());
            } else if token == "Q" {
                parts.push("UntapSelf".to_string());
            } else if token.starts_with("Sac<1/CARDNAME") {
                parts.push("SacrificeSelf".to_string());
            } else if let Some((n, kind)) = token
                .strip_prefix("AddCounter<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.split_once('/'))
            {
                // "Put a -1/-1 counter on this creature" as a cost. The
                // counter noun goes through the same table the `PutCounter`
                // *effect* reads, so the two cannot come to disagree about
                // what `M1M1` is — and a noun the DSL has no kind for takes
                // the card off the list rather than guessing at one.
                let Ok(n) = n.parse::<u16>() else {
                    return self.deny(format!("counter count `{n}`"));
                };
                let Some(kind) = counter_kind(kind) else {
                    return self.deny(format!("counter `{kind}`"));
                };
                parts.push(format!("PutCounterSelf {{ kind: {kind}, n: {n} }}"));
            } else if let Some((kind, body)) = object_cost(token) {
                parts.push(self.object_cost_part(kind, body, token)?);
            } else if let Some(n) = token
                .strip_prefix("PayLife<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.parse::<u16>().ok())
            {
                parts.push(format!("PayLife({n})"));
            } else if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token, "W" | "U" | "B" | "R" | "G" | "C")
            {
                mana.push('{');
                mana.push_str(token);
                mana.push('}');
            } else if let Some(pair) = hybrid_pair(token) {
                // `Cost$ UR T` is `{U/R}, {T}` — one mana of either colour
                // (CR 107.4e), which the corpus writes as the two letters run
                // together and this side writes with the slash the card
                // prints. The two have to be *different* letters: `{U/U}` is
                // not a symbol, and `ColorPair::new` asserts as much. Nothing
                // in the reference writes a doubled pair, which is checked
                // rather than assumed — a doubled one would be a token this
                // rule quietly turned into a panic at compile time.
                //
                // Strictly two colours, so `2W` and `WP` keep refusing by
                // name: a `{2/W}` costs *two* generic as its other half and a
                // `{W/P}` is paid with life, and neither is what a reader
                // that saw "two letters" would have written.
                mana.push_str(&pair);
            } else {
                let head = token.split('<').next().unwrap_or(token);
                return self.deny(format!("cost `{head}`"));
            }
        }
        Some((mana, parts))
    }

    /// One cost part the player pays by naming an object.
    ///
    /// Four spellings of one shape — `Sac<1/…>`, `Discard<1/…>`,
    /// `tapXType<1/…>`, `Return<1/…>` — and they were worth writing once
    /// rather than four times because what differs between them is two
    /// facts: which `CostPart` they are, and whether the object has to be
    /// one the payer controls.
    ///
    /// **One object per part**, which is what `CostPart` carries: 97 of the
    /// corpus's costs sacrifice two, three or X, and paying one of them
    /// would be a discount rather than the cost the card prints.
    ///
    /// "You control" is added where the script does not say it, which is the
    /// one place this reader writes a clause it did not read — and only for
    /// the three that take a permanent. CR 701.21a lets a player sacrifice
    /// only what they control and CR 118.3 says the same of tapping one to
    /// pay, so a filter without it would be the card and
    /// `cost_wizard::options` offering two different menus for one cost. A
    /// **discard** takes none of it: a card in a hand has no controller at
    /// all, and the engine reads that zone by whose hand it is.
    fn object_cost_part(&mut self, kind: &str, body: &str, token: &str) -> Option<String> {
        let mut fields = body.splitn(3, '/');
        let (Some(n), Some(spec)) = (fields.next(), fields.next()) else {
            return self.deny(format!("cost `{token}`"));
        };
        if n != "1" {
            return self.deny(format!("a cost naming `{n}` objects"));
        }
        let (variant, controlled) = match kind {
            "Sac" => ("Sacrifice", true),
            "Discard" => ("Discard", false),
            "tapXType" => ("TapOther", true),
            _ => ("ReturnToHand", true),
        };
        if spec == "CARDNAME" {
            // The source pays for itself, which is a part with no filter and
            // no question. `Sac<1/CARDNAME>` never reaches here (it is
            // matched one branch up), so this is the return's own case.
            return Some(match variant {
                "ReturnToHand" => "ReturnSelfToHand".to_string(),
                _ => return self.deny(format!("cost `{token}` naming itself")),
            });
        }
        let spec = if controlled {
            // Appended per alternative, because a valid-string's commas are
            // `Or` and a clause glued to the end would narrow only the last
            // branch.
            spec.split(',')
                .map(|alt| {
                    if alt.contains("YouCtrl") {
                        alt.to_string()
                    } else if alt.contains('.') {
                        format!("{alt}+YouCtrl")
                    } else {
                        format!("{alt}.YouCtrl")
                    }
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            spec.to_string()
        };
        let expr = self.filter_expr(&spec)?;
        let name = self.body.filter_static("COST", &expr);
        Some(format!("{variant}(&{name})"))
    }

    /// `T:Mode$ SpellCast` — "whenever a player casts a spell".
    ///
    /// One printed sentence and two script keys, because the reference asks
    /// *what* was cast and *who* cast it separately. `Trigger::SpellCast`
    /// carries one filter, which is the right shape — a spell on the stack
    /// is controlled by the player who cast it — so the two keys are joined
    /// by appending the controller atom to every alternative and letting
    /// [`Tx::filter_expr`] compose it: `Instant,Sorcery` with `You` is
    /// `Instant.YouCtrl,Sorcery.YouCtrl`. The atom is spelled the way a
    /// script would have spelled it rather than built here, so the two
    /// spellings cannot drift.
    fn spell_cast_trigger(&mut self, p: &mut Params, zoned: bool) -> Option<String> {
        // **An absent `TriggerZones$` is not the battlefield here.** 114 of
        // the corpus's 1444 `SpellCast` lines write no zone at all, and 98
        // of them are `ValidCard$ Card.Self` — "when you cast this spell",
        // which fires while the card is on the *stack*. This engine collects
        // triggers off the battlefield, so reading one of those as an
        // ordinary trigger is a card whose ability can never fire under a
        // `Coverage::Implemented` that says otherwise. The zone is therefore
        // demanded rather than defaulted, which is the opposite of what the
        // other modes may do.
        if !zoned {
            return self.deny("a `SpellCast` trigger with no `TriggerZones$`".to_string());
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `SpellCast` trigger with no `ValidCard$`".to_string());
        };
        // The same sentence, refused a second way: a cast trigger
        // is one whatever zone it claims.
        if valid.split(['.', '+']).any(|atom| atom == "Self") {
            return self.deny("a `SpellCast` trigger on the card itself".to_string());
        }
        // An absent `ValidActivatingPlayer$` is every player, which
        // is `Player`'s own meaning — 220 lines write nothing and
        // 39 write the word.
        let whose = match p.take("ValidActivatingPlayer").as_deref() {
            None | Some("Player") => "",
            Some("You") => ".YouCtrl",
            Some("Opponent" | "Player.Opponent") => ".OppCtrl",
            Some(other) => {
                return self.deny(format!("a spell cast by `{other}`"));
            }
        };
        let valid = if whose.is_empty() {
            valid
        } else {
            valid
                .split(',')
                .map(|alt| format!("{alt}{whose}"))
                .collect::<Vec<_>>()
                .join(",")
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        Some(format!("Trigger::SpellCast(&{filter})"))
    }

    /// `T:Mode$ …` as a `Trigger` expression.
    fn trigger_expr(&mut self, p: &mut Params, mode: &str) -> Option<String> {
        // Where the ability triggers **from**. This used to be taken and
        // thrown away, which is the one thing this reader is not allowed to
        // do: `AbilityDef::Triggered` carries no zone, the engine collects
        // triggers off the battlefield (plus CR 603.10's look-back and the
        // command zone's emblems), so a `TriggerZones$ Graveyard` read as an
        // ordinary trigger is a card whose ability can never fire and a
        // `Coverage::Implemented` that says otherwise.
        //
        // Five corpus scripts were being read this way, and all five are
        // Vanguard avatars whose trigger fires from the command zone:
        // Fallen Angel, Gerrard, Rofellos, Royal Assassin and Rumbling Slum.
        // None of them is a card this pool compiles — `Vanguard` is not a
        // card type here — so nothing in the tree changes, which is what
        // makes this the cheapest possible moment to shut the door.
        let zones = p.take("TriggerZones");
        match zones.as_deref() {
            None | Some("Battlefield") => {}
            Some(zones) => return self.deny(format!("`TriggerZones$ {zones}`")),
        }
        match mode {
            "ChangesZone" => {
                let origin = p.take("Origin");
                let dest = p.take("Destination");
                let valid = p.take("ValidCard");
                let (Some(origin), Some(dest), Some(valid)) = (origin, dest, valid) else {
                    return self
                        .deny("a `ChangesZone` trigger missing one of its three keys".to_string());
                };
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                match (origin.as_str(), dest.as_str()) {
                    // `Trigger::ETB` is the same bytes as the variant with
                    // `&Filter::This` in it, and is what the DSL carries the
                    // constant for: ninety-nine of the pool's hundred and ten
                    // enter-triggers point at the source, so the long form is
                    // the rare one and deserves to look rare.
                    ("Any", "Battlefield") if filter == "&Filter::This" => {
                        Some("Trigger::ETB".to_string())
                    }
                    ("Any", "Battlefield") => Some(format!("Trigger::EntersBattlefield({filter})")),
                    ("Battlefield", "Graveyard") => Some(format!("Trigger::Dies({filter})")),
                    _ => self.deny(format!("trigger on a move from {origin} to {dest}")),
                }
            }
            "Phase" => {
                let Some(phase) = p.take("Phase") else {
                    return self.deny("a `Phase` trigger with no `Phase$`".to_string());
                };
                let step = match phase.as_str() {
                    "Upkeep" => "StepKind::Upkeep",
                    "Draw" => "StepKind::Draw",
                    "BeginCombat" => "StepKind::CombatBegin",
                    "End of Turn" => "StepKind::End",
                    other => return self.deny(format!("trigger at step `{other}`")),
                };
                let valid = p.take("ValidPlayer");
                let Some(whose) = Self::player_rel(valid.as_deref()) else {
                    return self.deny(format!(
                        "trigger for player `{}`",
                        valid.unwrap_or_default()
                    ));
                };
                Some(format!(
                    "Trigger::StepBegin {{ step: {step}, whose: {whose} }}"
                ))
            }
            "Attacks" => {
                let Some(valid) = p.take("ValidCard") else {
                    return self.deny("an `Attacks` trigger with no `ValidCard$`".to_string());
                };
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                Some(format!("Trigger::Attacks({filter})"))
            }
            "SpellCast" => self.spell_cast_trigger(p, zones.is_some()),
            "Taps" => {
                let valid = p.take("ValidCard").unwrap_or_default();
                if valid == "Card.Self" {
                    Some("Trigger::BecomesTapped(&Filter::This)".to_string())
                } else {
                    self.deny(format!("a `Taps` trigger on `{valid}` rather than itself"))
                }
            }
            other => self.deny(format!("trigger mode `{other}`")),
        }
    }

    /// One `K:` line, as whichever of the four things it is.
    ///
    /// A keyword line is the corpus's catch-all and this is the one place
    /// that says so: a bit on the face, a static ability CR 613.11 makes of
    /// it, an as-it-enters modifier, or a whole activated ability the rules
    /// define for the word (CR 702.6a). Reading it here rather than in
    /// [`transcode`]'s loop is what lets a rule need the card's `SVar`s, its
    /// subtype catalogs and a cost parser — and what leaves exactly one
    /// answer to "was this line read", which two loops used to guess at
    /// separately and disagree about.
    /// `K:ETBReplacement:Other:<svar>` — "as this enters, …".
    ///
    /// The keyword is a *pointer*: what actually happens is the `SVar` it
    /// names, and the transcoder reads exactly one of them so far. `Other`
    /// is the ordinary as-it-enters replacement; `Copy` is a clone choosing
    /// what to come down as, which is a different mechanism and is refused
    /// by not being this.
    ///
    /// Every card in the reference writes the choice with `Defined$ You`,
    /// and this insists on it rather than ignoring it: "as this enters,
    /// choose a color" is a choice its *controller* makes, and a card that
    /// handed it to somebody else would be a different sentence.
    fn etb_replacement(&mut self, rest: &str) -> Option<()> {
        let mut fields = rest.split(':');
        let kind = fields.next()?.trim();
        if kind != "Other" {
            return self.deny(format!("`ETBReplacement:{kind}`"));
        }
        let Some(svar) = fields.next().map(str::trim) else {
            return self.deny("an `ETBReplacement` naming no ability".to_string());
        };
        let Some(body) = self.svars.get(svar) else {
            return self.deny(format!("an `ETBReplacement` naming the missing `{svar}`"));
        };
        let Some((api, mut p)) = Params::parse(body) else {
            return self.deny("an `ETBReplacement` ability with no `$` in it".to_string());
        };
        if api != "ChooseColor" {
            return self.deny(format!("as-enters effect `{api}`"));
        }
        p.drop_prose();
        match p.take("Defined").as_deref() {
            Some("You") => {}
            other => {
                return self.deny(format!(
                    "a colour chosen by `{}`",
                    other.unwrap_or("nobody")
                ));
            }
        }
        let modifier = match p.take("Exclude") {
            None => "EnterModifier::ChooseColor".to_string(),
            Some(name) => {
                let Some(color) = color_word(&name) else {
                    return self.deny(format!("a colour choice excluding `{name}`"));
                };
                format!("EnterModifier::ChooseColorExcept({color})")
            }
        };
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `ChooseColor.{key}`"));
        }
        self.body.enter_modifiers.push(modifier);
        Some(())
    }

    fn keyword(&mut self, line: &str) -> Option<()> {
        if let Some(modifier) = keyword_static(line) {
            self.body
                .abilities
                .push(Self::static_expr("Filter::This", modifier));
            return Some(());
        }
        if let Some(entry) = keyword_enter_modifier(line, self.svars) {
            self.body.enter_modifiers.push(entry);
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("ETBReplacement:") {
            return self.etb_replacement(rest);
        }
        if let Some(rest) = line.strip_prefix("Equip:") {
            return self.equip(rest);
        }
        if let Some(rest) = line.strip_prefix("Cycling:") {
            return self.cycling(rest);
        }
        if let Some(rest) = line.strip_prefix("Enchant:") {
            return self.enchant(rest);
        }
        if let Some(bit) = keyword_const(line) {
            self.body.keywords.push(bit.to_string());
            return Some(());
        }
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        self.deny(format!("keyword `{head}`"))
    }

    /// `K:Equip:<cost>` as the activated ability the keyword is.
    ///
    /// Equip prints one thing and the rules supply the rest (CR 702.6a):
    /// sorcery speed, "target creature you control", and attaching this
    /// permanent to it. `equip!` is that sentence, so the cost is all there
    /// is to read — and reading it through [`Self::cost_expr`] rather than
    /// as mana means the eight scripts that equip for a sacrifice, a
    /// discard or three life are the same rule as the 543 that equip for
    /// mana.
    ///
    /// A fourth field refuses. `K:Equip:1:Creature.Legendary+YouCtrl` is
    /// "equip legendary creature", which narrows the target the keyword
    /// otherwise defines — and `equip!` has no room for it precisely
    /// because the rules fill that room. Writing the card without the
    /// restriction would let it move onto anything.
    fn equip(&mut self, rest: &str) -> Option<()> {
        if rest.contains(':') {
            return self.deny("an `Equip` that narrows what it may attach to".to_string());
        }
        let cost = self.cost_expr(rest)?;
        self.body.abilities.push(format!("equip!({cost})"));
        Some(())
    }

    /// `K:Cycling:<cost>` as the activated ability the keyword is.
    ///
    /// CR 702.29a in full: "Cycling [cost]" means "[cost], Discard this card:
    /// Draw a card", activated from the hand. Nothing here is new to the DSL
    /// — [`crate::landgen`] has written that sentence from the printed text
    /// since the cycling lands — and this emits the same string through the
    /// same [`crate::body::cost_literal`], so a Desert comes out of either
    /// reader byte for byte the same.
    ///
    /// The cost is read by [`Self::cost_pieces`] rather than as mana, which
    /// is the `cost 'Sac'` lesson once more: the reference writes 57 distinct
    /// costs across these 306 lines and two of them are not mana at all
    /// (`PayLife<2>`, `Sac<1/Land>`). One reader answers every spelling, and
    /// `1 U` is the same sentence as `{1}{U}` to it.
    ///
    /// A fourth field refuses **by name**. All 306 lines carry three fields
    /// today, so a fourth is a sentence nobody here has read — and cycling is
    /// exactly where a guess would be invisible, because the keyword supplies
    /// everything the card does not print. `K:TypeCycling` is a different
    /// sentence with its own arm to come (101 lines, four fields).
    fn cycling(&mut self, rest: &str) -> Option<()> {
        if let Some((_, extra)) = rest.split_once(':') {
            return self.deny(format!("a `Cycling` with a fourth field `{extra}`"));
        }
        let (mana, mut parts) = self.cost_pieces(rest)?;
        parts.push("DiscardSelf".to_string());
        let cost = crate::body::cost_literal(&mana, &parts);
        self.body.abilities.push(format!(
            "activated!({cost}, &[Effect::draw(1)], zone = ActivationZone::Hand)"
        ));
        self.body.notes.push("cycling".to_string());
        Some(())
    }

    /// `K:Enchant:<valid>` as the spell an Aura card is.
    ///
    /// Enchant is a static ability of the *spell* (CR 702.5b): it says what
    /// the Aura targets as it is cast (CR 303.4a), and the Aura arrives on
    /// the battlefield already attached to that permanent. One `spell!`
    /// carrying `Effect::AttachSelf` is all of that, and it is the shape
    /// the pool's hand-written Auras already have.
    ///
    /// The target is named twice — once as what the spell may aim at and
    /// once as what the effect attaches to — which is one `static` in the
    /// generated file, because `filter_static` gives a filter written twice
    /// in a card a single name.
    ///
    /// **An Aura on a player is refused.** `Effect::AttachSelf` reads the
    /// resolution's first target as an object, so `K:Enchant:Player` would
    /// generate a card that resolves, attaches to nothing, and is put into
    /// its owner's graveyard by the next state-based action (CR 704.5m).
    /// The corpus prints it 50 times, and each is a card this engine cannot
    /// yet say rather than one it may guess at.
    ///
    /// A third field is the printed wording — "creature you control" — and
    /// this side takes the printed wording from Scryfall, so it is prose.
    fn enchant(&mut self, rest: &str) -> Option<()> {
        let valid = rest.split(':').next().unwrap_or(rest).trim();
        if matches!(valid, "Player" | "Opponent") {
            return self.deny(format!("an `Enchant {valid}`, which attaches to no object"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("ENCHANT", &expr);
        self.body.abilities.push(format!(
            "spell!(&[Effect::AttachSelf {{ target: TargetSpec::Object(&{name}) }}], \
             targets = Some(TargetReq::one(TargetSpec::Object(&{name}))))"
        ));
        Some(())
    }

    fn rule(&mut self, kind: char, spec: &str) -> Option<()> {
        self.has_x = kind == 'A';
        match kind {
            'A' => self.activated_or_spell(spec),
            'T' => self.triggered(spec),
            'S' => self.static_ability(spec),
            'R' => self.replacement(spec),
            other => self.deny(format!("rules line kind `{other}:`")),
        }
    }

    /// An `R:` replacement, for the one shape the engine models as data.
    ///
    /// "Enters tapped" is a replacement effect in the corpus and an
    /// `EnterModifier` here, and the difference matters: a modifier is read
    /// *as the permanent enters*, which is what CR 614.1c describes and what
    /// stops the land from being tapped a moment after it arrives untapped.
    /// Every other `Moved` replacement is a rule of its own and refuses.
    fn replacement(&mut self, spec: &str) -> Option<()> {
        let Some((event, mut p)) = Params::parse(spec) else {
            return self.deny("an `R:` line with no `$` in it".to_string());
        };
        if event == "Untap" {
            return self.does_not_untap(&mut p);
        }
        if event != "Moved" {
            self.note(format!("replacement `R: Event$ {event}`"));
            return None;
        }
        p.drop_prose();
        // Anything but the card itself entering the battlefield is a
        // different effect ("whenever another creature enters…").
        let about_self = p.take("ValidCard").as_deref() == Some("Card.Self");
        let entering = p.take("Destination").as_deref() == Some("Battlefield");
        // `Updated` means the event still happens, changed. `Prevented` and
        // the rest replace it with something else entirely.
        let updated = p.take("ReplacementResult").as_deref() == Some("Updated");
        let Some(with) = p.take("ReplaceWith") else {
            return self.deny("replacement `Moved` with no `ReplaceWith$`".to_string());
        };
        if !about_self || !entering || !updated || !p.exhausted() {
            self.note("replacement `Moved` this rule cannot read".to_string());
            return None;
        }
        let Some((api, mut body)) = self.svars.get(&with).and_then(|s| Params::parse(s)) else {
            return self.deny(format!("`ReplaceWith$ {with}` names no readable SVar"));
        };
        body.drop_prose();
        // `DB$ Tap | Defined$ Self | ETB$ True`: the tap has to be of this
        // card, as it enters, or it is not this modifier.
        if api != "Tap"
            || body.take("Defined").as_deref() != Some("Self")
            || body.take("ETB").as_deref() != Some("True")
        {
            self.note(format!("replacement `Moved` replacing with `{api}`"));
            return None;
        }
        // A land prints one of three sentences about coming down tapped, and
        // the reference writes all three on this one line: unconditionally,
        // *unless you control* enough of something, or *unless you pay*. They
        // are three `EnterModifier` variants, so they are read as three
        // shapes rather than one with options, and a line claiming both a
        // count and a cost is neither of them.
        let modifier = match (body.take("ConditionPresent"), body.take("UnlessCost")) {
            (Some(_), Some(_)) => {
                self.note("replacement `Moved` both counting and charging".to_string());
                return None;
            }
            (None, Some(cost)) => self.enters_tapped_or_pays(&mut body, &cost)?,
            (Some(present), None) => self.enters_tapped_unless(&mut body, &present)?,
            // A fourth sentence, and it arrives through a different key
            // because it counts something no `IsPresent$` filter reaches: a
            // number of *players*. Without one of those the line is the
            // plain "enters tapped".
            (None, None) => match body.take("ConditionCheckSVar") {
                None => "EnterModifier::Tapped".to_string(),
                Some(svar) => self.enters_tapped_unless_players(&mut body, &svar)?,
            },
        };
        if !body.exhausted() {
            self.note(format!(
                "replacement `Moved` tapping with `{}`",
                body.first_key().unwrap_or_default()
            ));
            return None;
        }
        self.body.enter_modifiers.push(modifier);
        Some(())
    }

    /// "You may pay N life. If you don’t, this enters tapped" — the
    /// shocklands, 26 of the corpus’s `Moved` replacements, and
    /// [`EnterModifier::TappedOrPayLife`] says it exactly. Steam Vents is
    /// hand-written in this pool as `TappedOrPayLife(2)` against the
    /// script’s `PayLife<2>`, which is the reading checked against a
    /// printed card rather than argued from the key’s name.
    ///
    /// `PayLife<N>` and nothing else: the other five `UnlessCost$` values on
    /// these lines reveal a card instead (Rustic Clachan’s Kithkin),
    /// which this modifier has nowhere to carry. And the payer has to be the
    /// land’s own controller, because that is the only player
    /// `TappedOrPayLife` can ask.
    fn enters_tapped_or_pays(&mut self, body: &mut Params, cost: &str) -> Option<String> {
        let life = cost
            .strip_prefix("PayLife<")
            .and_then(|rest| rest.strip_suffix('>'))
            .and_then(|n| n.parse::<u16>().ok());
        let Some(life) = life else {
            self.note(format!("replacement `Moved` charging `{cost}`"));
            return None;
        };
        match body.take("UnlessPayer").as_deref() {
            Some("You") => {}
            other => {
                self.note(format!(
                    "replacement `Moved` charging `{}`",
                    other.unwrap_or("nobody")
                ));
                return None;
            }
        }
        Some(format!("EnterModifier::TappedOrPayLife({life})"))
    }

    /// "This enters tapped unless you control N or more …".
    ///
    /// **The comparison is on the tap, and the card prints the opposite.**
    /// The reference says when the land comes down *tapped* — Rockfall Vale
    /// is `ConditionCompare$ LT2` over `Land.YouCtrl` and prints "enters
    /// tapped unless you control two or more other lands" — so `LT n` is
    /// `at_least = n` and `LE n` is `at_least = n + 1`. Canopy Vista settles
    /// that the arithmetic is right rather than plausible: its script writes
    /// `LE1` and its hand-written card in this pool writes `at_least: 2`.
    ///
    /// "Other" needs no clause. `controls_at_least` skips the entering
    /// object itself, so `Land.YouCtrl` counts the other lands whether or
    /// not the script spells `+Other` — and the ten that do spell it emit a
    /// redundant `Filter::Another` rather than a wrong count.
    ///
    /// `GT` and `GE` are the **upper** bound and come out as
    /// [`EnterModifier::TappedUnlessAtMost`]: `GT n` is `at_most = n` and
    /// `GE n` is `n - 1`. One predicate, and the corpus writes it from both
    /// ends — a fast land prints the bound ("unless you control two or fewer
    /// other lands", `GT2`, ten scripts) and the Forgotten Realms manlands
    /// print the complement ("if you control two or more other lands, this
    /// land enters tapped", `GE2` on three of them and `GT1` on the other
    /// two). Fifteen scripts, three spellings, one `at_most`.
    ///
    /// `GE0` is refused rather than read as `at_most` underflowing: a land
    /// that is tapped whatever the board is not this sentence, and the
    /// corpus writes it nowhere.
    fn enters_tapped_unless(&mut self, body: &mut Params, present: &str) -> Option<String> {
        let Some(cmp) = body.take("ConditionCompare") else {
            self.note("replacement `Moved` counting with no `ConditionCompare$`".to_string());
            return None;
        };
        let lt = cmp.strip_prefix("LT").and_then(|n| n.parse::<u16>().ok());
        let le = cmp
            .strip_prefix("LE")
            .and_then(|n| n.parse::<u16>().ok())
            .and_then(|n| n.checked_add(1));
        let gt = cmp.strip_prefix("GT").and_then(|n| n.parse::<u16>().ok());
        let ge = cmp
            .strip_prefix("GE")
            .and_then(|n| n.parse::<u16>().ok())
            .and_then(|n| n.checked_sub(1));
        let bound = match (cmp.as_str(), lt, le, gt, ge) {
            ("EQ0", ..) => Bound::AtLeast(1),
            (_, Some(n), _, _, _) | (_, None, Some(n), _, _) if n >= 1 => Bound::AtLeast(n),
            (_, _, _, Some(n), _) | (_, _, _, None, Some(n)) => Bound::AtMost(n),
            _ => {
                self.note(format!(
                    "an enter-tapped condition `{cmp}` this rule cannot read"
                ));
                return None;
            }
        };
        let expr = self.filter_expr(present)?;
        let name = self.body.filter_static("CHECK", &expr);
        Some(match bound {
            // One spelling for one sentence: "unless you control at least
            // one" is what a checkland prints, and `TappedUnless` is the
            // variant that says it. Emitting `TappedUnlessCount { at_least:
            // 1 }` beside it would give that sentence a second form nothing
            // but a hash could tell from the first.
            Bound::AtLeast(1) => format!("EnterModifier::TappedUnless(&{name})"),
            Bound::AtLeast(n) => {
                format!("EnterModifier::TappedUnlessCount {{ filter: &{name}, at_least: {n} }}")
            }
            Bound::AtMost(n) => {
                format!("EnterModifier::TappedUnlessAtMost {{ filter: &{name}, at_most: {n} }}")
            }
        })
    }

    /// The two enters-tapped sentences whose condition counts **players**.
    ///
    /// `ConditionCheckSVar$` points at an `SVar` and `ConditionSVarCompare$`
    /// says when the *tap* happens, so both readings are the card's sentence
    /// turned around — the same inversion [`Self::enters_tapped_unless`]
    /// documents, and the reason each direction is spelled out here rather
    /// than shared:
    ///
    /// * `PlayerCountOpponents$Amount` with `LT2` taps while you have fewer
    ///   than two opponents, which is "unless you have two or more
    ///   opponents": `LT n` is `at_least = n`, `LE n` is `n + 1`.
    /// * `PlayerCountPlayers$LowestLifeTotal` with `GT13` taps while the
    ///   *lowest* life total at the table is above thirteen, which is
    ///   "unless a player has 13 or less life": `GT n` is `life = n`, `GE n`
    ///   is `n - 1`.
    ///
    /// The two take **opposite** comparators, and that is used rather than
    /// tolerated: a count accepts only `LT`/`LE` and a life total only
    /// `GT`/`GE`, so a pairing this rule has never seen is refused instead
    /// of being read with the direction silently flipped. Together they are
    /// 20 of the reference's 27 conditional enters-tapped lines; the other
    /// seven count something else and are refused by the definition not
    /// matching.
    fn enters_tapped_unless_players(&mut self, body: &mut Params, svar: &str) -> Option<String> {
        let Some(defn) = self.svars.get(svar).cloned() else {
            self.note(format!("`ConditionCheckSVar$ {svar}` names no SVar"));
            return None;
        };
        let Some(cmp) = body.take("ConditionSVarCompare") else {
            self.note("replacement `Moved` counting an SVar with no comparison".to_string());
            return None;
        };
        let value = |prefix: &str| cmp.strip_prefix(prefix).and_then(|n| n.parse::<i64>().ok());
        let modifier = match defn.trim() {
            "PlayerCountOpponents$Amount" => {
                let at_least = match (value("LT"), value("LE")) {
                    (Some(n), _) => n,
                    (None, Some(n)) => n + 1,
                    (None, None) => return self.unread_enter_condition(&defn, &cmp),
                };
                let Ok(at_least) = u8::try_from(at_least) else {
                    return self.unread_enter_condition(&defn, &cmp);
                };
                format!("EnterModifier::TappedUnlessOpponents {{ at_least: {at_least} }}")
            }
            "PlayerCountPlayers$LowestLifeTotal" => {
                let life = match (value("GT"), value("GE")) {
                    (Some(n), _) => n,
                    (None, Some(n)) => n - 1,
                    (None, None) => return self.unread_enter_condition(&defn, &cmp),
                };
                let Ok(life) = i32::try_from(life) else {
                    return self.unread_enter_condition(&defn, &cmp);
                };
                format!("EnterModifier::TappedUnlessSomeoneAtOrBelow {{ life: {life} }}")
            }
            _ => return self.unread_enter_condition(&defn, &cmp),
        };
        Some(modifier)
    }

    /// One refusal for both halves above, so the report names the *count*
    /// rather than the letter the corpus happened to write.
    fn unread_enter_condition(&mut self, defn: &str, cmp: &str) -> Option<String> {
        self.note(format!(
            "an enter-tapped condition counting `{defn}` `{cmp}`"
        ));
        None
    }

    /// `R:Event$ Untap | … | Layer$ CantHappen` as
    /// `Modifier::DoesNotUntap`.
    ///
    /// **A static ability and not a replacement**, although the reference
    /// writes it on an `R:` line. CR 502.3 makes untapping a turn-based
    /// action whose *determination* an effect changes, and CR 613.11 calls
    /// that a continuous effect modifying a game rule — there is no event
    /// being replaced with another. `Layer$ CantHappen` is the reference's
    /// own word for the same thing, and requiring it is what keeps the two
    /// scripts that really do replace the untap (with a counter removal)
    /// out of this rule.
    ///
    /// 157 scripts print the line and 136 are exactly this shape. Every key
    /// is claimed or the card is refused, which is what leaves the rest
    /// out: `IsPresent$` is a static that is only sometimes on,
    /// `CheckSVar$` a computed condition, `ReplaceWith$` a real
    /// replacement, and `Secondary$` a reference-side marker that is not on
    /// the prose list and will not be put there to move a number.
    fn does_not_untap(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        // The battlefield is where a permanent untaps, and it is CR 113.6's
        // default — written out on 96 of the scripts and left off the other
        // 40. `Command` is Kaito's, which is a different card entirely.
        if let Some(zone) = p.take("ActiveZones")
            && zone != "Battlefield"
        {
            self.note(format!("a `doesn't untap` from `ActiveZones$ {zone}`"));
            return None;
        }
        // Whose untap step. Every printing says "your", and the variant is
        // documented as the effect controller's — so anything else here is
        // a rule this side cannot say rather than one it may assume.
        match p.take("ValidStepTurnToController").as_deref() {
            Some("You") => {}
            other => {
                return self.deny(format!(
                    "a `doesn't untap` during `{}`",
                    other.unwrap_or("any untap step")
                ));
            }
        }
        match p.take("Layer").as_deref() {
            Some("CantHappen") => {}
            other => {
                return self.deny(format!(
                    "an untap replacement on layer `{}`",
                    other.unwrap_or("none")
                ));
            }
        }
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `doesn't untap` with no `ValidCard$`".to_string());
        };
        // Inlined rather than hoisted into a `static`, the way the `S:` path
        // one function down does it: `static_ability!` is a `const fn` over
        // a `Filter` *value*, and a `static` item cannot be read in a const
        // context.
        let filter = if valid == "Card.Self" {
            "Filter::This".to_string()
        } else {
            self.filter_expr(&valid)?
        };
        if !p.exhausted() {
            self.note(format!(
                "a `doesn't untap` with `{}`",
                p.first_key().unwrap_or_default()
            ));
            return None;
        }
        self.body
            .abilities
            .push(Self::static_expr(&filter, "Modifier::DoesNotUntap"));
        Some(())
    }

    /// An `S: Mode$ Continuous` line as one or more `AbilityDef::Static`.
    ///
    /// One printed sentence can be several continuous effects: "get +1/+1
    /// and have flying" changes power/toughness in layer 7c and abilities
    /// in layer 6, and CR 613.1 applies those in order. The corpus writes both
    /// on one line, so this emits one `StaticAbility` per layer touched
    /// rather than trying to fold them into one.
    fn static_ability(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("an `S:` line with no `$` in it".to_string());
        };
        if mode != "Continuous" {
            self.note(format!("static ability `S: Mode$ {mode}`"));
            return None;
        }
        p.drop_prose();
        // `EffectZone$ Battlefield` is the default written out; any other
        // zone means the source works from somewhere else, which is a
        // different rule than the one below.
        if let Some(zone) = p.take("EffectZone")
            && zone != "Battlefield"
        {
            self.note(format!("static ability from `EffectZone$ {zone}`"));
            return None;
        }
        // Likewise `AffectedZone`: reaching past the battlefield is said by
        // a `Filter::InZone` in the filter, and a filter that has no zone
        // predicate in it cannot say *which* other zone. Refuse rather than
        // guess.
        if let Some(zone) = p.take("AffectedZone")
            && zone != "Battlefield"
        {
            self.note(format!("static ability reaching `AffectedZone$ {zone}`"));
            return None;
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("a continuous static with no `Affected$`".to_string());
        };
        let filter = self.filter_expr(&affected)?;
        let mut out = Vec::new();
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        // The honest-stub rule: one key nothing claimed and the card stays
        // a stub, however much of the line was understood.
        if !p.exhausted() || out.is_empty() {
            if let Some(key) = p.first_key() {
                self.note(format!("unclaimed parameter `Continuous.{key}`"));
            } else {
                self.note("a continuous static that changes nothing".to_string());
            }
            return None;
        }
        self.body.abilities.extend(out);
        Some(())
    }

    /// `AddPower`/`AddToughness` (layer 7c) and `SetPower`/`SetToughness`
    /// (layer 7b) as static abilities.
    fn pt_modifiers(&self, p: &mut Params, filter: &str, out: &mut Vec<String>) -> Option<()> {
        let add_p = p.take("AddPower");
        let add_t = p.take("AddToughness");
        if add_p.is_some() || add_t.is_some() {
            // An anthem that names only one half still moves the other by
            // zero, which is what the printed "+1/+0" says.
            let (Some(power), Some(tough)) = (
                add_p.map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
                add_t.map_or(Some(0), |v| v.trim().parse::<i16>().ok()),
            ) else {
                self.note("static ability with a computed P/T".to_string());
                return None;
            };
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::ModifyPT({power}, {tough})"),
            ));
        }
        let set_p = p.take("SetPower");
        let set_t = p.take("SetToughness");
        if set_p.is_some() || set_t.is_some() {
            // Setting one half and leaving the other alone is a real card
            // ("base power 4"), and `SetPT` cannot say it — refuse rather
            // than invent a value for the half that was not named.
            let (Some(power), Some(tough)) = (
                set_p.and_then(|v| v.trim().parse::<i16>().ok()),
                set_t.and_then(|v| v.trim().parse::<i16>().ok()),
            ) else {
                self.note("static ability setting one half of P/T".to_string());
                return None;
            };
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::SetPT({power}, {tough})"),
            ));
        }
        Some(())
    }

    /// `AddKeyword`/`RemoveKeyword` (layer 6) as static abilities.
    fn keyword_modifiers(&self, p: &mut Params, filter: &str, out: &mut Vec<String>) -> Option<()> {
        for (key, modifier) in [
            ("AddKeyword", "AddKeyword"),
            ("RemoveKeyword", "RemoveKeyword"),
        ] {
            let Some(raw) = p.take(key) else { continue };
            // A keyword the engine reads as a bit, or nothing: a keyword
            // that carries data ("Enchant creature", "Equip {2}") is an
            // ability, and granting it as a bit would grant a keyword no
            // rule reads.
            let mut bits = Vec::new();
            for word in raw.split(" & ") {
                let Some(bit) = keyword_const(word) else {
                    self.note(format!("static ability granting keyword `{word}`"));
                    return None;
                };
                bits.push(bit.to_string());
            }
            let set = bits.split_first().map(|(head, tail)| {
                tail.iter()
                    .fold(head.clone(), |acc, b| format!("{acc}.union({b})"))
            })?;
            out.push(Self::static_expr(
                filter,
                &format!("Modifier::{modifier}({set})"),
            ));
        }
        Some(())
    }

    /// `AddType`/`RemoveType` (layer 4) as static abilities.
    ///
    /// The corpus writes card types and subtypes in one list and the engine
    /// keeps them apart — a `TypeSet` is a bitmask the rules read, a
    /// subtype is an interned id — so `AddType$ Artifact Goblin` becomes
    /// two modifiers on the same layer.
    fn type_modifiers(&self, p: &mut Params, filter: &str, out: &mut Vec<String>) -> Option<()> {
        for (key, modifier) in [("AddType", "AddType"), ("RemoveType", "RemoveType")] {
            let Some(raw) = p.take(key) else { continue };
            let mut types = Vec::new();
            let mut subtypes = Vec::new();
            for word in raw.split_whitespace() {
                if let Some(t) = card_type_const(word) {
                    types.push(t);
                } else if modifier == "AddType" {
                    subtypes.push(self.cats.const_path(word)?);
                } else {
                    // `Modifier::RemoveType` takes a `TypeSet`, and there is
                    // no "remove one subtype" — refuse rather than drop it.
                    return None;
                }
            }
            if let Some((head, tail)) = types.split_first() {
                let set = tail
                    .iter()
                    .fold((*head).to_string(), |acc, t| format!("{acc}.union({t})"));
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::{modifier}({set})"),
                ));
            }
            for path in subtypes {
                out.push(Self::static_expr(
                    filter,
                    &format!("Modifier::AddSubtype({path})"),
                ));
            }
        }
        Some(())
    }

    /// `AddColor`/`SetColor` (layer 5) as static abilities.
    fn color_modifiers(&self, p: &mut Params, filter: &str, out: &mut Vec<String>) -> Option<()> {
        for (key, modifier) in [("AddColor", "AddColor"), ("SetColor", "SetColor")] {
            let Some(raw) = p.take(key) else { continue };
            let mut colors = Vec::new();
            for word in raw.split_whitespace() {
                colors.push(match word {
                    "White" => "Color::White",
                    "Blue" => "Color::Blue",
                    "Black" => "Color::Black",
                    "Red" => "Color::Red",
                    "Green" => "Color::Green",
                    // `Colorless` is the empty set rather than a colour, and
                    // `ChosenColor` is a choice this rule cannot make.
                    other => {
                        self.note(format!("static ability setting colour `{other}`"));
                        return None;
                    }
                });
            }
            out.push(Self::static_expr(
                filter,
                &format!(
                    "Modifier::{modifier}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
            ));
        }
        Some(())
    }

    /// One `AbilityDef::Static` expression.
    ///
    /// `static_ability!` takes no layer for the same reason [`Self::animate_expr`]
    /// passes none: CR 613.1 makes the layer a function of the modifier, and
    /// `Modifier::layer` is that function.
    fn static_expr(filter: &str, modifier: &str) -> String {
        format!("static_ability!({filter}, {modifier})")
    }

    fn activated_or_spell(&mut self, spec: &str) -> Option<()> {
        let is_activated = spec.starts_with("AB$");
        let Some((_, mut probe)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        probe.drop_prose();
        let cost = probe.take("Cost");
        // "Activate only once each turn" belongs to the ability for the same
        // reason the cost does: it restricts activating it, not what happens
        // when it resolves.
        let limit = probe.take("ActivationLimit");
        // And so does the clause, which on an `A:` line is a restriction on
        // activating rather than CR 603.4's intervening `if`: the reference
        // spells a resolution-time condition `ConditionPresent$`, a
        // different key on a different line (1521 of them, all on `SVar:`),
        // and this reader claims neither it nor its family.
        let condition = self.condition(&mut probe)?;
        let mut chain = Chain::default();
        // The cost belongs to the ability, not to the effect chain, so it is
        // removed from the spec before the chain reads it. The clause's keys
        // go with it, but **only** once the clause was read: a line carrying
        // `PresentZone$` and no `IsPresent$` at all keeps it, and refuses one
        // level down as the unclaimed parameter it is.
        let claimed: &[&str] = if condition.is_empty() {
            &[]
        } else {
            &[
                "IsPresent$",
                "PresentZone$",
                "PresentCompare$",
                "PresentDefined$",
            ]
        };
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$") && !part.starts_with("ActivationLimit$"))
            .filter(|part| !claimed.iter().any(|key| part.starts_with(key)))
            .collect();
        self.chain(&stripped.join(" | "), &mut chain)?;
        if chain.effects.is_empty() {
            return self.deny("an ability that reads as no effect at all".to_string());
        }
        let effects = format!("&[{}]", chain.effects.join(", "));
        if is_activated {
            let Some(cost) = cost else {
                return self.deny("an activated ability with no `Cost$`".to_string());
            };
            let cost = self.cost_expr(&cost)?;
            // CR 605.1a: an activated ability is a mana ability if it could
            // add mana, does not require a target, and is not a loyalty
            // ability. **Could**, not "does nothing else" — this read `all`
            // and so refused every ability with a rider, which is most of
            // the ones that have one: a Talisman's `{T}: Add {U} or {B}.
            // This artifact deals 1 damage to you.`, a painland's, a
            // Chromatic Sphere's `Add one mana of any color. Draw a card.`
            // Five cards in this pool were written as ordinary activated
            // abilities and put their mana on the stack, where an opponent
            // may respond to it — and the ability sheet, which reads the
            // flag to decide whether a press needs arming, asked for a
            // second tap before a land would make mana.
            //
            // The target is the ability's own (`target = Some(…)`) and not
            // a `TargetSpec` inside an effect: "deals 1 damage to you" names
            // a player without targeting one, and Deathrite Shaman, which
            // does target, is the card on the other side of the line.
            let mana_ability =
                chain.effects.iter().any(|e| e.contains("Effect::mana")) && chain.target.is_none();
            let target = chain
                .target
                .map(|t| format!(", target = Some({t})"))
                .unwrap_or_default();
            let limit = match limit {
                None => String::new(),
                Some(n) => {
                    let Ok(n) = n.parse::<u8>() else {
                        // `GE4` and `X` are the corpus's other two spellings
                        // (5 scripts between them) and neither is a count
                        // this side can read: one is a *threshold* on the
                        // counters already spent, the other a number the
                        // board works out.
                        return self.deny(format!("activation limit `{n}`"));
                    };
                    format!(", limit = ActivationLimit::PerTurn({n})")
                }
            };
            // `mana_ability!(effects)` *is* `mana_ability!({T}, effects)`
            // — the macro supplies the tap, because tapping is what almost
            // every mana ability costs. Writing the cost out again says
            // nothing and reads as though this one were the exception.
            //
            // The short form takes the cost as its *only* positional
            // argument, so it is right exactly while there is nothing else
            // to say: `mana_ability!(&[…], limit = …)` would bind the
            // effects where the cost goes and `limit = …` — an assignment
            // expression, and so a legal one — where the effects go.
            let extras = format!("{target}{limit}{condition}");
            let line = match (mana_ability, cost.as_str()) {
                (true, "Cost::TAP") if extras.is_empty() => format!("mana_ability!({effects})"),
                (true, _) => format!("mana_ability!({cost}, {effects}{extras})"),
                (false, _) => format!("activated!({cost}, {effects}{extras})"),
            };
            self.body.abilities.push(line);
        } else {
            if limit.is_some() {
                // A spell is cast, not activated, so there is nothing for the
                // key to restrict and dropping it quietly would be the
                // unclaimed-parameter fault one level down.
                return self.deny("`ActivationLimit$` on a spell line".to_string());
            }
            if !condition.is_empty() {
                // The same, for the same reason: `spell!` has no
                // precondition, and a restriction on casting is a rule this
                // DSL does not have (CR 601.2 has no place for one).
                return self.deny("`IsPresent$` on a spell line".to_string());
            }
            // And the third of them, which this branch dropped in silence
            // until a batch of four hundred cards walked into it. A spell's
            // `Cost$` is its mana cost *plus* whatever else the card charges
            // (CR 601.2b), and only the mana half is on the face — so
            // Kaervek's Spite came out as three mana for "target player
            // loses 5 life", with "sacrifice all permanents you control and
            // discard your hand" nowhere in the card, and Crop Rotation as a
            // one-mana tutor that sacrifices no land. 215 of the 236
            // `A:SP$` lines naming a `Cost$` charge something beside mana.
            //
            // `FaceDef::mandatory_additional_costs` is where they belong and
            // the engine collects them at cast; emitting them is the next
            // step and not this one, because it would walk a dozen cards
            // into a path exactly one card in the pool has ever played
            // (Toxic Deluge's `PayLifeX`). Until then the honest answer is
            // the stub. See #52.
            //
            // The token is read here rather than through `cost_pieces`,
            // which prices an *activation* and denies what it cannot price:
            // routing a spell's cost through it would refuse `Cost$ X G` for
            // its bare `X` — mana the face already carries — and take a card
            // off the list for the one thing that is not wrong with it.
            //
            // So the question asked of each token is the narrow one: is this
            // the card's own mana cost? Everything else refuses, which is an
            // allow-list on purpose. The alternative — refusing the `<…>`
            // spelling every one of those 215 additional costs happens to
            // use — goes silent on a restriction that needs no brackets, and
            // one of those is already here: `XMin1` is "X can't be 0" and
            // not mana at all (Ertai's Meddling and four others, none of
            // them in this pool yet, all five reachable by a batch).
            if let Some(raw) = &cost {
                for token in cost_parts(raw) {
                    let is_mana = token.chars().all(|c| c.is_ascii_digit())
                        || matches!(token.as_str(), "W" | "U" | "B" | "R" | "G" | "C" | "X")
                        || hybrid_pair(&token).is_some();
                    if !is_mana {
                        let head = token.split('<').next().unwrap_or(&token);
                        return self.deny(format!(
                            "`{head}` on a spell line, beside the mana the face carries"
                        ));
                    }
                }
            }
            let targets = chain
                .target
                .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
                .unwrap_or_default();
            self.body
                .abilities
                .push(format!("spell!({effects}{targets})"));
        }
        Some(())
    }

    /// The reference's `IsPresent$` family as an intervening-`if` clause
    /// (CR 603.4).
    ///
    /// What comes back is the macro argument and not the condition: three
    /// answers fit in one `Option<String>` that way — `None` is a line
    /// refused with a reason, an empty string one that prints no clause at
    /// all, and anything else the `, condition = Some(…)` to splice in.
    /// `triggered!`, `activated!` and `mana_ability!` all spell the field
    /// the same, so the next caller needs no second shape.
    ///
    /// The family is seven keys and they are read here once, rather than at
    /// each `Mode$` that might carry them: 605 `T:` lines in the corpus
    /// write `IsPresent$`, spread across every trigger mode there is.
    ///
    /// **Two sentences come out of it**, and which one depends on what the
    /// clause is *about*. `PresentDefined$ Self`, or a valid-string whose
    /// every alternative pins the object with `Self`, is a clause about
    /// this card — `Condition::SourceMatches`. Anything else is a count,
    /// and a count is only readable here when the filter says whose: every
    /// alternative has to carry `YouCtrl`, or the sentence is "there exists
    /// a creature" and `Condition::ControlCount` would answer a narrower
    /// question than the card asks.
    ///
    /// **The zone is written into the filter**, and that is not a
    /// redundancy. `IsPresent$` asks whether an object is present *in a
    /// zone* — `PresentZone$`, defaulting to the battlefield — so the
    /// clause about this card is "this permanent is on the battlefield and
    /// matches", and a filter that left the zone out would be a different
    /// sentence at CR 603.4's **second** check: the source is a battlefield
    /// permanent when the trigger is collected, and need not still be one
    /// when the ability resolves. This engine keeps an object's id across a
    /// zone change, so `SourceMatches` on its own answers for a card that
    /// has died. What is cleared on the way out (CR 400.7) is the half a
    /// permanent has — status, damage, counters — so `Card.tapped` would
    /// have been right by accident, and `Card.Self+YouCtrl` wrong, since a
    /// card in a graveyard keeps the controller it had.
    ///
    /// `NoResolvingCheck$ True` is the one that has to be named rather than
    /// ignored: 61 scripts carry it, and it is the reference opting *out*
    /// of CR 603.4's second check — a clause asked once instead of twice,
    /// which this DSL cannot say at all.
    fn condition(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("IsPresent") else {
            return Some(String::new());
        };
        if p.take("NoResolvingCheck").is_some() {
            return self.deny("`NoResolvingCheck$`, a clause checked once".to_string());
        }
        if p.take("IsPresent2").is_some() {
            return self.deny("a second `IsPresent2$` clause".to_string());
        }
        if let Some(who) = p.take("PresentPlayer") {
            return self.deny(format!("`PresentPlayer$ {who}`"));
        }
        match p.take("PresentZone").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => return self.deny(format!("`PresentZone$ {zone}`")),
        }
        let compare = p
            .take("PresentCompare")
            .unwrap_or_else(|| "GE1".to_string());
        let pins_self = |alt: &str| alt.split(['.', '+']).any(|atom| atom.trim() == "Self");
        let about_source = match p.take("PresentDefined").as_deref() {
            Some("Self") => true,
            Some(other) => return self.deny(format!("`PresentDefined$ {other}`")),
            None => valid.split(',').all(pins_self),
        };
        // Both readings need a number before the filter is worth building,
        // so that a refusal names the clause rather than an atom inside it.
        let count = if about_source {
            if compare != "GE1" {
                return self.deny(format!("a clause about this card compared `{compare}`"));
            }
            None
        } else {
            let Some(n) = compare
                .strip_prefix("GE")
                .and_then(|n| n.parse::<u8>().ok())
            else {
                return self.deny(format!("`PresentCompare$ {compare}`"));
            };
            if !valid
                .split(',')
                .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"))
            {
                return self.deny(format!("`IsPresent$ {valid}`, a count with no player"));
            }
            // And a count says nothing about the card that states it.
            // `eval::condition_holds` walks a battlefield handing each
            // candidate its *own* id as the object a filter's `This` and
            // `Another` compare against, so "another creature you control"
            // would count nothing at all — 28 corpus lines write one, and
            // a trigger that can never fire is exactly the wrong card the
            // honest-stub rule exists to refuse.
            if valid.split(',').any(|alt| {
                alt.split(['.', '+'])
                    .any(|a| matches!(a.trim(), "Self" | "Other"))
            }) {
                return self.deny(format!(
                    "`IsPresent$ {valid}`, a count relative to this card"
                ));
            }
            Some(n)
        };
        let expr = self.filter_expr(&valid)?;
        let clause = match count {
            None => {
                let zoned = on_the_battlefield(&expr);
                let name = self.body.filter_static("CHECK", &zoned);
                format!("Condition::SourceMatches(&{name})")
            }
            // `ControlCount` counts one player's battlefield and nothing
            // else, so the zone and the player are both already in the
            // sentence it is.
            Some(n) => {
                let name = self.body.filter_static("CHECK", &expr);
                format!("Condition::ControlCount(&{name}, {n})")
            }
        };
        Some(format!(", condition = Some({clause})"))
    }

    fn triggered(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("a `T:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let trigger = self.trigger_expr(&mut p, &mode)?;
        let condition = self.condition(&mut p)?;
        let Some(execute) = p.take("Execute") else {
            return self.deny(format!("a `{mode}` trigger with no `Execute$`"));
        };
        // "When …, you may …" (CR 603.5): the ability triggers and goes on
        // the stack whatever its controller intends, and the choice is made
        // as it resolves. That is `Effect::MayDo` exactly, and it is where
        // the pool's hand-written "may" triggers already put it.
        //
        // `You` and nothing else. 1506 of the corpus's 1584
        // `OptionalDecider$` values are `You`, which `Effect::MayDo` asks by
        // construction — the resolver puts the question to the resolving
        // ability's controller. Every other value names somebody else
        // (`TriggeredCardController`, `EnchantedController`, `Opponent`) and
        // no effect here can ask them, so they are refused by name rather
        // than quietly asked of the wrong player.
        let may = match p.take("OptionalDecider").as_deref() {
            None => false,
            Some("You") => true,
            Some(other) => return self.deny(format!("a `may` decided by `{other}`")),
        };
        if !p.exhausted() {
            if let Some(key) = p.first_key() {
                return self.deny(format!("unclaimed parameter `{mode}.{key}`"));
            }
            return None;
        }
        let Some(body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        let mut chain = Chain::default();
        self.chain(&body, &mut chain)?;
        if chain.effects.is_empty() {
            return self.deny("a trigger that reads as no effect at all".to_string());
        }
        let targets = chain
            .target
            .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
            .unwrap_or_default();
        // The targets stay **outside** the `may`, and the two rules say why:
        // CR 603.3d chose them when the ability went on the stack, CR 603.5
        // puts the choice at resolution. A declined "may" is therefore an
        // ability that targeted and then did nothing, which is what hoisting
        // `chain.target` into the macro's own field already gives.
        //
        // And the wrap is around the **whole** list rather than each effect,
        // because the printed word covers a whole clause: Ondu Cleric's "you
        // may gain life equal to the number of Allies you control" is one
        // decision, not one per operation it expands into.
        let effects = chain.effects.join(", ");
        let effects = if may {
            format!("Effect::MayDo {{ effects: &[{effects}] }}")
        } else {
            effects
        };
        self.body.abilities.push(format!(
            "triggered!({trigger}, &[{effects}]{targets}{condition})"
        ));
        Some(())
    }
}

/// A `Cost$` value split into its parts.
///
/// Whitespace separates the parts of a cost — and it also appears *inside*
/// one, because the corpus's bracketed forms carry its own interface prose
/// as their last field: `Sac<1/CARDNAME/this artifact>`,
/// `Return<1/Forest/a Forest>`, `Discard<1/Card/a card>`. A plain
/// `split_whitespace` cuts those in half, and the half that is left over
/// then refuses the card under a cost part nobody wrote, reported as
/// "artifact>".
/// **731** of the reference's costs and 20 of its token scripts' contain such
/// a space, Treasure's among them, and every one of them was being read as
/// two parts.
///
/// So the split happens at bracket depth zero only. Nesting is counted
/// rather than merely flagged, because a cost's brackets do nest —
/// `tapXType<2/Creature.untapped/untapped creature>` is the shallow case and
/// a valid-string carrying its own `<…>` the deeper one — and a boolean
/// would close the first bracket on the innermost `>`.
fn cost_parts(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut part = String::new();
    for c in raw.chars() {
        match c {
            '<' => {
                depth += 1;
                part.push(c);
            }
            '>' => {
                depth = depth.saturating_sub(1);
                part.push(c);
            }
            _ if c.is_whitespace() && depth == 0 => {
                if !part.is_empty() {
                    out.push(std::mem::take(&mut part));
                }
            }
            _ => part.push(c),
        }
    }
    if !part.is_empty() {
        out.push(part);
    }
    out
}

/// A filter expression with "and it is on the battlefield" added to it.
///
/// One `Filter::And` and never one inside another: an already-conjoined
/// expression is spliced open, so `Card.Self+YouCtrl` writes three clauses
/// in a row rather than a pair holding a pair. The splice is safe on the
/// text because `filter_expr` returns one complete expression, so a leading
/// `Filter::And(&[` is closed by the trailing `])` and by nothing else.
fn on_the_battlefield(expr: &str) -> String {
    const ZONE: &str = "Filter::InZone(ZoneRef::Battlefield)";
    match expr
        .strip_prefix("Filter::And(&[")
        .and_then(|rest| rest.strip_suffix("])"))
    {
        Some(clauses) => format!("Filter::And(&[{ZONE}, {clauses}])"),
        None => format!("Filter::And(&[{ZONE}, {expr}])"),
    }
}

/// A reference-script counter code as the `CounterKind` spelling for it.
///
/// Read by two rules that must not disagree: the `PutCounter` *effect* and
/// the `AddCounter<n/KIND>` *cost*. Written twice, `M1M1` could end up
/// meaning one thing on a card's ability and another on its cost, and
/// nothing in the build would notice — the two never meet.
///
/// Only the kinds the rules know are written out here — the nine named ones
/// plus the P/T family below. Every other counter Magic prints is a
/// `CounterKind::Custom` id, and assigning one is a decision with a printed
/// word behind it rather than something a reader may do on the way past. So
/// the ids are **not** assigned here: `baylee_cards_dsl::counters::ASSIGNED`
/// is the registry, this reads it, and a code that is in neither place
/// refuses the card. Deriving the table instead of retyping it is the
/// difference between a registry and two lists of numbers that happen to
/// agree — and the registry is on the other side of the seam anyway, since
/// what a generated card writes is the *constant*, `counters::STORAGE`.
///
/// `Lifelink` is spelled the way it looks. Every other named code here is
/// shouted and a *keyword* counter is written as the keyword (`Flying`,
/// `Indestructible`, and lifelink, which is the one of those the engine reads
/// — `layers.rs` grants the keyword from it). The corpus is not consistent
/// about this beyond the two groups: it prints `Stun` 72 times and `STUN` 22.
/// So a code is matched exactly as the script spells it, and one spelled the
/// other way refuses the card rather than being guessed at.
///
/// `PxPy` and `MxMy` are read by [`pt_counter`] instead of by name, because
/// CR 122.1a is one rule over an open-ended set of pairs: the corpus prints
/// eleven of them and a table of names would go silent on the twelfth.
fn counter_kind(code: &str) -> Option<String> {
    if let Some(pt) = pt_counter(code) {
        return Some(pt);
    }
    Some(
        match code {
            "LOYALTY" => "CounterKind::Loyalty",
            "LORE" => "CounterKind::Lore",
            "TIME" => "CounterKind::Time",
            "CHARGE" => "CounterKind::Charge",
            "POISON" => "CounterKind::Poison",
            "ENERGY" => "CounterKind::Energy",
            "RAD" => "CounterKind::Rad",
            "LEVEL" => "CounterKind::Level",
            "Lifelink" => "CounterKind::Lifelink",
            _ => return assigned_counter(code),
        }
        .to_string(),
    )
}

/// A counter word the DSL has already given a `Custom` id, as the constant
/// that names it.
///
/// The match is on the **word**, case-folded, because the two sides spell it
/// differently on purpose: the registry writes what a player says
/// (`"storage"`), the reference shouts a code (`STORAGE`), and the constant
/// is the word in capitals. `counters::ASSIGNED`'s own test keeps every word
/// lowercase ASCII, which is what makes that last step well defined rather
/// than a guess — and the generated card naming a constant that does not
/// exist would not compile, so the build is the second check.
fn assigned_counter(code: &str) -> Option<String> {
    let word = code.to_ascii_lowercase();
    baylee_cards_dsl::counters::ASSIGNED
        .iter()
        .find(|(assigned, _)| *assigned == word)
        .map(|(assigned, _)| format!("counters::{}", assigned.to_ascii_uppercase()))
}

/// `P1P1`, `M0M1`, `P2P2` — a +X/+Y or -X/-Y counter (CR 122.1a).
///
/// The sign letter is the same on both halves in every one of the eleven
/// codes the corpus prints, which is the rule's own two forms; a mixed pair
/// is refused rather than guessed at, because `CounterKind` cannot say one.
fn pt_counter(code: &str) -> Option<String> {
    let (sign, rest) = code.split_at_checked(1)?;
    let variant = match sign {
        "P" => "Plus",
        "M" => "Minus",
        _ => return None,
    };
    let (power, toughness) = rest.split_once(sign)?;
    let power: u8 = power.parse().ok()?;
    let toughness: u8 = toughness.parse().ok()?;
    // The two Magic prints everywhere are written as the constants that name
    // them. They are the same *value* as the general form — `CounterKind`
    // has one variant for all of them — so this is a spelling and not a
    // second meaning, and it keeps 2528 scripts' worth of cards reading
    // `CounterKind::P1P1` the way a player says it.
    Some(match (variant, power, toughness) {
        ("Plus", 1, 1) => "CounterKind::P1P1".to_string(),
        ("Minus", 1, 1) => "CounterKind::M1M1".to_string(),
        _ => format!("CounterKind::{variant} {{ power: {power}, toughness: {toughness} }}"),
    })
}

/// A script colour word as our `Color` constant.
///
/// `Colorless` is the empty set rather than a colour, and `ChosenColor` is
/// a choice a transcoder cannot make — both stay unread.
fn color_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "White" => "Color::White",
        "Blue" => "Color::Blue",
        "Black" => "Color::Black",
        "Red" => "Color::Red",
        "Green" => "Color::Green",
        _ => return None,
    })
}

/// A type word as the `TypeSet` constant for it, or `None` when the
/// word is a subtype (or a type the engine has no bit for).
fn card_type_const(word: &str) -> Option<&'static str> {
    Some(match word {
        "Artifact" => "TypeSet::ARTIFACT",
        "Creature" => "TypeSet::CREATURE",
        "Enchantment" => "TypeSet::ENCHANTMENT",
        "Instant" => "TypeSet::INSTANT",
        "Kindred" | "Tribal" => "TypeSet::KINDRED",
        "Land" => "TypeSet::LAND",
        "Planeswalker" => "TypeSet::PLANESWALKER",
        "Sorcery" => "TypeSet::SORCERY",
        "Battle" => "TypeSet::BATTLE",
        _ => return None,
    })
}

/// A keyword line → the bit in our `KeywordSet`, for the keywords that
/// are text-independent (CR 702). Parameterized keywords are data, not bits,
/// and are refused here on purpose.
///
/// So are the keywords **no engine rule reads**, which is the same honesty
/// rule one step earlier. A bit the layer system carries and combat never
/// asks about is worse than a stub: the card says `Coverage::Implemented`,
/// the view draws the sheath, the deckbuilder offers it as playable, and the
/// creature is blocked as if it had nothing. The table therefore lists only
/// what `keyword_tests::ENFORCED` lists, and the gate that found this —
/// `no_card_claims_a_keyword_the_engine_ignores` — is what fires when the
/// two drift. Ten entries came off it for that reason: fear, intimidate,
/// shadow, horsemanship, infect, wither, persist, undying, skulk and
/// flanking. Four of them were already on cards; the other six were waiting
/// for the pool to grow into them. A keyword returns here on the commit that
/// gives it a rule, not before.
/// A keyword line → a `static_ability!` on this card, for the one printed
/// sentence the reference files as a keyword and the rules make a static
/// ability.
///
/// "You may choose not to untap CARDNAME during your untap step" is not a
/// keyword at all (CR 702 lists none like it); the reference keeps it on a
/// `K:` line because it has no parameters, which is also why it can be read
/// by matching the whole sentence. It is written exactly one way across the
/// 45 scripts that print it, so the match is the literal string and not a
/// pattern — anything else that ever lands on a `K:` line is a keyword or a
/// refusal, as before.
fn keyword_static(line: &str) -> Option<&'static str> {
    match line.trim() {
        "You may choose not to untap CARDNAME during your untap step." => {
            Some("Modifier::MayChooseNotToUntap")
        }
        _ => None,
    }
}

fn keyword_const(line: &str) -> Option<&'static str> {
    Some(match line.trim() {
        "Flying" => "KeywordSet::FLYING",
        "First Strike" => "KeywordSet::FIRST_STRIKE",
        "Double Strike" => "KeywordSet::DOUBLE_STRIKE",
        "Deathtouch" => "KeywordSet::DEATHTOUCH",
        "Haste" => "KeywordSet::HASTE",
        "Hexproof" => "KeywordSet::HEXPROOF",
        "Indestructible" => "KeywordSet::INDESTRUCTIBLE",
        "Lifelink" => "KeywordSet::LIFELINK",
        "Menace" => "KeywordSet::MENACE",
        "Reach" => "KeywordSet::REACH",
        "Trample" => "KeywordSet::TRAMPLE",
        "Vigilance" => "KeywordSet::VIGILANCE",
        "Defender" => "KeywordSet::DEFENDER",
        "Flash" => "KeywordSet::FLASH",
        "Shroud" => "KeywordSet::SHROUD",
        "Prowess" => "KeywordSet::PROWESS",
        "Changeling" => "KeywordSet::CHANGELING",
        _ => return None,
    })
}

/// `etbCounter:<KIND>:<n>` as the `EnterModifier` it is.
///
/// The third of the three things a `K:` line can be, and the one that is no
/// ability at all: "this permanent enters with N counters on it" is a
/// replacement effect (CR 614.1c), which the DSL says on the face rather
/// than in `abilities`. The reference keeps it on a keyword line because it
/// has no ability body to write.
///
/// Two fields are read and everything else refuses the card. The corpus
/// prints 475 of these lines and this rule accepts 297:
///
/// - `n` is a plain number, or an `SVar` that resolves to one.
/// - `n` is `X` **and** the card's own `SVar:X` is `Count$xPaid` — the X
///   announced as the spell was cast (CR 107.3m), which is the one thing
///   `Amount::X` means. Reading every `X` as that one would be a wrong card
///   rather than a missing one: of the 59 scripts whose counter is `X` under
///   an explicit "no Condition", 57 mean a *count* — creatures in a
///   graveyard, colours of mana spent, lands you control — and Sautekh
///   Immortal means "for each creature that died this turn". Each of those
///   would have generated a body that enters with nothing.
/// - A third field is read only as the literal `no Condition`, which is the
///   reference's own way of saying there is none, and a fourth is the
///   reminder text. Anything else there is a condition the DSL cannot say
///   (`CheckSVar$ WasKicked`, `ValidCard$ Card.Self+escaped`, `Adamant`,
///   `Revolt`), so those lines stay honest stubs rather than becoming cards
///   that place the counter unconditionally. Sautekh Immortal is why the
///   field is matched and not merely counted: its third field is the
///   *description*, and a reader that skipped past it would have taken the
///   `X` beside it for the spell's.
///
/// The counter word goes through [`counter_kind`], so a card whose counter
/// the DSL has no id for refuses here exactly as it does on an ability.
/// A colour spelled as the reference writes it in prose (`Exclude$ white`).
fn color_word(word: &str) -> Option<&'static str> {
    Some(match word.trim().to_ascii_lowercase().as_str() {
        "white" => "ManaColor::White",
        "blue" => "ManaColor::Blue",
        "black" => "ManaColor::Black",
        "red" => "ManaColor::Red",
        "green" => "ManaColor::Green",
        _ => return None,
    })
}

fn keyword_enter_modifier(line: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    let mut fields = line.strip_prefix("etbCounter:")?.split(':');
    let kind = counter_kind(fields.next()?.trim())?;
    let raw = fields.next()?.trim();
    if let Some(condition) = fields.next()
        && !condition.trim().eq_ignore_ascii_case("no condition")
    {
        return None;
    }
    // `true`: an `etbCounter` rides on the permanent's own spell, and a spell
    // announces its `X` — the same number `Amount::X` reads back.
    let amount = amount(raw, svars, true)?;
    Some(format!(
        "EnterModifier::WithCounters {{ kind: {kind}, amount: {amount} }}"
    ))
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
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        tx.keyword(line)?;
    }
    for (kind, spec) in &script.rules {
        tx.rule(*kind, spec)?;
    }
    if tx.body.is_empty() {
        return None;
    }
    tx.body
        .notes
        .push("transcoded from the card's rules".into());
    Some(tx.body)
}

/// Reads a script and, when it is refused over a parameter, names it.
///
/// `None` means the transcoder has nothing to say about this script: it was
/// read in full, or refused somewhere that records no reason.
///
/// The transcoder reports this itself rather than a second table listing
/// each rule's keys: such a list would rot the first time a rule learned a
/// new one, and a stale worklist is worse than none.
#[must_use]
pub fn refusal_reason(
    script: &CardScript,
    cats: &SubtypeCatalogs,
    tokens: Option<&TokenLookup>,
) -> Option<String> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    let mut tx = Tx {
        svars: &script.svars,
        cats,
        tokens,
        has_x: false,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        if tx.keyword(line).is_none() {
            return tx.unclaimed.into_inner();
        }
    }
    for (kind, spec) in &script.rules {
        if tx.rule(*kind, spec).is_none() {
            return tx.unclaimed.into_inner();
        }
    }
    // [`transcode`]'s last refusal, mirrored. A script that is read in full
    // and yields nothing is a card whose rules text this transcoder has no
    // rule for at all — usually a vanilla body, and never a defect — but it
    // is a *reason*, and leaving it out filed every one of them under "no
    // reason recorded".
    if tx.body.is_empty() {
        return Some("a script that reads as an empty card".to_string());
    }
    None
}

/// The effect APIs [`transcode`] knows how to write.
///
/// Kept beside the match in [`Tx::chain`] so a report of what the corpus
/// still needs cannot drift from what the transcoder actually reads.
pub const SUPPORTED_APIS: &[&str] = &[
    "DealDamage",
    "GainLife",
    "LoseLife",
    "Draw",
    "Mill",
    "Scry",
    "Surveil",
    "Mana",
    "Destroy",
    "Tap",
    "Untap",
    "Counter",
    "PutCounter",
    "Sacrifice",
    "Pump",
    "ChangeZone",
    "Token",
];

/// A cost token that names an object, split into its kind and its body.
///
/// `Sac<1/CARDNAME…>` is deliberately not among them: it is matched one
/// branch earlier as `SacrificeSelf`, which asks nobody anything.
fn object_cost(token: &str) -> Option<(&'static str, &str)> {
    for kind in ["Sac", "Discard", "tapXType", "Return"] {
        if let Some(body) = token
            .strip_prefix(kind)
            .and_then(|t| t.strip_prefix('<'))
            .and_then(|t| t.strip_suffix('>'))
        {
            return Some((kind, body));
        }
    }
    None
}

/// Whether a cost part is paid by naming an object.
///
/// The four `cost_wizard` puts a list up for, spelled as the emitter writes
/// them rather than as the engine matches them, because this side has a
/// string and not a `CostPart`. `SacrificeSelf` and `ReturnSelfToHand` are
/// deliberately not among them: they name the source and ask nothing.
fn asks_for_an_object(part: &str) -> bool {
    ["Sacrifice(", "Discard(", "TapOther(", "ReturnToHand("]
        .iter()
        .any(|kind| part.starts_with(kind))
}

/// Whether [`transcode`] has a rule for this effect API.
#[must_use]
pub fn is_supported_api(api: &str) -> bool {
    SUPPORTED_APIS.contains(&api)
}

/// Every effect API a rules line reaches, following `SubAbility$` chains.
#[must_use]
pub fn apis_used(spec: &str, svars: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut queue = vec![spec.to_string()];
    let mut seen = 0usize;
    while let Some(spec) = queue.pop() {
        seen += 1;
        if seen > 32 {
            break; // a malformed chain must not spin here
        }
        let Some((api, _)) = Params::parse(&spec) else {
            continue;
        };
        for part in spec.split(" | ") {
            if let Some(name) = part.strip_prefix("SubAbility$ ")
                && let Some(body) = svars.get(name.trim())
            {
                queue.push(body.clone());
            }
            if let Some(name) = part.strip_prefix("Execute$ ")
                && let Some(body) = svars.get(name.trim())
            {
                queue.push(body.clone());
            }
        }
        if !matches!(api.as_str(), "ChangesZone" | "Phase" | "Attacks" | "Taps") {
            out.push(api);
        }
    }
    out
}

/// Whether a keyword line maps onto a `KeywordSet` bit.
#[must_use]
pub fn keyword_const_of(line: &str) -> Option<&'static str> {
    keyword_const(line)
}

/// The `K:` lines that are a static ability rather than a bit, for a
/// reporter that has to tell the two apart.
#[must_use]
pub fn keyword_static_of(line: &str) -> Option<&'static str> {
    keyword_static(line)
}

/// The `K:` lines that are an as-it-enters modifier on the face rather than
/// anything in `abilities`, for a reporter that has to tell them apart.
///
/// Takes the card's `SVar`s because the line's own amount is not enough to
/// say whether it was read: `etbCounter:P1P1:X` is a card under one `SVar:X`
/// and a refusal under every other.
#[must_use]
pub fn keyword_enter_modifier_of(line: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    keyword_enter_modifier(line, svars)
}

/// Every token script this card names, by the stem its `TokenScript$` gives.
///
/// Read off the raw text of the rules lines and the `SVar` bodies rather than
/// from a parsed effect, because it is asked **before** the card is
/// transcoded: the ledger has to have given the token an id before a card may
/// name the constant it sits at, and a card is refused for a dozen reasons
/// that have nothing to do with its token. Naming a stem here is therefore
/// not a claim that the card will be read — it is a claim that *this token*
/// is one a card in this pool reaches for.
///
/// Every stem in the corpus is `[A-Za-z0-9_]`, so the scan ends at the first
/// character outside that set and needs no `|` splitting.
#[must_use]
pub fn token_stems(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    let bodies = script
        .rules
        .iter()
        .map(|(_, body)| body.as_str())
        .chain(script.svars.values().map(String::as_str));
    for body in bodies {
        for at in body.match_indices("TokenScript$").map(|(i, _)| i) {
            let rest = body[at + "TokenScript$".len()..].trim_start();
            let stem: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !stem.is_empty() && !out.contains(&stem) {
                out.push(stem);
            }
        }
    }
    out
}

/// Every distinct mechanic a script touches, as flat strings.
///
/// This is the unit a coverage plan is built out of: `api:Token`,
/// `param:Pump.Duration`, `kw:Equip`, `line:S`. It is deliberately *not*
/// a list of what the transcoder refused — it names what the script
/// *uses*, so that atoms already appearing in scripts the transcoder reads
/// in full can be subtracted as known. That subtraction is what keeps the
/// plan honest without restating each rule's parameter list here, where
/// the copy would rot the first time a rule learned a new key.
#[must_use]
pub fn atoms(script: &CardScript) -> Vec<String> {
    let mut out = Vec::new();
    for line in &script.keywords {
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        out.push(format!("kw:{head}"));
    }
    for line in &script.unknown_lines {
        let head = line.split(':').next().unwrap_or(line);
        out.push(format!("line:{head}"));
    }
    for (kind, spec) in &script.rules {
        if matches!(kind, 'S' | 'R') {
            out.push(format!("line:{kind}"));
        }
        let mut queue = vec![spec.clone()];
        let mut seen = 0usize;
        while let Some(spec) = queue.pop() {
            seen += 1;
            if seen > 32 {
                break; // a malformed chain must not spin here
            }
            let Some((api, params)) = Params::parse(&spec) else {
                continue;
            };
            out.push(format!("api:{api}"));
            for (key, _) in &params.entries {
                if !PROSE_KEYS.contains(&key.as_str()) {
                    out.push(format!("param:{api}.{key}"));
                }
            }
            for part in spec.split(" | ") {
                for prefix in ["SubAbility$ ", "Execute$ "] {
                    if let Some(name) = part.strip_prefix(prefix)
                        && let Some(body) = script.svars.get(name.trim())
                    {
                        queue.push(body.clone());
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats() -> SubtypeCatalogs {
        let mut c = SubtypeCatalogs {
            creature: vec!["Goblin".into(), "Wizard".into()],
            land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
            ..SubtypeCatalogs::default()
        };
        c.normalize();
        c
    }

    fn read(text: &str) -> CardBody {
        transcode(&parse(text), &cats(), None).expect("should be read in full")
    }

    fn refused(text: &str) -> bool {
        transcode(&parse(text), &cats(), None).is_none()
    }

    /// CR 605.1a: **could** add mana, not "does nothing else".
    ///
    /// This was `all`, and so a Talisman, a painland and a Chromatic Sphere
    /// — every mana ability printed with a rider — came out as an ordinary
    /// activated ability. Two things follow from that flag and both were
    /// wrong: the engine put the mana on the stack, where an opponent may
    /// respond to it, and the client's ability sheet, which reads it to
    /// decide whether a press needs arming (CR 605.1 is the whole reason a
    /// mana ability stays one tap), asked for a second tap.
    #[test]
    fn an_ability_that_could_add_mana_is_a_mana_ability_whatever_else_it_does() {
        // A painland: mana, and a rider that names a player without
        // targeting one.
        let pain = read(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ Combo W U | SubAbility$ DBDmg | SpellDescription$ Add {W} or {U}.\n\
             SVar:DBDmg:DB$ DealDamage | Defined$ You | NumDmg$ 1",
        );
        assert_eq!(
            pain.abilities,
            [
                "mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]), \
              Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::You) }])"
            ]
        );

        // Chromatic Sphere: mana and a draw, off a cost that is not a bare
        // tap — so the long form of the macro, with the cost written out.
        let sphere = read(
            "Name:X\nTypes:Artifact\n\
             A:AB$ Mana | Cost$ 1 T Sac<1/CARDNAME> | Produced$ Any | SubAbility$ DBDraw\n\
             SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
        );
        assert_eq!(
            sphere.abilities,
            ["mana_ability!(cost!(\"{1}\", TapSelf, SacrificeSelf), \
              &[Effect::mana_of_any_color(), Effect::draw(1)])"]
        );

        // And the other side of the line, which is where CR 605.1a draws
        // it: an ability that **targets** is not a mana ability however much
        // mana it makes, so its mana goes on the stack like anything else.
        let targeted = read(
            "Name:X\nTypes:Creature Elf\nPT:1/2\n\
             A:AB$ Mana | Cost$ T | Produced$ G | ValidTgts$ Creature | SubAbility$ DBTap\n\
             SVar:DBTap:DB$ Tap",
        );
        assert_eq!(
            targeted.abilities,
            [
                "activated!(Cost::TAP, &[Effect::mana(ManaColor::Green, 1), Effect::TapTarget], \
              target = Some(TargetSpec::Object(&Filter::CREATURE)))"
            ]
        );
    }

    /// `K:ETBReplacement:Other:<svar>` is a **pointer**, and the rule is what
    /// it points at.
    ///
    /// Reading the keyword alone would say "as this enters, something", which
    /// is why the exclusion and the colour the tap makes are both asserted
    /// here: the two halves of these lands only work as a pair, and a card
    /// that chose a colour nothing read would be a land that taps for
    /// nothing.
    #[test]
    fn an_as_enters_colour_choice_is_read_together_with_what_taps_for_it() {
        let plain = read(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
             SVar:CC:DB$ ChooseColor | Defined$ You | AILogic$ MostProminentInComputerDeck | SpellDescription$ As CARDNAME enters, choose a color.\n\
             A:AB$ Mana | Cost$ T | Produced$ Chosen | SpellDescription$ Add one mana of the chosen color.",
        );
        assert_eq!(plain.enter_modifiers, ["EnterModifier::ChooseColor"]);
        assert_eq!(plain.abilities, ["mana_ability!(&[Effect::mana_chosen()])"]);

        // Thriving Heath: the colour it may not be told to make is the one
        // it always makes anyway.
        let except = read(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
             SVar:CC:DB$ ChooseColor | Defined$ You | Exclude$ white | SpellDescription$ As CARDNAME enters, choose a color other than white.\n\
             A:AB$ Mana | Cost$ T | Produced$ Combo W Chosen | SpellDescription$ Add {W} or one mana of the chosen color.",
        );
        assert_eq!(
            except.enter_modifiers,
            ["EnterModifier::ChooseColorExcept(ManaColor::White)"]
        );
        assert_eq!(
            except.abilities,
            ["mana_ability!(&[Effect::mana_chosen_or(&[ManaColor::White])])"]
        );

        // A clone choosing what to enter as is a different mechanism.
        assert!(refused(
            "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\nK:ETBReplacement:Copy:CC\n\
             SVar:CC:DB$ Clone | Defined$ You"
        ));
        // "As this enters, choose a color" is its controller's choice, and a
        // card handing it to somebody else is a different sentence.
        assert!(refused(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
             SVar:CC:DB$ ChooseColor | Defined$ Opponent"
        ));
        // A word that is not one of the five.
        assert!(refused(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
             SVar:CC:DB$ ChooseColor | Defined$ You | Exclude$ chartreuse"
        ));
        // A parameter no rule here claims.
        assert!(refused(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:CC\n\
             SVar:CC:DB$ ChooseColor | Defined$ You | Amount$ 2"
        ));
        // And the pointer has to point somewhere.
        assert!(refused(
            "Name:X\nTypes:Land\nK:ETBReplacement:Other:Missing"
        ));
    }

    /// The three token scripts the `Token` tests below read against, in the
    /// reference's own shape. Held rather than read off disk: the corpus is
    /// not vendored, so a test that needed a checkout is a test CI skips.
    fn tokens() -> TokenLookup {
        TokenLookup::held(&[
            (
                "r_1_1_goblin",
                "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
            ),
            (
                "u_1_1_wizard_flying",
                "Name:Wizard\nTypes:Creature Wizard\nColors:blue\nPT:1/1\nK:Flying",
            ),
            // A token that carries an ability, which is what 190 of the
            // reference's 852 token scripts do: the definition names it, and
            // the constant does not — two Goblins that differ only there are
            // one name.
            (
                "r_1_1_goblin_sac",
                "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1\n\
                 A:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any",
            ),
            // And one whose ability makes a token, which is the shape
            // [`crate::tokengen`] refuses: it hands the transcoder no
            // lookup, so a token cannot read another token into existence.
            (
                "r_1_1_goblin_maker",
                "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1\n\
                 A:AB$ Token | Cost$ T | TokenScript$ r_1_1_goblin | TokenOwner$ You",
            ),
        ])
    }

    fn read_with_tokens(text: &str) -> CardBody {
        transcode(&parse(text), &cats(), Some(&tokens())).expect("should be read in full")
    }

    fn refused_with_tokens(text: &str) -> bool {
        transcode(&parse(text), &cats(), Some(&tokens())).is_none()
    }

    fn filter(valid: &str) -> String {
        let svars = BTreeMap::new();
        let cats = cats();
        let tx = Tx {
            svars: &svars,
            cats: &cats,
            tokens: None,
            has_x: false,
            body: CardBody::default(),
            unclaimed: std::cell::RefCell::new(None),
        };
        tx.filter_expr(valid).expect("the valid-string is read")
    }

    /// A card's token scripts are found wherever they are written, and each
    /// is named once.
    ///
    /// Both halves matter. The commonest shape in the corpus puts the effect
    /// in an `SVar` and only the trigger on the rules line, so a scan of the
    /// rules alone finds nothing for most of the cards that make tokens —
    /// `SVar:TrigToken:DB$ Token` appears 1037 times against 402 `A:AB$
    /// Token`. And a card that makes the same token twice is one token: the
    /// list feeds an append-only ledger, where a repeat would be a second id
    /// for one permanent.
    #[test]
    fn a_card_names_each_of_its_token_scripts_once() {
        let script = parse(
            "Name:Two Sides\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             Execute$ TrigToken | TriggerDescription$ x\n\
             A:AB$ Token | Cost$ 2 | TokenScript$ r_1_1_goblin | TokenAmount$ 2\n\
             SVar:TrigToken:DB$ Token | TokenScript$ b_2_2_zombie | TokenOwner$ You\n\
             SVar:Other:DB$ Token | TokenScript$ r_1_1_goblin\n",
        );
        assert_eq!(token_stems(&script), ["r_1_1_goblin", "b_2_2_zombie"]);
        // A card that names none says so, rather than saying nothing at all.
        assert!(token_stems(&parse("Name:Plain\nK:Flying\n")).is_empty());
    }

    /// Every entry in `Tx::NAMED` is reachable from a valid-string the corpus
    /// actually prints — a table row nothing produces is a claim no run
    /// checks. The pairing itself is proved elsewhere and more strongly: the
    /// pool dump is byte-identical across this substitution, which it could
    /// not be if a constant held the clauses in another order.
    #[test]
    fn a_filter_the_dsl_already_names_is_written_as_that_name() {
        assert_eq!(filter("Creature.YouCtrl"), "Filter::YOUR_CREATURE");
        assert_eq!(filter("Creature.OppCtrl"), "Filter::OPPONENT_CREATURE");
        assert_eq!(filter("Creature.Other"), "Filter::ANOTHER_CREATURE");
        assert_eq!(filter("Creature.nonToken"), "Filter::NONTOKEN_CREATURE");
        assert_eq!(filter("Creature.Legendary"), "Filter::LEGENDARY_CREATURE");
        assert_eq!(filter("Creature.attacking"), "Filter::ATTACKING_CREATURE");
        assert_eq!(filter("Land.YouCtrl"), "Filter::YOUR_LAND");
        assert_eq!(filter("Artifact.YouCtrl"), "Filter::YOUR_ARTIFACT");
        assert_eq!(filter("Land.nonBasic"), "Filter::NONBASIC_LAND");
        assert_eq!(
            filter("Artifact,Enchantment"),
            "Filter::ARTIFACT_OR_ENCHANTMENT"
        );
        assert_eq!(filter("Artifact,Creature"), "Filter::ARTIFACT_OR_CREATURE");
        assert_eq!(
            filter("Artifact,Creature,Enchantment"),
            "Filter::ARTIFACT_CREATURE_OR_ENCHANTMENT"
        );
        assert_eq!(
            filter("Creature,Planeswalker"),
            "Filter::CREATURE_OR_PLANESWALKER"
        );
        assert_eq!(filter("Instant,Sorcery"), "Filter::INSTANT_OR_SORCERY");
        // The counter-test: a filter with no constant is still written out,
        // and one the DSL spells in the other order is left alone rather
        // than quietly reordered.
        assert_eq!(
            filter("Creature.tapped"),
            "Filter::And(&[Filter::CREATURE, Filter::Tapped])"
        );
        assert_eq!(
            filter("Land.Basic"),
            "Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC)])"
        );
        assert_eq!(
            filter("Land.Basic+YouCtrl"),
            "Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::BASIC), \
             Filter::ControlledByYou])",
            "three flat clauses are not the nested YOUR_BASIC_LAND"
        );
        // The two the corpus writes both ways round. Neither reordering is
        // this reader's to make, so the rarer spelling is written out and
        // "another creature you control" reaches no name at all — see the
        // measurement on `NAMED`.
        assert_eq!(
            filter("Creature,Artifact"),
            "Filter::Or(&[Filter::CREATURE, Filter::ARTIFACT])"
        );
        assert_eq!(
            filter("Creature.Other+YouCtrl"),
            "Filter::And(&[Filter::CREATURE, Filter::Another, Filter::ControlledByYou])",
            "the commoner spelling builds the order ANOTHER_CREATURE_YOU_CONTROL does not have"
        );
    }

    /// A filter that is already a name is not given a second one. Fifteen
    /// generated cards carried a `static` that was one bare constant, eight
    /// of them `static TARGET1: Filter = Filter::CREATURE;`, which is the
    /// duplication the hoist exists to prevent.
    #[test]
    fn a_filter_that_is_already_a_name_is_not_hoisted_into_a_static() {
        let body = read(
            "Name:X\nManaCost:R\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | SpellDescription$ deals 2 damage.",
        );
        assert!(body.statics.is_empty(), "{}", body.statics);
        assert!(
            body.abilities[0].contains("TargetSpec::Object(&Filter::CREATURE)"),
            "{}",
            body.abilities[0]
        );
    }

    #[test]
    fn a_damage_spell_keeps_its_target_and_its_amount() {
        let body = read(
            "Name:Shock the Bear\nManaCost:R\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SpellDescription$ deals 3 damage.\n\
             Oracle:Shock the Bear deals 3 damage to target creature.",
        );
        assert_eq!(body.abilities.len(), 1);
        assert!(body.abilities[0].starts_with("spell!("));
        assert!(body.abilities[0].contains("Effect::DealDamage { amount: Amount::Fixed(3)"));
        assert!(body.abilities[0].contains("targets = Some(TargetReq::one("));
    }

    #[test]
    fn keywords_become_bits_and_abilities_stay_abilities() {
        let body = read(
            "Name:Birds of Paradise\nManaCost:G\nTypes:Creature Bird\nPT:0/1\n\
             A:AB$ Mana | Cost$ T | Produced$ Any | SpellDescription$ Add one mana of any color.\n\
             K:Flying\nOracle:Flying",
        );
        assert_eq!(body.keywords, ["KeywordSet::FLYING"]);
        assert_eq!(
            body.abilities,
            ["mana_ability!(&[Effect::mana_of_any_color()])"]
        );
    }

    #[test]
    fn a_two_color_mana_ability_is_a_choice_and_an_amount_is_a_count() {
        let body = read(
            "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U\n\
             A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 2",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])",
                "mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)])",
            ]
        );
    }

    #[test]
    fn a_trigger_resolves_the_svar_it_executes() {
        let body = read(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ gain 2 life.\n\
             SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
        );
        assert_eq!(
            body.abilities,
            ["triggered!(Trigger::ETB, &[Effect::gain_life(2)])"]
        );
    }

    /// "Whenever an opponent casts a spell": the two keys join into one
    /// filter, and the three shapes that must not be read are refused by
    /// name.
    ///
    /// The zone is the one that matters. A `SpellCast` line with no
    /// `TriggerZones$` is almost always "when you cast **this** spell",
    /// which fires from the stack — 98 of the corpus's 114 zoneless lines
    /// name `Card.Self` — and this engine collects triggers off the
    /// battlefield. Read as an ordinary trigger it is a card whose ability
    /// can never fire, so both halves of that sentence are refused
    /// separately: the missing zone, and the self-reference under any zone.
    #[test]
    fn a_spell_cast_trigger_joins_what_was_cast_with_who_cast_it() {
        // Rhystic Study's own line, minus the `UnlessCost$` its `SVar`
        // writes — which is a different rule's business.
        let body = read(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Opponent \
             | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
        );
        assert_eq!(
            body.abilities,
            ["triggered!(Trigger::SpellCast(&Filter::ControlledByOpponent), &[Effect::draw(1)])"]
        );

        // Every alternative takes the controller atom, not just the first.
        let two = read(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You \
             | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.\n\
             SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1",
        );
        assert!(
            two.statics.contains(
                "Filter::Or(&[Filter::And(&[Filter::HasType(TypeSet::INSTANT), \
                 Filter::ControlledByYou]), Filter::And(&[Filter::HasType(TypeSet::SORCERY), \
                 Filter::ControlledByYou])])"
            ),
            "{}",
            two.statics
        );

        for (line, why) in [
            (
                "T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ You \
                 | Execute$ TrigDraw | TriggerDescription$ draw.",
                "a `SpellCast` trigger with no `TriggerZones$`",
            ),
            (
                "T:Mode$ SpellCast | ValidCard$ Card.Self | TriggerZones$ Battlefield \
                 | Execute$ TrigDraw | TriggerDescription$ draw.",
                "a `SpellCast` trigger on the card itself",
            ),
            (
                "T:Mode$ SpellCast | ValidCard$ Card | ValidActivatingPlayer$ Player.EnchantedBy \
                 | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ draw.",
                "a spell cast by `Player.EnchantedBy`",
            ),
        ] {
            let script = parse(&format!(
                "Name:X\nTypes:Enchantment\n{line}\n\
                 SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1"
            ));
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(why),
                "{line}"
            );
        }
    }

    /// "When this enters, you may …" is one decision over the whole clause.
    ///
    /// CR 603.5: an optional triggered ability goes on the stack whatever
    /// its controller intends, and the choice is made as it resolves — which
    /// is `Effect::MayDo`, asked of the resolving ability's controller. The
    /// three shapes that must not become one are refused instead:
    ///
    /// * a decider who is not the controller, which no effect here can ask;
    /// * a `may` whose clause is "pay this cost, then …" — the reference
    ///   writes that as a `Cost$` on the executed sub-ability, and 183 of
    ///   the 1442 `OptionalDecider$ You` triggers do. Read as a bare `MayDo`
    ///   it would be a free effect under `Coverage::Implemented`, for
    ///   hundreds of cards at once, so the counter-case is pinned here and
    ///   not merely inferred from where `Cost$` is claimed;
    /// * the same key on a **sub-ability** (83 in the corpus), where
    ///   declining skips the rest of the chain rather than one step, and so
    ///   is not this shape at all.
    #[test]
    fn a_may_on_a_trigger_wraps_the_whole_clause() {
        let body = read(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | OptionalDecider$ You | Execute$ TrigGain | TriggerDescription$ you may gain 2 life.\n\
             SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
        );
        assert_eq!(
            body.abilities,
            ["triggered!(Trigger::ETB, &[Effect::MayDo { effects: &[Effect::gain_life(2)] }])"]
        );

        // A target is chosen when the ability goes on the stack (CR 603.3d)
        // and the `may` is answered as it resolves, so the target stays
        // outside the wrap.
        let targeted = read(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | OptionalDecider$ You | Execute$ TrigLose | TriggerDescription$ you may drain.\n\
             SVar:TrigLose:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 1",
        );
        assert_eq!(
            targeted.abilities,
            [
                "triggered!(Trigger::ETB, &[Effect::MayDo { effects: &[Effect::LoseLife { \
                 amount: Amount::Fixed(1), target: PlayerRel::Chosen }] }], \
                 targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
            ]
        );

        for (lines, why) in [
            (
                "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
                 | ValidCard$ Card.Self | OptionalDecider$ TriggeredCardController \
                 | Execute$ TrigGain | TriggerDescription$ x.\n\
                 SVar:TrigGain:DB$ GainLife | LifeAmount$ 2",
                "a `may` decided by `TriggeredCardController`",
            ),
            (
                // Ruin Processor's shape: "you may put a card an opponent
                // owns from exile into that player's graveyard. If you
                // do, you gain 5 life."
                "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
                 | ValidCard$ Card.Self | OptionalDecider$ You | Execute$ TrigGain \
                 | TriggerDescription$ x.\n\
                 SVar:TrigGain:AB$ GainLife | Cost$ Sac<1/Creature.YouCtrl/a creature> \
                 | LifeAmount$ 5",
                "unclaimed parameter `GainLife.Cost`",
            ),
            (
                "T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield \
                 | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ x.\n\
                 SVar:TrigGain:DB$ GainLife | LifeAmount$ 2 | OptionalDecider$ You",
                "unclaimed parameter `GainLife.OptionalDecider`",
            ),
        ] {
            let script = parse(&format!("Name:X\nTypes:Creature Goblin\nPT:1/1\n{lines}"));
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(why),
                "{lines}"
            );
        }
    }

    #[test]
    fn a_subability_chain_becomes_a_sequence_of_effects() {
        let body = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ Draw | NumCards$ 2 | SubAbility$ DBLose\n\
             SVar:DBLose:DB$ LoseLife | LifeAmount$ 2",
        );
        assert_eq!(
            body.abilities,
            [
                "spell!(&[Effect::draw(2), Effect::LoseLife { amount: Amount::Fixed(2), target: PlayerRel::You }])"
            ]
        );
    }

    /// "Target player loses 1 life" (Piranha Marsh): the effect carries no
    /// `Defined$`, and reading that as `You` would drain the controller. The
    /// chain targets a player, so the absent key means the chosen one.
    #[test]
    fn an_undefined_player_effect_means_the_targeted_player() {
        let body = read(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLoseLife | TriggerDescription$ loses 1 life.\n\
             SVar:TrigLoseLife:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 1 | TgtPrompt$ Select target player",
        );
        assert_eq!(
            body.abilities,
            [
                "triggered!(Trigger::ETB, &[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::Chosen }], targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
            ]
        );
    }

    /// A checkland: a script writes the printed "enters tapped **unless** you
    /// control a Swamp or a Mountain" inside out, as "tap it when the count
    /// of those is zero". `EQ0` is that sentence and nothing else is.
    #[test]
    fn a_conditional_enters_tapped_becomes_tapped_unless() {
        let body = read(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Basic+YouCtrl | ConditionCompare$ EQ0",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnless(&CHECK1)"]
        );
        assert!(
            body.statics
                .contains("Filter::HasSupertype(SupertypeSet::BASIC)"),
            "{}",
            body.statics
        );
    }

    /// "Unless you control *two* other lands" is a count, and the
    /// arithmetic between the script and the card is the whole risk.
    ///
    /// The reference says when the land comes down **tapped** and the card
    /// prints when it does not, so `LT n` is `at_least = n` and `LE n` is
    /// `at_least = n + 1`. Neither is argued from the key’s name: Rockfall
    /// Vale writes `LT2` and prints "two or more other lands", and Canopy
    /// Vista writes `LE1` against a hand-written `at_least: 2` in this very
    /// pool. Off by one here is a land that enters untapped a turn early,
    /// which no test downstream would catch.
    ///
    /// `at_least: 1` is deliberately *not* emitted — that sentence is what
    /// `TappedUnless` already says, and a second spelling of one sentence is
    /// what the pool-wide lints exist to prevent.
    #[test]
    fn a_counted_enters_tapped_condition_becomes_a_count() {
        let two = read(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Other+YouCtrl | ConditionCompare$ LT2",
        );
        assert_eq!(
            two.enter_modifiers,
            ["EnterModifier::TappedUnlessCount { filter: &CHECK1, at_least: 2 }"]
        );

        // Canopy Vista’s own line, and the pool’s own answer to it.
        let battle = read(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Basic+YouCtrl | ConditionCompare$ LE1",
        );
        assert_eq!(
            battle.enter_modifiers,
            ["EnterModifier::TappedUnlessCount { filter: &CHECK1, at_least: 2 }"]
        );

        // Steam Vents: `PayLife<2>` is `TappedOrPayLife(2)`, which is how
        // that card is written by hand three directories away.
        let shock = read(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | UnlessCost$ PayLife<2> | UnlessPayer$ You",
        );
        assert_eq!(shock.enter_modifiers, ["EnterModifier::TappedOrPayLife(2)"]);

        // Blackcleave Cliffs: "unless you control two or fewer other lands",
        // which the reference states as the tap — `GT2`.
        let fast = read(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.YouCtrl | ConditionCompare$ GT2",
        );
        assert_eq!(
            fast.enter_modifiers,
            ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 2 }"]
        );

        // The manlands write one sentence two ways — Hall of Storm Giants
        // `GE2`, Den of the Bugbear `GT1` — and both are the same bound. A
        // reader that took the letter rather than the predicate would have
        // given one cycle two different cards.
        for tail in ["GE2", "GT1"] {
            let manland = read(&format!(
                "Name:X\nTypes:Land\n\
                 R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
                 SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.YouCtrl | ConditionCompare$ {tail}"
            ));
            assert_eq!(
                manland.enter_modifiers,
                ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 1 }"],
                "{tail}"
            );
        }

        for (tail, why) in [
            // `GE0` is where the conversion would underflow, and it is not a
            // sentence: a land tapped whatever the board says needs the other
            // variant entirely. The corpus writes it nowhere, so this is the
            // boundary being refused rather than a card being lost.
            (
                "| ConditionPresent$ Land.YouCtrl | ConditionCompare$ GE0",
                "an enter-tapped condition `GE0` this rule cannot read",
            ),
            // Rustic Clachan reveals a Kithkin instead of paying life.
            (
                "| UnlessCost$ Reveal<1/Kithkin> | UnlessPayer$ You",
                "replacement `Moved` charging `Reveal<1/Kithkin>`",
            ),
            (
                "| UnlessCost$ PayLife<2> | UnlessPayer$ Opponent",
                "replacement `Moved` charging `Opponent`",
            ),
            // A computed condition whose SVar is not there to read.
            (
                "| ConditionCheckSVar$ X | ConditionSVarCompare$ LT2",
                "`ConditionCheckSVar$ X` names no SVar",
            ),
        ] {
            let script = parse(&format!(
                "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True {tail}"
            ));
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(why),
                "{tail}"
            );
        }
    }

    /// `Produced$ W U` is "add {W}{U}" — two mana at once. `Combo W U` is
    /// the choice between them, and reading one as the other would hand a
    /// bounce land twice the mana or half of it.
    #[test]
    fn produced_lists_two_mana_and_combo_offers_a_choice() {
        let both = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ W U");
        assert_eq!(
            both.abilities,
            [
                "mana_ability!(&[Effect::mana(ManaColor::White, 1), Effect::mana(ManaColor::Blue, 1)])"
            ]
        );
        let either = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U");
        assert_eq!(
            either.abilities,
            ["mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])"]
        );
    }

    /// "Spend this mana only to cast a creature spell" is a rider on the
    /// mana, and only on spells: `Activated.Hero` restricts an *ability*,
    /// which `ManaRestriction` cannot say, so a card printing both stays a
    /// stub rather than becoming the half of itself we can express.
    #[test]
    fn restricted_mana_reads_a_spell_filter_and_only_that() {
        let body = read(
            "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Creature",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana_of_any_color().restricted(&Filter::CREATURE, SpendRider::None)])"
            ]
        );

        let script = parse(
            "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Hero,Activated.Hero",
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("`Mana.RestrictValid` beyond a spell")
        );
    }

    /// A manland: one printed sentence, four layers. "It's still a land" is
    /// why the types are *added*, and CR 613.1 is why each layer is its own
    /// effect rather than one lump.
    #[test]
    fn animate_becomes_one_continuous_effect_per_layer() {
        let body = read(
            "Name:X\nTypes:Land\n\
             A:AB$ Animate | Cost$ 1 G | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature,Goblin | Colors$ Green | OverwriteColors$ True | Keywords$ Trample",
        );
        let a = body.abilities.join("");
        // No layer is written: `Effect::continuous` derives it from the
        // modifier (CR 613.1), so the modifier *is* the layer claim here.
        for expected in [
            "Effect::continuous(&Filter::This, Modifier::AddType(TypeSet::CREATURE), Duration::UntilEndOfTurn)",
            "Modifier::AddSubtype(subtypes::creature::GOBLIN)",
            "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(&[Color::Green])), Duration::UntilEndOfTurn)",
            "Effect::continuous(&Filter::This, Modifier::AddKeyword(KeywordSet::TRAMPLE), Duration::UntilEndOfTurn)",
            "Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::UntilEndOfTurn)",
        ] {
            assert!(a.contains(expected), "missing `{expected}` in {a}");
        }
        assert!(!a.contains("RemoveType"), "it's still a land");

        // `Filter::This` binds to the first target when the chain has one,
        // so an animate that also targets would animate the wrong
        // permanent. Refuse rather than guess which was meant.
        let script = parse(
            "Name:X\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Land | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature",
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("`Animate` of something other than the source")
        );
    }

    /// The bounce land's sentence: nobody is targeted, the ability's
    /// controller picks one of their own lands, and it goes to its owner's
    /// hand.
    ///
    /// Each half is struck on its own below, because a reader that emitted
    /// this shape for *any* untargeted `Battlefield` → `Hand` line would
    /// pass the first assertion and be wrong about four other cards.
    #[test]
    fn a_hidden_battlefield_to_hand_is_a_choice_among_your_own() {
        let body = read(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
             SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
             | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl \
             | AILogic$ NeverBounceItself | SpellDescription$ Return a land you control.",
        );
        assert_eq!(
            body.abilities,
            [
                "triggered!(Trigger::ETB, &[Effect::ReturnChosenToHand { who: PlayerRel::You, \
                 filter: &Filter::YOUR_LAND }])"
            ]
        );

        // `Mandatory$ True` is what separates "return a land you control"
        // from "you may return a land you control", and the effect can only
        // say the first. Without the word it is refused rather than read as
        // either — the same bargain a library search makes about its count.
        let silent = parse(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
             SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
             | Hidden$ True | ChangeType$ Land.YouCtrl",
        );
        assert_eq!(
            refusal_reason(&silent, &cats(), None).as_deref(),
            Some("`ChangeZone` chosen without `Mandatory$`")
        );

        // `Hidden$` is the discriminator, and without it the line falls
        // through to the report it had before this rule existed.
        let open = parse(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
             SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
             | Mandatory$ True | ChangeType$ Land.YouCtrl",
        );
        assert_eq!(
            refusal_reason(&open, &cats(), None).as_deref(),
            Some("`ChangeZone` with neither a target nor `Defined$`")
        );

        // Arid Archway: the return is this rule, and the surveil hanging
        // off it reads the permanent that came back. Nothing claims
        // `RememberLKI$`, so the card is refused whole rather than written
        // as a bounce land that forgot half its sentence.
        let remembered = parse(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
             SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
             | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl | RememberLKI$ True",
        );
        assert_eq!(
            refusal_reason(&remembered, &cats(), None).as_deref(),
            Some("unclaimed parameter `ChangeZone.RememberLKI`")
        );

        // A second permanent is a different rule, and it says so rather
        // than writing a card that returns one of them.
        let two = parse(
            "Name:X\nTypes:Land\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigReturn | TriggerDescription$ return two lands you control.\n\
             SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
             | Hidden$ True | Mandatory$ True | ChangeNum$ 2 | ChangeType$ Land.YouCtrl",
        );
        assert_eq!(
            refusal_reason(&two, &cats(), None).as_deref(),
            Some("`ChangeZone` returning 2 chosen permanents")
        );
    }

    /// The two enters-tapped sentences that count players, and the four
    /// ways a line is refused instead.
    ///
    /// Both directions are struck on their own, because the whole risk in
    /// this rule is reading one of them with the other's sign: the
    /// comparator says when the land comes down *tapped* and the card prints
    /// when it does not.
    #[test]
    fn an_enters_tapped_condition_may_count_players() {
        let head = "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield \
             | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n";

        // Luxury Suite: taps while you have fewer than two opponents, which
        // is "unless you have two or more opponents".
        let crowd = read(&format!(
            "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
             | ConditionCheckSVar$ Y | ConditionSVarCompare$ LT2\n\
             SVar:Y:PlayerCountOpponents$Amount"
        ));
        assert_eq!(
            crowd.enter_modifiers,
            ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
        );

        // Razortrap Gorge: taps while the lowest life total is above
        // thirteen, which is "unless a player has 13 or less life".
        let unlucky = read(&format!(
            "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
             | ConditionCheckSVar$ X | ConditionSVarCompare$ GT13\n\
             SVar:X:PlayerCountPlayers$LowestLifeTotal"
        ));
        assert_eq!(
            unlucky.enter_modifiers,
            ["EnterModifier::TappedUnlessSomeoneAtOrBelow { life: 13 }"]
        );

        // `LE`/`GE` are the same sentences off by one, and the arithmetic is
        // asserted rather than assumed.
        let off_by_one = read(&format!(
            "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
             | ConditionCheckSVar$ Y | ConditionSVarCompare$ LE1\n\
             SVar:Y:PlayerCountOpponents$Amount"
        ));
        assert_eq!(
            off_by_one.enter_modifiers,
            ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
        );

        for (tail, svar, why) in [
            // The opponent count with the life total's comparator. Read with
            // a flipped sign this would be a land that enters untapped in
            // every duel; refused, it is a stub.
            (
                "| ConditionCheckSVar$ Y | ConditionSVarCompare$ GT1",
                "SVar:Y:PlayerCountOpponents$Amount",
                "an enter-tapped condition counting `PlayerCountOpponents$Amount` `GT1`",
            ),
            // And the life total with the count's comparator.
            (
                "| ConditionCheckSVar$ X | ConditionSVarCompare$ LT13",
                "SVar:X:PlayerCountPlayers$LowestLifeTotal",
                "an enter-tapped condition counting `PlayerCountPlayers$LowestLifeTotal` `LT13`",
            ),
            // A count this rule has never seen: named by what it counts, not
            // by the letter the corpus wrote.
            (
                "| ConditionCheckSVar$ Z | ConditionSVarCompare$ EQ0",
                "SVar:Z:Count$Valid Creature.YouCtrl",
                "an enter-tapped condition counting `Count$Valid Creature.YouCtrl` `EQ0`",
            ),
            // The comparison missing altogether.
            (
                "| ConditionCheckSVar$ Y",
                "SVar:Y:PlayerCountOpponents$Amount",
                "replacement `Moved` counting an SVar with no comparison",
            ),
        ] {
            let script = parse(&format!(
                "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True {tail}\n{svar}"
            ));
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(why),
                "{tail}"
            );
        }
    }

    /// A fetchland: `Origin$ Library` is a *search*, not a zone change with
    /// a hidden target — a card in a library cannot be targeted at all.
    #[test]
    fn a_library_change_zone_is_a_search() {
        let body = read(
            "Name:X\nTypes:Land\n\
             A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Forest",
        );
        assert_eq!(
            body.abilities,
            [
                "activated!(cost!(TapSelf, SacrificeSelf), &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD], optional: false }])"
            ]
        );

        // `finds` is positional and its length is the count, so two cards
        // are the same `Find` twice — and `Tapped$ True` is a different
        // `Find`, not a flag beside it.
        let two = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeNum$ 2 | Optional$ True | ChangeType$ Forest",
        );
        assert!(
            two.abilities.join("").contains(
                "finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED], optional: true"
            ),
            "{:?}",
            two.abilities
        );

        // The same line with neither word. `ChangeNum$` alone does not say
        // whether two is a maximum or a requirement, and the two are
        // different cards, so it is refused rather than guessed at.
        let ambiguous = parse(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeNum$ 2 | ChangeType$ Forest",
        );
        assert_eq!(
            refusal_reason(&ambiguous, &cats(), None).as_deref(),
            Some("`ChangeZone` finding several cards without saying whether that is a maximum")
        );

        // `Mandatory$ True` is the other word, and it says the opposite of
        // `Optional$ True` rather than merely failing to say it.
        let must = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeNum$ 2 | Mandatory$ True | ChangeType$ Card",
        );
        assert!(
            must.abilities
                .join("")
                .contains("finds: &[Find::HAND, Find::HAND], optional: false"),
            "{:?}",
            must.abilities
        );

        // A count of one needs no word at all, which is what keeps every
        // fetchland in the pool readable.
        let one = read(
            "Name:X\nTypes:Land\n\
             A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
        );
        assert!(
            one.abilities.join("").contains("optional: false"),
            "{:?}",
            one.abilities
        );
    }

    /// `ChangeTypeDesc$` restates the filter beside it for a human, and is
    /// claimed only while that filter is there to carry the meaning.
    ///
    /// Not a `PROSE_KEYS` entry, and this is the difference: alone on a
    /// line it is the only thing said about what is being found, and a
    /// reader that dropped it would be inventing a filter.
    #[test]
    fn a_search_label_is_claimed_only_beside_the_filter_it_restates() {
        let labelled = read(
            "Name:X\nTypes:Land\n\
             A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
        );
        assert!(
            labelled
                .abilities
                .join("")
                .contains("finds: &[Find::BATTLEFIELD_TAPPED]"),
            "{:?}",
            labelled.abilities
        );

        let bare = parse(
            "Name:X\nTypes:Land\n\
             A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeTypeDesc$ basic land",
        );
        assert_eq!(
            refusal_reason(&bare, &cats(), None).as_deref(),
            Some("unreadable value in `ChangeZone`"),
            "a label with no filter beside it says nothing a card can be built from"
        );
    }

    /// The load-bearing rule: an unread parameter, an unread effect, an
    /// unread line kind or an unread keyword all refuse the whole card. Each
    /// Giant Growth: the commonest shape in the whole script corpus.
    #[test]
    fn a_pump_binds_to_the_target_and_keeps_its_sign() {
        let body = read(
            "Name:Giant Growth\nManaCost:G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | \
             SpellDescription$ gets +3/+3.\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("Effect::PumpTarget"), "{text}");
        assert!(text.contains("power: Amount::Fixed(3)"), "{text}");
        assert!(text.contains("keywords: KeywordSet::EMPTY"), "{text}");

        // A shrink is the same effect with the sign in the variant,
        // because `Amount::Fixed` cannot hold one.
        let body = read(
            "Name:Weakness\nManaCost:B\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -2 | NumDef$ -1\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("power: Amount::NegXFixed(2)"), "{text}");
        assert!(text.contains("toughness: Amount::NegXFixed(1)"), "{text}");
    }

    /// "Doesn't untap during your untap step" is an `R:` line in the
    /// reference and a **static ability** here, because CR 613.11 makes it a
    /// continuous effect modifying a game rule rather than a replacement of
    /// any event. Basalt Monolith is the card, and the whole of it is read:
    /// the rule, the mana ability and the way out.
    #[test]
    fn a_cant_happen_untap_replacement_is_a_static_ability() {
        let body = read(
            "Name:Basalt Monolith\nManaCost:3\nTypes:Artifact\n\
             R:Event$ Untap | ValidCard$ Card.Self | ValidStepTurnToController$ You | \
             Layer$ CantHappen | Description$ This artifact doesn't untap.\n\
             A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 3\n\
             A:AB$ Untap | Cost$ 3\n",
        );
        assert_eq!(
            body.abilities,
            [
                "static_ability!(Filter::This, Modifier::DoesNotUntap)",
                "mana_ability!(&[Effect::mana(ManaColor::Colorless, 3)])",
                "activated!(cost!(\"{3}\"), &[Effect::UntapSelf])",
            ]
        );
        assert!(
            body.statics.is_empty(),
            "`Card.Self` is `Filter::This` and needs no `static`: {}",
            body.statics
        );
    }

    /// `Defined$ Self` is the source, not the target, even inside an
    /// ability that has one.
    #[test]
    fn a_pump_on_itself_is_not_a_pump_on_the_target() {
        let body = read(
            "Name:X\nManaCost:R\nTypes:Creature Goblin\nPT:1/1\n\
             A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | NumDef$ +0\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("Effect::PumpFilter"), "{text}");
        assert!(text.contains("filter: &Filter::This"), "{text}");
    }

    /// The storage lands' own clause: `PresentDefined$ Self | IsPresent$
    /// Card.tapped` as an intervening `if` about this card.
    ///
    /// `landgen` reads the same card from the printed text and writes a
    /// bare `Filter::Tapped` for the same ability, and the two are meant to
    /// differ. "If this land is tapped" is a sentence about a permanent,
    /// and a land that has left the battlefield is not tapped; `IsPresent$`
    /// is a question about a *zone*, with the predicate hung off it. Each
    /// reader translates the sentence it was handed, and the extra clause
    /// is true whenever the shorter one is.
    #[test]
    fn a_clause_about_this_card_becomes_a_condition_on_the_trigger() {
        let body = read(
            "Name:Bottomless Vault\nManaCost:no cost\nTypes:Land\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | PresentDefined$ Self | \
             IsPresent$ Card.tapped | Execute$ TrigStore\n\
             SVar:TrigStore:DB$ PutCounter | Defined$ Self | CounterType$ STORAGE | \
             CounterNum$ 1\n",
        );
        assert_eq!(
            body.abilities,
            [concat!(
                "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
                "&[Effect::AddCounter { kind: counters::STORAGE, amount: Amount::Fixed(1) }], ",
                "condition = Some(Condition::SourceMatches(&CHECK1)))"
            )]
        );
        assert!(
            body.statics.contains(concat!(
                "static CHECK1: Filter = ",
                "Filter::And(&[Filter::InZone(ZoneRef::Battlefield), Filter::Tapped]);"
            )),
            "the filter it named: {}",
            body.statics
        );
    }

    /// The same clause written without `PresentDefined$`, which is what the
    /// corpus does four times out of five: of the `T:` lines whose clause
    /// is about this card, 155 pin it in the valid-string alone against 38
    /// that say `PresentDefined$ Self`.
    ///
    /// A reader that missed the pin would not refuse these — it would
    /// *write* them, as a count of the permanents you control, and this one
    /// says `YouCtrl` so nothing downstream could tell. That is the whole
    /// argument for the case: `Card.Self` on its own would be caught by
    /// `ControlCount`'s own guard, and the shape that names a player would
    /// not.
    ///
    /// It is also the splice: the zone goes in beside the clauses the
    /// valid-string named, not around the pair of them.
    #[test]
    fn a_valid_string_that_pins_the_source_is_also_a_clause_about_this_card() {
        let body = read(
            "Name:X\nManaCost:W\nTypes:Enchantment\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
             IsPresent$ Card.Self+YouCtrl+YouOwn | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1\n",
        );
        assert_eq!(
            body.abilities,
            [concat!(
                "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
                "&[Effect::draw(1)], condition = Some(Condition::SourceMatches(&CHECK1)))"
            )]
        );
        assert!(
            body.statics.contains(concat!(
                "static CHECK1: Filter = Filter::And(&[Filter::InZone(ZoneRef::Battlefield), ",
                "Filter::This, Filter::ControlledByYou, Filter::OwnedByYou]);"
            )),
            "the filter it named: {}",
            body.statics
        );
    }

    /// The same family on an `A:` line, where it restricts *activating*
    /// rather than resolving: "Add {U}. Activate only if you control an
    /// Island or a Mountain."
    ///
    /// The verge lands are the shape worth testing, because the pool holds
    /// a hand-written one — Bleachbone Verge — that says the same thing
    /// with the same `Condition::ControlCount`, so this is the one place a
    /// reader and a person can be held against each other.
    ///
    /// It is also the arity trap: the one-argument `mana_ability!` takes
    /// the **cost** as its only positional argument, so a condition beside
    /// it has to spell `Cost::TAP` out or the effects would bind where the
    /// cost goes.
    #[test]
    fn an_activation_clause_becomes_the_condition_on_the_ability() {
        let script = "Name:Riverpyre Verge\nManaCost:no cost\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ U | \
             IsPresent$ Island.YouCtrl,Mountain.YouCtrl | SpellDescription$ Add {U}.\n";
        let body = read(script);
        assert_eq!(
            body.abilities,
            [concat!(
                "mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Blue, 1)], ",
                "condition = Some(Condition::ControlCount(&CHECK1, 1)))"
            )]
        );
        assert!(
            body.statics.contains(concat!(
                "static CHECK1: Filter = Filter::Or(&[",
                "Filter::And(&[Filter::HasSubtype(subtypes::land::ISLAND), Filter::ControlledByYou]), ",
                "Filter::And(&[Filter::HasSubtype(subtypes::land::MOUNTAIN), Filter::ControlledByYou])]);"
            )),
            "the filter it named: {}",
            body.statics
        );

        // A spell is cast and not activated, so there is nothing for the
        // clause to restrict: `spell!` has no precondition and the line
        // refuses by name rather than casting unconditionally.
        let spell = parse(
            "Name:X\nManaCost:U\nTypes:Instant\n\
             A:SP$ Draw | NumCards$ 1 | IsPresent$ Island.YouCtrl\n",
        );
        assert!(transcode(&spell, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&spell, &cats(), None).as_deref(),
            Some("`IsPresent$` on a spell line")
        );

        // And the family's other keys are claimed only where the clause
        // itself was read: a `PresentZone$` with no `IsPresent$` beside it
        // is still an unclaimed parameter, and still refuses the card.
        let lone = parse(
            "Name:X\nManaCost:no cost\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ U | PresentZone$ Graveyard\n",
        );
        assert!(transcode(&lone, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&lone, &cats(), None).as_deref(),
            Some("unclaimed parameter `Mana.PresentZone`")
        );
    }

    /// A clause that counts instead: "if you control two or more
    /// creatures".
    ///
    /// `Condition::ControlCount` is only the right reading while the
    /// valid-string says *whose* — the reference writes the controller into
    /// the filter, and a filter that does not name one is asking whether
    /// such a permanent exists at all, which is a wider question than the
    /// DSL has a sentence for. So the second half of this test is the same
    /// clause with `YouCtrl` taken off, refused by name.
    #[test]
    fn a_clause_that_counts_needs_the_filter_to_say_whose() {
        let script = "Name:X\nManaCost:G\nTypes:Creature Elf\nPT:1/1\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
             IsPresent$ Creature.YouCtrl | PresentCompare$ GE2 | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
        let body = read(script);
        assert_eq!(
            body.abilities,
            [concat!(
                "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::You }, ",
                "&[Effect::draw(1)], ",
                "condition = Some(Condition::ControlCount(&Filter::YOUR_CREATURE, 2)))"
            )]
        );

        let anyone = parse(&script.replace("Creature.YouCtrl", "Creature"));
        assert!(transcode(&anyone, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&anyone, &cats(), None).as_deref(),
            Some("`IsPresent$ Creature`, a count with no player")
        );

        // The same trap one atom further in, and the one that would have
        // been written rather than refused: `Other` is a filter about the
        // card stating the clause, which a count has no room for.
        let another = parse(&script.replace("Creature.YouCtrl", "Creature.Other+YouCtrl"));
        assert!(transcode(&another, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&another, &cats(), None).as_deref(),
            Some("`IsPresent$ Creature.Other+YouCtrl`, a count relative to this card")
        );
    }

    /// Every other key of the family refuses by name, and the reasons are
    /// what a worklist is made of.
    ///
    /// `NoResolvingCheck$ True` is the one that matters most and is easiest
    /// to read past: it is the reference opting *out* of CR 603.4's second
    /// check — the clause asked once instead of twice — and a reader that
    /// dropped the key would write a card that behaves differently from the
    /// script it came from, in the one direction nothing would notice.
    #[test]
    fn the_rest_of_the_condition_family_refuses_by_name() {
        let base = "Name:X\nManaCost:G\nTypes:Creature Elf\nPT:1/1\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | \
             IsPresent$ Creature.YouCtrl | {EXTRA}Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
        for (extra, reason) in [
            (
                "NoResolvingCheck$ True | ",
                "`NoResolvingCheck$`, a clause checked once",
            ),
            ("IsPresent2$ Card.Self | ", "a second `IsPresent2$` clause"),
            ("PresentPlayer$ You | ", "`PresentPlayer$ You`"),
            ("PresentZone$ Graveyard | ", "`PresentZone$ Graveyard`"),
            ("PresentCompare$ EQ0 | ", "`PresentCompare$ EQ0`"),
            (
                "PresentDefined$ Remembered | ",
                "`PresentDefined$ Remembered`",
            ),
        ] {
            let parsed = parse(&base.replace("{EXTRA}", extra));
            assert!(transcode(&parsed, &cats(), None).is_none(), "{extra}");
            assert_eq!(
                refusal_reason(&parsed, &cats(), None).as_deref(),
                Some(reason),
                "{extra}"
            );
        }
    }

    /// A trigger that fires from somewhere other than the battlefield is
    /// refused, rather than quietly relocated to it.
    ///
    /// `AbilityDef::Triggered` carries no zone and the engine collects
    /// triggers off the battlefield, so a `TriggerZones$ Command` read as
    /// an ordinary trigger is an ability that can never fire on a card
    /// claiming `Coverage::Implemented` — worse than the stub it replaced.
    /// The same line without the key is read as it always was, which is the
    /// half that says the refusal is about the zone and not about the
    /// trigger.
    #[test]
    fn a_trigger_that_fires_from_another_zone_is_refused_rather_than_relocated() {
        let elsewhere = "Name:X\nTypes:Creature\nPT:1/1\n\
             T:Mode$ ChangesZone | TriggerZones$ Command | Origin$ Battlefield | \
             Destination$ Graveyard | ValidCard$ Creature.YouCtrl | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1\n";
        let parsed = parse(elsewhere);
        assert!(transcode(&parsed, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("`TriggerZones$ Command`")
        );

        let here = parse(&elsewhere.replace("TriggerZones$ Command", "TriggerZones$ Battlefield"));
        assert!(
            transcode(&here, &cats(), None).is_some(),
            "the same trigger on the battlefield: {:?}",
            refusal_reason(&here, &cats(), None)
        );
    }

    /// A counter word the DSL registry has an id for reaches the card as
    /// the constant, on both the rules that read a counter code.
    ///
    /// The effect and the cost are tested together on purpose: they are
    /// two callers of one function precisely so that `STORAGE` cannot come
    /// out as two different counters, and a test that only exercised one of
    /// them would not notice if that stopped being true. No card prints
    /// storage counters as a *cost* in this direction — the cost half of
    /// this script is written for the shared table and not for a printing.
    #[test]
    fn an_assigned_counter_word_reaches_the_card_as_its_constant() {
        let body = read(
            "Name:Crucible\nManaCost:no cost\nTypes:Land\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ STORAGE | CounterNum$ 1\n\
             A:AB$ Untap | Cost$ AddCounter<1/STORAGE>\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("kind: counters::STORAGE"),
            "the effect: {text}"
        );
        assert!(
            text.contains("PutCounterSelf { kind: counters::STORAGE"),
            "the cost: {text}"
        );
    }

    /// The one `K:` line that is a static ability rather than a bit.
    ///
    /// The reference files "you may choose not to untap" as a keyword
    /// because it takes no parameters; the rules make it a continuous
    /// effect that modifies CR 502.3's turn-based action. So it lands in
    /// `abilities` beside the card's own, and leaves `keywords` alone —
    /// a bit there would be a keyword no engine rule reads.
    #[test]
    fn the_may_not_untap_keyword_is_a_static_ability_and_not_a_bit() {
        let body = read(
            "Name:Bottomless Vault\nManaCost:no cost\nTypes:Land\n\
             K:You may choose not to untap CARDNAME during your untap step.\n\
             A:AB$ Mana | Cost$ T | Produced$ B | Amount$ 1\n",
        );
        assert_eq!(
            body.abilities,
            [
                "static_ability!(Filter::This, Modifier::MayChooseNotToUntap)",
                "mana_ability!(&[Effect::mana(ManaColor::Black, 1)])",
            ]
        );
        assert!(body.keywords.is_empty(), "{:?}", body.keywords);

        // One word off the printed sentence and it is an unread keyword
        // again. The match is the whole line on purpose: every one of the
        // 45 scripts that print this writes it exactly one way, so a
        // looser reading would only ever be reading something else.
        let parsed = parse(
            "Name:X\nTypes:Land\n\
             K:You may choose not to untap CARDNAME during your upkeep.\n",
        );
        assert!(transcode(&parsed, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("keyword `You`")
        );
    }

    /// `AB$ Untap` says what it untaps by what it leaves *out*, and
    /// `Cost$ AddCounter<n/KIND>` is a counter paid rather than a counter an
    /// effect puts on. Devoted Druid prints both in one line, which is why
    /// it is the card read here.
    ///
    /// CR 115.1c is why the untap cannot be one rule with a self-filter: an
    /// ability whose script names no `ValidTgts$` has no target, and
    /// `UntapTarget` would walk an empty `res.targets` and untap nothing.
    #[test]
    fn an_untap_with_no_valid_string_untaps_the_source_that_paid_for_it() {
        let body = read(
            "Name:Devoted Druid\nManaCost:1 G\nTypes:Creature Elf Druid\nPT:0/2\n\
             A:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\n\
             A:AB$ Untap | Cost$ AddCounter<1/M1M1> | SpellDescription$ Untap CARDNAME.\n",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana(ManaColor::Green, 1)])",
                "activated!(cost!(PutCounterSelf { kind: CounterKind::M1M1, n: 1 }), \
                 &[Effect::UntapSelf])",
            ]
        );
    }

    /// `Amount$ X` over `SVar:X:Count$Valid …` is a counted mana amount.
    ///
    /// Cabal Coffers' sentence, and the DSL has been able to say it since
    /// `Amount::CountOf` — Gaea's Cradle is written with it by hand. What was
    /// missing was a reader, which is the shape this report keeps producing:
    /// the top blocker names a rule that exists and a value it cannot say.
    ///
    /// The second half is the one that would be written rather than refused.
    /// A definition that is not a battlefield count is named **by the
    /// definition** and never by the letter, because `Amount$ X` is one
    /// spelling standing for thirty different questions.
    #[test]
    fn a_counted_mana_amount_reads_the_definition_and_not_the_letter() {
        // Cabal Coffers' line with the one subtype this module's fixture
        // knows: `cats()` carries three land types and Swamp is not among
        // them, which is a fact about the fixture and not about the rule.
        let body = read(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ 2 T | Produced$ G | Amount$ X | \
             SpellDescription$ Add {G} for each Forest you control.\n\
             SVar:X:Count$Valid Forest.YouCtrl\n",
        );
        assert!(
            body.statics.contains(
                "static COUNT1: Filter = Filter::And(&[Filter::HasSubtype(subtypes::land::FOREST), \
                 Filter::ControlledByYou]);"
            ),
            "{}",
            body.statics
        );
        assert_eq!(
            body.abilities,
            [concat!(
                "mana_ability!(cost!(\"{2}\", TapSelf), ",
                "&[Effect::mana_dynamic(ManaColor::Green, Amount::CountOf { ",
                "filter: &COUNT1, zone: ZoneSel::Battlefield })])"
            )]
        );

        let elsewhere = parse(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ B | Amount$ X | SpellDescription$ Add.\n\
             SVar:X:Count$ValidGraveyard Creature.Black+YouCtrl\n",
        );
        assert!(transcode(&elsewhere, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&elsewhere, &cats(), None).as_deref(),
            Some("count `Count$ValidGraveyard Creature.Black+YouCtrl`"),
            "the definition is the worklist entry; the letter is not"
        );
    }

    /// `Produced$ Combo U R | Amount$ 2` is a pick **per mana**.
    ///
    /// "Add {U}{U}, {U}{R}, or {R}{R}" is the filter cycle's sentence and it
    /// is not "two mana of any one color" — the two answers may differ, which
    /// is `combination: true` and the reason the land is worth playing.
    /// `Produced$ Any | Amount$ 3` is the neighbouring sentence with one pick
    /// for the whole amount, and the two constructors are what keep them
    /// apart.
    ///
    /// `Combo Any` is the five colours written as one word, so Baxter
    /// Building's "four mana in any combination of colors" is the same rule
    /// as the filter lands' two, and `Combo ColorIdentity` is a *source*
    /// rather than a list — no card can name a commander's colours (CR
    /// 903.4). A combination word that is neither refuses by its own name.
    #[test]
    fn a_combination_picks_once_per_mana_and_any_one_color_picks_once() {
        let filter_land = read(
            "Name:Cascade Bluffs\nTypes:Land\n\
             A:AB$ Mana | Cost$ UR T | Produced$ Combo U R | Amount$ 2 | \
             SpellDescription$ Add {U}{U}, {U}{R}, or {R}{R}.\n",
        );
        assert_eq!(
            filter_land.abilities,
            [concat!(
                "mana_ability!(cost!(\"{U/R}\", TapSelf), ",
                "&[Effect::mana_combination(&[ManaColor::Blue, ManaColor::Red], ",
                "Amount::Fixed(2))])"
            )]
        );

        let any_one = read(
            "Name:Lotus Vale\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ Any | Amount$ 3 | \
             SpellDescription$ Add three mana of any one color.\n",
        );
        assert_eq!(
            any_one.abilities,
            ["mana_ability!(&[Effect::mana_choice_dynamic(ALL_MANA_COLORS, Amount::Fixed(3))])"],
            "one pick for three mana, which `mana_combination` would have \
             asked three times"
        );

        let every_colour = read(
            "Name:Baxter Building\nTypes:Land\n\
             A:AB$ Mana | Cost$ 4 T | Produced$ Combo Any | Amount$ 4 | \
             SpellDescription$ Add four mana in any combination of colors.\n",
        );
        assert_eq!(
            every_colour.abilities,
            [concat!(
                "mana_ability!(cost!(\"{4}\", TapSelf), ",
                "&[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(4))])"
            )]
        );

        let commander = read(
            "Name:Hidden Hideout\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ Combo ColorIdentity | \
             SpellDescription$ Add one mana of any color in your commander's color identity.\n",
        );
        assert_eq!(
            commander.abilities,
            ["mana_ability!(&[Effect::mana_commander_identity()])"]
        );

        // "Add two mana of different colors" is a third sentence again, and
        // the DSL has no room for "different" — so it is refused by the word
        // the corpus writes rather than by the line it sits on.
        let different = parse(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ 1 T | Produced$ Combo AnyDifferent | Amount$ 2 | \
             SpellDescription$ Add two mana of different colors.\n",
        );
        assert!(transcode(&different, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&different, &cats(), None).as_deref(),
            Some("`Mana` combination over `AnyDifferent`")
        );
    }

    /// `Cost$ UR T` is the filter cycle's price, and the letters run together.
    ///
    /// Three things are asserted rather than one, because a rule that saw
    /// "two letters" would get two of them wrong:
    ///
    /// - the pair keeps the **printed order**, `{U/R}` and not `{R/U}`;
    /// - `2W` and `WP` are two letters and neither is a colour pair — a
    ///   `{2/W}` costs two generic as its other half, a `{W/P}` is paid with
    ///   life — so both keep refusing **by name**;
    /// - a doubled pair is not a symbol at all, and `ColorPair::new` asserts
    ///   it, so `WW` has to be refused here rather than turned into a panic
    ///   at the card's compile time. No reference script writes one.
    #[test]
    fn a_hybrid_activation_cost_keeps_the_order_the_card_prints() {
        let body = read(
            "Name:Cascade Bluffs\nTypes:Land\n\
             A:AB$ Mana | Cost$ UR T | Produced$ U | SpellDescription$ Add {U}.\n",
        );
        assert_eq!(
            body.abilities,
            ["mana_ability!(cost!(\"{U/R}\", TapSelf), &[Effect::mana(ManaColor::Blue, 1)])"]
        );

        for token in ["2W", "WP", "WW", "WUB"] {
            let script = parse(&format!(
                "Name:X\nTypes:Land\n\
                 A:AB$ Mana | Cost$ {token} T | Produced$ U | SpellDescription$ Add {{U}}.\n"
            ));
            assert!(
                transcode(&script, &cats(), None).is_none(),
                "`{token}` is not a colour pair"
            );
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(format!("cost `{token}`").as_str()),
                "and it refuses by its own name rather than by the rule's"
            );
        }
    }

    /// `Cost$ Return<1/Forest>` is a permanent the *player* names, so it
    /// comes out as a filter and a `static` above the card, the way a
    /// target's does — and `Return<1/CARDNAME>` is the source and carries no
    /// filter at all. Quirion Ranger and Recurring Nightmare print the two
    /// halves, which is why one rule reads both.
    #[test]
    fn a_return_cost_is_a_filter_unless_it_names_the_card_itself() {
        let body = read(
            "Name:Quirion Ranger\nManaCost:G\nTypes:Creature Elf Ranger\nPT:1/1\n\
             A:AB$ Untap | Cost$ Return<1/Forest> | ValidTgts$ Creature | ActivationLimit$ 1\n",
        );
        assert!(
            body.statics.contains(
                "static COST1: Filter = Filter::And(&[Filter::HasSubtype(subtypes::land::FOREST), \
                 Filter::ControlledByYou]);"
            ),
            "{}",
            body.statics
        );
        assert_eq!(
            body.abilities,
            [
                "activated!(cost!(ReturnToHand(&COST1)), &[Effect::UntapTarget], \
                 target = Some(TargetSpec::Object(&Filter::CREATURE)), \
                 limit = ActivationLimit::PerTurn(1))",
            ]
        );

        let itself = read(
            "Name:X\nManaCost:2 B\nTypes:Enchantment\n\
             A:AB$ Untap | Cost$ Return<1/CARDNAME> | ValidTgts$ Creature\n",
        );
        assert_eq!(
            itself.abilities,
            [
                "activated!(cost!(ReturnSelfToHand), &[Effect::UntapTarget], \
              target = Some(TargetSpec::Object(&Filter::CREATURE)))"
            ],
        );
        assert!(
            itself.statics.is_empty(),
            "the source needs no filter: {}",
            itself.statics
        );
    }

    /// A bracketed cost carries the reference's own prose as its last field,
    /// and that prose has spaces in it. Splitting the cost on whitespace cut
    /// the part in two and refused the card under the leftover, reported as
    /// the cost "artifact>" — which is a part nobody wrote and a card nobody
    /// could have fixed. 731 of the reference's costs are written this way.
    ///
    /// The counter-test is the point of the second half, and it changed its
    /// shape when the reader learned these costs: a sacrifice of something
    /// other than the source used to be refused, and the refusal was the
    /// proof that the tokeniser had handed the whole bracket over in one
    /// piece. It is read now, so the proof is what comes *out* — one `{1}`
    /// and one `Sacrifice`, with the reference's own prose ("another
    /// creature", spaces and all) nowhere in the filter. A tokeniser that
    /// split on whitespace would put it there.
    #[test]
    fn a_cost_is_not_split_inside_its_own_brackets() {
        let body = read(
            "Name:X\nManaCost:no cost\nTypes:Artifact\n\
             A:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME/this artifact> | LifeAmount$ 3\n",
        );
        assert_eq!(
            body.abilities,
            ["activated!(cost!(\"{2}\", TapSelf, SacrificeSelf), &[Effect::gain_life(3)])"]
        );

        let quirion = read(
            "Name:Quirion Ranger\nManaCost:G\nTypes:Creature Elf Ranger\nPT:1/1\n\
             A:AB$ Untap | Cost$ Return<1/Forest/a Forest> | ValidTgts$ Creature\n",
        );
        assert!(
            quirion
                .statics
                .contains("Filter::HasSubtype(subtypes::land::FOREST)"),
            "{}",
            quirion.statics
        );

        let svars = BTreeMap::new();
        let cats = cats();
        let mut tx = Tx {
            svars: &svars,
            cats: &cats,
            tokens: None,
            has_x: false,
            body: CardBody::default(),
            unclaimed: std::cell::RefCell::new(None),
        };
        let cost = tx
            .cost_expr("1 Sac<1/Creature.Other/another creature>")
            .expect("a bracketed sacrifice is one token");
        assert_eq!(cost, "cost!(\"{1}\", Sacrifice(&COST1))");
        assert!(
            tx.body.statics.contains("Filter::Another")
                && tx.body.statics.contains("Filter::ControlledByYou")
                && !tx.body.statics.contains("another creature"),
            "the prose is the reference's own label, not part of the filter: {}",
            tx.body.statics
        );
        assert_eq!(
            tx.unclaimed.into_inner(),
            None,
            "nothing was refused, so nothing has a reason to give"
        );
    }

    /// The other half of the same rule: with a valid-string the untap is the
    /// chosen permanent's, and the ability carries the target it was read
    /// from. Asserted beside the self case so neither can quietly become the
    /// other.
    #[test]
    fn an_untap_with_a_valid_string_untaps_the_chosen_permanent() {
        let body = read(
            "Name:X\nManaCost:1 U\nTypes:Creature Goblin\nPT:1/1\n\
             A:AB$ Untap | Cost$ T | ValidTgts$ Creature | TgtPrompt$ Select target creature\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("Effect::UntapTarget"), "{text}");
        assert!(!text.contains("Effect::UntapSelf"), "{text}");
        assert!(text.contains("target = Some("), "{text}");
    }

    /// Wall of Roots is two rules in one line: a counter the DSL had no name
    /// for, and a sentence that caps the activation.
    ///
    /// `M0M1` is read by shape rather than by name — CR 122.1a is one rule
    /// over an open-ended set of pairs, and the corpus prints eleven of them
    /// — while `P1P1` and `M1M1` keep the constants that spell them, which
    /// are the *same value* and not a second meaning. That is the half worth
    /// asserting both ways: a rule that emitted the general form for +1/+1
    /// would rewrite hundreds of cards to say the same thing longer.
    #[test]
    fn a_wall_that_wears_its_own_counters_reads_as_one_mana_ability() {
        let body = read(
            "Name:Wall of Roots\nManaCost:1 G\nTypes:Creature Plant Wall\nPT:0/5\n\
             K:Defender\n\
             A:AB$ Mana | Cost$ AddCounter<1/M0M1> | Produced$ G | ActivationLimit$ 1\n",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(cost!(PutCounterSelf { kind: CounterKind::Minus { power: 0, \
                 toughness: 1 }, n: 1 }), &[Effect::mana(ManaColor::Green, 1)], \
                 limit = ActivationLimit::PerTurn(1))"
            ]
        );

        // The two Magic prints everywhere keep their names.
        let body = read(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ P1P1 | CounterNum$ 1\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("CounterKind::P1P1"), "{text}");

        // And a limit of more than one is a count, not a flag.
        let body = read(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             A:AB$ Draw | Cost$ 1 | NumCards$ 1 | ActivationLimit$ 2\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("limit = ActivationLimit::PerTurn(2)"),
            "{text}"
        );
    }

    /// A loyalty cost is not an ordinary counter cost. 413 scripts in the
    /// corpus print `Cost$ AddCounter<n/LOYALTY>`, and reading one as a
    /// `PutCounterSelf` would make a planeswalker's `+1` an ability anybody
    /// may activate at instant speed as often as they like — CR 606.3 allows
    /// it once a turn and only when a sorcery could be cast, and
    /// `activated!` says neither.
    ///
    /// Nothing in `cost_expr` knows that. What refuses the card is
    /// `Planeswalker$ True`, which sits on every loyalty ability and is
    /// claimed by no rule, so the refusal happens before the cost is read.
    /// That makes this test a tripwire as much as a check: a rule that
    /// claims the key later has to answer the loyalty question in the same
    /// commit, or this fails.
    #[test]
    fn a_loyalty_cost_does_not_become_an_ordinary_counter_cost() {
        let script = "Name:X\nManaCost:2 W W\nTypes:Legendary Planeswalker Ajani\nLoyalty:4\n\
                      A:AB$ PutCounter | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | \
                      CounterType$ P1P1 | CounterNum$ 1 | ValidTgts$ Creature";
        assert!(refused(script));
        assert_eq!(
            refusal_reason(&parse(script), &cats(), None).as_deref(),
            Some("unclaimed parameter `PutCounter.Planeswalker`")
        );
    }

    /// A pump that grants keywords carries them in the same effect — and
    /// a "keyword" that is really a sentence refuses the card.
    #[test]
    fn a_pump_carries_only_keywords_the_engine_has_a_bit_for() {
        let body = read(
            "Name:X\nManaCost:G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | \
             KW$ Trample & Haste\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("KeywordSet::TRAMPLE.union(KeywordSet::HASTE)"),
            "{text}"
        );

        assert!(refused(
            "Name:X\nManaCost:G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +0 | NumDef$ +0 | \
             KW$ HIDDEN CARDNAME can't block."
        ));
        // Any duration but the default is a different lifetime.
        assert!(refused(
            "Name:X\nManaCost:G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ +1 | Duration$ Permanent"
        ));
        // A pump with no target pumps nothing.
        assert!(refused(
            "Name:X\nManaCost:G\nTypes:Instant\nA:SP$ Pump | NumAtt$ +1 | NumDef$ +1"
        ));
    }

    /// Where a target lives is the effect's business, not the valid
    /// string's: `TargetSpec::Object` enumerates the battlefield only.
    #[test]
    fn a_counterspell_targets_the_stack_and_not_the_battlefield() {
        let body = read(
            "Name:Negate\nManaCost:1 U\nTypes:Instant\n\
             A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.nonCreature\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("TargetSpec::Spell"), "{text}");
        assert!(!text.contains("TargetSpec::Object"), "{text}");
    }

    /// The corpus's `Any` means creature, planeswalker, battle *or player*, and
    /// no `TargetSpec` spans objects and players. Read as `Filter::Any` it
    /// silently produced a burn spell that could not point at a player.
    /// The corpus's `Any` means creature, planeswalker, battle *or player*, so
    /// it is a `TargetSpec`, not a `Filter` — nothing on a player can be
    /// filtered on. Read as `Filter::Any` it silently produced a burn
    /// spell that could not point at a face.
    #[test]
    fn any_target_spans_objects_and_players() {
        let body = read(
            "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("TargetSpec::AnyTarget"), "{text}");
        assert!(!text.contains("Filter::Any"), "{text}");
    }

    /// of these would otherwise generate a card missing half its rules.
    #[test]
    fn an_anthem_is_a_static_ability_on_the_layer_it_belongs_to() {
        let body = read(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.Goblin+Other+YouCtrl | AddPower$ 1 | \
             Description$ Other Goblins you control get +1/+0.\n",
        );
        let a = body.abilities.join("\n");
        // `ModifyPT` *is* the "7c, not 7b" claim now: the macro takes no
        // layer and `Modifier::layer` derives one from the other.
        assert!(a.starts_with("static_ability!("), "{a}");
        assert!(a.contains("Modifier::ModifyPT(1, 0)"), "+1/+0, 7c: {a}");
        assert!(
            a.contains("Filter::Another"),
            "\"other\" is part of the filter: {a}"
        );
        assert!(
            a.contains("Filter::ControlledByYou"),
            "and so is \"you control\": {a}"
        );
    }

    #[test]
    fn one_line_that_moves_two_layers_becomes_two_abilities() {
        // CR 613.1 applies layer 6 before layer 7c, so "get +1/+1 and have
        // flying" is two effects, not one.
        let body = read(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | \
             AddKeyword$ Flying\n",
        );
        assert_eq!(body.abilities.len(), 2, "{:?}", body.abilities);
        let a = body.abilities.join("\n");
        // Each layer is named by its modifier rather than beside it — the
        // macro takes none, and `Modifier::layer` derives it. What order the
        // two are written in says nothing: the engine sorts a continuous
        // effect by its layer, not by where it sat in an ability list.
        assert!(a.contains("Modifier::ModifyPT(1, 1)"), "7c: {a}");
        assert!(
            a.contains("Modifier::AddKeyword(KeywordSet::FLYING)"),
            "6: {a}"
        );
    }

    #[test]
    fn a_static_ability_refuses_what_it_cannot_say() {
        // A keyword that carries data is an ability, not a bit — granting
        // it as a bit would grant a keyword no rule reads.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Equip:2\n"
        ));
        // Setting only one half of P/T is a real card ("base power 4") that
        // `SetPT` cannot express.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | SetPower$ 4\n"
        ));
        // A condition is a rule of its own; unread, it must refuse.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | \
             IsPresent$ Island.YouCtrl\n"
        ));
        // A mode that is not Continuous is not this rule.
        assert!(refused(
            "Name:X\nTypes:Creature\nS:Mode$ CantBlockBy | ValidAttacker$ Card.Self\n"
        ));
    }

    #[test]
    fn a_pump_of_x_reads_the_spells_x_and_keeps_its_sign() {
        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
             SVar:X:Count$xPaid\n",
        );
        let a = body.abilities.join("");
        assert!(a.contains("power: Amount::X"), "{a}");
        assert!(a.contains("toughness: Amount::X"), "{a}");

        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
             SVar:X:Count$xPaid\n",
        );
        // The sign lives in the variant, not in a negated `X` — the engine
        // negates `NegX` at the use site and would double-negate otherwise.
        assert!(body.abilities.join("").contains("Amount::NegX"));
    }

    /// The letter is not the number, and reading it as one gave seven cards
    /// in this pool a pump of nothing.
    #[test]
    fn a_pump_of_a_counted_x_is_refused_and_not_read_as_the_announced_one() {
        // Gaea's Might: domain, which the DSL cannot count. 207 scripts in
        // the reference pump by `X` and every one of them defines `SVar:X`;
        // only 44 define it as the number the player announced.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
             SVar:X:Count$Domain\n"
        ));
        // Irradiate: a count of permanents, and negative, so the sign is not
        // what makes the difference.
        assert!(refused(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
             SVar:X:Count$Valid Artifact.YouCtrl\n"
        ));
        // And a **triggered** ability announces no number at all, so even
        // `Count$xPaid` is `x.unwrap_or(0)` there.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             ValidCard$ Card.Self | Execute$ TrigPump\n\
             SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +X | NumDef$ +X\n\
             SVar:X:Count$xPaid\n"
        ));
        // The refusal names what the value resolves through, so the report
        // ranks the counts and not the letter.
        let script = parse(
            "Name:X\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n\
             SVar:X:Count$Domain\n",
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("pump amount `+X` = `Count$Domain`")
        );
    }

    #[test]
    fn a_refusal_says_which_key_it_choked_on() {
        // The report is only a worklist if it names the thing to build; a
        // second table of each rule's keys would rot, so the transcoder
        // reports what it actually failed to claim.
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | UnlessCost$ 2");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("unclaimed parameter `Draw.UnlessCost`")
        );

        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1");
        assert_eq!(refusal_reason(&script, &cats(), None), None, "read in full");

        // An unknown API is a missing effect, not a missing case in a rule
        // that exists, and is reported as its own kind. Leaving it silent
        // was worse than it looked: the report's fallback then guessed, and
        // named the first API *it* did not recognise — for a land whose
        // only unread line was `DB$ Discard`, that was the `R:Event$ Moved`
        // the transcoder had read perfectly well.
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Animate | Defined$ Self");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("effect `Animate`")
        );

        // A rule that exists but met a value it cannot say says so.
        let script = parse(
            "Name:X\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ 1 | Duration$ Permanent",
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("unreadable value in `Pump`")
        );

        // A static ability names the mode it cannot read, so the worklist
        // ranks `ReduceCost` and `Continuous` as the different work they are.
        let script =
            parse("Name:X\nTypes:Creature\nS:Mode$ CantBlockBy | ValidAttacker$ Card.Self");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("static ability `S: Mode$ CantBlockBy`")
        );
    }

    /// Every refusal names one, and the counter-test is the same set read
    /// the other way round.
    ///
    /// A report whose largest bucket is "refused with no reason recorded" is
    /// a worklist naming no work — the fault `refusal_cause` was written to
    /// fix one level up, and it had simply moved down here: `?` is silent,
    /// so a rule that met a cost, a trigger mode or a missing `SVar` it
    /// could not read refused without saying which. Over the 33666 scripts
    /// of the corpus that bucket is now empty, and this is the part of
    /// that measurement a build can make.
    #[test]
    #[allow(clippy::too_many_lines)] // one case per refusal point, and the list is the point
    fn a_refused_script_always_says_why() {
        // One per refusal point that used to be silent. The expected string
        // is spelled out rather than merely asserted non-empty, because
        // "some reason" is what a fallback produces too.
        let cases: &[(&str, &str)] = &[
            // `Discard<1/…>` is read now, so the refusal moved to the
            // count: `CostPart` carries one object per part, and a cost
            // paid with one card where the script charges two is a
            // discount.
            (
                "Name:X\nTypes:Creature\nA:AB$ Draw | Cost$ Discard<2/Card> | NumCards$ 1",
                "a cost naming `2` objects",
            ),
            (
                "Name:X\nTypes:Creature\nA:AB$ Draw | NumCards$ 1",
                "an activated ability with no `Cost$`",
            ),
            (
                "Name:X\nTypes:Creature\nT:Mode$ Championed | Execute$ TrigDraw\n\
                 SVar:TrigDraw:DB$ Draw | NumCards$ 1",
                "trigger mode `Championed`",
            ),
            (
                "Name:X\nTypes:Creature\n\
                 T:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | Execute$ TrigDraw\n\
                 SVar:TrigDraw:DB$ Draw | NumCards$ 1",
                "trigger at step `Main1`",
            ),
            (
                "Name:X\nTypes:Creature\n\
                 T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Exile | \
                 ValidCard$ Card.Self | Execute$ TrigDraw\n\
                 SVar:TrigDraw:DB$ Draw | NumCards$ 1",
                "trigger on a move from Battlefield to Exile",
            ),
            (
                "Name:X\nTypes:Creature\n\
                 T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
                 ValidCard$ Card.Self | Execute$ NoSuchSVar",
                "`Execute$ NoSuchSVar` names no SVar",
            ),
            (
                "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | SubAbility$ NoSuchSVar",
                "`SubAbility$ NoSuchSVar` names no SVar",
            ),
            (
                "Name:X\nTypes:Instant\n\
                 A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 1 | SubAbility$ Second\n\
                 SVar:Second:DB$ DealDamage | ValidTgts$ Player | NumDmg$ 1",
                "two different targets in one chain",
            ),
            // Both doors a counter noun comes through. A word the registry
            // has not assigned an id to is refused at each of them, and
            // neither door may invent one — `counters::ASSIGNED` is where
            // that decision is made. Hatchling counters are printed on one
            // card in the corpus and have no id.
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ PutCounter | Cost$ T | CounterType$ HATCHLING | CounterNum$ 1",
                "counter `HATCHLING`",
            ),
            (
                "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<1/HATCHLING>",
                "counter `HATCHLING`",
            ),
            // The count is read before the noun, and it is a number or
            // nothing: every one of the 434 `AddCounter<…>` costs in the
            // corpus writes a literal 0-4, and an announced X in a cost is
            // `RemoveCounterSelfX`'s stage, not this one.
            (
                "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<X/M1M1>",
                "counter count `X`",
            ),
            // The two activation limits that are not a count. `GE4` is a
            // threshold on what has already been spent and `X` a number the
            // board works out; five corpus scripts print them between them.
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ Draw | Cost$ T | NumCards$ 1 | ActivationLimit$ GE4",
                "activation limit `GE4`",
            ),
            (
                "Name:X\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | ActivationLimit$ 1",
                "`ActivationLimit$` on a spell line",
            ),
            // A mixed-sign P/T counter is not a counter Magic prints, and
            // `CounterKind` has no way to say one.
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ PutCounter | Cost$ T | CounterType$ P1M1 | CounterNum$ 1",
                "counter `P1M1`",
            ),
            // A return cost carries one permanent, so a count above one is
            // refused by that count rather than paid one short. Fourteen of
            // the corpus's 78 return costs print two, three or X.
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ Untap | Cost$ Return<2/Forest> | ValidTgts$ Creature",
                "a cost naming `2` objects",
            ),
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ Untap | Cost$ Return<X/Forest> | ValidTgts$ Creature",
                "a cost naming `X` objects",
            ),
            // "Doesn't untap" is a rule about *your* untap step and about
            // nothing else this reader can say. Both keys are required
            // rather than defaulted: one script leaves the step off and two
            // replace the untap with a counter removal instead of stopping
            // it, and neither is this modifier.
            (
                "Name:X\nTypes:Artifact\n\
                 R:Event$ Untap | ValidCard$ Card.Self | Layer$ CantHappen",
                "a `doesn't untap` during `any untap step`",
            ),
            (
                "Name:X\nTypes:Artifact\n\
                 R:Event$ Untap | ValidCard$ Card.Self | \
                 ValidStepTurnToController$ You | ReplaceWith$ RepRemoveCounter",
                "an untap replacement on layer `none`",
            ),
            (
                "Name:X\nTypes:Artifact\nR:Event$ Untap | \
                 ValidStepTurnToController$ You | Layer$ CantHappen",
                "a `doesn't untap` with no `ValidCard$`",
            ),
            // The one `transcode` refuses after every rule has been read.
            (
                "Name:X\nTypes:Creature",
                "a script that reads as an empty card",
            ),
        ];
        for (script, why) in cases {
            let parsed = parse(script);
            assert!(
                transcode(&parsed, &cats(), None).is_none(),
                "this case is supposed to be refused: {script}"
            );
            assert_eq!(
                refusal_reason(&parsed, &cats(), None).as_deref(),
                Some(*why),
                "the reason given for: {script}"
            );
        }

        // The counter-test, and the reason the list above is not enough on
        // its own: a `deny` that named the wrong thing would still pass a
        // non-empty check. Each of these is one clause away from a case
        // above and is read in full.
        for script in [
            "Name:X\nTypes:Creature\nA:AB$ Draw | Cost$ T | NumCards$ 1",
            "Name:X\nTypes:Creature\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             ValidCard$ Card.Self | Execute$ TrigDraw\n\
             SVar:TrigDraw:DB$ Draw | NumCards$ 1",
            "Name:X\nTypes:Creature\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ P1P1 | CounterNum$ 1",
            "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<1/M1M1>",
            "Name:X\nTypes:Creature\n\
             A:AB$ Mana | Cost$ T | Produced$ G | ActivationLimit$ 1",
            "Name:X\nTypes:Creature\n\
             A:AB$ PutCounter | Cost$ T | CounterType$ M0M1 | CounterNum$ 1",
            "Name:X\nTypes:Creature\n\
             A:AB$ Untap | Cost$ Return<1/Forest> | ValidTgts$ Creature",
            "Name:X\nTypes:Creature\n\
             A:AB$ Untap | Cost$ Return<1/CARDNAME> | ValidTgts$ Creature",
            "Name:X\nTypes:Artifact\n\
             R:Event$ Untap | ValidCard$ Card.Self | \
             ValidStepTurnToController$ You | Layer$ CantHappen",
            "Name:X\nTypes:Artifact\n\
             R:Event$ Untap | ActiveZones$ Battlefield | ValidCard$ Card.Self | \
             ValidStepTurnToController$ You | Layer$ CantHappen",
        ] {
            let parsed = parse(script);
            assert!(
                transcode(&parsed, &cats(), None).is_some(),
                "the near miss is supposed to be read: {script}"
            );
            assert_eq!(refusal_reason(&parsed, &cats(), None), None, "{script}");
        }
    }

    #[test]
    fn a_zone_change_is_read_as_the_pair_it_is() {
        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature\n",
        );
        assert!(body.abilities.join("").contains("Effect::bounce("));

        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature\n",
        );
        assert!(body.abilities.join("").contains("Effect::exile("));

        let body = read(
            "Name:X\nTypes:Creature\n\
             A:AB$ ChangeZone | Cost$ T | Origin$ Battlefield | Destination$ Exile | Defined$ Self\n",
        );
        assert!(body.abilities.join("").contains("Effect::ExileSource"));
    }

    #[test]
    fn putting_a_creature_in_a_graveyard_is_not_destroying_it() {
        // CR 701.8b: destruction checks indestructible and a zone change does
        // not, so the nearest effect is the wrong effect — a card written
        // this way would quietly kill creatures that survive.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Graveyard | ValidTgts$ Creature\n"
        ));
    }

    /// A spell's `Cost$` carries the card's mana cost *and* whatever the
    /// printing charges beside it, and only the first half has a home on the
    /// face. The branch read neither and refused nothing, so Crop Rotation
    /// shipped as a one-mana tutor that sacrifices no land and Kaervek's
    /// Spite as three mana for five life off a target — both
    /// `Coverage::Implemented`, both offered to a deckbuilder as playable.
    #[test]
    fn an_additional_cost_on_a_spell_is_refused_and_not_dropped() {
        // Crop Rotation.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Cost$ G Sac<1/Land> | Origin$ Library | \
             Destination$ Battlefield | ChangeType$ Land | ChangeNum$ 1"
        ));
        // Kaervek's Spite: two additional costs, and the first of them is
        // one `cost_pieces` can read, so the refusal has to come from the
        // spell branch rather than from a part nobody could map.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ LoseLife | Cost$ B B B Sac<All/Permanent> Discard<0/Hand> | \
             ValidTgts$ Player | LifeAmount$ 5"
        ));
        // The mana half alone is not an additional cost: it is the card's
        // own mana cost, which the face already carries.
        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ LoseLife | Cost$ B B B | ValidTgts$ Player | LifeAmount$ 5",
        );
        assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
        assert!(body.abilities[0].starts_with("spell!("));
        // `XMin1` is "X can't be 0", a printed restriction and not mana —
        // and it carries no brackets, which is the whole reason the reading
        // is an allow-list rather than a hunt for `<…>`.
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ Mill | Cost$ XMin1 X B | NumCards$ 1"
        ));
        // And neither is an announced `X`, which is why this reads the token
        // itself: `cost_pieces` prices an activation and denies a bare `X`,
        // so asking it here would have refused a card over its mana cost.
        let body = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ Draw | Cost$ X U | NumCards$ 1",
        );
        assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
    }

    #[test]
    fn anything_unread_refuses_the_whole_card() {
        // An unknown effect API.
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ Animate | Defined$ Self"
        ));
        // A known API with a parameter no rule claims.
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | UnlessCost$ 2"
        ));
        // A keyword that is data rather than a bit, and that no rule reads.
        // This was `K:Cycling:2` until one rule read it, which is the whole
        // point of the line: the example has to be a keyword nothing here
        // understands *today*, and typecycling is the nearest one — a
        // different sentence (CR 702.29e, a library search) that #51 will
        // take and this assertion will then have to move again.
        assert!(refused(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\nK:TypeCycling:Basic:2"
        ));
        // A line kind with rules in it that this module does not model.
        assert!(refused(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ Exile"
        ));
        // A `SubAbility$` whose SVar is missing.
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | SubAbility$ Missing"
        ));
    }

    /// A vanilla creature has no rules for this module to read, and must not
    /// be reported as a card it understood.
    #[test]
    fn a_card_with_no_rules_lines_is_not_a_transcoded_card() {
        assert!(refused(
            "Name:Grizzly Bears\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2"
        ));
    }

    #[test]
    fn a_valid_string_becomes_the_filter_it_describes() {
        let body =
            read("Name:X\nTypes:Sorcery\nA:SP$ Destroy | ValidTgts$ Creature.YouCtrl+nonToken");
        assert!(body.statics.contains(
            "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Not(&Filter::IsToken)])"
        ));
    }

    /// A token effect names the constant the **ledger** filed the token
    /// under, through the one module both halves of the ledger come out of.
    #[test]
    fn a_token_effect_names_the_constant_the_ledger_filed_it_under() {
        let body = read_with_tokens(
            "Name:Raise the Alarm\nTypes:Instant\n\
             A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You",
        );
        assert!(
            body.abilities[0]
                .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }"),
            "{}",
            body.abilities[0]
        );
    }

    /// "Create two 1/1 white Soldier creature tokens" is one effect with a
    /// number, and one token is not a count of one — `CreateToken` already
    /// means that, and two spellings of the commonest token effect there is
    /// would be two things to keep in step.
    #[test]
    fn a_count_is_written_only_where_there_is_something_to_count() {
        let two = read_with_tokens(
            "Name:Raise the Alarm\nTypes:Instant\n\
             A:SP$ Token | TokenAmount$ 2 | TokenScript$ r_1_1_goblin | TokenOwner$ You",
        );
        assert!(
            two.abilities[0].contains(
                "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_RED, \
                 amount: Amount::Fixed(2) }"
            ),
            "{}",
            two.abilities[0]
        );
        let one = read_with_tokens(
            "Name:X\nTypes:Instant\n\
             A:SP$ Token | TokenAmount$ 1 | TokenScript$ u_1_1_wizard_flying | TokenOwner$ You",
        );
        assert!(
            one.abilities[0].contains(
                "Effect::CreateToken { token: \
                 &generated_tokens::WIZARD_1_1_BLUE_FLYING }"
            ),
            "{}",
            one.abilities[0]
        );
    }

    /// A token the reader refuses refuses the card that makes it. The
    /// alternative is a card that puts an inert permanent on the battlefield
    /// and claims `Implemented` — a Treasure that cannot be sacrificed for
    /// mana is not a Treasure.
    ///
    /// This used to be asserted *of* the Treasure, because a token with an
    /// ability was refused outright. It is now asserted of the shape that is
    /// still refused — a token whose ability makes a token — and the
    /// Treasure is the case below.
    #[test]
    fn a_token_that_cannot_be_read_refuses_the_card() {
        let line = "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin_maker | TokenOwner$ You";
        assert!(refused_with_tokens(line));
        assert_eq!(
            refusal_reason(&parse(line), &cats(), Some(&tokens())).as_deref(),
            Some("token script `r_1_1_goblin_maker`")
        );
    }

    /// And the card that makes an ability-bearing token names it like any
    /// other. The 98 cards that create a Treasure are what this is about:
    /// they were refused, whole, because the Treasure was.
    #[test]
    fn a_card_may_make_a_token_that_carries_an_ability() {
        let body = read_with_tokens(
            "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin_sac | TokenOwner$ You",
        );
        assert!(
            body.abilities[0]
                .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }"),
            "{}",
            body.abilities[0]
        );
    }

    /// The number a player announced is a token amount, and only on a line
    /// that announced one.
    ///
    /// `X` is what 359 of the corpus's `TokenAmount$` values are — 356
    /// scripts, a few of which write it twice — and it is
    /// the same letter for thirty different counts: 58 of those scripts
    /// define `SVar:X:Count$xPaid` — the X paid for — and the rest count
    /// opponents, damage dealt, creatures in a graveyard. `Amount::X` reads
    /// back only the first of them, so the definition is demanded.
    ///
    /// The second half is the one that is invisible at the use site. A
    /// **triggered** ability announces no number at all, so `Amount::X`
    /// there is `x.unwrap_or(0)`: a card that compiles, claims
    /// `Implemented` and makes nothing. Both counter-cases below are
    /// refused by the same rule, and each names what it resolved through so
    /// the report ranks the count rather than the letter.
    #[test]
    fn an_x_is_a_token_amount_only_where_a_player_announced_one() {
        let body = read_with_tokens(
            "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ X G | TokenScript$ r_1_1_goblin \
             | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$xPaid",
        );
        assert!(
            body.abilities[0].contains(
                "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_RED, \
                 amount: Amount::X }"
            ),
            "{}",
            body.abilities[0]
        );

        // The same `X`, counted a way nothing here can say.
        let domain = parse(
            "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ G | TokenScript$ r_1_1_goblin \
             | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$Domain",
        );
        assert_eq!(
            refusal_reason(&domain, &cats(), Some(&tokens())).as_deref(),
            Some("token amount `X` = `Count$Domain`")
        );

        // The right definition on a line that announces nothing.
        let triggered = parse(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
             | Execute$ TrigToken\nSVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin \
             | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$xPaid",
        );
        assert_eq!(
            refusal_reason(&triggered, &cats(), Some(&tokens())).as_deref(),
            Some("token amount `X` = `Count$xPaid`")
        );
    }

    /// An absent `TokenOwner$` is you only where the chain has no player to
    /// mean instead. Rootcast Apprenticeship writes exactly that — "target
    /// player creates a 1/1 green Squirrel creature token", with no
    /// `TokenOwner$` on the line — and reading the absence as "you" there
    /// would hand the token to the wrong side of the table.
    #[test]
    fn an_absent_owner_is_you_only_where_no_player_is_targeted() {
        let body =
            read_with_tokens("Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin");
        assert!(body.abilities[0].contains("Effect::CreateToken"));

        let rootcast = parse(
            "Name:X\nTypes:Instant\nA:SP$ Token | ValidTgts$ Player | TokenScript$ r_1_1_goblin",
        );
        assert_eq!(
            refusal_reason(&rootcast, &cats(), Some(&tokens())).as_deref(),
            Some("a token created under another player's control")
        );
    }

    /// A named owner this cannot say is refused by name, and so is every
    /// parameter no rule claimed — a token that enters tapped and one that
    /// does not are different cards.
    #[test]
    fn a_token_parameter_with_no_rule_refuses_the_card() {
        for (line, why) in [
            (
                "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ Targeted",
                "token owner `Targeted`",
            ),
            (
                "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenTapped$ True",
                "unclaimed parameter `Token.TokenTapped`",
            ),
            (
                "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenAmount$ X",
                "token amount `X`",
            ),
            (
                "A:SP$ Token | TokenOwner$ You",
                "a `Token` effect naming no `TokenScript$`",
            ),
        ] {
            let script = parse(&format!("Name:X\nTypes:Instant\n{line}"));
            assert_eq!(
                refusal_reason(&script, &cats(), Some(&tokens())).as_deref(),
                Some(why),
                "{line}"
            );
        }
    }

    /// A run with no token corpus is not a gap in the DSL, and says so in
    /// those words: a report that ranked a missing directory as the top
    /// blocker would send somebody to write a rule that already exists.
    #[test]
    fn a_run_with_no_token_corpus_is_not_reported_as_a_missing_rule() {
        let script = parse("Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("`Token` with no token scripts to read it against")
        );
    }

    #[test]
    fn the_supported_api_list_matches_what_is_actually_read() {
        for api in SUPPORTED_APIS {
            assert!(is_supported_api(api));
        }
        assert!(!is_supported_api("Animate"));
    }

    /// A counter a permanent arrives under is a replacement effect
    /// (CR 614.1c), so it lands on the face and not in `abilities` — and
    /// a body that says nothing else is still a card.
    #[test]
    fn a_permanent_that_enters_with_counters_says_so_on_its_face() {
        let body = read("Name:X\nTypes:Creature\nK:etbCounter:P1P1:2");
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"]
        );
        assert!(body.abilities.is_empty() && body.keywords.is_empty());
        // The reference's own "there is no condition", with the reminder
        // text behind it, is the same card.
        let spelled_out = read(
            "Name:X\nTypes:Creature\n\
             K:etbCounter:CHARGE:3:no Condition:CARDNAME enters with three charge counters on it.",
        );
        assert_eq!(
            spelled_out.enter_modifiers,
            ["EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(3) }"]
        );
    }

    /// `X` is the one the spell was cast for (CR 107.3m) and nothing else,
    /// which the card's own `SVar:X` is what says.
    ///
    /// Both counter-tests are cards the corpus really prints: Hooded Hydra
    /// means the announced X, and Sautekh Immortal means "for each creature
    /// that died this turn" — and writes that meaning in a field a reader
    /// counting colons would have taken for a condition.
    #[test]
    fn an_x_on_the_way_in_is_read_only_where_the_spell_paid_it() {
        let body = read("Name:X\nTypes:Creature\nK:etbCounter:P1P1:X\nSVar:X:Count$xPaid");
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::X }"]
        );
        assert!(refused(
            "Name:X\nTypes:Creature\nK:etbCounter:P1P1:X\n\
             SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
        ));
        assert!(
            refused(
                "Name:X\nTypes:Creature\n\
                 K:etbCounter:P1P1:X:Elite Troops \u{2014} CARDNAME enters with a +1/+1 counter \
                 on it for each creature that died this turn.\n\
                 SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
            ),
            "the description is not a condition, and reading past it would \
             have taken the X beside it for the spell's"
        );
    }

    /// Equip prints a cost and the rules supply the rest (CR 702.6a), so
    /// that is all the line says and all `equip!` takes.
    #[test]
    fn equip_is_read_as_the_ability_the_rules_define() {
        assert_eq!(
            read("Name:X\nTypes:Artifact Equipment\nK:Equip:2").abilities,
            ["equip!(cost!(\"{2}\"))"]
        );
        // The cost goes through the same parser an `A:` line's does, so a
        // card that equips for something other than mana is the same rule.
        assert_eq!(
            read("Name:X\nTypes:Artifact Equipment\nK:Equip:PayLife<3>").abilities,
            ["equip!(cost!(PayLife(3)))"]
        );
        // And a narrowed one is refused rather than written without its
        // restriction, which would let the Equipment move onto anything.
        let parsed = parse(
            "Name:X\nTypes:Artifact Equipment\n\
             K:Equip:1:Creature.Legendary+YouCtrl:legendary creature",
        );
        assert!(transcode(&parsed, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("an `Equip` that narrows what it may attach to")
        );
    }

    /// Cycling prints a cost and the rules supply the rest (CR 702.29a), and
    /// the string this writes is the one [`crate::landgen`] writes from the
    /// printed text — so a card that both readers can reach comes out the
    /// same either way, which is the only way the two can be allowed to
    /// write the same sentence.
    #[test]
    fn cycling_is_read_as_the_ability_the_rules_define() {
        let want = "activated!(cost!(\"{2}\", DiscardSelf), &[Effect::draw(1)], \
                    zone = ActivationZone::Hand)";
        assert_eq!(
            read("Name:X\nTypes:Land\nK:Cycling:2").abilities,
            [want],
            "the transcoder and the land reader have to agree byte for byte"
        );
        // Spelled out and not defaulted: `ScryfallCard` is what a payload
        // deserializes into, and a `..Default::default()` tail on it is the
        // thing that would stop the compiler naming a new field here.
        let land = crate::scryfall::ScryfallCard {
            id: "id".into(),
            oracle_id: Some("oracle".into()),
            name: "Test Land".into(),
            mana_cost: None,
            type_line: Some("Land".into()),
            oracle_text: Some("Cycling {2} ({2}, Discard this card: Draw a card.)".into()),
            colors: None,
            color_identity: None,
            set: None,
            set_name: None,
            collector_number: None,
            rarity: None,
            layout: None,
            power: None,
            toughness: None,
            loyalty: None,
            card_faces: None,
        };
        assert_eq!(
            crate::landgen::recognize(&land, &cats())
                .expect("a land whose whole text is one cycling line")
                .abilities,
            [want]
        );

        // Space-separated and non-mana costs are the same sentence: the
        // reference writes 57 of them over 306 lines.
        assert_eq!(
            read("Name:X\nTypes:Creature\nK:Cycling:1 U").abilities,
            [
                "activated!(cost!(\"{1}{U}\", DiscardSelf), &[Effect::draw(1)], \
              zone = ActivationZone::Hand)"
            ]
        );
        assert_eq!(
            read("Name:X\nTypes:Creature\nK:Cycling:PayLife<2>").abilities,
            [
                "activated!(cost!(PayLife(2), DiscardSelf), &[Effect::draw(1)], \
              zone = ActivationZone::Hand)"
            ]
        );

        // A fourth field is refused by name. All 306 lines carry three
        // today, so a fourth is a sentence nobody has read.
        let parsed = parse("Name:X\nTypes:Creature\nK:Cycling:2:Island");
        assert!(transcode(&parsed, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("a `Cycling` with a fourth field `Island`")
        );
    }

    /// An Aura is a spell that targets what it will enchant (CR 303.4a) and
    /// arrives attached to it, and the target is named once.
    #[test]
    fn an_aura_is_the_spell_that_attaches_it() {
        let body = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 2 | AddToughness$ 1",
        );
        assert_eq!(
            body.abilities[0],
            "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        );
        assert_eq!(
            body.abilities.len(),
            2,
            "the spell, and the one layer-7c modifier that +2/+1 is"
        );
        // "Enchant creature you control" is a filter the DSL already has a
        // constant for, so it is used twice under that name and declares
        // nothing — and the third field, the printed wording, is prose.
        let constant =
            read("Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature.YouCtrl:creature you control");
        assert!(constant.statics.is_empty());
        assert_eq!(
            constant.abilities[0]
                .matches("&Filter::YOUR_CREATURE")
                .count(),
            2
        );
        // One that has no constant is named once and used twice, rather
        // than written out on both halves of the same sentence.
        let named =
            read("Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature.tapped:tapped creature");
        assert_eq!(named.statics.matches("static ").count(), 1);
        assert_eq!(named.abilities[0].matches("&ENCHANT1").count(), 2);
        // An Aura on a player has nothing for `AttachSelf` to attach to.
        let parsed = parse("Name:X\nTypes:Enchantment Aura\nK:Enchant:Player");
        assert!(transcode(&parsed, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("an `Enchant Player`, which attaches to no object")
        );
    }

    /// "Enchanted creature" and "equipped creature" are one question, and
    /// the answer is the permanent the source is attached to.
    #[test]
    fn what_a_permanent_is_attached_to_is_one_filter_under_two_words() {
        assert_eq!(
            filter("Creature.EnchantedBy"),
            "Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])"
        );
        assert_eq!(
            filter("Creature.EquippedBy"),
            filter("Creature.EnchantedBy")
        );
    }

    /// A condition the DSL cannot say refuses the card rather than dropping
    /// the "if" and placing the counter every time.
    #[test]
    fn a_counter_that_arrives_only_sometimes_is_refused() {
        for script in [
            "Name:X\nTypes:Creature\n\
             K:etbCounter:P1P1:2:CheckSVar$ WasKicked:If CARDNAME was kicked, \
             it enters with two +1/+1 counters on it.",
            "Name:X\nTypes:Creature\n\
             K:etbCounter:P1P1:1:ValidCard$ Card.Self+escaped:CARDNAME escapes with a counter.",
            // And a counter the DSL has no id for is refused exactly as it
            // is on an ability: this is `counter_kind`'s rule, reached from
            // a second place.
            "Name:X\nTypes:Creature\nK:etbCounter:NOTACOUNTER:1",
        ] {
            let parsed = parse(script);
            assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
            assert_eq!(
                refusal_reason(&parsed, &cats(), None).as_deref(),
                Some("keyword `etbCounter`"),
                "and the refusal names the line it stopped on: {script}"
            );
        }
    }
}

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
