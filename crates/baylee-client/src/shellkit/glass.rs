//! The client's one container look (owner, 26.11.2026: "dieses Glas-Ding"
//! of the sign-in card and the dialogs, for every container): a pane of
//! smoked blue glass lit from its head, framed by a cut-glass rim that
//! catches warm light on its upper edge and cool light on its sides.
//!
//! Every panel, sheet, dialog, menu and window wears it through [`pane`]
//! (or [`ground`] and [`rim`] where a surface keeps a rim of its own), so
//! there is one place it is decided. The Tower keeps its gilded frames:
//! they are the Tower's own, not a container's.

use bevy::prelude::*;
use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};

/// How much of what stands behind the glass shows through it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Density {
    /// Over the front door's painting, where a card is little more than a
    /// pane: the sign-in card and its small dialogs.
    Clear,
    /// Wherever words are read over the lobby's painting or a dimmed page:
    /// the same glass, denser, so the quietest ink keeps 4.5 : 1 even over
    /// the hearth's fire (`the_reading_glass_keeps_quiet_ink_over_the_fire`).
    Reading,
    /// Where the glass stands over other words with nothing dimming them:
    /// a menu, a popover. All but opaque, so no line shows through a line.
    Solid,
}

/// The glass's three tints, head to foot (its middle at 48 %).
#[must_use]
pub fn tints(density: Density) -> [Color; 3] {
    let [head, middle, foot] = match density {
        Density::Clear => [0.60, 0.54, 0.64],
        Density::Reading => [0.93, 0.90, 0.93],
        Density::Solid => [0.985, 0.98, 0.985],
    };
    [
        Color::srgba(0.080, 0.130, 0.200, head),
        Color::srgba(0.035, 0.068, 0.115, middle),
        Color::srgba(0.025, 0.050, 0.088, foot),
    ]
}

/// The glass itself, as a container's ground. Lay it over
/// `BackgroundColor(Color::NONE)`: the gradient is the whole ground.
#[must_use]
pub fn ground(density: Density) -> BackgroundGradient {
    let [head, middle, foot] = tints(density);
    BackgroundGradient::from(
        LinearGradient::to_bottom(vec![
            ColorStop::percent(head, 0.0),
            ColorStop::percent(middle, 48.0),
            ColorStop::percent(foot, 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

/// The cut-glass rim: warm at its head where the light falls, cool at its
/// sides, a shade darker at its foot.
#[must_use]
pub const fn rim() -> BorderColor {
    BorderColor {
        top: Color::srgba(0.86, 0.78, 0.62, 0.44),
        right: Color::srgba(0.58, 0.66, 0.76, 0.30),
        bottom: Color::srgba(0.46, 0.42, 0.34, 0.34),
        left: Color::srgba(0.58, 0.66, 0.76, 0.30),
    }
}

/// What a container wears: no flat fill under the glass (a second quad
/// would darken it), the glass, its rim and the card's shadow. The node
/// brings its own one-pixel border and its radius.
///
/// Not for a surface whose `BackgroundColor` is animated (a hover's
/// `Feel`): the gradient lies over the fill and would hide it.
#[must_use]
pub fn pane(density: Density) -> impl Bundle {
    (
        BackgroundColor(Color::NONE),
        ground(density),
        rim(),
        super::surfaces::elevation_card(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The denser glasses are the clear glass made denser: the same hues
    /// at every stop, only more of them, so all read as one material.
    #[test]
    fn every_density_is_one_glass() {
        for density in [Density::Reading, Density::Solid] {
            for (clear, denser) in tints(Density::Clear).into_iter().zip(tints(density)) {
                let (clear, denser) = (clear.to_srgba(), denser.to_srgba());
                assert_eq!(
                    (clear.red, clear.green, clear.blue),
                    (denser.red, denser.green, denser.blue)
                );
                assert!(denser.alpha > clear.alpha);
            }
        }
    }

    /// The reading glass lets the room through, so what comes through must
    /// not take the lines: over the tavern fire's orange, blended in linear
    /// light as the UI is, the quietest inks the lobby writes on it keep
    /// WCAG's 4.5 : 1 at every stop. The flat 0.66 the chat once had gave
    /// 2.5 : 1, which the last assertion holds on to.
    #[test]
    fn the_reading_glass_keeps_quiet_ink_over_the_fire() {
        let fire = Color::srgb(0.90, 0.50, 0.15).to_linear();
        let luminance = |c: LinearRgba| 0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue;
        let contrast = |ink: Color, glass: Color| {
            let over = glass.to_linear();
            let share = over.alpha;
            let under = LinearRgba::rgb(
                over.red * share + fire.red * (1.0 - share),
                over.green * share + fire.green * (1.0 - share),
                over.blue * share + fire.blue * (1.0 - share),
            );
            (luminance(ink.to_linear()) + 0.05) / (luminance(under) + 0.05)
        };
        let quiet = [
            super::super::tokens::MUTED,
            crate::social::rows::LOBBY_INKS.soft,
        ];
        for density in [Density::Reading, Density::Solid] {
            for glass in tints(density) {
                for ink in quiet {
                    let ratio = contrast(ink, glass);
                    assert!(ratio >= 4.5, "{density:?} {glass:?} {ink:?}: {ratio}");
                }
            }
        }
        // Glass, not a slab: the room still comes through the reading glass.
        assert!(
            tints(Density::Reading)
                .iter()
                .all(|glass| glass.alpha() < super::super::tokens::PANEL.alpha())
        );
        let flat = Color::srgba(0.035, 0.075, 0.105, 0.66);
        assert!(contrast(crate::social::rows::LOBBY_INKS.soft, flat) < 3.0);
    }
}
