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
use baylee_core::types::SubtypeKind;
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
        // The tail has to be read, not skipped. "Draw a card if you control
        // an artifact." parsed as "draw 1" once, and the card that came out
        // drew unconditionally — a condition dropped is a rule invented, and
        // the deckbuilder offers an `Implemented` card as playable.
        let mut words = rest.split_whitespace();
        let n = number(words.next()?)?;
        let tail = words.collect::<Vec<_>>().join(" ");
        if tail != "card" && tail != "cards" {
            return None;
        }
        return Some(vec![format!("Effect::draw({n})")]);
    }
    if let Some(rest) = lower.strip_prefix("scry ") {
        let n = number(rest.trim())?;
        return Some(vec![format!("Effect::scry({n})")]);
    }
    if let Some(rest) = lower.strip_prefix("you gain ") {
        let n = number(rest.split_whitespace().next()?)?;
        return (rest.ends_with(" life")).then(|| vec![format!("Effect::gain_life({n})")]);
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
            parts.push("TapSelf".to_string());
        } else if token == "Sacrifice this land" {
            parts.push("SacrificeSelf".to_string());
        } else if let Some(rest) = token.strip_prefix("Pay ")
            && let Some(n) = rest.strip_suffix(" life").and_then(number)
        {
            parts.push(format!("PayLife({n})"));
        } else if token.starts_with('{') && symbols(token).is_some() {
            if !mana.is_empty() {
                return None;
            }
            mana = token.to_string();
        } else {
            return None;
        }
    }
    Some(crate::body::cost_literal(&mana, &parts))
}

/// Splits a line into sentences, keeping `{1}, {T}: …` colons intact.
fn sentences(line: &str) -> Vec<String> {
    line.split(". ")
        .map(|s| s.trim().trim_end_matches('.').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The counting enters-clauses: "unless you control N or more …", by the
/// phrase that finishes the sentence, the filter each one of them counts and
/// the note it leaves behind.
///
/// The word before "lands" is the entire difference between them and it is
/// not a modifier a reader may drop: a battle land counts *basic* lands and a
/// slow land counts *other* lands. "Other" costs nothing extra here because
/// the entering land never counts itself anyway. Two phrases this table does
/// **not** carry, and deliberately: "two or fewer other lands" is the fast
/// lands and is the opposite comparison, and "three or more other Islands"
/// is a cycle whose second sentence the reader cannot read regardless.
///
/// Both filters are a name the DSL already carries, so neither is hoisted:
/// a `static CHECK: Filter = Filter::YOUR_LAND;` would give one spelling of
/// "a land you control" a second, card-local one on each of twenty lands.
const COUNTED: [(&str, &str, &str); 2] = [
    (
        " or more basic lands",
        "Filter::YOUR_BASIC_LAND",
        "battle land",
    ),
    (" or more other lands", "Filter::YOUR_LAND", "slow land"),
];

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
            // A *land* subtype, named as one. The filter below says
            // `Filter::LAND` beside this clause, so a word that resolved to
            // the creature catalog — which `const_path` searches first —
            // would build a check nothing can satisfy, and the land would
            // enter tapped forever while claiming to be a checkland.
            let path = self.cats.const_path_of(SubtypeKind::Land, word)?;
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
        // Noun first, like every other filter the DSL spells — and flat, not
        // `[Filter::YOUR_LAND, {inner}]`: nesting is a different shape and
        // not a reordering, which is more than an equivalence proof over
        // clause order can vouch for.
        Some(self.named_filter(&format!(
            "Filter::And(&[\n    Filter::LAND,\n    Filter::ControlledByYou,\n    {inner},\n])"
        )))
    }

    /// Hoists a filter expression into a `static` and answers with its name.
    ///
    /// Not because it has to be: a `&Filter::And(&[…])` inside a `static`
    /// promotes to `&'static` and compiles, which is how Urza's Saga writes
    /// one by hand. It is hoisted because a land's check filter is three
    /// clauses deep and reads as a sentence of its own above the card rather
    /// than as a parenthesis inside an `EnterModifier`. The numbering is
    /// shared so that two clauses on one card cannot both be called `CHECK`.
    fn named_filter(&mut self, expr: &str) -> String {
        self.filter_count += 1;
        let name = if self.filter_count == 1 {
            "CHECK".to_string()
        } else {
            format!("CHECK{}", self.filter_count)
        };
        let _ = write!(self.body.statics, "static {name}: Filter = {expr};\n\n");
        name
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
            "activated!(cost!(\"{cost}\", DiscardSelf), &[Effect::draw(1)], zone = ActivationZone::Hand)"
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
        // Before the checkland below it, which claims the same prefix and
        // would refuse the whole card on the phrases these read.
        if let Some(rest) = line.strip_prefix("This land enters tapped unless you control ") {
            for (phrase, filter, note) in COUNTED {
                let Some(n) = rest
                    .strip_suffix(phrase)
                    .and_then(number)
                    .and_then(|n| u8::try_from(n).ok())
                else {
                    continue;
                };
                self.body.enter_modifiers.push(format!(
                    "EnterModifier::TappedUnlessCount {{ filter: &{filter}, at_least: {n} }}"
                ));
                self.body.notes.push(note.to_string());
                return Some(());
            }
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

/// Why a land's printed text could not be read.
///
/// A reader that answers `None` is a reader whose limits can only be guessed
/// at, and guessing is exactly what made `transcode-report`'s ranking wrong: it
/// re-read the script and named the first thing it did not recognise, which
/// is not the same as the thing that actually stopped it. So this says what
/// stopped it, and `land-report` groups by that rather than by a theory.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LandRefusal {
    /// Two faces, or a type line with a word other than `Land` and its
    /// permitted modifiers. Their text lives on faces this module does not
    /// model.
    NotAPlainLand,
    /// Read through without a word left over, and with nothing to show for
    /// it. A land whose whole text is reminder text is not a land this
    /// reader can claim to have implemented.
    NothingToSay,
    /// The first printed line that stopped it, reminder text already
    /// stripped. This is the one worth counting: each distinct shape here is
    /// a sentence form the reader cannot parse, and one of them unlocked is
    /// every land that prints it.
    UnreadLine(String),
}

/// Reads a land's printed text into a [`CardBody`], or `None` when any part
/// of it is not understood.
///
/// The thin wrapper over [`read`], for callers that only need to know whether
/// a card was claimed. `codegen` is one: a refusal is a stub either way.
#[must_use]
pub fn recognize(card: &ScryfallCard, cats: &SubtypeCatalogs) -> Option<CardBody> {
    read(card, cats).ok()
}

/// Reads a land's printed text, and says why when it cannot.
///
/// Multi-face cards, and lands that are also creatures or enchantments, are
/// refused outright — their text lives on faces this module does not model.
///
/// # Errors
///
/// [`LandRefusal`], naming what stopped the read.
pub fn read(card: &ScryfallCard, cats: &SubtypeCatalogs) -> Result<CardBody, LandRefusal> {
    if card.card_faces.as_ref().is_some_and(|f| f.len() >= 2) {
        return Err(LandRefusal::NotAPlainLand);
    }
    let type_line = card
        .type_line
        .as_deref()
        .ok_or(LandRefusal::NotAPlainLand)?;
    let (left, right) = match type_line.split_once('\u{2014}') {
        Some((l, r)) => (l, r),
        None => (type_line, ""),
    };
    let mut is_land = false;
    for word in left.split_whitespace() {
        match word {
            "Land" => is_land = true,
            "Artifact" | "Basic" | "Legendary" | "Snow" | "World" => {}
            _ => return Err(LandRefusal::NotAPlainLand),
        }
    }
    if !is_land {
        return Err(LandRefusal::NotAPlainLand);
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
        rec.line(line)
            .ok_or_else(|| LandRefusal::UnreadLine(line.to_string()))?;
    }

    rec.body
        .notes
        .insert(0, "read from the printed text".to_string());
    if rec.body.abilities.is_empty()
        && rec.body.enter_modifiers.is_empty()
        && rec.body.keywords.is_empty()
    {
        return Err(LandRefusal::NothingToSay);
    }
    Ok(rec.body)
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
            // Not decoration: `const_path` searches creature first, so a
            // catalog with no creatures in it cannot show a land reader
            // reaching into the wrong one.
            creature: vec!["Cleric".into(), "Zombie".into()],
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

    /// A checkland checks a *land* subtype, and a sentence naming any other
    /// kind is a sentence this reader has not understood.
    ///
    /// The filter it builds says `Filter::LAND` beside the subtype clause, so
    /// a name resolved out of the creature catalog — which `const_path`
    /// reaches first, the catalogs being searched in id order — would compile
    /// to a check nothing on a battlefield can ever satisfy. The card would
    /// be generated `Implemented` and enter tapped for the rest of its life,
    /// which is precisely the "close enough" path
    /// [`one_unread_clause_refuses_the_whole_card`] exists to keep shut.
    #[test]
    fn a_checkland_that_names_something_other_than_a_land_type_is_refused() {
        assert!(
            recognize(
                &card(
                    "Land",
                    "This land enters tapped unless you control a Zombie.\n{T}: Add {B}.",
                ),
                &cats(),
            )
            .is_none()
        );
        // The land type of the same shape is still read, so the refusal is
        // about the kind and not about the sentence.
        let body = read(
            "Land",
            "This land enters tapped unless you control a Cave.\n{T}: Add {B}.",
        );
        assert!(
            body.statics
                .contains("Filter::HasSubtype(subtypes::land::CAVE)")
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
        assert!(body.abilities[1].contains("zone = ActivationZone::Hand"));
    }

    #[test]
    fn a_sacrifice_ability_keeps_its_mana_and_its_parts() {
        let body = read(
            "Land",
            "{T}: Add {C}.\n{1}, {T}, Sacrifice this land: Draw a card.",
        );
        assert_eq!(
            body.abilities[1],
            "activated!(cost!(\"{1}\", TapSelf, SacrificeSelf), &[Effect::draw(1)])"
        );
    }

    #[test]
    fn an_enters_trigger_gains_life() {
        let body = read(
            "Land",
            "This land enters tapped.\nWhen this land enters, you gain 1 life.\n{T}: Add {W} or {B}.",
        );
        assert!(body.abilities.iter().any(
            |a| a.contains("Trigger::EntersBattlefield") && a.contains("Effect::gain_life(1)")
        ));
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
        // …including a count the table beside it does not carry. A fast land
        // is one word from a slow land and the opposite comparison, so the
        // reader that learned "two or more" must not answer "two or fewer".
        assert!(
            recognize(
                &card("Land", "This land enters tapped unless you control two or fewer other lands.\n{T}: Add {G}."),
                &cats(),
            )
            .is_none()
        );
        // …and a count that *is* read still refuses the card when the
        // sentence after it is not. The Eldraine cycle's second clause is a
        // trigger on entering **untapped**, which is a condition no rule
        // here says, so those five stay stubs on their own merits.
        assert!(
            recognize(
                &card(
                    "Land \u{2014} Plains",
                    "This land enters tapped unless you control three or more other Plains.\nWhen this land enters untapped, put a +1/+1 counter on target creature you control.",
                ),
                &cats(),
            )
            .is_none()
        );
    }

    /// The slow lands: the same counting clause as a battle land, over
    /// *other* lands — which costs nothing, because the entering land never
    /// counts itself.
    #[test]
    fn a_slow_land_counts_the_other_lands() {
        let body = read(
            "Land",
            "This land enters tapped unless you control two or more other lands.\n{T}: Add {W} or {U}.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnlessCount { filter: &Filter::YOUR_LAND, at_least: 2 }"]
        );
        assert!(body.statics.is_empty(), "{}", body.statics);
    }

    /// The battle lands: a condition that **counts**, and counts *basic*
    /// lands — neither of which a checkland's sentence says.
    #[test]
    fn a_battle_land_counts_basic_lands() {
        let body = read(
            "Land \u{2014} Island Swamp",
            "({T}: Add {U} or {B}.)\nThis land enters tapped unless you control two or more basic lands.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnlessCount { filter: &Filter::YOUR_BASIC_LAND, at_least: 2 }"]
        );
        assert!(body.statics.is_empty(), "{}", body.statics);
        assert_eq!(
            body.abilities,
            ["mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Black])])"]
        );
    }

    /// Roadside Reliquary shipped as an `Implemented` land that drew two
    /// cards for nothing: the draw rule read the number and threw the rest of
    /// the sentence away, so both "if you control" clauses vanished. A tail
    /// the rule cannot say is a refusal, not a silently stronger card.
    #[test]
    fn a_draw_with_a_condition_on_it_is_refused_rather_than_granted() {
        assert!(
            recognize(
                &card(
                    "Land",
                    "{T}: Add {C}.\n{2}, {T}, Sacrifice this land: Draw a card if you control an artifact. Draw a card if you control an enchantment.",
                ),
                &cats(),
            )
            .is_none()
        );
        // The unconditional sentence still reads.
        let body = read(
            "Land",
            "{T}: Add {C}.\n{1}, {T}, Sacrifice this land: Draw a card.",
        );
        assert!(body.abilities[1].contains("Effect::draw(1)"));
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
