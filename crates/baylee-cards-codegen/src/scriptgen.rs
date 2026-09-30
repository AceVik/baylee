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
    // "For each creature that died this turn" (Scavenging Ghoul), counted as
    // the effect applies. Asked before the `X` rule, because `X` is the
    // letter the reference writes it under.
    if svars.get(raw).map(|def| def.trim())
        == Some("Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature")
    {
        return Some("Amount::CreaturesDiedThisTurn".to_string());
    }
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

/// `withFlying` as `Filter::HasKeyword(KeywordSet::FLYING)`, `withoutFlying`
/// as its negation. The reference runs the keyword into the word, spaces
/// removed (`withFirst Strike` is never written; `withFirstStrike` is), so
/// the name is matched against each bit's printed spelling with its spaces
/// taken out.
fn keyword_atom(atom: &str) -> Option<String> {
    let (negated, name) = match atom.strip_prefix("without") {
        Some(rest) => (true, rest),
        None => (false, atom.strip_prefix("with")?),
    };
    let printed = [
        "Flying",
        "First Strike",
        "Double Strike",
        "Deathtouch",
        "Haste",
        "Hexproof",
        "Indestructible",
        "Lifelink",
        "Menace",
        "Reach",
        "Trample",
        "Vigilance",
        "Defender",
    ]
    .into_iter()
    .find(|p| p.replace(' ', "") == name)?;
    let has = format!("Filter::HasKeyword({})", keyword_const(printed)?);
    Some(if negated {
        format!("Filter::Not(&{has})")
    } else {
        has
    })
}

/// "Creatures named Plague Rats": the name in a `named…` atom. A name is a
/// characteristic (CR 201.2), compared as the object carries it now. Only a
/// plain name: the reference also writes counts and limits after one
/// (`namedHedron Alignment/LimitMax`).
fn named_atom(atom: &str) -> Option<&str> {
    atom.strip_prefix("named").filter(|name| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || " '-".contains(c))
    })
}

/// A colour word in a valid-string, and whether it is negated: `Black` is
/// `(false, "Black")` and `nonBlack` is `(true, "Black")`, the second half
/// being the `Color` variant's name.
fn color_atom(atom: &str) -> Option<(bool, &'static str)> {
    let (negated, word) = atom
        .strip_prefix("non")
        .map_or((false, atom), |rest| (true, rest));
    let color = match word {
        "White" => "White",
        "Blue" => "Blue",
        "Black" => "Black",
        "Red" => "Red",
        "Green" => "Green",
        _ => return None,
    };
    Some((negated, color))
}

/// A power or toughness compared with a fixed number (`powerLE2`,
/// `toughnessGE4`), as the `Filter` that asks it. The strict comparisons
/// become the inclusive ones a step over, because the filters compare
/// inclusively and a power is a whole number.
fn stat_atom(atom: &str) -> Option<String> {
    let lower = atom.to_ascii_lowercase();
    let (stat, rest) = if let Some(rest) = lower.strip_prefix("power") {
        ("Power", rest)
    } else {
        ("Toughness", lower.strip_prefix("toughness")?)
    };
    let (cmp, number) = rest.split_at_checked(2)?;
    let n: i16 = number.parse().ok()?;
    let (bound, n) = match cmp {
        "le" => ("AtMost", n),
        "lt" => ("AtMost", n.checked_sub(1)?),
        "ge" => ("AtLeast", n),
        "gt" => ("AtLeast", n.checked_add(1)?),
        _ => return None,
    };
    Some(format!("Filter::{stat}{bound}({n})"))
}

fn plain_number(raw: &str, svars: &BTreeMap<String, String>) -> Option<i64> {
    let raw = raw.trim().trim_start_matches('+');
    raw.parse::<i64>()
        .ok()
        .or_else(|| svars.get(raw)?.trim().parse::<i64>().ok())
}

/// A `PresentCompare$` as the bound a count condition says: `GE2` is at least
/// two, `EQ0` none ("if no creatures are on the battlefield"), `LT3` at most
/// two. An exact count above zero is two bounds at once, which one
/// `Condition` cannot say.
fn count_bound(compare: &str) -> Option<Bound> {
    let (cmp, number) = compare.split_at_checked(2)?;
    let n: u16 = number.parse().ok()?;
    let fits = |n: u16| u8::try_from(n).is_ok().then_some(n);
    Some(match cmp {
        "GE" => Bound::AtLeast(fits(n)?),
        "GT" => Bound::AtLeast(fits(n.checked_add(1)?)?),
        "LE" => Bound::AtMost(fits(n)?),
        "LT" => Bound::AtMost(fits(n.checked_sub(1)?)?),
        "EQ" if n == 0 => Bound::AtMost(0),
        _ => return None,
    })
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

    /// "The next time a red source of your choice would deal damage to you
    /// this turn, prevent that damage" (the Circles of Protection; CR 609.7a,
    /// 615.8), and Reverse Damage's "you gain life equal to the damage
    /// prevented this way" after it (CR 615.5).
    ///
    /// The reference says it in three lines: `ChooseSource` picks, an
    /// `Effect` it puts in the command zone carries a replacement waiting for
    /// the chosen source, and the replacement exiles that effect once it has
    /// prevented. Every line must be this sentence key for key or the card
    /// is refused — a pact's delayed trigger, a chosen colour, a replacement
    /// that does not recheck what was chosen are other sentences.
    fn prevent_from_chosen_source(
        &mut self,
        mut p: Params,
        sub: Option<&str>,
    ) -> Option<Vec<String>> {
        let Some(choices) = p.take("Choices") else {
            return self.deny("`ChooseSource` with no `Choices$`".to_string());
        };
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `ChooseSource.{key}`"));
        }
        // What may be chosen, and the same words as the replacement must
        // recheck them when the damage comes (CR 609.7b). "A source of your
        // choice" is any card or emblem; the engine does not offer an emblem
        // (`prevention::source_options`), and no emblem in the pool deals
        // damage.
        let (filter, recheck) = if choices == "Card,Emblem" {
            (
                "Filter::Any".to_string(),
                "Card.ChosenCardStrict,Emblem.ChosenCard".to_string(),
            )
        } else if choices.contains(',') {
            return self.deny(format!("chosen source `{choices}`"));
        } else {
            let atoms = choices.strip_prefix("Card.").unwrap_or(&choices);
            // "A red source" is a red object: the reference's `RedSource`.
            let read = match atoms.strip_suffix("Source") {
                Some(color) if color_atom(color).is_some() => format!("Card.{color}"),
                Some(_) => return self.deny(format!("chosen source `{choices}`")),
                None => choices.clone(),
            };
            let filter = self.filter_expr(&read)?;
            (filter, format!("Card.ChosenCardStrict+{atoms}"))
        };
        let Some(effect_name) = sub else {
            return self.deny("`ChooseSource` with nothing chosen for".to_string());
        };
        let Some(body) = self.svars.get(effect_name).cloned() else {
            return self.deny(format!("`SubAbility$ {effect_name}` names no SVar"));
        };
        let Some((api, mut effect)) = Params::parse(&body) else {
            return self.deny(format!("`SubAbility$ {effect_name}` is no ability"));
        };
        if api != "Effect" {
            return self.deny("`ChooseSource` followed by something other than its shield".into());
        }
        effect.drop_prose();
        let condition_holds = effect.take("ConditionDefined").as_deref() == Some("ChosenCard")
            && matches!(
                effect.take("ConditionPresent").as_deref(),
                Some("Card" | "Card,Emblem")
            )
            && matches!(
                effect.take("ConditionCompare").as_deref(),
                None | Some("GE1")
            );
        if !condition_holds {
            return self.deny("a chosen-source shield on another condition".to_string());
        }
        let cleanup = effect.take("SubAbility");
        let Some(replacement) = effect.take("ReplacementEffects") else {
            return self.deny("a chosen-source `Effect` with no replacement".to_string());
        };
        if let Some(key) = effect.first_key() {
            return self.deny(format!("unclaimed parameter `Effect.{key}`"));
        }
        if let Some(name) = cleanup {
            let clears = self
                .svars
                .get(&name)
                .and_then(|body| Params::parse(body))
                .is_some_and(|(api, mut c)| {
                    api == "Cleanup"
                        && c.take("ClearChosenCard").as_deref() == Some("True")
                        && c.exhausted()
                });
            if !clears {
                return self.deny(format!("`{name}` after a chosen-source shield"));
            }
        }
        let gain_life = self.chosen_source_replacement(&replacement, recheck)?;
        let sources = self.body.filter_static("SOURCE", &filter);
        Some(vec![format!(
            "Effect::PreventNextFromChosenSource {{ sources: &{sources}, combat_only: false, \
             all_but: 0, gain_life: {gain_life} }}"
        )])
    }

    /// The replacement a chosen-source shield waits with: damage from the
    /// chosen source (`recheck`, its properties spelled as the choice spelled
    /// them) to you, prevented, and then either nothing more or "you gain
    /// life equal to the damage prevented this way". The answer is whether
    /// it gains the life.
    fn chosen_source_replacement(&self, replacement: &str, recheck: String) -> Option<bool> {
        let Some((event, mut shield)) = self.svars.get(replacement).and_then(|b| Params::parse(b))
        else {
            return self.deny(format!("replacement `{replacement}` names no SVar"));
        };
        shield.drop_prose();
        let waits = event == "DamageDone"
            && shield.take("ValidSource") == Some(recheck)
            && shield.take("ValidTarget").as_deref() == Some("You")
            && shield.take("PreventionEffect").as_deref() == Some("True");
        let Some(then) = shield.take("ReplaceWith").filter(|_| waits) else {
            return self.deny("a chosen-source replacement that is not a shield on you".into());
        };
        if let Some(key) = shield.first_key() {
            return self.deny(format!("unclaimed parameter `DamageDone.{key}`"));
        }
        match self.svars.get(&then).and_then(|b| Params::parse(b)) {
            Some((api, mut gain)) if api == "GainLife" => {
                let reads = gain.take("Defined").as_deref() == Some("You")
                    && gain.take("LifeAmount").is_some_and(|x| {
                        self.svars.get(&x).map(String::as_str) == Some("ReplaceCount$DamageAmount")
                    });
                let exile = gain.take("SubAbility");
                if !reads || !gain.exhausted() || !exile.is_some_and(|e| self.exiles_itself(&e)) {
                    return self.deny("a chosen-source shield's life gain".to_string());
                }
                Some(true)
            }
            _ if self.exiles_itself(&then) => Some(false),
            _ => self.deny(format!("`ReplaceWith$ {then}` on a chosen-source shield")),
        }
    }

    /// Whether `name` is the line a used-up command-zone effect exiles
    /// itself with, and nothing more.
    fn exiles_itself(&self, name: &str) -> bool {
        self.svars
            .get(name)
            .and_then(|body| Params::parse(body))
            .is_some_and(|(api, mut z)| {
                api == "ChangeZone"
                    && z.take("Defined").as_deref() == Some("Self")
                    && z.take("Origin").as_deref() == Some("Command")
                    && z.take("Destination").as_deref() == Some("Exile")
                    && z.exhausted()
            })
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
                    "EnchantedBy" | "EquippedBy" | "AttachedBy" => {
                        "Filter::AttachedToBySource".to_string()
                    }
                    "YouCtrl" => "Filter::ControlledByYou".to_string(),
                    "OppCtrl" => "Filter::ControlledByOpponent".to_string(),
                    "ActivePlayerCtrl" => "Filter::ControlledByActivePlayer".to_string(),
                    "YouOwn" => "Filter::OwnedByYou".to_string(),
                    "Other" => "Filter::Another".to_string(),
                    "Self" => "Filter::This".to_string(),
                    "attacking" => "Filter::Attacking".to_string(),
                    "blocking" => "Filter::Blocking".to_string(),
                    "unblocked" => "Filter::Unblocked".to_string(),
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
                    // The two card types a noun cannot say "not" to on its
                    // own ("nonartifact, nonblack creature").
                    "nonArtifact" => "Filter::LacksType(TypeSet::ARTIFACT)".to_string(),
                    "nonEnchantment" => "Filter::LacksType(TypeSet::ENCHANTMENT)".to_string(),
                    "Colorless" => "Filter::IsColorless".to_string(),
                    "" => continue,
                    // A colour word (CR 105.2): "black creatures", "target
                    // green spell", "nonblack creature". A colour is not a
                    // subtype, so it is asked before the subtype arm below,
                    // which would otherwise refuse `Black` as an unknown type.
                    other if color_atom(other).is_some() => {
                        let (negated, color) = color_atom(other).unwrap_or_default();
                        let has =
                            format!("Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))");
                        if negated {
                            format!("Filter::Not(&{has})")
                        } else {
                            has
                        }
                    }
                    other if self.worded_atom(other).is_some() => self.worded_atom(other)?,
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

    /// "Target spell or permanent": `TargetSpec::StackOrBattlefield` over the
    /// valid-string's filter.
    fn spell_or_permanent_target(&mut self, valid: &str) -> Option<String> {
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("TARGET", &expr);
        Some(format!("TargetSpec::StackOrBattlefield(&{name})"))
    }

    /// "Target creature card in your graveyard": a `CardInGraveyard` spec,
    /// whose player is whose graveyard. In a graveyard the reference's
    /// `YouCtrl` means "yours" (a card there is controlled by nobody, and
    /// its owner is whose graveyard it is in); no such qualifier is any
    /// graveyard. The rest of the valid-string is the card's filter.
    fn graveyard_target(&mut self, valid: &str) -> Option<String> {
        let (base, rest) = valid.split_once('.').unwrap_or((valid, ""));
        let mut whose = "PlayerRel::EachPlayer";
        let mut kept = Vec::new();
        for atom in rest.split('+').filter(|a| !a.is_empty()) {
            match atom {
                "YouCtrl" | "YouOwn" => whose = "PlayerRel::You",
                "OppCtrl" | "OppOwn" => whose = "PlayerRel::Opponent",
                other => kept.push(other),
            }
        }
        let filter = if kept.is_empty() {
            self.filter_expr(base)?
        } else {
            self.filter_expr(&format!("{base}.{}", kept.join("+")))?
        };
        let filter = self.body.filter_static("TARGET", &filter);
        Some(format!("TargetSpec::CardInGraveyard(&{filter}, {whose})"))
    }

    /// `Defined$ You` and friends as a `PlayerRel`.
    ///
    /// Two of them mean "that player" of the trigger being read, and what
    /// that is depends on the trigger: the player whose step began for a
    /// `Phase` trigger, the controller of the card that moved for a
    /// `ChangesZone` one. Anywhere else the same words name something this
    /// reader cannot see, and refuse.
    fn player_rel(&self, defined: Option<&str>) -> Option<&'static str> {
        let trigger = self.trigger_mode.as_deref();
        Some(match defined.unwrap_or("You") {
            "You" => "PlayerRel::You",
            "Opponent" | "Player.Opponent" => "PlayerRel::Opponent",
            "Player" => "PlayerRel::EachPlayer",
            // "Enchanted land's controller" (CR 303.4e).
            "EnchantedController" | "Player.EnchantedController" => {
                "PlayerRel::ControllerOfAttached"
            }
            "TriggeredPlayer" if trigger == Some("Phase") => "PlayerRel::ActivePlayer",
            // Last known (CR 603.10a): a land put into a graveyard is
            // controlled by nobody by the time the ability resolves.
            "TriggeredCardController" if trigger == Some("ChangesZone") => {
                "PlayerRel::ControllerOfEvent"
            }
            // The permanent tapped for mana is the event's object, and its
            // controller is the only player who can have tapped it for mana
            // (CR 602.2): "its controller" and "that player" are one seat.
            "TriggeredCardController" | "TriggeredActivator" if trigger == Some("TapsForMana") => {
                "PlayerRel::ControllerOfEvent"
            }
            // The permanent that became tapped is the event's object.
            "TriggeredCardController" if trigger == Some("Taps") => "PlayerRel::ControllerOfEvent",
            // The player a damage trigger's damage was dealt to: the
            // `DamageDone` rule reads only triggers whose target is a
            // player, so `TriggeredTarget` is one.
            "TriggeredTarget" if trigger == Some("DamageDone") => "PlayerRel::DamagedPlayer",
            _ => return None,
        })
    }

    /// The same as [`Self::player_rel`], for an effect on a line that may
    /// *target a player itself*.
    ///
    /// The corpus leaves `Defined$` off when the effect means its own line's
    /// target. Reading the absent key as `You` there is how Piranha Marsh —
    /// "target player loses 1 life" — generated as a land that drains its
    /// own controller.
    ///
    /// Its **own line's**, and never the chain's: a sub-ability inherits no
    /// target from the line in front of it, and says `Defined$ Targeted`
    /// when it means one. Last Caress is the card that proved it — "target
    /// player loses 1 life and you gain 1 life. Draw a card." is a targeting
    /// `LoseLife` followed by a bare `GainLife` and a bare `Draw`, and read
    /// against the chain it generated as a sorcery whose target gained the
    /// life and drew the card while its caster got neither.
    fn player_rel_of(&self, defined: Option<&str>, targets_a_player: bool) -> Option<&'static str> {
        if defined.is_none() && targets_a_player {
            return Some("PlayerRel::Chosen");
        }
        self.player_rel(defined)
    }

    /// [`Self::player_rel_of`], and the two `Defined$` words that name the
    /// chain's target: `Targeted` is the player it targeted (`Chosen`),
    /// `TargetedController` the controller of the object or spell it
    /// targeted (`ControllerOfTarget`, last known, CR 608.2h). Each is read
    /// only against a chain whose target is that kind of thing; a word that
    /// names a target the chain does not have is refused.
    fn player_of_line(
        &self,
        defined: Option<&str>,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<&'static str> {
        let player_target = target == Some("TargetSpec::Player(PlayerRel::Chosen)");
        let object_target = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        match defined {
            Some("Targeted" | "TargetedPlayer") if player_target => Some("PlayerRel::Chosen"),
            Some("TargetedController") if object_target => Some("PlayerRel::ControllerOfTarget"),
            Some("Targeted" | "TargetedPlayer" | "TargetedController") => None,
            other => self.player_rel_of(other, targets_a_player),
        }
    }

    /// One effect and everything its `SubAbility$` chain adds.
    fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let Some((api, mut p)) = Params::parse(spec) else {
            return self.deny("an ability spec with no `$` in it".to_string());
        };
        p.drop_prose();
        let valid = p.take("ValidTgts");
        let targets_here = valid.is_some();
        if let Some(valid) = valid {
            // A card in a graveyard is a different kind of target from a
            // permanent (CR 115.1 names both, and they are chosen from
            // different zones), so the zone the line moves *from* decides
            // the spec before the valid-string is read.
            let spec = if api == "ChangeZone" && p.peek("Origin") == Some("Graveyard") {
                self.graveyard_target(&valid)
            } else {
                // "Target spell or permanent" (the Laces): the stack is a
                // zone a target may be chosen in only where the line says
                // so. The battlefield alone is what every target already
                // is.
                match p.take("TgtZone").as_deref() {
                    None | Some("Battlefield") => self.target_spec(&valid, &api),
                    Some("Stack,Battlefield" | "Battlefield,Stack") => {
                        self.spell_or_permanent_target(&valid)
                    }
                    Some(zone) => return self.deny(format!("a target in `TgtZone$ {zone}`")),
                }
            };
            let Some(spec) = spec else {
                return self.deny(format!("target `{valid}`"));
            };
            if chain.target.get_or_insert(spec.clone()) != &spec {
                return self.deny("two different targets in one chain".to_string());
            }
        }
        let sub = p.take("SubAbility");
        // The chosen-source shield is three lines in the reference and one
        // sentence on the card: its `SubAbility$` is part of the sentence,
        // not the next one, so this rule reads the rest of the chain itself.
        if api == "ChooseSource" {
            let effects = self.prevent_from_chosen_source(p, sub.as_deref())?;
            chain.effects.extend(effects);
            return Some(());
        }
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
        // Whether an absent `Defined$` on this line means a chosen player:
        // only where this line declared the player target itself.
        let targets_a_player =
            targets_here && target.as_deref() == Some("TargetSpec::Player(PlayerRel::Chosen)");
        // "Sacrifice it unless you pay {U}", "counter target spell unless
        // its controller pays {2}": the price is the line's and not its
        // effect's, so it is read here for every API and wraps whatever the
        // line turns out to say.
        let unless = match p.take("UnlessCost") {
            Some(cost) => Some(self.unless(&cost, &mut p, target.as_deref())?),
            None => None,
        };
        // "If that creature would die this turn, exile it instead": a rider
        // on whatever the line does to its target, read for every API.
        let dying = match p.take("ReplaceDyingDefined") {
            Some(defined) => Some(self.exile_if_dies(&defined, target.as_deref())?),
            None => None,
        };
        let at_end = match p.take("AtEOT") {
            Some(what) => Some(self.at_next_end_step(&api, &what, target.as_deref())?),
            None => None,
        };

        let Some(mut effects) = self.effect_of(&api, &mut p, target.as_deref(), targets_a_player)
        else {
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
        effects.extend(dying);
        effects.extend(at_end);
        let effects = match unless {
            Some(unless) => vec![self.unless_wrap(unless, &effects)?],
            None => effects,
        };
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

    /// `ReplaceDyingDefined$` — "if that creature would die this turn,
    /// exile it instead" (Magma Spray), a replacement on the line's own
    /// target for the rest of the turn.
    ///
    /// `Targeted` is the target whatever it is (Scorching Dragonfire's
    /// "that creature or planeswalker"); `ThisTargetedCard.Creature` is
    /// Disintegrate's "if it's a creature", asked of an any-target as the
    /// spell resolves. `Remembered` is "a creature dealt damage this way",
    /// which asks whether damage was dealt, and is refused, as is a
    /// condition on the rider (`ReplaceDyingCondition$`, left unclaimed) and
    /// a line whose target is a player.
    fn exile_if_dies(&mut self, defined: &str, target: Option<&str>) -> Option<String> {
        let Some(target) = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        }) else {
            return self.deny(format!(
                "`ReplaceDyingDefined$ {defined}` with no object target"
            ));
        };
        let exile = format!("Effect::ExileIfDiesThisTurn {{ target: {target} }}");
        match defined {
            "Targeted" => Some(exile),
            "ThisTargetedCard.Creature" => Some(format!(
                "Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{exile}] }}"
            )),
            other => self.deny(format!("`ReplaceDyingDefined$ {other}`")),
        }
    }

    /// `AtEOT$` on a line that pumps its target — "destroy that creature at
    /// the beginning of the next end step" (Stone Giant): a delayed trigger
    /// about the target, `Effect::AtNextEndStep`.
    ///
    /// Only `Destroy` on a targeted `Pump`. A token's or a copy's "sacrifice
    /// it" is about the object the line made, not a target, and "exile it",
    /// "return it to your hand" and the upkeep spellings are other
    /// sentences; each is refused by name.
    fn at_next_end_step(&mut self, api: &str, what: &str, target: Option<&str>) -> Option<String> {
        let aimed = target.is_some_and(|t| {
            !matches!(
                t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        });
        if api != "Pump" || !aimed || what != "Destroy" {
            return self.deny(format!("`AtEOT$ {what}` on `{api}`"));
        }
        Some(
            "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
                .to_string(),
        )
    }

    /// The "unless" of a line: `UnlessCost$` with the keys that qualify it.
    ///
    /// **The payer.** `UnlessPayer$ You` is the source's controller. Absent,
    /// the reference asks the controller of the line's target (its default
    /// is `TargetedController`), which is Mana Leak's "unless its controller
    /// pays" — so an absent payer is read only on a line that targets a
    /// spell or a permanent, and refused on one that targets nothing, where
    /// that default names nobody. Every other payer is refused by name:
    /// `Player` is every player at once, a price no one question can put.
    ///
    /// **The subs.** Without `UnlessResolveSubs$` the rest of the chain runs
    /// either way, which is what wrapping this line alone gives. With it the
    /// rest runs on one answer only (Power Sink's "if that player doesn't,
    /// they tap all lands…"), a shape this does not read yet.
    fn unless(&mut self, cost: &str, p: &mut Params, target: Option<&str>) -> Option<Unless> {
        if let Some(subs) = p.take("UnlessResolveSubs") {
            return self.deny(format!("`UnlessResolveSubs$ {subs}`"));
        }
        let switched = match p.take("UnlessSwitched").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`UnlessSwitched$ {other}`")),
        };
        let payer = p.take("UnlessPayer");
        let targets_a_controlled_object = target.is_some_and(|t| {
            t.starts_with("TargetSpec::Spell(") || t.starts_with("TargetSpec::Object(")
        });
        let payer = match payer.as_deref() {
            Some("You") => "PlayerRel::You",
            // Paralyze's "that player may pay {4}": the enchanted
            // creature's controller (CR 303.4e names the Aura's host).
            Some("EnchantedController") => "PlayerRel::ControllerOfAttached",
            None | Some("TargetedController") if targets_a_controlled_object => {
                "PlayerRel::ControllerOfTarget"
            }
            other => {
                return self.deny(format!(
                    "an unless-cost paid by `{}`",
                    other.unwrap_or("TargetedController")
                ));
            }
        };
        let price = self.unless_price(cost.trim())?;
        Some(Unless {
            payer,
            price,
            switched,
        })
    }

    /// An `UnlessCost$` value as a [`Price`].
    ///
    /// Read by [`Tx::cost_pieces`], the reader an activation cost goes
    /// through, and then held to what the "unless" effects can carry. Plain
    /// generic mana stays an [`Amount`] (`PlayerMayPayOr`); anything with a
    /// colour in it is printed exactly (`PlayerMayPayManaOr`), where it used
    /// to be refused — a `{U}` charged as `{1}` is a card anyone could keep
    /// with a Mountain. One part the player pays by naming an object is
    /// `PlayerMayPayCostOr`. A part that needs no answer (`PayLife<2>`) is
    /// refused even though `CostPart` can hold it: that effect asks by
    /// putting up the list of what may pay, and an empty list is how a
    /// player declines — so a price nobody names an object for would
    /// decline itself every time.
    fn unless_price(&mut self, raw: &str) -> Option<Price> {
        // "Unless its controller pays {X}" (Power Sink): the X announced
        // for the source, and only where the card says that is what X is.
        if raw == "X" {
            let Some(x) = amount(raw, self.svars, self.has_x) else {
                return self.deny("an unless-cost of `X`".to_string());
            };
            return Some(Price::Generic(x));
        }
        let (mana, parts) = self.cost_pieces(raw)?;
        match (mana.as_str(), parts.as_slice()) {
            (m, []) if !m.is_empty() => Some(match generic_mana(m) {
                Some(n) => Price::Generic(format!("Amount::Fixed({n})")),
                None => Price::Printed(m.to_string()),
            }),
            ("", [one]) if asks_for_an_object(one) => Some(Price::Part(one.clone())),
            _ => self.deny(format!("an unless-cost of `{raw}`")),
        }
    }

    /// The line's effects behind its price. A tax runs them on a refusal and
    /// takes them as one effect (a `Sequence` when there are several); a
    /// switched price runs them on a payment and takes the list.
    fn unless_wrap(&mut self, unless: Unless, effects: &[String]) -> Option<String> {
        let Unless {
            payer,
            price,
            switched,
        } = unless;
        let one = match effects {
            [] => return self.deny("an unless-cost on a line with no effect".to_string()),
            [one] => one.clone(),
            many => format!("Effect::Sequence(&[{}])", many.join(", ")),
        };
        let list = effects.join(", ");
        Some(match (price, switched) {
            (Price::Generic(mana), false) => format!(
                "Effect::PlayerMayPayOr {{ player: {payer}, mana: {mana}, effect: &{one} }}"
            ),
            (Price::Generic(mana), true) => format!(
                "Effect::PlayerMayPayThen {{ player: {payer}, mana: {mana}, effects: &[{list}] }}"
            ),
            (Price::Printed(cost), false) => format!(
                "Effect::PlayerMayPayManaOr {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effect: &{one} }}"
            ),
            (Price::Printed(cost), true) => format!(
                "Effect::PlayerMayPayManaThen {{ player: {payer}, cost: mana!(\"{cost}\"), \
                 effects: &[{list}] }}"
            ),
            (Price::Part(part), false) => format!(
                "Effect::PlayerMayPayCostOr {{ player: {payer}, cost: &CostPart::{part}, \
                 effect: &{one} }}"
            ),
            // "You may <sacrifice a creature>. If you do, …" is a price no
            // effect here takes on the paying answer.
            (Price::Part(part), true) => {
                return self.deny(format!("a switched unless-cost of `{part}`"));
            }
        })
    }

    /// One effect API as the `Effect` expressions it stands for.
    ///
    /// Every parameter a rule reads is *taken* from `p`; the caller then
    /// refuses the card if anything is left, which is what stops an ignored
    /// `NoRegen$ True` from generating a card that does the wrong thing.
    // One arm per reference API, and the reasons a reading is what it is
    // live beside the arm that makes it.
    #[allow(clippy::too_many_lines)]
    fn effect_of(
        &mut self,
        api: &str,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        // What the effects below aim at when they take a target. `target` is
        // `None` when the chain declared none at all, which is a different
        // question — `Animate` needs to know, because `Filter::This` binds
        // to the first target if there is one and to the source if not.
        let aimed = target.unwrap_or("TargetSpec::AnyPlayer");
        Some(match api {
            "DealDamage" => {
                let n = self.amount_or_count(&p.take("NumDmg")?)?;
                // "Deals 4 damage to any target and 2 damage to you"
                // (Psionic Blast): the reference gathers both into one
                // simultaneous event and deals it at `DamageResolve`. The
                // engine journals one `DamageDealt` per recipient either
                // way, as it does for `DealDamageEach`, and nothing checks
                // state-based actions between two effects of one
                // resolution, so the two in sequence are the same event.
                if p.take("DamageMap").is_some_and(|v| v != "True") {
                    return None;
                }
                let to = match p.take("Defined").as_deref() {
                    None => aimed.to_string(),
                    // "Deals 1 damage to that player" and every other
                    // player the line names without targeting one.
                    Some(who) => format!("TargetSpec::Player({})", self.player_rel(Some(who))?),
                };
                vec![format!(
                    "Effect::DealDamage {{ amount: {n}, target: {to} }}"
                )]
            }
            // A number, or the X the player announced (Stream of Life's
            // "target player gains X life"): [`amount`] asks both of its
            // questions of an `X`, so a count spelled with the same letter
            // is still refused.
            "GainLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::gain_life({fixed})")],
                    ("PlayerRel::You", None) => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    (who, _) => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = self.amount_or_count(&p.take("LifeAmount")?)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            // The same two readings as `GainLife`: Braingeyser's "target
            // player draws X cards".
            "Draw" => {
                let n = amount(
                    p.take("NumCards").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                match (
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?,
                    n.strip_prefix("Amount::Fixed(")
                        .and_then(|r| r.strip_suffix(')')),
                ) {
                    ("PlayerRel::You", Some(fixed)) => vec![format!("Effect::draw({fixed})")],
                    ("PlayerRel::You", None) => {
                        vec![format!("Effect::DrawCards {{ amount: {n} }}")]
                    }
                    (who, _) => vec![format!(
                        "Effect::DrawCardsFor {{ amount: {n}, who: {who} }}"
                    )],
                }
            }
            // "Target player discards a card": the discarding player
            // chooses, which is what discarding means unless the effect
            // says otherwise (CR 701.9b).
            // "Each player discards their hand" (Wheel of Fortune): every
            // card, nobody choosing.
            "Discard" if p.peek("Mode") == Some("Hand") => {
                p.take("Mode");
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::DiscardHand {{ who: {who} }}")]
            }
            "Discard" if p.peek("Mode") == Some("TgtChoose") => {
                p.take("Mode");
                let n = p
                    .take("NumCards")
                    .as_deref()
                    .unwrap_or("1")
                    .parse::<u8>()
                    .ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardForPlayers {{ who: {who}, count: {n} }}"
                )]
            }
            "Discard" => {
                // Refuse other modes instead of turning a chosen discard
                // into a random one. Unconsumed qualifiers also refuse it.
                if p.take("Mode").as_deref() != Some("Random") {
                    return None;
                }
                let n = amount(
                    p.take("NumCards").as_deref().unwrap_or("1"),
                    self.svars,
                    self.has_x,
                )?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::DiscardRandom {{ who: {who}, count: {n} }}"
                )]
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars, self.has_x)?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
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
                // source when the ability has none. That is the API's own
                // default and so is the right reading of a line with no
                // `Defined$` at all — but `Defined$ Self` says the *source*,
                // and on a line that also targets those are two different
                // permanents.
                //
                // Consumptive Goo is the card that proved it: "{2}{B}{B}:
                // Target creature gets -1/-1 until end of turn. Put a +1/+1
                // counter on this creature." Read as `AddCounter` the counter
                // landed on the *target*, where it cancelled the -1/-1 it was
                // paired with exactly — so the ability resolved, charged four
                // mana and changed nothing at all that a test could see.
                // `AddCounterFilter` over `Filter::This` is the spelling that
                // names the source whatever the ability targets.
                match p.take("Defined").as_deref() {
                    Some("Self") if target.is_some() => {
                        return Some(vec![format!(
                            "Effect::AddCounterFilter {{ filter: &Filter::This, \
                             kind: {kind}, amount: {n} }}"
                        )]);
                    }
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
            "DelayedTrigger" => vec![self.delayed_trigger(p, target)?],
            "Destroy" => {
                // `NoRegen$ True` is **read** and no longer merely consumed.
                // It was vacuous while this engine had no regeneration at
                // all — one door, and every card printing "it can't be
                // regenerated" was right for free — and the day a shield
                // existed the two doors stopped being the same function
                // (CR 701.19c). Any other value is a third thing the DSL
                // cannot say, so it refuses.
                let no_regen = match p.take("NoRegen").as_deref() {
                    None => false,
                    Some("True") => true,
                    Some(_) => return None,
                };
                let verb = if no_regen {
                    "destroy_no_regen"
                } else {
                    "destroy"
                };
                // "Destroy that creature" in a delayed trigger's body: the
                // object it remembers ([`Tx::delayed_trigger`]). Anywhere
                // else the key is left for the unclaimed check to refuse.
                let remembered = self.in_delayed
                    && target.is_none()
                    && matches!(
                        p.peek("Defined"),
                        Some("DelayTriggerRememberedLKI" | "DelayTriggerRemembered")
                    );
                if remembered {
                    p.take("Defined");
                    vec![format!("Effect::{verb}(TargetSpec::EventObject)")]
                } else {
                    vec![format!("Effect::{verb}({aimed})")]
                }
            }
            // "Destroy all lands", "destroy all creatures. They can't be
            // regenerated": every permanent the valid-string names, none of
            // them a target (so an ability that also targets is refused
            // rather than read as a sweep of its target). `NoRegen$` is the
            // same two doors as the single destroy above (CR 701.19c).
            "DestroyAll" => {
                if target.is_some() {
                    return None;
                }
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let verb = match p.take("NoRegen").as_deref() {
                    None => "destroy_all",
                    Some("True") => "destroy_all_no_regen",
                    Some(_) => return None,
                };
                vec![format!("Effect::{verb}(&{filter})")]
            }
            // "X damage to each creature without flying and each player"
            // (Earthquake): `DealDamageEach` for the permanents and a
            // `DealDamage` for the players, which is the one spelling
            // `DealDamageEach` names for the second half. Nothing is
            // targeted, so an ability that also targets is refused.
            // Where the reference deals the damage `DamageMap$` gathered:
            // the `DealDamage` lines before it already did.
            "DamageResolve" => Vec::new(),
            // "Prevent the next N damage that would be dealt to any target
            // this turn" (Samite Healer; CR 615.7): a shield on what the
            // line targets, or on the player `Defined$` names. With
            // neither, nothing is shielded, and that is not a card.
            "PreventDamage" => {
                let n = amount(&p.take("Amount")?, self.svars, self.has_x)?;
                let to = match (p.take("Defined").as_deref(), target) {
                    (Some(who), None) => {
                        format!("TargetSpec::Player({})", self.player_rel(Some(who))?)
                    }
                    (None, Some(aimed)) => aimed.to_string(),
                    _ => return None,
                };
                vec![format!(
                    "Effect::PreventNextDamage {{ target: {to}, amount: {n} }}"
                )]
            }
            // "Prevent all combat damage that would be dealt this turn."
            // The bare line only: the reference narrows it with keys this
            // rule does not claim, and those refuse.
            "Fog" => vec!["Effect::PreventAllCombatDamageThisTurn".to_string()],
            "DamageAll" => {
                if target.is_some() {
                    return None;
                }
                let n = amount(&p.take("NumDmg")?, self.svars, self.has_x)?;
                // The printed words for the two keys below ("each creature
                // and each player"), which say nothing those keys do not.
                p.take("ValidDescription");
                let mut out = Vec::new();
                if let Some(valid) = p.take("ValidCards") {
                    let filter = self.filter_expr(&valid)?;
                    let filter = self.body.filter_static("EACH", &filter);
                    out.push(format!(
                        "Effect::DealDamageEach {{ amount: {n}, filter: &{filter} }}"
                    ));
                }
                if let Some(players) = p.take("ValidPlayers") {
                    let who = self.player_rel(Some(&players))?;
                    out.push(format!(
                        "Effect::DealDamage {{ amount: {n}, target: TargetSpec::Player({who}) }}"
                    ));
                }
                if out.is_empty() {
                    return None;
                }
                out
            }
            "Regenerate" => {
                // Bare `AB$ Regenerate` is "regenerate CARDNAME" — 181 of
                // the reference's 275 lines writing this API name no target
                // at all — so the absent target is the source and not a
                // missing one, the same reading `Untap` makes two arms
                // below. `Defined$` names something else entirely (the
                // enchanted creature, a remembered object; 33 lines) and is
                // refused rather than guessed at, which leaves the 62 that
                // carry a `ValidTgts$` as the targeted half.
                match p.take("Defined").as_deref() {
                    None => {}
                    // "Regenerate enchanted creature" (Regeneration): the
                    // Aura's host, which the filter binds to the source.
                    Some("Enchanted" | "Equipped") if target.is_none() => {
                        return Some(vec![
                            "Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"
                                .to_string(),
                        ]);
                    }
                    Some(_) => return None,
                }
                match target {
                    Some(t) => vec![format!("Effect::regenerate({t})")],
                    None => vec!["Effect::regenerate(TargetSpec::ThisObject)".to_string()],
                }
            }
            "Token" => self.token_effect(p, targets_a_player)?,
            "Investigate" => self.investigate_effect(p, targets_a_player)?,
            "Animate" => self.animate_effect(p, target)?,
            "Pump" => self.pump_effect(p, aimed)?,
            "Effect" => self.static_effect(p, target)?,
            "ChangeZone" => self.change_zone(p, target)?,
            "ChangeZoneAll" => self.shuffle_into_library(p, target, targets_a_player)?,
            // "Look at the top three cards of target player's library and
            // put them back in any order. You may have that player shuffle"
            // (Natural Selection). The ability's controller looks and
            // decides; a count the player announced is refused.
            "RearrangeTopOfLibrary" => {
                let count: u8 = p.take("NumCards")?.parse().ok()?;
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                let mut effects = vec![if who == "PlayerRel::You" {
                    format!("Effect::ReorderTopLibrary {{ count: {count} }}")
                } else {
                    format!("Effect::ReorderTopLibraryOf {{ who: {who}, count: {count} }}")
                }];
                match p.take("MayShuffle").as_deref() {
                    None => {}
                    Some("True") => effects.push(format!(
                        "Effect::MayDo {{ effects: &[Effect::ShuffleLibrary {{ who: {who} }}] }}"
                    )),
                    Some(_) => return None,
                }
                effects
            }
            "Sacrifice" => self.sacrifice_effect(p)?,
            // "Tap enchanted creature" (Paralyze): the host, and not a
            // target.
            // "Tap all lands target player controls" (Mana Short): the
            // permanents of the players `Defined$` names, or of the line's
            // own player target. Without either it is every matching
            // permanent, `TapAll`, and the line must target nothing, for
            // a target it never uses is not a card.
            "TapAll" => {
                let filter = self.filter_expr(&p.take("ValidCards")?)?;
                let defined = p.take("Defined");
                if defined.is_none() && !targets_a_player {
                    if target.is_some() {
                        return None;
                    }
                    return Some(vec![format!("Effect::TapAll {{ filter: &{filter} }}")]);
                }
                let who = self.player_of_line(defined.as_deref(), target, targets_a_player)?;
                vec![format!(
                    "Effect::TapAllOf {{ who: {who}, filter: &{filter} }}"
                )]
            }
            // "That player loses all unspent mana" (CR 106.4).
            "DrainMana" => {
                let who =
                    self.player_of_line(p.take("Defined").as_deref(), target, targets_a_player)?;
                vec![format!("Effect::LoseUnspentMana {{ who: {who} }}")]
            }
            // "You may tap or untap target artifact, creature, or land"
            // (Twiddle): one of the two choices always does nothing, so
            // the choice is a yes or a no (`Effect::ToggleTapTarget`).
            "TapOrUntap" if target.is_some() && p.take("Defined").is_none() => {
                vec!["Effect::MayDo { effects: &[Effect::ToggleTapTarget] }".to_string()]
            }
            "Tap" => match p.take("Defined").as_deref() {
                None => vec!["Effect::TapTarget".to_string()],
                Some("Enchanted" | "Equipped") if target.is_none() => {
                    vec!["Effect::TapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                Some(_) => return None,
            },
            // `AB$ Untap` with no `ValidTgts$` is the source, not a target
            // — Basalt Monolith's "{3}: Untap this artifact". Read as
            // `UntapTarget` it would walk an empty `res.targets` and untap
            // nothing at all, which is a card that compiles, claims
            // `Implemented` and does nothing. Nothing in the pool was
            // written that way (asserted in `untap_tests`); it was one
            // reference script away from being.
            "Untap" => match (p.take("Defined").as_deref(), target) {
                (None, Some(_)) => vec!["Effect::UntapTarget".to_string()],
                (None, None) => vec!["Effect::UntapSelf".to_string()],
                // "Untap enchanted creature" (Instill Energy).
                (Some("Enchanted" | "Equipped"), None) => {
                    vec!["Effect::UntapAll { filter: &Filter::AttachedToBySource }".to_string()]
                }
                _ => return None,
            },
            // "Take an extra turn after this one" (Time Walk; CR 500.7).
            // One turn, and yours: another count or another player is a
            // different sentence.
            "AddTurn" => {
                if p.take("NumTurns").as_deref() != Some("1")
                    || !matches!(p.take("Defined").as_deref(), None | Some("You"))
                {
                    return None;
                }
                vec!["Effect::TakeExtraTurn".to_string()]
            }
            "Counter" => {
                if p.take("TargetType").as_deref() != Some("Spell") {
                    return None;
                }
                vec!["Effect::CounterTargetSpell".to_string()]
            }
            _ => return None,
        })
    }

    /// `DB$ Sacrifice`: "sacrifice it".
    ///
    /// **Bare** is "sacrifice this" — 111 of the corpus's 895 sacrifice
    /// lines, and [`Effect::SacrificeSelf`] says it exactly. With an
    /// `UnlessCost$` it is the Karoo sentence, "sacrifice it unless you
    /// <pay>", 131 more — read by [`Tx::unless`] for every API, since the
    /// price belongs to the line and not to the sacrifice. What it is
    /// **not** is `Defined$`/`SacValid$`: those name somebody else's
    /// permanent ("each player sacrifices a creature"), a player choice this
    /// DSL has no effect for, and reading them as the source would be a
    /// card that sacrifices the wrong permanent under a
    /// `Coverage::Implemented`.
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
        Some(vec!["Effect::SacrificeSelf".to_string()])
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
    fn token_effect(&mut self, p: &mut Params, targets_a_player: bool) -> Option<Vec<String>> {
        // Who gets it. The corpus writes `TokenOwner$ You` on 2220 of its
        // 3610 token lines and leaves the key off on 1154 — and absence is
        // **not** a synonym for you: Rootcast Apprenticeship says "target
        // player creates a 1/1 green Squirrel creature token" with no
        // `TokenOwner$` at all, leaning on its own line's target instead.
        // That is the trap [`Self::player_rel_of`] was written for, so the
        // absent key is read the same way it reads one there: as the target
        // where this line targets a player, and as you where it does not.
        let owner = match p.take("TokenOwner").as_deref() {
            Some("You") => "PlayerRel::You",
            None => self.player_rel_of(None, targets_a_player)?,
            Some(who) => return self.deny(format!("token owner `{who}`")),
        };
        if owner != "PlayerRel::You" {
            return self.deny("a token created under another player's control".to_string());
        }
        let Some(stem) = p.take("TokenScript") else {
            return self.deny("a `Token` effect naming no `TokenScript$`".to_string());
        };
        self.create_tokens(&stem, p.take("TokenAmount").as_deref())
    }

    /// The half of a token effect that is about the token rather than about
    /// the line that asked for it: which script, and how many.
    ///
    /// Its own function because [`Self::investigate_effect`] is the same
    /// question asked in different words, and a second copy of it is a
    /// second chance to answer "how many" differently — which is the whole
    /// argument `effects::applies_to` already makes about a predicate with
    /// three readers. The keys are read by the caller, because the corpus
    /// spells them differently (`TokenAmount$` against `Num$`) and that is
    /// the only difference between the two.
    fn create_tokens(&mut self, stem: &str, raw_amount: Option<&str>) -> Option<Vec<String>> {
        let Some(tokens) = self.tokens else {
            return self.deny("`Token` with no token scripts to read it against".to_string());
        };
        let Some(body) = tokens.body(stem, self.cats) else {
            return self.deny(format!("token script `{stem}`"));
        };
        // `generated_tokens` and not `tokens`: the ledger is the one door,
        // and which half of it a constant is written in is the generator's
        // business — a hand-written token is re-exported from there under
        // the same name.
        let token = format!("&generated_tokens::{}", body.constant);
        let amount = match raw_amount {
            None => None,
            Some(raw) => match amount(raw, self.svars, self.has_x) {
                // A refusal names the `SVar` the amount resolves *through*
                // and not merely the letter, because `X` is what 356 scripts
                // write and each of them means it by a different count —
                // the letter alone ranks one entry that is really thirty.
                None => {
                    return match self.svars.get(raw) {
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

    /// CR 701.16a: "'Investigate' means 'Create a Clue token.'"
    ///
    /// One sentence in the rules, and nothing in the DSL was missing —
    /// `Effect::CreateToken` has been able to say it since tokens existed,
    /// and `tokens::CLUE` is already in the ledger. What was missing is the
    /// *word*: the corpus writes every other token as `Token` with a
    /// `TokenScript$` and writes this one as its own API, because Magic
    /// gives the action a keyword name. That is the fifth time an entry on
    /// the report turned out to be a sentence the DSL could already spell.
    ///
    /// Who investigates is read from **two** keys, because the corpus writes
    /// both: `Defined$ You` on five lines and `ValidPlayer$ You` on two.
    /// Every other value there names somebody else — `Opponent`,
    /// `TargetedController`, `Player.withMostTypeCreature` — and is refused
    /// for the reason [`Self::token_effect`] refuses a token under another
    /// player's control: `Effect::CreateToken` has no room for an owner.
    ///
    /// `Optional$ True` is on two lines and is claimed by nothing here, so
    /// those two refuse themselves. That is the honest-stub rule paying for
    /// itself rather than a case being handled: "you may investigate" is a
    /// question this DSL cannot ask, and reading the key as its absence
    /// would be an inference wearing a reading's clothes.
    fn investigate_effect(
        &mut self,
        p: &mut Params,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        let who = match (p.take("Defined"), p.take("ValidPlayer")) {
            // Both keys on one line would be two answers to one question.
            // No line in the corpus writes both, which is what makes this a
            // refusal rather than a precedence rule nobody can check.
            (Some(_), Some(_)) => {
                return self.deny("`Investigate` naming its player twice".to_string());
            }
            (Some(d), None) | (None, Some(d)) => self.player_rel(Some(&d)),
            (None, None) => self.player_rel_of(None, targets_a_player),
        };
        if who != Some("PlayerRel::You") {
            return self.deny("somebody other than you investigating".to_string());
        }
        self.create_tokens(CLUE_TOKEN_SCRIPT, p.take("Num").as_deref())
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
    /// Two objects are read, each the one `Filter::This` binds to: the
    /// source (`Defined$ Self`) on a chain that targets nothing, and the
    /// target on a line that names no `Defined$` — the Laces' "target spell
    /// or permanent becomes red". `Filter::This` binds to the first target
    /// when there is one and to the source when there is not, so a chain
    /// with both would animate the wrong permanent, and is refused.
    ///
    /// `Duration$ Permanent` is "for the rest of the game"
    /// (`Duration::Indefinitely`), `UntilEndOfCombat` is until end of combat;
    /// without one it is until end of turn.
    ///
    /// `RemoveCreatureTypes$ True` is CR 205.1b's "becomes a [creature type]
    /// artifact creature": the first creature type named replaces the ones it
    /// had (`Modifier::ReplaceCreatureTypes`) and every other type is kept.
    ///
    /// `RemoveLandTypes$ True` is "target land becomes a Forest" (Gaea's
    /// Liege): CR 305.7's setting of a land's subtype, `Modifier::SetLandType`,
    /// for the one basic land type named; a second land type, or none, is
    /// refused. `Duration$ UntilHostLeavesPlay` is "until [this] leaves the
    /// battlefield", `Duration::WhileSourceOnBattlefield`.
    fn animate_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        match (p.take("Defined").as_deref(), target) {
            (Some("Self"), None) | (None, Some(_)) => {}
            _ => {
                self.note("`Animate` of something other than the source".to_string());
                return None;
            }
        }
        let duration = match p.take("Duration").as_deref() {
            None => "Duration::UntilEndOfTurn",
            Some("Permanent") => "Duration::Indefinitely",
            Some("UntilEndOfCombat") => "Duration::UntilEndOfCombat",
            Some("UntilHostLeavesPlay") => "Duration::WhileSourceOnBattlefield",
            Some(other) => {
                self.note(format!("`Animate` lasting `{other}`"));
                return None;
            }
        };
        let replace_creature_types = match p.take("RemoveCreatureTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveCreatureTypes$ {other}`")),
        };
        let set_land_type = match p.take("RemoveLandTypes").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`RemoveLandTypes$ {other}`")),
        };
        let mut out = Vec::new();
        let types = p.take("Types").unwrap_or_default();
        self.animate_types(
            &types,
            (replace_creature_types, set_land_type),
            duration,
            &mut out,
        )?;
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
            out.push(Self::animate_expr(
                &format!(
                    "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
                duration,
            ));
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
            out.push(Self::animate_expr(
                &format!("Modifier::AddKeyword({joined})"),
                duration,
            ));
        }
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr(
                    &format!("Modifier::SetPT({power}, {toughness})"),
                    duration,
                ));
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

    /// Layer 4 of an [`Self::animate_effect`]: the types it becomes. A word
    /// is either a card type or a subtype, and the corpus writes both in one
    /// list. `remove` is (`RemoveCreatureTypes$ True`, `RemoveLandTypes$
    /// True`), each of which needs the one word it replaces with.
    fn animate_types(
        &mut self,
        types: &str,
        remove: (bool, bool),
        duration: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let (replace_creature_types, set_land_type) = remove;
        let mut set = false;
        let mut replaced = false;
        for word in types.split(',').filter(|w| !w.trim().is_empty()) {
            let word = word.trim();
            let modifier = if let Some(types) = card_type_const(word) {
                format!("Modifier::AddType({types})")
            } else if set_land_type && let Some(path) = self.basic_land_type(word) {
                if set {
                    return self.deny("`RemoveLandTypes$` naming two land types".to_string());
                }
                set = true;
                format!("Modifier::SetLandType({path})")
            } else {
                if set_land_type
                    && self
                        .cats
                        .const_path_of(baylee_core::types::SubtypeKind::Land, word)
                        .is_some()
                {
                    return self.deny(format!("`RemoveLandTypes$` into `{word}`"));
                }
                let Some(path) = self.cats.const_path(word) else {
                    self.note(format!("`Animate` into `{word}`"));
                    return None;
                };
                let creature_type = self
                    .cats
                    .const_path_of(baylee_core::types::SubtypeKind::Creature, word)
                    .is_some();
                if replace_creature_types && creature_type && !replaced {
                    replaced = true;
                    format!("Modifier::ReplaceCreatureTypes({path})")
                } else {
                    format!("Modifier::AddSubtype({path})")
                }
            };
            out.push(Self::animate_expr(&modifier, duration));
        }
        if replace_creature_types && !replaced {
            return self.deny("`RemoveCreatureTypes$` naming no creature type".to_string());
        }
        if set_land_type && !set {
            return self.deny("`RemoveLandTypes$` naming no basic land type".to_string());
        }
        Some(())
    }

    /// One layer of an [`Self::animate_effect`], as the `Effect` expression.
    ///
    /// No layer is passed in because none is written out: `Effect::continuous`
    /// derives it from the modifier the way CR 613.1 does, so the emitter
    /// cannot name a layer that disagrees with what it is applying.
    fn animate_expr(modifier: &str, duration: &str) -> String {
        format!("Effect::continuous(&Filter::This, {modifier}, {duration})")
    }

    /// One side of a pump, refused by what its value resolves *through*.
    ///
    /// The same rule as the token amount's: the letter is what 356 scripts
    /// write and each means it by a different count, so an entry naming `X`
    /// would rank thirty questions as one.
    fn pump_side(&mut self, raw: &str) -> Option<String> {
        if let Some(a) = pump_amount(raw, self.svars, self.has_x) {
            return Some(a);
        }
        let trimmed = raw.trim();
        let Some(how) = self
            .svars
            .get(trimmed.trim_start_matches(['+', '-']))
            .cloned()
        else {
            return self.deny(format!("pump amount `{raw}`"));
        };
        // A pump that **counts**. "Add {B} for each Swamp you control" and
        // "gets -1/-1 for each artifact you control" are one reading with a
        // sign in front of it, and [`Self::counted_amount`] has been able to
        // say the first since `Amount::CountOf`. Nothing read the second,
        // because the letter is not the number and the sign had nowhere to
        // live: `Amount::Fixed` holds a `u32`, and the two negatives the DSL
        // had are each a variant of their own magnitude.
        //
        // `Amount::Negated` is that sign as a wrapper rather than a negative
        // twin of every count there is, so the magnitude is said once. The
        // positive side comes with it and is the larger half by a long way —
        // 148 reference scripts against 26 — because a pump counting upwards
        // is what most of them print.
        match self.count_expr(&how) {
            Some(inner) if trimmed.starts_with('-') => Some(format!("Amount::Negated(&{inner})")),
            Some(inner) => Some(inner),
            None => self.deny(format!("pump amount `{raw}` = `{how}`")),
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
        // A pump that names nobody and targets nothing is the source's own:
        // the reference defaults an absent `Defined$` to the card itself,
        // and 175 of its `AB$ Pump` lines are written that way — Shivan
        // Dragon's "{R}: This creature gets +1/+0". A pump that moves
        // nothing at all is a placeholder some other line does the work
        // for (`SP$ Pump | StackDescription$ None`), and pumping the source
        // by nought would claim a card that does nothing. A spell has no
        // self to pump once it resolves, so its bare pump stays refused.
        let empty = power == "Amount::Fixed(0)"
            && toughness == "Amount::Fixed(0)"
            && keywords == "KeywordSet::EMPTY";
        let defined = match p.take("Defined") {
            None if target == "TargetSpec::AnyPlayer" && !self.on_a_spell && !empty => {
                Some("Self".to_string())
            }
            other => other,
        };
        // "Enchanted creature gets +1/+0 until end of turn" on an Aura's own
        // activated ability (Firebreathing): the object the source is
        // attached to, which `Filter::AttachedToBySource` binds to the
        // resolving ability's source.
        let whom = match defined.as_deref() {
            Some("Self") => Some("Filter::This"),
            Some("Enchanted" | "Equipped") => Some("Filter::AttachedToBySource"),
            _ => None,
        };
        if let Some(whom) = whom {
            return Some(vec![format!(
                "Effect::PumpFilter {{ filter: &{whom}, controlled_by: None, \
                 power: {power}, toughness: {toughness}, keywords: {keywords}, \
                 duration: Duration::UntilEndOfTurn }}"
            )]);
        }
        Some(match defined.as_deref() {
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

    /// An `Effect` line: a static ability that lasts the turn, read by the
    /// one sentence its static says. Every other static stays refused.
    fn static_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        let statics = p.peek("StaticAbilities")?.trim().to_string();
        let body = self.svars.get(&statics)?.clone();
        let (mode, _) = Params::parse(&body)?;
        match mode.as_str() {
            "CantBlockBy" => self.unblockable_effect(p, target),
            "CantRegenerate" => self.cant_regenerate_effect(p, target),
            _ => None,
        }
    }

    /// "It can't be regenerated this turn" (Disintegrate, Carbonize): an
    /// `Effect` whose one static says no regeneration applies to what it
    /// remembers, which is the line's target, and which ends when that
    /// leaves the battlefield — `Effect::CantBeRegeneratedThisTurn`, kept
    /// for that object only (CR 400.7, 701.19c).
    ///
    /// "If it's a creature" (`ConditionDefined$ ParentTarget |
    /// ConditionPresent$ Creature`) is asked of the target as the line
    /// resolves. "A creature dealt damage this way" (`Remembered.Creature`
    /// after `RememberDamaged$`) asks whether damage was dealt, and is
    /// refused.
    fn cant_regenerate_effect(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
    ) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantRegenerate"
            || st.take("ValidCard").as_deref() != Some("Card.IsRemembered")
            || !st.exhausted()
        {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let target = target.filter(|t| {
            !matches!(
                *t,
                "TargetSpec::Player(PlayerRel::Chosen)" | "TargetSpec::AnyPlayer"
            )
        })?;
        if !matches!(
            p.take("RememberObjects").as_deref(),
            Some("Targeted" | "ParentTarget")
        ) {
            return None;
        }
        let effect = format!("Effect::CantBeRegeneratedThisTurn {{ target: {target} }}");
        let creature = match (p.peek("ConditionDefined"), p.peek("ConditionPresent")) {
            (None, None) => false,
            (Some("ParentTarget" | "Targeted"), Some("Creature")) => {
                p.take("ConditionDefined");
                p.take("ConditionPresent");
                true
            }
            _ => return None,
        };
        Some(vec![if creature {
            format!("Effect::IfTargetMatches {{ filter: &Filter::CREATURE, then: &[{effect}] }}")
        } else {
            effect
        }])
    }

    /// `ChangeZoneAll` into a library with a shuffle, from hands and
    /// graveyards: Timetwister's "each player shuffles their hand and
    /// graveyard into their library", and "target player shuffles their
    /// graveyard into their library".
    ///
    /// Whose cards: `ChangeType$ Card` with no player named is every
    /// player's, each into their own library; `Card.YouOwn` is yours; a
    /// `Defined$` or a player target names them. Every other change —
    /// battlefield, exile, a library position, a random pick, a type other
    /// than any card — is refused.
    fn shuffle_into_library(
        &mut self,
        p: &mut Params,
        target: Option<&str>,
        targets_a_player: bool,
    ) -> Option<Vec<String>> {
        if p.take("Destination").as_deref() != Some("Library")
            || p.take("Shuffle").as_deref() != Some("True")
        {
            return None;
        }
        let origin = p.take("Origin")?;
        let (hand, graveyard) = match origin.as_str() {
            "Hand" => (true, false),
            "Graveyard" => (false, true),
            "Hand,Graveyard" | "Graveyard,Hand" => (true, true),
            _ => return None,
        };
        // The reference's "from every zone listed" rather than one of them:
        // what two origins mean on a card.
        if hand && graveyard {
            p.take("UseAllOriginZones");
        }
        let defined = p.take("Defined");
        let who = match p.take("ChangeType").as_deref() {
            Some("Card.YouOwn") if defined.is_none() && target.is_none() => {
                "PlayerRel::You".to_string()
            }
            Some("Card") | None if defined.is_some() || targets_a_player => self
                .player_of_line(defined.as_deref(), target, targets_a_player)?
                .to_string(),
            Some("Card") if target.is_none() => "PlayerRel::EachPlayer".to_string(),
            _ => return None,
        };
        Some(vec![format!(
            "Effect::ShuffleIntoLibrary {{ who: {who}, hand: {hand}, graveyard: {graveyard} }}"
        )])
    }

    /// "Target creature can't be blocked this turn" (Dwarven Warriors,
    /// Rogue's Passage, Infiltrate): an `Effect` whose one static says
    /// nothing may block what it remembers, and which ends when that leaves
    /// the battlefield. The engine's word for it is the keyword, granted for
    /// the rest of the turn — to the targets, or to the source itself.
    /// Every other `Effect` is its own sentence and stays refused.
    fn unblockable_effect(&mut self, p: &mut Params, target: Option<&str>) -> Option<Vec<String>> {
        let statics = p.take("StaticAbilities")?;
        let body = self.svars.get(statics.trim())?.clone();
        let (mode, mut st) = Params::parse(&body)?;
        st.drop_prose();
        if mode != "CantBlockBy"
            || st.take("ValidAttacker").as_deref() != Some("Card.IsRemembered")
            || !st.exhausted()
        {
            return None;
        }
        let ends = p.take("ExileOnMoved").or_else(|| p.take("ForgetOnMoved"));
        if ends.as_deref() != Some("Battlefield") || p.take("Duration").is_some() {
            return None;
        }
        p.take("IsCurse");
        let keywords = "KeywordSet::UNBLOCKABLE";
        Some(match p.take("RememberObjects").as_deref() {
            Some("Targeted") if target.is_some_and(|t| t != "TargetSpec::AnyPlayer") => {
                vec![format!(
                    "Effect::PumpTarget {{ power: Amount::Fixed(0), toughness: \
                     Amount::Fixed(0), keywords: {keywords}, duration: \
                     Duration::UntilEndOfTurn }}"
                )]
            }
            Some("Self") if !self.on_a_spell => vec![format!(
                "Effect::PumpFilter {{ filter: &Filter::This, controlled_by: None, \
                 power: Amount::Fixed(0), toughness: Amount::Fixed(0), \
                 keywords: {keywords}, duration: Duration::UntilEndOfTurn }}"
            )],
            _ => return None,
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
            // "Return target card from your graveyard to your hand"
            // (Regrowth): the target is a `CardInGraveyard`, which the chain
            // read off `Origin$` before it read the valid-string.
            ("Graveyard", "Hand", false) if target.starts_with("TargetSpec::CardInGraveyard") => {
                vec![format!("Effect::GraveyardToHand {{ target: {target} }}")]
            }
            // "Return target creature card from your graveyard to the
            // battlefield" (Resurrection): it enters under the control of
            // the player whose effect put it there (CR 110.2a), which is
            // `owner_control: false`. A line that says otherwise carries a
            // key (`GainControl$`, `WithCountersType$`) this does not claim.
            ("Graveyard", "Battlefield", false)
                if target.starts_with("TargetSpec::CardInGraveyard") =>
            {
                vec![format!(
                    "Effect::GraveyardToBattlefield {{ target: {target}, owner_control: false, \
                     counters: None }}"
                )]
            }
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
        // "Its controller adds an additional {R}" (Gauntlet of Might): mana
        // in another player's pool, which is fixed mana or nothing.
        let defined = p.take("Defined");
        if let Some(who) = defined.as_deref().filter(|d| *d != "You") {
            let who = self.player_rel(Some(who))?;
            return self.mana_for(p, who);
        }
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

    /// [`Self::mana_effect`] for a pool other than the controller's:
    /// `Effect::AddManaFor`, one per colour, of a fixed amount and with no
    /// restriction. Anything else is refused by name.
    fn mana_for(&mut self, p: &mut Params, who: &str) -> Option<Vec<String>> {
        let produced = p.take("Produced")?;
        if p.peek("RestrictValid").is_some() {
            return self.deny("restricted mana for another player".to_string());
        }
        let raw = p.take("Amount").unwrap_or_else(|| "1".to_string());
        let Some(amount) = plain_number(&raw, self.svars).and_then(|n| u16::try_from(n).ok())
        else {
            return self.deny(format!("mana for another player of `Amount$ {raw}`"));
        };
        let mut out = Vec::new();
        for symbol in produced.split_whitespace() {
            let color = match symbol {
                "W" => "ManaColor::White",
                "U" => "ManaColor::Blue",
                "B" => "ManaColor::Black",
                "R" => "ManaColor::Red",
                "G" => "ManaColor::Green",
                "C" => "ManaColor::Colorless",
                other => return self.deny(format!("mana for another player of `{other}`")),
            };
            out.push(format!(
                "Effect::AddManaFor {{ who: {who}, color: {color}, amount: {amount} }}"
            ));
        }
        Some(out)
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
        match self.count_expr(&def) {
            Some(expr) => Some(expr),
            // Named by what it resolves *through* and never by its own
            // spelling: `Amount$ X` is one letter standing for thirty
            // different questions, and the definition is the one of them this
            // card is asking.
            None => self.deny(format!("count `{}`", def.trim())),
        }
    }

    /// A `Count$Valid …` definition as an [`Amount::CountOf`] expression.
    ///
    /// Silent on a definition it cannot read, because its two callers refuse
    /// in different words and a refusal is a worklist entry.
    /// [`Self::counted_amount`] is reading a mana line and [`Self::pump_side`]
    /// a pump; a pump filed under "mana amount" sends whoever reads the
    /// report to a rule the card never touched, and a wrong reason travels
    /// further than a wrong reading because nothing downstream can check it.
    ///
    /// The definition is passed in rather than the `SVar` name so that the
    /// caller decides what an *undefined* name is called as well.
    fn count_expr(&mut self, def: &str) -> Option<String> {
        let valid = def.trim().strip_prefix("Count$Valid ")?;
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNT", &expr);
        Some(format!(
            "Amount::CountOf {{ filter: &{name}, zone: ZoneSel::Battlefield }}"
        ))
    }

    /// The atoms that are one word of printed text each, and the filter each
    /// is:
    ///
    /// - a name ("creatures named Plague Rats", [`named_atom`]);
    /// - a number the printed words compare against ("power 2 or less",
    ///   [`stat_atom`]), or "less than this creature's power" where `X` is
    ///   the source's power ([`Self::source_power_atom`]);
    /// - a mana value against a number or the announced X
    ///   ([`Self::cmc_atom`]);
    /// - "each creature with flying", "each creature without flying"
    ///   (Hurricane, Earthquake): a keyword the engine has a bit for
    ///   ([`keyword_atom`]). A keyword it has none for is a rule, not a flag,
    ///   and stays refused.
    fn worded_atom(&self, atom: &str) -> Option<String> {
        if let Some(name) = named_atom(atom) {
            return Some(format!("Filter::Named({name:?})"));
        }
        stat_atom(atom)
            .or_else(|| self.source_power_atom(atom))
            .or_else(|| self.cmc_atom(atom))
            .or_else(|| keyword_atom(atom))
    }

    /// `powerLTX` and `toughnessLTX` where `X` is `Count$CardPower`, the
    /// source's own power: "with power less than this creature's power",
    /// Stone Giant's "with toughness less than Stone Giant's power". Any
    /// other comparison or `X` is refused.
    fn source_power_atom(&self, atom: &str) -> Option<String> {
        if self.svars.get("X").map(|x| x.trim()) != Some("Count$CardPower") {
            return None;
        }
        match atom {
            "powerLTX" => Some("Filter::PowerLessThanSourcePower".to_string()),
            "toughnessLTX" => Some("Filter::ToughnessLessThanSourcePower".to_string()),
            _ => None,
        }
    }

    /// `cmcLE2`, `cmcGE4`, `cmcEQX`: a mana value against a fixed number,
    /// or against the X announced for this spell or ability — only where
    /// `X` is that announcement ([`amount`] gives the reason), since the
    /// filter reads it off the source and a trigger announced none.
    fn cmc_atom(&self, atom: &str) -> Option<String> {
        let (cmp, number) = atom.strip_prefix("cmc")?.split_at_checked(2)?;
        if number == "X" {
            let announced =
                self.has_x && self.svars.get("X").map(String::as_str) == Some("Count$xPaid");
            return match cmp {
                "LE" if announced => Some("Filter::CmcAtMostX".to_string()),
                "EQ" if announced => Some("Filter::CmcExactlyX".to_string()),
                _ => None,
            };
        }
        let n: u32 = number.parse().ok()?;
        Some(match cmp {
            "LE" => format!("Filter::CmcAtMost({n})"),
            "LT" => format!("Filter::CmcAtMost({})", n.checked_sub(1)?),
            "GE" => format!("Filter::CmcAtLeast({n})"),
            "GT" => format!("Filter::CmcAtLeast({})", n.checked_add(1)?),
            "EQ" => format!("Filter::And(&[Filter::CmcAtMost({n}), Filter::CmcAtLeast({n})])"),
            _ => return None,
        })
    }

    /// [`amount`], or else a count of permanents ([`Self::count_expr`]):
    /// Karma's "damage equal to the number of Swamps they control". A count
    /// is read as the ability resolves, like every other `Amount`.
    fn amount_or_count(&mut self, raw: &str) -> Option<String> {
        if let Some(n) = amount(raw, self.svars, self.has_x) {
            return Some(n);
        }
        let def = self.svars.get(raw.trim())?.clone();
        self.count_expr(&def)
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
            } else if let Some((n, kind)) = token
                .strip_prefix("SubCounter<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.split_once('/'))
            {
                // "Remove a corpse counter from this creature" as a cost:
                // a fixed number of one kind, from the source. Refused by
                // name: a count that is not a number (`X` is announced, and
                // "any number" is chosen), a loyalty cost (a loyalty
                // ability's, CR 606.4, which only a main phase with an empty
                // stack may activate, once a turn, CR 606.3), and the longer
                // forms that name where the counters come from ("from a
                // creature you control").
                let Ok(n) = n.parse::<u16>() else {
                    return self.deny(format!("counter count `{n}`"));
                };
                if kind.contains('/') {
                    return self.deny(format!("a counter cost from `{kind}`"));
                }
                if kind == "LOYALTY" {
                    return self.deny("a loyalty cost".to_string());
                }
                let Some(kind) = counter_kind(kind) else {
                    return self.deny(format!("counter `{kind}`"));
                };
                parts.push(format!("RemoveCounterSelf {{ kind: {kind}, n: {n} }}"));
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
    /// Five spellings of one shape — `Sac<1/…>`, `Discard<1/…>`,
    /// `tapXType<1/…>`, `Return<1/…>`, `ExileFromGrave<1/…>` — and they were
    /// worth writing once rather than five times because what differs
    /// between them is two facts: which `CostPart` they are, and whether the
    /// object has to be one the payer controls.
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
    /// all, and the engine reads that zone by whose hand it is. An exile
    /// from the graveyard is the same case one pile over, and
    /// `ExileFromGrave<1/CARDNAME>` — eternalize's "exile this card from
    /// your graveyard" — is refused by name, because it is paid from a zone
    /// no ability is activated from yet.
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
            "ExileFromGrave" => ("ExileFromGraveyard", false),
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

    /// `T:Mode$ DamageDone` — "whenever this creature deals [combat] damage
    /// to [a player | an opponent]" — and `DamageDoneOnce`, "whenever this
    /// creature is dealt damage".
    ///
    /// `DamageDone` needs a source and a player target: `Player` in combat
    /// is `DealsCombatDamageToPlayer`, `Opponent` is
    /// `DealsCombatDamageToOpponent` in combat and `DealsDamageToOpponent`
    /// out of it (Hypnotic Specter). Damage to a player out of combat, to a
    /// creature, or to anything else is refused by name.
    ///
    /// `DamageDoneOnce` is the reference's "once however many sources",
    /// which is what the rules make of simultaneous damage (CR 510.2,
    /// 603.2c) and what `Trigger::DealtDamage` does; it is read for a
    /// permanent dealt damage by anything, and refused for a player or with
    /// a source or combat named.
    fn damage_trigger(&mut self, p: &mut Params, once: bool) -> Option<String> {
        let target = p.take("ValidTarget");
        let source = p.take("ValidSource");
        let combat = match p.take("CombatDamage").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`CombatDamage$ {other}`")),
        };
        let filter_of = |this: &mut Self, valid: &str| -> Option<String> {
            if valid == "Card.Self" {
                return Some("&Filter::This".to_string());
            }
            let expr = this.filter_expr(valid)?;
            Some(format!("&{}", this.body.filter_static("TRIGGER", &expr)))
        };
        if once {
            let Some(target) = target else {
                return self.deny("a `DamageDoneOnce` trigger with no `ValidTarget$`".to_string());
            };
            if source.is_some() || combat {
                return self.deny("a `DamageDoneOnce` trigger naming its source".to_string());
            }
            if target
                .split(['.', ','])
                .any(|w| matches!(w, "You" | "Player" | "Opponent"))
            {
                return self.deny(format!("damage dealt to `{target}` at once"));
            }
            let filter = filter_of(self, &target)?;
            return Some(format!("Trigger::DealtDamage({filter})"));
        }
        let Some(source) = source else {
            return self.deny("a `DamageDone` trigger with no `ValidSource$`".to_string());
        };
        let filter = filter_of(self, &source)?;
        match (target.as_deref(), combat) {
            (Some("Player"), true) => Some(format!("Trigger::DealsCombatDamageToPlayer({filter})")),
            (Some("Opponent" | "Player.Opponent"), true) => {
                Some(format!("Trigger::DealsCombatDamageToOpponent({filter})"))
            }
            (Some("Opponent" | "Player.Opponent"), false) => {
                Some(format!("Trigger::DealsDamageToOpponent({filter})"))
            }
            (other, _) => self.deny(format!(
                "damage dealt to `{}`{}",
                other.unwrap_or("anything"),
                if combat { " in combat" } else { "" }
            )),
        }
    }

    /// `T:Mode$ TapsForMana` — "whenever a Mountain is tapped for mana"
    /// (Gauntlet of Might), "whenever a player taps a land for mana"
    /// (Manabarbs): CR 106.12a. No `Activator$` is anybody, as the two
    /// sentences say. `Static$ True` is the reference's mark on the ones that
    /// resolve at once, which the engine derives from the ability's shape
    /// instead (CR 605.1b) and so needs no word for.
    fn taps_for_mana_trigger(&mut self, p: &mut Params) -> Option<String> {
        let Some(valid) = p.take("ValidCard") else {
            return self.deny("a `TapsForMana` trigger with no `ValidCard$`".to_string());
        };
        let filter = if valid == "Card.Self" {
            "&Filter::This".to_string()
        } else {
            let expr = self.filter_expr(&valid)?;
            format!("&{}", self.body.filter_static("TRIGGER", &expr))
        };
        let by = match p.take("Activator").as_deref() {
            None => "PlayerRel::EachPlayer",
            Some("You") => "PlayerRel::You",
            Some("Opponent") => "PlayerRel::EachOpponent",
            Some(other) => {
                return self.deny(format!("a permanent tapped for mana by `{other}`"));
            }
        };
        match p.take("Static").as_deref() {
            None | Some("True") => {}
            Some(other) => return self.deny(format!("`Static$ {other}`")),
        }
        Some(format!(
            "Trigger::TappedForMana {{ by: {by}, filter: {filter} }}"
        ))
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
            "AttackerBlockedByCreature" => self.block_trigger(p),
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
                // No player named is every player's step: "at the beginning
                // of each upkeep" (Verdant Force), "at the beginning of the
                // end step" (Pestilence). The reference writes `ValidPlayer$
                // You` for "your", and reading its absence as "your" too made
                // Verdant Force a card that made a Saproling on one upkeep in
                // two.
                let valid = p.take("ValidPlayer");
                let whose = match valid.as_deref() {
                    None => Some("PlayerRel::EachPlayer"),
                    Some(who) => self.player_rel(Some(who)),
                };
                let Some(whose) = whose else {
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
            "TapsForMana" => self.taps_for_mana_trigger(p),
            "DamageDone" => self.damage_trigger(p, false),
            "DamageDoneOnce" => self.damage_trigger(p, true),
            // "Whenever [a permanent] becomes tapped": the source (City of
            // Brass), or any permanent the filter matches (Lifetap).
            "Taps" => {
                let Some(valid) = p.take("ValidCard") else {
                    return self.deny("a `Taps` trigger with no `ValidCard$`".to_string());
                };
                if valid == "Card.Self" {
                    return Some("Trigger::BecomesTapped(&Filter::This)".to_string());
                }
                let expr = self.filter_expr(&valid)?;
                let filter = self.body.filter_static("TRIGGER", &expr);
                Some(format!("Trigger::BecomesTapped(&{filter})"))
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
        if kind == "Copy" {
            let svar = fields.next().map(str::trim).unwrap_or_default().to_string();
            let optional = fields.next().map(str::trim) == Some("Optional");
            return self.copy_on_enter(&svar, optional);
        }
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
        if api == "ChooseType" {
            return self.choose_type_on_enter(p);
        }
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

    /// `DB$ ChooseType | Type$ Basic Land` behind an `ETBReplacement:Other`:
    /// "as this enters, choose a basic land type" (Phantasmal Terrain),
    /// `EnterModifier::ChooseBasicLandType`. The choice is its controller's,
    /// so a `Defined$` other than `You` is refused, and so is every other
    /// `Type$`: a creature type is a different question with other readers.
    fn choose_type_on_enter(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        match p.take("Defined").as_deref() {
            None | Some("You") => {}
            Some(other) => return self.deny(format!("a type chosen by `{other}`")),
        }
        match p.take("Type").as_deref() {
            Some("Basic Land") => {}
            other => {
                return self.deny(format!(
                    "as-enters choice of a `{}` type",
                    other.unwrap_or("nameless")
                ));
            }
        }
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `ChooseType.{key}`"));
        }
        self.body
            .enter_modifiers
            .push("EnterModifier::ChooseBasicLandType".to_string());
        Some(())
    }

    /// `K:ETBReplacement:Copy:<svar>:Optional` — "you may have this enter
    /// as a copy of any creature on the battlefield" (Clone), as
    /// `AbilityDef::CopyOnEnter`.
    ///
    /// The choice is made before the permanent enters (CR 614.12a), so
    /// the reference's `Other` on the choices names nothing the choice
    /// could include, and is dropped. `AddTypes$` is the one exception the
    /// copy may carry here ("except it's an enchantment", Copy Artifact).
    /// A clone that *must* copy, one that sets its colour or grants itself
    /// a trigger (Vesuvan Doppelganger), is another sentence and refused:
    /// `CopyOnEnter` asks with an empty answer allowed, which is the "may".
    fn copy_on_enter(&mut self, svar: &str, optional: bool) -> Option<()> {
        if !optional {
            return self.deny("a clone that must copy".to_string());
        }
        let Some(body) = self.svars.get(svar).cloned() else {
            return self.deny(format!("an `ETBReplacement` naming the missing `{svar}`"));
        };
        let Some((api, mut p)) = Params::parse(&body) else {
            return self.deny("an `ETBReplacement` ability with no `$` in it".to_string());
        };
        if api != "Clone" {
            return self.deny(format!("as-enters copy `{api}`"));
        }
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("a clone with no `Choices$`".to_string());
        };
        let (base, atoms) = choices.split_once('.').unwrap_or((&choices, ""));
        let kept: Vec<&str> = atoms
            .split('+')
            .filter(|atom| !atom.is_empty() && *atom != "Other")
            .collect();
        let valid = if kept.is_empty() {
            base.to_string()
        } else {
            format!("{base}.{}", kept.join("+"))
        };
        let expr = self.filter_expr(&valid)?;
        let filter = self.body.filter_static("CHOICE", &expr);
        let mut mods = Vec::new();
        if let Some(types) = p.take("AddTypes") {
            for word in types.split(',').map(str::trim) {
                let Some(types) = card_type_const(word) else {
                    return self.deny(format!("a clone that adds `{word}`"));
                };
                mods.push(format!("CopyMod::AddType({types})"));
            }
        }
        if !p.exhausted() {
            let key = p.first_key().unwrap_or_default();
            return self.deny(format!("unclaimed parameter `Clone.{key}`"));
        }
        self.body.abilities.push(format!(
            "AbilityDef::CopyOnEnter {{ target: TargetSpec::Object(&{filter}), mods: &[{}] }}",
            mods.join(", ")
        ));
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
        if let Some(from) = self.protection_filter(line) {
            self.body.abilities.push(Self::static_expr(
                "Filter::This",
                &format!("Modifier::ProtectionFrom(&{from})"),
            ));
            return Some(());
        }
        let head = line.split(':').next().unwrap_or(line);
        let head = head.split(' ').next().unwrap_or(head);
        self.deny(format!("keyword `{head}`"))
    }

    /// What a protection keyword protects from, as the `Filter` asked of a
    /// source (CR 702.16a: "protection from [quality]").
    ///
    /// Two spellings. `Protection from black` names a colour, the commonest
    /// shape by far (153 of the corpus's keyword lines). The other is
    /// `Protection:<valid>[:<prose>[:<exception>]]`, where the valid-string
    /// is the quality ("Artifact", "Creature") and is read by
    /// [`Self::filter_expr`] like any other. The one exception understood is
    /// the host card itself, which is how the Alpha Wards print "This effect
    /// doesn't remove this Aura": protection from white, except from the
    /// white Aura that grants it, so the Aura stays attached (CR 702.16c
    /// would otherwise put it into the graveyard). `Filter::This` names that
    /// Aura, the static's source. Any other exception is refused.
    fn protection_filter(&self, word: &str) -> Option<String> {
        if let Some(color) = word.trim().strip_prefix("Protection from ") {
            let color = match color {
                "white" => "White",
                "blue" => "Blue",
                "black" => "Black",
                "red" => "Red",
                "green" => "Green",
                _ => return None,
            };
            return Some(format!(
                "Filter::HasColor(ColorSet::from_slice(&[Color::{color}]))"
            ));
        }
        let rest = word.trim().strip_prefix("Protection:")?;
        let mut fields = rest.split(':');
        let quality = self.filter_expr(fields.next()?)?;
        let _prose = fields.next();
        let filter = match fields.next() {
            None => quality,
            Some("Card.CardUID_HostCardUID") => {
                format!("Filter::And(&[{quality}, Filter::Not(&Filter::This)])")
            }
            Some(other) => {
                return self.deny(format!("a protection that excepts `{other}`"));
            }
        };
        if fields.next().is_some() {
            return self.deny("a protection keyword with a fifth field".to_string());
        }
        Some(filter)
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
        self.on_a_spell = kind == 'A' && spec.trim_start().starts_with("SP$");
        self.trigger_mode = None;
        self.block_line = None;
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
        if event == "BeginPhase" {
            return self.skip_untap_steps(&mut p);
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

    /// `R:Event$ BeginPhase | Phase$ Untap | Skip$ True` — "players skip
    /// their untap steps" (Stasis) — as `Modifier::SkipUntapStep` for every
    /// player.
    ///
    /// A static ability and not a replacement here, for the reason
    /// [`Tx::does_not_untap`] gives: the skip is read by the untap step
    /// itself (CR 614.10), with nothing to put in the step's place. Only the
    /// untap step, only from the battlefield, and only with no player named:
    /// the corpus writes this line three times, and the third is a plane's,
    /// from the command zone, which refuses.
    fn skip_untap_steps(&mut self, p: &mut Params) -> Option<()> {
        p.drop_prose();
        match p.take("ActiveZones").as_deref() {
            None | Some("Battlefield") => {}
            Some(zone) => {
                return self.deny(format!("a skipped step from `ActiveZones$ {zone}`"));
            }
        }
        match (p.take("Phase").as_deref(), p.take("Skip").as_deref()) {
            (Some("Untap"), Some("True")) => {}
            (phase, skip) => {
                return self.deny(format!(
                    "`BeginPhase` of `{}` with `Skip$ {}`",
                    phase.unwrap_or("any step"),
                    skip.unwrap_or("nothing")
                ));
            }
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `BeginPhase.{key}`"));
        }
        self.body.abilities.push(
            "static_ability!(Filter::Any, Modifier::SkipUntapStep { who: PlayerRel::EachPlayer })"
                .to_string(),
        );
        Some(())
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
        if mode == "CantBlockBy" {
            return self.cant_block_by(p);
        }
        if let Some(modifier) = match mode.as_str() {
            "CanAttackDefender" => Some("AttacksDespiteDefender"),
            "CanAttackIfHaste" => Some("AttacksAsThoughHaste"),
            _ => None,
        } {
            return self.attack_as_though(modifier, p);
        }
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
        // "This creature's power and toughness are each equal to the number
        // of Swamps you control" (Nightmare): a characteristic-defining
        // ability (CR 604.3), about the card itself and so with no
        // `Affected$`.
        if p.peek("Affected").is_none()
            && p.take("CharacteristicDefining").as_deref() == Some("True")
        {
            return self.characteristic_pt(p);
        }
        let Some(affected) = p.take("Affected") else {
            return self.deny("a continuous static with no `Affected$`".to_string());
        };
        if p.peek("AddKeyword")
            .is_some_and(|k| k.starts_with("UntapAdjust:"))
        {
            return self.untap_limit(&affected, p);
        }
        let filter = self.filter_expr(&affected)?;
        // "Gets +1/+1 as long as you control a Swamp" (Sedge Troll): the
        // clause the `A:` line reads as a restriction is, on a static, the
        // condition under which the ability exists at all, and the engine
        // registers and removes it as the condition changes.
        let condition = self.condition(&mut p)?;
        let mut out = Vec::new();
        self.pt_modifiers(&mut p, &filter, &mut out)?;
        self.keyword_modifiers(&mut p, &filter, &mut out)?;
        self.type_modifiers(&mut p, &filter, &mut out)?;
        self.color_modifiers(&mut p, &filter, &mut out)?;
        self.grant_modifiers(&mut p, &filter, &mut out)?;
        // "You control enchanted creature" (Control Magic): layer 2
        // (CR 613.1b), and the static's controller is who gains control —
        // `You` is the only player the modifier can name.
        match p.take("GainControl").as_deref() {
            None => {}
            Some("You") => out.push(Self::static_expr(&filter, "Modifier::GainControl")),
            Some(other) => {
                self.note(format!("control of a static given to `{other}`"));
                return None;
            }
        }
        if !condition.is_empty() {
            for ability in &mut out {
                if let Some(open) = ability.strip_suffix(')') {
                    *ability = format!("{open}{condition})");
                }
            }
        }
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

    /// "Players can't untap more than one creature during their untap
    /// steps" (Smoke), and Winter Orb's "…one land…" while it is untapped:
    /// `Affected$ <player> | AddKeyword$ UntapAdjust:<valid>:<n>` as
    /// `Modifier::UntapAtMost`. The keyword is all the line may grant, the
    /// player one this reader can name, and the only other clause read is
    /// the `IsPresent$` condition every continuous static may carry.
    fn untap_limit(&mut self, affected: &str, mut p: Params) -> Option<()> {
        let who = match affected {
            "Player" => "PlayerRel::EachPlayer",
            "You" => "PlayerRel::You",
            "Opponent" | "Player.Opponent" => "PlayerRel::EachOpponent",
            other => return self.deny(format!("an untap limit on `Affected$ {other}`")),
        };
        let keyword = p.take("AddKeyword")?;
        let mut parts = keyword.split(':');
        let (Some("UntapAdjust"), Some(valid), Some(count), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return self.deny(format!("keyword `{keyword}` beside an untap limit"));
        };
        let Ok(count) = count.trim().parse::<u8>() else {
            return self.deny(format!("an untap limit of `{count}`"));
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("UNTAPPING", &expr);
        self.body.abilities.push(format!(
            "static_ability!(Filter::Any, Modifier::UntapAtMost {{ who: {who}, of: &{name}, \
             count: {count} }}{condition})"
        ));
        Some(())
    }

    /// A characteristic-defining power and toughness (CR 604.3): both equal
    /// to one count of permanents, the ones you control
    /// (`PtCount::YouControl`) or all of them (`PtCount::OnBattlefield`).
    /// Layer 7a on the battlefield, and the card's own number everywhere
    /// else (CR 604.3), both of which `Modifier::CharacteristicPT` is. A
    /// count of somebody else's permanents, a toughness apart from the power
    /// or a clause beside it (Gaea's Liege's "as long as it isn't
    /// attacking") is another sentence, and refused.
    fn characteristic_pt(&mut self, mut p: Params) -> Option<()> {
        let (Some(power), Some(toughness)) = (p.take("SetPower"), p.take("SetToughness")) else {
            return self.deny("a characteristic-defining ability with no P/T".to_string());
        };
        if power != toughness {
            return self.deny(format!(
                "a defined toughness `{toughness}` apart from power"
            ));
        }
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `Continuous.{key}`"));
        }
        let Some(count) = self.svars.get(&power).cloned() else {
            return self.deny(format!("`{power}` names no SVar"));
        };
        let Some(valid) = count.strip_prefix("Count$Valid ") else {
            return self.deny(format!("count `{count}`"));
        };
        let yours = valid.split(['.', '+']).any(|atom| atom == "YouCtrl");
        if !yours && (valid.contains("Ctrl") || valid.contains("Own")) {
            return self.deny(format!("count `{count}`"));
        }
        let filter = self.filter_expr(valid)?;
        let name = self.body.filter_static("COUNTED", &filter);
        let count = if yours { "YouControl" } else { "OnBattlefield" };
        self.body.abilities.push(format!(
            "static_ability!(Filter::This, Modifier::CharacteristicPT {{ \
             count: PtCount::{count}(&{name}), toughness_plus: 0 }})"
        ));
        Some(())
    }

    /// "Enchanted Wall can attack as though it didn't have defender"
    /// (Animate Wall) and "enchanted creature can attack as though it had
    /// haste" (Instill Energy): a permission on the creatures `ValidCard$`
    /// names, as `Modifier::AttacksDespiteDefender` or
    /// `Modifier::AttacksAsThoughHaste`. Only `ValidCard$` and the
    /// `IsPresent$` condition are read; a line that names what may be
    /// attacked (`ValidTarget$`) is another sentence and refuses.
    fn attack_as_though(&mut self, modifier: &str, mut p: Params) -> Option<()> {
        p.drop_prose();
        let Some(valid) = p.take("ValidCard") else {
            return self.deny(format!("`{modifier}` naming no creature"));
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            return self.deny(format!("unclaimed parameter `{modifier}.{key}`"));
        }
        let filter = self.filter_expr(&valid)?;
        self.body.abilities.push(format!(
            "static_ability!({filter}, Modifier::{modifier}{condition})"
        ));
        Some(())
    }

    /// "Can't be blocked by Walls" (Juggernaut), "can't be blocked except
    /// by Walls" (Invisibility), "can't block creatures with power 2 or
    /// greater" (Ironclaw Orcs): one restriction on a pairing (CR 509.1b),
    /// which `Modifier::CantBeBlockedBy` states from the attacker's side.
    ///
    /// `combat::can_block` reads the blocker's filter against the static's
    /// own source, so `Self` on that side is the card that states it, and
    /// a blocker-side restriction is the same modifier on every attacker
    /// the other filter names.
    fn cant_block_by(&mut self, mut p: Params) -> Option<()> {
        p.drop_prose();
        let (Some(attacker), Some(blocker)) = (p.take("ValidAttacker"), p.take("ValidBlocker"))
        else {
            return self.deny("`CantBlockBy` naming no attacker or no blocker".to_string());
        };
        let condition = self.condition(&mut p)?;
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `CantBlockBy.{key}`"));
            return None;
        }
        let attackers = self.filter_expr(&attacker)?;
        let blockers = self.filter_expr(&blocker)?;
        let name = self.body.filter_static("BLOCKER", &blockers);
        self.body.abilities.push(format!(
            "static_ability!({attackers}, Modifier::CantBeBlockedBy(&{name}){condition})"
        ));
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
                // Protection is granted as the static ability it is (CR
                // 702.16a), never as a bit; removing it is a different
                // sentence and is refused.
                if modifier == "AddKeyword"
                    && let Some(from) = self.protection_filter(word)
                {
                    out.push(Self::static_expr(
                        filter,
                        &format!("Modifier::ProtectionFrom(&{from})"),
                    ));
                    continue;
                }
                let Some(bit) = keyword_const(word) else {
                    self.note(format!("static ability granting keyword `{word}`"));
                    return None;
                };
                bits.push(bit.to_string());
            }
            let Some(set) = bits.split_first().map(|(head, tail)| {
                tail.iter()
                    .fold(head.clone(), |acc, b| format!("{acc}.union({b})"))
            }) else {
                continue;
            };
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
    ///
    /// `RemoveLandTypes$ True` beside an `AddType$` of one basic land type
    /// is "enchanted land is a Swamp" (Evil Presence, Conversion): CR 305.7's
    /// setting of a land's subtype, `Modifier::SetLandType`, which also takes
    /// the abilities the land's rules text gives it. `AddType$ ChosenType` is
    /// "enchanted land is the chosen type" (Phantasmal Terrain),
    /// `Modifier::SetLandTypeToChosen`, read only on a card that asks for a
    /// basic land type as it enters. Any other `AddType$` beside it — two
    /// types, a nonbasic one, a type chosen some other way — is refused.
    fn type_modifiers(&self, p: &mut Params, filter: &str, out: &mut Vec<String>) -> Option<()> {
        match p.take("RemoveLandTypes").as_deref() {
            None => {}
            Some("True") => {
                let raw = p.take("AddType")?;
                let modifier = if raw.trim() == "ChosenType" {
                    self.body
                        .enter_modifiers
                        .iter()
                        .any(|m| m == "EnterModifier::ChooseBasicLandType")
                        .then(|| "Modifier::SetLandTypeToChosen".to_string())?
                } else {
                    let path = self.basic_land_type(raw.trim())?;
                    format!("Modifier::SetLandType({path})")
                };
                out.push(Self::static_expr(filter, &modifier));
            }
            Some(_) => return None,
        }
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

    /// The constant of one basic land type (CR 205.3i names the five), or
    /// `None` for any other word.
    fn basic_land_type(&self, word: &str) -> Option<String> {
        if !matches!(word, "Plains" | "Island" | "Swamp" | "Mountain" | "Forest") {
            return None;
        }
        self.cats
            .const_path_of(baylee_core::types::SubtypeKind::Land, word)
    }

    /// `AddColor`/`SetColor` (layer 5) as static abilities.
    /// `AddAbility$ <SVar>[ & <SVar>]`: "Other Zombies have '{B}:
    /// Regenerate this permanent.'" (Zombie Master). Each named `SVar` is an
    /// `AB$` line, read as an activated ability of the object that gains it:
    /// `Modifier::GrantActivated` activates from that object, so "this
    /// permanent" in it is the one that has it.
    ///
    /// Without a target, because the modifier carries none — a granted
    /// ability that targets would lose its target here and act on nothing
    /// — and without anything else an activated line may say beside its
    /// cost and effect: the chain refuses a key it does not claim.
    fn grant_modifiers(
        &mut self,
        p: &mut Params,
        filter: &str,
        out: &mut Vec<String>,
    ) -> Option<()> {
        let Some(raw) = p.take("AddAbility") else {
            return Some(());
        };
        for name in raw.split(" & ").map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("`AddAbility$ {name}` naming no `SVar`"));
            };
            if !line.trim_start().starts_with("AB$") {
                return self.deny("a granted ability that is not an activated one".to_string());
            }
            let Some((_, mut probe)) = Params::parse(&line) else {
                return self.deny("a granted ability with no `$` in it".to_string());
            };
            let Some(cost) = probe.take("Cost") else {
                return self.deny("a granted ability with no `Cost$`".to_string());
            };
            let cost = self.cost_expr(&cost)?;
            let stripped: Vec<&str> = line
                .split(" | ")
                .filter(|part| !part.starts_with("Cost$"))
                .collect();
            let mut chain = Chain::default();
            self.chain(&stripped.join(" | "), &mut chain)?;
            if chain.target.is_some() {
                return self.deny("a granted ability with a target".to_string());
            }
            if chain.effects.is_empty() {
                return self.deny("a granted ability that reads as no effect at all".to_string());
            }
            let mana = chain.effects.iter().any(|e| e.contains("Effect::mana"));
            out.push(Self::static_expr(
                filter,
                &format!(
                    "Modifier::GrantActivated {{ cost: {cost}, effects: &[{}], mana_ability: {mana} }}",
                    chain.effects.join(", ")
                ),
            ));
        }
        Some(())
    }

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

    /// When an ability may be activated, beyond its `IsPresent$` clause: a
    /// restriction on activating (CR 602.5) that the `condition` field says,
    /// and so only where no other clause already fills it. Hands back the
    /// keys it claimed.
    ///
    /// - `PlayerTurn$ True`: "activate only during your turn" (Disrupting
    ///   Scepter).
    /// - `ActivationPhases$ BeginCombat->EndCombat`: "activate only during
    ///   combat" (Jade Statue). The other spellings — an upkeep, a single
    ///   combat step, "before blockers are declared" — are other sentences.
    fn activation_restriction(
        &mut self,
        probe: &mut Params,
        is_activated: bool,
        condition: &mut String,
    ) -> Option<Vec<&'static str>> {
        let mut claimed = Vec::new();
        if let Some(your_turn) = probe.take("PlayerTurn") {
            if your_turn != "True" || !is_activated || !condition.is_empty() {
                return self.deny(format!("`PlayerTurn$ {your_turn}` beside another clause"));
            }
            *condition = ", condition = Some(Condition::YourTurn)".to_string();
            claimed.push("PlayerTurn$");
        }
        if let Some(phases) = probe.take("ActivationPhases") {
            if phases != "BeginCombat->EndCombat" || !is_activated || !condition.is_empty() {
                return self.deny(format!("`ActivationPhases$ {phases}`"));
            }
            *condition = ", condition = Some(Condition::DuringCombat)".to_string();
            claimed.push("ActivationPhases$");
        }
        Some(claimed)
    }

    fn activated_or_spell(&mut self, spec: &str) -> Option<()> {
        let is_activated = spec.starts_with("AB$");
        if !is_activated && Params::parse(spec).is_some_and(|(api, _)| api == "Charm") {
            return self.charm(spec);
        }
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
        let mut condition = self.condition(&mut probe)?;
        // The `IsPresent$` family is claimed only where that clause was read,
        // not where a restriction below filled `condition` instead.
        let present_read = !condition.is_empty();
        let restrictions = self.activation_restriction(&mut probe, is_activated, &mut condition)?;
        let mut chain = Chain::default();
        // The cost belongs to the ability, not to the effect chain, so it is
        // removed from the spec before the chain reads it. The clause's keys
        // go with it, but **only** once the clause was read: a line carrying
        // `PresentZone$` and no `IsPresent$` at all keeps it, and refuses one
        // level down as the unclaimed parameter it is.
        let claimed: &[&str] = if present_read {
            &[
                "IsPresent$",
                "PresentZone$",
                "PresentCompare$",
                "PresentDefined$",
            ]
        } else {
            &[]
        };
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$") && !part.starts_with("ActivationLimit$"))
            .filter(|part| !restrictions.iter().any(|key| part.starts_with(key)))
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

    /// "Choose one —" (CR 700.2): `SP$ Charm | Choices$ A,B` is a modal
    /// spell whose modes are the named `SVar`s, each read as the chain it
    /// is and each targeting for itself, since a target a mode names is
    /// chosen only when that mode is (CR 700.2c).
    ///
    /// Only a spell, and only "choose one" or "choose two": a modal
    /// activated ability is a different `AbilityDef`, and the other counts
    /// the reference spells (`MinCharmNum$`, a count the board works out)
    /// are rules this reader has not met yet.
    fn charm(&mut self, spec: &str) -> Option<()> {
        let Some((_, mut p)) = Params::parse(spec) else {
            return self.deny("an `A:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        let Some(choices) = p.take("Choices") else {
            return self.deny("a charm with no `Choices$`".to_string());
        };
        let choose = match p.take("CharmNum").as_deref() {
            None | Some("1") => "ModeCount::ONE",
            Some("2") => "ModeCount::TWO",
            Some(n) => return self.deny(format!("a charm choosing `{n}`")),
        };
        if let Some(key) = p.first_key() {
            self.note(format!("unclaimed parameter `Charm.{key}`"));
            return None;
        }
        let mut modes = Vec::new();
        for name in choices.split(',').map(str::trim) {
            let Some(line) = self.svars.get(name).cloned() else {
                return self.deny(format!("charm choice `{name}` with no `SVar`"));
            };
            let mut chain = Chain::default();
            self.chain(&line, &mut chain)?;
            if chain.effects.is_empty() {
                return self.deny("a charm mode that reads as no effect at all".to_string());
            }
            let targets = chain
                .target
                .map(|t| format!(", targets = Some(TargetReq::one({t}))"))
                .unwrap_or_default();
            modes.push(format!("mode!(&[{}]{targets})", chain.effects.join(", ")));
        }
        if modes.len() < 2 {
            return self.deny("a charm with fewer than two modes".to_string());
        }
        self.body.abilities.push(format!(
            "AbilityDef::ModalSpell {{ choose: {choose}, modes: &[{}] }}",
            modes.join(", ")
        ));
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
            let Some(bound) = count_bound(&compare) else {
                return self.deny(format!("`PresentCompare$ {compare}`"));
            };
            // Whose permanents: yours (`YouCtrl` in every alternative), or
            // everybody's when no alternative names a controller or an
            // owner at all ("if no creatures are on the battlefield").
            let atoms = || valid.split(',').flat_map(|alt| alt.split(['.', '+']));
            let yours = valid
                .split(',')
                .all(|alt| alt.split(['.', '+']).any(|atom| atom.trim() == "YouCtrl"));
            let nobodys = !atoms().any(|atom| atom.contains("Ctrl") || atom.contains("Own"));
            if !yours && !nobodys {
                return self.deny(format!("`IsPresent$ {valid}`, a count of somebody else's"));
            }
            // And a count says nothing about the card that states it.
            // `eval::condition_holds` walks a battlefield handing each
            // candidate its *own* id as the object a filter's `This` and
            // `Another` compare against, so "another creature you control"
            // would count nothing at all — 28 corpus lines write one, and
            // a trigger that can never fire is exactly the wrong card the
            // honest-stub rule exists to refuse.
            // The attachment atoms are relative too: `AttachedToBySource`
            // asks what *this* card is attached to.
            if valid.split(',').any(|alt| {
                alt.split(['.', '+']).any(|a| {
                    matches!(
                        a.trim(),
                        "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
                    )
                })
            }) {
                return self.deny(format!(
                    "`IsPresent$ {valid}`, a count relative to this card"
                ));
            }
            Some((yours, bound))
        };
        let expr = self.filter_expr(&valid)?;
        let clause = match count {
            None => {
                let zoned = on_the_battlefield(&expr);
                let name = self.body.filter_static("CHECK", &zoned);
                format!("Condition::SourceMatches(&{name})")
            }
            // `ControlCount` counts one player's battlefield and nothing
            // else, and `BattlefieldCount` all of it, so the zone and the
            // player are both already in the sentence each is.
            Some((yours, bound)) => {
                let name = self.body.filter_static("CHECK", &expr);
                let (variant, n) = match (yours, bound) {
                    (true, Bound::AtLeast(n)) => ("ControlCount", n),
                    (true, Bound::AtMost(n)) => ("ControlCountAtMost", n),
                    (false, Bound::AtLeast(n)) => ("BattlefieldCount", n),
                    (false, Bound::AtMost(n)) => ("BattlefieldCountAtMost", n),
                };
                format!("Condition::{variant}(&{name}, {n})")
            }
        };
        Some(format!(", condition = Some({clause})"))
    }

    fn triggered(&mut self, spec: &str) -> Option<()> {
        let Some((mode, mut p)) = Params::parse(spec) else {
            return self.deny("a `T:` line with no `$` in it".to_string());
        };
        p.drop_prose();
        self.trigger_mode = Some(mode.clone());
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
        // "You may pay {1}. If you do, you gain 1 life" (Crystal Rod): the
        // reference writes the payment as a `Cost$` on the executed line and
        // the "may" as `OptionalDecider$ You`. The two are one decision —
        // the player who will not pay has declined — so the price replaces
        // the `MayDo` rather than sitting inside it, which would ask twice.
        // Generic mana only: a coloured price or a non-mana cost is another
        // payment the effect cannot take, and stays unclaimed.
        let (body, price) = match Self::optional_price(&body, may) {
            Some((stripped, price)) => (stripped, Some(price)),
            None => (body, None),
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
        let effects = match price {
            Some(Price::Generic(mana)) => format!(
                "Effect::PlayerMayPayThen {{ player: PlayerRel::You, \
                 mana: {mana}, effects: &[{effects}] }}"
            ),
            // Farmstead's "you may pay {W}{W}. If you do, you gain 1 life".
            Some(Price::Printed(cost)) => format!(
                "Effect::PlayerMayPayManaThen {{ player: PlayerRel::You, \
                 cost: mana!(\"{cost}\"), effects: &[{effects}] }}"
            ),
            Some(Price::Part(_)) => unreachable!("`optional_price` reads mana only"),
            None if may => format!("Effect::MayDo {{ effects: &[{effects}] }}"),
            None => effects,
        };
        let ability = format!("triggered!({trigger}, &[{effects}]{targets}{condition})");
        if let Some((role, other, secondary)) = self.block_line.take() {
            return self.pair_block_half(role, other, secondary, ability);
        }
        self.body.abilities.push(ability);
        Some(())
    }

    /// `T:Mode$ AttackerBlockedByCreature` — "whenever this creature blocks
    /// or becomes blocked by a non-Wall creature" (Cockatrice), as
    /// `Trigger::BlocksOrBecomesBlockedBy`.
    ///
    /// The reference writes the one printed ability as **two** lines, one
    /// for each side of the block: `ValidCard$ <other> | ValidBlocker$
    /// Card.Self` for "blocks" and `ValidCard$ Card.Self | ValidBlocker$
    /// <other>` for "becomes blocked by", the second marked `Secondary$
    /// True`. Each line is read here; [`Tx::pair_block_half`] keeps the
    /// first, drops its mirror, and a script that ends with a half unpaired
    /// is refused, because either half alone is another sentence ("whenever
    /// this creature blocks a creature", CR 509.3b, or "becomes blocked by
    /// a creature", 509.3d) that the trigger here would over-read.
    ///
    /// A source on neither side is an Aura's or an Equipment's "enchanted
    /// creature blocks", and an other side relative to another permanent
    /// (`AttachedBy`, `EnchantedBy`) is a sentence about it; both refuse.
    fn block_trigger(&mut self, p: &mut Params) -> Option<String> {
        let card = p.take("ValidCard");
        let blocker = p.take("ValidBlocker");
        let secondary = match p.take("Secondary").as_deref() {
            None => false,
            Some("True") => true,
            Some(other) => return self.deny(format!("`Secondary$ {other}`")),
        };
        let (role, other) = match (card, blocker) {
            (Some(card), Some(blocker)) if blocker == "Card.Self" && card != "Card.Self" => {
                (BlockRole::Blocks, card)
            }
            (Some(card), Some(blocker)) if card == "Card.Self" && blocker != "Card.Self" => {
                (BlockRole::Blocked, blocker)
            }
            (card, blocker) => {
                return self.deny(format!(
                    "a block between `{}` and `{}`",
                    card.unwrap_or_default(),
                    blocker.unwrap_or_default()
                ));
            }
        };
        if other.split([',', '.', '+']).any(|atom| {
            matches!(
                atom.trim(),
                "Self" | "Other" | "EnchantedBy" | "EquippedBy" | "AttachedBy"
            )
        }) {
            return self.deny(format!("a block with `{other}`, relative to another card"));
        }
        let expr = self.filter_expr(&other)?;
        let filter = self.body.filter_static("TRIGGER", &expr);
        self.block_line = Some((role, other, secondary));
        Some(format!("Trigger::BlocksOrBecomesBlockedBy(&{filter})"))
    }

    /// The pairing half of [`Tx::block_trigger`]: the first half read is
    /// written and kept; the second must be its mirror — the other side of
    /// the block, the same other creature, `Secondary$` on exactly one of
    /// the two, and the same ability once read — and writes nothing, since
    /// the card prints one ability.
    fn pair_block_half(
        &mut self,
        role: BlockRole,
        other: String,
        secondary: bool,
        ability: String,
    ) -> Option<()> {
        match self.block_half.take() {
            None => {
                self.body.abilities.push(ability.clone());
                self.block_half = Some(BlockHalf {
                    role,
                    other,
                    secondary,
                    ability,
                });
                Some(())
            }
            Some(first)
                if first.role != role
                    && first.other == other
                    && first.secondary != secondary
                    && first.ability == ability =>
            {
                Some(())
            }
            Some(_) => {
                self.deny("two block triggers that are not the halves of one sentence".to_string())
            }
        }
    }

    /// Refuses a script that ended with one half of a "blocks or becomes
    /// blocked by" trigger read and its mirror never met.
    fn block_halves_paired(&self) -> Option<()> {
        match &self.block_half {
            None => Some(()),
            Some(half) => self.deny(format!(
                "a `{}` trigger without its mirror",
                match half.role {
                    BlockRole::Blocks => "blocks",
                    BlockRole::Blocked => "becomes blocked by",
                }
            )),
        }
    }

    /// `DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat` — "destroy that
    /// creature at end of combat" (Cockatrice): a delayed trigger that
    /// triggers as the end of combat step begins (CR 511.2),
    /// `Effect::AtEndOfCombat`.
    ///
    /// Only under a block trigger ([`Tx::block_trigger`]) and only
    /// remembering the **other** creature of the block — the attacker on the
    /// "blocks" half, the blocker on the "becomes blocked by" one — which is
    /// the trigger's event object. Its `Execute$` is read as a chain of its
    /// own in which `Defined$ DelayTriggerRememberedLKI` is that object, and
    /// which may target nothing. Every other phase, player, remembered
    /// object or delayed mode is refused by name.
    fn delayed_trigger(&mut self, p: &mut Params, target: Option<&str>) -> Option<String> {
        let mode = p.take("Mode").unwrap_or_default();
        let phase = p.take("Phase").unwrap_or_default();
        if mode != "Phase" || phase != "EndCombat" {
            return self.deny(format!("a delayed trigger at `{mode} {phase}`"));
        }
        match p.take("ValidPlayer").as_deref() {
            None | Some("Player") => {}
            Some(other) => {
                return self.deny(format!("a delayed trigger in `{other}`'s turn"));
            }
        }
        let remembered = p.take("RememberObjects").unwrap_or_default();
        let other_side = match self.block_line.as_ref().map(|(role, ..)| *role) {
            Some(BlockRole::Blocks) => "TriggeredAttacker",
            Some(BlockRole::Blocked) => "TriggeredBlocker",
            None => {
                return self.deny(format!(
                    "a delayed trigger remembering `{remembered}` outside a block trigger"
                ));
            }
        };
        if remembered.strip_suffix("LKICopy").unwrap_or(&remembered) != other_side {
            return self.deny(format!("a delayed trigger remembering `{remembered}`"));
        }
        if target.is_some() {
            return self.deny("a delayed trigger on a line that targets".to_string());
        }
        let Some(execute) = p.take("Execute") else {
            return self.deny("a delayed trigger with no `Execute$`".to_string());
        };
        let Some(body) = self.svars.get(&execute).cloned() else {
            return self.deny(format!("`Execute$ {execute}` names no SVar"));
        };
        let mut inner = Chain::default();
        let outer = std::mem::replace(&mut self.in_delayed, true);
        let read = self.chain(&body, &mut inner);
        self.in_delayed = outer;
        read?;
        if inner.target.is_some() {
            return self.deny("a delayed trigger that targets".to_string());
        }
        if inner.effects.is_empty() {
            return self.deny("a delayed trigger that reads as no effect".to_string());
        }
        Some(format!(
            "Effect::AtEndOfCombat {{ about: TargetSpec::EventObject, effects: &[{}] }}",
            inner.effects.join(", ")
        ))
    }

    /// The executed line of a "you may pay {N}. If you do, …" trigger with
    /// its price taken off, and the price: `AB$ GainLife | Cost$ 1 | …`
    /// under `OptionalDecider$ You`. `None` for anything else — no "may",
    /// no `Cost$`, or a cost that is not mana alone — which leaves the line
    /// as it was for the chain to read or refuse.
    fn optional_price(body: &str, may: bool) -> Option<(String, Price)> {
        if !may || !body.trim_start().starts_with("AB$") {
            return None;
        }
        let parts: Vec<&str> = body.split(" | ").collect();
        let cost = parts
            .iter()
            .find_map(|part| part.strip_prefix("Cost$"))?
            .trim();
        let price = match cost.parse::<u32>() {
            Ok(0) => return None,
            Ok(n) => Price::Generic(format!("Amount::Fixed({n})")),
            Err(_) => Price::Printed(printed_mana(cost)?),
        };
        let rest: Vec<&str> = parts
            .into_iter()
            .filter(|part| !part.starts_with("Cost$"))
            .collect();
        Some((rest.join(" | "), price))
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
        "Shadow" => "KeywordSet::SHADOW",
        "Fear" => "KeywordSet::FEAR",
        "Landwalk:Plains" => "KeywordSet::PLAINSWALK",
        "Landwalk:Island" => "KeywordSet::ISLANDWALK",
        "Landwalk:Swamp" => "KeywordSet::SWAMPWALK",
        "Landwalk:Mountain" => "KeywordSet::MOUNTAINWALK",
        "Landwalk:Forest" => "KeywordSet::FORESTWALK",
        "Prowess" => "KeywordSet::PROWESS",
        "Changeling" => "KeywordSet::CHANGELING",
        "Banding" => "KeywordSet::BANDING",
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

/// Whether the script says nothing beyond what Scryfall supplies anyway: no
/// keyword, no ability and no line kind the parser does not model.
///
/// [`transcode`] refuses such a script, because an empty body is also what
/// a card it could not read at all would look like. A vanilla creature is
/// the other reading, and only the printing can tell the two apart, so
/// `stubgen` finishes the card when its printed text is empty as well; the
/// reports that walk scripts without a printing (`reach-list`,
/// `transcode-report`) count it as read on this alone, and `codegen` decides.
#[must_use]
pub fn is_vanilla(script: &CardScript) -> bool {
    script.keywords.is_empty() && script.rules.is_empty() && script.unknown_lines.is_empty()
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
        on_a_spell: false,
        trigger_mode: None,
        block_line: None,
        block_half: None,
        in_delayed: false,
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
    if tx.block_halves_paired().is_none() {
        return tx.unclaimed.into_inner();
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

/// The reference's name for the Clue token, which CR 701.16a makes the whole
/// definition of investigating.
///
/// Named here rather than written into [`Scriptgen::investigate_effect`]
/// because it is a fact about the *corpus* and not about the rule: the rule
/// says "a Clue token" and this is the file that happens to hold one. It
/// goes through the same `TokenLookup` every `TokenScript$` goes through, so
/// a run with no token corpus refuses investigating for the same stated
/// reason it refuses every other token, instead of emitting a constant that
/// the ledger may not have assigned.
const CLUE_TOKEN_SCRIPT: &str = "c_a_clue_draw";

/// The effect APIs [`transcode`] knows how to write.
///
/// Kept beside the match in [`Tx::chain`] so a report of what the corpus
/// still needs cannot drift from what the transcoder actually reads.
pub const SUPPORTED_APIS: &[&str] = &[
    "DealDamage",
    "GainLife",
    "LoseLife",
    "Draw",
    "Discard",
    "Mill",
    "Scry",
    "Surveil",
    "Mana",
    "Destroy",
    "DestroyAll",
    "DelayedTrigger",
    "DamageAll",
    "DamageResolve",
    "PreventDamage",
    "AddTurn",
    "Effect",
    "ChooseSource",
    "Fog",
    "Regenerate",
    "Tap",
    "TapOrUntap",
    "TapAll",
    "DrainMana",
    "Untap",
    "Counter",
    "PutCounter",
    "Sacrifice",
    "Pump",
    "ChangeZone",
    "ChangeZoneAll",
    "RearrangeTopOfLibrary",
    "Token",
    "Investigate",
];

/// A cost token that names an object, split into its kind and its body.
///
/// `Sac<1/CARDNAME…>` is deliberately not among them: it is matched one
/// branch earlier as `SacrificeSelf`, which asks nobody anything.
fn object_cost(token: &str) -> Option<(&'static str, &str)> {
    for kind in ["Sac", "Discard", "tapXType", "Return", "ExileFromGrave"] {
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
/// The five `cost_wizard` puts a list up for, spelled as the emitter writes
/// them rather than as the engine matches them, because this side has a
/// string and not a `CostPart`. `SacrificeSelf` and `ReturnSelfToHand` are
/// deliberately not among them: they name the source and ask nothing.
fn asks_for_an_object(part: &str) -> bool {
    [
        "Sacrifice(",
        "Discard(",
        "TapOther(",
        "ReturnToHand(",
        "ExileFromGraveyard(",
    ]
    .iter()
    .any(|kind| part.starts_with(kind))
}

/// `{N}` alone as its number: a price that is generic mana and nothing else.
fn generic_mana(mana: &str) -> Option<u16> {
    mana.strip_prefix('{')?.strip_suffix('}')?.parse().ok()
}

/// A cost of mana symbols only (`W W`, `1 U`) in its printed spelling
/// (`{W}{W}`, `{1}{U}`); `None` when any part is something other than mana.
fn printed_mana(cost: &str) -> Option<String> {
    let mut out = String::new();
    for part in cost_parts(cost) {
        let mana = part.chars().all(|c| c.is_ascii_digit())
            || matches!(part.as_str(), "W" | "U" | "B" | "R" | "G" | "C");
        if !mana || part.is_empty() {
            return None;
        }
        out.push('{');
        out.push_str(&part);
        out.push('}');
    }
    (!out.is_empty()).then_some(out)
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
        if !matches!(
            api.as_str(),
            "ChangesZone" | "Phase" | "Attacks" | "Taps" | "AttackerBlockedByCreature"
        ) {
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
        // The token a line names by not naming it. `Investigate` writes no
        // `TokenScript$` at all — CR 701.16a supplies which token it is —
        // so a scan for that key alone tells the ledger the card needs no
        // token, while [`Tx::investigate_effect`] is about to emit the
        // constant the ledger was never asked to assign. The emitter and
        // the ledger have to name the same tokens or one of them is wrong,
        // and today it survives only because another card happens to name
        // the Clue outright.
        //
        // Read through [`Params::parse`] and never for the word: every one
        // of these lines also *prints* it, and `SpellDescription$
        // Investigate. (Create a Clue token…)` is on 21 of the corpus's
        // 112.
        if Params::parse(body).is_some_and(|(api, _)| api == "Investigate")
            && !out.iter().any(|stem| stem == CLUE_TOKEN_SCRIPT)
        {
            out.push(CLUE_TOKEN_SCRIPT.to_string());
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
            // The Clue is here because a token's own type line is read
            // against this catalog, and investigating reaches one.
            artifact: vec!["Clue".into()],
            land: vec!["Forest".into(), "Island".into(), "Mountain".into()],
            ..SubtypeCatalogs::default()
        };
        c.normalize();
        c
    }

    fn read(text: &str) -> CardBody {
        let parsed = parse(text);
        transcode(&parsed, &cats(), None).unwrap_or_else(|| {
            panic!(
                "should be read in full, refused: {:?}",
                refusal_reason(&parsed, &cats(), None)
            )
        })
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

    #[test]
    fn random_discard_reads_x_and_refuses_a_discard_it_cannot_name() {
        let generated = read(
            "Name:Test
ManaCost:X B
Types:Sorcery
A:SP$ Discard | ValidTgts$ Player | NumCards$ X | Mode$ Random
SVar:X:Count$xPaid",
        );
        assert!(
            generated
                .abilities
                .iter()
                .any(|a| a.contains("Effect::DiscardRandom")
                    && a.contains("Amount::X")
                    && a.contains("PlayerRel::Chosen")),
            "{generated:?}"
        );
        // `Mode$ TgtChoose` is read since the discarding player's own choice
        // has a rule (`a_chosen_discard_on_your_turn_only`).
        for extra in ["", " | Mode$ Random | RevealNumber$ 2"] {
            assert!(refused(&format!(
                "Name:Test\nManaCost:B\nTypes:Sorcery\nA:SP$ Discard | ValidTgts$ Player | NumCards$ 1{extra}"
            )));
        }
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

        // A clone that must copy is not the "may" `CopyOnEnter` asks.
        assert!(refused(
            "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\nK:ETBReplacement:Copy:CC\n\
             SVar:CC:DB$ Clone | Defined$ You"
        ));
        // Clone and Copy Artifact: the choice is made before it enters, so
        // `Other` names nothing, and the except-clause is a copy mod.
        let clone = read(
            "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\n\
             K:ETBReplacement:Copy:CC:Optional\n\
             SVar:CC:DB$ Clone | Choices$ Creature.Other | SpellDescription$ x",
        );
        assert_eq!(
            clone.abilities,
            [
                "AbilityDef::CopyOnEnter { target: TargetSpec::Object(&Filter::CREATURE), mods: &[] }"
            ]
        );
        let artifact = read(
            "Name:X\nTypes:Artifact\n\
             K:ETBReplacement:Copy:CC:Optional\n\
             SVar:CC:DB$ Clone | Choices$ Artifact.Other | AddTypes$ Enchantment",
        );
        assert!(
            artifact.abilities[0].contains("mods: &[CopyMod::AddType(TypeSet::ENCHANTMENT)]"),
            "{:?}",
            artifact.abilities
        );
        // Vesuvan Doppelganger's colour and granted trigger are refused.
        assert!(refused(
            "Name:X\nTypes:Creature Shapeshifter\nPT:0/0\n\
             K:ETBReplacement:Copy:CC:Optional\n\
             SVar:CC:DB$ Clone | Choices$ Creature.Other | SetColor$ Blue"
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
            // The Clue, in the reference's own shape, because CR 701.16a
            // makes it the whole definition of investigating and the rule
            // reaches it through this same lookup.
            (
                CLUE_TOKEN_SCRIPT,
                "Name:Clue Token\nTypes:Artifact Clue\n\
                 A:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1",
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
            on_a_spell: false,
            trigger_mode: None,
            block_line: None,
            block_half: None,
            in_delayed: false,
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
    fn shadow_is_a_keyword_bit_rather_than_unblockable() {
        let body = read("Name:Shadow Test\nTypes:Creature Rogue\nPT:1/1\nK:Shadow\nOracle:Shadow");
        assert_eq!(body.keywords, ["KeywordSet::SHADOW"]);
        assert!(body.abilities.is_empty());
    }

    /// Banding is a keyword the engine reads (CR 702.22), on a body and
    /// granted by a pump (Helm of Chatzuk); "bands with other" names a
    /// quality and stays refused.
    #[test]
    fn banding_is_read_and_bands_with_other_is_not() {
        let body = read("Name:X\nTypes:Creature Human\nPT:1/1\nK:Banding\nOracle:Banding");
        assert_eq!(body.keywords, ["KeywordSet::BANDING"]);
        let body = read(
            "Name:X\nTypes:Artifact\n\
             A:AB$ Pump | Cost$ 1 T | ValidTgts$ Creature | KW$ Banding | SpellDescription$ …",
        );
        assert!(
            body.abilities
                .iter()
                .any(|a| a.contains("keywords: KeywordSet::BANDING")),
            "{:?}",
            body.abilities
        );
        let script = parse("Name:X\nTypes:Creature Human\nPT:1/1\nK:Bands with Other:Legendary");
        assert!(transcode(&script, &cats(), None).is_none());
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

    /// Crystal Rod: "Whenever a player casts a blue spell, you may pay {1}.
    /// If you do, you gain 1 life." The price is the "may", so it replaces
    /// the `MayDo` rather than sitting inside it. A coloured price is
    /// Farmstead's `{W}{W}`, printed exactly (`PlayerMayPayManaThen`); a
    /// price that is not mana is a payment neither takes, and stays refused.
    #[test]
    fn a_may_with_a_generic_price_is_a_payment_that_buys_the_clause() {
        let body = read(
            "Name:X\nManaCost:1\nTypes:Artifact\n\
             T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
             | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
             SVar:TrigGainLife:AB$ GainLife | Cost$ 1 | Defined$ You | LifeAmount$ 1",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains(
                "Effect::PlayerMayPayThen { player: PlayerRel::You, mana: Amount::Fixed(1), \
                 effects: &[Effect::gain_life(1)] }"
            ),
            "{text}"
        );
        assert!(!text.contains("MayDo"), "one question, not two: {text}");

        let script = parse(
            "Name:X\nManaCost:1\nTypes:Artifact\n\
             T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
             | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
             SVar:TrigGainLife:AB$ GainLife | Cost$ W W | Defined$ You | LifeAmount$ 1",
        );
        let text = transcode(&script, &cats(), None)
            .expect("a coloured price is read")
            .abilities
            .join("\n");
        assert!(
            text.contains(
                "Effect::PlayerMayPayManaThen { player: PlayerRel::You, cost: mana!(\"{W}{W}\"), \
                 effects: &[Effect::gain_life(1)] }"
            ),
            "{text}"
        );

        let script = parse(
            "Name:X\nManaCost:1\nTypes:Artifact\n\
             T:Mode$ SpellCast | ValidCard$ Card.Blue | TriggerZones$ Battlefield \
             | OptionalDecider$ You | Execute$ TrigGainLife | TriggerDescription$ x.\n\
             SVar:TrigGainLife:AB$ GainLife | Cost$ PayLife<1> | Defined$ You | LifeAmount$ 1",
        );
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("unclaimed parameter `GainLife.Cost`")
        );
    }

    /// "That player" is the trigger's to say: whose step began for a
    /// `Phase` trigger (Copper Tablet), the moved card's controller for a
    /// `ChangesZone` one (Dingus Egg), and the enchanted permanent's
    /// controller where the upkeep is theirs (Cursed Land). The same words
    /// on an activated ability name nothing this reader can see.
    #[test]
    fn that_player_is_the_one_the_trigger_names() {
        let tablet = read(
            "Name:X\nManaCost:2\nTypes:Artifact\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield \
             | Execute$ TrigDamage | TriggerDescription$ x.\n\
             SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1",
        );
        assert_eq!(
            tablet.abilities,
            [
                "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, \
                 &[Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::ActivePlayer) }])"
            ]
        );

        let egg = read(
            "Name:X\nManaCost:4\nTypes:Artifact\n\
             T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Land \
             | TriggerZones$ Battlefield | Execute$ TrigDamage | TriggerDescription$ x.\n\
             SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 2",
        );
        let text = egg.abilities.join("\n");
        assert!(
            text.contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
            "{text}"
        );

        let cursed = read(
            "Name:X\nManaCost:2 B B\nTypes:Enchantment Aura\nK:Enchant:Land\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player.EnchantedController \
             | TriggerZones$ Battlefield | Execute$ TrigDamage | TriggerDescription$ x.\n\
             SVar:TrigDamage:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1",
        );
        let text = cursed.abilities.join("\n");
        assert!(
            text.contains("whose: PlayerRel::ControllerOfAttached"),
            "{text}"
        );

        assert!(refused(
            "Name:X\nManaCost:2\nTypes:Artifact\n\
             A:AB$ DealDamage | Cost$ T | Defined$ TriggeredPlayer | NumDmg$ 1"
        ));
    }

    /// A card in a graveyard is its own kind of target: Regrowth and
    /// Resurrection move from `Origin$ Graveyard`, and read as a permanent
    /// on the battlefield they offered nothing to target. `YouCtrl` there is
    /// "your graveyard".
    #[test]
    fn a_graveyard_card_is_targeted_in_its_graveyard() {
        let regrowth = read(
            "Name:X\nManaCost:1 G\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Card.YouCtrl",
        );
        let text = regrowth.abilities.join("\n");
        assert!(
            text.contains(
                "Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::Any, \
                 PlayerRel::You) }"
            ),
            "{text}"
        );

        let resurrection = read(
            "Name:X\nManaCost:2 W W\nTypes:Sorcery\n\
             A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield \
             | ValidTgts$ Creature.YouCtrl",
        );
        let text = resurrection.abilities.join("\n");
        assert!(
            text.contains(
                "Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(\
                 &Filter::CREATURE, PlayerRel::You), owner_control: false, counters: None }"
            ),
            "{text}"
        );
    }

    /// Braingeyser and Stream of Life: the X the caster announced, drawn or
    /// gained by the player the spell targets. An `X` that counts
    /// something is a different number and stays refused.
    #[test]
    fn an_announced_x_is_drawn_and_gained() {
        let geyser = read(
            "Name:X\nManaCost:X U U\nTypes:Sorcery\n\
             A:SP$ Draw | NumCards$ X | ValidTgts$ Player\nSVar:X:Count$xPaid",
        );
        assert_eq!(
            geyser.abilities,
            [
                "spell!(&[Effect::DrawCardsFor { amount: Amount::X, who: PlayerRel::Chosen }], \
                 targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
            ]
        );
        let stream = read(
            "Name:X\nManaCost:X G G\nTypes:Sorcery\n\
             A:SP$ GainLife | ValidTgts$ Player | LifeAmount$ X\nSVar:X:Count$xPaid",
        );
        assert_eq!(
            stream.abilities,
            [
                "spell!(&[Effect::GainLifeFor { amount: Amount::X, who: PlayerRel::Chosen }], \
                 targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
            ]
        );
        // A count is a count and not the announced X: read as one.
        let counted = read(
            "Name:X\nManaCost:2 G\nTypes:Sorcery\n\
             A:SP$ GainLife | LifeAmount$ X\nSVar:X:Count$Valid Creature.YouCtrl",
        );
        assert!(
            counted.abilities[0].contains("Effect::GainLife { amount: Amount::CountOf"),
            "{:?}",
            counted.abilities
        );
        // And the announced X on a trigger is still refused.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | \
             Execute$ G\nSVar:G:DB$ GainLife | LifeAmount$ X\nSVar:X:Count$xPaid"
        ));
    }

    /// Earthquake: "X damage to each creature without flying and each
    /// player" — the permanents through `DealDamageEach`, the players
    /// through `DealDamage`, and "without flying" as a keyword the engine
    /// has a bit for.
    #[test]
    fn damage_to_each_is_the_permanents_and_the_players() {
        let quake = read(
            "Name:X\nManaCost:X R\nTypes:Sorcery\n\
             A:SP$ DamageAll | ValidCards$ Creature.withoutFlying | ValidPlayers$ Player \
             | NumDmg$ X\nSVar:X:Count$xPaid",
        );
        let text = quake.abilities.join("\n");
        assert!(
            text.contains(
                "Effect::DealDamageEach { amount: Amount::X, filter: &EACH1 }, \
                 Effect::DealDamage { amount: Amount::X, target: TargetSpec::Player(\
                 PlayerRel::EachPlayer) }"
            ),
            "{text}"
        );
        assert!(
            quake
                .statics
                .contains("Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING))"),
            "{}",
            quake.statics
        );
        assert!(refused(
            "Name:X\nManaCost:R\nTypes:Sorcery\n\
             A:SP$ DamageAll | ValidCards$ Creature.withBushido | NumDmg$ 1"
        ));
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

    /// The other half of that rule: the absent key means the *line's* own
    /// target, and a sub-ability inherits none. Last Caress — "target player
    /// loses 1 life and you gain 1 life. Draw a card." — is a targeting
    /// `LoseLife` and then a bare `GainLife` and a bare `Draw`, and read
    /// against the chain's target it handed the life and the card to the
    /// player it was draining.
    #[test]
    fn an_undefined_player_effect_after_the_targeting_line_means_you() {
        let body = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ 1 | SubAbility$ DBGainLife | SpellDescription$ drain.\n\
             SVar:DBGainLife:DB$ GainLife | LifeAmount$ 1 | SubAbility$ DBDraw\n\
             SVar:DBDraw:DB$ Draw",
        );
        assert_eq!(
            body.abilities,
            [
                "spell!(&[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::Chosen }, \
                 Effect::gain_life(1), Effect::draw(1)], \
                 targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))"
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

        // The Laces: the target is what `Filter::This` binds to, the stack
        // is a zone it may be in, and "becomes" lasts the game.
        let lace = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Animate | Colors$ Red | OverwriteColors$ True | ValidTgts$ Card \
             | TgtZone$ Stack,Battlefield | Duration$ Permanent",
        );
        let a = lace.abilities.join("");
        assert!(
            a.contains(
                "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(\
                 &[Color::Red])), Duration::Indefinitely)"
            ),
            "{a}"
        );
        assert!(
            a.contains("TargetSpec::StackOrBattlefield(&Filter::Any)"),
            "{a}"
        );
        let exiled = parse(
            "Name:X\nTypes:Instant\n\
             A:SP$ Animate | Colors$ Red | ValidTgts$ Card | TgtZone$ Exile",
        );
        assert_eq!(
            refusal_reason(&exiled, &cats(), None).as_deref(),
            Some("a target in `TgtZone$ Exile`")
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

    /// A pump that **counts**, both ways round.
    ///
    /// The letter is not the number: `NumAtt$ -X` with `SVar:X:Count$xPaid`
    /// is the X a player announced and reads as `Amount::NegX`, while
    /// `Count$Valid Artifact.YouCtrl` is a board count and has nothing to do
    /// with an announced number at all. Both were refused, because a pump
    /// asked [`amount`] and [`amount`] reads no count; a mana line has asked
    /// [`Tx::counted_amount`] the whole time.
    ///
    /// The positive side is the larger half — 148 reference scripts against
    /// 26 — and comes from the same fallthrough, so refusing it would have
    /// been a guard written for no reason a card could state.
    #[test]
    fn a_pump_counts_in_either_direction() {
        // Irradiate: "-1/-1 until end of turn for each artifact you control".
        let body = read(
            "Name:Irradiate\nManaCost:3 B\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
             SVar:X:Count$Valid Artifact.YouCtrl\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("Effect::PumpTarget"), "{text}");
        assert_eq!(
            text.matches("Amount::Negated(&Amount::CountOf {").count(),
            2,
            "both sides of the pump negate the same count: {text}"
        );

        // Wirewood Pride's shape — "+X/+X, where X is the number of Elves" —
        // over a subtype this fixture's catalog knows. The same reading with
        // no sign in front of it.
        let body = read(
            "Name:Goblin Pride\nManaCost:G\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ X | NumDef$ X\n\
             SVar:X:Count$Valid Goblin\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("power: Amount::CountOf {"), "{text}");
        assert!(
            !text.contains("Negated"),
            "nothing here counts downwards: {text}"
        );

        // And a count this reader still cannot say is still a refusal, named
        // by what it resolves *through*. Blood Lust is the card; the
        // honest-stub rule does not care which side of the sign it is on.
        assert_eq!(
            refusal_reason(
                &parse(
                    "Name:Blood Lust\nManaCost:B\nTypes:Instant\n\
                     A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
                     SVar:X:Count$Compare T GE4.4.T\n"
                ),
                &cats(),
                None
            )
            .as_deref(),
            Some("pump amount `-X` = `Count$Compare T GE4.4.T`"),
            "a refused pump says what its letter resolves through"
        );
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

    /// A static's `IsPresent$` is the condition it exists under, and
    /// `GainControl$ You` is layer 2 to the static's controller.
    #[test]
    fn a_static_holds_while_its_clause_does_and_can_give_control() {
        // Sedge Troll, with a land the test catalog knows.
        let body = read(
            "Name:X\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
             S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 1 | AddToughness$ 1 | \
             IsPresent$ Mountain.YouCtrl | Description$ gets +1/+1.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("condition = Some(Condition::ControlCount(&CHECK"),
            "{text}"
        );
        assert!(text.trim_end().ends_with("))"), "{text}");

        // Control Magic: layer 2, to the static's controller.
        let body = read(
            "Name:X\nManaCost:2 U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ You | \
             Description$ You control enchanted creature.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("static_ability!(Filter::AttachedToBySource, Modifier::GainControl)"),
            "{text}"
        );

        // Control given to anyone else is not a sentence it can write.
        let script = parse(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ Opponent\n",
        );
        assert!(transcode(&script, &cats(), None).is_none());
    }

    /// Disrupting Scepter: the target player chooses the card, and the
    /// ability is activated only during its controller's turn.
    #[test]
    fn a_chosen_discard_on_your_turn_only() {
        let body = read(
            "Name:X\nManaCost:3\nTypes:Artifact\n\
             A:AB$ Discard | Cost$ 3 T | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose | \
             PlayerTurn$ True | SpellDescription$ Target player discards a card.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 1 }"),
            "{text}"
        );
        assert!(
            text.contains("condition = Some(Condition::YourTurn)"),
            "{text}"
        );
    }

    /// Psionic Blast: `DamageMap$` gathers, `DamageResolve` deals.
    #[test]
    fn gathered_damage_is_dealt_in_one_resolution() {
        let body = read(
            "Name:X\nManaCost:2 U\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 4 | DamageMap$ True | \
             SubAbility$ DBDealDamage | SpellDescription$ 4 to any target and 2 to you.\n\
             SVar:DBDealDamage:DB$ DealDamage | Defined$ You | NumDmg$ 2 | \
             SubAbility$ DBDamageResolve\n\
             SVar:DBDamageResolve:DB$ DamageResolve\n",
        );
        let text = body.abilities.join("\n");
        assert_eq!(text.matches("Effect::DealDamage").count(), 2, "{text}");
        assert!(
            text.contains("TargetSpec::Player(PlayerRel::You)"),
            "{text}"
        );
    }

    /// Zombie Master: a granted activated ability, "this permanent" being
    /// the one that has it; a granted ability that targets is refused.
    #[test]
    fn a_granted_ability_is_the_holders_own() {
        let body = read(
            "Name:X\nManaCost:1 B B\nTypes:Creature Goblin\nPT:2/3\n\
             S:Mode$ Continuous | Affected$ Card.Goblin+Other | AddAbility$ Regenerate | \
             Description$ Other Goblins have regenerate.\n\
             SVar:Regenerate:AB$ Regenerate | Cost$ B | SpellDescription$ Regenerate this permanent.\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("Modifier::GrantActivated {"), "{text}");
        assert!(text.contains("cost: cost!(\"{B}\")"), "{text}");
        assert!(text.contains("TargetSpec::ThisObject"), "{text}");
        assert!(text.contains("mana_ability: false"), "{text}");

        assert!(refused(
            "Name:X\nTypes:Creature Goblin\nPT:2/3\n\
             S:Mode$ Continuous | Affected$ Creature.Goblin | AddAbility$ Ping\n\
             SVar:Ping:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1\n"
        ));
    }

    /// Lifetap and Psychic Venom: a `Taps` trigger on another permanent, and
    /// "that land's controller" read off it.
    #[test]
    fn a_tapped_trigger_reads_any_permanent_and_its_controller() {
        let venom = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
             T:Mode$ Taps | ValidCard$ Card.AttachedBy | TriggerZones$ Battlefield | Execute$ D\n\
             SVar:D:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 2",
        );
        let a = venom.abilities.join("");
        assert!(
            a.contains("Trigger::BecomesTapped(&Filter::AttachedToBySource)"),
            "{a}"
        );
        assert!(
            a.contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
            "{a}"
        );
        let lifetap = read(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ Taps | ValidCard$ Forest.OppCtrl | TriggerZones$ Battlefield | Execute$ G\n\
             SVar:G:DB$ GainLife | LifeAmount$ 1",
        );
        assert!(
            lifetap
                .abilities
                .join("")
                .contains("Trigger::BecomesTapped(&TRIGGER"),
            "{:?}",
            lifetap.abilities
        );
    }

    /// Disintegrate and Magma Spray: "if it's a creature, it can't be
    /// regenerated this turn, and if it would die this turn, exile it
    /// instead", on an any-target and on a creature target. "A creature
    /// dealt damage this way" (`Remembered`), a condition on the rider and
    /// a player target are refused.
    #[test]
    fn exile_if_it_dies_and_no_regeneration_read_the_lines_target() {
        let disintegrate = read(
            "Name:X\nTypes:Sorcery\nManaCost:X R\n\
             A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ E | \
             ReplaceDyingDefined$ ThisTargetedCard.Creature\n\
             SVar:E:DB$ Effect | RememberObjects$ ParentTarget | ForgetOnMoved$ Battlefield | \
             StaticAbilities$ NoRegen | IsCurse$ True | ConditionDefined$ ParentTarget | \
             ConditionPresent$ Creature | AILogic$ CantRegenerate\n\
             SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered | \
             Description$ It can't be regenerated.\n\
             SVar:X:Count$xPaid",
        );
        let a = disintegrate.abilities.join("");
        assert!(
            a.contains(
                "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
                 &[Effect::ExileIfDiesThisTurn { target: TargetSpec::AnyTarget }] }"
            ),
            "{a}"
        );
        assert!(
            a.contains(
                "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
                 &[Effect::CantBeRegeneratedThisTurn { target: TargetSpec::AnyTarget }] }"
            ),
            "{a}"
        );
        let spray = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | ReplaceDyingDefined$ Targeted",
        );
        let a = spray.abilities.join("");
        assert!(
            a.contains("Effect::ExileIfDiesThisTurn { target: TargetSpec::Object("),
            "{a}"
        );
        assert!(!a.contains("IfTargetMatches"), "{a}");
        for refused_line in [
            "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | ReplaceDyingDefined$ Remembered.Creature",
            "A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3 | ReplaceDyingDefined$ Targeted",
            "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | \
             ReplaceDyingDefined$ ThisTargetedCard.Creature | ReplaceDyingCondition$ Kicked",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Instant\n{refused_line}")),
                "{refused_line}"
            );
        }
        // "A creature dealt damage this way can't be regenerated"
        // (Incinerate) asks whether damage was dealt.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SubAbility$ E | RememberDamaged$ True\n\
             SVar:E:DB$ Effect | RememberObjects$ Remembered.Creature | ForgetOnMoved$ Battlefield | \
             StaticAbilities$ NoRegen | IsCurse$ True\n\
             SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered"
        ));
    }

    /// Wheel of Fortune, Timetwister and Natural Selection: whose hand,
    /// whose graveyard, whose library. A library position, a random pick
    /// and a count the player announced are refused.
    #[test]
    fn whole_hands_graveyards_and_another_players_library_read_whose() {
        let wheel = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ Discard | Mode$ Hand | Defined$ Player | SubAbility$ D\n\
             SVar:D:DB$ Draw | Defined$ Player | NumCards$ 7",
        );
        let a = wheel.abilities.join("");
        assert!(
            a.contains("Effect::DiscardHand { who: PlayerRel::EachPlayer }"),
            "{a}"
        );
        let twister = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
             Destination$ Library | Shuffle$ True | UseAllOriginZones$ True",
        );
        let a = twister.abilities.join("");
        assert!(
            a.contains(
                "Effect::ShuffleIntoLibrary { who: PlayerRel::EachPlayer, hand: true, \
                 graveyard: true }"
            ),
            "{a}"
        );
        let feldon = read(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ ChangeZoneAll | ValidTgts$ Player | ChangeType$ Card | Origin$ Graveyard | \
             Destination$ Library | Shuffle$ True",
        );
        let a = feldon.abilities.join("");
        assert!(
            a.contains(
                "Effect::ShuffleIntoLibrary { who: PlayerRel::Chosen, hand: false, \
                 graveyard: true }"
            ),
            "{a}"
        );
        let selection = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ RearrangeTopOfLibrary | ValidTgts$ Player | NumCards$ 3 | MayShuffle$ True",
        );
        let a = selection.abilities.join("");
        assert!(
            a.contains("Effect::ReorderTopLibraryOf { who: PlayerRel::Chosen, count: 3 }"),
            "{a}"
        );
        assert!(
            a.contains(
                "Effect::MayDo { effects: &[Effect::ShuffleLibrary { who: PlayerRel::Chosen }] }"
            ),
            "{a}"
        );
        let mine = read(
            "Name:X\nTypes:Artifact\n\
             A:AB$ RearrangeTopOfLibrary | Cost$ 1 | Defined$ You | NumCards$ 3",
        );
        assert!(
            mine.abilities
                .join("")
                .contains("Effect::ReorderTopLibrary { count: 3 }"),
            "{:?}",
            mine.abilities
        );
        for refused_line in [
            "A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
             Destination$ Library | Shuffle$ True | Random$ True",
            "A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | \
             Destination$ Library | LibraryPosition$ -1",
            "A:SP$ RearrangeTopOfLibrary | Defined$ You | NumCards$ X",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Sorcery\n{refused_line}")),
                "{refused_line}"
            );
        }
    }

    /// Stone Giant: "toughness less than this creature's power" only where
    /// `X` is the source's power, and "destroy that creature at the
    /// beginning of the next end step" only on a targeted pump.
    #[test]
    fn a_comparison_with_the_sources_power_and_a_destroy_at_the_next_end_step() {
        let giant = read(
            "Name:X\nTypes:Creature\nPT:3/4\n\
             A:AB$ Pump | Cost$ T | ValidTgts$ Creature.YouCtrl+toughnessLTX | KW$ Flying | \
             AtEOT$ Destroy\n\
             SVar:X:Count$CardPower",
        );
        let a = giant.abilities.join("");
        assert!(
            a.contains(
                "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
            ),
            "{a}"
        );
        assert!(
            giant
                .statics
                .contains("Filter::ToughnessLessThanSourcePower"),
            "{}",
            giant.statics
        );
        for refused_card in [
            // `X` is not the source's power.
            "A:AB$ Pump | Cost$ T | ValidTgts$ Creature.toughnessLTX | KW$ Flying\n\
             SVar:X:Count$Valid Island.YouCtrl",
            // Another delayed sentence, and a delayed one about no target.
            "A:AB$ Pump | Cost$ T | ValidTgts$ Creature | KW$ Haste | AtEOT$ Sacrifice",
            "A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | AtEOT$ Destroy",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Creature\nPT:3/4\n{refused_card}")),
                "{refused_card}"
            );
        }
    }

    /// Stasis: "Players skip their untap steps" is every player's untap step,
    /// from the battlefield; a plane's skip from the command zone, another
    /// step, and a skip for one player refuse.
    #[test]
    fn a_skipped_untap_step_is_every_players() {
        let body = read(
            "Name:X\nTypes:Enchantment\n\
             R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Untap | Skip$ True | \
             Description$ Players skip their untap steps.",
        );
        assert_eq!(
            body.abilities,
            vec![
                "static_ability!(Filter::Any, Modifier::SkipUntapStep { who: PlayerRel::EachPlayer })"
            ]
        );
        for refused_line in [
            "R:Event$ BeginPhase | ActiveZones$ Command | Phase$ Untap | Skip$ True",
            "R:Event$ BeginPhase | Phase$ Draw | Skip$ True",
            "R:Event$ BeginPhase | Phase$ Untap | Skip$ True | ValidPlayer$ You",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
                "{refused_line}"
            );
        }
    }

    /// "Can attack as though it didn't have defender" and "…as though it
    /// had haste", on the creatures the line names; naming what may be
    /// attacked instead is another sentence and refuses.
    #[test]
    fn an_attack_as_though_is_a_permission_on_the_named_creatures() {
        let body = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ CanAttackIfHaste | ValidCard$ Creature.EnchantedBy | Description$ …",
        );
        assert!(
            body.abilities.iter().any(|a| a
                == "static_ability!(Filter::And(&[Filter::CREATURE, \
                    Filter::AttachedToBySource]), Modifier::AttacksAsThoughHaste)"),
            "{:?}",
            body.abilities
        );
        let body = read(
            "Name:X\nTypes:Creature\nPT:0/4\nK:Defender\n\
             S:Mode$ CanAttackDefender | ValidCard$ Card.Self | Description$ …",
        );
        assert!(
            body.abilities
                .iter()
                .any(|a| a == "static_ability!(Filter::This, Modifier::AttacksDespiteDefender)"),
            "{:?}",
            body.abilities
        );
        assert!(refused(
            "Name:X\nTypes:Creature\nPT:1/1\n\
             S:Mode$ CanAttackIfHaste | ValidTarget$ Opponent | Description$ …"
        ));
    }

    /// "Enchanted land is a Swamp" and "target land becomes a Forest": CR
    /// 305.7's setting of a land's subtype, from a static ability and from
    /// an `Animate`, and nothing but one basic land type is read that way.
    #[test]
    fn a_land_set_to_a_basic_land_type_is_read_as_one() {
        let body = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
             S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Mountain | \
             RemoveLandTypes$ True | Description$ …",
        );
        assert!(
            body.abilities.iter().any(|a| a
                == "static_ability!(Filter::AttachedToBySource, \
                    Modifier::SetLandType(subtypes::land::MOUNTAIN))"),
            "{:?}",
            body.abilities
        );
        let body = read(
            "Name:X\nTypes:Creature\nPT:1/1\n\
             A:AB$ Animate | Cost$ T | ValidTgts$ Land | Types$ Forest | \
             RemoveLandTypes$ True | Duration$ UntilHostLeavesPlay | SpellDescription$ …",
        );
        assert!(
            body.abilities.iter().any(|a| a.contains(
                "Effect::continuous(&Filter::This, Modifier::SetLandType(subtypes::land::FOREST), \
                 Duration::WhileSourceOnBattlefield)"
            )),
            "{:?}",
            body.abilities
        );
        let body = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
             K:ETBReplacement:Other:DBChooseBasic\n\
             SVar:DBChooseBasic:DB$ ChooseType | Type$ Basic Land | SpellDescription$ …\n\
             S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
             RemoveLandTypes$ True | Description$ …",
        );
        assert_eq!(body.enter_modifiers, ["EnterModifier::ChooseBasicLandType"]);
        assert!(
            body.abilities.iter().any(|a| a
                == "static_ability!(Filter::AttachedToBySource, Modifier::SetLandTypeToChosen)"),
            "{:?}",
            body.abilities
        );
        assert!(refused(
            "Name:X\nTypes:Enchantment\n\
             K:ETBReplacement:Other:DBChoose\n\
             SVar:DBChoose:DB$ ChooseType | Type$ Creature | SpellDescription$ …"
        ));
        // A chosen type nothing asked for as the card entered.
        for refused_line in [
            "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
             RemoveLandTypes$ True | Description$ …",
            "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Desert | \
             RemoveLandTypes$ True | Description$ …",
            "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Island Swamp | \
             RemoveLandTypes$ True | Description$ …",
        ] {
            assert!(
                refused(&format!(
                    "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n{refused_line}"
                )),
                "{refused_line}"
            );
        }
    }

    /// Smoke's and Winter Orb's "players can't untap more than one …
    /// during their untap steps": a limit on the players the line affects,
    /// with the Orb's "as long as this is untapped" as the static's
    /// condition. Another keyword beside it, another player, or a key the
    /// reader does not claim refuses.
    #[test]
    fn an_untap_limit_is_read_for_the_players_it_names() {
        let body = read(
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Creature:1 | \
             Description$ Players can't untap more than one creature during their untap steps.",
        );
        assert_eq!(
            body.abilities,
            vec![
                "static_ability!(Filter::Any, Modifier::UntapAtMost { who: PlayerRel::EachPlayer, \
                 of: &Filter::CREATURE, count: 1 })"
            ]
        );
        let body = read(
            "Name:X\nTypes:Artifact\n\
             S:Mode$ Continuous | Affected$ Player.Opponent | AddKeyword$ UntapAdjust:Land:2 | \
             IsPresent$ Card.Self+untapped | Description$ …",
        );
        assert_eq!(body.abilities.len(), 1);
        assert!(
            body.abilities[0].contains("who: PlayerRel::EachOpponent")
                && body.abilities[0].contains("count: 2")
                && body.abilities[0].contains("condition = Some(Condition::SourceMatches(&CHECK"),
            "{}",
            body.abilities[0]
        );
        for refused_line in [
            "S:Mode$ Continuous | Affected$ Creature | AddKeyword$ UntapAdjust:Land:1",
            "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:one",
            "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:1 | \
             AddHiddenKeyword$ Shroud",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
                "{refused_line}"
            );
        }
    }

    /// Cockatrice: "whenever this creature blocks or becomes blocked by a
    /// non-Wall creature, destroy that creature at end of combat", written
    /// by the reference as two lines, each executing a delayed trigger that
    /// remembers the other creature of its side of the block.
    const BLOCK_PAIR: &str = "Name:X\nTypes:Creature\nPT:2/4\n\
        T:Mode$ AttackerBlockedByCreature | ValidCard$ Creature.nonGoblin | \
        ValidBlocker$ Card.Self | Execute$ DelBlocked | TriggerDescription$ …\n\
        T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | \
        ValidBlocker$ Creature.nonGoblin | Execute$ DelBlocker | Secondary$ True | \
        TriggerDescription$ …\n\
        SVar:DelBlocked:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | \
        ValidPlayer$ Player | Execute$ TrigDestroy | \
        RememberObjects$ TriggeredAttackerLKICopy | TriggerDescription$ …\n\
        SVar:DelBlocker:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | \
        ValidPlayer$ Player | Execute$ TrigDestroy | \
        RememberObjects$ TriggeredBlockerLKICopy | TriggerDescription$ …\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI";

    /// The pair is one ability — the card prints one — about the other
    /// creature, destroyed by a delayed trigger at end of combat.
    #[test]
    fn a_blocks_or_becomes_blocked_pair_is_one_ability_at_end_of_combat() {
        let body = read(BLOCK_PAIR);
        assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
        let a = &body.abilities[0];
        assert!(a.contains("Trigger::BlocksOrBecomesBlockedBy(&"), "{a}");
        assert!(
            a.contains(
                "Effect::AtEndOfCombat { about: TargetSpec::EventObject, \
                 effects: &[Effect::destroy(TargetSpec::EventObject)] }"
            ),
            "{a}"
        );
        assert!(
            body.statics.contains("creature::GOBLIN"),
            "{}",
            body.statics
        );
    }

    /// Either half alone is another sentence, and so is a pair whose halves
    /// disagree; the remembered object outside a delayed body, a delayed
    /// trigger remembering its own source's side, and another phase are
    /// each refused.
    #[test]
    fn a_block_trigger_is_read_only_as_the_whole_pair() {
        let lines: Vec<&str> = BLOCK_PAIR.lines().collect();
        let without = |skip: usize| {
            lines
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != skip)
                .map(|(_, l)| *l)
                .collect::<Vec<_>>()
                .join("\n")
        };
        // The "blocks" half alone, and the "becomes blocked by" half alone.
        for skip in [3, 4] {
            let text = without(skip);
            assert!(refused(&text), "{text}");
            let reason = refusal_reason(&parse(&text), &cats(), None);
            assert!(
                reason
                    .as_deref()
                    .is_some_and(|r| r.contains("without its mirror")),
                "{reason:?}"
            );
        }
        for (from, to) in [
            // The halves are about different creatures.
            ("ValidBlocker$ Creature.nonGoblin", "ValidBlocker$ Creature"),
            // Both halves marked, or neither.
            (
                "Execute$ DelBlocked |",
                "Execute$ DelBlocked | Secondary$ True |",
            ),
            ("Secondary$ True | ", ""),
            // A delayed trigger about the source's own side of the block.
            (
                "RememberObjects$ TriggeredAttackerLKICopy",
                "RememberObjects$ TriggeredBlockerLKICopy",
            ),
            // Another phase.
            (
                "Phase$ EndCombat | ValidPlayer$ Player | Execute$ TrigDestroy | \
              RememberObjects$ TriggeredBlockerLKICopy",
                "Phase$ End of Turn | ValidPlayer$ Player | Execute$ TrigDestroy | \
              RememberObjects$ TriggeredBlockerLKICopy",
            ),
        ] {
            assert_eq!(BLOCK_PAIR.matches(from).count(), 1, "{from}");
            let text = BLOCK_PAIR.replace(from, to);
            assert!(refused(&text), "{from} → {to}");
        }
        // "Destroy that creature" remembered by nothing.
        assert!(refused(
            "Name:X\nTypes:Creature\nPT:2/4\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             ValidCard$ Card.Self | Execute$ TrigDestroy\n\
             SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI"
        ));
    }

    /// Hypnotic Specter and Fungusaur: damage to an opponent in or out of
    /// combat, "that player" the one dealt damage, and "is dealt damage"
    /// once however many sources. A player dealt damage "once", a creature
    /// target, and damage to a player out of combat are refused.
    #[test]
    fn damage_triggers_read_whose_damage_and_to_whom() {
        let specter = read(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Opponent | Execute$ D | \
             TriggerZones$ Battlefield\n\
             SVar:D:DB$ Discard | Defined$ TriggeredTarget | NumCards$ 1 | Mode$ Random",
        );
        let a = specter.abilities.join("");
        assert!(
            a.contains("Trigger::DealsDamageToOpponent(&Filter::This)"),
            "{a}"
        );
        assert!(a.contains("who: PlayerRel::DamagedPlayer"), "{a}");
        let combat = read(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True \
             | Execute$ D\n\
             SVar:D:DB$ Draw | NumCards$ 1",
        );
        assert!(
            combat
                .abilities
                .join("")
                .contains("Trigger::DealsCombatDamageToPlayer(&Filter::This)"),
            "{:?}",
            combat.abilities
        );
        let fungusaur = read(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ DamageDoneOnce | ValidTarget$ Card.Self | Execute$ C\n\
             SVar:C:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
        );
        assert!(
            fungusaur
                .abilities
                .join("")
                .contains("Trigger::DealtDamage(&Filter::This)"),
            "{:?}",
            fungusaur.abilities
        );
        for refused_line in [
            "T:Mode$ DamageDoneOnce | ValidTarget$ You | Execute$ C",
            "T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | Execute$ C",
            "T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ C",
        ] {
            assert!(
                refused(&format!(
                    "Name:X\nTypes:Creature\nPT:2/2\n{refused_line}\n\
                     SVar:C:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1"
                )),
                "{refused_line}"
            );
        }
    }

    /// Gauntlet of Might, Manabarbs, Badgermole Cub: who tapped it is
    /// `Activator$` (nobody named is anybody), "its controller" and "that
    /// player" are the tapped permanent's controller, and mana for them is
    /// `AddManaFor`.
    #[test]
    fn a_permanent_tapped_for_mana_names_who_tapped_it_and_whose_mana_it_is() {
        let gauntlet = read(
            "Name:X\nTypes:Artifact\n\
             T:Mode$ TapsForMana | ValidCard$ Mountain | Execute$ TrigMana | TriggerZones$ \
             Battlefield | Static$ True | TriggerDescription$ x\n\
             SVar:TrigMana:DB$ Mana | Produced$ R | Amount$ 1 | Defined$ TriggeredCardController",
        );
        let a = gauntlet.abilities.join("");
        assert!(a.contains("by: PlayerRel::EachPlayer"), "{a}");
        assert!(
            a.contains(
                "Effect::AddManaFor { who: PlayerRel::ControllerOfEvent, color: ManaColor::Red, \
                 amount: 1 }"
            ),
            "{a}"
        );
        let barbs = read(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ TapsForMana | ValidCard$ Land | TriggerZones$ Battlefield | Execute$ D\n\
             SVar:D:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ 1",
        );
        assert!(
            barbs
                .abilities
                .join("")
                .contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
            "{:?}",
            barbs.abilities
        );
        let cub = read(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ TapsForMana | ValidCard$ Creature | Activator$ You | Execute$ M | \
             TriggerZones$ Battlefield | Static$ True\n\
             SVar:M:DB$ Mana | Produced$ G",
        );
        assert!(
            cub.abilities.join("").contains("by: PlayerRel::You"),
            "{:?}",
            cub.abilities
        );
        // "That player" of any other trigger is not the tapper.
        assert!(refused(
            "Name:X\nTypes:Enchantment\n\
             T:Mode$ Attacks | ValidCard$ Creature | Execute$ D\n\
             SVar:D:DB$ DealDamage | Defined$ TriggeredActivator | NumDmg$ 1"
        ));
    }

    /// Mana Short: the line's player target is whose lands tap, and the
    /// sub-line's `Defined$ Targeted` is the same player. A `Targeted` on a
    /// chain that targets no player names nobody and is refused; a `TapAll`
    /// that names nobody is every matching permanent.
    #[test]
    fn tapping_all_of_a_players_lands_and_their_mana_reads_the_chains_player() {
        let short = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ TapAll | ValidTgts$ Player | ValidCards$ Land | SubAbility$ DrainMana\n\
             SVar:DrainMana:DB$ DrainMana | Defined$ Targeted",
        );
        let a = short.abilities.join("");
        assert!(
            a.contains("Effect::TapAllOf { who: PlayerRel::Chosen, filter: &Filter::LAND }"),
            "{a}"
        );
        assert!(
            a.contains("Effect::LoseUnspentMana { who: PlayerRel::Chosen }"),
            "{a}"
        );
        assert!(a.contains("TargetSpec::AnyPlayer"), "{a}");
        // `Targeted` where the chain targets no player.
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ DrainMana | Defined$ Targeted"
        ));
        // `TargetedController` of a spell target is its controller.
        let spell = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ Drain\n\
             SVar:Drain:DB$ DrainMana | Defined$ TargetedController",
        );
        assert!(
            spell
                .abilities
                .join("")
                .contains("Effect::LoseUnspentMana { who: PlayerRel::ControllerOfTarget }"),
            "{:?}",
            spell.abilities
        );
        // Twiddle: "tap or untap" is a yes or a no around the toggle.
        let twiddle = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ TapOrUntap | ValidTgts$ Artifact,Creature,Land",
        );
        assert!(
            twiddle
                .abilities
                .join("")
                .contains("Effect::MayDo { effects: &[Effect::ToggleTapTarget] }"),
            "{:?}",
            twiddle.abilities
        );
        let all = read("Name:X\nTypes:Sorcery\nA:SP$ TapAll | ValidCards$ Creature");
        assert!(
            all.abilities
                .join("")
                .contains("Effect::TapAll { filter: &Filter::CREATURE }"),
            "{:?}",
            all.abilities
        );
    }

    /// Pestilence, Karma, Spell Blast, Dwarven Warriors: counts of
    /// everybody's permanents, the active player's permanents, a mana value
    /// of exactly X, and a creature nobody can block this turn.
    #[test]
    fn an_unless_cost_wraps_the_line_whatever_it_says() {
        // Phantasmal Forces: a colour, printed exactly.
        let forces = read(
            "Name:X\nTypes:Creature\nPT:5/1\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
             | Execute$ U | TriggerDescription$ x.\n\
             SVar:U:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ U",
        );
        assert!(
            forces.abilities[0].contains(
                "Effect::PlayerMayPayManaOr { player: PlayerRel::You, \
                 cost: mana!(\"{U}\"), effect: &Effect::SacrificeSelf }"
            ),
            "{:?}",
            forces.abilities
        );
        // Force of Nature: the price is the line's, whatever the line does.
        let nature = read(
            "Name:X\nTypes:Creature\nPT:8/8\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
             | Execute$ D | TriggerDescription$ x.\n\
             SVar:D:DB$ DealDamage | Defined$ You | NumDmg$ 8 | UnlessCost$ G G G G \
             | UnlessPayer$ You",
        );
        assert!(
            nature.abilities[0]
                .contains("cost: mana!(\"{G}{G}{G}{G}\"), effect: &Effect::DealDamage"),
            "{:?}",
            nature.abilities
        );
        // Generic mana stays the `Amount` tax, and Mana Leak's absent payer
        // is its target's controller.
        let leak = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ 3",
        );
        assert!(
            leak.abilities[0].contains(
                "Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, \
                 mana: Amount::Fixed(3), effect: &Effect::CounterTargetSpell }"
            ),
            "{:?}",
            leak.abilities
        );
        // Switched: paying buys the effect.
        let bought = read(
            "Name:X\nTypes:Artifact\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
             | Execute$ G | TriggerDescription$ x.\n\
             SVar:G:DB$ GainLife | LifeAmount$ 1 | UnlessCost$ W | UnlessPayer$ You \
             | UnlessSwitched$ True",
        );
        assert!(
            bought.abilities[0].contains("Effect::PlayerMayPayManaThen"),
            "{:?}",
            bought.abilities
        );
        // Refused by name: a payer this cannot ask, subs on one answer, and
        // an absent payer on a line with no target to take one from.
        for (line, reason) in [
            (
                "DB$ Sacrifice | UnlessPayer$ Player | UnlessCost$ 2",
                "an unless-cost paid by `Player`",
            ),
            (
                "DB$ Sacrifice | UnlessCost$ 2",
                "an unless-cost paid by `TargetedController`",
            ),
            (
                "DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ 2 | UnlessResolveSubs$ WhenNotPaid",
                "`UnlessResolveSubs$ WhenNotPaid`",
            ),
            (
                "DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ PayLife<2>",
                "an unless-cost of `PayLife<2>`",
            ),
        ] {
            let script = parse(&format!(
                "Name:X\nTypes:Creature\nPT:1/1\n\
                 T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield \
                 | Execute$ S | TriggerDescription$ x.\nSVar:S:{line}"
            ));
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(reason),
                "{line}"
            );
        }
    }

    #[test]
    fn counts_bounds_and_an_unblockable_target() {
        let body = read(
            "Name:X\nManaCost:B B\nTypes:Enchantment\n\
             T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | \
             IsPresent$ Creature | PresentCompare$ EQ0 | Execute$ S\nSVar:S:DB$ Sacrifice\n",
        );
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(
            text.contains("Condition::BattlefieldCountAtMost(&Filter::CREATURE, 0)"),
            "{text}"
        );
        // "At the beginning of the end step": everybody's, not yours.
        assert!(text.contains("whose: PlayerRel::EachPlayer"), "{text}");
        // And Pestilence's ability, whose printed recipients are a key the
        // rule reading the recipients claims.
        let body = read(
            "Name:X\nManaCost:B B\nTypes:Enchantment\n\
             A:AB$ DamageAll | Cost$ B | NumDmg$ 1 | ValidCards$ Creature | \
             ValidPlayers$ Player | ValidDescription$ each creature and each player.\n",
        );
        assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
        // Somebody's permanents, but not yours: still refused.
        assert!(refused(
            "Name:X\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | \
             IsPresent$ Creature.OppCtrl | Execute$ S\nSVar:S:DB$ Sacrifice\n"
        ));
        // An exact count above zero is two bounds.
        assert!(refused(
            "Name:X\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | \
             IsPresent$ Creature | PresentCompare$ EQ2 | Execute$ S\nSVar:S:DB$ Sacrifice\n"
        ));

        let body = read(
            "Name:X\nManaCost:B B\nTypes:Enchantment\n\
             T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | \
             Execute$ D\nSVar:D:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ X\n\
             SVar:X:Count$Valid Creature.ActivePlayerCtrl\n",
        );
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(text.contains("Filter::ControlledByActivePlayer"), "{text}");
        assert!(text.contains("amount: Amount::CountOf"), "{text}");

        let body = read(
            "Name:X\nManaCost:U\nTypes:Instant\n\
             A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.cmcEQX\nSVar:X:Count$xPaid\n",
        );
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(text.contains("Filter::CmcExactlyX"), "{text}");
        // X that counts something else is not the announcement.
        assert!(refused(
            "Name:X\nManaCost:U\nTypes:Instant\n\
             A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.cmcEQX\n\
             SVar:X:Count$Valid Creature.YouCtrl\n"
        ));

        let body = read(
            "Name:X\nManaCost:2 R\nTypes:Creature\n\
             A:AB$ Effect | Cost$ T | ValidTgts$ Creature.powerLE2 | RememberObjects$ Targeted | \
             ExileOnMoved$ Battlefield | StaticAbilities$ U\n\
             SVar:U:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | Description$ No.\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("keywords: KeywordSet::UNBLOCKABLE"), "{text}");
        assert!(text.contains("Effect::PumpTarget"), "{text}");
        // A blocker named is "can't be blocked by …", another sentence.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             A:AB$ Effect | Cost$ T | ValidTgts$ Creature | RememberObjects$ Targeted | \
             ExileOnMoved$ Battlefield | StaticAbilities$ U\n\
             SVar:U:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | ValidBlocker$ Wall\n"
        ));
    }

    /// Keldon Warlord, Plague Rats, Time Walk, Regeneration: the sentences
    /// the DSL already had words for.
    #[test]
    fn characteristic_counts_extra_turns_and_the_enchanted_host() {
        let body = read(
            "Name:X\nManaCost:2 R R\nTypes:Creature Goblin\nPT:*/*\n\
             S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X | \
             Description$ Its power.\n\
             SVar:X:Count$Valid Creature.nonWizard+YouCtrl\n",
        );
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(
            text.contains("count: PtCount::YouControl(&COUNTED1)"),
            "{text}"
        );
        assert!(text.contains("Filter::ControlledByYou"), "{text}");

        let body = read(
            "Name:X\nManaCost:2 B\nTypes:Creature Rat\nPT:*/*\n\
             S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X\n\
             SVar:X:Count$Valid Creature.namedPlague Rats\n",
        );
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(text.contains("PtCount::OnBattlefield(&COUNTED1)"), "{text}");
        assert!(text.contains("Filter::Named(\"Plague Rats\")"), "{text}");

        // Somebody else's permanents, and a clause beside the count.
        assert!(refused(
            "Name:X\nTypes:Creature\nS:Mode$ Continuous | CharacteristicDefining$ True | \
             SetPower$ X | SetToughness$ X\nSVar:X:Count$Valid Forest.DefenderCtrl\n"
        ));
        assert!(refused(
            "Name:X\nTypes:Creature\nS:Mode$ Continuous | CharacteristicDefining$ True | \
             IsPresent$ Card.Self+attacking | SetPower$ X | SetToughness$ X\n\
             SVar:X:Count$Valid Forest.YouCtrl\n"
        ));

        let body = read("Name:X\nManaCost:U\nTypes:Sorcery\nA:SP$ AddTurn | NumTurns$ 1\n");
        assert_eq!(body.abilities.len(), 1);
        assert!(body.abilities[0].contains("Effect::TakeExtraTurn"));
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ AddTurn | NumTurns$ 2\n"
        ));

        let body = read(
            "Name:X\nManaCost:1 G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             A:AB$ Regenerate | Cost$ G | Defined$ Enchanted | SpellDescription$ Regenerate.\n",
        );
        assert!(
            body.abilities
                .join("")
                .contains("Effect::RegenerateAll { filter: &Filter::AttachedToBySource }"),
            "{:?}",
            body.abilities
        );
    }

    /// The Circles of Protection and Reverse Damage: a source chosen as the
    /// line resolves, and a shield on you that waits for it.
    #[test]
    fn a_chosen_source_shield_is_one_sentence_over_three_lines() {
        const COP: &str = "Name:X\nManaCost:1 W\nTypes:Enchantment\n\
             A:AB$ ChooseSource | Cost$ 1 | Choices$ Card.RedSource | AILogic$ NeedsPrevention | \
             SubAbility$ DBEffect | SpellDescription$ The next time.\n\
             SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | SubAbility$ DBCleanup | \
             ConditionDefined$ ChosenCard | ConditionPresent$ Card | ConditionCompare$ GE1\n\
             SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict+RedSource | \
             ValidTarget$ You | ReplaceWith$ ExileEffect | PreventionEffect$ True | \
             Description$ Prevent it.\n\
             SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
             SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True\n";
        let body = read(COP);
        let text = format!("{}\n{}", body.abilities.join("\n"), body.statics);
        assert!(
            text.contains("Effect::PreventNextFromChosenSource { sources: &SOURCE1, combat_only: false, all_but: 0, gain_life: false }"),
            "{text}"
        );
        assert!(text.contains("Color::Red"), "{text}");

        let body = read(
            "Name:X\nManaCost:1 W W\nTypes:Instant\n\
             A:SP$ ChooseSource | Choices$ Card,Emblem | SubAbility$ DBEffect\n\
             SVar:DBEffect:DB$ Effect | ReplacementEffects$ RPrevent | ConditionDefined$ ChosenCard | \
             ConditionPresent$ Card,Emblem\n\
             SVar:RPrevent:Event$ DamageDone | ValidSource$ Card.ChosenCardStrict,Emblem.ChosenCard | \
             ValidTarget$ You | ReplaceWith$ GainLifeInstead | PreventionEffect$ True\n\
             SVar:GainLifeInstead:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ ExileEffect\n\
             SVar:ExileEffect:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
             SVar:X:ReplaceCount$DamageAmount\n",
        );
        assert!(
            body.abilities
                .join("\n")
                .contains("sources: &Filter::Any, combat_only: false, all_but: 0, gain_life: true"),
            "{:?}",
            body.abilities
        );

        // A replacement that does not recheck what was chosen, a delayed
        // trigger in place of the cleanup, and a chosen colour.
        assert!(refused(&COP.replace(
            "Card.ChosenCardStrict+RedSource",
            "Card.ChosenCardStrict"
        )));
        assert!(refused(&COP.replace(
            "SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
            "SVar:DBCleanup:DB$ DelayedTrigger | Mode$ Phase"
        )));
        assert!(refused(
            &COP.replace("Card.RedSource", "Card.ChosenColorSource")
        ));
    }

    /// Samite Healer, Conservator, Fog: the shields a line names.
    #[test]
    fn prevention_is_a_shield_on_what_the_line_names() {
        let body = read(
            "Name:X\nManaCost:1 W\nTypes:Creature Goblin\nPT:1/1\n\
             A:AB$ PreventDamage | Cost$ T | ValidTgts$ Any | Amount$ 1 | \
             SpellDescription$ Prevent the next 1 damage.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains(
                "Effect::PreventNextDamage { target: TargetSpec::AnyTarget, amount: Amount::Fixed(1) }"
            ),
            "{text}"
        );
        assert!(
            text.contains("target = Some(TargetSpec::AnyTarget)"),
            "{text}"
        );

        let body = read(
            "Name:X\nManaCost:2\nTypes:Artifact\n\
             A:AB$ PreventDamage | Cost$ 3 T | Defined$ You | Amount$ 2 | \
             SpellDescription$ Prevent the next 2 damage that would be dealt to you.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("target: TargetSpec::Player(PlayerRel::You), amount: Amount::Fixed(2)"),
            "{text}"
        );

        let body = read("Name:X\nManaCost:G\nTypes:Instant\nA:SP$ Fog | SpellDescription$ Fog.\n");
        assert!(
            body.abilities
                .join("\n")
                .contains("Effect::PreventAllCombatDamageThisTurn"),
            "{:?}",
            body.abilities
        );

        // A shield on nothing, and a Fog narrowed by a key it does not claim.
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ PreventDamage | Amount$ 2\n"
        ));
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ Fog | ValidSource$ Creature.nonBlack\n"
        ));
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ PreventDamage | ValidTgts$ Any | Amount$ 3 | \
             DividedAsYouChoose$ 3\n"
        ));
    }

    /// A charm is a modal spell, each mode targeting for itself.
    #[test]
    fn a_charm_is_a_modal_spell_whose_modes_target_for_themselves() {
        let body = read(
            "Name:X\nManaCost:U\nTypes:Instant\n\
             A:SP$ Charm | Choices$ DBCounter,DBDestroy\n\
             SVar:DBCounter:DB$ Counter | TargetType$ Spell | ValidTgts$ Card.Red | \
             SpellDescription$ Counter target red spell.\n\
             SVar:DBDestroy:DB$ Destroy | ValidTgts$ Permanent.Red | \
             SpellDescription$ Destroy target red permanent.\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("AbilityDef::ModalSpell"), "{text}");
        assert!(text.contains("choose: ModeCount::ONE"), "{text}");
        assert_eq!(text.matches("mode!(").count(), 2, "{text}");
        assert!(text.contains("TargetSpec::Spell("), "{text}");
        assert!(text.contains("TargetSpec::Object("), "{text}");

        // A mode the reader cannot say takes the whole card with it.
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ Charm | Choices$ DBGain,DBBalance\n\
             SVar:DBGain:DB$ GainLife | ValidTgts$ Player | LifeAmount$ 3\n\
             SVar:DBBalance:DB$ Balance | Valid$ Land\n"
        ));
        // A count it has not met.
        assert!(refused(
            "Name:X\nTypes:Instant\nA:SP$ Charm | Choices$ A,B | MinCharmNum$ 0\n\
             SVar:A:DB$ Draw | NumCards$ 1\nSVar:B:DB$ Draw | NumCards$ 2\n"
        ));
    }

    /// `CantBlockBy` from either side of the pairing.
    #[test]
    fn a_pairing_nobody_may_block_is_the_attackers_restriction() {
        // Invisibility: every creature but a Wall is refused.
        let body = read(
            "Name:X\nManaCost:U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ CantBlockBy | ValidAttacker$ Creature.EnchantedBy | \
             ValidBlocker$ Creature.nonGoblin | Description$ can't be blocked except by Goblins.\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("Modifier::CantBeBlockedBy(&BLOCKER"),
            "{text}"
        );
        assert!(text.contains("Filter::AttachedToBySource"), "{text}");
        let statics = &body.statics;
        assert!(statics.contains("Not("), "{statics}");

        // Ironclaw Orcs: the source is the blocker, every big creature the
        // attacker.
        let body = read(
            "Name:X\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\n\
             S:Mode$ CantBlockBy | ValidAttacker$ Creature.powerGE2 | \
             ValidBlocker$ Creature.Self | Description$ can't block big ones.\n",
        );
        let statics = &body.statics;
        assert!(statics.contains("Filter::This"), "{statics}");

        assert!(refused(
            "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
             S:Mode$ CantBlockBy | ValidAttacker$ Creature.Self\n"
        ));
    }

    /// No `Defined$` and no target is the source too: Shivan Dragon's
    /// "{R}: This creature gets +1/+0 until end of turn." A pump that
    /// moves nothing stays refused, because pumping the source by nought
    /// would be a card that claims to work and does nothing.
    #[test]
    fn a_pump_naming_nobody_is_the_sources_own() {
        let body = read(
            "Name:X\nManaCost:4 R R\nTypes:Creature Dragon\nPT:5/5\nK:Flying\n\
             A:AB$ Pump | Cost$ R | NumAtt$ +1 | SpellDescription$ gets +1/+0.\n",
        );
        let text = body.abilities.join("\n");
        assert!(text.contains("filter: &Filter::This"), "{text}");
        assert!(text.contains("power: Amount::Fixed(1)"), "{text}");

        let script = parse("Name:X\nTypes:Instant\nA:SP$ Pump | StackDescription$ None");
        assert!(transcode(&script, &cats(), None).is_none());

        // An Aura's "enchanted creature gets +1/+0" is the host, not the
        // Aura: Firebreathing.
        let body = read(
            "Name:X\nManaCost:R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             A:AB$ Pump | Cost$ R | Defined$ Enchanted | NumAtt$ +1\n",
        );
        let text = body.abilities.join("\n");
        assert!(
            text.contains("filter: &Filter::AttachedToBySource"),
            "{text}"
        );
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
    /// such a permanent exists at all: `Condition::BattlefieldCount`. A
    /// filter naming somebody else's is a third question, refused by name.
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

        let anyone = read(&script.replace("Creature.YouCtrl", "Creature"));
        assert!(
            anyone.abilities[0].contains("Condition::BattlefieldCount(&Filter::CREATURE, 2)"),
            "{:?}",
            anyone.abilities
        );
        let theirs = parse(&script.replace("Creature.YouCtrl", "Creature.OppCtrl"));
        assert!(transcode(&theirs, &cats(), None).is_none());
        assert_eq!(
            refusal_reason(&theirs, &cats(), None).as_deref(),
            Some("`IsPresent$ Creature.OppCtrl`, a count of somebody else's")
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
            ("PresentCompare$ EQ2 | ", "`PresentCompare$ EQ2`"),
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

    /// `Defined$ Self` on a line that also targets is the **source**, and
    /// `Effect::AddCounter` cannot say so.
    ///
    /// That effect puts its counters on the first target when there is one,
    /// which is the right reading of a `PutCounter` line with no `Defined$`
    /// at all and the wrong one the moment the script names a subject. Both
    /// spellings were accepted and emitted identically, so Consumptive Goo —
    /// "{2}{B}{B}: Target creature gets -1/-1 until end of turn. Put a +1/+1
    /// counter on this creature." — put its counter on the creature it was
    /// shrinking, where the two cancelled each other exactly. The card
    /// compiled, claimed `Coverage::Implemented`, charged four mana and
    /// changed nothing any test could see.
    ///
    /// Both directions, because the fix is a *distinction*: the same line
    /// without a target must keep the simpler spelling, or one rule would
    /// have been traded for another.
    #[test]
    fn defined_self_beside_a_target_puts_the_counter_on_the_source() {
        let targeted = read(
            "Name:Goo\nManaCost:B B\nTypes:Creature Ooze\nPT:1/1\n\
             A:AB$ Pump | Cost$ 2 B B | ValidTgts$ Creature | NumAtt$ -1 | \
             NumDef$ -1 | SubAbility$ DBCounter\n\
             SVar:DBCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | \
             CounterNum$ 1\n",
        );
        let text = targeted.abilities.join("\n");
        assert!(
            text.contains("Effect::AddCounterFilter { filter: &Filter::This"),
            "the counter is the source's, not the target's: {text}"
        );
        assert!(
            !text.contains("Effect::AddCounter {"),
            "and the spelling that would land it on the target is gone: {text}"
        );

        let untargeted = read(
            "Name:Vault\nManaCost:no cost\nTypes:Land\n\
             A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ STORAGE | \
             CounterNum$ 1\n",
        );
        let text = untargeted.abilities.join("\n");
        assert!(
            text.contains("Effect::AddCounter {"),
            "with no target the source is already what `AddCounter` means, and \
             the simpler spelling is the one to keep: {text}"
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

    /// `Cost$ ExileFromGrave<1/Creature>` is a card the player names out of
    /// their own graveyard, so it is the filter as written with no "you
    /// control" added — the zone is the whole of the "your" — and the source
    /// naming itself
    /// is refused, because eternalize's "exile this card from your
    /// graveyard" is paid from a zone no ability is activated from yet.
    /// Three cards is three objects, which this reader refuses by name as it
    /// does for every other kind.
    #[test]
    fn a_graveyard_exile_cost_is_a_filter_over_the_payers_own_pile() {
        let body = read(
            "Name:X\nTypes:Land\n\
             A:AB$ Draw | Cost$ W U T ExileFromGrave<1/Creature> | NumCards$ 1\n",
        );
        assert_eq!(
            body.abilities,
            [
                "activated!(cost!(\"{W}{U}\", TapSelf, ExileFromGraveyard(&Filter::CREATURE)), \
                 &[Effect::draw(1)])"
            ],
            "the spelling Moorland Haunt is written in by hand"
        );
        assert!(
            body.statics.is_empty(),
            "a named filter needs no static, and nothing was added to it: {}",
            body.statics
        );

        for (cost, why) in [
            (
                "ExileFromGrave<1/CARDNAME/this card>",
                "cost `ExileFromGrave<1/CARDNAME/this card>` naming itself",
            ),
            ("ExileFromGrave<3/Card>", "a cost naming `3` objects"),
        ] {
            let script = parse(&format!(
                "Name:X\nTypes:Land\n\
                 A:AB$ Draw | Cost$ 1 {cost} | NumCards$ 1\n"
            ));
            assert!(
                transcode(&script, &cats(), None).is_none(),
                "{cost} is refused, not paid with one card"
            );
            assert_eq!(
                refusal_reason(&script, &cats(), None).as_deref(),
                Some(why),
                "and it says so by name"
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
            on_a_spell: false,
            trigger_mode: None,
            block_line: None,
            block_half: None,
            in_delayed: false,
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

    /// A colour word is a colour (CR 105.2), not an unknown subtype: Bad
    /// Moon, Crusade, Terror and Northern Paladin were all refused over it.
    #[test]
    fn a_colour_word_in_a_valid_string_is_the_colour() {
        let body = read(
            "Name:X\nTypes:Enchantment\n\
             S:Mode$ Continuous | Affected$ Creature.Black | AddPower$ 1 | AddToughness$ 1 | \
             Description$ Black creatures get +1/+1.\n",
        );
        assert_eq!(
            body.abilities,
            ["static_ability!(Filter::And(&[Filter::CREATURE, \
              Filter::HasColor(ColorSet::from_slice(&[Color::Black]))]), Modifier::ModifyPT(1, 1))"]
        );
        let terror = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Destroy | ValidTgts$ Creature.nonArtifact+nonBlack | NoRegen$ True\n",
        );
        let text = format!("{}{}", terror.statics, terror.abilities.join("\n"));
        assert!(
            text.contains("Filter::LacksType(TypeSet::ARTIFACT)"),
            "{text}"
        );
        assert!(
            text.contains("Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black])))"),
            "{text}"
        );
        // A fixed number compared with a power is read; one compared with
        // another object's power is not.
        assert_eq!(
            stat_atom("powerLE2").as_deref(),
            Some("Filter::PowerAtMost(2)")
        );
        assert_eq!(
            stat_atom("PowerGE3").as_deref(),
            Some("Filter::PowerAtLeast(3)")
        );
        assert_eq!(
            stat_atom("toughnessLT3").as_deref(),
            Some("Filter::ToughnessAtMost(2)")
        );
        assert_eq!(stat_atom("toughnessLTX"), None);
        assert_eq!(color_atom("nonWhite"), Some((true, "White")));
        assert_eq!(color_atom("Goblin"), None);
    }

    /// "Destroy all lands" and "destroy all creatures. They can't be
    /// regenerated" (Armageddon, Wrath of God): a sweep, not a target.
    #[test]
    fn destroy_all_sweeps_what_its_valid_string_names() {
        let body = read("Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Land\n");
        assert_eq!(
            body.abilities,
            ["spell!(&[Effect::destroy_all(&Filter::LAND)])"]
        );
        let wrath = read(
            "Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Creature | NoRegen$ True\n",
        );
        assert_eq!(
            wrath.abilities,
            ["spell!(&[Effect::destroy_all_no_regen(&Filter::CREATURE)])"]
        );
        // "Destroy all Forests" is the land type, and the Disk's three
        // types are an `or`.
        let disk = read(
            "Name:X\nTypes:Artifact\n\
             A:AB$ DestroyAll | Cost$ 1 T | ValidCards$ Artifact,Creature,Enchantment\n",
        );
        assert!(
            disk.abilities[0].contains("Effect::destroy_all("),
            "{:?}",
            disk.abilities
        );
        assert!(
            read("Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Forest\n").abilities[0]
                .contains("Filter::HasSubtype(")
        );
        assert!(refused(
            "Name:X\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Land | NoRegen$ Maybe\n"
        ));
    }

    /// Protection (CR 702.16a) is a static ability with a quality, printed
    /// or granted: the Knights print it, the Wards grant it with the one
    /// exception that keeps the Ward itself attached.
    #[test]
    fn protection_from_a_quality_is_a_static_ability() {
        let knight = read("Name:X\nTypes:Creature\nK:First Strike\nK:Protection from black\n");
        assert_eq!(knight.keywords, ["KeywordSet::FIRST_STRIKE"]);
        assert_eq!(
            knight.abilities,
            ["static_ability!(Filter::This, Modifier::ProtectionFrom(\
              &Filter::HasColor(ColorSet::from_slice(&[Color::Black]))))"]
        );
        let ward = read(
            "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | \
             AddKeyword$ Protection:Card.White:white:Card.CardUID_HostCardUID | \
             Description$ Enchanted creature has protection from white.\n",
        );
        let a = ward.abilities.join("\n");
        assert!(
            a.contains(
                "Modifier::ProtectionFrom(&Filter::And(&[Filter::HasColor(\
                 ColorSet::from_slice(&[Color::White])), Filter::Not(&Filter::This)]))"
            ),
            "{a}"
        );
        assert!(a.contains("Filter::AttachedToBySource"), "{a}");
        assert!(refused(
            "Name:X\nTypes:Creature\nK:Protection:Card.White:white:Card.Other\n"
        ));
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
        // A condition is a rule of its own; one it cannot read refuses.
        assert!(refused(
            "Name:X\nTypes:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | \
             IsPresent$ Island.YouCtrl | PresentCompare$ EQ2\n"
        ));
        // A mode that is not Continuous is not this rule.
        assert!(refused(
            "Name:X\nTypes:Creature\nS:Mode$ MustAttack | ValidCreature$ Card.Self\n"
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
    ///
    /// One of the three cases this pinned has since moved, which is what a
    /// pinned limitation is for. `Count$Valid …` is read now — see
    /// [`Self::a_pump_counts_in_either_direction`] — so what is left here is
    /// the counts that still have no shape in the DSL, and a reader that had
    /// become a catch-all would fail on them.
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
        // Oboro Envoy: a count of a **zone**, and negative, so the sign is
        // not what makes the difference. `Count$ValidHand` is one letter away
        // from the `Count$Valid ` the reader answers and is a different
        // question; a `strip_prefix` without its trailing space would take
        // this one and count the battlefield.
        assert!(refused(
            "Name:X\nTypes:Sorcery\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
             SVar:X:Count$ValidHand Card.YouOwn\n"
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
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | Bogus$ 2");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("unclaimed parameter `Draw.Bogus`")
        );

        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1");
        assert_eq!(refusal_reason(&script, &cats(), None), None, "read in full");

        // An unknown API is a missing effect, not a missing case in a rule
        // that exists, and is reported as its own kind. Leaving it silent
        // was worse than it looked: the report's fallback then guessed, and
        // named the first API *it* did not recognise — for a land whose
        // only unread line was `DB$ Discard`, that was the `R:Event$ Moved`
        // the transcoder had read perfectly well.
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Bogus | Defined$ Self");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("effect `Bogus`")
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
        let script = parse("Name:X\nTypes:Creature\nS:Mode$ MustAttack | ValidCreature$ Card.Self");
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some("static ability `S: Mode$ MustAttack`")
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
            // that decision is made. Blaze counters (Obsidian Fireheart) are
            // printed by no card in this pool and have no id. (This was
            // hatchling until Eumidian Hatchery's word was assigned one.)
            (
                "Name:X\nTypes:Creature\n\
                 A:AB$ PutCounter | Cost$ T | CounterType$ BLAZE | CounterNum$ 1",
                "counter `BLAZE`",
            ),
            (
                "Name:X\nTypes:Creature\nA:AB$ Untap | Cost$ AddCounter<1/BLAZE>",
                "counter `BLAZE`",
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
        // "For each creature that died this turn" is a count the DSL says.
        assert_eq!(
            read(
                "Name:X\nTypes:Creature\nK:etbCounter:P1P1:X\n\
                 SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
            )
            .enter_modifiers,
            ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
              amount: Amount::CreaturesDiedThisTurn }"]
        );
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

    /// Scavenging Ghoul: "put a corpse counter on this creature for each
    /// creature that died this turn" and "remove a corpse counter from this
    /// creature: regenerate this creature". A counter cost is a fixed number
    /// from the source; `X`, loyalty and the longer forms are refused.
    #[test]
    fn a_counter_removed_as_a_cost_and_a_count_of_the_turns_deaths() {
        let ghoul = read(
            "Name:X\nTypes:Creature\nPT:2/2\n\
             T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | \
             Execute$ TrigPutCounter\n\
             A:AB$ Regenerate | Cost$ SubCounter<1/CORPSE>\n\
             SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ CORPSE | \
             CounterNum$ X\n\
             SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature",
        );
        let a = ghoul.abilities.join("\n");
        assert!(
            a.contains("RemoveCounterSelf { kind: counters::CORPSE, n: 1 }"),
            "{a}"
        );
        assert!(
            a.contains(
                "Effect::AddCounter { kind: counters::CORPSE, \
                 amount: Amount::CreaturesDiedThisTurn }"
            ),
            "{a}"
        );
        for cost in [
            "SubCounter<X/CHARGE>",
            "SubCounter<1/LOYALTY>",
            "SubCounter<1/P1P1/Creature.YouCtrl/a creature you control>",
        ] {
            assert!(
                refused(&format!(
                    "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ {cost} | NumCards$ 1"
                )),
                "{cost}"
            );
        }
    }

    /// Jade Statue: "{2}: this becomes a 3/6 Golem artifact creature until
    /// end of combat. Activate only during combat."
    #[test]
    fn a_creature_type_that_replaces_until_end_of_combat_only_during_combat() {
        let statue = read(
            "Name:X\nTypes:Artifact\n\
             A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
             Types$ Creature,Artifact,Goblin | RemoveCreatureTypes$ True | \
             Duration$ UntilEndOfCombat | ActivationPhases$ BeginCombat->EndCombat",
        );
        let a = statue.abilities.join("\n");
        for part in [
            "Modifier::ReplaceCreatureTypes(subtypes::creature::GOBLIN), Duration::UntilEndOfCombat",
            "Modifier::AddType(TypeSet::CREATURE), Duration::UntilEndOfCombat",
            "Modifier::SetPT(3, 6), Duration::UntilEndOfCombat",
            "condition = Some(Condition::DuringCombat)",
        ] {
            assert!(a.contains(part), "{part} in {a}");
        }
        for refused_line in [
            // "Activate only during your upkeep" is another sentence.
            "A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
             Types$ Creature | ActivationPhases$ Upkeep",
            // Replacing creature types with none named is losing them all.
            "A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
             Types$ Creature | RemoveCreatureTypes$ True",
            // A restriction fills the condition, and the `IsPresent$` family
            // beside it is still unread.
            "A:AB$ Draw | Cost$ T | NumCards$ 1 | PlayerTurn$ True | PresentZone$ Graveyard",
        ] {
            assert!(
                refused(&format!("Name:X\nTypes:Artifact\n{refused_line}")),
                "{refused_line}"
            );
        }
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
            image_uris: None,
            image_status: None,
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

    /// CR 701.16a: "'Investigate' means 'Create a Clue token.'"
    ///
    /// The entry on the report said `effect Investigate`, ten pool stubs
    /// deep, and the DSL turned out to be able to say all of it already —
    /// the fifth time that has been true. So what is asserted here is the
    /// *spelling*: that the word reaches `Effect::CreateToken` with the
    /// Clue in it, and that it goes through the same token lookup every
    /// `TokenScript$` goes through rather than naming a constant of its own.
    #[test]
    fn investigating_is_creating_a_clue_token() {
        let land = read_with_tokens(
            "Name:X\nTypes:Land\n\
             A:AB$ Investigate | Cost$ 4 T | SpellDescription$ Investigate.",
        );
        assert_eq!(
            land.abilities,
            [
                "activated!(cost!(\"{4}\", TapSelf), &[Effect::CreateToken { token: &generated_tokens::CLUE }])"
            ]
        );

        // `Num$` is the same question `TokenAmount$` asks, and reaches the
        // same two spellings through the same reader — one is `CreateToken`
        // and more is `CreateTokenN`, so the commonest token effect there is
        // does not acquire a second way of being written.
        let twice = read_with_tokens(
            "Name:X\nTypes:Land\n\
             A:AB$ Investigate | Cost$ T | Num$ 2",
        );
        assert_eq!(
            twice.abilities,
            [
                "activated!(Cost::TAP, &[Effect::CreateTokenN { token: &generated_tokens::CLUE, amount: Amount::Fixed(2) }])"
            ]
        );
    }

    /// Each way of refusing to investigate, named by the reason it gives.
    ///
    /// `refused()` alone would pass on any of them for any reason at all,
    /// including one from a different rule entirely — which is the failure
    /// this file's own `refusal_reason` exists to make impossible. The
    /// player check is asserted to fire **before** the token lookup, so a
    /// machine with no token corpus still reports what is wrong with the
    /// *card* rather than what is missing from the machine.
    #[test]
    fn who_investigates_is_refused_before_anything_about_this_machine_is() {
        let why = |text: &str, tokens: Option<&TokenLookup>| {
            refusal_reason(&parse(text), &cats(), tokens).expect("a reason is recorded")
        };
        let held = tokens();

        // Somebody else investigating: `Effect::CreateToken` has no room
        // for an owner, exactly as a `Token` line under another player's
        // control has none.
        for who in [
            "Defined$ Opponent",
            "ValidPlayer$ Player",
            "Defined$ TargetedController",
        ] {
            let text = format!("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | {who}");
            assert_eq!(
                why(&text, Some(&held)),
                "somebody other than you investigating",
                "for {who}"
            );
            // And with no token corpus it still says that, because the
            // question about the card comes first.
            assert_eq!(why(&text, None), "somebody other than you investigating");
        }

        // Two keys for one question. No line in the corpus writes both,
        // which is what makes this a refusal rather than a precedence rule
        // nobody could check against anything.
        assert_eq!(
            why(
                "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Defined$ You | ValidPlayer$ You",
                Some(&held)
            ),
            "`Investigate` naming its player twice"
        );

        // "You may investigate" is a question this DSL cannot ask, and the
        // key is claimed by nothing, so the card refuses itself rather than
        // being read as the mandatory sentence beside it.
        assert!(
            why(
                "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Optional$ True",
                Some(&held)
            )
            .contains("Optional"),
            "the optional clause is named, not swallowed: got {:?}",
            why(
                "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Optional$ True",
                Some(&held)
            )
        );

        // And without the corpus the token half refuses in the same words
        // every other token refuses in.
        assert_eq!(
            why("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T", None),
            "`Token` with no token scripts to read it against"
        );
    }

    /// A body that is not a rules line at all is refused rather than
    /// parsed into one.
    ///
    /// `SVar:SacMe:1` is a bare number, and this read it as an API named
    /// "1" and then removed a parameter that had never been pushed. It
    /// panicked — in a generator, over a corpus, which is a run that stops
    /// on card 1 of 2716 — and it held for as long as it did only because
    /// every caller asked about a body some rules line had named.
    /// [`token_stems`] asks about all of them, and found it on the first
    /// run.
    #[test]
    fn a_body_with_no_api_at_all_is_refused_and_does_not_panic() {
        for body in ["1", "True", ""] {
            assert!(
                Params::parse(body).is_none(),
                "{body:?} was read as a rules line"
            );
        }
        // A counted `SVar` does have a `$` and so is read — as an API named
        // `Valid Creature.YouCtrl`, which matches nothing and is refused one
        // step later. That is the existing contract and not a second bug:
        // what the head means is the caller's question, and what this
        // function owes is an answer rather than a panic.
        assert_eq!(
            Params::parse("Count$Valid Creature.YouCtrl").map(|(api, _)| api),
            Some("Valid Creature.YouCtrl".to_string())
        );
        // And through the walk that reaches them: a card whose `SVar`s are
        // bare values is read, not a panic.
        assert!(
            token_stems(&parse(
                "Name:X\nTypes:Artifact\n\
                 A:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1\n\
                 SVar:SacMe:1\n"
            ))
            .is_empty()
        );
    }

    /// The ledger is told about the token a line does not name.
    ///
    /// The emitter and the ledger have to agree about which tokens a card
    /// needs: [`Tx::investigate_effect`] writes `generated_tokens::CLUE`,
    /// and if [`token_stems`] does not report the Clue then nothing ever
    /// asked the ledger to assign it. Today that survives only because
    /// another card in the pool names the Clue outright — which is luck,
    /// not a rule, and luck of exactly the kind that holds until the card
    /// naming it is cut.
    #[test]
    fn a_card_that_only_investigates_still_names_the_clue() {
        assert_eq!(
            token_stems(&parse("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ 4 T")),
            [CLUE_TOKEN_SCRIPT]
        );
        // Through an `SVar` chain as well, which is where the corpus puts
        // most of its token effects.
        assert_eq!(
            token_stems(&parse(
                "Name:X\nTypes:Creature\n\
                 T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
                 Execute$ Trig | TriggerDescription$ x\n\
                 SVar:Trig:DB$ Investigate"
            )),
            [CLUE_TOKEN_SCRIPT]
        );
        // Named once when the card investigates twice, since the list feeds
        // an append-only ledger where a repeat is a second id for one
        // permanent.
        assert_eq!(
            token_stems(&parse(
                "Name:X\nTypes:Land\n\
                 A:AB$ Investigate | Cost$ 4 T\n\
                 A:AB$ Investigate | Cost$ 6 T"
            )),
            [CLUE_TOKEN_SCRIPT]
        );
        // And the word in prose is not a line that makes one: this is the
        // reason the reading goes through `Params::parse` and not a search
        // for the word, which 21 of the corpus's 112 lines would fool.
        assert!(
            token_stems(&parse(
                "Name:X\nTypes:Land\n\
                 A:AB$ Mana | Cost$ T | Produced$ W | \
                 SpellDescription$ Add {W}. Investigate. (Create a Clue token.)"
            ))
            .is_empty()
        );
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
