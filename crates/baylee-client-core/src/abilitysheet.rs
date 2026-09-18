//! The ability sheet's arithmetic: which rows a page shows, which row a digit
//! names, and what pressing one does.
//!
//! The sheet itself is a piece of parchment anchored to the permanent it
//! belongs to, and everything about *drawing* it lives in the renderer. What
//! is here is the part that has answers rather than pixels, and the part a
//! test can reach without a window.
//!
//! # Why a page holds nine
//!
//! Because the keycap on a row is the key that sends it, and there are nine
//! digits that are not zero. Ten rows would need a key that is not a digit
//! for the tenth, and a player who has learned "the number beside it" would
//! meet an exception on the one card that has ten things to do.
//!
//! That card is real but rare: a land under a Chromatic Lantern is granted a
//! mana ability on top of whatever it prints, and a permanent can collect
//! several grants. So the tenth row is not an ability at all — it is the
//! pager, it carries [`PAGER`] instead of a digit, and it is drawn only when
//! there is a page to turn to.

use crate::card_face::TextBlock;
use core::ops::Range;

/// How many abilities one page of the sheet lists.
///
/// Nine, for the reason in the module header: a row is sent by the digit
/// drawn on it, and `0` is spoken for.
pub const PAGE: usize = 9;

/// The key the pager row carries, when there is one.
///
/// Zero rather than a letter or an arrow, because it is the one key on the
/// same row of the keyboard as the nine digits above it and the one digit
/// that names no row. A player who has read "press 4" has already found it.
pub const PAGER: char = '0';

/// How many pages a list of `len` abilities needs.
///
/// At least one, so an empty list is a page with nothing on it rather than no
/// page at all — a sheet that exists and lists nothing is a bug the renderer
/// should be able to see, and division that answers zero hides it.
#[must_use]
pub fn pages(len: usize) -> usize {
    len.div_ceil(PAGE).max(1)
}

/// Whether the list is long enough to need the pager row.
#[must_use]
pub fn paged(len: usize) -> bool {
    len > PAGE
}

/// Which rows of the whole list `page` shows.
///
/// Empty when the page is past the end, which is what makes this safe to call
/// with a page held from a frame ago: the engine withdraws abilities between
/// frames, and a sheet standing on page two of a list that has shrunk to four
/// draws nothing rather than slicing out of bounds.
#[must_use]
pub fn rows(len: usize, page: usize) -> Range<usize> {
    let start = page.saturating_mul(PAGE).min(len);
    start..(start + PAGE).min(len)
}

/// The page `page` becomes after the pager is pressed, wrapping at the end.
///
/// Wrapping rather than stopping: the pager is one key and there is no second
/// one for going back, so a player who overshoots gets there by pressing it
/// again. With one page it answers zero and pressing it changes nothing.
#[must_use]
pub fn turn(len: usize, page: usize) -> usize {
    let pages = pages(len);
    if pages <= 1 {
        return 0;
    }
    (page + 1) % pages
}

/// The page that still exists nearest to `page`.
///
/// The sheet stands open across frames and the list under it is rebuilt from
/// `LegalActions` every time, so it can shrink under a page that was valid
/// when it was turned to.
#[must_use]
pub fn clamp(len: usize, page: usize) -> usize {
    page.min(pages(len) - 1)
}

/// Which option in the whole list the digit `digit` names on `page`.
///
/// `None` for a digit that names no row — `0`, or a `7` on a page with four
/// rows on it. Deliberately not "the nearest row": a digit that missed is a
/// digit the player meant for something else, and answering it with a
/// neighbour would activate an ability nobody asked for.
#[must_use]
pub fn option_of(len: usize, page: usize, digit: char) -> Option<usize> {
    let place = digit.to_digit(10)?;
    if place == 0 {
        return None;
    }
    let row = rows(len, page);
    let at = row.start + (place as usize - 1);
    (at < row.end).then_some(at)
}

/// Where the cursor lands when the sheet is walked **up or down**.
///
/// The whole list, wrapping at both ends, which is what it has always been:
/// the pips and the written rows are one column of options as far as this is
/// concerned, and the drawn page follows the cursor rather than the other way
/// round. `step` is `+1` for down and `-1` for up.
///
/// An empty list answers zero rather than dividing by it. The sheet's own
/// reader has already closed a list with nothing to choose from by the time
/// this is asked, and a cursor is not the place to find that out.
#[must_use]
pub fn step_down(len: usize, pick: usize, step: i32) -> usize {
    if len == 0 {
        return 0;
    }
    let len = i32::try_from(len).unwrap_or(i32::MAX);
    let at = i32::try_from(pick).unwrap_or(0);
    usize::try_from((at + step).rem_euclid(len)).unwrap_or(0)
}

/// Where the cursor lands when the sheet is walked **left or right**.
///
/// The mana header is a row of pips laid side by side, and it is the one
/// thing on this sheet that is horizontal — so the two horizontal keys are
/// what walks it, and they reach it from wherever the cursor happens to be.
/// The owner asked for exactly that: *"bei den Mana Symbolen, die kann man
/// mit A und D navigieren. Wenn man A oder D navigiert, switcht es sofort zur
/// Mana Zeile"*.
///
/// A press from a written row therefore does not step at all — it **arrives**,
/// at the end of the strip the press was travelling towards: rightwards enters
/// at the first pip, leftwards at the last. A fixed end would make one of the
/// two directions cross the whole header before it did anything, and entering
/// at "the pip above the row" would be answering a question about geometry
/// that a centred header of a different width cannot answer.
///
/// A sheet with no header has nothing horizontal on it, and there the two keys
/// keep doing what they always did: stepping the list like [`step_down`],
/// because a player holding one of them to walk a list should not have to know
/// which permanents have pips.
#[must_use]
pub fn step_along(len: usize, pips: usize, pick: usize, step: i32) -> usize {
    if pips == 0 {
        return step_down(len, pick, step);
    }
    if pick >= pips {
        return if step >= 0 { 0 } else { pips - 1 };
    }
    step_down(pips, pick, step)
}

/// What pressing a row's digit does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Press {
    /// Sends the ability at once — the CR 605.1 one-tap exemption, and the
    /// cost that is paid out of the card itself and given back at untap.
    Send,
    /// Arms the row: the keycap goes gilt, the card goes gilt, and the same
    /// digit again sends it. There is no undo in the engine, so anything with
    /// a cost is asked for twice.
    Arm,
    /// The same digit on a row that is already armed.
    Fire,
}

/// What the digit on a row does, given what that ability costs and whether
/// this row is the one already armed.
///
/// `one_tap` is the two exemptions rolled together, because they arrive at the
/// same answer from different directions: a mana ability never uses the stack
/// (CR 605.1), and an ability whose whole cost is tapping its own permanent
/// costs a turn of that permanent and nothing else. Both are cheaper to undo
/// by playing on than to confirm.
#[must_use]
pub const fn press(one_tap: bool, armed: bool) -> Press {
    if armed {
        Press::Fire
    } else if one_tap {
        Press::Send
    } else {
        Press::Arm
    }
}

/// How far into a sentence a cost prefix may begin and still be one.
///
/// A printed activation cost is short — `{2}{B}, {T}, Sacrifice this` is
/// already at the long end of the pool — and a colon further in than this
/// belongs to the effect rather than to a cost: a sentence that grants an
/// ability quotes one, and cutting there would leave the row saying what the
/// *granted* ability does instead of what this one does.
const COST_PREFIX: usize = 48;

/// A printed line split where a cost *would* end: `(prefix, rest)`.
///
/// A guess, and named as one. It is the only reading available to a caller
/// holding a sentence and nothing else, and it is wrong wherever a sentence
/// quotes an ability: `Lands you control have "{T}: Add one mana of any
/// color."` splits at the colon **inside** the quotation, and neither half is
/// a thing to draw. So nothing acts on it alone — [`effect`] cuts only where
/// the caller says a cost was drawn, and [`loyalty_split`] only where the
/// prefix parses as a loyalty cost, which that sentence's does not.
fn prefix_of(line: &str) -> Option<(&str, &str)> {
    let at = line.find(": ")?;
    if at >= COST_PREFIX {
        return None;
    }
    let rest = line[at + 2..].trim();
    if rest.is_empty() {
        return None;
    }
    Some((&line[..at], rest))
}

/// The loyalty badge a printed line **begins** with, if it begins with one.
///
/// A planeswalker's ability is printed `+2: Look at the top card…`, and that
/// cost is a shape rather than a word: a line that sets it as letters loses
/// the one mark a player recognises a walker's ability by. So it comes off
/// the front of the sentence and is drawn as the initial the card draws it
/// as, and [`effect`] takes the rest.
///
/// `None` for everything else, which is nearly everything. The narrowness is
/// [`manapip::printed_loyalty`]'s, and the prefix asked about is exactly the
/// one [`effect`] is then handed, so the two answers cannot disagree.
///
/// [`manapip::printed_loyalty`]: crate::manapip::printed_loyalty
#[must_use]
pub fn loyalty_initial(blocks: &[TextBlock]) -> Option<crate::manapip::Loyalty> {
    loyalty_split(blocks).map(|(_, badge)| badge)
}

/// The same reading, as the text the badge was drawn *from*.
///
/// [`effect`] cuts only what its caller drew, and on the stack the thing
/// drawn is this badge — so the caller has to be able to name the prefix it
/// took, rather than assert that it took one. The two come out of one split
/// for that reason: a reader that found the badge here and the prefix
/// somewhere else could drop a different span than it drew.
#[must_use]
pub fn loyalty_prefix(blocks: &[TextBlock]) -> Option<&str> {
    loyalty_split(blocks).map(|(prefix, _)| prefix)
}

/// The one reading behind [`loyalty_initial`] and [`loyalty_prefix`].
fn loyalty_split(blocks: &[TextBlock]) -> Option<(&str, crate::manapip::Loyalty)> {
    let TextBlock::Rules(first) = blocks.first()? else {
        return None;
    };
    let (prefix, _) = prefix_of(first)?;
    Some((prefix, crate::manapip::printed_loyalty(prefix)?))
}

/// The half of a printed ability that says what it *does*.
///
/// A row of the sheet draws the cost on the right as pips, so the sentence on
/// the left must not repeat it: the printed text of `{2}, {T}: Draw a card.`
/// is the whole ability, and a row reading "{2}, {T}: Draw a card" beside a
/// `{2}, {T}` column says the same thing twice in one line.
///
/// **A row cuts only what it drew.** `drawn` is the cost the caller put in a
/// column of its own — the sheet's pips, the stack's loyalty badge — and with
/// `None` the sentence is returned whole, because a row that dropped a prefix
/// it drew no badge for would lose it outright. That is the rule and not a
/// precaution: a sentence granting an ability quotes a cost, so
/// `Lands you control have "{T}: Add one mana of any color."` reads as a
/// prefix and an effect to any reader that only looks, and cutting it leaves
/// a row saying `Add one mana of any color."` about a permanent that does
/// nothing of the kind. The Chromatic Lantern's own row drew exactly that,
/// in every language, until this argument existed.
///
/// The string is evidence and is deliberately **not compared** to what the
/// line prints. The sheet's column is built from the ability's [`Cost`] and
/// is translated — `{T}, Sacrifice this` where the card prints
/// `{T}, Sacrifice this artifact` — so equality would refuse nearly every
/// row it was meant to allow. What the caller is asserting by passing it is
/// that the cost is already drawn *somewhere else on this row*, which is the
/// whole of what licenses the cut.
///
/// Only the first block is ever cut, and only at a colon near its start — the
/// reminder text after it is a different block and never carries the cost.
/// Everything else is returned untouched, which is what a triggered ability,
/// a keyword and a granted ability all want.
///
/// [`Cost`]: baylee_cards_dsl::Cost
#[must_use]
pub fn effect(blocks: Vec<TextBlock>, drawn: Option<&str>) -> Vec<TextBlock> {
    let mut blocks = blocks;
    if drawn.is_none() {
        return blocks;
    }
    let Some(TextBlock::Rules(first)) = blocks.first_mut() else {
        return blocks;
    };
    let Some((_, rest)) = prefix_of(first) else {
        return blocks;
    };
    let rest = rest.to_string();
    *first = rest;
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> TextBlock {
        TextBlock::Rules(text.to_string())
    }

    #[test]
    fn a_row_says_what_an_ability_does_and_not_what_it_costs_twice() {
        // The cost column carries `{2}, {T}`; the sentence must not.
        assert_eq!(
            effect(vec![rules("{2}, {T}: Draw a card.")], Some("{2}, {T}")),
            vec![rules("Draw a card.")]
        );
        // A trigger has no cost, so there is no column and nothing to cut —
        // and the row keeps every word of itself for that reason rather than
        // because the sentence happens to have no colon in it.
        let trigger = "Whenever this creature attacks, draw a card.";
        assert_eq!(effect(vec![rules(trigger)], None), vec![rules(trigger)]);
        // The reminder is its own block and is never the one cut.
        assert_eq!(
            effect(
                vec![
                    rules("Cycling {2}"),
                    TextBlock::Reminder("{2}, Discard this card: Draw a card.".into()),
                ],
                Some("{2}")
            ),
            vec![
                rules("Cycling {2}"),
                TextBlock::Reminder("{2}, Discard this card: Draw a card.".into()),
            ]
        );
    }

    /// The defect this argument exists for, in the two languages it was
    /// measured in.
    ///
    /// A sentence that grants an ability quotes a cost, so it reads as
    /// `prefix: effect` to anything that only looks — and the prefix is half
    /// a clause about *other* permanents while the effect keeps the quotation
    /// mark that closed it. Every row whose cost column is empty is safe from
    /// it by construction, which is every granted row, every trigger and
    /// every static.
    #[test]
    fn a_sentence_that_grants_an_ability_is_never_cut_at_the_quoted_cost() {
        for granting in [
            "Lands you control have \"{T}: Add one mana of any color.\"",
            "L\u{e4}nder, die du kontrollierst, haben \u{201e}{T}: \
             Erzeuge ein Mana einer beliebigen Farbe.\"",
        ] {
            // What the row would have drawn, had it cut: the tail of the
            // quotation, with the closing mark still on it.
            let cut = effect(vec![rules(granting)], Some("{T}"));
            assert_ne!(cut, vec![rules(granting)], "the guess does split it");
            // And what it draws, because a static ability has no cost column.
            assert_eq!(effect(vec![rules(granting)], None), vec![rules(granting)]);
        }
    }

    #[test]
    fn a_colon_deep_in_a_sentence_is_not_a_cost() {
        let quoted = "Target creature gains an ability until end of turn: \
                      \"{T}: Add {G}.\"";
        assert_eq!(
            effect(vec![rules(quoted)], Some("{2}")),
            vec![rules(quoted)]
        );
        // Nothing after the colon is nothing to say, so the row keeps the
        // whole line rather than going blank.
        assert_eq!(
            effect(vec![rules("{T}: ")], Some("{T}")),
            vec![rules("{T}: ")]
        );
        assert!(effect(Vec::new(), Some("{T}")).is_empty());
    }

    /// A walker's line comes apart in two, with nothing said twice and
    /// nothing lost: the badge off the front, the sentence out of [`effect`].
    #[test]
    fn a_walkers_line_gives_up_its_badge_before_it_is_printed() {
        let line = vec![rules(
            "+2: Look at the top card of target player's library.",
        )];
        let badge = loyalty_initial(&line).expect("a badge");
        assert_eq!(badge.tick, crate::manapip::Tick::Up);
        assert_eq!(badge.caption(), "2");
        // The prefix the badge was read from is the span the sentence gives
        // up, which is what lets the stack cut exactly what it drew.
        let drawn = loyalty_prefix(&line).expect("a prefix").to_string();
        assert_eq!(drawn, "+2");
        assert_eq!(
            effect(line, Some(&drawn)),
            vec![rules("Look at the top card of target player's library.")]
        );
        // The printed minus, which is what an oracle line actually carries.
        let down = loyalty_initial(&[rules("\u{2212}1: Draw a card.")]).expect("a badge");
        assert_eq!(down.tick, crate::manapip::Tick::Down);
        assert_eq!(down.caption(), "1");
        // Everything that is not a walker's cost keeps its whole prefix, and
        // a reminder block is never the one read.
        for other in [
            "{2}, {T}: Draw a card.",
            "Whenever this creature attacks, draw a card.",
            "Cycling {2}",
            "Target creature gains an ability until end of turn: \"{T}: Add {G}.\"",
        ] {
            assert!(loyalty_initial(&[rules(other)]).is_none(), "{other}");
        }
        assert!(loyalty_initial(&[TextBlock::Reminder("+2: Draw a card.".into())]).is_none());
        assert!(loyalty_initial(&[]).is_none());
    }

    #[test]
    fn a_short_list_is_one_page_with_no_pager() {
        assert_eq!(pages(0), 1, "an empty sheet is still a sheet");
        assert_eq!(pages(1), 1);
        assert_eq!(pages(PAGE), 1, "nine fills a page exactly");
        assert!(!paged(PAGE));
        assert_eq!(rows(4, 0), 0..4);
        assert_eq!(turn(4, 0), 0, "nowhere to turn to");
    }

    #[test]
    fn the_tenth_ability_is_what_makes_a_second_page() {
        assert_eq!(pages(PAGE + 1), 2);
        assert!(paged(PAGE + 1));
        assert_eq!(rows(10, 0), 0..9);
        assert_eq!(rows(10, 1), 9..10);
        assert_eq!(turn(10, 0), 1);
        assert_eq!(turn(10, 1), 0, "and back round");
    }

    #[test]
    fn a_page_past_the_end_shows_nothing_rather_than_panicking() {
        assert!(rows(4, 1).is_empty());
        assert!(rows(0, 0).is_empty());
        assert_eq!(clamp(4, 3), 0, "the list shrank under the page");
        assert_eq!(clamp(20, 1), 1, "this one still exists");
    }

    #[test]
    fn a_digit_names_the_row_it_is_drawn_on_and_no_other() {
        assert_eq!(option_of(4, 0, '1'), Some(0));
        assert_eq!(option_of(4, 0, '4'), Some(3));
        assert_eq!(option_of(4, 0, '5'), None, "past the last row");
        assert_eq!(option_of(4, 0, '0'), None, "the pager names no ability");
        assert_eq!(option_of(4, 0, 'x'), None);
        // The second page counts from one again: the digits are positions on
        // the sheet, not indices into the list behind it.
        assert_eq!(option_of(12, 1, '1'), Some(9));
        assert_eq!(option_of(12, 1, '3'), Some(11));
        assert_eq!(option_of(12, 1, '4'), None);
    }

    /// Up and down walk the whole sheet, header included, and wrap.
    #[test]
    fn the_vertical_keys_walk_one_column_of_everything_there_is() {
        // Six options: five pips and a written row (Harabaz Druid under a
        // Great Divide Guide).
        assert_eq!(step_down(6, 0, 1), 1);
        assert_eq!(step_down(6, 4, 1), 5, "off the last pip onto the sentence");
        assert_eq!(step_down(6, 5, 1), 0, "and round");
        assert_eq!(step_down(6, 0, -1), 5, "the other way round");
        assert_eq!(step_down(0, 0, 1), 0, "an empty sheet has one place to be");
    }

    /// Left and right belong to the pip strip, and reach it from anywhere.
    #[test]
    fn the_horizontal_keys_are_the_mana_row_and_jump_to_it() {
        // Five pips, one written row.
        assert_eq!(step_along(6, 5, 0, 1), 1, "along the header");
        assert_eq!(step_along(6, 5, 4, 1), 0, "and round the header alone");
        assert_eq!(step_along(6, 5, 0, -1), 4);
        // From the sentence: one press arrives, at the end it was heading for.
        assert_eq!(step_along(6, 5, 5, 1), 0, "rightwards enters at the first");
        assert_eq!(step_along(6, 5, 5, -1), 4, "leftwards at the last");
        // A sheet with no header: the two keys step the list, as before.
        assert_eq!(step_along(4, 0, 0, 1), 1);
        assert_eq!(step_along(4, 0, 0, -1), 3);
        // A bubble is all header, so there is nothing to jump *from*.
        assert_eq!(step_along(5, 5, 4, 1), 0);
    }

    #[test]
    fn a_cost_is_asked_for_twice_and_a_tap_is_not() {
        assert_eq!(press(true, false), Press::Send);
        assert_eq!(press(false, false), Press::Arm);
        assert_eq!(press(false, true), Press::Fire);
        // A one-tap ability cannot be armed, so the combination should never
        // arise — and if it does, sending what is armed is the honest answer.
        assert_eq!(press(true, true), Press::Fire);
    }
}
