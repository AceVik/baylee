//! A clock beside each player's plate: the time that seat has left to
//! answer, at every seat at the table (owner, 08.10.2026: *„neben dem
//! Player-Details-Overlay, für ALLE Spieler sichtbar, das Uhr-Icon und
//! darunter oder daneben die Zeit, die der Spieler noch hat"*).
//!
//! **A small element of its own, beside the plate and not inside it.** The
//! plate is [`attached::Panel::Identity`]; this module only *reads* where it
//! stands ([`attached::pose`]) and its size ([`attached::Panel::size`]), and
//! stands its own box at the plate's right-hand side ([`beside`]), turned and
//! scaled with it. Nothing in the plate's tree or layout knows it is there,
//! so the plate can be redrawn without this file. The one place the two
//! meet is [`beside`].
//!
//! **Its own root and its own lifetime.** One [`PlateClock`] per seat, built
//! when the table's seats change and never on a view: a clock is shown and
//! hidden by `Display`, and its seconds are written in place. The plates'
//! tree is rebuilt on [`super::BarRevision`] and this one never is, so a
//! rebuild of a plate cannot drop its clock for a frame either.
//!
//! **Once a second.** [`tick_plate_clocks`] counts
//! [`Duel::seat_clocks`](crate::Duel::seat_clocks) down every frame past
//! change detection and writes a `Text`, a `TextColor` or a `Node` only
//! when what it holds differs: a count that has not crossed a whole second
//! writes nothing, and a camera standing still moves nothing.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::decisionclock::Urgency;

/// The clock's box, before the plate's scale: room for the icon and the
/// widest time a table may set (`60:00`, the gateway's hour).
const CLOCK_W: f32 = 92.0;
/// Its height.
const CLOCK_H: f32 = 30.0;
/// The air between the plate's edge and the clock, before the scale.
const GAP: f32 = 8.0;
/// The time's size.
const TIME_PT: f32 = 17.0;
/// The icon's size.
const ICON_PT: f32 = 14.0;

/// The root every plate clock hangs off. A sibling of the seat bars' root,
/// never a child, so their rebuilds cannot reach it.
#[derive(Component)]
pub struct PlateClockRoot {
    /// The seats it was built for, in view order.
    seats: Vec<PlayerId>,
}

/// One seat's clock beside its plate.
#[derive(Component)]
pub struct PlateClock {
    /// Whose time this is.
    pub player: PlayerId,
}

/// The time's text inside a [`PlateClock`], and its icon's: both inked by
/// [`Urgency`].
#[derive(Component)]
pub struct PlateClockInk {
    /// Whose clock.
    player: PlayerId,
    /// Whether this is the time (written each second) or the icon.
    time: bool,
}

/// The ink a clock is drawn in: the plate's own until the last minute,
/// then the active seat's amber, and in the last ten seconds the overlay's
/// danger — the sound's two thresholds, seen.
#[must_use]
pub fn ink(urgency: Urgency) -> Color {
    match urgency {
        Urgency::Calm => palette::INK,
        Urgency::Low => palette::ACTIVE,
        Urgency::Last => palette::DANGER,
    }
}

/// Where the clock stands, given where the plate stands: its box's top-left
/// before the turn, the turn and the scale, which are the plate's.
///
/// The plate's own pose ([`attached::pose`]) is a box `plate` big, turned
/// and scaled about its middle; the clock's middle is that middle moved
/// along the plate's own row by half the plate, the gap and half the clock,
/// all at the plate's scale. **The one line that says where the clock
/// attaches:** change the offset here to stand it elsewhere (below the
/// plate is `Vec2::new(0.0, plate.y * 0.5 + GAP + CLOCK_H * 0.5)`).
#[must_use]
pub fn beside(plate_corner: Vec2, plate: Vec2, tilt: f32, scale: f32) -> Vec2 {
    let middle = plate_corner + plate * 0.5;
    let along = Vec2::new(plate.x * 0.5 + GAP + CLOCK_W * 0.5, 0.0);
    middle + Rot2::radians(tilt) * along * scale - Vec2::new(CLOCK_W, CLOCK_H) * 0.5
}

/// Builds one clock per seat when the table's seats change: a hidden box
/// per seat, shown by [`tick_plate_clocks`] while that seat is on a clock.
pub fn sync_plate_clocks(
    mut commands: Commands,
    duel: Res<Duel>,
    roots: Query<(Entity, &PlateClockRoot)>,
    fonts: Res<UiFonts>,
) {
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    let seats: Vec<PlayerId> = view.seats.iter().map(|s| s.player).collect();
    if roots.iter().any(|(_, root)| root.seats == seats) {
        return;
    }
    for (entity, _) in &roots {
        commands.entity(entity).despawn();
    }
    let root = commands
        .spawn((
            PlateClockRoot {
                seats: seats.clone(),
            },
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Pickable::IGNORE,
            // With the plates, under everything the player summons over the
            // table (the seat bars' `GlobalZIndex(-1)` and its reason).
            GlobalZIndex(-1),
        ))
        .id();
    for player in seats {
        let clock = commands
            .spawn((
                PlateClock { player },
                UiTransform::default(),
                Node {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    width: px(CLOCK_W),
                    height: px(CLOCK_H),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: px(6),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(CLOCK_H * 0.5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                BorderColor::all(palette::DOCK_EDGE.with_alpha(0.35)),
                Pickable::IGNORE,
                children![
                    (
                        PlateClockInk {
                            player,
                            time: false,
                        },
                        Text::new(glyph::CLOCK.to_string()),
                        icon_tf(&fonts, ICON_PT),
                        TextColor(ink(Urgency::Calm)),
                        Pickable::IGNORE,
                    ),
                    (
                        PlateClockInk { player, time: true },
                        Text::default(),
                        tf_bold(&fonts, TIME_PT),
                        TextColor(ink(Urgency::Calm)),
                        Pickable::IGNORE,
                    ),
                ],
            ))
            .id();
        commands.entity(root).add_child(clock);
    }
}

/// Counts every seat's clock down and stands each beside its plate.
///
/// Hidden where the seat is on no clock, and wherever its plate is not
/// drawn ([`attached::pose`] says `None`: a tear, a shelf off screen, a
/// plate under the hand zone), because a clock with no plate beside it
/// names nobody.
#[allow(clippy::type_complexity)] // two disjoint queries over one clock's parts
pub fn tick_plate_clocks(
    time: Res<Time>,
    mut duel: ResMut<Duel>,
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    mut clocks: Query<(&PlateClock, &mut Node, &mut UiTransform)>,
    mut inks: Query<(&PlateClockInk, &mut Text, &mut TextColor)>,
) {
    // Past change detection, as `count_down_the_decision` counts its own: a
    // clock ticking is not the duel changing.
    duel.bypass_change_detection()
        .seat_clocks
        .advance(time.delta_secs());
    let lens = shown.rig().zip(windows.single().ok()).map(|(rig, window)| {
        crate::table::Lens::new(rig, Vec2::new(window.width(), window.height()))
    });
    for (clock, mut node, mut turn) in &mut clocks {
        let label = duel.seat_clocks.label(clock.player);
        let pose = label.as_ref().and_then(|_| {
            attached::pose(
                &duel,
                lens.as_ref(),
                clock.player,
                attached::Panel::Identity,
            )
        });
        let Some((corner, tilt, scale)) = pose else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        let at = beside(corner, attached::Panel::Identity.size(), tilt, scale);
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
        if node.left != px(at.x) {
            node.left = px(at.x);
        }
        if node.top != px(at.y) {
            node.top = px(at.y);
        }
        let rotation = Rot2::radians(tilt);
        if turn.rotation != rotation {
            turn.rotation = rotation;
        }
        if turn.scale != Vec2::splat(scale) {
            turn.scale = Vec2::splat(scale);
        }
    }
    for (part, mut text, mut colour) in &mut inks {
        let Some((says, urgency)) = duel.seat_clocks.label(part.player) else {
            continue;
        };
        if part.time && text.0 != says.as_str() {
            says.write_into(&mut text.0);
        }
        let want = ink(urgency);
        if colour.0 != want {
            colour.0 = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clock stands to the right of the plate, clear of it, its middle
    /// on the plate's middle line — upright, and turned with the plate.
    #[test]
    fn the_clock_stands_beside_the_plate_and_turns_with_it() {
        let plate = Vec2::new(240.0, 60.0);
        let corner = Vec2::new(100.0, 400.0);
        let at = beside(corner, plate, 0.0, 1.0);
        let middle = at + Vec2::new(CLOCK_W, CLOCK_H) * 0.5;
        assert!((middle.y - (corner.y + plate.y * 0.5)).abs() < 1e-3);
        assert!(
            at.x >= corner.x + plate.x + GAP - 1e-3,
            "the clock overlaps its plate: {at} beside {corner}"
        );

        // Half the size: the gap and the offset halve with it.
        let small = beside(corner, plate, 0.0, 0.5);
        let small_middle = small + Vec2::new(CLOCK_W, CLOCK_H) * 0.5;
        let plate_middle = corner + plate * 0.5;
        assert!(
            (small_middle.x - plate_middle.x - (plate.x * 0.5 + GAP + CLOCK_W * 0.5) * 0.5).abs()
                < 1e-3
        );

        // A plate turned a quarter stands its clock below its middle.
        let turned = beside(corner, plate, std::f32::consts::FRAC_PI_2, 1.0);
        let turned_middle = turned + Vec2::new(CLOCK_W, CLOCK_H) * 0.5;
        assert!((turned_middle.x - plate_middle.x).abs() < 1e-3);
        assert!(turned_middle.y > plate_middle.y + plate.x * 0.5);
    }

    /// The table this client draws for seat 0 against seat 1, in a default
    /// window, with the camera at home and `clocks` on the view.
    fn table(clocks: Vec<baylee_view::SeatClock>) -> App {
        use crate::table::{CameraRig, Canvas, ShownRig};
        use baylee_client_core::layout::TableLayout;
        let mut app = App::new();
        let window = Window::default();
        let size = Vec2::new(window.width(), window.height());
        app.world_mut().spawn(window);
        let canvas = Canvas::hud(size);
        let seats = [PlayerId::new(0), PlayerId::new(1)];
        let layout = TableLayout::new(&seats, canvas.aspect(), None);
        let rig = CameraRig::home(&layout, canvas);
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.clocks = clocks;
        let mut duel = Duel {
            layout: Some(layout),
            ..Duel::default()
        };
        duel.seat_clocks.sync(&view.clocks);
        duel.view = Some(view);
        app.insert_resource(duel)
            .insert_resource(ShownRig::standing(rig))
            .insert_resource(Time::<()>::default())
            .insert_resource(UiFonts {
                text: default(),
                medium: default(),
                bold: default(),
                italic: default(),
                medium_italic: default(),
                serif: default(),
                serif_italic: default(),
                icons: default(),
                mana: default(),
            })
            .init_resource::<Wrote>()
            .add_systems(
                Update,
                (sync_plate_clocks, tick_plate_clocks, watch).chain(),
            );
        app
    }

    /// Whether a clock's time was written this frame, read in the frame
    /// itself: a change tick is visible only against the run it happened in.
    #[derive(Resource, Default)]
    struct Wrote(bool);

    fn watch(inks: Query<(&PlateClockInk, Ref<Text>)>, mut wrote: ResMut<Wrote>) {
        wrote.0 = inks.iter().any(|(ink, text)| ink.time && text.is_changed());
    }

    /// `seat`'s clock: whether it is drawn, where, and what it says.
    fn clock_of(app: &mut App, seat: u8) -> (Display, Vec2, String) {
        let player = PlayerId::new(seat);
        let mut clocks = app.world_mut().query::<(&PlateClock, &Node)>();
        let (display, at) = clocks
            .iter(app.world())
            .find(|(clock, _)| clock.player == player)
            .map(|(_, node)| {
                let px = |v: Val| match v {
                    Val::Px(px) => px,
                    _ => f32::NAN,
                };
                (node.display, Vec2::new(px(node.left), px(node.top)))
            })
            .expect("every seat has a clock built");
        let mut inks = app.world_mut().query::<(&PlateClockInk, &Text)>();
        let says = inks
            .iter(app.world())
            .find(|(ink, _)| ink.player == player && ink.time)
            .map(|(_, text)| text.0.clone())
            .expect("the clock has a time");
        (display, at, says)
    }

    /// The owner's ask, at the seat that is *not* deciding: the opponent's
    /// clock stands beside the opponent's plate with the time it has left,
    /// and a seat on no clock draws none. Counted down between views, and
    /// written only when the second turns.
    #[test]
    fn an_opponents_clock_stands_beside_its_plate_and_counts_down() {
        let mut app = table(vec![baylee_view::SeatClock {
            seat: PlayerId::new(1),
            remaining_ms: 180_000,
        }]);
        app.update();
        app.update();
        let (display, at, says) = clock_of(&mut app, 1);
        assert_eq!(display, Display::Flex, "the opponent's clock is not drawn");
        assert_eq!(says, "3:00");
        let (none, _, _) = clock_of(&mut app, 0);
        assert_eq!(none, Display::None, "a seat on no clock was drawn one");

        // Beside the opponent's plate, as that plate stands this frame.
        let duel = app.world().resource::<Duel>();
        let rig = app
            .world()
            .resource::<crate::table::ShownRig>()
            .rig()
            .expect("a rig");
        let lens = crate::table::Lens::new(rig, Vec2::new(1280.0, 720.0));
        let (corner, tilt, scale) = attached::pose(
            duel,
            Some(&lens),
            PlayerId::new(1),
            attached::Panel::Identity,
        )
        .expect("the opponent's plate is drawn");
        assert_eq!(
            at,
            beside(corner, attached::Panel::Identity.size(), tilt, scale)
        );

        // A second and a half later: written once, as 2:59.
        let tick = |app: &mut App, secs: f32| {
            app.world_mut()
                .resource_mut::<Time<()>>()
                .advance_by(std::time::Duration::from_secs_f32(secs));
            app.update();
        };
        let changed = |app: &mut App| app.world().resource::<Wrote>().0;
        tick(&mut app, 0.4);
        assert!(!changed(&mut app), "a second that did not turn was written");
        tick(&mut app, 0.7);
        assert_eq!(clock_of(&mut app, 1).2, "2:59");
        assert!(
            changed(&mut app),
            "the second turned and nothing was written"
        );

        // The next view without it takes the clock away.
        app.world_mut().resource_mut::<Duel>().seat_clocks.sync(&[]);
        tick(&mut app, 0.0);
        assert_eq!(clock_of(&mut app, 1).0, Display::None);
    }

    /// The join: a view's clocks reach the count the plates read, and the
    /// next view replaces them.
    #[test]
    fn a_views_clocks_are_what_the_plates_count() {
        let mut duel = Duel::default();
        let mut view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        view.clocks = vec![baylee_view::SeatClock {
            seat: PlayerId::new(1),
            remaining_ms: 95_000,
        }];
        duel.receive_view(view);
        let (says, urgency) = duel
            .seat_clocks
            .label(PlayerId::new(1))
            .expect("seat 1 is on a clock");
        assert_eq!((says.as_str(), urgency), ("1:35", Urgency::Calm));
        let mut next = baylee_client_core::test_support::ViewBuilder::new(2).build();
        next.seq += 1;
        duel.receive_view(next);
        assert!(duel.seat_clocks.label(PlayerId::new(1)).is_none());
    }

    /// The ink turns at the sound's thresholds and is the plate's own until
    /// then.
    #[test]
    fn the_ink_turns_at_the_minute_and_at_ten_seconds() {
        assert_eq!(ink(Urgency::of(180.0)), palette::INK);
        assert_eq!(ink(Urgency::of(60.0)), palette::ACTIVE);
        assert_eq!(ink(Urgency::of(10.0)), palette::DANGER);
    }
}
