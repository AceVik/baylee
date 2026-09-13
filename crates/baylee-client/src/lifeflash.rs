//! Drawing a life total that changed.
//!
//! [`baylee_client_core::lifeflash`] is the whole of the thinking — what
//! changed, by how much, whether two hits are one number, and how far through
//! its life the flash is. This is the twenty lines that put it on the screen,
//! and the two things it knows that the model cannot.
//!
//! # It is drawn in its own root, not on the bar
//!
//! The obvious home for `−3` over a seat's heart is a child of the life cell,
//! and it is the one place it cannot go: the seat bar is a retained tree
//! rebuilt whenever [`crate::hud::BarRevision`] changes, and a life total
//! changing *is* such a change — so the very event that starts the animation
//! destroys anything parented into it. The flash therefore lives in a root of
//! its own, like [`crate::depart`]'s, and follows the bar by reading where
//! the bar is.
//!
//! # Up is the screen's up, not the bar's
//!
//! A seat across the table has its bar rotated half a turn
//! ([`crate::hud::place_seat_bars`]), so a popup that rose in the bar's own
//! frame would travel *down* the screen for half the table and land on the
//! board it belongs to. Every flash rises towards the top of the window
//! instead. That is the same decision the bar itself makes for its text —
//! the ink reads in the viewer's order — carried one step further, and it is
//! why the anchor is read as a **screen point** off the laid-out node rather
//! than composed out of the bar's corner and tilt.
//!
//! The anchor is remembered on the flash, for the reason the first section
//! gives: the cell it is measured from is despawned and respawned by every
//! view, and a node spawned this frame has no layout until `PostUpdate`. A
//! flash that took the missing measurement at face value would jump to the
//! window's top-left corner for one frame out of every view that arrived.

use crate::hud::palette;
use baylee_client_core::lifeflash::Flash;
use baylee_core::ids::PlayerId;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};

/// Root of every life flash. A sibling of `HudRoot`, not a child.
#[derive(Component)]
pub struct FlashRoot;

/// One seat's number, hanging over its heart.
#[derive(Component)]
pub struct Flashing {
    /// Whose life total moved.
    pub player: PlayerId,
    /// Where it hangs from, in logical window pixels: the point just above
    /// the seat's life cell, re-read every frame it can be and remembered
    /// when it cannot.
    pub anchor: Vec2,
}

/// The box a flash is drawn in, in logical pixels.
///
/// Wide enough for `−21` at [`SIZE`] with room to spare, because the node is
/// centred on the anchor and a box that had to grow with the number would
/// move the digits sideways as they were added up.
const BOX: Vec2 = Vec2::new(72.0, 24.0);

/// How far above the life cell the number hangs before it starts rising.
const CLEARANCE: f32 = 4.0;

/// The numeral's size, in logical pixels.
///
/// Larger than the life total under it (14 at a full bar) and the same at
/// every density: the flash is not part of the bar's layout and has no cell
/// to fit inside, and a seat whose bar has shrunk to pips is exactly the seat
/// whose life change is hardest to notice.
const SIZE: f32 = 19.0;

/// Ages every flash, draws the ones that are still alive, and takes down the
/// ones that are not.
///
/// One system and not three, because a flash is one entity with one clock:
/// splitting it would mean a spawner, a mover and a reaper all re-deriving
/// which seat is flashing from the same list.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub fn flash_life_changes(
    time: Res<Time>,
    mut commands: Commands,
    mut duel: ResMut<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    roots: Query<Entity, With<FlashRoot>>,
    cells: Query<(&crate::hud::LifeCell, &ComputedNode, &UiGlobalTransform)>,
    bars: Query<(&crate::hud::SeatBar, &ComputedNode, &UiGlobalTransform)>,
    mut drawn: Query<(
        Entity,
        &mut Flashing,
        &mut Node,
        &mut UiTransform,
        &mut Text,
        &mut TextColor,
    )>,
) {
    duel.life_flash.tick(time.delta_secs());
    let reduce_motion = prefs.is_some_and(|p| p.all().reduce_motion);

    for (entity, mut flashing, mut node, mut transform, mut text, mut colour) in &mut drawn {
        let Some(flash) = duel.life_flash.of(flashing.player) else {
            commands.entity(entity).despawn();
            continue;
        };
        if let Some(at) = anchor(flashing.player, &cells, &bars) {
            flashing.anchor = at;
        }
        let label = flash.label();
        if **text != label {
            text.0 = label;
        }
        draw(
            flash,
            flashing.anchor,
            reduce_motion,
            &mut node,
            &mut transform,
            &mut colour,
        );
    }

    let Some(fonts) = fonts else {
        return;
    };
    let mut root = roots.iter().next();
    for flash in duel.life_flash.flashes() {
        if drawn.iter().any(|(_, f, ..)| f.player == flash.seat) {
            continue;
        }
        // No anchor yet is a bar that has not been laid out — the frame a
        // view arrives on, every time. The flash keeps its place in the
        // ledger and is spawned on the next frame, a sixtieth of a second
        // into a life that lasts a second.
        let Some(at) = anchor(flash.seat, &cells, &bars) else {
            continue;
        };
        let root = *root.get_or_insert_with(|| spawn_root(&mut commands));
        let mut node = Node {
            position_type: PositionType::Absolute,
            width: px(BOX.x),
            height: px(BOX.y),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };
        let mut transform = UiTransform::IDENTITY;
        let mut colour = TextColor(ink(flash));
        draw(
            flash,
            at,
            reduce_motion,
            &mut node,
            &mut transform,
            &mut colour,
        );
        let entity = commands
            .spawn((
                Flashing {
                    player: flash.seat,
                    anchor: at,
                },
                node,
                transform,
                Text::new(flash.label()),
                crate::hud::tf(&fonts, SIZE),
                colour,
                TextLayout::justify(Justify::Center),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(root).add_child(entity);
    }
}

/// Where a seat's flash hangs from, in logical window pixels.
///
/// The life cell when the bar has one, and the bar itself when it has not:
/// [`Density::Mark`](baylee_client_core::seatbar::Density::Mark) is a caret,
/// a swatch and twelve pips on a shelf 151 px long, and the seat whose bar
/// has been squeezed down to that is the last one a player should have to
/// work out a life change for by watching a number that no longer exists.
/// The whole bar is then the seat's identity, so the number hangs over the
/// middle of it.
///
/// `None` means the node has not been laid out yet, which the caller answers
/// by keeping the anchor it already had.
fn anchor(
    player: PlayerId,
    cells: &Query<(&crate::hud::LifeCell, &ComputedNode, &UiGlobalTransform)>,
    bars: &Query<(&crate::hud::SeatBar, &ComputedNode, &UiGlobalTransform)>,
) -> Option<Vec2> {
    let over = |computed: &ComputedNode, at: &UiGlobalTransform| {
        let scale = computed.inverse_scale_factor;
        let size = computed.size() * scale;
        // A node `bevy_ui` has never measured is the whole of what this
        // guards: it stands at the origin with no size, and a flash placed on
        // it would be drawn in the corner of the window.
        (size.x > 0.0 && size.y > 0.0).then(|| {
            at.translation * scale - Vec2::new(0.0, size.y / 2.0 + BOX.y / 2.0 + CLEARANCE)
        })
    };
    cells
        .iter()
        .find(|(cell, ..)| cell.player == player)
        .and_then(|(_, computed, at)| over(computed, at))
        .or_else(|| {
            bars.iter()
                .find(|(bar, ..)| bar.player == player)
                .and_then(|(_, computed, at)| over(computed, at))
        })
}

/// Red for a loss and green for a gain, which is the whole of the colour.
///
/// [`palette::DANGER`] is the red the bar itself turns at five life, so a
/// player losing life and a player about to lose the game are told in one
/// colour — they are the same fact at two distances. The green is
/// [`palette::HEAL`] and deliberately not [`palette::ACCENT`], which means
/// *this is asking you something*.
fn ink(flash: &Flash) -> Color {
    if flash.is_loss() {
        palette::DANGER
    } else {
        palette::HEAL
    }
}

/// Puts one flash where its pose says it is.
fn draw(
    flash: &Flash,
    anchor: Vec2,
    reduce_motion: bool,
    node: &mut Node,
    transform: &mut UiTransform,
    colour: &mut TextColor,
) {
    let pose = flash.pose(reduce_motion);
    node.left = px(anchor.x - BOX.x / 2.0);
    node.top = px(anchor.y - BOX.y / 2.0 - pose.rise);
    // About the node's own centre, which is what `bevy_ui` scales a node
    // about, so the swell happens around the number instead of pushing it
    // away from the point it is pinned to.
    transform.scale = Vec2::splat(pose.scale);
    colour.0 = ink(flash).with_alpha(pose.alpha);
}

/// The root every flash is drawn in.
fn spawn_root(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            FlashRoot,
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            Pickable::IGNORE,
            // Over `HudRoot` and the seat bars, which sit at zero, and over
            // a card leaving the hand at one: this is the shortest-lived and
            // most important thing on the screen while it is there.
            ZIndex(2),
        ))
        .id()
}

/// The system, *run*.
///
/// A flash declared and never wired is the bug this module would ship, so
/// every assertion is on an entity that exists, the text it carries or the
/// `Node` it moved to — never on `update()` having returned.
#[cfg(test)]
mod running {
    use super::*;
    use baylee_client_core::lifeflash::LIFE;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_view::PlayerView;

    /// The scale factor a Retina window reports, which is what
    /// `UiGlobalTransform` is in and `inverse_scale_factor` undoes.
    const SCALE: f32 = 2.0;

    /// A life cell for `player`, laid out as `bevy_ui` would have left it.
    fn a_bar_on_screen(app: &mut App, player: PlayerId, centre: Vec2) {
        app.world_mut().spawn((
            crate::hud::LifeCell { player },
            ComputedNode {
                size: Vec2::new(48.0, 20.0) * SCALE,
                inverse_scale_factor: 1.0 / SCALE,
                ..default()
            },
            UiGlobalTransform::from(bevy::math::Affine2::from_translation(centre * SCALE)),
        ));
    }

    /// A two-seat view whose life totals are `lives`.
    ///
    /// Written on the built view rather than through the builder because life
    /// is the one seat field nothing else has ever had to set: every other
    /// test in this workspace reads the twenty a new table starts on.
    fn at(lives: [i32; 2]) -> PlayerView {
        let mut view = ViewBuilder::new(2).build();
        for (seat, life) in view.seats.iter_mut().zip(lives) {
            seat.life = life;
        }
        view
    }

    /// A duel whose seats are at `lives`, with the ledger already seeded.
    fn seated(lives: [i32; 2]) -> App {
        let mut app = App::new();
        app.add_plugins(bevy::time::TimePlugin);
        app.insert_resource(crate::Duel::default());
        // The number is text and text needs a face. Handles that resolve to
        // no asset are enough here: nothing in this harness rasterises a
        // glyph, and what is being asked is which node exists and where.
        app.insert_resource(crate::hud::UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        });
        app.add_systems(Update, flash_life_changes);
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .receive_view(at(lives));
        app
    }

    /// A new view of the table reaching the client, the way `poll_host` hands
    /// one over.
    fn arrives(app: &mut App, lives: [i32; 2]) {
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .receive_view(at(lives));
    }

    fn label_of(app: &mut App, player: PlayerId) -> Option<String> {
        app.world_mut()
            .query::<(&Flashing, &Text)>()
            .iter(app.world())
            .find(|(f, _)| f.player == player)
            .map(|(_, text)| text.0.clone())
    }

    /// The whole thing, end to end: a view with less life in it puts a red
    /// number over that seat's heart.
    #[test]
    fn losing_life_puts_a_number_over_the_seat_that_lost_it() {
        let mut app = seated([20, 20]);
        a_bar_on_screen(&mut app, PlayerId::new(0), Vec2::new(400.0, 700.0));
        a_bar_on_screen(&mut app, PlayerId::new(1), Vec2::new(400.0, 200.0));
        arrives(&mut app, [17, 20]);
        app.update();

        assert_eq!(
            label_of(&mut app, PlayerId::new(0)).as_deref(),
            Some("\u{2212}3"),
            "the seat that lost three life is not saying so"
        );
        assert!(
            label_of(&mut app, PlayerId::new(1)).is_none(),
            "the seat that lost nothing flashed anyway"
        );
    }

    /// It hangs *above* the heart, wherever on the screen that heart is —
    /// including the seat across the table, whose bar is drawn upside down.
    #[test]
    fn every_seats_number_hangs_above_its_own_heart() {
        let mut app = seated([20, 20]);
        let near = Vec2::new(400.0, 700.0);
        let far = Vec2::new(400.0, 200.0);
        a_bar_on_screen(&mut app, PlayerId::new(0), near);
        a_bar_on_screen(&mut app, PlayerId::new(1), far);
        arrives(&mut app, [17, 15]);
        app.update();

        for (player, heart) in [(PlayerId::new(0), near), (PlayerId::new(1), far)] {
            let top = app
                .world_mut()
                .query::<(&Flashing, &Node)>()
                .iter(app.world())
                .find(|(f, _)| f.player == player)
                .map(|(_, node)| node.top)
                .expect("a flash for every seat that was hit");
            let Val::Px(top) = top else {
                panic!("a flash was placed in something other than pixels")
            };
            assert!(
                top + BOX.y < heart.y,
                "seat {player:?}'s number sits at {top}, which is not above {}",
                heart.y
            );
        }
    }

    /// A bar squeezed down to a caret and twelve pips has no life cell, and
    /// the seat behind it still gets its number — over the bar itself.
    #[test]
    fn a_seat_whose_bar_has_no_life_cell_flashes_over_the_bar() {
        let mut app = seated([20, 20]);
        app.world_mut().spawn((
            crate::hud::SeatBar {
                player: PlayerId::new(0),
                placed: None,
            },
            ComputedNode {
                size: Vec2::new(151.0, 12.0) * SCALE,
                inverse_scale_factor: 1.0 / SCALE,
                ..default()
            },
            UiGlobalTransform::from(bevy::math::Affine2::from_translation(
                Vec2::new(300.0, 600.0) * SCALE,
            )),
        ));
        arrives(&mut app, [16, 20]);
        app.update();
        assert_eq!(
            label_of(&mut app, PlayerId::new(0)).as_deref(),
            Some("\u{2212}4"),
            "a seat with no life cell was told nothing"
        );
    }

    /// It ends on its own, and takes its entity with it.
    #[test]
    fn a_flash_is_taken_off_the_screen_when_it_is_over() {
        let mut app = seated([20, 20]);
        a_bar_on_screen(&mut app, PlayerId::new(0), Vec2::new(400.0, 700.0));
        arrives(&mut app, [17, 20]);
        app.update();
        assert!(label_of(&mut app, PlayerId::new(0)).is_some());

        app.world_mut()
            .resource_mut::<crate::Duel>()
            .life_flash
            .tick(LIFE);
        app.update();
        assert!(
            label_of(&mut app, PlayerId::new(0)).is_none(),
            "the flash outlived its own clock"
        );
    }
}
