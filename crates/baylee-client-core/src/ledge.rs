//! The ledge's own arithmetic: where the question stands, and which key
//! answers it.
//!
//! The ledge is the opaque top edge of the hand zone — the mana pool at one
//! end, the engine's question and its answers in the middle, the two ways to
//! leave the game at the other. This module is the half of it with no
//! renderer in it, in the shape [`crate::seatbar`] already uses for a seat's
//! shelf: the renderer measures its three columns, asks here which **density**
//! the window can hold and where the middle sits, and builds nodes; a test
//! asks here with three numbers and needs no window.
//!
//! # Three columns, and why they are not a `space-between`
//!
//! The two outer columns are pinned to the window's edges and the middle one
//! stands on the window's **centre** — which is the centre of the local
//! seat's own mat, where the player is already looking. A flex row would
//! instead divide the slack evenly, so the question would drift left every
//! time a mana pip appeared beside it and the answers would be flung a
//! thousand pixels apart on a wide screen. Three independent columns is what
//! keeps the question still.
//!
//! Still, but not immovable. When the middle would collide with a neighbour
//! it is allowed to slide towards the free side before it is allowed to lose
//! anything: sixty pixels off centre is cheaper than a legend off a key.
//!
//! # One rule, three rungs
//!
//! [`Density`] is a ladder like [`crate::seatbar::Density`], and for the same
//! reason — two rules (one for the sentence, one for the buttons) would be
//! two places where 1280 looks unlike 1920.
//!
//! 1. [`Density::Full`] — everything, shifted off centre if that is what it
//!    takes.
//! 2. [`Density::Compact`] — the keycaps come off. The keyboard still works;
//!    it is only no longer written down.
//! 3. [`Density::Split`] — the sentence moves to the drawer and the ledge
//!    keeps the answers. The buttons are what the player needs; the sentence
//!    is what they have already read.
//!
//! Split is the **floor**, not a fourth measurement: [`arrange`] is told the
//! middle's full width and the width its keycaps account for, and it is never
//! told how wide the answers alone would be. That is deliberate. A window too
//! narrow even for the bare answers has no better arrangement to fall to, so
//! asking would only let the function return an answer it could not act on —
//! the same reason `seatbar`'s ladder ends at `Mark` rather than at nothing.

use crate::prefs::Action;

/// The least air between two columns of the ledge.
///
/// Below this the question and the mana pool read as one run of ink rather
/// than as two things, which is the whole point of putting them at opposite
/// ends.
pub const COLUMN_GAP: f32 = 24.0;

/// The air between two buttons on the ledge, and between the sentence and the
/// first of them.
///
/// The sentence gets [`SENTENCE_GAP`] instead — a bigger step, because the
/// break between "what you are being asked" and "what you may answer" is the
/// one break in that row a reader has to see.
pub const BUTTON_GAP: f32 = 8.0;

/// The air between the question and the first answer.
pub const SENTENCE_GAP: f32 = 16.0;

/// How much of the ledge each of its three columns wants.
///
/// `left` and `right` are measured **from the window's own edge**, so
/// whatever inset the renderer holds a column off the edge by is already in
/// them; [`arrange`] adds nothing. `mid` is the middle column's natural
/// width — the sentence, the gap, and every answer with its keycap.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Columns {
    /// The mana pool's end.
    pub left: f32,
    /// The question and its answers.
    pub mid: f32,
    /// The two ways to leave the game.
    pub right: f32,
}

/// How much of the middle the ledge can afford to draw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Density {
    /// The sentence, the answers and their keycaps.
    Full,
    /// The sentence and the answers, with the keycaps dropped.
    Compact,
    /// The answers alone; the sentence goes to the drawer.
    Split,
}

/// Where the ledge puts its middle column, and how much of it it draws.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Arrangement {
    /// What the middle column may draw.
    pub density: Density,
    /// The centre of the middle column, in logical pixels from the window's
    /// left edge.
    ///
    /// The window's own centre whenever that fits, and the nearest place it
    /// does fit otherwise.
    pub mid_x: f32,
}

impl Density {
    /// Every rung, richest first.
    pub const ALL: [Self; 3] = [Self::Full, Self::Compact, Self::Split];

    /// Whether this rung still writes the keys on the buttons.
    #[must_use]
    pub const fn shows_keycaps(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether the sentence is still on the ledge rather than in the drawer.
    #[must_use]
    pub const fn shows_sentence(self) -> bool {
        matches!(self, Self::Full | Self::Compact)
    }
}

/// The richest arrangement a `window_w`-wide window can hold.
///
/// `caps_w` is the total width the middle's keycaps account for — the part
/// [`Density::Compact`] gives back. It is a number rather than a count
/// because a legend is as wide as it is spelled: `⇧Tab` is not `Space`.
///
/// The middle fits when it and both neighbours and a [`COLUMN_GAP`] on either
/// side of it are together no wider than the window. Where inside that the
/// middle sits is then a separate question with its own answer: the window's
/// centre, pulled to the nearest point that clears both neighbours.
#[must_use]
pub fn arrange(window_w: f32, cols: Columns, caps_w: f32) -> Arrangement {
    for density in Density::ALL {
        let mid = match density {
            Density::Full => cols.mid,
            Density::Compact => (cols.mid - caps_w).max(0.0),
            // The floor. Nothing here knows how wide the bare answers are —
            // see the module comment — so Split is taken on faith and is the
            // last rung either way.
            Density::Split => {
                return Arrangement {
                    density,
                    mid_x: centre_between(window_w, cols, 0.0),
                };
            }
        };
        if mid + cols.left + cols.right + 2.0 * COLUMN_GAP <= window_w {
            return Arrangement {
                density,
                mid_x: centre_between(window_w, cols, mid),
            };
        }
    }
    unreachable!("Density::ALL ends at Split, which always returns")
}

/// Where a middle column `mid` wide sits between the two outer ones.
///
/// The window's centre when that clears both neighbours by [`COLUMN_GAP`],
/// and otherwise the nearest place that does — the question slides rather
/// than shrinking, because a sentence sixty pixels off centre is still the
/// sentence and a sentence with its keys taken off is not.
///
/// When there is no such place at all the two limits have crossed, and the
/// answer is the point midway between them: everything overlaps by then, and
/// splitting the overlap evenly is the only symmetric thing left to do.
fn centre_between(window_w: f32, cols: Columns, mid: f32) -> f32 {
    let lo = cols.left + COLUMN_GAP + mid / 2.0;
    let hi = window_w - cols.right - COLUMN_GAP - mid / 2.0;
    if lo > hi {
        f32::midpoint(lo, hi)
    } else {
        (window_w / 2.0).clamp(lo, hi)
    }
}

/// What a prompt button answers.
///
/// Here rather than beside the `PromptButton` component it rides on, because
/// [`shortcut_for`] is the bridge from an answer to the key that sends it and
/// a keymap is [`crate::prefs`]' business. The renderer re-exports this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PromptAction {
    /// Yes.
    Yes,
    /// No.
    No,
    /// Keep the hand.
    Keep,
    /// Take the mulligan.
    Mulligan,
    /// Confirm / pass / OK.
    Confirm,
    /// Declare no attackers, or no blockers.
    DeclareNothing,
    /// Aim the next declaration at the next defender (or attacker).
    AimNext,
    /// Hand the rest of this turn to the autopilot.
    ///
    /// The other half of the arrow buttons the rail lost. It belongs beside
    /// the rest for the reason the ledge exists at all: "pass this window"
    /// and "pass every window until my next turn" are the same decision at
    /// two sizes, and a player who has just been offered the first should not
    /// have to look somewhere else for the second.
    SkipTurn,
    /// One arm of the number stepper: `+1` or `-1`.
    ///
    /// A prompt button rather than a component of its own, because that is
    /// what it is — the one choice with nothing on the table to click, and
    /// the arms belong in the same row as every other answer. It also keeps
    /// `input::pointer` off Bevy's system-parameter limit.
    Step(i32),
}

impl PromptAction {
    /// Every answer a button can carry, with the stepper's two arms spelled
    /// out.
    ///
    /// [`Self::Step`] is the one variant with a payload, and the only two
    /// values anything builds are `+1` and `-1` — a stepper moves by one.
    /// Listing them is what lets [`shortcut_for`] be tested over the whole
    /// enum rather than over the variants somebody remembered.
    pub const ALL: [Self; 10] = [
        Self::Yes,
        Self::No,
        Self::Keep,
        Self::Mulligan,
        Self::Confirm,
        Self::DeclareNothing,
        Self::AimNext,
        Self::SkipTurn,
        Self::Step(1),
        Self::Step(-1),
    ];
}

/// The action whose key legend belongs on this answer's button.
///
/// The legend on a keycap is never a string in the renderer: it comes from
/// `Keymap::chords(action)[0].display()`, so a player who rebinds `Space`
/// sees the new key on the button on the next frame rather than a label that
/// lies until somebody remembers to change it.
///
/// `None` is a button with no key behind it, which is a real answer and not a
/// gap: a stepper arm that moves by something other than one has no key
/// because nothing builds one.
#[must_use]
pub const fn shortcut_for(action: PromptAction) -> Option<Action> {
    Some(match action {
        PromptAction::Yes => Action::AnswerYes,
        PromptAction::No => Action::AnswerNo,
        PromptAction::Keep => Action::MulliganKeep,
        PromptAction::Mulligan => Action::MulliganTake,
        PromptAction::Confirm => Action::Confirm,
        PromptAction::DeclareNothing => Action::CombatNone,
        PromptAction::AimNext => Action::CombatFocusNext,
        // Both of these hand the turn to the client's own autopilot; the
        // button and the key are one mechanism, so they share one legend.
        PromptAction::SkipTurn => Action::NextTurn,
        PromptAction::Step(1) => Action::NumberUp,
        PromptAction::Step(-1) => Action::NumberDown,
        PromptAction::Step(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::Keymap;

    /// The three widths of §2.3, at the screen it wrote them for.
    ///
    /// `LEFT` is a six-entry mana pool and `RIGHT` is both leave-the-game
    /// buttons — the widest either column ever gets — and both are measured
    /// from the window's edge, so the renderer's own inset is in them.
    ///
    /// All three are **measured** rather than estimated, which is §10.1 item
    /// 3's acceptance and moved every one of them: §2.3 read 325, 588 and 214
    /// off `0.52 × pt` per character, and the shipped face gives 365 for the
    /// pool (the label is 60.5 and an entry 42.0, the design's 36 having left
    /// out the restriction rim's padding), 623 for a German priority — out of
    /// the buttons' own drawn widths at 1728, which is where `Shift+Tab`
    /// spelled out in full lands — and 222 for "Remis anbieten" beside
    /// "Aufgeben". All three moved *up*, which is the direction that costs
    /// something: `arrange` slides the question to clear what it is told the
    /// neighbours take.
    ///
    /// `RIGHT` is the column **at rest**. An armed concession is wider than
    /// both buttons together and is drawn alone for exactly that reason; the
    /// renderer's own `RIGHT_RESERVED` carries the measurement and the
    /// argument.
    ///
    /// `LEFT` is the **historical** worst case and is kept at it deliberately.
    /// The renderer moved the mana pool off this shelf in September 2026 and
    /// the left column is now the hand's sorting buttons, which reserve 294 at
    /// their widest — so the rungs below are stated against a neighbour wider
    /// than any the client draws today. That is the right way for a test of
    /// `arrange` to be stale: the arithmetic it pins is the function's, the
    /// number is one caller's, and a rule checked at a harder number than it
    /// meets keeps holding when a column grows back.
    const LEFT: f32 = 365.0;
    const RIGHT: f32 = 222.0;
    /// A priority window's middle: the sentence, three answers, three caps.
    const PRIORITY: f32 = 623.0;
    /// What the three keycaps in that middle account for.
    const CAPS: f32 = 91.0;

    fn worst(mid: f32) -> Columns {
        Columns {
            left: LEFT,
            mid,
            right: RIGHT,
        }
    }

    /// The whole of the density rule, as one arithmetic statement.
    ///
    /// The middle fits when `mid + left + right + 2·COLUMN_GAP <= window`, so
    /// at 1280 between the widest neighbours the rungs are
    ///
    /// ```text
    ///   Full     mid <= 1280 - 365 - 222 - 48  =  645
    ///   Compact  645 < mid <= 645 + 91         =  736
    ///   Split    mid > 736
    /// ```
    ///
    /// which is written out here because the design's own worked example puts
    /// 800 in `Compact`, and 800 − 91 is 709, which is 64 px past what 1280
    /// has. Every input below sits inside a rung rather than on a boundary.
    #[test]
    fn the_rungs_are_where_the_arithmetic_puts_them() {
        for (mid, want) in [
            (PRIORITY, Density::Full),
            (645.0, Density::Full),
            (646.0, Density::Compact),
            (700.0, Density::Compact),
            (736.0, Density::Compact),
            (737.0, Density::Split),
            (900.0, Density::Split),
        ] {
            assert_eq!(
                arrange(1280.0, worst(mid), CAPS).density,
                want,
                "a {mid}-px middle at 1280 between the widest neighbours"
            );
        }
    }

    #[test]
    fn a_priority_window_is_drawn_whole_on_every_screen_that_exists() {
        for window in [1280.0, 1920.0, 3200.0] {
            let at = arrange(window, worst(PRIORITY), CAPS);
            assert_eq!(
                at.density,
                Density::Full,
                "the longest priority question at {window}"
            );
        }
    }

    /// At 1280 the widest mana pool reaches 365 and the centred question
    /// would start at 640 − 311.5 = 328.5, well inside the 24 the two columns
    /// owe each other. The question slides; it does not shrink — and it is
    /// the *measured* pool that makes the slide 60 px rather than the 3 the
    /// estimate predicted, which is what the reservation is for.
    #[test]
    fn the_question_slides_off_centre_rather_than_losing_its_keys() {
        let at = arrange(1280.0, worst(PRIORITY), CAPS);
        assert_eq!(at.density, Density::Full);
        assert!(
            (at.mid_x - 700.5).abs() < 1e-3,
            "expected the question pushed 60 px right of centre, got {}",
            at.mid_x
        );
        assert!(
            at.mid_x - PRIORITY / 2.0 >= LEFT + COLUMN_GAP - 1e-3,
            "it has to clear the mana pool by the gap it slid for"
        );
    }

    /// The same question on a wide screen stands dead centre, because that is
    /// where the local seat's own mat is.
    #[test]
    fn with_room_to_spare_the_question_stands_on_the_windows_centre() {
        for window in [1920.0, 3200.0] {
            let at = arrange(window, worst(PRIORITY), CAPS);
            assert!(
                (at.mid_x - window / 2.0).abs() < 1e-3,
                "at {window} the question should be centred, got {}",
                at.mid_x
            );
        }
    }

    /// An empty mana pool is 269 px of room the worst case does not have, and
    /// it buys back a whole rung — which is the reason the left column's
    /// width is an input rather than a constant.
    ///
    /// 96 is the measured empty column: the edge, the label and the em dash
    /// that stands where the entries would. Note what this test is *not* — the
    /// renderer hands `arrange` the reservation whatever is floating (§2.3
    /// refuses to let the question follow the pool), so this is the rule
    /// answering honestly about an input it is not currently given.
    #[test]
    fn an_empty_mana_pool_pays_for_a_longer_question() {
        let full = Columns {
            left: 96.0,
            mid: 800.0,
            right: RIGHT,
        };
        assert_eq!(arrange(1280.0, full, CAPS).density, Density::Full);
        assert_eq!(arrange(1280.0, worst(800.0), CAPS).density, Density::Split);
    }

    /// A window narrower than its own two outer columns has nowhere to put
    /// the middle, and has to answer anyway.
    #[test]
    fn a_hopeless_window_still_gets_an_answer_and_not_a_panic() {
        let at = arrange(600.0, worst(PRIORITY), CAPS);
        assert_eq!(at.density, Density::Split);
        assert!(at.mid_x.is_finite(), "got {}", at.mid_x);
    }

    /// The legend on a keycap comes out of the keymap, so an answer mapped to
    /// an action nobody has bound would draw an empty cap. A compile-time
    /// list plus a bound check is what stops that being discovered live.
    #[test]
    fn every_answer_with_a_key_has_one_in_the_default_keymap() {
        let keymap = Keymap::default();
        for action in PromptAction::ALL {
            let Some(bound) = shortcut_for(action) else {
                continue;
            };
            assert!(
                !keymap.chords(bound).is_empty(),
                "{action:?} points at {bound:?}, which the default keymap does not bind"
            );
        }
    }

    /// …and the other half, because a `shortcut_for` that answered `None` to
    /// everything would pass the test above in silence.
    #[test]
    fn every_answer_a_button_carries_has_a_key_except_the_stepper_by_two() {
        for action in PromptAction::ALL {
            assert!(
                shortcut_for(action).is_some(),
                "{action:?} has no key, and every answer on the ledge needs one"
            );
        }
        assert!(
            shortcut_for(PromptAction::Step(2)).is_none(),
            "nothing builds a stepper arm that moves by two, so nothing binds one"
        );
    }

    /// Pass and skip-the-turn are two sizes of one decision and must not
    /// share a legend; aim-next and declare-nothing are the two combat
    /// answers and must not either.
    #[test]
    fn no_two_answers_in_one_question_wear_the_same_key() {
        for row in [
            vec![PromptAction::Confirm, PromptAction::SkipTurn],
            vec![
                PromptAction::Confirm,
                PromptAction::AimNext,
                PromptAction::DeclareNothing,
            ],
            vec![PromptAction::Keep, PromptAction::Mulligan],
            vec![PromptAction::Yes, PromptAction::No],
            vec![PromptAction::Step(1), PromptAction::Step(-1)],
        ] {
            let keymap = Keymap::default();
            let mut legends: Vec<String> = row
                .iter()
                .filter_map(|answer| shortcut_for(*answer))
                .map(|action| keymap.chords(action)[0].display())
                .collect();
            let asked = legends.len();
            legends.sort();
            legends.dedup();
            assert_eq!(legends.len(), asked, "two answers of {row:?} share a key");
        }
    }
}
