//! Where the caret stands in text that wraps, read off the laid-out runs.
//!
//! A single-line box needs none of this: its caret is a node standing
//! between two runs of text in a row (`lobby::ui::field_runs`), and the row
//! puts it where the letters end. A box whose text **wraps** cannot do that
//! — a node cannot stand in the middle of a paragraph — so the report form's
//! box (#320) draws its text as one paragraph of spans, head · selection ·
//! tail, and stands the caret where the renderer laid those spans out.
//!
//! The renderer reports each **run** it laid out: which span it belongs to
//! and the box it covers on its line. The caret is the seam between the
//! span before it and the span after it, so it stands at the **start of the
//! first run after it** — the one reading that holds whether the seam is
//! mid-line, at a soft wrap or right after a line break. The span after the
//! caret is never empty, because the drawer ends the tail with one space it
//! never shows; so the start of that run exists even with the caret at the
//! very end of the text, on a line of its own after a final line break.
//! Only where no run after it exists at all does the end of the last run
//! before it stand in.
//!
//! One seam that reading misses: a line break right after the caret. A line
//! with nothing on it lays out no run, so the first run after the caret is
//! then on a later line: the caret at the end of "wird." in "wird.\n\nabc"
//! would stand before "abc", two lines down. There the caret ends its line
//! instead: at the end of the text before it, or, where that text itself
//! ends in line breaks, at the start of the line they open ([`Seam`]).
//!
//! No glyph metrics and no font here: the numbers are the renderer's, in
//! whatever unit it laid out in, and come back in that unit.

/// One laid-out run of text: which span it belongs to, and the box it
/// covers on its line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    /// The span, as the renderer numbers them.
    pub section: usize,
    /// Left edge.
    pub left: f32,
    /// Top of its line.
    pub top: f32,
    /// Right edge.
    pub right: f32,
    /// Bottom of its line.
    pub bottom: f32,
}

/// Where the caret stands: a bar from `top`, `height` tall, at `x`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    /// Horizontal position of the bar.
    pub x: f32,
    /// Top of the bar: the top of the line it stands on.
    pub top: f32,
    /// Height of the bar: the height of that line.
    pub height: f32,
}

impl Spot {
    /// The bottom of the bar.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.top + self.height
    }
}

/// The line breaks at the caret, which the runs cannot show: a line with
/// nothing on it lays out no run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seam {
    /// How many line breaks the text before the caret ends in.
    pub breaks_before: usize,
    /// Whether the text after the caret begins with a line break.
    pub break_after: bool,
}

impl Seam {
    /// The seam between `before` and `after`, read off the text.
    #[must_use]
    pub fn between(before: &str, after: &str) -> Self {
        Self {
            breaks_before: before.chars().rev().take_while(|c| *c == '\n').count(),
            break_after: after.starts_with('\n'),
        }
    }
}

/// Where the caret stands between the spans in `before` and those in
/// `after`, given the runs the renderer laid out and the line breaks at the
/// seam.
///
/// `line_height` is the empty box's answer: with no run laid out at all
/// (nothing typed yet, or a layout that has not run), the caret stands at
/// the start of the first line.
#[must_use]
pub fn spot(runs: &[Run], before: &[usize], after: &[usize], seam: Seam, line_height: f32) -> Spot {
    if !seam.break_after {
        let first_after = runs
            .iter()
            .filter(|run| after.contains(&run.section))
            .min_by(|a, b| a.top.total_cmp(&b.top).then(a.left.total_cmp(&b.left)));
        if let Some(run) = first_after {
            return Spot {
                x: run.left,
                top: run.top,
                height: run.bottom - run.top,
            };
        }
    }
    let last_before = runs
        .iter()
        .filter(|run| before.contains(&run.section))
        .max_by(|a, b| a.top.total_cmp(&b.top).then(a.right.total_cmp(&b.right)));
    // The line breaks the text before the caret ends in open lines below
    // its last run, with nothing on them: the caret starts the last.
    #[allow(clippy::cast_precision_loss)]
    let down = seam.breaks_before as f32;
    match last_before {
        Some(run) if seam.breaks_before == 0 => Spot {
            x: run.right,
            top: run.top,
            height: run.bottom - run.top,
        },
        Some(run) => {
            let height = run.bottom - run.top;
            Spot {
                x: 0.0,
                top: run.top + height * down,
                height,
            }
        }
        None => Spot {
            x: 0.0,
            top: line_height * down,
            height: line_height,
        },
    }
}

/// The scroll offset that keeps `spot` inside a window `viewport` tall,
/// moving the window as little as it can from `scroll`.
///
/// A caret already in view leaves the window where it is: the box scrolls
/// because the caret went out of it, never to recentre it, which would make
/// every keystroke move the text under the player's eyes.
#[must_use]
pub fn follow(scroll: f32, viewport: f32, spot: Spot) -> f32 {
    let scroll = scroll.max(0.0);
    if spot.top < scroll {
        spot.top.max(0.0)
    } else if spot.bottom() > scroll + viewport {
        (spot.bottom() - viewport).max(0.0)
    } else {
        scroll
    }
}

#[cfg(test)]
// Exact: every position here is a small whole number of pixels, which a
// float holds and adds without rounding.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const HEAD: usize = 1;
    const SELECTED: usize = 2;
    const TAIL: usize = 3;

    fn run(section: usize, line: u8, left: f32, right: f32) -> Run {
        let top = f32::from(line) * 20.0;
        Run {
            section,
            left,
            top,
            right,
            bottom: top + 20.0,
        }
    }

    /// Mid-line: the caret is where the tail begins, which is where the head
    /// ends.
    #[test]
    fn the_caret_stands_where_the_text_after_it_begins() {
        let runs = [run(HEAD, 0, 0.0, 40.0), run(TAIL, 0, 40.0, 90.0)];
        let at = spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0);
        assert_eq!(
            at,
            Spot {
                x: 40.0,
                top: 0.0,
                height: 20.0
            }
        );
    }

    /// The text after the caret runs over three lines, and its runs arrive
    /// in no promised order: the caret takes the first of them on the page.
    #[test]
    fn a_tail_over_several_lines_puts_the_caret_at_its_first_run() {
        let runs = [
            run(TAIL, 2, 0.0, 30.0),
            run(HEAD, 0, 0.0, 10.0),
            run(TAIL, 1, 0.0, 120.0),
            run(TAIL, 0, 10.0, 120.0),
        ];
        assert_eq!(spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0).x, 10.0);
        assert_eq!(
            spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0).top,
            0.0
        );
    }

    /// After a line break at the very end, the caret is on the next line, at
    /// its start — which is where the drawer's closing space was laid out.
    /// This is the case a caret read off the end of the text before it
    /// gets wrong: that text ends on the line above.
    #[test]
    fn after_a_final_line_break_the_caret_is_on_the_new_line() {
        let runs = [run(HEAD, 0, 0.0, 60.0), run(TAIL, 1, 0.0, 4.0)];
        let at = spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0);
        assert_eq!((at.x, at.top), (0.0, 20.0));
    }

    /// Where the text wrapped, the caret follows the words onto the next
    /// line rather than staying at the right edge of the one above.
    #[test]
    fn a_wrapped_line_takes_the_caret_with_it() {
        let runs = [
            run(HEAD, 0, 0.0, 200.0),
            run(HEAD, 1, 0.0, 35.0),
            run(TAIL, 1, 35.0, 80.0),
        ];
        let at = spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0);
        assert_eq!((at.x, at.top), (35.0, 20.0));
    }

    /// With a selection the caret is at the end the player holds: before
    /// the selection, the selection is the text after it; after it, only the
    /// tail is.
    #[test]
    fn a_selection_puts_the_caret_at_the_end_being_held() {
        let runs = [
            run(HEAD, 0, 0.0, 20.0),
            run(SELECTED, 0, 20.0, 50.0),
            run(TAIL, 0, 50.0, 70.0),
        ];
        assert_eq!(
            spot(&runs, &[HEAD], &[SELECTED, TAIL], Seam::default(), 20.0).x,
            20.0
        );
        assert_eq!(
            spot(&runs, &[HEAD, SELECTED], &[TAIL], Seam::default(), 20.0).x,
            50.0
        );
    }

    /// Nothing laid out after the caret: the end of what is before it.
    #[test]
    fn with_nothing_after_it_the_caret_ends_the_text_before_it() {
        let runs = [run(HEAD, 0, 0.0, 200.0), run(HEAD, 1, 0.0, 35.0)];
        let at = spot(&runs, &[HEAD], &[TAIL], Seam::default(), 20.0);
        assert_eq!((at.x, at.top), (35.0, 20.0));
    }

    /// An empty box: the first line's start, as tall as a line.
    #[test]
    fn an_empty_box_has_its_caret_at_the_start() {
        assert_eq!(
            spot(&[], &[HEAD], &[TAIL], Seam::default(), 18.0),
            Spot {
                x: 0.0,
                top: 0.0,
                height: 18.0
            }
        );
    }

    /// The owner's text in the live check, "wird.\n\nabc": "wird." on line
    /// 0, nothing on line 1 (a blank line lays out no run), "abc" and the
    /// closing space on line 2. With the caret at the end of "wird." the
    /// text after it begins with a line break and its first run is two
    /// lines down; the caret ends "wird." instead.
    #[test]
    fn before_a_line_break_the_caret_ends_its_own_line() {
        let runs = [run(HEAD, 0, 0.0, 50.0), run(TAIL, 2, 0.0, 34.0)];
        let seam = Seam::between("wird.", "\n\nabc ");
        assert_eq!(
            seam,
            Seam {
                breaks_before: 0,
                break_after: true
            }
        );
        let at = spot(&runs, &[HEAD], &[TAIL], seam, 20.0);
        assert_eq!((at.x, at.top, at.height), (50.0, 0.0, 20.0));
    }

    /// One step on, the caret is on the blank line itself: after one line
    /// break, before the next. Neither text has a run there.
    #[test]
    fn on_a_blank_line_the_caret_starts_it() {
        let runs = [run(HEAD, 0, 0.0, 50.0), run(TAIL, 2, 0.0, 34.0)];
        let seam = Seam::between("wird.\n", "\nabc ");
        assert_eq!(seam.breaks_before, 1);
        let at = spot(&runs, &[HEAD], &[TAIL], seam, 20.0);
        assert_eq!((at.x, at.top), (0.0, 20.0));
        // Two blank lines: the caret on the second.
        let runs = [run(HEAD, 0, 0.0, 50.0), run(TAIL, 3, 0.0, 34.0)];
        let seam = Seam::between("wird.\n\n", "\nabc ");
        let at = spot(&runs, &[HEAD], &[TAIL], seam, 20.0);
        assert_eq!((at.x, at.top), (0.0, 40.0));
        // One more step: the start of "abc", which a run shows.
        let seam = Seam::between("wird.\n\n\n", "abc ");
        let at = spot(&runs, &[HEAD], &[TAIL], seam, 20.0);
        assert_eq!((at.x, at.top), (0.0, 60.0));
    }

    /// Text that is nothing but line breaks lays out no run before the
    /// caret: it counts lines from the top.
    #[test]
    fn line_breaks_alone_count_lines_from_the_top() {
        let at = spot(&[], &[HEAD], &[TAIL], Seam::between("\n\n", "\n "), 18.0);
        assert_eq!((at.x, at.top, at.height), (0.0, 36.0, 18.0));
    }

    #[test]
    fn the_window_moves_only_when_the_caret_leaves_it() {
        let line = |n: f32| Spot {
            x: 0.0,
            top: n * 20.0,
            height: 20.0,
        };
        // In view: nothing moves.
        assert_eq!(follow(40.0, 100.0, line(3.0)), 40.0);
        // Below the window: it moves just far enough to show the line.
        assert_eq!(follow(0.0, 100.0, line(7.0)), 60.0);
        // Above it: the line comes to the top.
        assert_eq!(follow(100.0, 100.0, line(2.0)), 40.0);
        // Never past the top of the text.
        assert_eq!(follow(-5.0, 100.0, line(0.0)), 0.0);
    }
}
