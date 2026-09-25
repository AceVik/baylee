//! Battlefield-attached seat information, with ledge marks at overview scale.
//!
//! `attached` separates identity, counts and phases around the outside rim.
//! The shelf model below remains the density probe and tiny overview fallback;
//! it no longer describes the desktop ink bounds.
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
use baylee_client_core::board::SeatRole;
use baylee_client_core::seatbar::{
    CELL_GAP, Cell, Density, HALO_OUT, SPLIT_COLUMN_GAP, SPLIT_LIFE_H, SPLIT_NAME_H,
    SPLIT_PLAQUE_PAD, SPLIT_PLAQUE_W, Zone,
};
use baylee_view::SeatView;

pub(crate) mod attached;

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
    /// **shelf** instead: the plaque is fixed, the twelve tiles grow into the
    /// remaining ledge, and the slack past
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

/// Who is answering for `player`, as the board model read it off the roster.
///
/// Through the pod and not through `GameStatic` directly, although the bar has
/// both in hand. The pod is where [`SeatRole::of`] resolved the roster's two
/// bools into the three states that exist, and three surfaces ask this
/// question — this bar, the mat's rim and the seat sheet after it. A second
/// reading of the same payload is a second chance to answer it differently,
/// which is the whole argument for the field being on the pod at all.
///
/// A board this client has not built yet seats a present player, which is the
/// same answer an empty roster gives and for the same reason: nobody has been
/// introduced.
pub(in crate::hud) fn role_of(duel: &Duel, player: PlayerId) -> SeatRole {
    duel.board
        .as_ref()
        .and_then(|board| board.pod(player))
        .map_or(SeatRole::Present, |pod| pod.role)
}

/// What the bars were last built from.
///
/// Deliberately *not* the pointer, and deliberately not the shelves either:
/// a shelf moves every frame the camera does, and the bar follows it by
/// having its `Node` rewritten rather than by being rebuilt. Only the
/// **density** of a shelf is here, because that changes what the tree is.
///
/// Compared and assigned **whole**, which is the point of it being a struct at
/// all. Six `&&`s against six field assignments is a list that has to be
/// added to in two places, and the field a change forgets is not a
/// compilation error — it is a bar that stops redrawing for one of the seven
/// things that should redraw it, silently, for as long as nobody looks. A
/// struct literal is a list of its fields and the compiler reads it; that is
/// what `drawer::reading` is built this way for too.
#[derive(Resource, Default, PartialEq)]
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
    /// Who is answering for each chair, in seat order.
    ///
    /// The one thing in this key that does not come from the view. See where
    /// it is built for why a bar that left it out drew a stand-in's clock
    /// late and then kept it after the player was back.
    roles: Vec<(PlayerId, SeatRole)>,
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
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    mut bars: Query<(
        &mut SeatBar,
        &mut Node,
        &mut UiTransform,
        Option<&attached::Panel>,
    )>,
) {
    let designated = duel.view.as_ref().is_some_and(|v| v.day_night.is_some());
    let lens = shown.rig().zip(windows.single().ok()).map(|(rig, window)| {
        crate::table::Lens::new(rig, Vec2::new(window.width(), window.height()))
    });
    for (mut bar, mut node, mut turn, panel) in &mut bars {
        if let Some(panel) = panel {
            attached::place(
                &duel,
                lens.as_ref(),
                bar.player,
                *panel,
                &mut node,
                &mut turn,
            );
            continue;
        }
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
    mut cloth: Option<ResMut<crate::frontal::Cloth>>,
    mut materials: Option<ResMut<Assets<crate::frontal::FrontalMaterial>>>,
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
    let fresh = BarRevision {
        seq: duel.board.as_ref().map(|b| b.seq),
        orders: Some(orders.clone()),
        focus: duel.focus,
        densities,
        // Who is answering for each chair, and it is in the key rather than
        // left to `seq` for a reason that is invisible until it bites. `seq`
        // is the **view's**, and a chair changing hands is not a view: the
        // host marks every seat's roster stale and re-sends `GameStatic`,
        // which reaches this client as its own message. A bar keyed on the
        // view alone is built from whichever roster happened to be in hand and
        // goes on saying so — which for a stand-in means the clock arrives
        // late and, worse, stays after the player has come back. "Nothing
        // remembers it afterwards" is the whole design of `away`, and this
        // field is what makes it true in the direction that matters.
        roles: view
            .seats
            .iter()
            .map(|seat| (seat.player, role_of(&duel, seat.player)))
            .collect(),
        designated,
        lang: Some(lang),
    };
    if *revision == fresh && !existing.is_empty() {
        return;
    }
    *revision = fresh;

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

    attached::spawn_turn(&mut commands, root, view, &fonts);
    for (index, seat) in view.seats.iter().enumerate() {
        let Some(shelf) = shelves.of(seat.player) else {
            continue;
        };
        // At overview scale retain the tiny legacy marks; focusing a seat
        // restores the complete, battlefield-attached furniture.
        if shelf.density != Density::Mark {
            attached::spawn(
                &mut commands,
                root,
                lang,
                view,
                duel.statics.as_ref(),
                seat,
                role_of(&duel, seat.player),
                &orders,
                &fonts,
            );
            continue;
        }
        let surface = cloth
            .as_mut()
            .and_then(|cloth| cloth.seat(index, materials.as_deref_mut()));
        let bar = spawn_bar(
            &mut commands,
            lang,
            view,
            duel.statics.as_ref(),
            seat,
            role_of(&duel, seat.player),
            shelf,
            &orders,
            &fonts,
            designated,
            surface,
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
    role: SeatRole,
    shelf: Shelf,
    orders: &PhaseOrders,
    fonts: &UiFonts,
    designated: bool,
    surface: Option<Handle<crate::frontal::FrontalMaterial>>,
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
                // Single-row fallbacks use flex; Split places both columns
                // explicitly inside the same measured ink envelope.
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

    if density.tiles_have_glyphs() {
        backing(commands, bar, density, surface);
    }

    let split = density.is_split().then(|| split_frame(commands, bar, seat));
    for (index, cells) in density.rows().into_iter().enumerate() {
        let height = row_height(density, index);
        let row = if let Some(rows) = split {
            rows[index]
        } else {
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
            commands.entity(bar).add_child(row);
            row
        };
        for cell in cells {
            let (parent, height) = split.map_or((row, height), |rows| match cell {
                Cell::Steps => (rows[2], density.tile_height() + HALO_OUT * 2.0),
                Cell::Count(_) | Cell::Hinge => (rows[3], density.identity_height()),
                _ => (row, height),
            });
            if density.is_split() && cell == Cell::Hinge {
                let strut = commands
                    .spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(parent).add_child(strut);
            }
            let width = density.cell_width(cell, designated);
            let node = match cell {
                Cell::Caret => caret(commands, view, seat, fonts, width, height),
                Cell::Swatch => swatch(commands, view, statics, seat, width, height),
                Cell::Name => name(
                    commands, lang, view, statics, seat, role, fonts, width, height,
                ),
                Cell::Life => life(commands, seat, fonts, width, height, density),
                Cell::Count(zone) => count(commands, view, seat, zone, fonts, width, height),
                Cell::Hinge => hinge(commands, view, seat, fonts, width, height),
                Cell::Steps => steps(
                    commands, view, statics, seat, orders, fonts, density, size.x,
                ),
            };
            commands.entity(parent).add_child(node);
        }
    }
    bar
}

/// The panel a bar's ink sits on, inset to the measured ink envelope.
///
/// Its own function because it is the one part of a bar that is not a cell:
/// it is drawn behind every row, it is the piece the procedural seat surface
/// is hung on when there is one, and the hitbox above it stays transparent —
/// `Pickable::IGNORE` here and on the bar, so a click reaches the tile it
/// looks like it hit rather than the plate behind it.
fn backing(
    commands: &mut Commands,
    bar: Entity,
    density: Density,
    surface: Option<Handle<crate::frontal::FrontalMaterial>>,
) {
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px((density.height() - density.ink_height()) * 0.5),
                width: percent(100),
                height: px(density.ink_height()),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(palette::SEAT_BACKING),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(handle) = surface {
        commands.entity(panel).insert((
            BackgroundColor(Color::NONE),
            MaterialNode(handle),
            crate::frontal::Hanging,
        ));
    }
    commands.entity(bar).add_child(panel);
}

/// Two independent vertical divisions inside one measured ink envelope.
fn split_frame(commands: &mut Commands, bar: Entity, seat: &SeatView) -> [Entity; 4] {
    let density = Density::Split;
    let top = (density.height() - density.ink_height()) * 0.5;
    let plaque = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(top),
                width: px(SPLIT_PLAQUE_W),
                height: px(density.ink_height()),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
            BackgroundColor(palette::DOCK_EDGE.with_alpha(if seat.has_lost() {
                0.06
            } else {
                0.18
            })),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(if seat.has_lost() {
                DEAD_INK
            } else {
                0.85
            })),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(bar).add_child(plaque);

    let name_left = SPLIT_PLAQUE_W - SPLIT_PLAQUE_PAD - density.name_width();
    [
        (
            SPLIT_PLAQUE_PAD,
            top + HALO_OUT,
            SPLIT_NAME_H,
            Some(SPLIT_PLAQUE_W - SPLIT_PLAQUE_PAD * 2.0),
        ),
        (
            name_left,
            top + HALO_OUT + SPLIT_NAME_H,
            SPLIT_LIFE_H,
            Some(density.name_width()),
        ),
        (
            SPLIT_PLAQUE_W + SPLIT_COLUMN_GAP,
            top,
            density.tile_height() + HALO_OUT * 2.0,
            None,
        ),
        (
            SPLIT_PLAQUE_W + SPLIT_COLUMN_GAP,
            top + density.tile_height() + HALO_OUT * 2.0 + density.row_gap(),
            density.identity_height(),
            None,
        ),
    ]
    .map(|(left, top, height, width)| {
        let row = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(top),
                    right: if width.is_none() { px(0) } else { Val::Auto },
                    width: width.map_or(Val::Auto, px),
                    height: px(height),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(CELL_GAP),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(row);
        row
    })
}

/// A type size that fits the row it is written on.
///
/// Leave room for descenders even in the smaller fallback forms.
fn fits(pt: f32, height: f32) -> f32 {
    pt.min(height - 2.0)
}

/// Split's left column uses a compact name row and a prominent life row.
/// The right column is sized separately, within the same ink envelope.
fn row_height(density: Density, index: usize) -> f32 {
    if !density.is_split() {
        density.height()
    } else if index == 0 {
        SPLIT_NAME_H
    } else {
        SPLIT_LIFE_H
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
    if seat.has_lost() {
        palette::DOCK_INK.with_alpha(DEAD_INK)
    } else {
        palette::DOCK_INK
    }
}

/// The priority caret: its own column, always drawn, invisible when the seat
/// is not holding priority.
///
/// "Holding" is [`baylee_client_core::board::is_awaited`], the mat rim's own
/// predicate: before turn 1 every seat still deciding its hand wears one.
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
    let holding = baylee_client_core::board::is_awaited(view, seat.player);
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
    let colour = seat_colour(view.seat, statics, seat.player);
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
            BackgroundColor(if seat.has_lost() {
                colour.with_alpha(DEAD_INK)
            } else {
                colour
            }),
            Pickable::IGNORE,
        ))
        .id()
}

/// What a seat is called, on its bar and on its button in the players' strip
/// ([`crate::hud::ledge`]'s `players`, #264): one function, so the two never
/// name one seat two ways.
pub(in crate::hud) fn called(
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    player: PlayerId,
    role: SeatRole,
) -> String {
    let printed = statics.map_or_else(
        || Phrase::SeatNumbered.fill(lang, &[&player.to_string()]),
        |s| s.seat_name(player).to_string(),
    );
    // A chair the house plays is called the house in the player's own
    // language. Off the **flag** and never by matching the string, for the
    // reason `Phrase::SeatHouse` gives: neither host has another name for such
    // a chair, so there is nothing here to hide — only an English word a
    // German player was being shown. A chair that is merely *held* keeps its
    // player's name, which is the whole reason `away` is not `is_ai`.
    let printed = if role == SeatRole::House {
        Phrase::SeatHouse.text(lang).to_string()
    } else {
        printed
    };
    if player == view.seat {
        baylee_client_core::i18n::own_seat_name(lang, &printed)
    } else {
        printed
    }
}

/// The seat's name, and the cell a player clicks to frame that seat.
///
/// It is also the whole of where a seat's **role** is said, and the reason
/// for that is arithmetic rather than taste. The design
/// (`docs/game-log-design.md` §"The seat that stepped away") asked for a role
/// line under the name, written for a strip of seat tabs that no longer
/// exists; the plaque that replaced it is `HEADER_H` = 60 logical pixels tall
/// and its two rows already spend 56 of them, and the plaque's size is what
/// `Panel::Identity` is fitted to the mat's band with. A third row means a
/// taller plaque means a band fit that two tests hold. So the role is a
/// **mark**, and the name cell is where it goes.
///
/// Nothing costs the bar a pixel when a seat is an ordinary player, which is
/// the rule the caret beside this cell already obeys: this cell is
/// `cell_node`, a fixed `width` that does not shrink, so everything inside it
/// spends the name's room and never the bar's.
#[allow(clippy::too_many_arguments)] // one cell, one flat build
fn name(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    seat: &SeatView,
    role: SeatRole,
    fonts: &UiFonts,
    width: f32,
    height: f32,
) -> Entity {
    let player = seat.player;
    let display = called(lang, view, statics, player, role);
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
                tf_bold(fonts, fits(15.0, height)),
                bevy::text::LineHeight::Px(height),
                TextColor(ink_of(seat)),
                TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
                // The name is what gives way, and it has to be *told* to.
                // A flex item's `min-width` is `auto`, which for a `NoWrap`
                // text resolves to the whole string — so a long name refuses
                // to shrink, and anything after it in the row is pushed past
                // the `clip_x` edge and drawn nowhere. That is the shape the
                // mark below was in from the day it was written: present in
                // the tree, invisible for exactly the names long enough to
                // matter.
                Node {
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            )],
        ))
        .id();
    // A chair that is not being answered for by the person whose chair it is
    // says so where its name is, which is the one place a player is already
    // looking to find out who it is.
    //
    // Only [`SeatRole::Away`] gets a mark. A chair the house plays by
    // arrangement has already said so in the name itself, and a bar that said
    // it twice would be spending the cell's room on its own repetition.
    if role == SeatRole::Away {
        let clock = commands
            .spawn((
                // A clock, because the caveat the design's sentence carried —
                // *stepped away*, not *gone* — is the one a clock makes
                // without any words: it says "for a while". The chair is not
                // empty and the next question will be answered.
                Text::new('\u{f017}'.to_string()),
                icon_tf(fonts, fits(10.0, height)),
                // The bar's own glyph ink, the life heart's, and deliberately
                // not the seat's accent: the accent is identity, and this is
                // not a fact about who the player is. `docs/game-log-design.md`
                // says `MUTED`, which is the panel register and not this one.
                TextColor(palette::PARCHMENT_EDGE),
                // It does not give way. The name does — see above — and with
                // both halves of that in place the mark is at the cell's
                // right whatever the name is, instead of off the end of it.
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
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
    let low = seat.life <= 5 && !seat.has_lost();
    let heart = if low {
        palette::DANGER
    } else {
        palette::PARCHMENT_EDGE
    };
    let numeral = if seat.has_lost() {
        ink_of(seat)
    } else if low {
        palette::DANGER
    } else {
        palette::DOCK_INK
    };
    let mut node = cell_node(width, height);
    node.border_radius = BorderRadius::all(px(3));
    if density.is_split() {
        node.justify_content = JustifyContent::FlexStart;
    }
    commands
        .spawn((
            SeatInk {
                player: seat.player,
            },
            LifeCell {
                player: seat.player,
            },
            node,
            BackgroundColor(Color::NONE),
            children![(
                Text::new(baylee_client_core::tableicons::LIFE.to_string()),
                table_icon_tf(
                    fonts,
                    baylee_client_core::tableicons::LIFE,
                    fits(18.0, height)
                ),
                bevy::text::LineHeight::Px(height),
                TextColor(heart),
                Pickable::IGNORE,
                children![(
                    TextSpan::new(format!(" {}", seat.life)),
                    tf_bold(
                        fonts,
                        fits(
                            if density.is_split() {
                                if height > 40.0 { 28.0 } else { 24.0 }
                            } else {
                                16.0
                            },
                            height
                        ),
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
    let soft = if seat.has_lost() {
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
                icon_tf(fonts, fits(if height > 40.0 { 14.0 } else { 10.0 }, height)),
                TextColor(soft),
                Pickable::IGNORE,
                children![(
                    TextSpan::new(format!(" {value}")),
                    tf(fonts, fits(if height > 40.0 { 19.0 } else { 13.0 }, height)),
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
                Text::new(view.turn.to_string()),
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
    // The model reserves the plaque first: every tile is
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
        // A recessed channel joins the steps of one phase. No border or
        // padding: the existing tile arithmetic and hitboxes stay exact.
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
                    border_radius: BorderRadius::all(px(TILE_RADIUS)),
                    ..default()
                },
                BackgroundColor(if density.tiles_have_glyphs() {
                    palette::SEAT_TRACK
                } else {
                    Color::NONE
                }),
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
                    lost: seat.has_lost(),
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

    if state.now && state.gold {
        commands.entity(tile).insert((
            PhaseNow::default(),
            BoxShadow(vec![ShadowStyle {
                color: Color::NONE,
                ..default()
            }]),
        ));
    }

    if density.tiles_have_glyphs() {
        let glyph = commands
            .spawn((
                Text::new(icon.to_string()),
                table_icon_tf(fonts, icon, 13.0),
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

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::ViewBuilder;
    use baylee_view::SeatIdentity;

    /// What one name cell is asked about, handed to the system that draws it.
    ///
    /// A resource and a one-system app rather than a direct call, because
    /// `name` builds through `Commands` and what is under test is the tree it
    /// leaves behind — the same reason `combatlines::running` and the ledge's
    /// harness are apps rather than arithmetic.
    #[derive(Resource, Clone)]
    struct Asked {
        lang: Lang,
        role: SeatRole,
        statics: GameStatic,
        view: PlayerView,
    }

    fn draw(mut commands: Commands, asked: Res<Asked>, fonts: Res<UiFonts>) {
        let seat = &asked.view.seats[OTHER];
        name(
            &mut commands,
            asked.lang,
            &asked.view,
            Some(&asked.statics),
            seat,
            asked.role,
            &fonts,
            240.0,
            22.0,
        );
    }

    /// The chair every test below is about: the opponent's, not the viewer's.
    ///
    /// Not seat 0, because `own_seat_name` wraps the viewing seat's own name
    /// — "You (Alice)" — and a test reading that would be reading the wrapper
    /// rather than the name. Nothing about a role differs between the two
    /// chairs; the wrapper is simply noise here.
    const OTHER: usize = 1;

    /// One name cell, drawn for a chair in `role` and called `called`.
    fn cell(role: SeatRole, lang: Lang, called: &str) -> App {
        let view = ViewBuilder::new(2).build();
        let mut statics = baylee_client_core::test_support::statics(1);
        statics.seats = vec![SeatIdentity {
            player: view.seats[OTHER].player,
            display_name: called.to_string(),
            is_ai: role == SeatRole::House,
            away: role == SeatRole::Away,
            team: None,
        }];
        let mut app = App::new();
        app.insert_resource(Asked {
            lang,
            role,
            statics,
            view,
        })
        .insert_resource(fonts())
        .add_systems(Update, draw);
        app.update();
        app
    }

    /// A two-seat table with its shelves already measured, running the real
    /// [`sync_seat_bars`].
    ///
    /// The shelves are handed in rather than projected, because what is under
    /// test is the gate and not the camera: `measure_shelves` runs off a rig
    /// and a window, and a harness that needed both would be testing those.
    /// Long enough for `Density::Full`, so both seats get a bar with a name on
    /// it. `Cloth` and its materials are left out on purpose — that is the
    /// branch a machine with no GPU takes.
    fn table() -> App {
        let mut duel = Duel {
            view: Some(ViewBuilder::new(2).build()),
            statics: Some(baylee_client_core::test_support::statics(1)),
            ..Duel::default()
        };
        if let Some(statics) = duel.statics.as_mut() {
            statics.seats = (0..2)
                .map(|at| SeatIdentity {
                    player: PlayerId::new(at),
                    display_name: format!("Seat {at}"),
                    is_ai: false,
                    away: false,
                    team: None,
                })
                .collect();
        }
        crate::rebuild_board(&mut duel);
        let shelves = Shelves(
            (0..2)
                .map(|at| {
                    (
                        PlayerId::new(at),
                        Shelf {
                            middle: Vec2::new(600.0, 200.0 + f32::from(at) * 400.0),
                            along: 1100.0,
                            depth: 46.0,
                            tilt: 0.0,
                            density: Density::Full,
                        },
                    )
                })
                .collect(),
        );
        let mut app = App::new();
        app.insert_resource(duel)
            .insert_resource(shelves)
            .insert_resource(fonts())
            .insert_resource(crate::settings::ClientSettings::default())
            .init_resource::<crate::prefs::Prefs>()
            .init_resource::<BarRevision>()
            .add_systems(Update, sync_seat_bars);
        app.update();
        app
    }

    /// Fonts with no asset server behind them: what is under test is the tree
    /// the bar builds, and none of that is the GPU's.
    fn fonts() -> UiFonts {
        UiFonts {
            text: Handle::default(),
            medium: Handle::default(),
            bold: Handle::default(),
            italic: Handle::default(),
            medium_italic: Handle::default(),
            serif: Handle::default(),
            serif_italic: Handle::default(),
            icons: Handle::default(),
            mana: Handle::default(),
        }
    }

    /// Every word the cell put on the screen.
    fn said(app: &mut App) -> Vec<String> {
        let mut q = app.world_mut().query::<&Text>();
        q.iter(app.world()).map(|t| t.0.clone()).collect()
    }

    /// The clock, which is the whole of what a held chair says.
    const CLOCK: &str = "\u{f017}";

    /// A chair the house is *holding* wears a clock; the other two do not.
    ///
    /// Three cases and not one, because each of the other two is a different
    /// way to get this wrong. A present player wearing a clock is the table
    /// telling everyone somebody has left when nobody has. And a chair the
    /// house plays by arrangement wearing one would be saying the same thing
    /// twice — its name already says the house is there — while spending the
    /// name's own room on the repetition.
    #[test]
    fn only_a_held_chair_wears_the_clock() {
        let mut held = cell(SeatRole::Away, Lang::En, "Alice");
        assert!(
            said(&mut held).iter().any(|word| word == CLOCK),
            "a chair the house is holding says so beside its name: {:?}",
            said(&mut held)
        );
        for role in [SeatRole::Present, SeatRole::House] {
            let mut other = cell(role, Lang::En, "Alice");
            assert!(
                !said(&mut other).iter().any(|word| word == CLOCK),
                "{role:?} is not an interruption and must not wear its mark: \
                 {:?}",
                said(&mut other)
            );
        }
    }

    /// Before turn 1 every seat still deciding its opening hand wears the
    /// caret (#257), not only the one this view's `awaiting` names — which
    /// is this seat until it keeps and nobody after. The caret asks the mat
    /// rim's own predicate, so this is also the proof that it does.
    #[test]
    fn every_seat_still_deciding_its_hand_wears_the_caret() {
        fn draw_carets(mut commands: Commands, asked: Res<Asked>, fonts: Res<UiFonts>) {
            for seat in &asked.view.seats {
                caret(&mut commands, &asked.view, seat, &fonts, 12.0, 22.0);
            }
        }
        // Seat 0's view, as `awaiting` and `deciding` would have it.
        let lit = |awaiting: Option<u8>, deciding: &[u8]| -> Vec<u8> {
            let mut view = ViewBuilder::new(2).build();
            view.awaiting = awaiting.map(PlayerId::new);
            view.deciding = deciding.iter().copied().map(PlayerId::new).collect();
            let mut app = App::new();
            app.insert_resource(Asked {
                lang: Lang::En,
                role: SeatRole::Present,
                statics: baylee_client_core::test_support::statics(1),
                view,
            })
            .insert_resource(fonts())
            .add_systems(Update, draw_carets);
            app.update();
            let world = app.world_mut();
            let carets: Vec<(PlayerId, Vec<Entity>)> = world
                .query::<(&SeatInk, &Children)>()
                .iter(world)
                .map(|(ink, kids)| (ink.player, kids.to_vec()))
                .collect();
            assert_eq!(carets.len(), 2, "one caret per seat");
            let mut lit: Vec<u8> = carets
                .into_iter()
                .filter(|(_, kids)| {
                    kids.iter().any(|kid| {
                        world
                            .get::<TextColor>(*kid)
                            .is_some_and(|c| c.0 != Color::NONE)
                    })
                })
                .map(|(player, _)| player.get())
                .collect();
            lit.sort_unstable();
            lit
        };
        assert_eq!(lit(Some(0), &[0, 1]), [0, 1], "nobody has kept yet");
        assert_eq!(
            lit(None, &[1]),
            [1],
            "seat 0 has kept and its view names nobody, but seat 1 is still deciding"
        );
        assert_eq!(lit(Some(1), &[]), [1], "turn 1: the one seat asked");
    }

    /// A chair the house plays is called the house in the player's language,
    /// and a chair it is merely holding keeps its player's name.
    ///
    /// The second half is the one worth having. `away` and `is_ai` are two
    /// fields rather than one precisely so that a thirty-second hiccup does
    /// not rename somebody's chair to the house — a rename that would still
    /// be on the bar after they came back.
    #[test]
    fn a_house_chair_is_called_the_house_in_the_players_own_language() {
        let mut house = cell(SeatRole::House, Lang::De, "House AI");
        assert!(
            said(&mut house).iter().any(|word| word == "Haus-KI"),
            "a German player was still being shown an English word: {:?}",
            said(&mut house)
        );

        let mut held = cell(SeatRole::Away, Lang::De, "Alice");
        assert!(
            said(&mut held).iter().any(|word| word == "Alice"),
            "a held chair still belongs to the player who left it: {:?}",
            said(&mut held)
        );
        assert!(
            !said(&mut held).iter().any(|word| word == "Haus-KI"),
            "and must not be relabelled the house while they are gone"
        );
    }

    /// A chair changing hands redraws the bars, although no view arrived.
    ///
    /// The one field in [`BarRevision`] that does not come from the view, and
    /// the reason it has to be there. A roster is its own payload: the host
    /// marks every seat's roster stale when a chair changes hands and the
    /// fresh `GameStatic` reaches this client as a separate message, so a bar
    /// gated on the view's `seq` alone is built from whichever roster was in
    /// hand at the time and goes on saying so.
    ///
    /// Both directions, and the **second** is the one worth having: a clock
    /// that arrives late is a nuisance, and a clock that stays after the
    /// player has come back is the bar telling the table something untrue —
    /// against the whole design of `away`, which is that a thirty-second
    /// hiccup leaves no trace.
    ///
    /// The counter-half is the third `update`: with the roster settled and
    /// nothing else moved, the bars are *not* rebuilt. Without it this test
    /// would pass just as well against a gate that had been deleted.
    #[test]
    fn a_chair_changing_hands_redraws_the_bar_and_settling_does_not() {
        let bars = |app: &mut App| {
            let mut q = app
                .world_mut()
                .query_filtered::<Entity, With<SeatBarRoot>>();
            q.iter(app.world()).next()
        };
        let set_away = |app: &mut App, away: bool| {
            let mut duel = app.world_mut().resource_mut::<Duel>();
            if let Some(statics) = duel.statics.as_mut() {
                statics.seats[OTHER].away = away;
            }
            crate::rebuild_board(&mut duel);
        };

        let mut app = table();
        let first = bars(&mut app).expect("the seats were given bars");

        // The player steps away. No view arrives — only the roster changes.
        set_away(&mut app, true);
        app.update();
        let held = bars(&mut app).expect("the seats still have bars");
        assert_ne!(
            held, first,
            "the roster said a chair had changed hands and the bar did not              notice, so its clock is one view behind"
        );

        // Nothing moves. The gate is a gate.
        app.update();
        assert_eq!(
            bars(&mut app),
            Some(held),
            "nothing changed and the bars were rebuilt anyway, so the test              above is not measuring a gate at all"
        );

        // And the player comes back.
        set_away(&mut app, false);
        app.update();
        let back = bars(&mut app).expect("and after they return");
        assert_ne!(
            back, held,
            "the clock would still be on a bar whose player is answering              again, which is the half of `away` that must leave no trace"
        );
    }

    /// The mark does not give way; the name does.
    ///
    /// This is the claim that decides whether any of the above is ever *seen*,
    /// and it is asserted on the two nodes rather than on a measured layout
    /// because nothing in this repository runs `bevy_ui`'s layout in a test —
    /// every other `ComputedNode` here is hand-written by the test that reads
    /// it. What the pair stands for: a flex item's `min-width` is `auto`,
    /// which for a `NoWrap` text resolves to the whole string, so a name long
    /// enough to fill the cell refuses to shrink and everything after it is
    /// pushed past the `clip_x` edge. The mark has been in this tree since the
    /// bar was written and was invisible for exactly the names that matter.
    #[test]
    fn a_long_name_gives_way_to_the_mark_and_not_the_other_way_round() {
        let mut app = cell(SeatRole::Away, Lang::En, "Alexandra Wintersmith-Ó");
        let mut q = app.world_mut().query::<(&Text, &Node)>();
        let nodes: Vec<(String, Val, f32)> = q
            .iter(app.world())
            .map(|(text, node)| (text.0.clone(), node.min_width, node.flex_shrink))
            .collect();
        let name = nodes
            .iter()
            .find(|(word, _, _)| word != CLOCK)
            .expect("the cell drew a name");
        let mark = nodes
            .iter()
            .find(|(word, _, _)| word == CLOCK)
            .expect("and a clock beside it");
        assert_eq!(
            name.1,
            px(0),
            "the name keeps `min-width: auto`, so it is the whole string wide \
             and will not shrink: {nodes:?}"
        );
        assert!(
            name.2 > 0.0,
            "and it has to be the one that gives way: {nodes:?}"
        );
        // Not positive, rather than exactly zero: what the row does is divide
        // the overflow by the shrink factors, so any factor at all is a share
        // of the shortfall and the claim is that the mark takes none of it.
        assert!(
            mark.2 <= 0.0,
            "the mark must not be the thing that shrinks — it is one glyph, \
             and a shrunk glyph is no glyph: {nodes:?}"
        );
    }
}
