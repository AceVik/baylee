//! `validate`: a card's definition against its printing — types, faces,
//! colours, how it enters, and its header.

use crate::{
    KEYWORD_WORDS, PrintingTally, check_activation_cost_matches_the_printing,
    check_mana_matches_the_printing, check_optional_clauses_are_offered,
    check_scope_matches_the_text, code_costs, face_texts, knob, mentions_word, printed_text,
    quoted_value, strip_reminders,
};

/// A Scryfall color letter.
pub(crate) fn color_of_letter(letter: &str) -> Option<baylee_cards::dsl::Color> {
    use baylee_cards::dsl::Color;
    Some(match letter {
        "W" => Color::White,
        "U" => Color::Blue,
        "B" => Color::Black,
        "R" => Color::Red,
        "G" => Color::Green,
        _ => return None,
    })
}

/// A color set as the letters a printing would use, for a message someone
/// can hold against Scryfall.
pub(crate) fn color_letters(set: baylee_cards::dsl::ColorSet) -> String {
    use baylee_cards::dsl::Color;
    let mut out = String::new();
    for (color, letter) in [
        (Color::White, 'W'),
        (Color::Blue, 'U'),
        (Color::Black, 'B'),
        (Color::Red, 'R'),
        (Color::Green, 'G'),
    ] {
        if set.contains(color) {
            out.push(letter);
        }
    }
    if out.is_empty() {
        out.push_str("(colorless)");
    }
    out
}

/// The compiled card against the printing: its color identity, the keyword
/// bits it claims, and the mana its abilities offer to make.
///
/// The other half of [`check_code_matches_the_printing`], and separate from
/// it because these three are not literals a `find` can lift out of a source
/// file — they are sets, computed by codegen or written across several
/// fields, and the compiled `CardDef` is the only place they exist as one
/// answer.
///
/// Two of the five fields the plan named are not here, and both for the same
/// reason: the cache holds `ScryfallCard`, a trimmed struct, not Scryfall's
/// whole payload. It has no `keywords` array and no `produced_mana`, so both
/// are read out of the printed text instead — which the cache does keep, and
/// which is what a person checking the card would read anyway. `defense` is
/// absent from all three: the cache, the DSL, and the pool, which has no
/// battle in it.
pub(crate) fn check_card_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // Color identity (CR 903.4) decides which decks may play the card, and
    // it is codegen's arithmetic over every face rather than anything a
    // person typed — which is exactly why nothing was checking it.
    if let Some(letters) = payload
        .get("color_identity")
        .and_then(serde_json::Value::as_array)
    {
        let printed = letters
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter_map(color_of_letter)
            .fold(baylee_cards::dsl::ColorSet::EMPTY, |set, c| {
                set.union(baylee_cards::dsl::ColorSet::of(c))
            });
        tally.identity += 1;
        if printed != def.color_identity {
            println!(
                "{slug}: the printing's color identity is {} and the code's is {}",
                color_letters(printed),
                color_letters(def.color_identity),
            );
            *problems += 1;
        }
    }

    let whole = printed_text(payload).to_lowercase();
    let printed = strip_reminders(&whole);

    // Keywords, in the one direction that is a bug. A bit the code claims
    // has to be printed on the card; a keyword in the text with no bit is
    // ordinary — most keywords are `AbilityDef` data rather than bits, and a
    // partial card is allowed to leave one unimplemented.
    let claimed = def.all_keywords();
    let mut asked = false;
    for (bit, word) in KEYWORD_WORDS {
        if !claimed.contains(*bit) {
            continue;
        }
        asked = true;
        if !mentions_word(&printed, word) {
            println!("{slug}: the code claims {word} and the printed text never says it");
            *problems += 1;
        }
    }
    tally.keywords += usize::from(asked);

    // Reminder text *kept* for the mana check, and taken out for the keyword
    // one above. Both are load-bearing and they pull opposite ways: a Reach
    // creature must not read as having Flash because its own reminder
    // mentions flashback, and a dual land prints its mana ability as nothing
    // but reminder text — Taiga's whole oracle text is
    // "({T}: Add {R} or {G}.)", so stripping it leaves a land that taps for
    // two colors and says nothing at all.
    check_mana_matches_the_printing(slug, def, &whole, tally, problems);
    check_activation_cost_matches_the_printing(slug, def, &whole, tally, problems);
}

/// Each face's type line, against the one the printing puts on the card.
///
/// This is the cheapest check in the family and it was the missing one: what
/// a card *is* was the one printed characteristic nothing ever asked the
/// printing about. `validate` holds the `//!` header against the **code**,
/// the sweeps hold the engine's object against the `CardDef`, and a card
/// whose header and code agree on the wrong thing walks through both — Ondu
/// Cleric said Human in its header and in its `subtypes` for as long as it
/// had existed, and the printing says Kor. Raffine's Tower is the extreme of
/// the same class: a `TypeSet::LAND` with no subtypes at all, so it made no
/// mana (CR 305.6), and it took a second reader of the reference script to find.
///
/// What is compared is [`baylee_cards::pool::type_line`], the **one**
/// renderer there is, rather than a set built here for the purpose. That is
/// the whole trick: it is the string the deckbuilder shows a player, so a
/// pass says the pool prints what the cards print, and it carries supertypes
/// and types along with the subtypes for free — Karakas and Volrath's
/// Stronghold are `Legendary Land` on the card and were plain `Land` in the
/// code, which is the legend rule (CR 704.5j) not applying to two of them.
/// A second table of type words in this file would have been a second
/// classifier to keep in step, which is the mistake `transcode-report` already
/// made once.
///
/// Order is part of the comparison because it is part of the card: the words
/// on a type line are printed in a fixed order, which is why `TypeSet::words`
/// and `SupertypeSet::words` each carry a hand-kept `PRINTED_ORDER` table to
/// reproduce it, and why a `FaceDef`'s `subtypes` is a slice printed in the
/// order it is written rather than a set. Holding the rendered string against
/// the printed one compares that order for free, and against the only
/// authority there is for it — the card. No rule here says what the order
/// *is*, so nothing here can be wrong about it.
///
/// Two tolerances, and both are about *this repo's* limits rather than the
/// cards'. A face count that disagrees skips the card, exactly as
/// [`check_card_matches_the_printing`]'s cost check does — a split card and
/// an adventure print two faces Scryfall's way and are one `FaceDef` here, so
/// a positional pairing would hold a creature's line against an instant's.
/// And a subtype word the catalog does not know skips the face: the catalogs
/// are what `codegen` builds the constants from, so an unknown word is this
/// command's own gap and never a fact about the card.
/// The five printing checks that need the compiled `CardDef`, in one door.
///
/// `validate` walks the card *files* and a file whose name resolves to no
/// `CardDef` is a finding of its own, made above — so everything here may
/// take the definition rather than an `Option` of it, and `validate` keeps
/// one line for the five instead of five.
pub(crate) fn check_def_against_the_printing(
    slug: &str,
    def: &'static baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    check_scope_matches_the_text(slug, def.name(), def, payload, problems);
    check_card_matches_the_printing(slug, def, payload, tally, problems);
    check_type_line_matches_the_printing(slug, def, payload, tally, problems);
    check_optional_clauses_are_offered(slug, def, payload, tally, problems);
    check_enters_tapped_bound_matches_the_printing(slug, def, payload, tally, problems);
    check_back_face_castability_matches_the_layout(slug, def, payload, tally, problems);
    check_flashback_matches_the_printing(slug, def, payload, problems);
}

/// A printed "Flashback {…}" is `FaceDef::flashback`, at the printed price,
/// and a `flashback` the printing does not have is a graveyard cast nobody
/// may make (CR 702.34a).
///
/// Mana only, both ways: "Flashback—Pay 3 life" and the like are no
/// `ManaCost` and are left to the card's coverage. Implemented cards only,
/// the exemption every check here takes.
pub(crate) fn check_flashback_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    problems: &mut usize,
) {
    if !def.is_implemented() {
        return;
    }
    let printed: Vec<String> = printed_text(payload)
        .lines()
        .filter_map(|line| line.strip_prefix("Flashback "))
        .filter_map(|rest| rest.split(" (").next())
        .filter_map(|cost| cost.trim().parse::<baylee_core::mana::ManaCost>().ok())
        .map(|cost| cost.to_string())
        .collect();
    let written: Vec<String> = def
        .faces
        .iter()
        .filter_map(|f| f.flashback)
        .map(|cost| cost.to_string())
        .collect();
    if printed != written {
        println!("{slug}: the printing's flashback is {printed:?} and the code's is {written:?}");
        *problems += 1;
    }
}

/// Whether a two-faced card's **back** may be played out of hand, against the
/// kind of double-faced card the printing says it is.
///
/// CR 712.8a: a double-faced card in hand has only its front face's
/// characteristics, and CR 712.8c spells out the consequence — a nonmodal
/// (transforming) double-faced card is cast as its front face and reaches its
/// back only by turning over. A modal one is the exception the player
/// chooses between. `FaceDef::castable_from_hand` is the flag, it defaults to
/// `true`, and the default is right for exactly one of those two kinds.
///
/// Nothing compared it, and the cost of that was not one card. A
/// transforming card whose back face is a **land** is a land drop anybody can
/// make: the engine offers the back out of hand, the arrival is a perfectly
/// ordinary land, and every other check agrees, because the card is right
/// about everything except which face a player may reach. The sweep in
/// `card_tests::lands` cannot see it either — it plays every spell-fronted
/// land back and asserts the arrival matches that face, which it does.
///
/// Scryfall's `layout` is the one place the two kinds are told apart, which
/// is why this is a printing check and not a pool lint: a card file cannot
/// state its own layout without a person deciding it, and a person deciding
/// it is what went wrong twenty times.
pub(crate) fn check_back_face_castability_matches_the_layout(
    slug: &str,
    def: &'static baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let [_, back] = def.faces else { return };
    let want = match payload.get("layout").and_then(serde_json::Value::as_str) {
        Some("transform") => false,
        Some("modal_dfc") => true,
        // Every other layout either has one castable face (`normal`) or two
        // that are both played out of hand (`adventure`, `split`, `flip`),
        // and a meld back is a card of its own rather than a face here.
        _ => return,
    };
    tally.back_faces += 1;
    if back.castable_from_hand != want {
        let kind = if want { "modal" } else { "transforming" };
        println!(
            "{slug}: the printing is a {kind} double-faced card (CR 712.8c), so its \
             back face `{}` is castable_from_hand = {want} and the code says {}",
            back.name, back.castable_from_hand
        );
        *problems += 1;
    }
}

pub(crate) fn check_type_line_matches_the_printing(
    slug: &str,
    def: &'static baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let printed_lines: Vec<&str> = match payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        Some(faces) => faces
            .iter()
            .filter_map(|f| f.get("type_line").and_then(serde_json::Value::as_str))
            .collect(),
        None => payload
            .get("type_line")
            .and_then(serde_json::Value::as_str)
            .into_iter()
            .collect(),
    };
    if printed_lines.len() != def.faces.len() {
        return;
    }
    for (at, (line, face)) in printed_lines.iter().zip(def.faces).enumerate() {
        if printed_subtypes(line).is_none() {
            continue;
        }
        tally.type_lines += 1;
        let code = baylee_cards::pool::type_line(face);
        if code != line.trim() {
            let where_ = if def.faces.len() > 1 {
                format!("face {at} of {slug}")
            } else {
                slug.to_string()
            };
            println!("{where_}: the printing's type line is {line:?} and the code's is {code:?}");
            *problems += 1;
        }
    }
}

/// The subtypes one printed type line names, or `None` if a word is unknown.
///
/// Everything after the em dash, longest match first, because "Time Lord" is
/// the one subtype in the whole catalog written as two words and a plain
/// whitespace split would read it as two subtypes that do not exist. A line
/// with no dash prints no subtypes and is an empty list rather than a skip:
/// that a card has none is a fact worth comparing, and it is the half of
/// this check that Raffine's Tower needed.
pub(crate) fn printed_subtypes(type_line: &str) -> Option<Vec<baylee_core::ids::SubtypeId>> {
    use baylee_core::generated::subtypes;
    let Some((_, tail)) = type_line.split_once('\u{2014}') else {
        return Some(Vec::new());
    };
    let words: Vec<&str> = tail.split_whitespace().collect();
    let mut out = Vec::new();
    let mut at = 0;
    while at < words.len() {
        if at + 1 < words.len() {
            let pair = format!("{} {}", words[at], words[at + 1]);
            if let Some(id) = subtypes::by_name(&pair) {
                out.push(id);
                at += 2;
                continue;
            }
        }
        out.push(subtypes::by_name(words[at])?);
        at += 1;
    }
    Some(out)
}

/// The bound a printed enters-tapped-unless sentence states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrintedBound {
    /// "unless you control N or more …" — untapped from N upwards.
    AtLeast(u32),
    /// "unless you control N or fewer …" — untapped at N and below.
    AtMost(u32),
}

/// The English number words a printed card uses for a count, and the digits
/// beside them because a payload is allowed to write either.
pub(crate) fn number_word(word: &str) -> Option<u32> {
    Some(match word {
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        other => other.parse().ok()?,
    })
}

/// What one printed face says about entering tapped, or `None` for a face
/// that states no counted bound.
///
/// Read one **sentence** at a time and never over the whole text, because
/// "unless you control" is a common clause: a card that sacrifices itself
/// unless you control two Swamps and separately enters tapped would
/// otherwise have the two halves read as one sentence they are not.
///
/// The three forms the pool prints are two sentences, because the third is
/// the second turned round. "If you control two or more other lands, this
/// land enters tapped" says tapped from two upwards, which is untapped at
/// one and below — so it is `AtMost(1)` and the subtraction is the whole
/// reason this check exists: five cards print that sentence and one of them
/// wrote the printed number straight into the bound.
pub(crate) fn printed_enters_tapped_bound(text: &str) -> Option<PrintedBound> {
    let lower = text.to_lowercase();
    for sentence in lower.split('.') {
        if !(sentence.contains("enters") && sentence.contains("tapped")) {
            continue;
        }
        for (lead, inverted) in [("unless you control ", false), ("if you control ", true)] {
            let Some(at) = sentence.find(lead) else {
                continue;
            };
            let mut words = sentence[at + lead.len()..].split_whitespace();
            let Some(n) = words.next().and_then(number_word) else {
                continue;
            };
            if words.next() != Some("or") {
                continue;
            }
            match (words.next(), inverted) {
                (Some("more"), false) => return Some(PrintedBound::AtLeast(n)),
                (Some("fewer"), false) => return Some(PrintedBound::AtMost(n)),
                (Some("more"), true) => return Some(PrintedBound::AtMost(n.checked_sub(1)?)),
                _ => {}
            }
        }
    }
    None
}

/// The bound a face's `EnterModifier`s code, or `None` for a face that codes
/// no counted one.
pub(crate) fn coded_enters_tapped_bound(face: &baylee_cards::dsl::FaceDef) -> Option<PrintedBound> {
    face.enter_modifiers.iter().find_map(|m| match m {
        // Both bounds are `u8` on the card and the printed one is parsed
        // as a `u32`, so the widening happens here rather than at the
        // comparison, where a cast is a place a mistake can hide.
        baylee_cards::dsl::EnterModifier::TappedUnlessCount { at_least, .. } => {
            Some(PrintedBound::AtLeast(u32::from(*at_least)))
        }
        baylee_cards::dsl::EnterModifier::TappedUnlessAtMost { at_most, .. } => {
            Some(PrintedBound::AtMost(u32::from(*at_most)))
        }
        _ => None,
    })
}

/// Holds a counted enters-tapped-unless bound against the number the card
/// prints, face by face.
///
/// This is the one printed *number* on a land that nothing compared. The
/// oracle check says the header quotes the printing, and the header of a
/// wrong card quotes it perfectly — Lair of the Hydra printed "If you
/// control two or more other lands, this land enters tapped" in its header,
/// built `at_most: 2` underneath it, and came down untapped off exactly two
/// other lands where the four other cards printing that identical sentence
/// came down tapped. Every side of that card read as correct: right header,
/// right mana, right type line, wrong number.
///
/// `EnterModifier::TappedUnlessAtMost`'s own doc says this in prose -- that
/// the manlands print the complement of a fast land's sentence and that "all
/// of it is one `at_most`" -- so the rule was written down and a card broke
/// it anyway. Prose beside a variant binds nobody; this is the same sentence
/// as something a run can fail on.
///
/// The two directions are not asked of the same population, for the reason
/// the mana check gives: a code bound that **disagrees** with the print is
/// wrong on any card, but a bound the code does not have at all looks
/// exactly like a clause a `Partial` card refused by name — so the missing
/// half is asked only of `Coverage::Implemented`.
pub(crate) fn check_enters_tapped_bound_matches_the_printing(
    slug: &str,
    def: &'static baylee_cards::dsl::CardDef,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    let printed = face_texts(payload);
    if printed.len() != def.faces.len() {
        return;
    }
    for (at, (text, face)) in printed.iter().zip(def.faces).enumerate() {
        let Some(want) = printed_enters_tapped_bound(text) else {
            continue;
        };
        let where_ = if def.faces.len() > 1 {
            format!("face {at} of {slug}")
        } else {
            slug.to_string()
        };
        match coded_enters_tapped_bound(face) {
            Some(have) => {
                tally.enters_tapped += 1;
                if have != want {
                    println!(
                        "{where_}: the printing's enters-tapped bound is {want:?} and the \
                         code's is {have:?}"
                    );
                    *problems += 1;
                }
            }
            None if def.coverage == baylee_cards::dsl::Coverage::Implemented => {
                println!(
                    "{where_}: the printing states an enters-tapped bound of {want:?} and the \
                     code counts nothing"
                );
                *problems += 1;
            }
            None => {}
        }
    }
}

/// The `//!` header, against the `CardDef` the file builds below it.
///
/// Name, cost and the two ids were the whole of it, and the type segment was
/// the gap: thirteen lands said `Land — SWAMP MOUNTAIN` in the constants'
/// spelling rather than the card's, two said `Land` where the card prints
/// `Legendary Land`, and nothing read any of it.
///
/// It closes a triangle rather than adding a check.
/// [`check_type_line_matches_the_printing`] holds the code against the
/// printing, so holding the header against the **code** is what makes the
/// header right about the *card*. Before that check existed this comparison
/// would only have said that a person typed the same thing twice, which is
/// precisely what Ondu Cleric did for as long as it existed — so the order
/// the two checks were written in is the reason either means anything.
///
/// The code's side is [`baylee_cards::pool::type_line`] joined with `" // "`,
/// for the same reason the printing check uses it: one renderer, and it is
/// the one the deckbuilder shows a player. A header names the faces the
/// *file* has rather than the ones Scryfall writes, so the join is exact at
/// both ends of that difference — Conqueror's Galleon prints
/// `Artifact — Vehicle // Land` and renders it, and Emeritus of Woe, the
/// adventure this pool models as one face, compares its one line and passes
/// where the printing check has to skip it.
pub(crate) fn check_header_matches_code(
    slug: &str,
    content: &str,
    def: Option<&'static baylee_cards::dsl::CardDef>,
    header_types: &mut usize,
    problems: &mut usize,
) {
    // First header line: `//! <Name> — <cost or "(no cost)"> — <types>`.
    let Some(header) = content.lines().find(|l| l.starts_with("//! ")) else {
        println!("{slug}: no header line");
        *problems += 1;
        return;
    };
    let header = &header[4..];
    // Three, and the third keeps whatever is left: a double-faced card's
    // segment is `Artifact — Vehicle // Land` and has a dash of its own.
    let mut parts = header.splitn(3, " — ");
    let (head_name, head_cost, head_types) = (
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
        parts.next().unwrap_or(""),
    );

    // The first face's name (token statics can appear before CARD, so
    // anchor on the `faces` field). MDFC headers read "Front // Back":
    // either side may headline the file's first face.
    let code_name = content
        .find("faces = &[")
        .and_then(|pos| knob(&content[pos..], "name"))
        .and_then(|v| quoted_value(v, "\""));
    if let Some(code_name) = code_name {
        let matches = head_name
            .split(" // ")
            .any(|side| side == code_name.as_str());
        if !matches {
            println!("{slug}: header name {head_name:?} != code name {code_name:?}");
            *problems += 1;
        }
    }
    // The header cost must be one of the faces' costs.
    let costs = code_costs(content);
    let head_cost_norm = if head_cost == "{0}" {
        "(no cost)"
    } else {
        head_cost
    };
    if !costs.is_empty() && !costs.iter().any(|c| c == head_cost_norm) {
        println!("{slug}: header cost {head_cost:?} matches none of the code costs {costs:?}");
        *problems += 1;
    }
    // The type segment, through the one renderer there is.
    if let Some(def) = def {
        let code_types = def
            .faces
            .iter()
            .map(baylee_cards::pool::type_line)
            .collect::<Vec<_>>()
            .join(" // ");
        *header_types += 1;
        if head_types != code_types {
            println!("{slug}: header type line {head_types:?} != code {code_types:?}");
            *problems += 1;
        }
    }
    // `knob` and never a literal `"<field>: \""`: a card file writes
    // `scryfall_id = "…"`, so the colon spelling matched nought of the 1365
    // and `code_value` was `None` for every card — which this comparison
    // answers by saying nothing, so it ran on every card and compared none.
    // A missing half is *also* reported now, because "the header and the
    // code agree" and "one of them is not there" are different answers and
    // only one of them was being given.
    for (label, field) in [("Scryfall ID", "scryfall_id"), ("Oracle ID", "oracle_id")] {
        let header_has = content
            .lines()
            .find(|l| l.starts_with("//!") && l.contains(label))
            .and_then(|l| l.split(&format!("{label}: ")).nth(1))
            .map(|v| v.split([' ', '|']).next().unwrap_or("").trim());
        let code_value = knob(content, field)
            .and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.split_once('"'))
            .map(|(id, _)| id);
        match (header_has, code_value) {
            (Some(h), Some(c)) if h != c => {
                println!("{slug}: header {label} {h:?} != code {c:?}");
                *problems += 1;
            }
            (Some(_), None) => {
                println!("{slug}: header carries a {label} and the code writes none");
                *problems += 1;
            }
            _ => {}
        }
    }
}
