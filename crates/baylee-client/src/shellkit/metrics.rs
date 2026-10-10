//! Every size a shell screen takes, in one place (the shell design, §8).
//!
//! The lobby's `Metrics::of(width)` is also read by table UI (the burger
//! menu, the report form over the table) and stays as it is; shell screens
//! read [`ShellMetrics::of`] instead: today's values multiplied by the text
//! step's factor, with floors where a finger or a small screen needs them.
//! A test holds the 3D table and its felt off it (`lint::the_table_never_reads_the_shell_s_metrics`); the table's HUD may read it.
//!
//! Every length a shell module writes goes through [`ShellMetrics::px`]
//! (scaled) or [`px_fixed`] (hairlines, the focus ring, gutters, the QR
//! module, the 44-px hit area); a bare `px(` with a number in a shell module
//! is a test failure (`lint`).

use super::size::{Frame, InputClass, TextSize, Viewport};
use super::tokens;
use bevy::prelude::*;

/// A length that does not follow the text step.
#[must_use]
pub const fn px_fixed(n: f32) -> Val {
    Val::Px(n)
}

/// The widest window (effective, at the text step) and the tallest one the
/// shell draws at its own sizes; past them it scales up.
pub const SCALE_FROM: (f32, f32) = (1920.0, 1080.0);
/// The most the big-screen scale grows (a 3840-wide window).
pub const SCALE_MAX: f32 = 1.75;

/// How much larger the shell draws on a big window: a 2560 × 1440 window
/// reads 1.25, a 3840 × 2160 one 1.75, anything up to 1920 × 1080 (and
/// every Phone, Compact or Narrow window) 1. Proportional to the smaller of
/// the width (effective, so a large text step does not scale twice) and the
/// height over [`SCALE_FROM`], three quarters of the way, and stepped in
/// eighths so that a resize rebuilds the lobby only when a step is crossed.
#[must_use]
pub fn screen_scale(view: Viewport, step: TextSize) -> f32 {
    if !matches!(Frame::classify(view, step), Frame::Wide | Frame::Vast) {
        return 1.0;
    }
    let wide = view.width / step.factor() / SCALE_FROM.0;
    let tall = view.height / SCALE_FROM.1;
    let over = wide.min(tall) - 1.0;
    let raw = (1.0 + over * 0.75).clamp(1.0, SCALE_MAX);
    (raw * 8.0).round() / 8.0
}

/// What a shell screen is sized by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShellMetrics {
    /// The size class.
    pub frame: Frame,
    /// Finger or pointer.
    pub input: InputClass,
    /// The text step.
    pub step: TextSize,
    /// The step's factor times [`Self::scale`]: what every scaled length is
    /// multiplied by.
    pub factor: f32,
    /// The big-screen scale ([`screen_scale`]): 1 up to a 1920 × 1080
    /// window, a step up on the large ones.
    pub scale: f32,
    /// Screen titles.
    pub h1: f32,
    /// Panel headings (`shell.head`).
    pub head: f32,
    /// Body text (`shell.text`).
    pub text: f32,
    /// Captions, meta lines, the artist's credit (`shell.small`).
    pub small: f32,
    /// A control's height: 44, or 36 under a pointer at the small steps.
    pub control: f32,
    /// A hit area's least side: 44 under Touch whatever the step, else the
    /// control's height.
    pub hit: f32,
    /// Padding inside a panel.
    pub pad: f32,
    /// The gap between stacked controls.
    pub gap: f32,
    /// The body's padding and the gap between its panels.
    pub body: f32,
    /// A list row's pitch (`shell.row`).
    pub row: f32,
    /// A deck tile's height.
    pub tile: f32,
    /// The header's height.
    pub header: f32,
    /// The widest a form-like body grows.
    pub body_max: f32,
    /// The widest a collection (tiles, rows) grows.
    pub collection_max: f32,
}

impl ShellMetrics {
    /// The metrics for a window at a text step.
    #[must_use]
    pub fn of(view: Viewport, step: TextSize) -> Self {
        Self::under(view, step, 1.0)
    }

    /// The metrics for a window whose UI is already drawn `ui` times larger
    /// (Bevy's `UiScale`, which the table HUD sets: `hud::scale`): the
    /// big-screen step that scale already applies is taken out, so a shell
    /// panel over the table is drawn the size it is in the lobby. `view` is
    /// the window itself, in logical pixels.
    #[must_use]
    pub fn under(view: Viewport, step: TextSize, ui: f32) -> Self {
        let frame = Frame::classify(view, step);
        let scale = screen_scale(view, step) / ui.max(f32::EPSILON);
        let f = step.factor() * scale;
        let touch = view.input == InputClass::Touch;
        // Today's three rows (`lobby::Metrics::of`), as the default step's (M) values.
        let (h1, head, text, small) = match frame {
            Frame::Phone => (22.0, 18.0, 15.0, 12.5),
            Frame::Compact | Frame::Narrow => (22.0, 18.0, 15.0, 11.5),
            Frame::Wide | Frame::Vast => (26.0, 22.0, 16.0, 12.5),
        };
        let phone = frame == Frame::Phone;
        // Floors (§2.3, §8): body 13 on a phone or under a finger; small 9 on
        // a desktop and 12 on a phone (N4-5); headings never below body.
        let text_floor = if phone || touch { 13.0 } else { 0.0 };
        let small_floor = if phone { 12.0 } else { 9.0 };
        let text = (text * f).max(text_floor);
        let small = (small * f).max(small_floor);
        let head = (head * f).max(text + 2.0);
        let h1 = (h1 * f).max(head + 2.0);
        // `tap = max(touch ? 44 : 36, 44 × factor)` (§8).
        let control = (if touch { tokens::HIT } else { 36.0 }).max(tokens::HIT * f);
        let hit = if touch {
            control.max(tokens::HIT)
        } else {
            control
        };
        let (pad, body) = match frame {
            Frame::Phone => (10.0, 8.0),
            Frame::Compact | Frame::Narrow => (14.0, 12.0),
            Frame::Wide if touch => (16.0, 14.0),
            Frame::Wide => (20.0, 20.0),
            Frame::Vast => (24.0, 24.0),
        };
        let gap = match frame {
            Frame::Phone => 8.0,
            Frame::Compact | Frame::Narrow | Frame::Wide | Frame::Vast => 12.0,
        };
        let (row, tile) = if phone { (60.0, 190.0) } else { (72.0, 206.0) };
        Self {
            frame,
            input: view.input,
            step,
            factor: f,
            scale,
            h1,
            head,
            text,
            small,
            control,
            hit,
            pad: pad * f,
            gap: gap * f,
            body: body * f,
            row: row * f,
            tile: tile * f,
            header: if phone { 44.0 } else { 56.0_f32.max(44.0 * f) },
            body_max: 1480.0 * f,
            collection_max: 2400.0 * f,
        }
    }

    /// `n` logical pixels at the default step (M), at this step.
    #[must_use]
    pub fn px(self, n: f32) -> Val {
        Val::Px(self.scaled(n))
    }

    /// `n` at this step, as a number.
    #[must_use]
    pub fn scaled(self, n: f32) -> f32 {
        n * self.factor
    }

    /// One of the spacing tokens (§2.2), 0 = 4 px … 6 = 48 px, scaled.
    #[must_use]
    pub fn space(self, index: usize) -> Val {
        self.px(tokens::SPACE[index.min(tokens::SPACE.len() - 1)])
    }

    /// Whether a finger is the input.
    #[must_use]
    pub fn touch(self) -> bool {
        self.input == InputClass::Touch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(w: f32, h: f32, step: TextSize, input: InputClass) -> ShellMetrics {
        ShellMetrics::of(
            Viewport {
                input,
                ..Viewport::desktop(w, h)
            },
            step,
        )
    }

    /// §8's table at the default step (M, 3) is today's desktop: 16 / 22 / 12.5, row 72.
    #[test]
    fn the_default_step_is_today_and_the_rest_scale_with_floors() {
        let wide = at(1920.0, 1080.0, TextSize::M, InputClass::Pointer);
        assert_eq!((wide.text, wide.head, wide.small), (16.0, 22.0, 12.5));
        assert!((wide.row - 72.0).abs() < 1e-3 && (wide.body_max - 1480.0).abs() < 1e-3);
        let small = at(1920.0, 1080.0, TextSize::Xs, InputClass::Pointer);
        assert!((small.text - 12.64).abs() < 0.01, "{}", small.text);
        assert!((small.small - 9.875).abs() < 1e-3, "{}", small.small);
        assert!((small.row - 56.88).abs() < 0.01);
        let large = at(1920.0, 1080.0, TextSize::Xl, InputClass::Pointer);
        assert!((large.text - 20.256).abs() < 1e-3 && (large.head - 27.852).abs() < 1e-3);
        // A phone keeps body 13 and small 12 at the smallest step.
        let phone = at(844.0, 390.0, TextSize::Xs, InputClass::Touch);
        assert_eq!(phone.frame, Frame::Phone);
        assert!((phone.text - 13.0).abs() < 1e-3 && (phone.small - 12.0).abs() < 1e-3);
        assert!((at(844.0, 390.0, TextSize::M, InputClass::Touch).text - 15.0).abs() < 1e-3);
    }

    /// The big-screen scale: today's sizes up to 1920 × 1080, a step up at
    /// 2560 × 1440, more at 3840 × 2160, never on a short window, and a large
    /// text step does not scale a second time.
    #[test]
    fn a_big_window_draws_the_shell_a_step_larger() {
        let p = InputClass::Pointer;
        for (w, h) in [
            (1280.0, 800.0),
            (1708.0, 1032.0),
            (1920.0, 1080.0),
            (844.0, 390.0),
        ] {
            assert!((at(w, h, TextSize::M, p).scale - 1.0).abs() < 1e-6, "{w}");
        }
        let qhd = at(2560.0, 1440.0, TextSize::M, p);
        assert!((qhd.scale - 1.25).abs() < 1e-6 && (qhd.text - 20.0).abs() < 1e-3);
        assert!((at(2560.0, 1369.0, TextSize::M, p).scale - 1.25).abs() < 1e-6);
        let uhd = at(3840.0, 2160.0, TextSize::M, p);
        assert!((uhd.scale - SCALE_MAX).abs() < 1e-6 && (uhd.text - 28.0).abs() < 1e-3);
        assert!((at(3840.0, 2089.0, TextSize::M, p).scale - 1.75).abs() < 1e-6);
        assert!((at(7680.0, 4320.0, TextSize::M, p).scale - SCALE_MAX).abs() < 1e-6);
        // Wide but short: the height holds it.
        assert!((at(3840.0, 1080.0, TextSize::M, p).scale - 1.0).abs() < 1e-6);
        // XL at 2560 is 2022 effective: hardly larger.
        assert!(at(2560.0, 1440.0, TextSize::Xl, p).scale <= 1.0 + 1e-6);
        // Monotonic in the width.
        let mut last = 0.0;
        for w in (1900..=4000).step_by(20) {
            let s = at(w as f32, 2160.0, TextSize::M, p).scale;
            assert!(s >= last, "{w}");
            last = s;
        }
    }

    /// A finger's target never shrinks below 44 (§2.4, S4-2); a pointer's
    /// control may go to 36 at the small steps.
    #[test]
    fn a_touch_target_is_never_under_forty_four() {
        for step in TextSize::ALL {
            for (w, h) in [(844.0, 390.0), (1180.0, 820.0), (1920.0, 1080.0)] {
                let touch = at(w, h, step, InputClass::Touch);
                assert!(touch.hit >= 44.0 && touch.control >= 44.0, "{step:?} {w}");
            }
        }
        assert!(
            (at(1920.0, 1080.0, TextSize::Xs, InputClass::Pointer).control - 36.0).abs() < 1e-3
        );
        assert!(
            (at(1920.0, 1080.0, TextSize::Xl, InputClass::Pointer).control - 55.704).abs() < 1e-3
        );
    }
}
