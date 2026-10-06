//! `validate`: costs and mana, as printed against as coded.

use crate::{PrintingTally, knob};

/// All `mana_cost` literals in the file (one per face), normalized:
/// `{0}` and `ManaCost::ZERO` are the same thing.
pub(crate) fn code_costs(content: &str) -> Vec<String> {
    content.split("face!(").skip(1).map(face_cost).collect()
}

/// The `mana_cost` one `face!` block writes, or `(no cost)` for a face that
/// writes none.
///
/// A costless face writes no `mana_cost` line at all — `FaceDef::DEFAULT`
/// supplies `ManaCost::ZERO` and the authoring rule is never to restate a
/// default — so [`code_costs`] has to be **positional** rather than a
/// scrape of the written lines. It was a scrape with one "(no cost)"
/// appended when the counts disagreed, which put the entry on the wrong
/// face: Ishgard, the Holy See is a Town on the front of an Adventure, so
/// the first cost the file writes belongs to the *second* face, and the
/// checker reported that the printing costs nothing while the code costs
/// {3}{W}{W}. Five cards said it, all of them a land or a town in front of
/// a spell — and none of them said it until the payload lookup started
/// finding double-faced cards at all.
///
/// A block reaches to the start of the next one, so the last block reaches
/// the end of the file and would pick up a `mana_cost:` written after the
/// `faces` list closes. Measured 2026-09-09 over the 109 multi-faced files:
/// none writes one there, and both callers would be blind to it if one did —
/// the header check compares sets and the printing check reads the front
/// face.
pub(crate) fn face_cost(face: &str) -> String {
    const NONE: &str = "(no cost)";
    let Some(pos) = face.find("mana_cost = ") else {
        return NONE.to_string();
    };
    let rest = &face[pos + "mana_cost = ".len()..];
    // Either spelling of the macro. It used to read only the qualified one,
    // and the day `mana!` reached the prelude all 478 costed faces in the
    // pool read as costing nothing — a reader pinned to one spelling that
    // answers a *value* when it cannot read, rather than saying so.
    let rest = rest
        .strip_prefix("mana!(\"")
        .or_else(|| rest.strip_prefix("baylee_core::mana!(\""));
    let Some(rest) = rest else {
        // A `mana_cost:` written some third way is this function going
        // blind, not a face with no cost. Callers compare against the
        // printing, so a sentinel here would read as a disagreement about
        // the card.
        return "(unreadable)".to_string();
    };
    match rest.find('"') {
        Some(end) if &rest[..end] != "{0}" => rest[..end].to_string(),
        _ => NONE.to_string(),
    }
}

/// Compares the mandatory human-readable header against the `CardDef`
/// data — the header is the safety net against generation drift, and it
/// is only a net if something checks it (two cost fixes once shipped
/// with stale headers).
/// Compares the code's first face against the **printing** — Scryfall's own
/// payload in `data/scryfall-cache`, which is what `codegen` reads.
///
/// [`check_header_matches_code`] compares two things a person wrote, so a
/// card whose header was edited to agree with a wrong `CardDef` passes it
/// with nothing to say. Fifteen cards in the pool had done exactly that: six
/// wrong mana costs and nine wrong power/toughness pairs, every one of them
/// green. Wartime Protestors was printed `{3}{R}` 4/4 and played `{2}{R}`
/// 3/2, which is how it was reported — a card that costs one mana less than
/// the one drawn on it.
///
/// The numbers on the front face are what this one checks: a cost is what a
/// player pays, a P/T is what survives combat, and a starting loyalty is what
/// a planeswalker has to lose before it dies — all single values a hand edit
/// can silently move. [`check_card_matches_the_printing`] is the other half,
/// asked of the *compiled* card rather than of its source text, because
/// colors and keywords are sets rather than literals a `find` can lift out of
/// a file.
///
/// A card with no cached payload is skipped rather than failed, because
/// failing on its absence would turn a data-fetching problem into a build
/// failure about card rules. It now reaches all 1365 — it reached 1263 until
/// [`cached_printing`] learned the name a double-faced card is filed under —
/// and [`PrintingTally`]'s floors are what say the reach has not shrunk
/// again.
pub(crate) fn check_code_matches_the_printing(
    slug: &str,
    content: &str,
    payload: &serde_json::Value,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    // A double-faced card carries its per-face costs and P/T inside
    // `card_faces`, and its top-level `mana_cost` is the two sides joined
    // with " // " — a string no `CardDef` face ever holds.
    let front = payload.get("card_faces").and_then(|f| f.get(0));
    let printed = |key: &str| -> Option<String> {
        front
            .and_then(|f| f.get(key))
            .or_else(|| payload.get(key))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    };

    let Some(face) = content.find("faces = &[").map(|pos| &content[pos..]) else {
        return;
    };
    // Every face the printing puts a cost on, not the front one alone: an
    // MDFC's back side is a card you cast for its own printed cost, and
    // nothing here read it.
    //
    // Two tolerances, and both are about what a face's cost *means* rather
    // than about slack. The lists are compared only when they are the same
    // length, because a differing count is a modelling decision rather than
    // a typo — a split card and an adventure print two faces Scryfall's way
    // and are one `FaceDef` here. And a back face the printing gives no cost
    // is skipped **only when the code calls it a disturb face**, because
    // that is the one reason a `FaceDef` carries a cost the printing does
    // not: Ghastly Mimicry's `mana_cost` is its **disturb** cost, which
    // Scryfall writes in the oracle text and not in
    // `card_faces[1].mana_cost`. That is why this check would not have found
    // the `{5}{U}` it was built at — [`check_oracle_matches_the_printing`]
    // did, by comparing the sentence.
    //
    // The `disturb` half of that sentence was missing, and one card walked
    // through the gap. The True Scriptures is a *transformed* back — reached
    // by Sheoldred's `{4}{B}` ability, cast for nothing ever — and it was
    // written with a `{2}{B}{B}` no printing has. Nothing compared it,
    // because the tolerance covered every costless back rather than the
    // faces that earn it; and `a_back_face_with_no_printed_cost_is_never_
    // castable_from_the_hand` reads the *code's* cost, so an invented one is
    // exactly what makes a transformed back look like an MDFC's to it. The
    // cast wizard duly offered the Saga out of hand for five mana. A skipped
    // comparison is how a hand-written number gets to mean whatever it says.
    //
    // `tally.costs` is what keeps either tolerance from quietly swallowing
    // the whole check.
    let printed_costs: Vec<String> = match payload
        .get("card_faces")
        .and_then(serde_json::Value::as_array)
    {
        Some(faces) => faces
            .iter()
            .map(|f| {
                f.get("mana_cost")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            })
            .collect(),
        None => vec![
            payload
                .get("mana_cost")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
        ],
    };
    let code_costs = code_costs(face);
    // Which of those code faces says `disturb: true`, positionally — the
    // tolerance below is for that field and nothing else. Blocks run to the
    // next `face!(` and the last to the end of the file, the same reach
    // [`face_cost`] documents.
    let code_disturb: Vec<bool> = face
        .split("face!(")
        .skip(1)
        .map(|block| block.contains("disturb = true"))
        .collect();
    if printed_costs.len() == code_costs.len() {
        for (at, (printed, code)) in printed_costs.iter().zip(&code_costs).enumerate() {
            if at > 0 && printed.is_empty() && code_disturb.get(at).copied().unwrap_or(false) {
                continue;
            }
            tally.costs += 1;
            if normalise_cost(printed) != *code {
                println!(
                    "{slug}: face {at} costs {printed} in the printing and {code} in the code"
                );
                *problems += 1;
            }
        }
    }
    // A creature only, and for loyalty a planeswalker only. `power` on a
    // printing with no P/T is absent, and a face that writes neither is a
    // noncreature card the check has nothing to say about. A transforming
    // planeswalker whose *front* face has no loyalty is skipped the same
    // way: the payload's front face carries `null` there, so nothing is
    // compared against the back face's number.
    for (key, field) in [
        ("power", "power = Some("),
        ("toughness", "toughness = Some("),
        ("loyalty", "loyalty = Some("),
    ] {
        let (Some(printed), Some(code)) = (printed(key), field_number(face, field)) else {
            continue;
        };
        // `*` is not a number the code got wrong — it is a
        // characteristic-defining ability (CR 208.2), and the printing
        // names no value at all. The code still has to write one, because
        // CR 208.1 gives every creature a power and the pool's own lint
        // enforces it, so what it writes is a **base** and not a claim
        // about the printing. Comparing the two is comparing a number to a
        // sentence.
        //
        // What is worth saying instead is whether the card keeps the
        // promise that base implies. The one `Modifier` on `Layer::PtCda`
        // is `CharacteristicPT`, so a card claiming `Coverage::Implemented`
        // with a printed `*` and no such modifier is claiming something no
        // rule performs. Pyrogoyf writes it; this is the gate that keeps
        // the rest honest rather than a count that goes stale. The other
        // door is `SetPTToCount`: a `*` whose sentence holds only "as long
        // as" something is not characteristic-defining (CR 604.3a, its
        // fifth criterion) and sets power and toughness in layer 7b under a
        // condition (Gaea's Liege).
        if printed.parse::<i32>().is_err() {
            tally.defined_pt += 1;
            if knob(content, "coverage").is_some_and(|v| v.starts_with("Coverage::Implemented"))
                && !content.contains("Modifier::CharacteristicPT")
                && !content.contains("Modifier::SetPTToCount")
            {
                println!(
                    "{slug}: the printing defines {key} by an ability ({printed}) and the card \
                     claims Coverage::Implemented without a Modifier::CharacteristicPT or \
                     Modifier::SetPTToCount"
                );
                *problems += 1;
            }
            continue;
        }
        if printed != code {
            println!("{slug}: the printing has {key} {printed} and the code has {code}");
            *problems += 1;
        }
        if key == "loyalty" {
            tally.loyalty += 1;
        }
    }
}

/// What the card's mana abilities make, against what its text offers to add.
///
/// Read from "Add" to the end of the line, not from the whole text, because
/// an activation cost is written in the same symbols and `{G}, {T}: …` would
/// otherwise license a green mana the land never makes.
///
/// What a printed rules text's "add …" clauses offer, as the mana check can
/// read it.
///
/// Three answers and not two, because the check does three different things
/// with them: a card that never says "add" has a mana ability nothing printed
/// (CR 305.6 puts a basic land's mana on its type line, so an ability beside
/// it is a duplicate), a card that spells its mana in symbols can be held
/// against them, and a card that promises mana **in words** can be held
/// against nothing at all.
///
/// The third case used to be two spellings — "any color" and "any type" —
/// and Black Lotus prints neither. "Add three mana of any one color" reached
/// the symbol scan, which found no symbol, and the card was reported as
/// making five colours the printing never offers. Counted over the payload
/// cache, the printed forms of that promise are 28 and every one of them says
/// "any": "one mana of any color", "three mana of any one color", "two mana
/// in any combination of colors", "any type that a land you control could
/// produce", "any of the exiled card's colors". So the rule is the one the
/// check actually needs — a clause that says "any" and spells no symbol names
/// its mana in words — and the single form that says "any" *and* spells
/// symbols, "add X mana in any combination of {R} and/or {W}", is read rather
/// than declined, which the narrower spelling also managed by accident.
pub(crate) fn printed_mana_offered(printed: &str) -> PrintedMana {
    let mut offered: Vec<char> = Vec::new();
    let mut saw_a_clause = false;
    for (at, _) in printed.match_indices("add ") {
        let clause = printed[at..].split('\n').next().unwrap_or_default();
        let symbols: Vec<char> = clause
            .as_bytes()
            .windows(3)
            .filter(|w| w[0] == b'{' && w[2] == b'}' && b"wubrgc".contains(&w[1]))
            .map(|w| w[1] as char)
            .collect();
        if clause.contains("any") && symbols.is_empty() {
            return PrintedMana::InWords;
        }
        saw_a_clause = true;
        offered.extend(symbols);
    }
    if saw_a_clause {
        PrintedMana::Symbols(offered)
    } else {
        PrintedMana::Nothing
    }
}

/// The three answers [`printed_mana_offered`] gives.
pub(crate) enum PrintedMana {
    /// The printing never says "add".
    Nothing,
    /// The printing promises mana in words that name no symbol.
    InWords,
    /// The symbols every "add …" clause names, in printed order.
    Symbols(Vec<char>),
}

/// A clause that says "any color" names no symbol at all and is the card
/// making every color, so such a card is skipped rather than reported: the
/// code is right to claim five and the text is right to print none.
pub(crate) fn check_mana_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    printed: &str,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    use baylee_cards::dsl::{AbilityDef, ManaColor};

    let mut claimed: Vec<char> = Vec::new();
    let lists = std::iter::once(def.abilities).chain(def.faces.iter().map(|f| f.abilities));
    for ability in lists.flatten() {
        let (cost, effects) = match ability {
            AbilityDef::Activated {
                cost,
                effects,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                mana_ability: true,
                ..
            } => (cost, *effects),
            _ => continue,
        };
        // `mana_made` refuses anything a planner could not read — a second
        // effect, a board-dependent source, an amount that is not fixed —
        // and every refusal is a card this check has nothing to say about.
        let Some((made, _restricted)) = baylee_cards::dsl::mana_made(cost, effects) else {
            continue;
        };
        for color in made.colors {
            claimed.push(match color {
                ManaColor::White => 'w',
                ManaColor::Blue => 'u',
                ManaColor::Black => 'b',
                ManaColor::Red => 'r',
                ManaColor::Green => 'g',
                ManaColor::Colorless => 'c',
            });
        }
    }
    if claimed.is_empty() {
        return;
    }

    let offered = match printed_mana_offered(printed) {
        // A clause naming its mana in words offers no symbol to hold the
        // code against, and a comparison against nothing reports every
        // colour the card makes.
        PrintedMana::InWords => return,
        PrintedMana::Nothing => {
            println!(
                "{slug}: the code has a mana ability and the printed text never says \"add\"; \
                 a land's intrinsic mana comes off the type line (CR 305.6) and needs no ability"
            );
            *problems += 1;
            return;
        }
        PrintedMana::Symbols(offered) => offered,
    };
    tally.mana += 1;
    for color in claimed {
        if !offered.contains(&color) {
            println!(
                "{slug}: the code taps for {{{}}} and the printing never offers it",
                color.to_ascii_uppercase()
            );
            *problems += 1;
        }
    }
}

/// Each printed activation cost, against the cost the code makes a player pay.
///
/// [`check_mana_matches_the_printing`] reads what a mana ability *produces*
/// and holds it against the card's "add …" clause; nothing asked the other
/// half of the same sentence. A land whose ability is free where the card
/// charges for it is a land that reads as correct from every side — the
/// `//! Oracle:` header is the printing's own words, the effects are right,
/// and only the price is wrong. Mystic Gate and Fetid Heath each charged
/// `{1}` for what their printing sells for `{W/U}` and `{W/B}`: strictly
/// cheaper, and colourless where the card demands a colour.
///
/// The comparison is made in the **engine's** vocabulary rather than in text:
/// the printed prefix is parsed into a [`baylee_core::mana::ManaCost`] and
/// held against the `ManaCost` on the ability, so a difference in spelling is
/// not a finding and a difference in price always is. A printed cost the
/// parser cannot read is skipped and counted nowhere, which is what keeps the
/// floor below honest.
///
/// It asks only [`Coverage::Implemented`] cards, and that line is the whole
/// difference between a check and a nuisance. A `Partial` card has said in
/// prose which ability it does not have, and a missing ability looks from
/// here exactly like a mispriced one: Kenrith, Lotleth Troll, Urza and
/// Yawgmoth are each refusing one printed ability by name in a
/// `// NOT SUPPORTED:` comment, and every one of them was reported before
/// this clause existed. Finishing any of them puts that card back in the
/// population, which is the same bargain
/// `lints::every_layer_in_the_pool_is_the_one_its_modifier_derives` makes.
pub(crate) fn check_activation_cost_matches_the_printing(
    slug: &str,
    def: &baylee_cards::dsl::CardDef,
    printed: &str,
    tally: &mut PrintingTally,
    problems: &mut usize,
) {
    use baylee_cards::dsl::AbilityDef;

    if !def.is_implemented() {
        return;
    }

    // Every mana price the code would make a player pay, lowercased because
    // the printed text this is held against already is.
    let mut paid: Vec<String> = Vec::new();
    let lists = std::iter::once(def.abilities).chain(def.faces.iter().map(|f| f.abilities));
    for ability in lists.flatten() {
        let (AbilityDef::Activated { cost, .. } | AbilityDef::ActivatedConditional { cost, .. }) =
            ability
        else {
            continue;
        };
        // A free activation renders as the empty string, which in a list of
        // prices reads as a missing entry rather than as a price of nothing.
        let mana = cost.mana.to_string().to_ascii_lowercase();
        paid.push(if mana.is_empty() {
            "{0}".to_string()
        } else {
            mana
        });
    }
    if paid.is_empty() {
        return;
    }

    for line in printed.lines() {
        let Some(prefix) = printed_activation_cost(line) else {
            continue;
        };
        // `{T}:` alone is the commonest activation cost there is and names no
        // mana, so it says nothing about a price and would only inflate the
        // count below.
        if !prefix.contains(['w', 'u', 'b', 'r', 'g', 'c'])
            && !prefix.contains(|c: char| c.is_ascii_digit())
        {
            continue;
        }
        let Ok(want) = prefix
            .to_ascii_uppercase()
            .parse::<baylee_core::mana::ManaCost>()
        else {
            continue;
        };
        let want = want.to_string().to_ascii_lowercase();
        tally.activation_costs += 1;
        if !paid.contains(&want) {
            println!(
                "{slug}: the printing charges {want} to activate and no ability in the code \
                 costs it (the code pays {})",
                paid.join(", ")
            );
            *problems += 1;
        }
    }
}

/// The mana a printed line charges before its first `:`, or `None` when the
/// line is not an activated ability at all.
///
/// A cost prefix is a run of `{…}` symbols at the start of the line, and what
/// follows it is either the `:` or another cost clause the code spells as a
/// [`baylee_cards::dsl::CostPart`] rather than as mana ("`, {T},
/// Sacrifice a creature:`"). Anything else — a sentence that merely opens
/// with a symbol, a reminder clause in parentheses — is not a price.
pub(crate) fn printed_activation_cost(line: &str) -> Option<String> {
    let line = line.trim();
    let mut rest = line;
    let mut mana = String::new();
    while let Some(tail) = rest.strip_prefix('{') {
        let end = tail.find('}')?;
        mana.push('{');
        mana.push_str(&tail[..end]);
        mana.push('}');
        rest = &tail[end + 1..];
    }
    if mana.is_empty() {
        return None;
    }
    // The prefix has to reach a `:` to be a cost, and it may pass through
    // further cost clauses to get there — but not through the end of the
    // sentence, which is what tells "`{2}, {T}: …`" from a rules line that
    // happens to start with a symbol.
    let head = rest.split('.').next().unwrap_or(rest);
    (head.starts_with(':') || (head.starts_with(',') && head.contains(':'))).then_some(mana)
}

/// Scryfall's spelling of a costless card is an empty string; the pool's is
/// the one [`code_costs`] already normalises everything else to.
pub(crate) fn normalise_cost(printed: &str) -> String {
    if printed.is_empty() || printed == "{0}" {
        "(no cost)".to_string()
    } else {
        printed.to_string()
    }
}

/// The first `field` number in `face`, as it is written.
pub(crate) fn field_number(face: &str, field: &str) -> Option<String> {
    let rest = &face[face.find(field)? + field.len()..];
    let end = rest.find(')')?;
    Some(rest[..end].to_string())
}
