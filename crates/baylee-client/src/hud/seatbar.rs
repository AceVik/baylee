//! The bar written on each seat's ledge.
//!
//! Screen-space ink pinned to a rectangle of 3D table:
//! [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)
//! is the shelf, [`crate::table::Lens`] says where that shelf is drawn, and
//! everything below writes a `Node` over it. The ground belongs to the seat
//! and the ink reads in the viewer's order — the seat across the table has
//! its mat rotated half a turn and its bar the right way up, because a bar
//! nobody can read but the one player who cannot see it is not a bar.
//!
//! There is no text on the 3D table by discipline (numerals on a card are a
//! 4×6 stencil in the plate shader), so a player's *name* cannot be drawn in
//! world space at all without a text-to-texture pipeline built for one
//! feature. That is the whole of the ground/ink split: furniture in the mat,
//! writing in the overlay.
//!
//! # Its own tree, and its own revision
//!
//! [`crate::hud::HudRevision`] includes the hovered object, so the overlay's
//! whole tree is rebuilt whenever the pointer moves from one card to another.
//! The bars must not be — twelve tiles and nine cells per seat, rebuilt on
//! every pointer move, is exactly the traffic the hover preview was pulled
//! out of the retained tree to avoid. So the bars are their own root with
//! their own [`BarRevision`], and the pointer is not in it.
//!
//! # Placement runs in `Update`, not `PostUpdate`
//!
//! `bevy_ui` orders `UiSystems::Layout` *before* `TransformSystems::Propagate`,
//! so a placer reading a camera's propagated `GlobalTransform` would be
//! writing a `Node` position the layout had already read past — the ink would
//! swim one frame behind the felt for as long as the camera moved. The rig
//! itself is written in `Update` by
//! [`apply_camera_rig`](crate::table::apply_camera_rig), and
//! [`measure_shelves`] reads it there, one system later, through the same
//! [`CameraRig::eye`](crate::table::CameraRig::eye) the camera was set from.

use super::rail::row_visual;
#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::automation::{PhaseOrders, RailSide};
use baylee_client_core::seatbar::{CELL_GAP, Cell, Density, HALO_OUT, Zone};
use baylee_view::SeatView;

/// Root of every seat bar. A sibling of [`HudRoot`], not a child.
#[derive(Component)]
pub struct SeatBarRoot;

/// One seat's bar.
#[derive(Component)]
pub struct SeatBar {
    /// Whose shelf this is written on.
    pub player: PlayerId,
    /// Where this bar was last put — its top-left corner, its tilt and how
    /// wide its box was drawn — so that a camera standing still costs
    /// nothing.
    ///
    /// The width is in there because of [`Density::Split`], whose box is the
    /// shelf's own length: a camera that dollies straight in changes how long
    /// a ledge projects without moving its middle, so a bar guarded on the
    /// corner alone would keep the width it was born with while the shelf
    /// under it grew.
    ///
    /// [`place_seat_bars`] writes `Node::left`/`top`, and a `Mut<Node>` marks
    /// the node changed on any write at all, so placing every bar every frame
    /// would relayout every bar every frame whether or not the camera had
    /// moved. `table::apply_camera_rig` keeps the same discipline for the
    /// same reason. What is stored is the pair actually written rather than
    /// the shelf it came from, because the corner also moves when the
    /// designation appears and widens the hinge.
    pub placed: Option<(Vec2, f32, f32)>,
}

/// A step tile on a seat's bar — the seat bar's half of [`PhaseButton`].
///
/// A separate component rather than reusing `PhaseButton`, because the two
/// answer differently: a rail button is one of twenty-four in a fixed strip,
/// and this is one of twelve on a bar that also knows whose it is. The
/// *order* they toggle is the same one — `PhaseOrders` is keyed by
/// [`RailSide`] and not by seat, so a standing order about opponents' turns
/// is one order however many opponents there are.
#[derive(Component)]
pub struct SeatStep {
    /// Which order set this tile belongs to.
    pub side: RailSide,
    /// Which step of a turn.
    pub row: RailRow,
}

/// A step tile's share of the shelf its bar is written on.
///
/// The box follows the shelf every frame ([`place_seat_bars`]) and the twelve
/// tiles inside it did not, which made a liar of the whole form: a split bar
/// is built once, when its *density* changes, and its tiles were given the
/// width the ledge projected at that moment. A camera still easing towards
/// its home — which is every duel, for the first second of it — then left the
/// box on the whole ledge and the tiles a tenth short of it, with
/// `SpaceBetween` quietly spending the difference on the phase gaps.
/// Photographed on a 1728-wide window: 66 px tiles and 45 px phase gaps where
/// the model says 72 and 24.5, on a shelf the bar had already been told was
/// 1127 long. Nothing looked broken, which is why it took a row profile to
/// find — the bar still spanned its ledge, in the wrong proportions.
///
/// So the tiles follow the shelf too, the same way and in the same schedule.
/// The span is here because a main phase is [`Density::main_span`] tiles wide
/// and the new width has to be reached by the arithmetic that reached the old
/// one.
#[derive(Component)]
pub struct SeatTile {
    /// Whose bar this tile stands on.
    pub player: PlayerId,
    /// How many step widths this tile is drawn: one, or [`Density::main_span`]
    /// for a main phase, which is one step and one phase at once.
    pub span: f32,
}

/// A cell that opens the seat sheet: the caret, the name, life, any count.
#[derive(Component)]
pub struct SeatInk {
    /// Whose sheet.
    pub player: PlayerId,
}

/// The life cell in particular, so that a life total changing can be drawn
/// over the number it changed.
///
/// A second marker beside [`SeatInk`] rather than a search through the tree:
/// every cell on the bar wears `SeatInk`, so "which of this seat's cells is
/// the life one" has no answer from the components alone, and answering it
/// by position would be a claim about [`Density::cells`] made in the one
/// place that cannot see it.
///
/// [`crate::lifeflash`] is the only reader. Not every bar has one — a
/// [`Density::Mark`] bar has no life cell at all — which is why that module
/// falls back to the bar itself rather than treating a missing cell as a
/// missing seat.
#[derive(Component)]
pub struct LifeCell {
    /// Whose life total.
    pub player: PlayerId,
}

/// Where one seat's shelf is on screen, and what fits on it.
///
/// Measured along the shelf's **own** axis rather than the window's, which is
/// the difference between a table of two and a table of four. A side seat's
/// ledge runs up and down the screen: its bounding box is sixty pixels wide
/// and its ledge is four hundred long, and a bar sized from the box would be
/// a pip strip on a shelf with room for every label.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shelf {
    /// The middle of the ledge, in logical pixels.
    pub middle: Vec2,
    /// How long it is along its own axis — what the density is chosen from.
    pub along: f32,
    /// How deep it is across that axis. The bar's box may be taller — it
    /// draws nothing above its tiles — but the **ink** may not be: a shelf
    /// shallower than `Density::ink_height` is a bar standing on the creature
    /// lane behind it, which `camera_tests` is what stops.
    pub depth: f32,
    /// Which way the shelf runs, clockwise from the window's `+x`.
    ///
    /// Folded into a half-turn, so a bar is never drawn upside-down: the seat
    /// across the table has its ledge rotated 180° and its ink the right way
    /// up. **The ink reads in the viewer's order; the ground belongs to the
    /// seat.**
    pub tilt: f32,
    /// The densest bar this shelf can hold.
    pub density: Density,
}

impl Shelf {
    /// The projection of one seat's ledge.
    ///
    /// `corners` is
    /// [`SeatSlot::ledge_corners`](baylee_client_core::layout::SeatSlot::ledge_corners)'
    /// loop, already projected: the two corners on the mat's outside edge
    /// first — whichever of the two long edges that seat's shelf took — then
    /// the two that meet the lane behind it.
    #[must_use]
    pub fn of(corners: [Vec2; 4], designated: bool) -> Self {
        let [near_a, near_b, far_b, far_a] = corners;
        // The mean of the two long edges, because a foreshortened rectangle's
        // near edge is longer than its far one and the bar lies between them.
        let along = f32::midpoint(near_a.distance(near_b), far_a.distance(far_b));
        let near = near_a.midpoint(near_b);
        let far = far_a.midpoint(far_b);
        let middle = near.midpoint(far);
        let axis = near_b - near_a;
        // Folded into a half-turn: `atan2` on a screen whose `+y` runs down
        // gives a clockwise angle, which is what `Rot2` wants, and an axis
        // pointing left is the same shelf read from the other side.
        let mut tilt = axis.y.atan2(axis.x);
        if tilt > std::f32::consts::FRAC_PI_2 {
            tilt -= std::f32::consts::PI;
        } else if tilt < -std::f32::consts::FRAC_PI_2 {
            tilt += std::f32::consts::PI;
        }
        Self {
            middle,
            along,
            depth: near.distance(far),
            tilt,
            density: Density::for_shelf(along, near.distance(far), designated),
        }
    }

    /// How big the bar's box is drawn on this shelf.
    ///
    /// The form's own width for every single-row bar: a fixed-width box is
    /// what keeps a numeral going from 9 to 10 from moving anything else.
    /// [`Density::Split`] is the one form whose box is measured from the
    /// **shelf** instead, because its steps are meant to have the whole
    /// length of the ledge — the twelve tiles grow into it and the slack past
    /// their cap goes into the gaps between them, so the row spans the shelf
    /// at any length rather than sitting centred with felt showing at both
    /// ends.
    #[must_use]
    pub fn box_size(&self, designated: bool) -> Vec2 {
        let width = if self.density.is_split() {
            self.along.max(self.density.width(designated))
        } else {
            self.density.width(designated)
        };
        Vec2::new(width, self.density.height())
    }

    /// The top-left of the bar's box before it is turned.
    ///
    /// A `Node`'s `left`/`top` place the *un-rotated* box and
    /// [`UiTransform`] turns it about its own middle, so centring the box on
    /// the shelf and turning it is the whole of the placement.
    #[must_use]
    pub fn corner(&self, designated: bool) -> Vec2 {
        self.middle - self.box_size(designated) * 0.5
    }
}

/// Every seat's shelf, measured this frame.
///
/// A resource rather than a component per bar, because it is read before the
/// bars exist: the tree is built from it, and a bar spawned without its
/// position would be drawn in the top-left corner for one frame.
#[derive(Resource, Default)]
pub struct Shelves(pub Vec<(PlayerId, Shelf)>);

impl Shelves {
    /// One seat's shelf, if it has one this frame.
    #[must_use]
    pub fn of(&self, player: PlayerId) -> Option<Shelf> {
        self.0
            .iter()
            .find_map(|(who, shelf)| (*who == player).then_some(*shelf))
    }
}

/// What the bars were last built from.
///
/// Deliberately *not* the pointer, and deliberately not the shelves either:
/// a shelf moves every frame the camera does, and the bar follows it by
/// having its `Node` rewritten rather than by being rebuilt. Only the
/// **density** of a shelf is here, because that changes what the tree is.
#[derive(Resource, Default)]
pub struct BarRevision {
    /// The snapshot the seats, life and counts came from.
    seq: Option<u64>,
    /// The standing orders, which decide every tile's frame.
    orders: Option<PhaseOrders>,
    /// Which seat the camera is framing — the bar's name cell is where a
    /// player clicks to change it.
    focus: Option<PlayerId>,
    /// The densities, in seat order. A camera that zooms past a boundary is
    /// the one thing that rebuilds these trees without the game moving.
    densities: Vec<(PlayerId, Density)>,
    /// Whether the game has a day/night designation, which widens the hinge.
    designated: bool,
    /// The interface's language.
    lang: Option<Lang>,
}

/// Measures every seat's shelf through the rig the camera was set from.
///
/// Runs after [`apply_camera_rig`](crate::table::apply_camera_rig) and in
/// `Update`, for the scheduling reason in this module's header.
pub fn measure_shelves(
    duel: Res<Duel>,
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    mut shelves: ResMut<Shelves>,
) {
    shelves.0.clear();
    let (Some(layout), Some(rig)) = (duel.layout.as_ref(), shown.rig()) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let lens = crate::table::Lens::new(rig, size);
    let designated = duel.view.as_ref().is_some_and(|v| v.day_night.is_some());
    for slot in &layout.slots {
        let Some(corners) = lens.corners(slot.ledge_corners()) else {
            continue;
        };
        shelves
            .0
            .push((slot.player, Shelf::of(corners, designated)));
    }
}

/// Follows the shelves with the bars that are already built.
///
/// The camera moves every frame a player drags it and the bars have to move
/// with it, but a bar whose life total has not changed is the same tree in a
/// different place — so this writes where it is and which way it lies, and
/// nothing else. The tree itself is [`sync_seat_bars`]'s.
pub fn place_seat_bars(
    shelves: Res<Shelves>,
    duel: Res<Duel>,
    mut bars: Query<(&mut SeatBar, &mut Node, &mut UiTransform)>,
) {
    let designated = duel.view.as_ref().is_some_and(|v| v.day_night.is_some());
    for (mut bar, mut node, mut turn) in &mut bars {
        let Some(shelf) = shelves.of(bar.player) else {
            // A shelf the camera cannot see is a bar with nowhere to be.
            // Hidden rather than despawned: the seat has not gone anywhere,
            // and rebuilding the tree when the camera swings back would make
            // an orbit cost a rebuild per seat.
            //
            // Guarded on what the node already says rather than on `placed`,
            // which is the same thing for a bar that has been placed once and
            // is *not* the same thing for a bar spawned this frame: that one
            // has no `placed` and is nonetheless visible, at whatever corner
            // `spawn_bar` happened to give it.
            if node.display != Display::None {
                bar.placed = None;
                node.display = Display::None;
            }
            continue;
        };
        // The camera stands still most of the time and should cost nothing
        // then: touching `Node` at all is a relayout of this bar's whole
        // subtree, so the write is guarded by where the bar already is.
        let corner = shelf.corner(designated);
        let width = shelf.box_size(designated).x;
        if bar.placed == Some((corner, shelf.tilt, width)) {
            continue;
        }
        bar.placed = Some((corner, shelf.tilt, width));
        node.display = Display::Flex;
        node.left = px(corner.x);
        node.top = px(corner.y);
        node.width = px(width);
        turn.rotation = Rot2::radians(shelf.tilt);
    }
}

/// Follows the shelves with the twelve tiles inside the bars, too.
///
/// [`place_seat_bars`] moves and resizes the *box*; this is the one thing
/// inside it whose width is not a fixed number of pixels, and it has to be
/// recomputed from the same length for the same reason — see [`SeatTile`] for
/// what the bar looked like while it was not.
///
/// Its own write is guarded the same way, on the width already in the node,
/// so a camera standing still costs one comparison per tile and no relayout.
pub fn stretch_step_tiles(
    shelves: Res<Shelves>,
    duel: Res<Duel>,
    mut tiles: Query<(&SeatTile, &mut Node)>,
) {
    let designated = duel.view.as_ref().is_some_and(|v| v.day_night.is_some());
    for (tile, mut node) in &mut tiles {
        let Some(shelf) = shelves.of(tile.player) else {
            // The bar is hidden, not despawned. Leaving the width alone means
            // the tiles are right again the frame the shelf comes back.
            continue;
        };
        let length = shelf.box_size(designated).x;
        let width = px(shelf.density.tile_width_on(length) * tile.span);
        if node.width != width {
            node.width = width;
        }
    }
}

/// Builds every seat's bar, when what is written on one changes.
#[allow(clippy::too_many_arguments)]
pub fn sync_seat_bars(
    mut commands: Commands,
    duel: Res<Duel>,
    shelves: Res<Shelves>,
    mut revision: ResMut<BarRevision>,
    existing: Query<Entity, With<SeatBarRoot>>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    prefs: Res<crate::prefs::Prefs>,
) {
    let lang = Lang::of(&settings.lang);
    let orders = prefs.orders().clone();
    let Some(view) = duel.view.as_ref() else {
        return;
    };
    let designated = view.day_night.is_some();
    let densities: Vec<(PlayerId, Density)> = shelves
        .0
        .iter()
        .map(|(player, shelf)| (*player, shelf.density))
        .collect();
    let seq = duel.board.as_ref().map(|b| b.seq);
    let unchanged = revision.seq == seq
        && revision.focus == duel.focus
        && revision.densities == densities
        && revision.designated == designated
        && revision.lang == Some(lang)
        && revision
            .orders
            .as_ref()
            .is_some_and(|had| had.same_as(&orders));
    if unchanged && !existing.is_empty() {
        return;
    }
    revision.seq = seq;
    revision.focus = duel.focus;
    revision.densities = densities;
    revision.designated = designated;
    revision.lang = Some(lang);
    revision.orders = Some(orders.clone());

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let root = commands
        .spawn((
            SeatBarRoot,
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            // The bars stand over the table; the space between them must not.
            Pickable::IGNORE,
            // A bar belongs to the felt and everything the player summons
            // stands *over* it — the stack panel, the hand, the zone
            // browser, a hover preview. A bar that drew over a preview would
            // hide the card the player asked to see in order to say which
            // step it is, which they already know.
            //
            // `GlobalZIndex` and not `ZIndex`, which is the whole point of
            // the line. The seat bars are their own retained tree with their
            // own root (`BarRevision`), so a plain `ZIndex` here orders them
            // against *nothing*: `ZIndex` is local to a parent's children,
            // and the overlay's own 1/2/3/10 are inside `HudRoot`, a
            // different stacking context. Two roots both at zero are tied
            // and broken by whichever was rebuilt last — which is how the
            // local seat's phase tiles came to be drawn straight through the
            // zone browser's dialog.
            GlobalZIndex(-1),
        ))
        .id();

    for seat in &view.seats {
        let Some(shelf) = shelves.of(seat.player) else {
            continue;
        };
        let bar = spawn_bar(
            &mut commands,
            lang,
            view,
            duel.statics.as_ref(),
            seat,
            shelf,
            &orders,
            &fonts,
            designated,
        );
        commands.entity(root).add_child(bar);
    }
}

/// One seat's bar.
#[allow(clippy::too_many_arguments)]
fn spawn_bar(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    shelf: Shelf,
    orders: &PhaseOrders,
    fonts: &UiFonts,
    designated: bool,
) -> Entity {
    let density = shelf.density;
    let corner = shelf.corner(designated);
    let size = shelf.box_size(designated);
    let player = seat.player;
    let bar = commands
        .spawn((
            SeatBar {
                player,
                placed: None,
            },
            // Turned to lie along its own ledge. Bevy 0.19's UI picking runs
            // the pointer through `UiGlobalTransform::inverse`, so a rotated
            // tile is still clickable where it is drawn — which is what makes
            // the side seats of a four-player table a bar and not a fallback.
            UiTransform::from_rotation(Rot2::radians(shelf.tilt)),
            Node {
                position_type: PositionType::Absolute,
                left: px(corner.x),
                top: px(corner.y),
                width: px(size.x),
                height: px(size.y),
                // A column of rows, one row for every form but the split one.
                // The wrapper costs a node per bar and buys the two forms one
                // build: a single-row bar is a column with one row in it.
                //
                // The box's **top** is the shelf's outer edge, away from the
                // lanes, at every seat — `ledge_corners` winds the rectangle
                // from that edge inwards and `Shelf::of`'s half-turn fold
                // flips with the seat that needed it. So the first row is the
                // one furthest from the board, which is where the steps go.
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Center,
                row_gap: px(density.row_gap()),
                ..default()
            },
            // The hitbox stays transparent; backing must fit the ink bounds.
            Pickable::IGNORE,
        ))
        .id();

    if density.is_split() {
        let backing = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px((density.height() - density.ink_height()) * 0.5),
                    width: percent(100),
                    height: px(density.ink_height()),
                    border_radius: BorderRadius::all(px(TILE_RADIUS)),
                    ..default()
                },
                BackgroundColor(palette::SEAT_BACKING),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(backing);
    }

    for (index, cells) in density.rows().into_iter().enumerate() {
        let height = row_height(density, index);
        let row = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: px(height),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(CELL_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let mut strutted = false;
        for cell in cells {
            // The identity row of a split bar is **a nameplate at one end and
            // a tally at the other**, and the gap between them is deliberate.
            // It used to be the whole row bunched into the left third with the
            // turn number alone at the right, which is a row anchored by a
            // caret on one side and nothing on the other. The strut used to
            // sit before the hinge alone, which anchored the ends and left
            // every count in the left-hand cluster. Moving it in front of the
            // first count takes the four of them with it: who and how much
            // life at the left, what is in each zone and what turn it is at
            // the right, and the empty stretch where the eye passes over it.
            //
            // Not aligned to the phase groups above, which was the other
            // candidate: the tiles' widths follow the shelf while these cells
            // are fixed by rule, and a life total standing under "combat"
            // reads as being *about* combat. Two rows about two things do not
            // borrow each other's grid.
            let opens_the_tally = matches!(cell, Cell::Count(_) | Cell::Hinge);
            if density.is_split() && !strutted && opens_the_tally {
                strutted = true;
                let strut = commands
                    .spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(row).add_child(strut);
            }
            let width = density.cell_width(cell, designated);
            let node = match cell {
                Cell::Caret => caret(commands, view, seat, fonts, width, height),
                Cell::Swatch => swatch(commands, view, statics, seat, width, height),
                Cell::Name => name(commands, lang, view, statics, seat, fonts, width, height),
                Cell::Life => life(commands, seat, fonts, width, height, density),
                Cell::Count(zone) => count(commands, view, seat, zone, fonts, width, height),
                Cell::Hinge => hinge(commands, view, seat, fonts, width, height),
                Cell::Steps => steps(
                    commands, view, statics, seat, orders, fonts, density, size.x,
                ),
            };
            commands.entity(row).add_child(node);
        }
        commands.entity(bar).add_child(row);
    }
    bar
}

/// A type size that fits the row it is written on.
///
/// Leave room for descenders even in the smaller fallback forms.
fn fits(pt: f32, height: f32) -> f32 {
    pt.min(height - 2.0)
}

/// A split phase row includes the under-tick; its identity row has its own
/// height. Single-row forms use the whole transparent hitbox.
fn row_height(density: Density, index: usize) -> f32 {
    if !density.is_split() {
        density.height()
    } else if index == 0 {
        density.tile_height() + HALO_OUT * 2.0
    } else {
        density.identity_height()
    }
}

/// How faint the ink on a lost seat's bar is drawn. It says nothing any more,
/// and a bar that said nothing at full strength would go on drawing the eye.
const DEAD_INK: f32 = 0.28;

/// How faint a step the standing orders skip is written.
///
/// Still readable: the absence of the stop accent carries the standing order,
/// so the label need not fade to a watermark. Contrast needs a live composite.
const SKIP_INK: f32 = 0.68;

/// The corner a step tile is cut with.
const TILE_RADIUS: f32 = 3.0;

/// The ink a seat's bar is written in.
fn ink_of(seat: &SeatView) -> Color {
    if seat.has_lost {
        palette::PARCHMENT.with_alpha(DEAD_INK)
    } else {
        palette::PARCHMENT
    }
}

/// The priority caret: its own column, always drawn, invisible when the seat
/// is not holding priority.
///
/// Invisible rather than absent for the reason the seat tab's was: a caret
/// that took its width with it moved the whole bar sideways every time
/// priority passed, which is several times a turn.
fn caret(
    commands: &mut Commands,
    view: &PlayerView,
    seat: &SeatView,
    fonts: &UiFonts,
    width: f32,
    height: f32,
) -> Entity {
    let holding = view.priority == Some(seat.player);
    commands
        .spawn((
            SeatInk {
                player: seat.player,
            },
            cell_node(width, height),
            children![(
                Text::new("\u{25b6}"),
                tf(fonts, fits(12.0, height)),
                TextColor(if holding {
                    palette::ACTIVE
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// Three pixels of the seat's own colour — the same one its mat's rim wears,
/// so the ink and the ground under it name the same player.
fn swatch(
    commands: &mut Commands,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    width: f32,
    height: f32,
) -> Entity {
    let team = statics.and_then(|s| s.seats.iter().find(|i| i.player == seat.player)?.team);
    let colour = if seat.player == view.seat {
        palette::ACTIVE
    } else {
        team_color(team)
    };
    commands
        .spawn((
            Node {
                width: px(width),
                // As tall as a tile, so the swatch and the steps are one
                // band of ink rather than two things of different heights.
                height: px(height * 0.55),
                border_radius: BorderRadius::all(px(1.5)),
                ..default()
            },
            BackgroundColor(if seat.has_lost {
                colour.with_alpha(DEAD_INK)
            } else {
                colour
            }),
            Pickable::IGNORE,
        ))
        .id()
}

/// The seat's name, and the cell a player clicks to frame that seat.
#[allow(clippy::too_many_arguments)] // one cell, one flat build
fn name(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    fonts: &UiFonts,
    width: f32,
    height: f32,
) -> Entity {
    let player = seat.player;
    let printed = statics.map_or_else(
        || Phrase::SeatNumbered.fill(lang, &[&player.to_string()]),
        |s| s.seat_name(player).to_string(),
    );
    let display = if player == view.seat {
        baylee_client_core::i18n::own_seat_name(lang, &printed)
    } else {
        printed
    };
    let mut node = cell_node(width, height);
    node.justify_content = JustifyContent::FlexStart;
    node.overflow = Overflow::clip_x();
    let cell = commands
        .spawn((
            SeatInk { player },
            PlayerTab { player },
            node,
            children![(
                Text::new(display),
                tf(fonts, fits(14.0, height)),
                TextColor(ink_of(seat)),
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                Pickable::IGNORE,
            )],
        ))
        .id();
    // A seat somebody has walked away from says so where its name is, which
    // is the one place a player is already looking to find out who it is.
    // `away` is on the **roster**, not on the view of the seat: a chair the
    // house is holding is not relabelled an AI, so "is somebody there" and
    // "what is their life total" come from two different payloads.
    let away = statics.is_some_and(|s| {
        s.seats
            .iter()
            .any(|identity| identity.player == player && identity.away)
    });
    if away {
        let clock = commands
            .spawn((
                Text::new('\u{f017}'.to_string()),
                icon_tf(fonts, fits(10.0, height)),
                TextColor(palette::PARCHMENT_EDGE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(cell).add_child(clock);
    }
    cell
}

/// Life, with a heart, and both turning to [`palette::DANGER`] at five.
fn life(
    commands: &mut Commands,
    seat: &SeatView,
    fonts: &UiFonts,
    width: f32,
    height: f32,
    density: Density,
) -> Entity {
    let low = seat.life <= 5 && !seat.has_lost;
    let heart = if low {
        palette::DANGER
    } else {
        palette::ACCENT
    };
    let numeral = if seat.has_lost {
        ink_of(seat)
    } else if low {
        palette::DANGER
    } else {
        palette::PARCHMENT
    };
    commands
        .spawn((
            SeatInk {
                player: seat.player,
            },
            LifeCell {
                player: seat.player,
            },
            cell_node(width, height),
            children![(
                Text::new(glyph::HEART.to_string()),
                icon_tf(fonts, fits(11.0, height)),
                TextColor(heart),
                Pickable::IGNORE,
                children![(
                    TextSpan::new(format!(" {}", seat.life)),
                    tf(
                        fonts,
                        fits(if density.is_split() { 18.0 } else { 14.0 }, height),
                    ),
                    TextColor(numeral),
                    Pickable::IGNORE,
                )],
            )],
        ))
        .id()
}

/// One of the four counts: a glyph and a numeral in a fixed-width cell.
fn count(
    commands: &mut Commands,
    view: &PlayerView,
    seat: &SeatView,
    zone: Zone,
    fonts: &UiFonts,
    width: f32,
    height: f32,
) -> Entity {
    let (icon, value) = match zone {
        Zone::Hand => (glyph::HAND, seat.hand_count),
        Zone::Library => (glyph::LIBRARY, seat.library_count),
        Zone::Graveyard => (glyph::SKULL, seat.graveyard_count),
        Zone::Exile => (
            glyph::EXILE,
            u32::try_from(
                view.exile
                    .get(seat.player.get() as usize)
                    .map_or(0, Vec::len),
            )
            .unwrap_or(u32::MAX),
        ),
    };
    let soft = if seat.has_lost {
        palette::PARCHMENT_EDGE.with_alpha(DEAD_INK)
    } else {
        palette::PARCHMENT_EDGE
    };
    commands
        .spawn((
            SeatInk {
                player: seat.player,
            },
            cell_node(width, height),
            children![(
                Text::new(icon.to_string()),
                icon_tf(fonts, fits(10.0, height)),
                TextColor(soft),
                Pickable::IGNORE,
                children![(
                    TextSpan::new(format!(" {value}")),
                    tf(fonts, fits(13.0, height)),
                    TextColor(ink_of(seat)),
                    Pickable::IGNORE,
                )],
            )],
        ))
        .id()
}

/// The hinge: the turn number on the active seat's bar, a brass tick on every
/// other one.
///
/// It is what keeps the two halves of the bar from reading as two things —
/// the number belongs to the seat whose bar it is *and* to the twelve steps
/// beside it. Brass appears nowhere else on a bar: brass is the game's clock,
/// parchment is the seat's ink, gold is "here, now".
fn hinge(
    commands: &mut Commands,
    view: &PlayerView,
    seat: &SeatView,
    fonts: &UiFonts,
    width: f32,
    height: f32,
) -> Entity {
    if view.active != seat.player {
        return commands
            .spawn((
                cell_node(width, height),
                Pickable::IGNORE,
                children![(
                    Node {
                        width: px(1),
                        height: px(height * 0.55),
                        ..default()
                    },
                    BackgroundColor(palette::BRASS.with_alpha(0.5)),
                    Pickable::IGNORE,
                )],
            ))
            .id();
    }
    let cell = commands
        .spawn((
            cell_node(width, height),
            Pickable::IGNORE,
            children![(
                Text::new(format!("T{}", view.turn)),
                tf(fonts, fits(13.0, height)),
                TextColor(palette::ACTIVE),
                Pickable::IGNORE,
            )],
        ))
        .id();
    // The designation stands beside the number, where a fact about the game
    // already lives. It appears once and never leaves (CR 730.1), which is
    // why the hinge widens rather than reserving a slot.
    if let Some(now) = view.day_night {
        let (mark, tone) = designation_of(now);
        let glyph = commands
            .spawn((
                Text::new(mark.to_string()),
                icon_tf(fonts, fits(11.0, height)),
                TextColor(tone),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(cell).add_child(glyph);
    }
    cell
}

/// The glyph and the ink for a day/night designation (CR 731).
///
/// Two colours and no third, and both of them are in the **glyph**. Day is
/// the sun in [`palette::PARCHMENT`], the warmest light in the palette and
/// the one colour that never means a status; night is the moon in
/// [`palette::INK`], the cool one. Neither is [`palette::ACTIVE`] — that
/// lights the step the game is in a few cells to the right, and a designation
/// wearing it would read as a step.
///
/// Its own function because it is what
/// `tests::designation::the_designation_does_not_borrow_a_step_glyph` reads:
/// the sun was the untap step's mark until the day designation wanted it, and
/// the two sets have to stay disjoint on a bar that draws both.
fn designation_of(now: baylee_view::DayNight) -> (char, Color) {
    match now {
        baylee_view::DayNight::Day => ('\u{f185}', palette::PARCHMENT),
        baylee_view::DayNight::Night => ('\u{f186}', palette::INK),
    }
}

/// The twelve steps of a turn, in the five phases they belong to.
///
/// The grouping is the row's whole structure. Twelve tiles at one spacing is
/// a list of twelve equal things and a turn is not that — three steps in the
/// beginning phase, five in combat, two at the end, and two main phases with
/// no steps at all (CR 500.1). So there are two gaps: tiles of a phase are
/// butted together at [`Density::tile_gap`] and the phases stand apart at
/// [`Density::phase_gap`], and it is the *phase* gaps that a split bar's
/// slack goes into. The eye then reads five groups where it used to count
/// twelve pills.
#[allow(clippy::too_many_arguments)]
fn steps(
    commands: &mut Commands,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    orders: &PhaseOrders,
    fonts: &UiFonts,
    density: Density,
    length: f32,
) -> Entity {
    let side = if same_team(statics, seat.player, view.seat) {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };
    // On a split bar the steps have the shelf to themselves, and what they do
    // with it is decided in the model rather than by flex: every tile is
    // `Density::tile_width_on` wide, the seven tight gaps never move, and the
    // slack the cap leaves goes into the four phase gaps — which is where a
    // wider gap says something true. `SpaceBetween` is what puts it there.
    // Every other form is a fixed strip.
    let tile_w = density.tile_width_on(length);
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(density.phase_gap()),
                flex_grow: f32::from(u8::from(density.is_split())),
                justify_content: if density.is_split() {
                    JustifyContent::SpaceBetween
                } else {
                    JustifyContent::FlexStart
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let current = RailRow::current(view.phase, view.step);
    // Whose turn it is decides *how* the step the game is in is marked on
    // this bar, not whether it is marked at all: gold on the active seat's,
    // a brass under-tick on every other. That is the whole answer to "where
    // is the game" from anywhere on the table — every bar says which step,
    // and gold says whose.
    let is_active_seat = view.active == seat.player;
    let skipped = |step: RailRow| orders.rows_for(side).any(|(r, s)| r == step && s);
    for phase in baylee_client_core::automation::RAIL_PHASES {
        // A phase carries its own tiles and nothing else — no ground, no
        // label, no rule. On baize a plinth under three tiles would be the
        // tab strip again, moved onto the felt; the gap is enough, and a gap
        // is the one grouping device that adds no ink.
        let group = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(density.tile_gap()),
                    // Sized by its tiles and nothing else. It must *not*
                    // grow: flex would hand each group an equal share of the
                    // slack, so a phase of one tile and a phase of five would
                    // end up with tiles of different widths, and the short
                    // groups would keep a pocket of dead space once their
                    // tiles hit the cap. Twelve tiles that no longer agree on
                    // their width is the one thing this form promised not to
                    // do, so the width is a division in the model instead.
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // A phase with one tile *is* that phase drawn where a step is drawn,
        // and both of them are main phases. `MAIN_SPAN` of a step's width is
        // what says so — see the constant for why it is a claim about the
        // turn and not only about the slack it happens to take up.
        let span = if phase.rows().len() == 1 {
            density.main_span()
        } else {
            1.0
        };
        for step in phase.rows().iter().copied() {
            let tile = spawn_tile(
                commands,
                fonts,
                density,
                tile_w * span,
                TileState {
                    side,
                    row: step,
                    skipped: skipped(step),
                    live: step.grants_priority(),
                    now: step == current,
                    gold: is_active_seat,
                    selected: orders.selected() == Some((side, step)),
                    lost: seat.has_lost,
                },
            );
            // What lets the tile follow its shelf without the tree being
            // rebuilt. Every tile carries it, the two dead ones included:
            // they hold the silhouette of the row and a silhouette that did
            // not grow with its neighbours would be the gap that says a phase
            // boundary, drawn where there is none.
            commands.entity(tile).insert(SeatTile {
                player: seat.player,
                span,
            });
            commands.entity(group).add_child(tile);
        }
        commands.entity(row).add_child(group);
    }
    row
}

/// Everything one tile has to say about itself.
#[derive(Clone, Copy)]
#[allow(clippy::struct_excessive_bools)] // four independent facts about one tile
struct TileState {
    side: RailSide,
    row: RailRow,
    /// Whether the standing order says to skip this step.
    skipped: bool,
    /// Whether a stop can be arranged here at all (CR 502.4, CR 514.3a).
    live: bool,
    /// Whether the game is in this step.
    now: bool,
    /// Whether "now" on this bar is gold — that is, whether this is the
    /// active seat's own bar.
    gold: bool,
    /// Whether the keyboard's selection is on it.
    selected: bool,
    /// Whether the seat has lost.
    lost: bool,
}

/// One step tile, with separate channels for each claim:
///
/// - A small top accent marks a standing stop; skipped labels are quieter.
/// - Gold fill is the active seat's current live step. Other current steps
///   keep their under-tick, including untap and cleanup.
/// - A frame is keyboard selection, never a standing order.
/// - Lost seats and non-priority steps keep their dim ink and no stop accent.
///
/// Position carries past/future; another fade would collide with skipped ink.
#[allow(clippy::too_many_lines)] // one tile, three channels, one flat build
fn spawn_tile(
    commands: &mut Commands,
    fonts: &UiFonts,
    density: Density,
    width: f32,
    state: TileState,
) -> Entity {
    let (icon, short) = row_visual(state.row);
    let dead = !state.live || state.lost;
    let here = state.now && state.gold && state.live;
    let ink = if here {
        palette::PARCHMENT_INK
    } else if dead {
        palette::PARCHMENT.with_alpha(DEAD_INK)
    } else if state.skipped {
        palette::PARCHMENT.with_alpha(SKIP_INK)
    } else {
        palette::PARCHMENT
    };
    let fill = if here { palette::ACTIVE } else { Color::NONE };
    // A frame is the shape of a control, so it is drawn exactly when one is
    // being operated, independently of the stop accent and current-step fill.
    let frame = if state.selected && state.live {
        palette::ACCENT
    } else {
        Color::NONE
    };
    let border = if state.selected && state.live {
        1.0
    } else {
        0.0
    };

    let tile = commands
        .spawn((
            Node {
                // One width for all twelve, handed down from
                // `Density::tile_width_on` — so nothing on the row can twitch
                // relative to anything else on it, which is the same promise
                // the fixed-width numeral cells make, kept the same way.
                width: px(width),
                height: px(density.tile_height()),
                flex_grow: 0.0,
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(2),
                border: UiRect::all(px(border)),
                // A ruler tab, not a rounded browser chip.
                border_radius: BorderRadius::all(px(TILE_RADIUS)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(frame),
        ))
        .id();

    if density.tiles_have_glyphs() {
        let glyph = commands
            .spawn((
                Text::new(icon.to_string()),
                icon_tf(fonts, 10.0),
                TextColor(ink),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(tile).add_child(glyph);
        if density.tiles_are_labelled() {
            let label = commands
                .spawn((
                    Text::new(short),
                    tf_bold(fonts, if density.is_split() { 10.5 } else { 10.0 }),
                    TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                    TextColor(ink),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(tile).add_child(label);
        }
    }

    if state.live && !state.skipped && !state.lost {
        let marker = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(if density.tiles_have_glyphs() {
                        4.0
                    } else {
                        1.0
                    }),
                    top: px(0),
                    width: px(if density.tiles_have_glyphs() {
                        8.0
                    } else {
                        3.0
                    }),
                    height: px(2),
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(if here {
                    palette::PARCHMENT_INK
                } else {
                    palette::SEAT_STOP
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(tile).add_child(marker);
    }

    if state.live {
        // Transparent tiles need an explicit hover target: `Feel` preserves
        // alpha. The wash is pointer feedback, not a preview of a stop.
        let feel = if fill == Color::NONE {
            Feel::rising_to(fill, palette::SEAT_STEP_HOVER)
        } else {
            Feel::new(fill)
        };
        commands.entity(tile).insert((
            SeatStep {
                side: state.side,
                row: state.row,
            },
            feel,
        ));
    } else {
        commands.entity(tile).insert(Pickable::IGNORE);
    }

    // Every turn passes through untap and cleanup, so "the game is in a step
    // no stop can be arranged in" happens twice a turn and is not an edge
    // case. It is marked with an under-tick rather than a frame, because the
    // tile is still dead: the game being somewhere does not make it a
    // control.
    if state.now && (!state.live || !state.gold) {
        let tick = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    bottom: px(-HALO_OUT),
                    height: px(2),
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(if state.gold {
                    palette::ACTIVE
                } else {
                    palette::BRASS.with_alpha(0.5)
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(tile).add_child(tick);
    }
    tile
}

/// A fixed-width cell with its content centred.
///
/// Fixed, because a numeral going from 9 to 10 must move nothing else on the
/// shelf — otherwise every count on every bar twitches sideways whenever
/// anybody draws a card.
fn cell_node(width: f32, height: f32) -> Node {
    Node {
        width: px(width),
        height: px(height),
        flex_shrink: 0.0,
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: px(3),
        ..default()
    }
}
