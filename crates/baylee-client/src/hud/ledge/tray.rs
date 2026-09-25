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
//! # Where it stands: in the bar, beside the menu (#264)
//!
//! It was a strip hanging off the shelf's right end, the drawer's shorter
//! and right-aligned cousin. On 25.09.2026 the owner took the strips'
//! places for other things — *"On the right there are the log and zones
//! dialog buttons, put them left from the menu button at the right side
//! (sticked to right) into the actions bar"* — so the two buttons now stand
//! **on** the shelf, in its button row, directly left of the burger
//! ([`super::menu::burger`]) and the same size and ground as it: three doors
//! at the bar's right end that read as one set. The mana pool took the
//! right-hand strip ([`super::pool`]) and the players the left.
//!
//! It is still its own retained node and not a child the shelf rebuilds, for
//! the reason above: an absolute node in the HUD root, laid over the shelf's
//! row, whose box is exactly its two buttons. The shelf keeps the room for
//! it free — [`WIDTH`] is counted into the right column's reserve — so the
//! question in the middle cannot slide under it.
//!
//! It stands one rung up the z-ladder, at [`Z_TRAY`], over the sheet. As a
//! strip that was what kept it visible: a sheet is placed in a band that
//! stops [`EDGE`] above the hand zone, the strip stood seventeen pixels
//! taller than that gap, and *"Demnach ist der Button im Tray immer
//! sichtbar"* is the one thing the owner asked of it. In the bar no sheet
//! reaches it at all, so the rung is now a margin rather than the guard; it
//! stays, because a node over the sheet cannot be covered by one whatever
//! the band does next.
//!
//! **That sentence is about a sheet and about nothing else**, and it stood
//! here without its scope long enough to be read the other way: as a claim
//! that nothing may ever be drawn over this button. It cost a design decision
//! before the owner said what it meant — the game menu's panel was rejected
//! on it — so the clarification is recorded here and beside [`Z_TRAY`]:
//! *"Es bedeutet nicht, dass dieser immer im Viewport sein muss, sondern dass
//! er immer irgend wie erreichbar sein soll um den Zonen-Dialog jeder Zeit
//! öffnen zu können. Er ist ein ganz gewöhnlicher Button wo aus ausklapbares
//! Menü (Submenü vom Burger) durchaus drüber darf"* (19.09.2026).
//!
//! The rule is **reachable**, not visible. A panel the next press dismisses
//! may stand over this button; the sheet may not, because the sheet is the
//! thing the button is the way back from — a door covered by what is behind
//! it is not a door.
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

/// The gap between two buttons in the row, and between the row and the
/// burger to its right.
///
/// There are two now: the game log's scroll (#262) and, at the right end,
/// the zones. The row was a row from the start, for the owner's *"hier kommen
/// noch mehr Buttons rein"*, so the second needed no change to the strip.
const TRAY_GAP: f32 = 6.0;

/// A button in the row: square, and the burger's size, so the three doors at
/// the bar's right end are one set.
const TRAY_BTN: f32 = menu::BURGER;

/// How much of the shelf's right end the row takes, with the gap that parts
/// it from the burger: what the right column reserves for it.
pub(super) const WIDTH: f32 = 2.0 * TRAY_BTN + 2.0 * TRAY_GAP;

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
    /// Whether the game log's panel is up (#262). Its button says so the
    /// way the zones button does.
    log: bool,
}

/// Where the row stands: in the shelf's button row, directly left of the
/// burger.
///
/// Its own function so a test can read it without an app, which is what
/// [`super::drawer::root_node`] has for the same reason.
pub(in crate::hud) fn root_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        // The shelf's own row: its bottom padding up from the shelf's
        // bottom, which is the hand zone's height less the shelf's.
        bottom: px(hand::HAND_ZONE_H - hand::LEDGE_H + LEDGE_PAD_Y),
        right: px(EDGE + menu::BURGER + TRAY_GAP),
        height: px(BUTTON_H),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(TRAY_GAP),
        ..default()
    }
}

/// The middle of the zones button, in the coordinates a sheet is placed in.
///
/// This is where the dialog goes when it is put away and where it comes back
/// from — see `hud::tray::reveal_tray`. It is derived from [`root_node`]'s
/// own numbers rather than stated again, because the two have to agree about
/// one point on the screen and a second copy of the burger's width is
/// exactly how they would stop agreeing.
///
/// The band is the rectangle `hud::band_of` returns: the window, less
/// [`EDGE`] at the top and the hand zone at the bottom. So the button is
/// measured **down from the band's bottom edge**, which is the same edge the
/// shelf's lip stands on: the shelf's top padding (the lip in it) and half a
/// button. Horizontally the band *is* the window, so the button's centre is
/// its own inset from the right, past the burger.
///
/// It is the button's middle and not the row's, because the row holds two
/// buttons and the sheet belongs to this one. The zones are the row's right
/// end, which is what lets this count from the burger; the log's scroll
/// stands to their left for that reason.
///
/// The one place it is wrong is a window too short for `Placement::MIN_H`,
/// where `band_of` clamps the height it returns and the band no longer ends
/// where the shelf does. The endpoint is then a few pixels out — it is where
/// a movement *aims*, not where anything is drawn, so a short window costs a
/// slightly crooked flight and nothing else.
pub(in crate::hud) fn zones_button_centre(band: (f32, f32)) -> Vec2 {
    Vec2::new(
        band.0 - EDGE - menu::BURGER - TRAY_GAP - TRAY_BTN / 2.0,
        band.1 + LEDGE_PAD_Y + BUTTON_H / 2.0,
    )
}

/// Spawns the strip, once, beside the shelf.
pub(in crate::hud) fn spawn_tray_strip(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TrayStrip,
            root_node(),
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
        log: duel.log_open,
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
    // The log's scroll first, so the zones stay the strip's right end: the
    // sheet is put away into that button, and `zones_button_centre` counts
    // it from the right margin.
    let log = log_button(&mut commands, &fonts, next.log);
    let zones = zone_button(&mut commands, &fonts, next.open, next.free);
    commands.entity(strip).add_children(&[log, zones]);
}

/// The game log's door (#262): a scroll, beside the zones.
///
/// Never held. Nothing a question asks lives in the log, so there is no
/// moment it may not be opened or put away. `open` lights its ground the way
/// the zones button's is lit while the sheet is up.
fn log_button(commands: &mut Commands, fonts: &UiFonts, open: bool) -> Entity {
    let ground = if open {
        palette::DIALOG_LIT
    } else {
        palette::DIALOG
    };
    commands
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
            Button,
            Feel::new(ground),
            MenuButton {
                action: MenuAction::ToggleLog,
            },
            children![(
                Text::new(glyph::LOG.to_string()),
                icon_tf(fonts, TRAY_ICON_PT),
                TextColor(FREE_INK),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// The zone dialog's door, at the strip's right end.
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

    /// *"put them left from the menu button at the right side (sticked to
    /// right) into the actions bar"* (the owner, 25.09.2026), as the three
    /// things it says: in the bar's own row, against the burger, and the
    /// burger's size.
    #[test]
    fn the_doors_stand_in_the_bar_directly_left_of_the_burger() {
        let node = root_node();
        let (Val::Px(bottom), Val::Px(h)) = (node.bottom, node.height) else {
            panic!("the row is placed in pixels or this test reads nothing");
        };
        // Inside the shelf: its bottom over the shelf's bottom and its top
        // under the shelf's lip, which is where every answer on it stands.
        let shelf_bottom = hand::HAND_ZONE_H - hand::LEDGE_H;
        assert!(
            bottom >= shelf_bottom && bottom + h <= hand::HAND_ZONE_H - LIP,
            "the row runs {bottom}..{} against a shelf of {shelf_bottom}..{}",
            bottom + h,
            hand::HAND_ZONE_H
        );
        assert_eq!(
            node.bottom,
            px(hand::HAND_ZONE_H - hand::LEDGE_H + LEDGE_PAD_Y),
            "the doors are not on the shelf's own row"
        );
        // Against the burger, one gap from it, and not against the margin
        // where the burger is.
        assert_eq!(
            node.right,
            px(EDGE + menu::BURGER + TRAY_GAP),
            "the doors are not beside the burger"
        );
        assert_eq!(node.left, Val::Auto, "a left edge would stretch it wide");
        assert_eq!(
            px(TRAY_BTN),
            px(menu::BURGER),
            "three doors of two sizes are not a set"
        );
        assert_eq!(
            node.height,
            px(TRAY_BTN),
            "the row is its buttons and nothing round them"
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

    /// The log's scroll stands left of the zones (#262), so the zones stay
    /// the strip's right end: that is the button the sheet is put away into,
    /// and [`zones_button_centre`] counts it from the right margin.
    #[test]
    fn the_log_s_door_stands_left_of_the_zones() {
        let mut app = App::new();
        app.insert_resource(Duel::default())
            .insert_resource(UiFonts {
                text: Handle::default(),
                medium: Handle::default(),
                bold: Handle::default(),
                italic: Handle::default(),
                medium_italic: Handle::default(),
                serif: Handle::default(),
                serif_italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            })
            .init_resource::<StripRevision>()
            .add_systems(Update, sync_tray_strip);
        let strip = app.world_mut().spawn((TrayStrip, root_node())).id();
        app.update();
        let kids: Vec<Entity> = app
            .world()
            .get::<Children>(strip)
            .expect("the strip was filled")
            .iter()
            .collect();
        assert_eq!(
            kids.len(),
            2,
            "the strip holds the log's door and the zones"
        );
        assert!(
            app.world()
                .get::<MenuButton>(kids[0])
                .is_some_and(|b| b.action == MenuAction::ToggleLog),
            "the first button is not the log's"
        );
        assert!(
            app.world().get::<TrayZones>(kids[1]).is_some(),
            "the zones are not the strip's right end"
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

        // Where the layout puts the button, from the window's own edges: the
        // zones are the row's right end.
        let want_x = BAND.0 - right - TRAY_BTN / 2.0;
        // `bottom` is measured up from the window's bottom edge and the band
        // stops `hand::HAND_ZONE_H` above it, so the row's own top is this
        // far below the band's bottom, down in the shelf.
        let row_top = BAND.1 + (hand::HAND_ZONE_H - bottom) - height;
        let want_y = row_top + height / 2.0;

        let got = zones_button_centre(BAND);
        assert!(
            (got.x - want_x).abs() < 0.01 && (got.y - want_y).abs() < 0.01,
            "the flight is aimed at {got:?}, the button is drawn at \
             ({want_x}, {want_y})"
        );

        // And the counter-test, which is what says the agreement above is
        // about this row and not about two constants that happen to be
        // zero: the button is on the shelf, under the band's bottom-right
        // corner, and not at the origin a defaulted `Vec2` would give.
        assert!(
            got.x > BAND.0 * 0.9 && got.y > BAND.1 && got.y < BAND.1 + hand::LEDGE_H,
            "the doors are on the shelf's right end and this is {got:?} under a {BAND:?} band"
        );
    }
}
