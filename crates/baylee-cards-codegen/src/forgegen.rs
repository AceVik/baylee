//! forge-reference card script → `CardDef` abilities.
//!
//! A Forge card script is a line-oriented rules encoding:
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
//! Forge is read as an automated lookup only; no Forge file is copied into
//! this repository.

use crate::body::CardBody;
use crate::catalog::SubtypeCatalogs;
use std::collections::BTreeMap;

/// One parsed card script.
#[derive(Debug, Default)]
pub struct ForgeScript {
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
/// that are Forge's own deckbuilding/AI hints and carry no rules.
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
pub fn parse(text: &str) -> ForgeScript {
    let mut out = ForgeScript::default();
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
    body: CardBody,
    /// The first `Api.Key` no rule claimed, if that is why this
    /// script was refused. Recorded rather than derived, because a
    /// second list of each rule's keys would rot the first time a
    /// rule learned a new one.
    unclaimed: std::cell::RefCell<Option<String>>,
}

/// A whole number, or an `SVar` that resolves to one.
fn amount(raw: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    let raw = raw.trim().trim_start_matches('+');
    if let Ok(n) = raw.parse::<i64>() {
        return Some(format!("Amount::Fixed({n})"));
    }
    let resolved = svars.get(raw)?;
    let n = resolved.trim().parse::<i64>().ok()?;
    Some(format!("Amount::Fixed({n})"))
}

/// A `NumAtt`/`NumDef` value as an `Amount`.
///
/// Separate from [`amount`] because a pump is the one place a *negative*
/// constant is ordinary, and `Amount::Fixed` holds a `u32` — the sign
/// lives in the variant, not in the number.
fn pump_amount(raw: &str, svars: &BTreeMap<String, String>) -> Option<String> {
    let raw = raw.trim();
    // `+X/+X` is the second commonest pump printed, and X is a value the
    // engine already carries on the spell — the sign still lives in the
    // variant, so `-X` is its own one rather than a negated `X`.
    match raw {
        "X" | "+X" => return Some("Amount::X".to_string()),
        "-X" => return Some("Amount::NegX".to_string()),
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

    /// A Forge valid-string (`Creature.YouCtrl+nonToken`) as a `Filter`.
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
                    "nonLand" => "Filter::Not(&Filter::LAND)".to_string(),
                    "nonCreature" => "Filter::Not(&Filter::CREATURE)".to_string(),
                    // Supertypes read like subtypes in a forge filter but are
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
        Some(match alternatives.len() {
            0 => return None,
            1 => alternatives.remove(0),
            _ => format!("Filter::Or(&[{}])", alternatives.join(", ")),
        })
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
    /// Forge leaves `Defined$` off when the effect means the target, and only
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
        let (api, mut p) = Params::parse(spec)?;
        p.drop_prose();
        if let Some(valid) = p.take("ValidTgts") {
            let spec = self.target_spec(&valid, &api)?;
            if chain.target.get_or_insert(spec.clone()) != &spec {
                return None; // two different targets in one chain
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
                let body = self.svars.get(&name)?.clone();
                self.chain(&body, chain)
            }
            None => Some(()),
        }
    }

    /// One Forge effect API as the `Effect` expressions it stands for.
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
                let n = amount(&p.take("NumDmg")?, self.svars)?;
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
                let n = amount(&p.take("LifeAmount")?, self.svars)?;
                match Self::player_rel_of(p.take("Defined").as_deref(), target)? {
                    "PlayerRel::You" => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    who => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = amount(&p.take("LifeAmount")?, self.svars)?;
                let who = Self::player_rel_of(p.take("Defined").as_deref(), target)?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Draw" => {
                let n = amount(p.take("NumCards").as_deref().unwrap_or("1"), self.svars)?;
                match Self::player_rel_of(p.take("Defined").as_deref(), target)? {
                    "PlayerRel::You" => vec![format!("Effect::DrawCards {{ amount: {n} }}")],
                    who => vec![format!(
                        "Effect::DrawCardsFor {{ amount: {n}, who: {who} }}"
                    )],
                }
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars)?;
                let who = Self::player_rel_of(p.take("Defined").as_deref(), target)?;
                vec![format!("Effect::Mill {{ amount: {n}, target: {who} }}")]
            }
            "PutCounter" => {
                let kind = match p.take("CounterType")?.as_str() {
                    "P1P1" => "CounterKind::P1P1",
                    "M1M1" => "CounterKind::M1M1",
                    "LOYALTY" => "CounterKind::Loyalty",
                    "LORE" => "CounterKind::Lore",
                    "TIME" => "CounterKind::Time",
                    "CHARGE" => "CounterKind::Charge",
                    "POISON" => "CounterKind::Poison",
                    "ENERGY" => "CounterKind::Energy",
                    "RAD" => "CounterKind::Rad",
                    "LEVEL" => "CounterKind::Level",
                    _ => return None,
                };
                let n = amount(p.take("CounterNum").as_deref().unwrap_or("1"), self.svars)?;
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
                vec![format!("Effect::Scry {{ amount: Amount::Fixed({n}) }}")]
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
                vec![format!("Effect::Destroy {{ target: {aimed} }}")]
            }
            "Animate" => self.animate_effect(p, target)?,
            "Pump" => self.pump_effect(p, aimed)?,
            "ChangeZone" => self.change_zone(p, aimed)?,
            "Tap" => vec!["Effect::TapTarget".to_string()],
            "Untap" => vec!["Effect::UntapTarget".to_string()],
            "Counter" => {
                if p.take("TargetType").as_deref() != Some("Spell") {
                    return None;
                }
                vec!["Effect::CounterTargetSpell".to_string()]
            }
            _ => return None,
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
        // subtype, and Forge writes both in one list.
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
            out.push(Self::animate_expr("Layer::Type", &modifier));
        }
        // Layer 5: colour. Without `OverwriteColors$ True` the card keeps
        // the colours it had, which is `AddColor` (CR 613.1c).
        if let Some(raw) = p.take("Colors") {
            let overwrite = p.take("OverwriteColors").as_deref() == Some("True");
            let colors: Option<Vec<&str>> = raw.split(',').map(|c| color_const(c.trim())).collect();
            let Some(colors) = colors else {
                self.note(format!("`Animate` into colours `{raw}`"));
                return None;
            };
            let which = if overwrite { "SetColor" } else { "AddColor" };
            out.push(Self::animate_expr(
                "Layer::Color",
                &format!(
                    "Modifier::{which}(ColorSet::from_slice(&[{}]))",
                    colors.join(", ")
                ),
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
                "Layer::Ability",
                &format!("Modifier::AddKeyword({joined})"),
            ));
        }
        // Layer 7b: the printed P/T it takes on. Both halves or neither —
        // `SetPT` sets both, and half a set would invent the other.
        match (p.take("Power"), p.take("Toughness")) {
            (Some(power), Some(toughness)) => {
                let power: i16 = power.parse().ok()?;
                let toughness: i16 = toughness.parse().ok()?;
                out.push(Self::animate_expr(
                    "Layer::PtSet",
                    &format!("Modifier::SetPT({power}, {toughness})"),
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

    /// One layer of an [`Self::animate_effect`], as the `Effect` literal.
    fn animate_expr(layer: &str, modifier: &str) -> String {
        format!(
            "Effect::CreateContinuousEffect {{ layer: {layer}, filter: &Filter::This, \
             modifier: {modifier}, duration: Duration::UntilEndOfTurn }}"
        )
    }

    /// `Pump`: `NumAtt$ +2 | NumDef$ +2 | KW$ Trample`, the commonest
    /// effect in the whole script corpus.
    ///
    /// `Defined$ Self` and `Defined$ Targeted` are two different effects
    /// here, not one with a flag: `PumpFilter` binds `Filter::This` to the
    /// source, `PumpTarget` to what the spell targeted, and an ability can
    /// have both a target and a pump on itself.
    fn pump_effect(&mut self, p: &mut Params, target: &str) -> Option<Vec<String>> {
        let power = pump_amount(p.take("NumAtt").as_deref().unwrap_or("0"), self.svars)?;
        let toughness = pump_amount(p.take("NumDef").as_deref().unwrap_or("0"), self.svars)?;
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
                "Effect::PumpFilter {{ filter: &Filter::This, power: {power}, \
                 toughness: {toughness}, keywords: {keywords}, \
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
    /// Forge writes every zone change with one API and two zone names; the
    /// engine has a named effect per movement, because the movements differ
    /// in rules and not only in destination. So this is a table of pairs,
    /// not a translation of `Destination$` — and a pair with no effect
    /// refuses rather than reaching for the nearest one. Battlefield →
    /// Graveyard is the pair that makes the point: it is *not* `Destroy`,
    /// which checks indestructible (CR 700.4), and generating one for the
    /// other would quietly kill creatures that survive.
    fn change_zone(&self, p: &mut Params, target: &str) -> Option<Vec<String>> {
        let origin = p.take("Origin")?;
        let destination = p.take("Destination")?;
        let itself = match p.take("Defined").as_deref() {
            None => false,
            Some("Self") => true,
            Some(other) => {
                self.note(format!("`ChangeZone` of `Defined$ {other}`"));
                return None;
            }
        };
        // Without a target this would move nothing at all.
        if !itself && target == "TargetSpec::AnyPlayer" {
            self.note("`ChangeZone` with neither a target nor `Defined$`".to_string());
            return None;
        }
        Some(match (origin.as_str(), destination.as_str(), itself) {
            ("Battlefield", "Hand", false) => {
                vec![format!("Effect::ReturnToHand {{ target: {target} }}")]
            }
            ("Battlefield", "Exile", false) => {
                vec![format!("Effect::Exile {{ target: {target} }}")]
            }
            ("Battlefield", "Exile", true) => vec!["Effect::ExileSource".to_string()],
            _ => {
                self.note(format!("`ChangeZone` {origin} to {destination}"));
                return None;
            }
        })
    }

    /// `Produced$ Combo W U | Amount$ 2` and friends.
    fn mana_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let produced = p.take("Produced")?;
        let restrict = match p.take("RestrictValid") {
            None => None,
            Some(valid) => Some(self.spend_restriction(&valid)?),
        };
        let amount = plain_number(p.take("Amount").as_deref().unwrap_or("1"), self.svars)?;
        let amount = u32::try_from(amount).ok()?;
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
            (amount == 1).then(|| vec!["Effect::mana_of_any_color()".to_string()])?
        } else if let Some(list) = produced.strip_prefix("Combo ") {
            let colors: Option<Vec<&str>> = list.split_whitespace().map(color).collect();
            let colors = colors?;
            (amount == 1).then(|| vec![format!("Effect::mana_choice(&[{}])", colors.join(", "))])?
        } else {
            // `Produced$ W U` is "add {W}{U}" — two mana at once, not a
            // choice between them (that is `Combo`). One effect per colour,
            // which is how the bounce lands were already written by hand.
            let colors: Option<Vec<&str>> = produced.split_whitespace().map(color).collect();
            colors?
                .into_iter()
                .map(|c| format!("Effect::mana({c}, {amount})"))
                .collect()
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
    fn cost_expr(raw: &str) -> Option<String> {
        let mut mana = String::new();
        let mut parts: Vec<String> = Vec::new();
        for token in raw.split_whitespace() {
            if token == "T" {
                parts.push("CostPart::TapSelf".to_string());
            } else if token == "Q" {
                parts.push("CostPart::UntapSelf".to_string());
            } else if token.starts_with("Sac<1/CARDNAME") {
                parts.push("CostPart::SacrificeSelf".to_string());
            } else if let Some(n) = token
                .strip_prefix("PayLife<")
                .and_then(|t| t.strip_suffix('>'))
                .and_then(|t| t.parse::<u16>().ok())
            {
                parts.push(format!("CostPart::PayLife({n})"));
            } else if token.chars().all(|c| c.is_ascii_digit())
                || matches!(token, "W" | "U" | "B" | "R" | "G" | "C")
            {
                mana.push('{');
                mana.push_str(token);
                mana.push('}');
            } else {
                return None;
            }
        }
        if mana.is_empty() && parts == ["CostPart::TapSelf"] {
            return Some("Cost::TAP".to_string());
        }
        let mana_expr = if mana.is_empty() {
            "ManaCost::ZERO".to_string()
        } else {
            format!("baylee_core::mana!(\"{mana}\")")
        };
        Some(format!(
            "Cost {{ mana: {mana_expr}, parts: &[{}] }}",
            parts.join(", ")
        ))
    }

    /// `T:Mode$ …` as a `Trigger` expression.
    fn trigger_expr(&mut self, p: &mut Params, mode: &str) -> Option<String> {
        p.take("TriggerZones");
        match mode {
            "ChangesZone" => {
                let origin = p.take("Origin")?;
                let dest = p.take("Destination")?;
                let valid = p.take("ValidCard")?;
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                match (origin.as_str(), dest.as_str()) {
                    ("Any", "Battlefield") => Some(format!("Trigger::EntersBattlefield({filter})")),
                    ("Battlefield", "Graveyard") => Some(format!("Trigger::Dies({filter})")),
                    _ => None,
                }
            }
            "Phase" => {
                let step = match p.take("Phase")?.as_str() {
                    "Upkeep" => "StepKind::Upkeep",
                    "Draw" => "StepKind::Draw",
                    "BeginCombat" => "StepKind::CombatBegin",
                    "End of Turn" => "StepKind::End",
                    _ => return None,
                };
                let whose = Self::player_rel(p.take("ValidPlayer").as_deref())?;
                Some(format!(
                    "Trigger::StepBegin {{ step: {step}, whose: {whose} }}"
                ))
            }
            "Attacks" => {
                let valid = p.take("ValidCard")?;
                let filter = if valid == "Card.Self" {
                    "&Filter::This".to_string()
                } else {
                    let expr = self.filter_expr(&valid)?;
                    format!("&{}", self.body.filter_static("TRIGGER", &expr))
                };
                Some(format!("Trigger::Attacks({filter})"))
            }
            "Taps" => {
                let valid = p.take("ValidCard")?;
                (valid == "Card.Self").then(|| "Trigger::BecomesTapped(&Filter::This)".to_string())
            }
            _ => None,
        }
    }

    fn rule(&mut self, kind: char, spec: &str) -> Option<()> {
        match kind {
            'A' => self.activated_or_spell(spec),
            'T' => self.triggered(spec),
            'S' => self.static_ability(spec),
            'R' => self.replacement(spec),
            _ => None,
        }
    }

    /// An `R:` replacement, for the one shape the engine models as data.
    ///
    /// "Enters tapped" is a replacement effect in Forge and an
    /// `EnterModifier` here, and the difference matters: a modifier is read
    /// *as the permanent enters*, which is what CR 614.1c describes and what
    /// stops the land from being tapped a moment after it arrives untapped.
    /// Every other `Moved` replacement is a rule of its own and refuses.
    fn replacement(&mut self, spec: &str) -> Option<()> {
        let (event, mut p) = Params::parse(spec)?;
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
        let with = p.take("ReplaceWith")?;
        if !about_self || !entering || !updated || !p.exhausted() {
            self.note("replacement `Moved` this rule cannot read".to_string());
            return None;
        }
        let (api, mut body) = Params::parse(self.svars.get(&with)?)?;
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
        // A checkland taps *conditionally*: forge writes "tap it when you
        // control none of these", which is the printed "enters tapped unless
        // you control a Swamp or a Mountain" turned inside out. `EQ0` is the
        // only comparison that is that sentence — `GE2` and friends are
        // other cards, and a `ConditionCheckSVar$` is a computed value the
        // DSL cannot say at all.
        let modifier = match body.take("ConditionPresent") {
            None => "EnterModifier::Tapped".to_string(),
            Some(present) => {
                if body.take("ConditionCompare").as_deref() != Some("EQ0") {
                    self.note("replacement `Moved` with a condition other than `EQ0`".to_string());
                    return None;
                }
                let expr = self.filter_expr(&present)?;
                let name = self.body.filter_static("CHECK", &expr);
                format!("EnterModifier::TappedUnless(&{name})")
            }
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

    /// An `S: Mode$ Continuous` line as one or more `AbilityDef::Static`.
    ///
    /// One printed sentence can be several continuous effects: "get +1/+1
    /// and have flying" changes power/toughness in layer 7c and abilities
    /// in layer 6, and CR 613.1 applies those in order. Forge writes both
    /// on one line, so this emits one `StaticAbility` per layer touched
    /// rather than trying to fold them into one.
    fn static_ability(&mut self, spec: &str) -> Option<()> {
        let (mode, mut p) = Params::parse(spec)?;
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
        // Likewise `AffectedZone`: our `cross_zone` says the effect reaches
        // past the battlefield, and a filter that has no zone predicate in
        // it cannot say *which* other zone. Refuse rather than guess.
        if let Some(zone) = p.take("AffectedZone")
            && zone != "Battlefield"
        {
            self.note(format!("static ability reaching `AffectedZone$ {zone}`"));
            return None;
        }
        let filter = self.filter_expr(&p.take("Affected")?)?;
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
                "Layer::PtModify",
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
                "Layer::PtSet",
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
                "Layer::Ability",
                filter,
                &format!("Modifier::{modifier}({set})"),
            ));
        }
        Some(())
    }

    /// `AddType`/`RemoveType` (layer 4) as static abilities.
    ///
    /// Forge writes card types and subtypes in one list and the engine
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
                    "Layer::Type",
                    filter,
                    &format!("Modifier::{modifier}({set})"),
                ));
            }
            for path in subtypes {
                out.push(Self::static_expr(
                    "Layer::Type",
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
                "Layer::Color",
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
    fn static_expr(layer: &str, filter: &str, modifier: &str) -> String {
        format!(
            "AbilityDef::Static(StaticAbility {{ layer: {layer}, filter: {filter}, \
             modifier: {modifier}, cross_zone: false }})"
        )
    }

    fn activated_or_spell(&mut self, spec: &str) -> Option<()> {
        let is_activated = spec.starts_with("AB$");
        let (_, mut probe) = Params::parse(spec)?;
        probe.drop_prose();
        let cost = probe.take("Cost");
        let mut chain = Chain::default();
        // The cost belongs to the ability, not to the effect chain, so it is
        // removed from the spec before the chain reads it.
        let stripped: Vec<&str> = spec
            .split(" | ")
            .filter(|part| !part.starts_with("Cost$"))
            .collect();
        self.chain(&stripped.join(" | "), &mut chain)?;
        if chain.effects.is_empty() {
            return None;
        }
        let effects = format!("&[{}]", chain.effects.join(", "));
        if is_activated {
            let cost = Self::cost_expr(&cost?)?;
            let mana_ability = chain.effects.iter().all(|e| e.contains("Effect::mana"));
            let macro_name = if mana_ability {
                "mana_ability!"
            } else {
                "activated!"
            };
            let target = chain
                .target
                .map(|t| format!(", target: Some({t})"))
                .unwrap_or_default();
            self.body
                .abilities
                .push(format!("{macro_name}({cost}, {effects}{target})"));
        } else {
            let targets = chain
                .target
                .map(|t| format!(", targets: Some(TargetReq::one({t}))"))
                .unwrap_or_default();
            self.body
                .abilities
                .push(format!("spell!({effects}{targets})"));
        }
        Some(())
    }

    fn triggered(&mut self, spec: &str) -> Option<()> {
        let (mode, mut p) = Params::parse(spec)?;
        p.drop_prose();
        let trigger = self.trigger_expr(&mut p, &mode)?;
        let execute = p.take("Execute")?;
        if !p.exhausted() {
            return None;
        }
        let body = self.svars.get(&execute)?.clone();
        let mut chain = Chain::default();
        self.chain(&body, &mut chain)?;
        if chain.effects.is_empty() {
            return None;
        }
        let targets = chain
            .target
            .map(|t| format!(", targets: Some(TargetReq::one({t}))"))
            .unwrap_or_default();
        self.body.abilities.push(format!(
            "triggered!({trigger}, &[{}]{targets})",
            chain.effects.join(", ")
        ));
        Some(())
    }
}

/// A forge colour word as our `Color` constant.
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

/// A Forge type word as the `TypeSet` constant for it, or `None` when the
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

/// Forge keyword line → the bit in our `KeywordSet`, for the keywords that
/// are text-independent (CR 702). Parameterized keywords are data, not bits,
/// and are refused here on purpose.
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
        "Fear" => "KeywordSet::FEAR",
        "Intimidate" => "KeywordSet::INTIMIDATE",
        "Shadow" => "KeywordSet::SHADOW",
        "Horsemanship" => "KeywordSet::HORSEMANSHIP",
        "Infect" => "KeywordSet::INFECT",
        "Wither" => "KeywordSet::WITHER",
        "Persist" => "KeywordSet::PERSIST",
        "Undying" => "KeywordSet::UNDYING",
        "Prowess" => "KeywordSet::PROWESS",
        "Skulk" => "KeywordSet::SKULK",
        "Flanking" => "KeywordSet::FLANKING",
        "Changeling" => "KeywordSet::CHANGELING",
        _ => return None,
    })
}

/// Reads a whole card script, or refuses it.
///
/// # Errors
/// Never errors; an unreadable script is `None`, which is what keeps a
/// generated card honest.
#[must_use]
pub fn transcode(script: &ForgeScript, cats: &SubtypeCatalogs) -> Option<CardBody> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    let mut tx = Tx {
        svars: &script.svars,
        cats,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        tx.body.keywords.push(keyword_const(line)?.to_string());
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
pub fn refusal_reason(script: &ForgeScript, cats: &SubtypeCatalogs) -> Option<String> {
    if !script.unknown_lines.is_empty() {
        return None;
    }
    let mut tx = Tx {
        svars: &script.svars,
        cats,
        body: CardBody::default(),
        unclaimed: std::cell::RefCell::new(None),
    };
    for line in &script.keywords {
        keyword_const(line)?;
    }
    for (kind, spec) in &script.rules {
        if tx.rule(*kind, spec).is_none() {
            return tx.unclaimed.into_inner();
        }
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
    "Mana",
    "Destroy",
    "Tap",
    "Untap",
    "Counter",
    "PutCounter",
    "Pump",
    "ChangeZone",
];

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

/// Whether a Forge keyword line maps onto a `KeywordSet` bit.
#[must_use]
pub fn keyword_const_of(line: &str) -> Option<&'static str> {
    keyword_const(line)
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
pub fn atoms(script: &ForgeScript) -> Vec<String> {
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
            land: vec!["Forest".into()],
            ..SubtypeCatalogs::default()
        };
        c.normalize();
        c
    }

    fn read(text: &str) -> CardBody {
        transcode(&parse(text), &cats()).expect("should be read in full")
    }

    fn refused(text: &str) -> bool {
        transcode(&parse(text), &cats()).is_none()
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
        assert!(body.abilities[0].contains("targets: Some(TargetReq::one("));
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
            ["mana_ability!(Cost::TAP, &[Effect::mana_of_any_color()])"]
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
                "mana_ability!(Cost::TAP, &[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])",
                "mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::Colorless, 2)])",
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
            [
                "triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::GainLife { amount: Amount::Fixed(2) }])"
            ]
        );
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
                "spell!(&[Effect::DrawCards { amount: Amount::Fixed(2) }, Effect::LoseLife { amount: Amount::Fixed(2), target: PlayerRel::You }])"
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
                "triggered!(Trigger::EntersBattlefield(&Filter::This), &[Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::Chosen }], targets: Some(TargetReq::one(TargetSpec::AnyPlayer)))"
            ]
        );
    }

    /// A checkland: forge writes the printed "enters tapped **unless** you
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

    /// "Unless you control *two* other lands" is a count, not a presence
    /// test, and `TappedUnless` cannot say it — so the card stays a stub
    /// rather than becoming a land that enters untapped one land early.
    #[test]
    fn a_counted_enters_tapped_condition_is_refused() {
        let script = parse(
            "Name:X\nTypes:Land\n\
             R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n\
             SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Land.Other+YouCtrl | ConditionCompare$ LT2",
        );
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
            Some("replacement `Moved` with a condition other than `EQ0`")
        );
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
                "mana_ability!(Cost::TAP, &[Effect::mana(ManaColor::White, 1), Effect::mana(ManaColor::Blue, 1)])"
            ]
        );
        let either = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U");
        assert_eq!(
            either.abilities,
            [
                "mana_ability!(Cost::TAP, &[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])"
            ]
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
                "mana_ability!(Cost::TAP, &[Effect::mana_of_any_color().restricted(&SPEND1, SpendRider::None)])"
            ]
        );

        let script = parse(
            "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Hero,Activated.Hero",
        );
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
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
        for expected in [
            "layer: Layer::Type, filter: &Filter::This, modifier: Modifier::AddType(TypeSet::CREATURE)",
            "modifier: Modifier::AddSubtype(subtypes::creature::GOBLIN)",
            "layer: Layer::Color, filter: &Filter::This, modifier: Modifier::SetColor(ColorSet::from_slice(&[Color::Green]))",
            "layer: Layer::Ability, filter: &Filter::This, modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE)",
            "layer: Layer::PtSet, filter: &Filter::This, modifier: Modifier::SetPT(3, 3)",
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
            refusal_reason(&script, &cats()).as_deref(),
            Some("`Animate` of something other than the source")
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

    /// Forge's `Any` means creature, planeswalker, battle *or player*, and
    /// no `TargetSpec` spans objects and players. Read as `Filter::Any` it
    /// silently produced a burn spell that could not point at a player.
    /// Forge's `Any` means creature, planeswalker, battle *or player*, so
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
        assert!(a.contains("Layer::PtModify"), "7c, not 7b: {a}");
        assert!(a.contains("Modifier::ModifyPT(1, 0)"), "+1/+0: {a}");
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
        assert!(a.contains("Layer::PtModify") && a.contains("Modifier::ModifyPT(1, 1)"));
        assert!(a.contains("Layer::Ability") && a.contains("KeywordSet::FLYING"));
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
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +X | NumDef$ +X\n",
        );
        let a = body.abilities.join("");
        assert!(a.contains("power: Amount::X"), "{a}");
        assert!(a.contains("toughness: Amount::X"), "{a}");

        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n",
        );
        // The sign lives in the variant, not in a negated `X` — the engine
        // negates `NegX` at the use site and would double-negate otherwise.
        assert!(body.abilities.join("").contains("Amount::NegX"));
    }

    #[test]
    fn a_refusal_says_which_key_it_choked_on() {
        // The report is only a worklist if it names the thing to build; a
        // second table of each rule's keys would rot, so the transcoder
        // reports what it actually failed to claim.
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | UnlessCost$ 2");
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
            Some("unclaimed parameter `Draw.UnlessCost`")
        );

        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1");
        assert_eq!(refusal_reason(&script, &cats()), None, "read in full");

        // An unknown API is a missing effect, not a missing case in a rule
        // that exists, and is reported as its own kind. Leaving it silent
        // was worse than it looked: the report's fallback then guessed, and
        // named the first API *it* did not recognise — for a land whose
        // only unread line was `DB$ Discard`, that was the `R:Event$ Moved`
        // the transcoder had read perfectly well.
        let script = parse("Name:X\nTypes:Sorcery\nA:SP$ Animate | Defined$ Self");
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
            Some("effect `Animate`")
        );

        // A rule that exists but met a value it cannot say says so.
        let script = parse(
            "Name:X\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ 1 | Duration$ Permanent",
        );
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
            Some("unreadable value in `Pump`")
        );

        // A static ability names the mode it cannot read, so the worklist
        // ranks `ReduceCost` and `Continuous` as the different work they are.
        let script =
            parse("Name:X\nTypes:Creature\nS:Mode$ CantBlockBy | ValidAttacker$ Card.Self");
        assert_eq!(
            refusal_reason(&script, &cats()).as_deref(),
            Some("static ability `S: Mode$ CantBlockBy`")
        );
    }

    #[test]
    fn a_zone_change_is_read_as_the_pair_it_is() {
        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature\n",
        );
        assert!(body.abilities.join("").contains("Effect::ReturnToHand"));

        let body = read(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature\n",
        );
        assert!(body.abilities.join("").contains("Effect::Exile"));

        let body = read(
            "Name:X\nTypes:Creature\n\
             A:AB$ ChangeZone | Cost$ T | Origin$ Battlefield | Destination$ Exile | Defined$ Self\n",
        );
        assert!(body.abilities.join("").contains("Effect::ExileSource"));
    }

    #[test]
    fn putting_a_creature_in_a_graveyard_is_not_destroying_it() {
        // CR 700.4: destruction checks indestructible and a zone change does
        // not, so the nearest effect is the wrong effect — a card written
        // this way would quietly kill creatures that survive.
        assert!(refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Graveyard | ValidTgts$ Creature\n"
        ));
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
        // A keyword that is data rather than a bit.
        assert!(refused("Name:X\nTypes:Creature Goblin\nPT:1/1\nK:Equip:2"));
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

    #[test]
    fn the_supported_api_list_matches_what_is_actually_read() {
        for api in SUPPORTED_APIS {
            assert!(is_supported_api(api));
        }
        assert!(!is_supported_api("Animate"));
    }
}

/// A forge-reference checkout, resolved by card name.
///
/// Held by `codegen` so a stub can be transcoded from the rules reference
/// when one is available locally, and generated exactly as before when it is
/// not — the checkout is never part of the build.
#[derive(Debug)]
pub struct ForgeLookup {
    root: std::path::PathBuf,
    index: BTreeMap<String, String>,
}

impl ForgeLookup {
    /// Wraps a cardsfolder and the name → relative-path index `codegen`
    /// already builds.
    #[must_use]
    pub fn new(root: std::path::PathBuf, index: BTreeMap<String, String>) -> Self {
        Self { root, index }
    }

    /// The script for a card, by its Scryfall name.
    #[must_use]
    pub fn script(&self, name: &str) -> Option<ForgeScript> {
        // Multi-face Scryfall names ("A // B") are one Forge script, filed
        // under the front face.
        let key = self.index.get(name).or_else(|| {
            let front = name.split(" // ").next()?;
            self.index.get(front)
        })?;
        let text = std::fs::read_to_string(self.root.join(key)).ok()?;
        Some(parse(&text))
    }
}
