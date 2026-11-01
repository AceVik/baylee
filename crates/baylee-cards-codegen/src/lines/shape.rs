//! Shape of a printed Oracle ability.

use super::{
    LineShape, a_numbered_tapping_keyword, ability_colon, granted_crew_number,
    reminder_spells_out_an_ability, without_reminder,
};

/// The shape of one printed line.
pub fn line_shape(line: &str) -> LineShape {
    let printed = line;
    let line = without_reminder(line);
    let line = line.as_str();
    // The trigger word is read **before** the colon, and putting it after
    // was this function's own first bug: most triggered abilities print no
    // colon at all ("When this creature enters, draw a card"), so a reader
    // that split on one first sent all of them to `Other` and reported 125
    // cards as misaligned. The pool was right and the heuristic was wrong,
    // which is the failure mode a measurement like this exists to have
    // early rather than after a table is built on it.
    let lower = line.to_lowercase();
    // "As this creature enters" is a replacement rather than a trigger by
    // the rules (CR 614), and every card that prints it writes it as
    // `triggered!` here — so for the purpose of finding which sentence an
    // ability came from it is a third trigger word, not a fourth shape.
    //
    // The word is not only that sentence's: "As long as …" opens a
    // *static* ability, and read as a trigger it left The World Tree's
    // conditional grant without its sentence once the grant was built. It
    // goes on to the colon test below, where a quoted ability makes it a
    // grant.
    if lower.starts_with("when")
        || lower.starts_with("at ")
        || (lower.starts_with("as ") && !lower.starts_with("as long as "))
        || lower.starts_with("ward ")
        || lower.starts_with("ward—")
    {
        // A trigger on tapping for mana that adds mana and targets nothing
        // is a mana ability (CR 605.1b) and never a stack entry: Badgermole
        // Cub's "Whenever you tap a creature for mana, add an additional
        // {G}." Forbidden Orchard's, which targets an opponent, stays a
        // trigger.
        if lower.contains(" for mana, add ") && !lower.contains("target") {
            return LineShape::Mana;
        }
        return LineShape::Triggered;
    }
    // A chapter's separator is an em dash, not a colon.
    if line
        .split_once(" — ")
        .is_some_and(|(head, _)| !head.is_empty() && head.chars().all(|c| "IVX, ".contains(c)))
    {
        return LineShape::Chapter;
    }
    let Some(colon) = ability_colon(line) else {
        // Every colon this line prints stands inside quotation marks, so
        // the line defines no ability of its own — see [`ability_colon`] —
        // and only the ability that *grants* the quoted one may claim it.
        // Said here rather than left to fall through, because the branch
        // below reads a *keyword* line and a short grant fits it: `Lands
        // have "{T}: Add {C}."` is 26 characters, ends in a quotation mark
        // rather than a full stop, and carries a `{`. The pool's shortest
        // grant is 46 characters and only that length stood between this
        // and the same defect one branch over.
        if line.contains(':') || granted_crew_number(line).is_some() {
            return LineShape::Grant;
        }
        // A keyword ability with a cost is a printed sentence too, and it
        // is the *right* sentence for the ability it compiles to — cycling
        // and equip are `AbilityDef::Activated` here, and "Cycling {B}" is
        // what a player reads when one of them goes on the stack. It has no
        // colon because the card put the cost and the effect in a keyword
        // instead of side by side. An em-dash keyword may put a full stop
        // after its prose cost before spelling the ability out in reminder
        // text: `Equip—Pay 3 life. (Pay 3 life: …)`. `keyword_cost` keeps
        // that exception anchored to one printed keyword, not any sentence.
        let activated_reminder = reminder_spells_out_an_ability(printed);
        let punctuated_keyword = activated_reminder && super::keyword_cost(line).is_some();
        return if punctuated_keyword
            || (line.len() <= 40
                && !line.ends_with('.')
                && (line.contains('{') || activated_reminder || a_numbered_tapping_keyword(line)))
        {
            LineShape::Activated
        } else {
            LineShape::Other
        };
    };
    let (head, body) = line.split_at(colon);
    let body = body[':'.len_utf8()..].trim();
    // A line that makes mana is its own shape, and the reasons are on
    // [`LineShape::Mana`] — including why it has to be said here as well as
    // in `ability_shape`.
    let lower_body = body.to_lowercase();
    // A later sentence may still add mana, but every immediate sentence
    // belongs to this activation. A direct target there keeps it on the
    // stack even when "Add" is the first instruction. Targets and library
    // movement in a later delayed trigger belong to that separate ability.
    let immediate = lower_body.split(". ").take_while(|clause| {
        !clause.starts_with("when")
            && !clause.starts_with("at ")
            && !clause.contains(" at the beginning of ")
    });
    let targets_directly = immediate
        .clone()
        .flat_map(|clause| clause.split(|c: char| !c.is_ascii_alphabetic()))
        .any(|word| matches!(word, "target" | "targets"));
    let makes = |s: &str| s.starts_with("add ") && (s.contains('{') || s.contains("mana"));
    let adds_first = makes(&lower_body)
        || (lower_body.starts_with("for each color among ")
            && lower_body
                .split_once(", ")
                .is_some_and(|(_, effect)| makes(effect)));
    let adds_later = immediate.clone().skip(1).any(makes);
    let moves_library = immediate
        .flat_map(|clause| clause.split(|c: char| !c.is_ascii_alphabetic()))
        .any(|word| matches!(word, "draw" | "draws" | "mill" | "mills"));
    if (adds_first || adds_later) && !targets_directly && !moves_library {
        return LineShape::Mana;
    }
    // A loyalty cost is the whole of what precedes the colon, and the minus
    // a card prints is U+2212, not a hyphen.
    if !head.is_empty()
        && head
            .chars()
            .all(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == '\u{2212}' || c == 'X')
    {
        return LineShape::Loyalty;
    }
    // An activation cost is mana, a tap, a sacrifice — never a sentence, so
    // the head is length-bounded rather than merely non-empty. The bound is
    // 120 and not 60 because a cost may be several clauses of prose:
    // Recurring Nightmare pays "Sacrifice a creature, Return this
    // enchantment to its owner's hand", which is 76 characters and was
    // being read as a card with no activated ability at all.
    if head.len() <= 120 {
        LineShape::Activated
    } else {
        LineShape::Other
    }
}
