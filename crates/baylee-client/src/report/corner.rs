//! The report button in the table's top-right corner (#309).
//!
//! The owner's: at the table the way to report a problem is a button of its
//! own, always in reach and over everything, the end screen included. The
//! game menu's row and `F8` stay; this is the one a player finds without
//! knowing either.
//!
//! A root of its own at [`G_CORNER`], so it is ordered against the other
//! roots and not inside the overlay: the end screen is a root over the
//! overlay, and a button that lived in the overlay would be under it. Its
//! square is [`crate::hud::report_corner`], and every panel that used to
//! reach the corner now stands under [`crate::hud::TOP_CLEAR`]; no seat's
//! place on the table reaches it (`camera_tests`).

use bevy::prelude::*;

use super::ReportDesk;
use crate::DuelPhase;
use crate::ambience::Feel;
use crate::hud::{CORNER_BUTTON, EDGE, UiFonts, btn_radius, icon_tf, palette};

/// The button's root rung: over every root the table and the lobby draw
/// (the end screen at 1, the loading veil at 400, the arrival at 500) and
/// under the form it opens (1000).
pub(crate) const G_CORNER: i32 = 900;

/// A bug, from the shipped `fa-solid-900.ttf` (U+F188, "bug"): the mark for
/// "report a problem" a player has met elsewhere.
pub(crate) const BUG: char = '\u{f188}';

/// The glyph's size in the square, the burger's.
const BUG_PT: f32 = 12.0;

/// The button.
#[derive(Component)]
pub(crate) struct ReportCorner;

/// Stands the button up while a table is up, playing or finished, and takes
/// it down otherwise: the lobby has its own button beside the music.
pub(super) fn keep_the_corner(
    mut commands: Commands,
    phase: Option<Res<State<DuelPhase>>>,
    fonts: Option<Res<UiFonts>>,
    standing: Query<Entity, With<ReportCorner>>,
) {
    let wanted =
        phase.is_some_and(|phase| matches!(phase.get(), DuelPhase::Playing | DuelPhase::Finished));
    match (wanted, standing.iter().next()) {
        (true, None) => {
            if let Some(fonts) = fonts {
                spawn(&mut commands, &fonts);
            }
        }
        (false, Some(_)) => {
            for button in &standing {
                commands.entity(button).despawn();
            }
        }
        _ => {}
    }
}

fn spawn(commands: &mut Commands, fonts: &UiFonts) {
    let ground = palette::DIALOG;
    commands
        .spawn((
            ReportCorner,
            Node {
                position_type: PositionType::Absolute,
                right: px(EDGE),
                top: px(EDGE),
                width: px(CORNER_BUTTON),
                height: px(CORNER_BUTTON),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(palette::DIALOG_LINE),
            Button,
            Feel::new(ground),
            GlobalZIndex(G_CORNER),
            children![(
                Text::new(BUG.to_string()),
                icon_tf(fonts, BUG_PT),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .observe(pressed);
}

/// Opens the form, as `F8` and the menu's row do.
fn pressed(mut click: On<Pointer<Click>>, mut desk: ResMut<ReportDesk>) {
    click.propagate(false);
    desk.asked = true;
}
