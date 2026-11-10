//! What a window size can break, found on one [`Screen`].
//!
//! Each check reads only what a person can see or reach: a node hidden by
//! its `Visibility` is skipped, and so is one a scroll viewport hides (a row
//! scrolled out of its list is not misplaced; [`Screen::scrolled_away`]).
//!
//! - **Cut off**: a pressable node lies partly or wholly outside an ancestor
//!   that clips without scrolling, so part of it can never be seen or
//!   pressed. The lobby's root clips at the window's edge, so a control
//!   pushed out of the window is reported here.
//! - **Out of the window**: a pressable node no ancestor clips, or one of the
//!   table's buttons, reaches past the window's edge.
//! - **Text overflow**: a text's glyphs measure wider (or taller) than the
//!   box it was given, so it spills or is cut.
//! - **Overlap**: two pressable nodes, neither inside the other, cover the
//!   same pixels, so one press could mean either.
//!
//! Under a modal's shade (a pressable node over the whole window, which a
//! tap answers, holding the modal's own pressable nodes) nothing else can be
//! pressed: the pressable checks then read only the topmost shade's own
//! nodes. A bare scrim over the window, with its menu beside it, is no
//! shade.
//! - **Small target** (only with [`Layout::touch`]): a pressable node smaller
//!   than a finger can hit, 44 px. The desktop client's small text links are
//!   23 px high by design, so this is a check for the coming phone builds
//!   (`docs/mobile.md`), not for a narrow desktop window.
//!
//! The tree is `shell_nodes` and `ui_nodes` together, so the table's HUD is
//! checked as well as the lobby; only the lobby's nodes say they are
//! pressable, so at the table the pressable checks read `buttons`.
use crate::screen::{Clip, ControlList, Rect, Screen};
use anyhow::{Result, bail};
use std::fmt;

/// One thing wrong with a screen's layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Issue {
    /// A pressable node lies outside an ancestor that clips without scrolling.
    CutOff {
        /// What it says.
        label: String,
        /// Where it is.
        rect: Rect,
        /// The box that cuts it.
        by: Rect,
    },
    /// A pressable node reaches past the window.
    OutOfWindow {
        /// What it says.
        label: String,
        /// Where it is.
        rect: Rect,
    },
    /// A text measures larger than its box.
    TextOverflow {
        /// The text.
        text: String,
        /// Its box.
        rect: Rect,
        /// What its glyphs measure.
        measured: (f32, f32),
    },
    /// Two pressable nodes cover the same pixels.
    Overlap {
        /// What the first says.
        first: String,
        /// What the second says.
        second: String,
        /// The pixels they share.
        shared: Rect,
    },
    /// A pressable node is smaller than a finger.
    SmallTarget {
        /// What it says.
        label: String,
        /// Where it is.
        rect: Rect,
        /// The smallest side it should have.
        min: f32,
    },
}

impl fmt::Display for Issue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CutOff { label, rect, by } => {
                write!(f, "{label:?} is cut off: {rect}, inside a clip of {by}")
            }
            Self::OutOfWindow { label, rect } => {
                write!(f, "{label:?} reaches out of the window: {rect}")
            }
            Self::TextOverflow {
                text,
                rect,
                measured,
            } => write!(
                f,
                "{text:?} measures {:.0}×{:.0} in a box of {rect}",
                measured.0, measured.1
            ),
            Self::Overlap {
                first,
                second,
                shared,
            } => write!(f, "{first:?} and {second:?} overlap: {shared}"),
            Self::SmallTarget { label, rect, min } => {
                write!(f, "{label:?} is smaller than {min:.0} px: {rect}")
            }
        }
    }
}

/// Which checks run, and how strictly.
#[derive(Clone, Debug)]
pub struct Layout {
    /// Pixels a rectangle may stray before it counts (rounding, borders).
    pub slack: f32,
    /// The smallest side a pressable node may have; `None` skips the check.
    pub min_target: Option<f32>,
    /// Texts containing one of these are not checked for overflow (an
    /// ellipsised name, a deliberately clipped preview).
    pub may_overflow: Vec<String>,
    /// Labels containing one of these are skipped by every check.
    pub ignore: Vec<String>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            slack: 1.0,
            min_target: None,
            may_overflow: Vec::new(),
            ignore: Vec::new(),
        }
    }
}

impl Layout {
    /// The desktop checks: out of the window, overflow, overlap.
    pub fn desktop() -> Self {
        Self::default()
    }

    /// The desktop checks plus 44-px targets (the compact and phone frames).
    pub fn touch() -> Self {
        Self {
            min_target: Some(44.0),
            ..Self::default()
        }
    }

    /// Skips texts containing `words` in the overflow check.
    #[must_use]
    pub fn may_overflow(mut self, words: &str) -> Self {
        self.may_overflow.push(words.to_owned());
        self
    }

    /// Skips nodes whose label contains `words` in every check.
    #[must_use]
    pub fn ignoring(mut self, words: &str) -> Self {
        self.ignore.push(words.to_owned());
        self
    }

    /// Everything wrong with the screen.
    pub fn issues(&self, screen: &Screen) -> Vec<Issue> {
        let window = screen.window();
        let nodes = screen.nodes();
        let shown =
            |i: usize| !nodes[i].hidden && !nodes[i].rect.is_empty() && !screen.scrolled_away(i);
        let ignored = |label: &str| self.ignore.iter().any(|w| label.contains(w.as_str()));
        // A pressable node over the whole window that holds pressable nodes
        // of its own is a modal's shade (a tap on it answers the modal):
        // nothing under it can be pressed, so the pressable checks read only
        // the topmost shade's own nodes. A bare scrim (a menu's, with the
        // menu beside it) holds none, and leaves everything checked.
        let shade = (0..nodes.len()).rev().find(|&i| {
            nodes[i].pressable
                && shown(i)
                && nodes[i].rect.contains(&window, self.slack)
                && screen
                    .subtree(i)
                    .skip(1)
                    .any(|(j, node)| node.pressable && shown(j))
        });
        let in_front = |i: usize| shade.is_none_or(|shade| screen.is_within(i, shade));
        let pressable: Vec<(usize, String)> = (0..nodes.len())
            .filter(|&i| nodes[i].pressable && shown(i) && in_front(i))
            .map(|i| (i, screen.label_of(i)))
            .filter(|(_, label)| !ignored(label))
            .collect();
        let mut out = Vec::new();
        for (i, label) in &pressable {
            let rect = nodes[*i].rect;
            if let Some((by, Clip::Cut)) = screen.clipped_by(*i) {
                out.push(Issue::CutOff {
                    label: label.clone(),
                    rect,
                    by: nodes[by].rect,
                });
            } else if !window.contains(&rect, self.slack) {
                out.push(Issue::OutOfWindow {
                    label: label.clone(),
                    rect,
                });
            }
            if let Some(min) = self.min_target
                && (rect.w + self.slack < min || rect.h + self.slack < min)
            {
                out.push(Issue::SmallTarget {
                    label: label.clone(),
                    rect,
                    min,
                });
            }
        }
        for control in screen.controls() {
            if control.list == ControlList::Table
                && !ignored(&control.press)
                && !window.contains(&control.rect, self.slack)
            {
                out.push(Issue::OutOfWindow {
                    label: control.press.clone(),
                    rect: control.rect,
                });
            }
        }
        for (n, (i, first)) in pressable.iter().enumerate() {
            for (j, second) in &pressable[n + 1..] {
                if screen.is_within(*i, *j) || screen.is_within(*j, *i) {
                    continue;
                }
                let shared = nodes[*i].rect.overlap(&nodes[*j].rect);
                if shared.w > self.slack && shared.h > self.slack {
                    out.push(Issue::Overlap {
                        first: first.clone(),
                        second: second.clone(),
                        shared,
                    });
                }
            }
        }
        for (i, node) in nodes.iter().enumerate() {
            let (Some(text), Some(measured)) = (node.text.as_deref(), node.text_size) else {
                continue;
            };
            if text.trim().is_empty()
                || !shown(i)
                || ignored(text)
                || self.may_overflow.iter().any(|w| text.contains(w.as_str()))
            {
                continue;
            }
            if measured.0 > node.rect.w + self.slack || measured.1 > node.rect.h + self.slack {
                out.push(Issue::TextOverflow {
                    text: text.to_owned(),
                    rect: node.rect,
                    measured,
                });
            }
        }
        out
    }

    /// Fails listing every issue, if there is one.
    pub fn check(&self, screen: &Screen) -> Result<()> {
        let issues = self.issues(screen);
        if issues.is_empty() {
            return Ok(());
        }
        bail!(
            "{} layout issue(s):\n  - {}",
            issues.len(),
            issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  - ")
        )
    }
}

impl Screen {
    /// The default layout checks, [`Layout::desktop`], at any size.
    pub fn layout(&self) -> Layout {
        Layout::desktop()
    }

    /// Fails listing every layout issue the default checks find.
    pub fn expect_layout_ok(&self) -> Result<()> {
        self.layout().check(self)
    }
}
