//! The tray: the shelf's right-hand attachment, and the door the zone dialog
//! is put away behind.
//!
//! # The name, which collides
//!
//! `hud::tray` is **not** this. It is the zone dialog, and its name predates
//! the owner's word for the strip — *"Baue den Tray ein. Die Actions Bar ist
//! voll, aber an der Actions Bar hängt manchmal so ein Info text. Auf eine
//! ähnliche Art und Weise ist der Tray so ein Attachement an die Actions Bar
//! nur rechtsbündig und kleiner von der Höhe her"* (19.09.2026). The module
//! that draws the dialog should be `hud::zones`; renaming it is 409
//! occurrences across 26 source files and eight documents, it is nobody's
//! request, and it would bury this one. So the two words stand side by side
//! for now and this paragraph is the map: **`ledge::tray` is the strip,
//! `hud::tray` is the sheet it puts away.**
//!
//! # What it is
//!
//! The third retained attachment on this ledge, after [`super::drawer`] and
//! [`super::pool`], and it is one for the same reason both of those are: it
//! outlives [`super::sync_ledge`]'s rebuild, which despawns every child of
//! the shelf on every sentence. A button that came and went with the
//! question above it could not be *always visible*, which is the one thing
//! the owner asked of it.
//!
//! The drawer is the shape it was asked to be *like*, and the likeness is
//! exact where it matters and deliberate where it is not. Both hang off the
//! shelf's top edge with one pixel of overlap, so the attachment's bottom and
//! the shelf's lip are one line and not two; both are `Pickable::IGNORE` at
//! the root, because a node that answered the pointer across its whole width
//! would take every click aimed at the table under it. What differs is what
//! the owner named: the drawer is **centred** and grows as wide as its
//! sentence, the tray is **right-aligned** and does not grow at all, and the
//! tray is shorter — [`STRIP_H`] against a drawer that is as tall as what is
//! in it.
//!
//! There are two of these strips now. [`super::pool`] is the same node with
//! `left` where this one has `right`, because the owner asked for the mana
//! pool *as* this — *"Es soll symetrisch zum Tray aussehen nur auf der
//! linken Seite"* — so [`super::strip_node`] is what both of them spawn and
//! the four numbers that decide a strip's shape live one level up. This file
//! keeps only what is the tray's own: the gap between its buttons, the
//! button's own side, and the icon in it.
//!
//! And it differs in one rung of the z-ladder, which is the only place the
//! likeness had to be broken. The drawer sits at [`Z_LEDGE`], under the
//! sheet; the tray sits at [`Z_TRAY`], over it. A sheet is placed in a band
//! that stops [`EDGE`] above the hand zone and this strip is seventeen
//! pixels taller than that gap, so a maximised sheet covers most of the
//! button — and *"Demnach ist der Button im Tray immer sichtbar"* is the one
//! thing the owner asked of it. Lifting the strip costs nothing; shortening
//! the band would cost every sheet the height of a strip standing at one end
//! of it.
//!
//! # Why the button is a door and not a way out
//!
//! The zone dialog used to be *closed*. It is now *minimised*, and the
//! difference is not in [`Browser::close`] — which has always kept the ticks,
//! the filter and the placement — but in whether anything advertises that.
//! A window with an `✕` and no taskbar entry has been dismissed; the same
//! window with a button still standing on the shelf has been put down. So the
//! state is unchanged and the vocabulary around it is not, which is why
//! [`Browser::toggle_by_hand`] is one method and not two.
//!
//! The button is **held** while a question owns the sheet
//! ([`Browser::may_be_put_away`]). A question opens this dialog because one of
//! its answers is in a zone the table cannot show, so a tray button that put
//! it away would leave the player holding a question with no way to answer it.
//!
//! [`Browser::close`]: baylee_client_core::browser::Browser::close
//! [`Browser::toggle_by_hand`]: baylee_client_core::browser::Browser::toggle_by_hand
//! [`Browser::may_be_put_away`]: baylee_client_core::browser::Browser::may_be_put_away

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// The gap between two buttons in the strip.
///
/// There is one button today. The owner has already named the second —
/// *"hier kommen noch mehr Buttons rein"* is said of the burger menu beside
/// it — so the row is a row from the start rather than a node that has to be
/// turned into one later.
const TRAY_GAP: f32 = 6.0;

/// A button in the strip: square, and the same height as the strip's inside.
const TRAY_BTN: f32 = BUTTON_H - 4.0;

/// The icon in a tray button.
const TRAY_ICON_PT: f32 = 12.0;

/// The glyph on a button the player may press.
const FREE_INK: Color = palette::DIALOG_SOFT;

/// And on one a question is holding.
///
/// [`palette::LEDGE_DEAD`], whose own doc names this case — *"a draw offer the
/// engine would refuse"* — and which is held to 3.08 : 1 rather than the 4.5
/// prose clears, so it reads as present and not as something to attend to.
///
/// It was [`palette::LEDGE_SOFT`] for one build, chosen by analogy with the
/// shelf and never measured. That is the *ledge's own* ink and is **brighter**
/// than `DIALOG_SOFT`: photographed in the running client, the held button's
/// glyph came out at 150 against the free button's 132, so the control that
/// could not be pressed was the louder of the two. `a_held_button_is_quieter`
/// is that measurement turned into a bound, because the pair would read as
/// deliberate to anyone who did not take the picture.
const HELD_INK: Color = palette::LEDGE_DEAD;

/// The retained strip.
#[derive(Component)]
pub struct TrayStrip;

/// The button that puts the zone dialog up, or puts it away.
#[derive(Component)]
pub struct TrayZones;

/// What the strip was last drawn from.
///
/// The fourth revision counter in this client and the second on this ledge,
/// for [`super::pool`]'s reason: the strip changes on the browser's clock and
/// the shelf changes on the question's, and one counter would have to lie
/// about one of them.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct StripRevision {
    /// Whether the sheet is up. The button says which way it points.
    open: bool,
    /// Whether the button may be pressed at all.
    free: bool,
}

/// Where the strip stands: on the shelf's top edge, against the right margin.
///
/// Its own function so a test can read it without an app, which is what
/// [`super::drawer::root_node`] has for the same reason.
pub(in crate::hud) fn root_node() -> Node {
    Node {
        column_gap: px(TRAY_GAP),
        ..strip_node(StripSide::Right)
    }
}

/// The middle of the zones button, in the coordinates a sheet is placed in.
///
/// This is where the dialog goes when it is put away and where it comes back
/// from — see `hud::tray::reveal_tray`. It is derived from [`root_node`]'s
/// own numbers rather than stated again, because the two have to agree about
/// one point on the screen and a second copy of `STRIP_PAD` is exactly how
/// they would stop agreeing.
///
/// The band is the rectangle `hud::band_of` returns: the window, less
/// [`EDGE`] at the top and the hand zone at the bottom. So the strip's height
/// is measured **up from the band's bottom edge**, which is the same edge the
/// shelf's lip stands on — the strip overlaps it by a pixel and the button
/// sits [`STRIP_PAD`] inside that. Horizontally the band *is* the window, so
/// the button's centre is its own inset from the right.
///
/// It is the button's middle and not the strip's, because the strip is a row
/// that will grow more buttons and the sheet belongs to this one. With one
/// button today the two happen to differ by [`STRIP_PAD`], which is a
/// coincidence and not a shortcut worth taking.
///
/// The one place it is wrong is a window too short for `Placement::MIN_H`,
/// where `band_of` clamps the height it returns and the band no longer ends
/// where the shelf does. The endpoint is then a few pixels out — it is where
/// a movement *aims*, not where anything is drawn, so a short window costs a
/// slightly crooked flight and nothing else.
pub(in crate::hud) fn zones_button_centre(band: (f32, f32)) -> Vec2 {
    Vec2::new(
        band.0 - EDGE - STRIP_PAD - TRAY_BTN / 2.0,
        band.1 - (STRIP_H - STRIP_LIP) + STRIP_PAD + TRAY_BTN / 2.0,
    )
}

/// Spawns the strip, once, beside the shelf.
pub(in crate::hud) fn spawn_tray_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TrayStrip,
            root_node(),
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            ZIndex(Z_TRAY),
            // Unlike the drawer's root this node is exactly as wide as what
            // is in it, so it could answer the pointer without stealing
            // anything — but it is still ignored, because the *button* is the
            // control and a press on the strip's padding is a press on
            // nothing. The button below is pickable in its own right.
            Pickable::IGNORE,
        ))
        .id()
}

/// Fills the strip, or redraws it when what its buttons say has changed.
///
/// Runs on [`StripRevision`] rather than on every frame for the reason every
/// retained node here does: the children are despawned and rebuilt, and
/// rebuilding them under the pointer would take the hover with it on each
/// frame.
pub fn sync_tray_strip(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    mut revision: ResMut<StripRevision>,
    strip: Query<(Entity, Option<&Children>), With<TrayStrip>>,
) {
    let Ok((strip, kids)) = strip.single() else {
        // No strip yet — the overlay has not built its root. Nothing of ours
        // can be standing, so the gate is left untouched and the next frame
        // that has one builds on it.
        return;
    };
    let next = StripRevision {
        open: duel.browser.is_open(),
        free: duel.browser.may_be_put_away(),
    };
    // The counter alone is not enough, and the reason is the same one
    // `sync_tray`'s `drawn` records: the overlay can take this node away and
    // spawn a fresh one — a game that ends, a root rebuilt — and a counter
    // that still described the old strip would leave the new one empty until
    // something about the browser happened to change. The world is asked
    // instead of a third field being kept in step with it.
    let drawn = kids.is_some_and(|kids| !kids.is_empty());
    if *revision == next && drawn {
        return;
    }
    *revision = next;
    if let Some(kids) = kids {
        for kid in kids {
            commands.entity(*kid).despawn();
        }
    }
    let zones = zone_button(&mut commands, &fonts, next.open, next.free);
    commands.entity(strip).add_child(zones);
}

/// The one button the strip has: the zone dialog's door.
///
/// Its icon is an archive box rather than the layer-group the seat bars draw
/// for a library, which would be the obvious picture of "the piles" and is
/// taken. What the dialog shows is the cards a game has put *away* —
/// graveyards, exile, the command zone, the stack — so the box is not a
/// second-best: it is the more exact of the two.
///
/// `open` turns it, `free` holds it. A held button keeps its ground and loses
/// its [`Feel`] and its `Button`, which is the same three-part answer the
/// unlit Confirm on the shelf gives: it is visibly there, visibly not now,
/// and it does not light under the pointer to promise otherwise.
fn zone_button(commands: &mut Commands, fonts: &UiFonts, open: bool, free: bool) -> Entity {
    let ground = if open {
        palette::DIALOG_LIT
    } else {
        palette::DIALOG
    };
    let ink = if free { FREE_INK } else { HELD_INK };
    let button = commands
        .spawn((
            Node {
                width: px(TRAY_BTN),
                height: px(TRAY_BTN),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(palette::DIALOG_LINE),
            children![(
                Text::new(glyph::ZONES.to_string()),
                icon_tf(fonts, TRAY_ICON_PT),
                TextColor(ink),
                Pickable::IGNORE,
            )],
        ))
        .id();
    if free {
        commands
            .entity(button)
            .insert((TrayZones, Button, Feel::new(ground)));
    }
    button
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The strip is over the sheet and under the preview, and both halves
    /// are the assertion.
    ///
    /// A bound on one side only would pass with the strip at `Z_PREVIEW` and
    /// standing over a card's own description, which is the drawing this
    /// ladder exists to order.
    #[test]
    fn the_tray_stands_over_the_sheet_it_puts_away_and_under_the_preview() {
        const {
            assert!(
                Z_TRAY > Z_SHEET,
                "a maximised sheet covers the button the owner asked to always see"
            );
            assert!(
                Z_TRAY < Z_PREVIEW,
                "the strip stands over the description of whatever is under the pointer"
            );
        }
    }

    /// The likeness the owner asked for is one number, and it is this one:
    /// both attachments hang off the same edge of the same shelf. A pixel
    /// between them would draw as a step in the lip they share.
    #[test]
    fn the_tray_hangs_off_the_same_edge_as_the_drawer() {
        assert_eq!(
            root_node().bottom,
            super::super::drawer::root_node().bottom,
            "the tray and the drawer would meet the shelf on two lines"
        );
    }

    /// *"nur rechtsbündig und kleiner von der Höhe her"*, as two assertions.
    #[test]
    fn the_tray_is_right_aligned_and_shorter_than_the_shelf() {
        let node = root_node();
        assert_eq!(node.right, px(EDGE), "the tray is not against the margin");
        assert_eq!(node.left, Val::Auto, "a left edge would stretch it wide");
        let Val::Px(h) = node.height else {
            panic!("the strip's height is not a pixel count");
        };
        assert!(
            h < hand::LEDGE_H,
            "the tray is {h} tall against a shelf of {}, which is not smaller",
            hand::LEDGE_H
        );
        // And not so much smaller that the button inside it is clipped: the
        // strip is drawn *around* a control, so the bound is two-sided for
        // the reason every measurement in this client is.
        assert!(
            h >= TRAY_BTN + 2.0 * STRIP_PAD,
            "a {TRAY_BTN}px button does not fit in {h}px of strip"
        );
    }

    /// A held button is quieter than a free one, in that direction.
    ///
    /// The bound is one-sided on purpose and the side is the whole point: two
    /// inks that merely *differ* is what shipped, with the held one brighter.
    /// Relative luminance rather than the sRGB triple, because that is what
    /// "quieter" means to an eye and the three channels do not move together.
    #[test]
    fn a_held_button_is_quieter_than_one_that_can_be_pressed() {
        fn luminance(color: Color) -> f32 {
            let c = color.to_linear();
            0.2126 * c.red + 0.7152 * c.green + 0.0722 * c.blue
        }
        let free = luminance(FREE_INK);
        let held = luminance(HELD_INK);
        assert!(
            held < free,
            "the button a question is holding draws at {held} against {free}              for one that can be pressed, so the dead control is the louder"
        );
    }

    /// A question owning the sheet holds the button, and a held button is not
    /// a `Button` at all — so a click cannot reach the handler that would
    /// have to refuse it.
    #[test]
    fn a_held_button_is_not_a_control() {
        let mut app = App::new();
        let fonts = UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        };
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let (held, free) = {
            let mut commands = Commands::new(&mut queue, app.world());
            (
                zone_button(&mut commands, &fonts, true, false),
                zone_button(&mut commands, &fonts, true, true),
            )
        };
        queue.apply(app.world_mut());
        assert!(
            app.world().get::<Button>(held).is_none(),
            "a held tray button still answers the pointer"
        );
        assert!(
            app.world().get::<Button>(free).is_some(),
            "a free tray button answers nothing"
        );
        assert!(
            app.world().get::<TrayZones>(held).is_none(),
            "a held button carries the marker the click handler looks for"
        );
    }

    /// The point the sheet flies to is the point the button is drawn at.
    ///
    /// Two numbers agreeing about one place on the screen, arrived at from
    /// two directions: [`root_node`] lays the strip out for `bevy_ui` and
    /// [`zones_button_centre`] computes where that puts the button, and
    /// nothing in a running client would ever notice them disagreeing — a
    /// sheet aimed twenty pixels past the button still reads as a sheet going
    /// away. So the layout is read back out of the `Node` rather than
    /// restated, which is what makes this a measurement and not a copy.
    #[test]
    fn the_sheet_is_aimed_at_the_button_the_layout_actually_draws() {
        /// The band a `1200 x 800` window leaves, which is `hud::band_of`'s
        /// own arithmetic and is written out here rather than called: the
        /// function takes a `Query<&Window>` and this test has no world.
        const BAND: (f32, f32) = (1200.0, 800.0 - EDGE - hand::HAND_ZONE_H);

        let node = root_node();
        let Val::Px(right) = node.right else {
            panic!("the strip is placed in pixels or this test reads nothing");
        };
        let Val::Px(bottom) = node.bottom else {
            panic!("the strip is placed in pixels or this test reads nothing");
        };
        let Val::Px(height) = node.height else {
            panic!("the strip is placed in pixels or this test reads nothing");
        };

        // Where the layout puts the button, from the window's own edges.
        let want_x = BAND.0 - right - STRIP_PAD - TRAY_BTN / 2.0;
        // `bottom` is measured up from the window's bottom edge and the band
        // stops `hand::HAND_ZONE_H` above it, so the strip's own top is this
        // far down the band.
        let strip_top = BAND.1 - (bottom - hand::HAND_ZONE_H) - height;
        let want_y = strip_top + STRIP_PAD + TRAY_BTN / 2.0;

        let got = zones_button_centre(BAND);
        assert!(
            (got.x - want_x).abs() < 0.01 && (got.y - want_y).abs() < 0.01,
            "the flight is aimed at {got:?}, the button is drawn at \
             ({want_x}, {want_y})"
        );

        // And the counter-test, which is what says the agreement above is
        // about this strip and not about two constants that happen to be
        // zero: the button is inside the band, near its bottom-right corner,
        // and not at the origin a defaulted `Vec2` would give.
        assert!(
            got.x > BAND.0 * 0.9 && got.y > BAND.1 - STRIP_H,
            "the tray is bottom-right and this is {got:?} in a {BAND:?} band"
        );
    }
}
