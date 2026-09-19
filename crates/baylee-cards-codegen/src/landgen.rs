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

/// The mana ability a land's **type line** gives it (CR 305.6).
///
/// Both readers need this and neither corpus contains it. A printed card
/// restates intrinsic mana only as reminder text — Taiga's whole rules box is
/// `({T}: Add {R} or {G}.)` in italics — and a reference script leaves it out
/// for the same reason the transcoder ignores names and costs: the type line
/// already says it. So a land that reaches the script reader without this
/// would be written with no way to tap for mana at all, which is invisible
/// with one basic type (`casting::intrinsic_mana` covers it) and fatal with
/// two, where that shortcut deliberately returns `None` rather than guess
/// which colour the player wanted.
pub(crate) fn intrinsic_mana_ability(type_line: &str) -> Option<String> {
    let (left, right) = type_line.split_once('\u{2014}')?;
    // CR 305.6 is about *lands*, and this is asked of every card the script
    // reader writes rather than only of the ones [`recognize`] admitted. The
    // five words are subtypes and nothing else prints them today, but a rule
    // that reads a subtype list without asking what the card is would hand a
    // creature a mana ability the day one is printed with a land type.
    if !left.split_whitespace().any(|w| w == "Land") {
        return None;
    }
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
        0 => None,
        1 => Some(format!("mana_ability!(&[Effect::mana({}, 1)])", basics[0])),
        _ => Some(format!(
            "mana_ability!(&[Effect::mana_choice(&[{}])])",
            basics.join(", ")
        )),
    }
}

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

/// The counters a land prints, and how a card file spells each one.
///
/// Four words, and the pool decides which four. `charge` is a
/// [`CounterKind`] variant because the rules know the word; `depletion`,
/// `mining` and `storage` are ids the DSL's `counters` module assigns,
/// because nothing in the rules has ever heard of them — the card that
/// prints one says what happens when it runs out and that is all they are.
///
/// What is **not** here is the point of having a table. `verse` is printed
/// by a land in this pool and has no id, so the card that prints it refuses
/// and stays a stub; a reader that took the number and threw the noun away
/// would give a land a counter its own text never mentions.
///
/// [`CounterKind`]: baylee_cards_dsl::CounterKind
const COUNTERS: [(&str, &str); 4] = [
    ("charge", "CounterKind::Charge"),
    ("depletion", "counters::DEPLETION"),
    ("mining", "counters::MINING"),
    ("storage", "counters::STORAGE"),
];

/// `"depletion"` → `"counters::DEPLETION"`; a counter with no id → `None`.
fn counter_kind(noun: &str) -> Option<&'static str> {
    COUNTERS
        .iter()
        .find_map(|(word, spelling)| (*word == noun).then_some(*spelling))
}

/// `"two depletion counters"` → `("counters::DEPLETION", 2)`.
///
/// The plural has to agree, for the reason the noun is read at all: a
/// mismatch is a phrase this did not actually understand, and the pool
/// prints none.
fn counter_phrase(phrase: &str) -> Option<(&'static str, u16)> {
    let (count, rest) = phrase.split_once(' ')?;
    let n = number(count)?;
    let (noun, tail) = rest.split_once(' ')?;
    if tail != if n == 1 { "counter" } else { "counters" } {
        return None;
    }
    Some((counter_kind(noun)?, u16::try_from(n).ok()?))
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
///
/// `announced` is the counter the cost beside this asked for a *number* of,
/// which one of these sentences needs: "Add X mana …" reads that number back
/// as `Amount::X`, and with nothing announced `Amount::X` evaluates to nought
/// — a land that taps for nothing while its file claims otherwise.
fn parse_add(rest: &str, announced: Option<&'static str>) -> Option<Vec<String>> {
    let rest = rest.trim();
    if rest == "one mana of any color" {
        return Some(vec!["Effect::mana_of_any_color()".to_string()]);
    }
    // "X mana in any combination of {W} and/or {U}" (the five Time Spiral
    // storage lands) and "five mana in any combination of colors"
    // (Cascading Cataracts, Great Hall of the Citadel, Baxter Building).
    //
    // **"In any combination" is a pick per mana**, which is the whole of
    // what distinguishes this from `mana_choice_dynamic`: Harabaz Druid's
    // "add X mana of any one color" asks once for the whole amount, and a
    // card read with the wrong one of the two lets a player make {W}{U}{B}
    // where the card says three of one colour, or the reverse. The engine
    // has carried the difference since Mystic Gate — `combination: true`
    // splits the amount into `n` picks of one in `resolve::mana::add_mana`
    // — so this rule adds a reading and no rule at all.
    //
    // Nine cards in the pool print the phrase and this reads **all nine
    // sentences** — measured one at a time, not assumed. Six of the cards
    // come out; the other three refuse on a clause that is not this one
    // ("Spend this mana only to …" twice, an `Activate only if` condition
    // once), which is the honesty rule holding a card back over a sentence
    // this reader never claimed.
    if let Some((head, tail)) = rest.split_once(" mana in any combination of ") {
        let amount = if head == "X" {
            // The cost has to have asked for a number. Nothing in the pool
            // prints "Add X mana" beside a cost that announces none, and a
            // card that did would tap for nought — so it refuses instead.
            announced?;
            "Amount::X".to_string()
        } else {
            format!("Amount::Fixed({})", number(head)?)
        };
        let colors = if tail == "colors" {
            "ALL_MANA_COLORS".to_string()
        } else {
            // "{W} and/or {U}". A three-colour spelling would be
            // "{W}, {U}, and/or {B}", which this split leaves with a comma
            // in the first half and `symbols` then refuses — no card in the
            // pool prints one, and inventing the reading would be a filter
            // nobody wrote.
            let mut out = Vec::new();
            for alt in tail.split(" and/or ") {
                let syms = symbols(alt.trim())?;
                let [only] = syms.as_slice() else {
                    return None;
                };
                out.push(symbol_color(only)?);
            }
            format!("&[{}]", out.join(", "))
        };
        return Some(vec![format!(
            "Effect::mana_combination({colors}, {amount})"
        )]);
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
fn parse_effect(sentence: &str, announced: Option<&'static str>) -> Option<Vec<String>> {
    let s = sentence.trim().trim_end_matches('.');
    let lower = s.to_lowercase();
    // "Add {W} for each storage counter removed this way" — the other half
    // of a storage land, and the only sentence in this reader that depends
    // on the *cost* beside it. `announced` is the counter whose number the
    // cost asked for, and the two nouns have to be the same one: a card
    // adding mana for each charge counter while its cost removed storage
    // counters would be a sentence this did not read, whoever printed it.
    //
    // Above the plain `Add ` arm, which claims the same prefix and would
    // hand `parse_add` a phrase with prose in it.
    if let Some(rest) = s.strip_prefix("Add ")
        && let Some((symbol, tail)) = rest.split_once(" for each ")
        && let Some(noun) = tail.strip_suffix(" counter removed this way")
    {
        let syms = symbols(symbol)?;
        let [only] = syms.as_slice() else {
            return None;
        };
        let color = symbol_color(only)?;
        if counter_kind(noun)? != announced? {
            return None;
        }
        // `Amount::X` and not a count of what is on the land: the counters
        // are gone by the time this runs, and the number the player
        // announced is what the cost took off.
        return Some(vec![format!("Effect::mana_dynamic({color}, Amount::X)")]);
    }
    if let Some(rest) = s.strip_prefix("Add ") {
        return parse_add(rest, announced);
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
    if let Some(rest) = s.strip_prefix("Put ")
        && let Some(phrase) = rest.strip_suffix(" on this land")
        && let Some((kind, n)) = counter_phrase(phrase)
    {
        // No target, which is what `Effect::AddCounter` reads as "the source"
        // — Mirrodin's Core's `{T}: Put a charge counter on this land`, the
        // half that fills the land the removal above empties.
        return Some(vec![format!(
            "Effect::AddCounter {{ kind: {kind}, amount: Amount::Fixed({n}) }}"
        )]);
    }
    // "If there are no depletion counters on this land, sacrifice it." —
    // the clause that finishes the depletion lands' and Gemstone Mine's one
    // ability, and an effect in the same list as the mana rather than a
    // trigger, because that is how the card prints it. The counter kind is
    // read here too: "if there are no storage counters" is a card this
    // cannot write, and reading it as depletion would sacrifice a land that
    // is meant to keep filling up.
    if let Some(rest) = s.strip_prefix("If there are no ")
        && let Some(noun) = rest
            .strip_suffix(" counters on this land, sacrifice it")
            .and_then(counter_kind)
    {
        return Some(vec![format!(
            "Effect::IfNoCountersOnSelf {{ kind: {noun}, then: &[Effect::SacrificeSelf] }}"
        )]);
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
fn parse_cost(text: &str) -> Option<(String, Option<&'static str>)> {
    let mut mana = String::new();
    let mut parts: Vec<String> = Vec::new();
    let mut announced: Option<&'static str> = None;
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
        } else if token == "Tap an untapped creature you control" {
            // The convoke land's other half. One printed phrase and no noun
            // parsing at all, which is the honesty rule rather than laziness:
            // the same sentence names a Gate on Heap Gate and a *legendary*
            // creature on Dungeon Descent, and a reader that guessed at the
            // noun would emit a filter nobody wrote. Those refuse here and
            // stay stubs until the phrase is read for real.
            parts.push("TapOther(&Filter::YOUR_CREATURE)".to_string());
        } else if let Some(rest) = token.strip_prefix("Remove ")
            && let Some(phrase) = rest.strip_suffix(" from this land")
            && let Some((kind, n)) = counter_phrase(phrase)
        {
            // A counter paid as a cost, which is not the door the counters a
            // land *enters* with take: CR 614.16 doubles what is put on a
            // permanent and Magic prints nothing that multiplies a removal,
            // so this one is arithmetic and that one is a replacement effect.
            parts.push(format!("RemoveCounterSelf {{ kind: {kind}, n: {n} }}"));
        } else if let Some(kind) = token
            .strip_prefix("Remove any number of ")
            .or_else(|| token.strip_prefix("Remove X "))
            .and_then(|rest| rest.strip_suffix(" counters from this land"))
            .and_then(counter_kind)
        {
            // Two printed spellings, one rule: the storage lands say "any
            // number of" eleven times and "X" six, and both mean a number
            // the player announces as the ability is activated. Read into
            // one `CostPart` because the effect reads both back the same
            // way, as `Amount::X`.
            parts.push(format!("RemoveCounterSelfX {{ kind: {kind} }}"));
            announced = Some(kind);
        } else if token.starts_with('{') && symbols(token).is_some() {
            if !mana.is_empty() {
                return None;
            }
            mana = token.to_string();
        } else {
            return None;
        }
    }
    Some((crate::body::cost_literal(&mana, &parts), announced))
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
/// the entering land never counts itself anyway. One phrase this table does
/// **not** carry: "three or more other Islands" is a cycle whose second
/// sentence the reader cannot read regardless, so reading its first would
/// finish no card.
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

/// The same sentence bounded from above: "unless you control N or fewer …".
///
/// One phrase and its own table rather than a flag on [`COUNTED`], because
/// the two emit different modifiers and a shared table would have had to
/// carry the direction as data anyway. The predicate reaches five more pool
/// cards through the *other* arm below — the manlands print its complement —
/// and those two spellings meeting one `EnterModifier` is the whole point of
/// the variant.
const COUNTED_AT_MOST: [(&str, &str, &str); 1] =
    [(" or fewer other lands", "Filter::YOUR_LAND", "fast land")];

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
        let (cost, announced) = parse_cost(left)?;
        let mut effects = Vec::new();
        let mut is_mana = false;
        for (i, sentence) in sentences(right).iter().enumerate() {
            let parsed = parse_effect(sentence, announced)?;
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

    /// "You may choose not to untap this land during your untap step."
    ///
    /// A whole-line match, as in the transcoder and for the same reason:
    /// all six printings in this pool write it exactly this way, so a
    /// looser reading could only ever be reading something else. It is a
    /// static ability rather than a keyword bit — the rules make it a
    /// continuous effect modifying CR 502.3's turn-based action.
    fn may_not_untap_line(&mut self, line: &str) -> Option<()> {
        (line == "You may choose not to untap this land during your untap step").then(|| {
            self.body
                .abilities
                .push("static_ability!(Filter::This, Modifier::MayChooseNotToUntap)".to_string());
            self.body.notes.push("may choose not to untap".to_string());
        })
    }

    /// "At the beginning of your upkeep, if this land is tapped, put a
    /// storage counter on it."
    ///
    /// The storage lands' banking trigger, and the pool's one printing of
    /// an intervening-`if` clause on a land (CR 603.4): the ability does
    /// not trigger while the land is untapped, and is removed from the
    /// stack if the land has untapped by the time it would resolve. The
    /// clause travels to the card as `condition = Some(…)`, which is the
    /// only place that rule can be written down.
    ///
    /// The effect is read by the shared reader, so a counter noun with no
    /// id refuses the card the way it does everywhere else. `on it` is
    /// rewritten to `on this land` first, which is safe in **this** shape
    /// and nowhere else: the clause immediately before it says what "it"
    /// is, so the pronoun has one referent by construction.
    fn upkeep_if_tapped(&mut self, line: &str) -> Option<()> {
        let rest =
            line.strip_prefix("At the beginning of your upkeep, if this land is tapped, ")?;
        let sentence = format!(
            "{}{}",
            rest.chars().next()?.to_uppercase(),
            rest.get(1..)?.replace(" on it", " on this land")
        );
        let effects = parse_effect(&sentence, None)?;
        self.body.abilities.push(format!(
            "triggered!(Trigger::StepBegin {{ step: StepKind::Upkeep, whose: PlayerRel::You }}, \
             &[{}], condition = Some(Condition::SourceMatches(&Filter::Tapped)))",
            effects.join(", ")
        ));
        self.body
            .notes
            .push("upkeep trigger with an intervening if".to_string());
        Some(())
    }

    fn etb_trigger(&mut self, line: &str) -> Option<()> {
        let rest = line.strip_prefix("When this land enters, ")?;
        // A trigger has no cost, so nothing announced a number to it.
        let effects = parse_effect(rest, None)?;
        self.body.abilities.push(format!(
            "triggered!(Trigger::ETB, &[{}])",
            effects.join(", ")
        ));
        Some(())
    }

    /// The enters-tapped clauses whose condition is a **count**, in the three
    /// ways the printed cards write one.
    ///
    /// Its own method rather than three more arms in `enters_line`, because
    /// the three belong together: they count the same kind of thing and
    /// differ only in which side of the number turns the land on, and a
    /// reader deciding that has to have all three in front of it. Two say
    /// "unless you control N or more/fewer …" and share a prefix; the third
    /// is the second one's complement written as a condition, which is a
    /// different sentence and the same bound.
    fn counted_enters_clause(&mut self, line: &str) -> Option<()> {
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
            for (phrase, filter, note) in COUNTED_AT_MOST {
                let Some(n) = rest
                    .strip_suffix(phrase)
                    .and_then(number)
                    .and_then(|n| u8::try_from(n).ok())
                else {
                    continue;
                };
                self.body.enter_modifiers.push(format!(
                    "EnterModifier::TappedUnlessAtMost {{ filter: &{filter}, at_most: {n} }}"
                ));
                self.body.notes.push(note.to_string());
                return Some(());
            }
        }
        // The complement, printed as a condition rather than as an exception:
        // "if you control two or more other lands, this land enters tapped"
        // is the fast lands' bound one lower, and the manlands are the only
        // cycle that writes it. `n - 1` is the whole conversion, and `n = 0`
        // is refused because "if you control no other lands it enters tapped"
        // is not a sentence Magic prints and would need the opposite variant
        // to mean anything.
        let n = line
            .strip_prefix("If you control ")
            .and_then(|rest| rest.strip_suffix(" or more other lands, this land enters tapped"))
            .and_then(number)
            .and_then(|n| u8::try_from(n).ok())
            .and_then(|n| n.checked_sub(1))?;
        self.body.enter_modifiers.push(format!(
            "EnterModifier::TappedUnlessAtMost {{ filter: &Filter::YOUR_LAND, at_most: {n} }}"
        ));
        self.body.notes.push("manland bound".to_string());
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
        // Two modifiers out of one sentence, and they are two rules: a Vivid
        // land enters tapped *and* enters with counters, both as replacement
        // effects applied to the same event (CR 614.1c). The engine's list is
        // a list of modifiers rather than of sentences, so the sentence that
        // says both pushes both — and the order between them does not matter,
        // which is why nothing here states one.
        //
        // Before the plain `enters tapped` arm would have to be, if that arm
        // were a prefix test; it is an equality test, so this sits after it
        // and the reading is the same either way.
        if let Some(rest) = line.strip_prefix("This land enters tapped with ")
            && let Some((kind, n)) = rest.strip_suffix(" on it").and_then(counter_phrase)
        {
            self.body
                .enter_modifiers
                .push("EnterModifier::Tapped".into());
            self.body.enter_modifiers.push(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: Amount::Fixed({n}) }}"
            ));
            self.body
                .notes
                .push("enters tapped with counters".to_string());
            return Some(());
        }
        // The same sentence without the word "tapped" — Tendo Ice Bridge
        // enters untapped and still brings its counter, which is the whole
        // difference between it and the Vivid lands.
        if let Some(rest) = line.strip_prefix("This land enters with ")
            && let Some((kind, n)) = rest.strip_suffix(" on it").and_then(counter_phrase)
        {
            self.body.enter_modifiers.push(format!(
                "EnterModifier::WithCounters {{ kind: {kind}, amount: Amount::Fixed({n}) }}"
            ));
            self.body.notes.push("enters with counters".to_string());
            return Some(());
        }
        // Before the checkland below it, which claims the same prefix and
        // would refuse the whole card on the phrases these read.
        if self.counted_enters_clause(line).is_some() {
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
        // The two cycles whose condition counts *players* rather than
        // permanents, and which read nothing like the checkland above them —
        // "you control" is what every other clause here says, and neither of
        // these says it.
        if let Some(n) = line
            .strip_prefix("This land enters tapped unless you have ")
            .and_then(|rest| rest.strip_suffix(" or more opponents"))
            .and_then(number)
            .and_then(|n| u8::try_from(n).ok())
        {
            self.body.enter_modifiers.push(format!(
                "EnterModifier::TappedUnlessOpponents {{ at_least: {n} }}"
            ));
            self.body.notes.push("crowd land".to_string());
            return Some(());
        }
        // "a player", not "an opponent": Duskmourn prints the first, and the
        // difference is whether your own low life turns your own land on.
        if let Some(n) = line
            .strip_prefix("This land enters tapped unless a player has ")
            .and_then(|rest| rest.strip_suffix(" or less life"))
            .and_then(number)
            .and_then(|n| i32::try_from(n).ok())
        {
            self.body.enter_modifiers.push(format!(
                "EnterModifier::TappedUnlessSomeoneAtOrBelow {{ life: {n} }}"
            ));
            self.body.notes.push("unlucky land".to_string());
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
            .or_else(|| self.may_not_untap_line(line))
            .or_else(|| self.upkeep_if_tapped(line))
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
    let left = match type_line.split_once('\u{2014}') {
        Some((l, _)) => l,
        None => type_line,
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
    if let Some(ability) = intrinsic_mana_ability(type_line) {
        rec.body.abilities.push(ability);
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
            image_uris: None,
            image_status: None,
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

    /// The convoke land, which is two mana abilities on one `{T}` and the
    /// only land shape whose cost taps something that is not itself.
    #[test]
    fn a_convoke_land_taps_a_creature_beside_its_own_tap() {
        let body = read(
            "Land — Desert",
            "{T}: Add {C}.\n{T}, Tap an untapped creature you control: Add one mana of any color.",
        );
        assert_eq!(
            body.abilities[1],
            concat!(
                "mana_ability!(cost!(TapSelf, TapOther(&Filter::YOUR_CREATURE)), ",
                "&[Effect::mana_of_any_color()])"
            )
        );
    }

    /// The same sentence with a different noun, which this reader cannot
    /// build a filter for — so the card is refused whole rather than
    /// transcoded with a creature filter it never printed. Heap Gate and
    /// Dungeon Descent are the two live cases.
    #[test]
    fn the_same_phrase_with_another_noun_refuses_the_card() {
        for noun in ["Gate", "legendary creature", "artifact"] {
            let oracle = format!(
                "{{T}}: Add {{C}}.\n{{T}}, Tap an untapped {noun} you control: Add one mana of any color."
            );
            assert!(
                super::read(&card("Land", &oracle), &cats()).is_err(),
                "an unread noun has to refuse the card: {noun}"
            );
        }
    }

    /// The Vivid lands, which are the pool's whole shape for a permanent that
    /// arrives with counters and then spends them.
    ///
    /// One printed sentence, two modifiers, and an activation cost that is
    /// not the tap — held together because the card is worth nothing if
    /// either half is read alone: a land that enters with counters and cannot
    /// spend them is a tapland, and one that spends counters it never gets is
    /// an ability nothing can afford.
    #[test]
    fn a_vivid_land_enters_tapped_with_counters_and_spends_one_for_any_colour() {
        let body = read(
            "Land",
            "This land enters tapped with two charge counters on it.\n{T}: Add {R}.\n\
             {T}, Remove a charge counter from this land: Add one mana of any color.",
        );
        assert_eq!(
            body.enter_modifiers,
            [
                "EnterModifier::Tapped",
                "EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(2) }",
            ],
            "the one sentence says both, so it pushes both"
        );
        assert_eq!(
            body.abilities[1],
            concat!(
                "mana_ability!(cost!(TapSelf, RemoveCounterSelf { kind: CounterKind::Charge, ",
                "n: 1 }), &[Effect::mana_of_any_color()])"
            )
        );
    }

    /// Tendo Ice Bridge, which is the same card without the word "tapped" —
    /// and the reason the two sentences are read by two arms rather than by
    /// one with an optional word in it.
    #[test]
    fn a_counter_land_can_enter_untapped_and_still_bring_its_counter() {
        let body = read(
            "Land",
            "This land enters with a charge counter on it.\n{T}: Add {C}.\n\
             {T}, Remove a charge counter from this land: Add one mana of any color.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(1) }"],
            "no `Tapped`, because the card does not print the word"
        );
    }

    /// Mirrodin's Core fills itself, which is the other half of the same
    /// counter and the one that goes through the *placing* door.
    #[test]
    fn a_land_that_puts_a_counter_on_itself_needs_no_target() {
        let body = read(
            "Land",
            "{T}: Add {C}.\n{T}: Put a charge counter on this land.\n\
             {T}, Remove a charge counter from this land: Add one mana of any color.",
        );
        assert_eq!(
            body.abilities[1],
            concat!(
                "activated!(Cost::TAP, &[Effect::AddCounter { kind: CounterKind::Charge, ",
                "amount: Amount::Fixed(1) }])"
            )
        );
    }

    /// A depletion land, whole: two counters on arrival, a counter off as a
    /// cost, and the clause that ends the card.
    ///
    /// The point of reading it here rather than only in the engine is which
    /// *macro* comes out. `activated_line` decides "mana ability" from the
    /// first sentence alone, so a trailing sacrifice must not turn the line
    /// into an `activated!` — a land whose mana went on the stack would ask
    /// for priority in the middle of paying for a spell (CR 605.3b).
    #[test]
    fn a_depletion_land_spends_a_counter_and_sacrifices_itself_at_nought() {
        let body = read(
            "Land",
            "This land enters tapped with two depletion counters on it.\n\
             {T}, Remove a depletion counter from this land: Add {B}{B}. \
             If there are no depletion counters on this land, sacrifice it.",
        );
        assert_eq!(
            body.enter_modifiers,
            [
                "EnterModifier::Tapped",
                "EnterModifier::WithCounters { kind: counters::DEPLETION, amount: Amount::Fixed(2) }",
            ]
        );
        assert_eq!(
            body.abilities,
            [concat!(
                "mana_ability!(cost!(TapSelf, RemoveCounterSelf { kind: counters::DEPLETION, ",
                "n: 1 }), &[Effect::mana(ManaColor::Black, 2), ",
                "Effect::IfNoCountersOnSelf { kind: counters::DEPLETION, ",
                "then: &[Effect::SacrificeSelf] }])"
            )],
            "one ability, two effects, and still a mana ability"
        );
    }

    /// Gemstone Mine: the same card with another noun and another colour
    /// clause, which is what says the noun is a parameter rather than a
    /// second copy of the rule.
    #[test]
    fn the_same_clause_reads_a_mining_counter_and_an_untapped_arrival() {
        let body = read(
            "Land",
            "This land enters with three mining counters on it.\n\
             {T}, Remove a mining counter from this land: Add one mana of any color. \
             If there are no mining counters on this land, sacrifice it.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::WithCounters { kind: counters::MINING, amount: Amount::Fixed(3) }"],
            "no `Tapped`, because the card does not print the word"
        );
        assert_eq!(
            body.abilities,
            [concat!(
                "mana_ability!(cost!(TapSelf, RemoveCounterSelf { kind: counters::MINING, ",
                "n: 1 }), &[Effect::mana_of_any_color(), ",
                "Effect::IfNoCountersOnSelf { kind: counters::MINING, ",
                "then: &[Effect::SacrificeSelf] }])"
            )]
        );
    }

    /// Every counter a land prints that has no id, in all three sentences
    /// that name one — and now in the fourth, which is the clause that ends
    /// a depletion land.
    ///
    /// Verse counters are printed by a land in this pool and nobody has
    /// assigned that word an id. Reading the number and dropping the noun
    /// would hand the card a charge counter and an ability that spends one
    /// — so the whole card is refused instead, and this is the test that it
    /// is.
    #[test]
    fn a_counter_the_dsl_cannot_name_refuses_the_card() {
        for noun in ["verse"] {
            for oracle in [
                format!(
                    "This land enters tapped with two {noun} counters on it.\n{{T}}: Add {{R}}."
                ),
                format!("This land enters with a {noun} counter on it.\n{{T}}: Add {{R}}."),
                format!(
                    "{{T}}: Add {{C}}.\n{{T}}, Remove a {noun} counter from this land: Add one mana of any color."
                ),
                format!("{{T}}: Add {{C}}.\n{{T}}: Put a {noun} counter on this land."),
                format!(
                    "{{T}}: Add {{C}}. If there are no {noun} counters on this land, sacrifice it."
                ),
            ] {
                assert!(
                    super::read(&card("Land", &oracle), &cats()).is_err(),
                    "a counter with no kind has to refuse the card: {oracle}"
                );
            }
        }
    }

    /// The clause is matched whole, which is the difference between reading
    /// a sentence and recognising three of its words.
    ///
    /// None of these four is printed by any card, and that is the point: a
    /// reader loose enough to accept one of them is loose enough to accept
    /// a sentence Wizards prints next year that means something else.
    #[test]
    fn a_near_miss_of_the_sacrifice_clause_is_not_that_clause() {
        for tail in [
            "If there are no depletion counters on this land, destroy it",
            "If there is no depletion counter on this land, sacrifice it",
            "If there are no depletion counters on this creature, sacrifice it",
            "If there are no counters on this land, sacrifice it",
        ] {
            assert_eq!(
                super::parse_effect(tail, None),
                None,
                "a sentence this did not read has to refuse: {tail}"
            );
        }
        assert!(
            super::parse_effect(
                "If there are no depletion counters on this land, sacrifice it",
                None,
            )
            .is_some(),
            "and the one the card prints has to be read"
        );
    }

    /// A storage land, whole: bank a counter a turn, then spend any number
    /// of them at once.
    ///
    /// Two sentences that only make sense together. The cost announces a
    /// number and names no figure; the effect says "for each storage counter
    /// removed this way", which is that same number and is written
    /// `Amount::X` — not a count of what is on the land, because by the time
    /// the mana is added the counters are gone.
    #[test]
    fn a_storage_land_banks_a_counter_and_spends_any_number_of_them() {
        let body = read(
            "Land",
            "This land enters tapped.\n{T}: Put a storage counter on this land.\n\
             {T}, Remove any number of storage counters from this land: \
             Add {W} for each storage counter removed this way.",
        );
        assert_eq!(body.enter_modifiers, ["EnterModifier::Tapped"]);
        assert_eq!(
            body.abilities,
            [
                concat!(
                    "activated!(Cost::TAP, &[Effect::AddCounter { kind: counters::STORAGE, ",
                    "amount: Amount::Fixed(1) }])"
                ),
                concat!(
                    "mana_ability!(cost!(TapSelf, RemoveCounterSelfX { kind: counters::STORAGE ",
                    "}), &[Effect::mana_dynamic(ManaColor::White, Amount::X)])"
                ),
            ]
        );
    }

    /// The Fallen Empires storage cycle, whole — the five lands whose
    /// counter comes from an upkeep trigger instead of an activation.
    ///
    /// Every clause of Bottomless Vault, and the two new ones are what the
    /// cycle was waiting for: "you may choose not to untap" as a static
    /// ability, and the upkeep trigger with its intervening `if` carried as
    /// a `condition` rather than dropped. A dropped clause here would be a
    /// land that banks a counter every upkeep whether or not it is tapped,
    /// which is not a card anybody printed.
    #[test]
    fn a_fallen_empires_storage_land_reads_whole() {
        let body = read(
            "Land",
            "This land enters tapped.\n\
             You may choose not to untap this land during your untap step.\n\
             At the beginning of your upkeep, if this land is tapped, \
             put a storage counter on it.\n\
             {T}, Remove any number of storage counters from this land: \
             Add {B} for each storage counter removed this way.",
        );
        assert_eq!(body.enter_modifiers, ["EnterModifier::Tapped"]);
        assert_eq!(
            body.abilities,
            [
                "static_ability!(Filter::This, Modifier::MayChooseNotToUntap)",
                concat!(
                    "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: ",
                    "PlayerRel::You }, &[Effect::AddCounter { kind: counters::STORAGE, ",
                    "amount: Amount::Fixed(1) }], condition = ",
                    "Some(Condition::SourceMatches(&Filter::Tapped)))"
                ),
                concat!(
                    "mana_ability!(cost!(TapSelf, RemoveCounterSelfX { kind: counters::STORAGE ",
                    "}), &[Effect::mana_dynamic(ManaColor::Black, Amount::X)])"
                ),
            ]
        );
    }

    /// One word off either new sentence and the land is a stub again.
    ///
    /// The untap line is matched whole, as in the transcoder; the upkeep
    /// trigger is matched on its clause, so a trigger with a *different*
    /// intervening `if` must not be read as this one — the condition would
    /// be a sentence the card does not print.
    #[test]
    fn a_neighbouring_sentence_is_not_read_as_either_of_them() {
        for line in [
            "You may choose not to untap this land during your upkeep",
            "At the beginning of your end step, if this land is tapped, \
             put a storage counter on it",
            "At the beginning of your upkeep, if this land is untapped, \
             put a storage counter on it",
            "At the beginning of your upkeep, if this land is tapped, \
             put a verse counter on it",
        ] {
            let card = card("Land", &format!("{line}."));
            assert_eq!(
                super::read(&card, &cats()).err(),
                Some(LandRefusal::UnreadLine(line.to_string())),
                "{line}"
            );
        }
    }

    /// The Time Spiral storage cycle, whole — the other printed spelling of
    /// the same cost, on the five lands that pay mana for their counters
    /// instead of tapping.
    ///
    /// Three things this asserts that the Mercadian Masques cycle cannot.
    /// The cost carries **mana and an announced number and no `{T}`**, which
    /// is the shape that would break a reader treating the announcement as
    /// the whole cost. The mana line is a *combination* — a pick per mana,
    /// so three counters here can buy `{W}{U}{W}` where Fountain of Cho's
    /// three buy `{W}{W}{W}`. And the `{1}` on the banking line is a second
    /// cost on the same card, so nothing about the land is `Cost::TAP`.
    #[test]
    fn a_storage_land_can_pour_its_counters_into_two_colours_at_once() {
        assert_eq!(
            super::parse_cost("{1}, Remove X storage counters from this land"),
            Some((
                "cost!(\"{1}\", RemoveCounterSelfX { kind: counters::STORAGE })".to_string(),
                Some("counters::STORAGE"),
            ))
        );
        let body = read(
            "Land",
            "{T}: Add {C}.\n{1}, {T}: Put a storage counter on this land.\n\
             {1}, Remove X storage counters from this land: \
             Add X mana in any combination of {G} and/or {W}.",
        );
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])",
                concat!(
                    "activated!(cost!(\"{1}\", TapSelf), &[Effect::AddCounter { kind: ",
                    "counters::STORAGE, amount: Amount::Fixed(1) }])"
                ),
                concat!(
                    "mana_ability!(cost!(\"{1}\", RemoveCounterSelfX { kind: counters::STORAGE ",
                    "}), &[Effect::mana_combination(&[ManaColor::Green, ManaColor::White], ",
                    "Amount::X)])"
                ),
            ]
        );
    }

    /// The same sentence with a number in it instead of an X, which is
    /// Cascading Cataracts — and which needs no cost to have announced
    /// anything.
    ///
    /// It is the pair to the guard below: the *fixed* form is self-contained
    /// and the *X* form is not, so one reads off a bare `{5}, {T}` and the
    /// other refuses on it. Its keyword line rides along, which is the rest
    /// of that card.
    #[test]
    fn a_fixed_combination_reads_without_an_announced_number() {
        let body = read(
            "Land",
            "Indestructible\n{T}: Add {C}.\n\
             {5}, {T}: Add five mana in any combination of colors.",
        );
        assert_eq!(body.keywords, ["KeywordSet::INDESTRUCTIBLE"]);
        assert_eq!(
            body.abilities,
            [
                "mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])",
                concat!(
                    "mana_ability!(cost!(\"{5}\", TapSelf), ",
                    "&[Effect::mana_combination(ALL_MANA_COLORS, Amount::Fixed(5))])"
                ),
            ]
        );
    }

    /// `Amount::X` with nothing to read it from is nought, so "Add X mana"
    /// beside a cost that announced no number has to refuse.
    ///
    /// No card prints that pair — this is the guard on a reader, not on a
    /// printing — and the counter-test beside it is the card that does: one
    /// word of the cost changed, and the same sentence reads.
    #[test]
    fn x_mana_needs_a_cost_that_announced_a_number() {
        assert_eq!(
            super::parse_effect("Add X mana in any combination of {G} and/or {W}", None),
            None,
            "nothing announced a number, so X is nought and the card is a lie"
        );
        assert!(
            super::parse_effect(
                "Add X mana in any combination of {G} and/or {W}",
                Some("counters::STORAGE"),
            )
            .is_some(),
            "and the cost the card actually prints beside it makes it read"
        );
    }

    /// Three cards print the phrase and are still refused, each on a clause
    /// that has nothing to do with mana.
    ///
    /// Great Hall of the Citadel and Crucible of the Spirit Dragon both say
    /// "Spend this mana only to …", which this reader does not read, and
    /// Baxter Building hangs an `Activate only if` condition off a draw.
    ///
    /// **The mana sentence is asserted to read first**, and that is the
    /// whole test rather than a preamble: `is_err()` is satisfied by a
    /// refusal from *any* clause, so without the two lines above it this
    /// would pass just as well if the combination rule had quietly stopped
    /// working — it would be a test named after one cause and held up by
    /// another.
    #[test]
    fn a_spend_restriction_still_refuses_a_land_whose_mana_reads() {
        assert!(
            super::parse_effect("Add two mana in any combination of colors", None).is_some(),
            "the mana half of Great Hall of the Citadel reads on its own"
        );
        assert!(
            super::parse_effect(
                "Add X mana in any combination of colors",
                Some("counters::STORAGE"),
            )
            .is_some(),
            "and so does Crucible of the Spirit Dragon's"
        );
        for text in [
            "{T}: Add {C}.\n{1}, {T}: Add two mana in any combination of colors. \
             Spend this mana only to cast legendary spells.",
            "{T}: Add {C}.\n{1}, {T}: Put a storage counter on this land.\n\
             {T}, Remove X storage counters from this land: \
             Add X mana in any combination of colors. Spend this mana only to \
             cast Dragon spells or activate abilities of Dragons.",
            "{T}: Add {C}.\n{4}, {T}: Add four mana in any combination of colors.\n\
             {4}, {T}: Draw a card. Activate only if you control a creature \
             with toughness 4 or greater.",
        ] {
            assert!(
                super::read(&card("Land", text), &cats()).is_err(),
                "one unread clause refuses the whole card: {text}"
            );
        }
    }

    /// The effect reads the cost beside it, which is the one place in this
    /// reader where a sentence is not self-contained.
    ///
    /// None of these four is printed by any card. Each is one word away from
    /// the sentence that is, and a reader loose enough to take any of them
    /// would emit a land that makes mana out of counters it never removed.
    #[test]
    fn mana_for_each_counter_removed_has_to_match_the_cost_that_removed_it() {
        let storage = Some("counters::STORAGE");
        assert!(
            super::parse_effect("Add {W} for each storage counter removed this way", storage)
                .is_some(),
            "the sentence the card prints"
        );
        for (sentence, announced) in [
            // The cost announced nothing: no number was ever chosen.
            ("Add {W} for each storage counter removed this way", None),
            // The cost announced another counter.
            ("Add {W} for each charge counter removed this way", storage),
            // A counter with no id at all.
            ("Add {W} for each verse counter removed this way", storage),
            // Counters on the land, which is a different card (City of
            // Shadows) and a different amount.
            ("Add {W} for each storage counter on this land", storage),
        ] {
            assert_eq!(
                super::parse_effect(sentence, announced),
                None,
                "a sentence this did not read has to refuse: {sentence}"
            );
        }
    }

    /// Every counter this reader can spell is one the DSL actually assigns.
    ///
    /// `COUNTERS` is a second copy of a fact that lives in
    /// `baylee_cards_dsl::counters`, and the compiler catches only half of a
    /// disagreement: a spelling that names no constant fails to build the
    /// card it wrote, but a **word** invented here would happily emit
    /// `counters::DEPLETION` for a counter nobody has ever printed. So the
    /// pair is asserted rather than trusted — the same bargain as the
    /// authoring contract's list of cost parts.
    #[test]
    fn every_counter_this_reads_is_one_the_dsl_assigns() {
        use baylee_cards_dsl::counters::ASSIGNED;
        for (word, spelling) in super::COUNTERS {
            let Some(name) = spelling.strip_prefix("counters::") else {
                assert!(
                    spelling.starts_with("CounterKind::"),
                    "{word} is spelled as neither an assigned id nor a \
                     named kind: {spelling}"
                );
                continue;
            };
            assert!(
                ASSIGNED.iter().any(|(assigned, _)| *assigned == word),
                "{word} is a counter this reader names and the DSL does not \
                 assign"
            );
            assert_eq!(
                name,
                word.to_uppercase(),
                "the constant and the printed word have to be the same word"
            );
        }
    }

    /// And the number has to agree with its noun, which is the cheap half of
    /// reading the phrase at all: a singular beside a plural is a sentence
    /// this did not parse, whatever else it matched.
    #[test]
    fn a_count_that_disagrees_with_its_noun_is_not_a_phrase_this_read() {
        let charge = "CounterKind::Charge";
        assert_eq!(
            super::counter_phrase("two charge counters"),
            Some((charge, 2))
        );
        assert_eq!(super::counter_phrase("a charge counter"), Some((charge, 1)));
        assert_eq!(
            super::counter_phrase("one charge counter"),
            Some((charge, 1))
        );
        assert_eq!(
            super::counter_phrase("three mining counters"),
            Some(("counters::MINING", 3))
        );
        assert_eq!(super::counter_phrase("two charge counter"), None);
        assert_eq!(super::counter_phrase("a charge counters"), None);
        assert_eq!(super::counter_phrase("a +1/+1 counter"), None);
        assert_eq!(super::counter_phrase("two verse counters"), None);
        assert_eq!(super::counter_phrase("two counters"), None);
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
                .any(|a| a.contains("Trigger::ETB") && a.contains("Effect::gain_life(1)")),
            "a land's enter-trigger points at the land, which is what the \
             constant spells: {:?}",
            body.abilities
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

    /// The fast lands: a slow land's sentence with one word changed, and the
    /// bound is the other way round.
    ///
    /// Held beside `a_slow_land_counts_the_other_lands` deliberately. The two
    /// texts differ in exactly "more"/"fewer", they count the same filter to
    /// the same number, and a reader that read one of them for the other
    /// would produce a card that is right on an empty board and wrong on
    /// every other — which is the board these lands are played on.
    #[test]
    fn a_fast_land_is_a_slow_land_bounded_the_other_way() {
        let body = read(
            "Land",
            "This land enters tapped unless you control two or fewer other lands.\n{T}: Add {G}.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 2 }"]
        );
        assert!(body.statics.is_empty(), "{}", body.statics);
    }

    /// The manlands print the same bound as its complement, and one lower.
    ///
    /// "If you control two or more other lands, this land enters tapped" is
    /// `at_most: 1` — the conversion is the whole reading, and getting it
    /// wrong by one is a land that is untapped on precisely the turn it
    /// should not be. The card here stops at the mana ability: the five that
    /// print this sentence also animate themselves, which no rule reads, so
    /// this is the sentence being read rather than a card being finished.
    #[test]
    fn a_manland_prints_the_same_bound_as_a_condition() {
        let body = read(
            "Land",
            "If you control two or more other lands, this land enters tapped.\n{T}: Add {U}.",
        );
        assert_eq!(
            body.enter_modifiers,
            ["EnterModifier::TappedUnlessAtMost { filter: &Filter::YOUR_LAND, at_most: 1 }"]
        );
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

    /// The two cycles whose condition counts players, read off the printed
    /// line rather than off a reference script.
    ///
    /// They sit here and not only in `scriptgen` because `landgen` runs
    /// first: for a land it is this reader that decides whether a card is
    /// written at all.
    #[test]
    fn a_crowd_and_an_unlucky_land_count_players() {
        let crowd = read(
            "Land",
            "This land enters tapped unless you have two or more opponents.\n{T}: Add {B} or {R}.",
        );
        assert_eq!(
            crowd.enter_modifiers,
            ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
        );

        let unlucky = read(
            "Land",
            "This land enters tapped unless a player has 13 or less life.\n{T}: Add {B} or {R}.",
        );
        assert_eq!(
            unlucky.enter_modifiers,
            ["EnterModifier::TappedUnlessSomeoneAtOrBelow { life: 13 }"]
        );

        // "A player" and "an opponent" are two different cards, and only the
        // first is printed. The second has to refuse rather than be read as
        // the first, because it is the one whose own low life would not turn
        // the land on.
        assert!(
            recognize(
                &card(
                    "Land",
                    "This land enters tapped unless an opponent has 13 or less life.\n{T}: Add {B}."
                ),
                &cats()
            )
            .is_none(),
            "an opponent's life is not a player's life"
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
