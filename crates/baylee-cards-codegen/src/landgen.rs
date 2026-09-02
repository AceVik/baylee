//! Printed land text → `CardDef` abilities.
//!
//! Lands are the one card type whose rules text is formulaic enough to read
//! mechanically: across every unique land Scryfall prints, a dozen sentence
//! shapes account for most of the corpus. This module turns those shapes into
//! the same Rust a hand-written card file would contain.
//!
//! # The rule that makes it safe
//!
//! [`recognize`] returns `Some` **only when every sentence of the oracle text
//! was consumed by a rule**. One unrecognised clause and the whole card falls
//! back to an ordinary `// GENERATED STUB` with
//! [`Coverage::Unimplemented`](baylee_cards_dsl::Coverage::Unimplemented).
//! There is deliberately no "close enough" path: a land that claims
//! `Implemented` while silently dropping half its text is worse than a stub,
//! because the deckbuilder would offer it as playable.
//!
//! Intrinsic mana comes from the type line, not from the text (CR 305.6) —
//! `Taiga` prints only reminder text, and its `{R} or {G}` is granted by
//! being a Mountain Forest. Reminder text in parentheses is therefore
//! stripped before matching rather than parsed.

use crate::body::CardBody;
use crate::catalog::SubtypeCatalogs;
use crate::scryfall::ScryfallCard;
use std::fmt::Write as _;

/// The five basic land types and the mana each grants (CR 305.6).
const BASIC_TYPES: [(&str, &str); 5] = [
    ("Plains", "ManaColor::White"),
    ("Island", "ManaColor::Blue"),
    ("Swamp", "ManaColor::Black"),
    ("Mountain", "ManaColor::Red"),
    ("Forest", "ManaColor::Green"),
];

fn symbol_color(sym: &str) -> Option<&'static str> {
    Some(match sym {
        "W" => "ManaColor::White",
        "U" => "ManaColor::Blue",
        "B" => "ManaColor::Black",
        "R" => "ManaColor::Red",
        "G" => "ManaColor::Green",
        "C" => "ManaColor::Colorless",
        _ => return None,
    })
}

/// `"{W}{U}"` → `["W", "U"]`; `None` if the string is not a clean symbol run.
fn symbols(text: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let inner = rest.strip_prefix('{')?;
        let end = inner.find('}')?;
        out.push(inner[..end].to_string());
        rest = &inner[end + 1..];
    }
    (!out.is_empty()).then_some(out)
}

/// Number words as they appear in rules text.
fn number(word: &str) -> Option<u32> {
    if let Ok(n) = word.parse::<u32>() {
        return Some(n);
    }
    Some(match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    })
}

/// Removes reminder text and normalises whitespace.
///
/// Reminder text is parenthesised by definition (CR 207.2), carries no rules
/// meaning, and would otherwise make every cycling land unparseable.
fn strip_reminders(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut depth = 0usize;
    for c in line.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `"Add {G} or {W}"` → the `Effect` expressions it produces.
fn parse_add(rest: &str) -> Option<Vec<String>> {
    let rest = rest.trim();
    if rest == "one mana of any color" {
        return Some(vec!["Effect::mana_of_any_color()".to_string()]);
    }
    // "{W}, {U}, or {B}" / "{G} or {W}" — a choice of one.
    if rest.contains(" or ") {
        let flat = rest.replace(", or ", ", ").replace(" or ", ", ");
        let mut colors = Vec::new();
        for alt in flat.split(", ") {
            let syms = symbols(alt.trim())?;
            if syms.len() != 1 {
                return None;
            }
            colors.push(symbol_color(&syms[0])?);
        }
        return Some(vec![format!(
            "Effect::mana_choice(&[{}])",
            colors.join(", ")
        )]);
    }
    // A plain run: "{C}", "{C}{C}", "{C}{U}".
    let syms = symbols(rest)?;
    let mut out = Vec::new();
    let mut i = 0;
    while i < syms.len() {
        let color = symbol_color(&syms[i])?;
        let run = syms[i..].iter().take_while(|s| **s == syms[i]).count();
        out.push(format!("Effect::mana({color}, {run})"));
        i += run;
    }
    Some(out)
}

/// One sentence of an ability's effect, for abilities that are not mana
/// abilities.
fn parse_effect(sentence: &str) -> Option<Vec<String>> {
    let s = sentence.trim().trim_end_matches('.');
    let lower = s.to_lowercase();
    if let Some(rest) = s.strip_prefix("Add ") {
        return parse_add(rest);
    }
    if let Some(rest) = lower.strip_prefix("draw ") {
        let n = number(rest.split_whitespace().next()?)?;
        return Some(vec![format!(
            "Effect::DrawCards {{ amount: Amount::Fixed({n}) }}"
        )]);
    }
    if let Some(rest) = lower.strip_prefix("scry ") {
        let n = number(rest.trim())?;
        return Some(vec![format!(
            "Effect::Scry {{ amount: Amount::Fixed({n}) }}"
        )]);
    }
    if let Some(rest) = lower.strip_prefix("you gain ") {
        let n = number(rest.split_whitespace().next()?)?;
        return (rest.ends_with(" life"))
            .then(|| vec![format!("Effect::GainLife {{ amount: Amount::Fixed({n}) }}")]);
    }
    if let Some(rest) = lower.strip_prefix("this land deals ") {
        let mut words = rest.split_whitespace();
        let n = number(words.next()?)?;
        if words.collect::<Vec<_>>().join(" ") == "damage to you" {
            return Some(vec![format!(
                "Effect::DealDamage {{ amount: Amount::Fixed({n}), target: TargetSpec::Player(PlayerRel::You) }}"
            )]);
        }
    }
    None
}

/// The activation cost left of the colon.
fn parse_cost(text: &str) -> Option<String> {
    let mut mana = String::new();
    let mut parts: Vec<String> = Vec::new();
    for token in text.split(", ") {
        let token = token.trim();
        if token == "{T}" {
            parts.push("CostPart::TapSelf".to_string());
        } else if token == "Sacrifice this land" {
            parts.push("CostPart::SacrificeSelf".to_string());
        } else if let Some(rest) = token.strip_prefix("Pay ")
            && let Some(n) = rest.strip_suffix(" life").and_then(number)
        {
            parts.push(format!("CostPart::PayLife({n})"));
        } else if token.starts_with('{') && symbols(token).is_some() {
            if !mana.is_empty() {
                return None;
            }
            mana = token.to_string();
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

/// Splits a line into sentences, keeping `{1}, {T}: …` colons intact.
fn sentences(line: &str) -> Vec<String> {
    line.split(". ")
        .map(|s| s.trim().trim_end_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

struct Recognizer<'a> {
    cats: &'a SubtypeCatalogs,
    body: CardBody,
    filter_count: usize,
}

impl Recognizer<'_> {
    /// `"a Swamp or a Mountain"` → a `Filter` static, returning its name.
    fn control_filter(&mut self, phrase: &str) -> Option<String> {
        let flat = phrase.replace(" or ", ", ");
        let mut clauses = Vec::new();
        for part in flat.split(", ") {
            let word = part
                .trim()
                .trim_start_matches("an ")
                .trim_start_matches("a ")
                .trim();
            let path = self.cats.const_path(word)?;
            clauses.push(format!("Filter::HasSubtype({path})"));
        }
        let inner = if clauses.len() == 1 {
            clauses.remove(0)
        } else {
            format!(
                "Filter::Or(&[\n        {},\n    ])",
                clauses.join(",\n        ")
            )
        };
        self.filter_count += 1;
        let name = if self.filter_count == 1 {
            "CHECK".to_string()
        } else {
            format!("CHECK{}", self.filter_count)
        };
        let _ = write!(
            self.body.statics,
            "static {name}: Filter = Filter::And(&[\n    Filter::ControlledByYou,\n    Filter::LAND,\n    {inner},\n]);\n\n"
        );
        Some(name)
    }

    /// An `{T}: Add …` style line, with any rider sentences that follow.
    fn activated_line(&mut self, line: &str) -> Option<()> {
        let (left, right) = line.split_once(": ")?;
        let cost = parse_cost(left)?;
        let mut effects = Vec::new();
        let mut is_mana = false;
        for (i, sentence) in sentences(right).iter().enumerate() {
            let parsed = parse_effect(sentence)?;
            if i == 0 {
                is_mana = sentence.starts_with("Add ");
            }
            effects.extend(parsed);
        }
        if effects.is_empty() {
            return None;
        }
        let effects_expr = format!("&[{}]", effects.join(", "));
        let macro_name = if is_mana {
            "mana_ability!"
        } else {
            "activated!"
        };
        self.body.abilities.push(if cost == "Cost::TAP" && is_mana {
            format!("{macro_name}({effects_expr})")
        } else {
            format!("{macro_name}({cost}, {effects_expr})")
        });
        Some(())
    }

    fn cycling_line(&mut self, line: &str) -> Option<()> {
        let cost = line.strip_prefix("Cycling ")?.trim();
        symbols(cost)?;
        self.body.abilities.push(format!(
            "activated!(Cost {{ mana: baylee_core::mana!(\"{cost}\"), parts: &[CostPart::DiscardSelf] }}, &[Effect::DrawCards {{ amount: Amount::Fixed(1) }}], zone: ActivationZone::Hand)"
        ));
        self.body.notes.push("cycling".to_string());
        Some(())
    }

    fn etb_trigger(&mut self, line: &str) -> Option<()> {
        let rest = line.strip_prefix("When this land enters, ")?;
        let effects = parse_effect(rest)?;
        self.body.abilities.push(format!(
            "triggered!(Trigger::EntersBattlefield(&Filter::This), &[{}])",
            effects.join(", ")
        ));
        Some(())
    }

    fn enters_line(&mut self, line: &str) -> Option<()> {
        if line == "This land enters tapped" {
            self.body
                .enter_modifiers
                .push("EnterModifier::Tapped".into());
            self.body.notes.push("enters tapped".to_string());
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("This land enters tapped unless you control ") {
            let name = self.control_filter(rest)?;
            self.body
                .enter_modifiers
                .push(format!("EnterModifier::TappedUnless(&{name})"));
            self.body.notes.push("checkland".to_string());
            return Some(());
        }
        if let Some(rest) = line.strip_prefix("As this land enters, you may pay ")
            && let Some(n) = rest
                .strip_suffix(" life. If you don't, it enters tapped")
                .and_then(number)
        {
            self.body
                .enter_modifiers
                .push(format!("EnterModifier::TappedOrPayLife({n})"));
            self.body.notes.push("shockland".to_string());
            return Some(());
        }
        None
    }

    fn line(&mut self, line: &str) -> Option<()> {
        match line {
            "Indestructible" => {
                self.body.keywords.push("KeywordSet::INDESTRUCTIBLE".into());
                return Some(());
            }
            "Hexproof" => {
                self.body.keywords.push("KeywordSet::HEXPROOF".into());
                return Some(());
            }
            _ => {}
        }
        // A land's enters-clause and its trigger are whole lines; an
        // activated ability is recognised by the colon that separates its
        // cost from its effect.
        self.enters_line(line)
            .or_else(|| self.etb_trigger(line))
            .or_else(|| self.cycling_line(line))
            .or_else(|| self.activated_line(line))
    }
}

/// Reads a land's printed text into a [`CardBody`], or `None` when any part
/// of it is not understood.
///
/// Multi-face cards, and lands that are also creatures or enchantments, are
/// refused outright — their text lives on faces this module does not model.
#[must_use]
pub fn recognize(card: &ScryfallCard, cats: &SubtypeCatalogs) -> Option<CardBody> {
    if card.card_faces.as_ref().is_some_and(|f| f.len() >= 2) {
        return None;
    }
    let type_line = card.type_line.as_deref()?;
    let (left, right) = match type_line.split_once('\u{2014}') {
        Some((l, r)) => (l, r),
        None => (type_line, ""),
    };
    let mut is_land = false;
    for word in left.split_whitespace() {
        match word {
            "Land" => is_land = true,
            "Artifact" | "Basic" | "Legendary" | "Snow" | "World" => {}
            _ => return None,
        }
    }
    if !is_land {
        return None;
    }

    let mut rec = Recognizer {
        cats,
        body: CardBody::default(),
        filter_count: 0,
    };

    // Intrinsic mana from the type line (CR 305.6) — the printed text only
    // ever restates it as reminder text.
    let basics: Vec<&str> = right
        .split_whitespace()
        .filter_map(|w| {
            BASIC_TYPES
                .iter()
                .find(|(name, _)| *name == w)
                .map(|(_, color)| *color)
        })
        .collect();
    match basics.len() {
        0 => {}
        1 => rec
            .body
            .abilities
            .push(format!("mana_ability!(&[Effect::mana({}, 1)])", basics[0])),
        _ => rec.body.abilities.push(format!(
            "mana_ability!(&[Effect::mana_choice(&[{}])])",
            basics.join(", ")
        )),
    }
    if !basics.is_empty() {
        rec.body.notes.push("intrinsic type mana".to_string());
    }

    for raw in card.oracle_text.as_deref().unwrap_or("").lines() {
        let line = strip_reminders(raw);
        let line = line.trim().trim_end_matches('.');
        if line.is_empty() {
            continue;
        }
        rec.line(line)?;
    }

    rec.body
        .notes
        .insert(0, "read from the printed text".to_string());
    if rec.body.abilities.is_empty()
        && rec.body.enter_modifiers.is_empty()
        && rec.body.keywords.is_empty()
    {
        return None;
    }
    Some(rec.body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cats() -> SubtypeCatalogs {
        let mut c = SubtypeCatalogs {
            land: vec![
                "Plains".into(),
                "Island".into(),
                "Swamp".into(),
                "Mountain".into(),
                "Forest".into(),
                "Cave".into(),
            ],
            ..SubtypeCatalogs::default()
        };
        c.normalize();
        c
    }

    fn card(type_line: &str, oracle: &str) -> ScryfallCard {
        ScryfallCard {
            id: "id".into(),
            oracle_id: Some("oracle".into()),
            name: "Test Land".into(),
            mana_cost: None,
            type_line: Some(type_line.into()),
            oracle_text: Some(oracle.into()),
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
        }
    }

    fn read(type_line: &str, oracle: &str) -> CardBody {
        recognize(&card(type_line, oracle), &cats()).expect("should be recognised")
    }

    /// Taiga prints nothing but reminder text: its mana comes from being a
    /// Mountain Forest (CR 305.6), which is the whole reason intrinsic mana
    /// is read off the type line rather than out of the text.
    #[test]
    fn a_dual_lands_mana_comes_from_its_type_line_not_its_reminder_text() {
        let body = read("Land \u{2014} Mountain Forest", "({T}: Add {R} or {G}.)");
        assert_eq!(
            body.abilities,
            ["mana_ability!(&[Effect::mana_choice(&[ManaColor::Red, ManaColor::Green])])"]
        );
        assert!(body.enter_modifiers.is_empty());
    }

    #[test]
    fn a_basic_land_produces_one_color() {
        let body = read("Basic Land \u{2014} Mountain", "({T}: Add {R}.)");
        assert_eq!(
            body.abilities,
            ["mana_ability!(&[Effect::mana(ManaColor::Red, 1)])"]
        );
    }

    #[test]
    fn a_shockland_pays_life_instead_of_entering_tapped() {
        let body = read(
            "Land \u{2014} Forest Plains",
            "({T}: Add {G} or {W}.)\nAs this land enters, you may pay 2 life. If you don't, it enters tapped.",
        );
        assert_eq!(body.enter_modifiers, ["EnterModifier::TappedOrPayLife(2)"]);
    }

    #[test]
    fn a_checkland_builds_the_filter_it_checks() {
        let body = read(
            "Land",
            "This land enters tapped unless you control an Island or a Swamp.\n{T}: Add {U} or {B}.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnless(&CHECK)"]
        );
        assert!(
            body.statics
                .contains("Filter::HasSubtype(subtypes::land::ISLAND)")
        );
        assert!(
            body.statics
                .contains("Filter::HasSubtype(subtypes::land::SWAMP)")
        );
        assert_eq!(
            body.abilities,
            ["mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])])"]
        );
    }

    /// A painland's damage is part of the mana ability, not a separate
    /// trigger — the rider sentence has to attach to the ability above it.
    #[test]
    fn a_painlands_damage_rides_along_with_its_mana() {
        let body = read(
            "Land",
            "{T}: Add {C}.\n{T}: Add {W} or {U}. This land deals 1 damage to you.",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])",
                "mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue]), Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::Player(PlayerRel::You) }])",
            ]
        );
    }

    #[test]
    fn a_triome_enters_tapped_and_cycles_from_the_hand() {
        let body = read(
            "Land \u{2014} Plains Swamp Forest",
            "({T}: Add {W}, {B}, or {G}.)\nThis land enters tapped.\nCycling {2} ({2}, Discard this card: Draw a card.)",
        );
        assert_eq!(body.enter_modifiers, ["EnterModifier::Tapped"]);
        assert_eq!(body.abilities.len(), 2);
        assert!(body.abilities[0].contains("mana_choice"));
        assert!(body.abilities[1].contains("zone: ActivationZone::Hand"));
    }

    #[test]
    fn a_sacrifice_ability_keeps_its_mana_and_its_parts() {
        let body = read(
            "Land",
            "{T}: Add {C}.\n{1}, {T}, Sacrifice this land: Draw a card.",
        );
        assert_eq!(
            body.abilities[1],
            "activated!(Cost { mana: baylee_core::mana!(\"{1}\"), parts: &[CostPart::TapSelf, CostPart::SacrificeSelf] }, &[Effect::DrawCards { amount: Amount::Fixed(1) }])"
        );
    }

    #[test]
    fn an_enters_trigger_gains_life() {
        let body = read(
            "Land",
            "This land enters tapped.\nWhen this land enters, you gain 1 life.\n{T}: Add {W} or {B}.",
        );
        assert!(
            body.abilities
                .iter()
                .any(|a| a.contains("Trigger::EntersBattlefield") && a.contains("GainLife"))
        );
    }

    #[test]
    fn a_keyword_line_becomes_a_keyword() {
        let body = read("Land", "Indestructible\n{T}: Add {C}.");
        assert_eq!(body.keywords, ["KeywordSet::INDESTRUCTIBLE"]);
    }

    /// The load-bearing rule: one clause the reader does not understand and
    /// the whole card is refused, so it stays an honest stub rather than
    /// claiming to be playable with half its text dropped.
    #[test]
    fn one_unread_clause_refuses_the_whole_card() {
        assert!(
            recognize(
                &card(
                    "Land",
                    "{T}: Add {C}.\nWhenever a Cleric enters, you may untap this land.",
                ),
                &cats()
            )
            .is_none()
        );
        // …including a condition on an otherwise familiar enters-clause.
        assert!(
            recognize(
                &card("Land", "This land enters tapped unless you control two or more basic lands.\n{T}: Add {G}."),
                &cats(),
            )
            .is_none()
        );
    }

    /// Nonland cards, and lands that are also creatures, are not this
    /// module's business — their text lives somewhere it does not look.
    #[test]
    fn only_plain_lands_are_read() {
        assert!(
            recognize(
                &card("Creature \u{2014} Elemental", "{T}: Add {G}."),
                &cats()
            )
            .is_none()
        );
        assert!(
            recognize(
                &card("Land Creature \u{2014} Forest Dryad", "({T}: Add {G}.)"),
                &cats()
            )
            .is_none()
        );
    }
}
