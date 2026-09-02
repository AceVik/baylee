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
    /// A Forge valid-string (`Creature.YouCtrl+nonToken`) as a `Filter`.
    fn filter_expr(&self, valid: &str) -> Option<String> {
        let mut alternatives = Vec::new();
        for alt in valid.split(',') {
            let mut clauses = Vec::new();
            let mut atoms = alt.split('.');
            let base = atoms.next()?.trim();
            match base {
                "Card" | "Permanent" | "Any" => {}
                "Creature" => clauses.push("Filter::CREATURE".to_string()),
                "Artifact" => clauses.push("Filter::ARTIFACT".to_string()),
                "Enchantment" => clauses.push("Filter::ENCHANTMENT".to_string()),
                "Land" => clauses.push("Filter::LAND".to_string()),
                "Planeswalker" => clauses.push("Filter::PLANESWALKER".to_string()),
                "Instant" => clauses.push("Filter::HasType(TypeSet::INSTANT)".to_string()),
                "Sorcery" => clauses.push("Filter::HasType(TypeSet::SORCERY)".to_string()),
                _ => {
                    let path = self.cats.const_path(base)?;
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
                    "" => continue,
                    _ => return None,
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
    fn target_spec(&mut self, valid: &str) -> Option<String> {
        if valid == "Player" || valid == "Opponent" {
            return Some("TargetSpec::AnyPlayer".to_string());
        }
        let expr = self.filter_expr(valid)?;
        let name = self.body.filter_static("TARGET", &expr);
        Some(format!("TargetSpec::Object(&{name})"))
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

    /// One effect and everything its `SubAbility$` chain adds.
    fn chain(&mut self, spec: &str, chain: &mut Chain) -> Option<()> {
        let (api, mut p) = Params::parse(spec)?;
        p.drop_prose();
        if let Some(valid) = p.take("ValidTgts") {
            let spec = self.target_spec(&valid)?;
            if chain.target.get_or_insert(spec.clone()) != &spec {
                return None; // two different targets in one chain
            }
        }
        let sub = p.take("SubAbility");
        let target = chain
            .target
            .clone()
            .unwrap_or_else(|| "TargetSpec::AnyPlayer".to_string());

        let effects = self.effect_of(&api, &mut p, &target)?;
        if !p.exhausted() {
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
    fn effect_of(&mut self, api: &str, p: &mut Params, target: &str) -> Option<Vec<String>> {
        Some(match api {
            "DealDamage" => {
                let n = amount(&p.take("NumDmg")?, self.svars)?;
                let to = match p.take("Defined").as_deref() {
                    None => target.to_string(),
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
                match Self::player_rel(p.take("Defined").as_deref())? {
                    "PlayerRel::You" => vec![format!("Effect::GainLife {{ amount: {n} }}")],
                    who => vec![format!("Effect::GainLifeFor {{ amount: {n}, who: {who} }}")],
                }
            }
            "LoseLife" => {
                let n = amount(&p.take("LifeAmount")?, self.svars)?;
                let who = Self::player_rel(p.take("Defined").as_deref())?;
                vec![format!("Effect::LoseLife {{ amount: {n}, target: {who} }}")]
            }
            "Draw" => {
                let n = amount(p.take("NumCards").as_deref().unwrap_or("1"), self.svars)?;
                match Self::player_rel(p.take("Defined").as_deref())? {
                    "PlayerRel::You" => vec![format!("Effect::DrawCards {{ amount: {n} }}")],
                    who => vec![format!(
                        "Effect::DrawCardsFor {{ amount: {n}, who: {who} }}"
                    )],
                }
            }
            "Mill" => {
                let n = amount(&p.take("NumCards")?, self.svars)?;
                let who = Self::player_rel(p.take("Defined").as_deref())?;
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
                // "can't be regenerated" is not modelled; a card that says so
                // must not be silently generated without it.
                if p.take("NoRegen").is_some_and(|v| v != "True") {
                    return None;
                }
                vec![format!("Effect::Destroy {{ target: {target} }}")]
            }
            "Pump" => self.pump_effect(p, target)?,
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

    /// `Produced$ Combo W U | Amount$ 2` and friends.
    fn mana_effect(&mut self, p: &mut Params) -> Option<Vec<String>> {
        let produced = p.take("Produced")?;
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
        if produced == "Any" {
            return (amount == 1).then(|| vec!["Effect::mana_of_any_color()".to_string()]);
        }
        if let Some(list) = produced.strip_prefix("Combo ") {
            let colors: Option<Vec<&str>> = list.split_whitespace().map(color).collect();
            let colors = colors?;
            return (amount == 1)
                .then(|| vec![format!("Effect::mana_choice(&[{}])", colors.join(", "))]);
        }
        let c = color(&produced)?;
        Some(vec![format!("Effect::mana({c}, {amount})")])
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
            _ => None, // S: and R: are not modelled yet.
        }
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
            "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\n\
             A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ deals 3 damage.\n\
             Oracle:Lightning Bolt deals 3 damage to any target.",
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

    /// of these would otherwise generate a card missing half its rules.
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
            "Name:X\nTypes:Creature Goblin\nPT:1/1\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1"
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
