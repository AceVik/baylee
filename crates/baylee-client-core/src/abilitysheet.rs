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

/// A row's printed sentence, cut into the cost its column draws and the words
/// it draws beside it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Cut {
    /// The cost as the card prints it, in the player's language when the
    /// sentence is theirs: `{1}, {T}, opfere dieses Artefakt`. A walker's is
    /// its badge as a `{L…}` token. `None` when the sentence is drawn whole
    /// and the row has no column.
    pub head: Option<String>,
    /// What the ability does: the sentence after its cost, then any reminder.
    pub blocks: Vec<TextBlock>,
}

/// Where a printed sentence gives up its cost, if the ability costs it.
///
/// The owner's rule for the sheet is the card's own words — *„nicht custom
/// texte für abilities verwenden, sondern die echten texte von skryfall"* —
/// and it holds for the cost as well: the column is the head of the printed
/// sentence ([`baylee_cardtext::split_cost`]) and never a wording of the
/// ability's [`Cost`] this client composes.
///
/// `key` is what the ability costs, as the engine has it: its mana and its
/// tap or untap symbol in Scryfall's spelling (`{2}, {T}`), or a walker's
/// badge (`{L+2}`). It **licenses** the cut and is not drawn. A head printing
/// a symbol the ability does not cost is some other sentence's, and the
/// sentence is drawn whole rather than put a wrong cost in the column
/// ([`baylee_cardtext::licensed`]). A head in words alone is licensed only
/// by a key with no symbols, which is every `Sacrifice a Forest`: an ability
/// that costs mana or a tap prints it as a symbol, so a head with none beside
/// a key with some is a colon inside the effect.
///
/// Drawn whole, with no column:
///
/// - a sentence with no cost colon ahead of its reminder or quotation, which
///   is every keyword line (`Equip {1}`) and every sentence that grants an
///   ability in quotes (`Lands you control have "{T}: Add one mana…"`);
/// - a colon with nothing after it, because a row with no words is worse than
///   a row that says its cost twice;
/// - a walker's badge that is not the one the ability costs, and a mana head
///   on a walker's line.
///
/// Only the first block is ever cut. The reminder after it is a block of its
/// own and never carries the cost.
///
/// [`Cost`]: baylee_cards_dsl::Cost
#[must_use]
pub fn cut(blocks: Vec<TextBlock>, key: &str) -> Cut {
    let mut blocks = blocks;
    let Some(TextBlock::Rules(first)) = blocks.first_mut() else {
        return Cut { head: None, blocks };
    };
    let Some(split) = baylee_cardtext::split_cost(first) else {
        return Cut { head: None, blocks };
    };
    if split.body.is_empty() {
        return Cut { head: None, blocks };
    }
    let badge = key_badge(key);
    let head = match crate::manapip::printed_loyalty(split.head) {
        Some(printed) => (badge == Some(printed)).then(|| key.to_string()),
        None => (badge.is_none() && licensed(split.head, key)).then(|| split.head.to_string()),
    };
    if head.is_some() {
        *first = split.body.to_string();
    }
    Cut { head, blocks }
}

/// [`baylee_cardtext::licensed`], and a head in words only for a key in words.
fn licensed(head: &str, key: &str) -> bool {
    let spelled = baylee_cardtext::symbols(head).is_empty();
    baylee_cardtext::licensed(head, key) && (!spelled || baylee_cardtext::symbols(key).is_empty())
}

/// The badge a `{L…}` key names, or `None` for a key that is a cost in mana.
fn key_badge(key: &str) -> Option<crate::manapip::Loyalty> {
    let body = key.strip_prefix('{')?.strip_suffix('}')?;
    match crate::manapip::symbol(body)? {
        crate::manapip::Pip::Loyalty(badge) => Some(badge),
        _ => None,
    }
}

/// A walker's line on the stack, as its badge and the words after it.
///
/// The stack has no key to license a cut with — it draws what resolves, not
/// what was offered — so it takes only the one head that licenses itself: a
/// badge ([`crate::manapip::printed_loyalty`]), whose strictness refuses
/// `{2}{B}, {T}` and `Sacrifice a land` alike. Everything else keeps its
/// whole line: a cost paid in mana is already a row of marks inside the
/// sentence, and a trigger has no cost at all.
#[must_use]
pub fn loyalty_cut(blocks: Vec<TextBlock>) -> (Option<crate::manapip::Loyalty>, Vec<TextBlock>) {
    let mut blocks = blocks;
    let Some(TextBlock::Rules(first)) = blocks.first_mut() else {
        return (None, blocks);
    };
    let Some(split) = baylee_cardtext::split_cost(first).filter(|split| !split.body.is_empty())
    else {
        return (None, blocks);
    };
    let Some(badge) = crate::manapip::printed_loyalty(split.head) else {
        return (None, blocks);
    };
    *first = split.body.to_string();
    (Some(badge), blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(text: &str) -> TextBlock {
        TextBlock::Rules(text.to_string())
    }

    fn head(text: &str, key: &str) -> Option<String> {
        cut(vec![rules(text)], key).head
    }

    #[test]
    fn a_row_says_what_an_ability_does_and_not_what_it_costs_twice() {
        // The column carries the printed `{2}, {T}`; the sentence does not.
        assert_eq!(
            cut(vec![rules("{2}, {T}: Draw a card.")], "{2}, {T}"),
            Cut {
                head: Some("{2}, {T}".into()),
                blocks: vec![rules("Draw a card.")],
            }
        );
        // In the card's own words, whichever language they are in: the head
        // is the printing's, not a wording of the cost.
        assert_eq!(
            head(
                "{1}, {T}, opfere dieses Artefakt: Ziehe eine Karte.",
                "{1}, {T}"
            ),
            Some("{1}, {T}, opfere dieses Artefakt".into())
        );
        assert_eq!(
            head("Sacrifice a Forest: Add {G}{G}.", ""),
            Some("Sacrifice a Forest".into()),
            "a cost printed in words is licensed by a key with no symbols"
        );
        // A keyword line has no cost colon ahead of its reminder, so it keeps
        // every word and the row draws no column.
        let cycling = vec![
            rules("Cycling {2}"),
            TextBlock::Reminder("({2}, Discard this card: Draw a card.)".into()),
        ];
        assert_eq!(
            cut(cycling.clone(), "{2}"),
            Cut {
                head: None,
                blocks: cycling
            }
        );
    }

    /// Both branches of the licence: a head the ability's own cost accounts
    /// for, and one it does not.
    #[test]
    fn a_head_printing_a_symbol_the_ability_does_not_cost_is_not_its_column() {
        let line = "{R}, {T}: This deals 1 damage to any target.";
        assert_eq!(head(line, "{R}, {T}"), Some("{R}, {T}".into()));
        let refused = cut(vec![rules(line)], "{W}, {T}");
        assert_eq!(refused.head, None, "a {{R}} head is some other line's");
        assert_eq!(refused.blocks, vec![rules(line)], "drawn whole");
        // Old printings brace part of a cost and spell the rest (`6, {T}`).
        assert_eq!(
            head("6, {T}: Return target permanent.", "{6}, {T}"),
            Some("6, {T}".into())
        );
        // A tap the ability does not cost.
        assert_eq!(head("{T}: Add {C}.", ""), None);
        // Words before a colon, for an ability that costs symbols: the colon
        // is inside the effect.
        assert_eq!(head("Choose one: Draw a card.", "{2}"), None);
        assert_eq!(
            head("Choose one: Draw a card.", ""),
            Some("Choose one".into())
        );
    }

    /// The defect the licence replaced a guard for, in the two languages it
    /// was measured in. A sentence that grants an ability quotes a cost, and
    /// the colon inside the quotation is not this sentence's.
    #[test]
    fn a_sentence_that_grants_an_ability_is_never_cut_at_the_quoted_cost() {
        for granting in [
            "Lands you control have \"{T}: Add one mana of any color.\"",
            "L\u{e4}nder, die du kontrollierst, haben \u{201e}{T}: \
             Erzeuge ein Mana einer beliebigen Farbe.\"",
            "Target creature gains an ability until end of turn: \"{T}: Add {G}.\"",
        ] {
            assert_eq!(
                cut(vec![rules(granting)], "{T}"),
                Cut {
                    head: None,
                    blocks: vec![rules(granting)]
                },
                "{granting}"
            );
        }
    }

    /// No cap on how long a cost may be. Gemstone Mine's German head is fifty
    /// bytes, and the 48-byte cap this replaced drew it as an effect.
    #[test]
    fn a_long_printed_cost_is_still_a_cost() {
        let mine = "{T}, entferne eine Minenmarke von der Edelsteinmine: \
                    Erzeuge ein Mana einer beliebigen Farbe.";
        assert_eq!(
            head(mine, "{T}"),
            Some("{T}, entferne eine Minenmarke von der Edelsteinmine".into())
        );
        // Nothing after the colon is nothing to say, so the row keeps the
        // whole line rather than going blank.
        assert_eq!(
            cut(vec![rules("{T}: ")], "{T}"),
            Cut {
                head: None,
                blocks: vec![rules("{T}: ")]
            }
        );
        assert_eq!(cut(Vec::new(), "{T}").blocks, Vec::new());
    }

    /// A walker's head is its badge, licensed only by the same badge, in `:`
    /// and in the full-width `：` a Japanese printing uses.
    #[test]
    fn a_walkers_head_is_its_badge_and_only_its_own() {
        let line = "+2: Look at the top card of target player's library.";
        assert_eq!(
            cut(vec![rules(line)], "{L+2}"),
            Cut {
                head: Some("{L+2}".into()),
                blocks: vec![rules("Look at the top card of target player's library.")],
            }
        );
        assert_eq!(
            head("\u{2212}1: Draw a card.", "{L-1}"),
            Some("{L-1}".into())
        );
        assert_eq!(
            head("+2：プレイヤー１人を対象とする。", "{L+2}"),
            Some("{L+2}".into())
        );
        assert_eq!(head(line, "{L-1}"), None, "a different badge");
        assert_eq!(head(line, ""), None, "a badge on a line that costs mana");
        assert_eq!(
            head("{2}, {T}: Draw a card.", "{L+2}"),
            None,
            "mana on a walker"
        );
    }

    /// The stack's reading: a badge off the front, the sentence after it, and
    /// nothing else ever cut.
    #[test]
    fn a_walkers_line_gives_up_its_badge_before_it_is_printed() {
        let (badge, blocks) = loyalty_cut(vec![rules(
            "+2: Look at the top card of target player's library.",
        )]);
        let badge = badge.expect("a badge");
        assert_eq!(badge.tick, crate::manapip::Tick::Up);
        assert_eq!(badge.caption(), "2");
        assert_eq!(
            blocks,
            vec![rules("Look at the top card of target player's library.")]
        );
        let (down, _) = loyalty_cut(vec![rules("\u{2212}1: Draw a card.")]);
        assert_eq!(down.expect("a badge").tick, crate::manapip::Tick::Down);
        let (wide, _) = loyalty_cut(vec![rules("+2：プレイヤー１人を対象とする。")]);
        assert!(wide.is_some(), "the full-width colon");
        for other in [
            "{2}, {T}: Draw a card.",
            "Whenever this creature attacks, draw a card.",
            "Cycling {2}",
            "Target creature gains an ability until end of turn: \"{T}: Add {G}.\"",
        ] {
            assert_eq!(
                loyalty_cut(vec![rules(other)]),
                (None, vec![rules(other)]),
                "{other}"
            );
        }
        let reminder = vec![TextBlock::Reminder("+2: Draw a card.".into())];
        assert_eq!(loyalty_cut(reminder.clone()), (None, reminder));
        assert_eq!(loyalty_cut(Vec::new()), (None, Vec::new()));
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
