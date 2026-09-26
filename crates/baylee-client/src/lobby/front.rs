//! The front door: the gateway form, and the account form at the gateway
//! chosen on it (#252).
//!
//! One panel at a time, because the account form means nothing until a
//! gateway is chosen, and a form that is on screen but not yet usable was the
//! least clear thing on this screen. Three panels in all: the gateway form,
//! and the account form's two tabs, signing in and creating an account.
//!
//! # The two motions
//!
//! Choosing a gateway goes *into* it, through the cavity the scene behind
//! the panels frames them in (#295, [`crate::vista`]): the rim brightens,
//! the viewer walks through, and arrives inside a wider cavity on the far
//! side, the light broadened.
//! The panels ride that passage: the gateway panel grows past the viewer
//! from the chosen row and fades, while the account panel comes up out of
//! the depth, from nine tenths of its size and nothing. Back, or Escape, is
//! the same film run backwards, a little faster: the account panel sinks
//! away and the gateway panel comes back from in front.
//!
//! The account form's two tabs sit on one carousel, a quarter turn apart,
//! turning about an upright axis. Both panels are in sight while it turns:
//! the one going swings aside and back, smaller and a touch darker, while
//! the one coming swings forward from the other side. Bevy's UI has no
//! perspective, so each panel is foreshortened as a whole (narrowed by the
//! angle, and scaled by its distance) and not as a trapezoid.
//!
//! The passage takes [`PASSAGE_IN`] (back, [`PASSAGE_OUT`]), the panels
//! moving over [`MOVE_SECONDS`] of it from [`FILM_START`]; the carousel
//! takes [`MOVE_SECONDS`]. Both are eased at both ends. Under
//! `reduce_motion` the panel changes at once and nothing moves. Nothing
//! answers a press while either runs.
//!
//! The tree is rebuilt from state, so a motion cannot live in it:
//! [`FrontMotion`] holds it, [`FrontCast`] says which panels the tree draws
//! (one at rest, both of a motion's while it runs, so it changes only when a
//! motion starts or ends), and [`pose_front`] and [`fade_front`] write each
//! frame's pose onto whatever panels stand.

use super::ui::{Masked, music_toggle, status_ink};
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use crate::frontal::FrontalMaterial;
use bevy::ui::{UiTransform, Val2};

/// How long the panels take to move, in either motion.
const MOVE_SECONDS: f32 = 0.48;

/// How long walking through the cavity takes, and walking back.
const PASSAGE_IN: f32 = 1.0;
/// See [`PASSAGE_IN`].
const PASSAGE_OUT: f32 = 0.8;

/// When in the passage the panels start to move, in seconds of the way in:
/// after the rim has brightened, with the gate passing the viewer.
const FILM_START: f32 = 0.20;

/// How much bigger the gateway panel is when it has been gone through: it
/// passes the viewer, so it ends larger than the screen's middle.
const GROW: f32 = 0.6;

/// The size the account panel comes up from, out of the depth.
const RISE_FROM: f32 = 0.9;

/// How far the viewer stands from the carousel's axis, in panel widths: far
/// enough that a panel a quarter turn away is still in sight, near enough
/// that it is visibly further off.
const VIEW_DISTANCE: f32 = 2.2;

/// How dark a panel is a quarter turn away.
const SHADE: f32 = 0.45;

/// A panel's corner radius, all four corners. The leather inside its one
/// pixel border is cut one pixel tighter (`dock::GROUND_RADIUS`).
pub(super) const CARD_RADIUS: f32 = 14.0;

/// The room above a panel's first row.
const HEADER_TOP: f32 = 6.0;

/// A panel's first row: its title, and the gear. It sits above the tooled
/// line the leather draws 44 px down, in the band that line closes.
const HEADER_HEIGHT: f32 = 36.0;

/// The notice the Fan Content Policy asks of fan content, word for word.
///
/// Not a `Phrase`: it is quoted, not written, and a translation of it would
/// be a notice nobody approved. It stands under the panel in every language.
pub(crate) const FAN_CONTENT_NOTICE: &str = "baylee is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.";

/// The width of a panel on a tablet and a desktop. A phone gives it the
/// whole page.
fn card_width(frame: Frame) -> Val {
    match frame {
        Frame::Phone => percent(100),
        Frame::Tablet => px(480),
        Frame::Desktop => px(520),
    }
}

/// The room the panels stand in: as tall as the tallest of them, so that
/// neither a motion nor a tab moves anything outside the panel. Each panel
/// is as tall as what it holds and stands in the middle of this room, so a
/// short one is not a tall one with a hole at its foot. A phone scrolls
/// instead.
///
/// Measured on a desktop window before guests: the account form on "Create
/// account", four fields and two hints, was about 594 px; the gateway form
/// with its list at the four rows in sight about 548; signing in about 393.
/// The guest entry (#269) adds a field and a rule above the tabs, about 80 px
/// more, which is what this was grown by; it is to be measured again at the
/// next screenshots. A panel taller than this room makes the room taller,
/// and the page moves by the excess.
fn stage_height(frame: Frame) -> Val {
    match frame {
        Frame::Phone => Val::Auto,
        Frame::Tablet | Frame::Desktop => px(690),
    }
}

/// The three panels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Panel {
    /// Choose a gateway.
    #[default]
    Gateway,
    /// Sign in there.
    SignIn,
    /// Create an account there.
    Create,
}

impl Panel {
    /// The panel the lobby asks for.
    fn of(state: &LobbyState) -> Self {
        if !state.lobby.gateway_chosen() {
            Self::Gateway
        } else if matches!(state.lobby.screen(), Screen::SignIn { registering: true }) {
            Self::Create
        } else {
            Self::SignIn
        }
    }

    /// Each panel's own leather: a panel fades on its own, and the leather's
    /// density is a uniform of its material, which is one per slot.
    fn slot(self) -> u8 {
        match self {
            Self::Gateway => 1,
            Self::SignIn => 10,
            Self::Create => 11,
        }
    }
}

/// What is moving: the panel going, the one coming, and how far, `0` to
/// `1`. At rest the two are one panel.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub(crate) struct FrontMotion {
    from: Panel,
    to: Panel,
    t: f32,
    /// Where on the gateway panel the zoom is centred, from the panel's
    /// centre: the chosen row, so that going in reads as going into it.
    anchor: Vec2,
}

impl Default for FrontMotion {
    fn default() -> Self {
        Self::at_rest(Panel::Gateway)
    }
}

impl FrontMotion {
    fn at_rest(panel: Panel) -> Self {
        Self {
            from: panel,
            to: panel,
            t: 1.0,
            anchor: Vec2::ZERO,
        }
    }

    /// Whether a panel is on its way. Nothing answers a press until it has
    /// landed.
    pub(crate) fn moving(&self) -> bool {
        self.from != self.to
    }

    /// Whether this is the passage through the cavity, rather than the
    /// carousel of the account form's tabs.
    fn through_the_door(&self) -> bool {
        self.from == Panel::Gateway || self.to == Panel::Gateway
    }

    /// How long this motion takes.
    fn seconds(&self) -> f32 {
        if !self.through_the_door() {
            MOVE_SECONDS
        } else if self.to == Panel::Gateway {
            PASSAGE_OUT
        } else {
            PASSAGE_IN
        }
    }

    /// How far through the cavity the viewer is: 0 before it, on the
    /// gateway's side, 1 arrived. The carousel turns on the far side.
    pub(crate) fn progress(&self) -> f32 {
        if !self.moving() || !self.through_the_door() {
            return if self.to == Panel::Gateway { 0.0 } else { 1.0 };
        }
        if self.to == Panel::Gateway {
            1.0 - self.t
        } else {
            self.t
        }
    }

    /// Lands on the panel the lobby asks for, without moving.
    #[cfg(test)]
    pub(crate) fn settle(&mut self, state: &LobbyState) {
        *self = Self::at_rest(Panel::of(state));
    }
}

/// Which panels the tree draws: the one standing, and while a motion runs
/// the one going too.
#[derive(Resource, Default, PartialEq, Eq, Debug)]
pub(super) struct FrontCast {
    pub(super) shown: Panel,
    pub(super) going: Option<Panel>,
}

/// A panel on screen, for [`pose_front`] and [`fade_front`].
#[derive(Component)]
pub(super) struct FrontCard(pub(super) Panel);

/// The dark a panel takes on as it turns away.
#[derive(Component)]
pub(super) struct FrontShade;

/// The tab of the account form that is open.
#[derive(Component)]
pub(super) struct ActiveTab;

/// Where each saved gateway's row stood on the gateway panel, from the
/// panel's centre, the last time the panel stood still. Read when a motion
/// starts, since by then the row is being rebuilt.
#[derive(Resource, Default)]
pub(super) struct RowPlaces(Vec<(String, Vec2)>);

/// Moves the motion on, and starts one when the lobby asks for another panel.
pub(super) fn move_front(
    time: Res<Time>,
    state: Res<LobbyState>,
    prefs: Res<crate::prefs::Prefs>,
    places: Res<RowPlaces>,
    mut motion: ResMut<FrontMotion>,
    mut cast: ResMut<FrontCast>,
) {
    let target = Panel::of(&state);
    // Anywhere but the front door there is nothing to move, and coming back
    // to it (signing out) should find it already standing.
    let still = prefs.all().reduce_motion || !matches!(state.lobby.screen(), Screen::SignIn { .. });
    let mut next = *motion;
    if still {
        next = FrontMotion::at_rest(target);
    } else {
        let start = |from: Panel| FrontMotion {
            from,
            to: target,
            t: 0.0,
            anchor: places
                .0
                .iter()
                .find(|(url, _)| *url == state.gateway)
                .map_or(Vec2::ZERO, |(_, at)| *at),
        };
        if !next.moving() {
            if target != next.to {
                next = start(next.to);
            }
        } else if target == next.from {
            // Asked back before it landed: the same film, run backwards from
            // the frame it had reached.
            next = FrontMotion {
                from: next.to,
                to: next.from,
                t: 1.0 - next.t,
                anchor: next.anchor,
            };
        } else if target != next.to {
            next = start(next.to);
        }
        if next.moving() {
            next.t += time.delta_secs() / next.seconds();
            if next.t >= 1.0 {
                next = FrontMotion::at_rest(next.to);
            }
        }
    }
    motion.set_if_neq(next);
    cast.set_if_neq(FrontCast {
        shown: next.to,
        going: next.moving().then_some(next.from),
    });
}

/// Tells the scene behind the front door where the passage is, and whether
/// the front door is on screen at all.
pub(super) fn show_scene(
    state: Res<LobbyState>,
    motion: Res<FrontMotion>,
    mut scene: ResMut<crate::vista::FrontScene>,
) {
    let peak = if motion.to == Panel::Gateway {
        crate::vista::HAZE_OUT
    } else {
        crate::vista::HAZE_IN
    };
    scene.set_if_neq(crate::vista::FrontScene {
        stage: crate::vista::passage(motion.progress(), peak),
        shown: matches!(state.lobby.screen(), Screen::SignIn { .. }),
    });
}

/// Eased at both ends.
fn ease(t: f32) -> f32 {
    0.5 - 0.5 * (std::f32::consts::PI * t.clamp(0.0, 1.0)).cos()
}

/// Where one panel stands in one frame of a motion.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Pose {
    scale: Vec2,
    /// From where it stands at rest, in logical pixels.
    shift: Vec2,
    /// How much of it there is: `1` whole, `0` gone.
    alpha: f32,
    /// How dark it has turned, `0` not at all.
    shade: f32,
    /// Whether it is drawn over the other panel of the motion.
    over: bool,
}

impl Pose {
    const REST: Self = Self {
        scale: Vec2::ONE,
        shift: Vec2::ZERO,
        alpha: 1.0,
        shade: 0.0,
        over: true,
    };
}

/// Where `panel` stands in this frame of `motion`, on a panel `width` wide.
fn pose(motion: &FrontMotion, panel: Panel, width: f32) -> Pose {
    if !motion.moving() || (panel != motion.from && panel != motion.to) {
        return Pose::REST;
    }
    let eased = ease(motion.t);
    if motion.through_the_door() {
        // 0 with the gateway form up, 1 with the account form up: the
        // panels' share of the passage, which is the same frames either way.
        let film = (motion.progress() * PASSAGE_IN - FILM_START) / MOVE_SECONDS;
        door_pose(panel, ease(film), motion.anchor)
    } else {
        // 0 with signing in up, 1 with creating an account up.
        let round = if motion.to == Panel::Create {
            eased
        } else {
            1.0 - eased
        };
        carousel_pose(panel, round, width)
    }
}

/// Going into a gateway, `into` of the way.
fn door_pose(panel: Panel, into: f32, anchor: Vec2) -> Pose {
    if panel == Panel::Gateway {
        let scale = 1.0 + GROW * into;
        Pose {
            scale: Vec2::splat(scale),
            // Scaled about the anchor rather than about its own centre: the
            // chosen row stays where it was while everything else leaves it.
            shift: anchor * (1.0 - scale),
            alpha: 1.0 - smoothstep(0.0, 0.75, into),
            shade: 0.0,
            // In front: it is passing the viewer.
            over: true,
        }
    } else {
        Pose {
            scale: Vec2::splat(RISE_FROM + (1.0 - RISE_FROM) * into),
            shift: Vec2::ZERO,
            alpha: smoothstep(0.3, 1.0, into),
            shade: 0.0,
            over: false,
        }
    }
}

/// The account form's carousel, `round` of the way from signing in to
/// creating an account, for a panel `width` wide.
///
/// The two panels are two faces of a square prism a quarter turn apart,
/// turning about an upright axis half a panel behind them, so their near
/// edges meet at the middle when both are half-way.
fn carousel_pose(panel: Panel, round: f32, width: f32) -> Pose {
    let quarter = std::f32::consts::FRAC_PI_2;
    let angle = if panel == Panel::SignIn {
        -quarter * round
    } else {
        quarter * (1.0 - round)
    };
    let (sin, cos) = angle.sin_cos();
    // How far behind its resting place it is, in panel widths.
    let depth = 0.5 * (1.0 - cos);
    let distance = VIEW_DISTANCE / (VIEW_DISTANCE + depth);
    Pose {
        // Never quite zero: a node of no width is one nothing can invert.
        scale: Vec2::new((cos * distance).max(0.001), distance),
        shift: Vec2::new(0.5 * width * sin * distance, 0.0),
        alpha: 1.0,
        shade: SHADE * (1.0 - cos),
        over: angle.abs() < quarter * 0.5,
    }
}

fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Writes this frame's pose onto the standing panels: where they are, which
/// is in front, and how dark they have turned. And, while the gateway panel
/// stands still, notes where its rows are, for the next motion's anchor.
#[allow(clippy::type_complexity)]
pub(super) fn pose_front(
    motion: Res<FrontMotion>,
    state: Res<LobbyState>,
    mut places: ResMut<RowPlaces>,
    mut cards: Query<(
        &FrontCard,
        &mut UiTransform,
        &mut ZIndex,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
        &Children,
    )>,
    mut shades: Query<&mut BackgroundColor, With<FrontShade>>,
    rows: Query<(&Press, &ComputedNode, &bevy::ui::UiGlobalTransform), Without<FrontCard>>,
) {
    for (card, mut transform, mut z, computed, global, children) in &mut cards {
        let width = computed.size().x * computed.inverse_scale_factor();
        let posed = pose(&motion, card.0, width);
        transform.set_if_neq(UiTransform {
            translation: Val2::px(posed.shift.x, posed.shift.y),
            scale: posed.scale,
            ..default()
        });
        z.set_if_neq(ZIndex(i32::from(posed.over)));
        for child in children {
            if let Ok(mut shade) = shades.get_mut(*child) {
                shade.set_if_neq(BackgroundColor(Color::BLACK.with_alpha(posed.shade)));
            }
        }
        if card.0 == Panel::Gateway && !motion.moving() && computed.size().y > 0.0 {
            let centre = global.translation;
            let scale = computed.inverse_scale_factor();
            let found: Vec<(String, Vec2)> = rows
                .iter()
                .filter_map(|(press, node, at)| match press {
                    Press::SelectGateway(index) if node.size().y > 0.0 => state
                        .gateways
                        .get(*index)
                        .map(|url| (url.clone(), (at.translation - centre) * scale)),
                    _ => None,
                })
                .collect();
            if !found.is_empty() {
                places.0 = found;
            }
        }
    }
}

/// The colours a node had when its panel began to fade, which each faded
/// frame is a share of.
#[derive(Component, Clone)]
pub(super) struct Unfaded {
    fill: Option<Color>,
    border: Option<BorderColor>,
    ink: Option<Color>,
    shadow: Option<Vec<Color>>,
}

/// Fades a panel on its way through the door, and its leather with it.
///
/// Bevy's UI has no opacity for a node and everything under it, so every
/// colour under the panel is written as a share of what it was when the
/// fade began. No panel is ever put back: when the motion ends the tree is
/// rebuilt, with every colour as the builder wrote it. The leather is the
/// exception, since its material outlives the tree, so its density is
/// written every frame, back to whole at rest.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn fade_front(
    mut commands: Commands,
    motion: Res<FrontMotion>,
    cards: Query<(Entity, &FrontCard)>,
    children: Query<&Children>,
    mut nodes: Query<(
        Option<&mut BackgroundColor>,
        Option<&mut BorderColor>,
        Option<&mut TextColor>,
        Option<&mut BoxShadow>,
        Option<&Unfaded>,
    )>,
    grounds: Query<&MaterialNode<FrontalMaterial>>,
    materials: Option<ResMut<Assets<FrontalMaterial>>>,
    mut grains: Local<Vec<(AssetId<FrontalMaterial>, f32)>>,
) {
    let mut materials = materials;
    for (card, panel) in &cards {
        let alpha = pose(&motion, panel.0, 0.0).alpha;
        for node in std::iter::once(card).chain(children.iter_descendants(card)) {
            if let Ok(ground) = grounds.get(node)
                && let Some(materials) = materials.as_mut()
            {
                fade_leather(materials, &ground.0, alpha, &mut grains);
            }
            if alpha >= 1.0 {
                continue;
            }
            let Ok((fill, border, ink, shadow, unfaded)) = nodes.get_mut(node) else {
                continue;
            };
            // The first faded frame: what it is now is what it was. Faded in
            // this same frame, or a panel coming out of nothing would stand
            // whole for one frame first.
            let fresh;
            let unfaded = if let Some(unfaded) = unfaded {
                unfaded
            } else {
                fresh = Unfaded {
                    fill: fill.as_ref().map(|f| f.0),
                    border: border.as_deref().copied(),
                    ink: ink.as_ref().map(|i| i.0),
                    shadow: shadow
                        .as_ref()
                        .map(|s| s.0.iter().map(|layer| layer.color).collect()),
                };
                commands.entity(node).insert(fresh.clone());
                &fresh
            };
            let share = |colour: Color| colour.with_alpha(colour.alpha() * alpha);
            if let (Some(mut fill), Some(base)) = (fill, unfaded.fill) {
                fill.set_if_neq(BackgroundColor(share(base)));
            }
            if let (Some(mut border), Some(base)) = (border, unfaded.border) {
                border.set_if_neq(BorderColor {
                    top: share(base.top),
                    right: share(base.right),
                    bottom: share(base.bottom),
                    left: share(base.left),
                });
            }
            if let (Some(mut ink), Some(base)) = (ink, unfaded.ink) {
                ink.set_if_neq(TextColor(share(base)));
            }
            if let (Some(mut shadow), Some(base)) = (shadow, unfaded.shadow.as_ref()) {
                for (layer, colour) in shadow.0.iter_mut().zip(base) {
                    layer.color = share(*colour);
                }
            }
        }
    }
}

/// The leather's own fade: its density, and the grain that moves it, as a
/// share of what they are at rest.
fn fade_leather(
    materials: &mut Assets<FrontalMaterial>,
    handle: &Handle<FrontalMaterial>,
    alpha: f32,
    grains: &mut Vec<(AssetId<FrontalMaterial>, f32)>,
) {
    let Some(material) = materials.get(handle) else {
        return;
    };
    let id = handle.id();
    let grain = if let Some((_, grain)) = grains.iter().find(|(seen, _)| *seen == id) {
        *grain
    } else {
        // First seen at rest, so what it has now is the whole of it.
        grains.push((id, material.params.grain));
        material.params.grain
    };
    let density = super::dock::GROUND_DENSITY * alpha;
    if (material.params.ramp.y - density).abs() < 1e-4 {
        return;
    }
    let Some(mut material) = materials.get_mut(handle) else {
        return;
    };
    material.params.ramp.y = density;
    material.params.ramp.z = density;
    material.params.grain = grain * alpha;
}

/// The front door's panels, standing in one place: the one the lobby asks
/// for, and while a motion runs the one going as well.
pub(super) fn stage(
    commands: &mut Commands,
    state: &LobbyState,
    cast: &FrontCast,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    // The room the tallest panel needs, so the page does not jump when the
    // panel changes, with the stage in its middle.
    let room = commands
        .spawn((
            Node {
                width: card_width(metrics.frame),
                max_width: percent(100),
                min_height: stage_height(metrics.frame),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let stage = commands
        .spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                grid_template_columns: vec![GridTrack::flex(1.0)],
                grid_template_rows: vec![GridTrack::auto()],
                ..default()
            },
            // What the scene behind is framed around: the stage keeps its
            // place while the panels on it move, and is as tall as the
            // panels on it, not as the room kept for them (#295).
            crate::vista::Framed(crate::vista::Vista::Front),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(room).add_child(stage);
    let panels = [Some(cast.shown), cast.going];
    for panel in panels.into_iter().flatten() {
        let card = card(
            commands,
            state,
            panel,
            cast.going.is_none(),
            fonts,
            metrics,
            scrolled_to,
        );
        commands.entity(stage).add_child(card);
    }
    room
}

/// The card's elevation: a close ambient shadow and a long key shadow. It is
/// the panel's own, so it goes wherever the panel goes.
///
/// Its own and not [`soft_shadow`]: that one is a control's, and a control
/// sits on a surface where this panel stands off the page.
pub(super) fn card_shadow() -> BoxShadow {
    BoxShadow(vec![
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.35),
            x_offset: px(0),
            y_offset: px(2),
            spread_radius: px(0),
            blur_radius: px(6),
        },
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.50),
            x_offset: px(0),
            y_offset: px(12),
            spread_radius: px(-2),
            blur_radius: px(30),
        },
    ])
}

/// One panel. `standing` is whether it is at rest, which is when its gear
/// menu may be open.
fn card(
    commands: &mut Commands,
    state: &LobbyState,
    panel: Panel,
    standing: bool,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) -> Entity {
    let side = metrics.pad * 1.6;
    let card = commands
        .spawn((
            FrontCard(panel),
            Node {
                grid_row: GridPlacement::start(1),
                grid_column: GridPlacement::start(1),
                width: percent(100),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                row_gap: px(card_gap(metrics)),
                // The foot's inlays stand clear of the tooled line, and the
                // last row stands clear of them.
                padding: UiRect {
                    left: px(side),
                    right: px(side),
                    top: px(HEADER_TOP),
                    bottom: px(side + super::dock::INLAY_LIFT),
                },
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(CARD_RADIUS)),
                ..default()
            },
            // No fill of its own: the leather under it is the fill, cut to
            // the same corners, and a square fill here is what showed past
            // the rounding before.
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            card_shadow(),
            UiTransform::default(),
            ZIndex(1),
            super::dock::Dock(panel.slot()),
            Pickable::IGNORE,
        ))
        .id();
    match panel {
        Panel::Gateway => gateway_face(commands, card, state, fonts, metrics, scrolled_to),
        Panel::SignIn => account_face(commands, card, state, fonts, metrics, false),
        Panel::Create => account_face(commands, card, state, fonts, metrics, true),
    }
    if standing && state.front_menu {
        let menu = gear_menu(commands, state, fonts, metrics, side);
        commands.entity(card).add_child(menu);
    }
    let shade = commands
        .spawn((
            FrontShade,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                border_radius: BorderRadius::all(px(CARD_RADIUS - 1.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            ZIndex(10),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(shade);
    card
}

/// A panel's first row, above the leather's tooled line: an optional way
/// back, the title, and the gear.
fn header(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    back: bool,
    title: &str,
) -> Entity {
    let lang = state.lobby.lang();
    let header = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(HEADER_HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if back {
        let busy = state.lobby.busy();
        let back = icon_button(
            commands,
            fonts,
            metrics,
            BACK_GLYPH,
            (!busy).then_some(Press::LeaveGateway),
            Phrase::Back.text(lang),
            false,
        );
        commands.entity(header).add_child(back);
    }
    let words = commands
        .spawn((
            Text::new(title),
            tf(fonts, metrics.head),
            TextColor(palette::INK),
            TextLayout::no_wrap(),
            Node {
                flex_grow: 1.0,
                min_width: px(0),
                overflow: Overflow::clip_x(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let gear = icon_button(
        commands,
        fonts,
        metrics,
        GEAR_GLYPH,
        Some(Press::FrontMenu),
        Phrase::LanguageAndSettings.text(lang),
        state.front_menu,
    );
    let music = music_toggle(commands, fonts, metrics, lang);
    commands.entity(header).add_children(&[words, music, gear]);
    header
}

/// A square button with one glyph and its meaning in a hint.
fn icon_button(
    commands: &mut Commands,
    fonts: &UiFonts,
    metrics: Metrics,
    glyph: char,
    press: Option<Press>,
    meaning: &str,
    lit: bool,
) -> Entity {
    let side = if metrics.frame == Frame::Phone {
        metrics.tap
    } else {
        HEADER_HEIGHT
    };
    let rest = if lit { palette::PANEL_HOT } else { Color::NONE };
    let button = commands
        .spawn((
            Node {
                width: px(side),
                height: px(side),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(rest),
            super::hint::HoverHint(meaning.to_string()),
        ))
        .id();
    match press {
        Some(press) => {
            commands.entity(button).insert((
                press,
                crate::ambience::Feel::rising_to(rest, palette::PANEL_HOT),
            ));
        }
        None => {
            commands.entity(button).insert(Pickable::IGNORE);
        }
    }
    let ink = if press.is_some() {
        palette::DOCK_INK
    } else {
        palette::MUTED
    };
    let glyph = commands
        .spawn((
            Text::new(glyph.to_string()),
            crate::hud::icon_tf(fonts, metrics.text),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_child(glyph);
    button
}

/// The gear's menu, under the gear: the language, by each language's own
/// name, and the way to the whole settings screen.
///
/// A veil over the rest of the page closes it when pressed, as any press
/// outside a menu does.
fn gear_menu(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    side: f32,
) -> Entity {
    let lang = state.lobby.lang();
    let holder = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(-4000),
                right: px(-4000),
                top: px(-4000),
                bottom: px(-4000),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(590),
            Press::FrontMenu,
        ))
        .id();
    let menu = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_TOP + HEADER_HEIGHT + 4.0),
                right: px(side - 6.0),
                width: px(240),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad * 0.8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            soft_shadow(),
            GlobalZIndex(600),
            // A press on the menu's own ground is not a press outside it.
            Press::PickerNothing,
        ))
        .id();
    let caption = note(commands, fonts, metrics, Phrase::Language.text(lang));
    let mut chips = Vec::new();
    for offered in Lang::ALL {
        let chip = button(
            commands,
            fonts,
            metrics,
            offered.name(),
            Press::PickLang(offered),
            palette::PANEL,
            true,
        );
        if offered == lang {
            commands.entity(chip).insert((
                BackgroundColor(palette::PANEL_HOT),
                crate::ambience::Feel::new(palette::PANEL_HOT),
            ));
        }
        chips.push(chip);
    }
    let languages = halves(commands, metrics, &chips);
    let rule = rule(commands);
    let all = button(
        commands,
        fonts,
        metrics,
        Phrase::AllSettings.text(lang),
        Press::OpenSettings,
        palette::PANEL,
        true,
    );
    commands.entity(all).entry::<Node>().and_modify(|mut node| {
        node.justify_content = JustifyContent::Center;
    });
    commands
        .entity(menu)
        .add_children(&[caption, languages, rule, all]);
    commands.entity(holder).add_children(&[veil, menu]);
    holder
}

/// A hairline across a panel.
fn rule(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                height: px(1),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DOCK_EDGE.with_alpha(0.30)),
            Pickable::IGNORE,
        ))
        .id()
}

/// The space between two rows of a panel.
fn card_gap(metrics: Metrics) -> f32 {
    metrics.gap * 1.5
}

/// The lobby's status line, between two rows of a panel: the slot is a
/// sliver, and the line is laid over it and the two gaps round it, which
/// hold two lines with air. A refusal arriving does not push the form about,
/// and an empty status leaves no hole in it.
fn status_slot(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let sliver = metrics.small;
    let slot = commands
        .spawn((
            Node {
                width: percent(100),
                height: px(sliver),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let status = commands
        .spawn((
            Text::new(state.lobby.status()),
            tf(fonts, metrics.small),
            TextColor(status_ink(state.lobby.tone())),
            Pickable::IGNORE,
        ))
        .id();
    let band = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(-card_gap(metrics)),
                height: px(2.0 * card_gap(metrics) + sliver),
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(band).add_child(status);
    commands.entity(slot).add_child(band);
    slot
}

/// Under the panels: this build, the notice the Fan Content Policy asks
/// for, word for word, and where the source is.
///
/// The build is the same string the signed-in header draws
/// (`baylee_build::short()`), for the same reason: it is what a bug report
/// is worthless without. The source line is the AGPL's §13 offer (#270),
/// drawn where every player passes; see [`source_address`].
pub(super) fn colophon(
    commands: &mut Commands,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) -> Entity {
    let colophon = commands
        .spawn((
            Node {
                width: card_width(metrics.frame),
                max_width: percent(100),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(metrics.gap * 0.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let build = commands
        .spawn((
            Text::new(baylee_build::short()),
            tf(fonts, metrics.small * 0.9),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let notice = commands
        .spawn((
            Text::new(FAN_CONTENT_NOTICE),
            tf(fonts, metrics.small * 0.85),
            TextColor(palette::MUTED.with_alpha(0.8)),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    let mut source = commands.spawn((
        Text::new(Phrase::SourceCode.fill(state.lobby.lang(), &[source_address(state)])),
        tf(fonts, metrics.small * 0.85),
        TextColor(palette::MUTED.with_alpha(0.8)),
        // An address is one long word; on a phone it breaks where it
        // must rather than running off the card.
        TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
    ));
    // A link only for an address that passed the check at the door
    // (`source::keep_the_code`), and the same address as a code beside it
    // on a screen a phone could be held up to (#299).
    let code = state.source_code.as_ref();
    if code.is_some() {
        source.insert((
            Button,
            Press::OpenSource,
            Underline,
            UnderlineColor(palette::MUTED.with_alpha(0.5)),
        ));
    } else {
        source.insert(Pickable::IGNORE);
    }
    let source = source.id();
    commands
        .entity(colophon)
        .add_children(&[build, notice, source]);
    if let Some(code) = code.filter(|_| metrics.frame != Frame::Phone) {
        #[allow(clippy::cast_precision_loss)] // a code is at most 177 modules a side
        let side = px(code.side as f32 * super::source::MODULE_PX);
        let picture = commands
            .spawn((
                ImageNode::new(code.image.clone()),
                Node {
                    width: side,
                    height: side,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // Framed like the form it belongs to (#295), so it is a plate of
        // the page and not a hole in the scene.
        let frame = commands
            .spawn((
                Node {
                    padding: UiRect::all(px(3)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(5)),
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
                Button,
                Press::OpenSource,
            ))
            .add_child(picture)
            .id();
        commands.entity(colophon).add_child(frame);
    }
    colophon
}

/// Where the source is, for the colophon.
///
/// The AGPL's §13 makes whoever runs a modified gateway offer its players
/// that version's source, so a gateway says where in `/info` (`source`,
/// #270) and the front door shows what the gateway it points at said. A
/// gateway that has not answered, or one too old to say, leaves this
/// client's own repository, which is this program's source either way.
pub(super) fn source_address(state: &LobbyState) -> &str {
    match state.probes.get(&state.gateway) {
        Some(Probe::Known(info)) => info.source.as_deref(),
        _ => None,
    }
    .unwrap_or(baylee_build::REPOSITORY)
}

/// Two controls sharing a row in equal halves.
fn halves(commands: &mut Commands, metrics: Metrics, controls: &[Entity]) -> Entity {
    let halves = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::Center,
                column_gap: px(metrics.gap * 1.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for &control in controls {
        commands
            .entity(control)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_basis = px(0);
                node.flex_grow = 1.0;
                node.min_width = px(0);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(halves).add_child(control);
    }
    halves
}

/// Panel one: which gateway.
fn gateway_face(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    scrolled_to: &Scrolled,
) {
    let lang = state.lobby.lang();
    let header = header(
        commands,
        state,
        fonts,
        metrics,
        false,
        Phrase::ChooseGateway.text(lang),
    );
    let hint = note(commands, fonts, metrics, Phrase::GatewayHint.text(lang));
    commands.entity(card).add_children(&[header, hint]);

    if !state.gateways.is_empty() {
        let list = super::gateway::list(commands, state, fonts, metrics, scrolled_to);
        commands.entity(card).add_child(list);
    }

    let field = text_field(
        commands,
        fonts,
        metrics,
        Phrase::GatewayAddress.text(lang),
        &FieldLook {
            buffer: state.lobby.buffer(Field::Gateway),
            focused: state.lobby.focus() == Field::Gateway,
            mask: None,
            press: Press::Focus(Field::Gateway),
            lead: None,
            hint: Some("https://"),
            tail: None,
        },
    );
    let save = button(
        commands,
        fonts,
        metrics,
        Phrase::SaveGateway.text(lang),
        Press::AddGateway,
        palette::PANEL_LIT,
        !state.lobby.busy() && state.adding.is_none(),
    );
    if metrics.frame == Frame::Phone {
        commands.entity(card).add_children(&[field, save]);
    } else {
        // The address and its button, one group on one line: the box grows,
        // the button keeps its width, and the two share a baseline because
        // the row sits at its foot, under the box's caption.
        let add = commands
            .spawn((
                Node {
                    width: percent(100),
                    align_items: AlignItems::FlexEnd,
                    column_gap: px(metrics.gap * 1.5),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(field)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_grow = 1.0;
                node.min_width = px(0);
            });
        commands
            .entity(save)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = px(150);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(add).add_children(&[field, save]);
        commands.entity(card).add_child(add);
    }
    let status = status_slot(commands, state, fonts, metrics);
    let rule = rule(commands);
    // Playing without a gateway, apart from the list, across the whole
    // panel: the one door on this panel that does not lead to a gateway.
    let offline = button(
        commands,
        fonts,
        metrics,
        Phrase::PlayOffline.text(lang),
        Press::PlayOffline,
        palette::PANEL,
        true,
    );
    commands
        .entity(offline)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = percent(100);
            node.justify_content = JustifyContent::Center;
        });
    commands.entity(offline).insert(super::hint::HoverHint(
        Phrase::OfflineBenefit.text(lang).to_string(),
    ));
    commands.entity(card).add_children(&[status, rule, offline]);
}

/// Panels two and three: the account, at the gateway chosen on panel one,
/// signing in or creating one.
///
/// Each word once: the gateway's name is the title, the tabs say which of
/// the two this is, and the submit says only that it goes on.
#[allow(clippy::too_many_lines)] // one flat form, read top to bottom
fn account_face(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
    registering: bool,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();

    let title = super::gateway::title_of(state, &state.gateway);
    let header = header(commands, state, fonts, metrics, true, &title);
    let line = super::gateway::chosen_line(commands, state, fonts, metrics);
    commands.entity(card).add_children(&[header, line]);

    // Playing as a guest (#269), first, between the gateway and the tabs, and
    // a rule under it: the tabs below are the other way in.
    if lobby.guest_offered() {
        guest_entry(commands, card, state, fonts, metrics);
        let rule = rule(commands);
        commands.entity(card).add_child(rule);
    }

    let mut tabs = Vec::new();
    for (label, active) in [
        (Phrase::SignIn, !registering),
        (Phrase::CreateAccount, registering),
    ] {
        let enabled =
            !lobby.busy() && (label != Phrase::CreateAccount || lobby.registration_enabled());
        let tab = button(
            commands,
            fonts,
            metrics,
            label.text(lang),
            if active {
                Press::PickerNothing
            } else {
                Press::ToggleRegistering
            },
            palette::PANEL,
            enabled,
        );
        // `button` draws every enabled tone but the loud ones as the same
        // key, so the tab that is open is lit here.
        if active && enabled {
            commands.entity(tab).insert((
                ActiveTab,
                BackgroundColor(palette::PANEL_HOT),
                crate::ambience::Feel::new(palette::PANEL_HOT),
            ));
        }
        tabs.push(tab);
    }
    let tabs = halves(commands, metrics, &tabs);
    commands.entity(card).add_child(tabs);

    let plain = |field: Field| FieldLook {
        buffer: lobby.buffer(field),
        focused: lobby.focus() == field,
        mask: None,
        press: Press::Focus(field),
        lead: None,
        hint: None,
        tail: None,
    };
    let masked = |field: Field| FieldLook {
        mask: Some(Masked {
            field,
            shown: lobby.showing(field),
        }),
        ..plain(field)
    };
    let username = text_field(
        commands,
        fonts,
        metrics,
        Phrase::Username.text(lang),
        &plain(Field::Username),
    );
    commands.entity(card).add_child(username);
    if registering {
        let name = text_field(
            commands,
            fonts,
            metrics,
            Phrase::DisplayName.text(lang),
            &plain(Field::DisplayName),
        );
        let hint = note(commands, fonts, metrics, Phrase::AccountNameHint.text(lang));
        commands.entity(card).add_children(&[name, hint]);
    }
    let password = text_field(
        commands,
        fonts,
        metrics,
        Phrase::Password.text(lang),
        &masked(Field::Password),
    );
    commands.entity(card).add_child(password);
    if registering {
        let again = text_field(
            commands,
            fonts,
            metrics,
            Phrase::PasswordAgain.text(lang),
            &masked(Field::PasswordAgain),
        );
        let hint = note(
            commands,
            fonts,
            metrics,
            Phrase::AccountPasswordHint.text(lang),
        );
        commands.entity(card).add_children(&[again, hint]);
    }

    let submit = button(
        commands,
        fonts,
        metrics,
        Phrase::Continue.text(lang),
        Press::Submit,
        palette::ACCENT,
        state.gateway_selected && !lobby.busy(),
    );
    commands
        .entity(submit)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::Center;
        });
    // Over the form's last gap, next to what it answers.
    let status = status_slot(commands, state, fonts, metrics);
    commands.entity(card).add_children(&[status, submit]);
}

/// The guest's way in (#269): the guest this device keeps here, as one
/// full-width button with its handle, or a name to play under and the button
/// beside it — the gateway form's address row, in shape and in behaviour.
/// Either way the hint says what a guest is.
fn guest_entry(
    commands: &mut Commands,
    card: Entity,
    state: &LobbyState,
    fonts: &UiFonts,
    metrics: Metrics,
) {
    let lobby = &state.lobby;
    let lang = lobby.lang();
    let enabled = state.gateway_selected && !lobby.busy();
    let hint = super::hint::HoverHint(Phrase::GuestNotice.text(lang).to_string());
    if let Some(kept) = lobby.kept_guest() {
        let back = button(
            commands,
            fonts,
            metrics,
            &Phrase::ContinueAsGuest.fill(lang, &[&kept.handle]),
            Press::PlayAsGuest,
            palette::PANEL_LIT,
            enabled,
        );
        commands
            .entity(back)
            .insert(hint)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.width = percent(100);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(card).add_child(back);
        return;
    }
    let field = text_field(
        commands,
        fonts,
        metrics,
        Phrase::GuestName.text(lang),
        &FieldLook {
            buffer: lobby.buffer(Field::GuestName),
            focused: lobby.focus() == Field::GuestName,
            mask: None,
            press: Press::Focus(Field::GuestName),
            lead: None,
            hint: Some(Phrase::GuestDefaultName.text(lang)),
            tail: None,
        },
    );
    let play = button(
        commands,
        fonts,
        metrics,
        Phrase::PlayAsGuest.text(lang),
        Press::PlayAsGuest,
        palette::PANEL_LIT,
        enabled,
    );
    commands.entity(play).insert(hint);
    if metrics.frame == Frame::Phone {
        commands.entity(card).add_children(&[field, play]);
        return;
    }
    let row = commands
        .spawn((
            Node {
                width: percent(100),
                align_items: AlignItems::FlexEnd,
                column_gap: px(metrics.gap * 1.5),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(field)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.flex_grow = 1.0;
            node.min_width = px(0);
        });
    commands
        .entity(play)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = px(170);
            node.justify_content = JustifyContent::Center;
        });
    commands.entity(row).add_children(&[field, play]);
    commands.entity(card).add_child(row);
}

/// Font Awesome's chevron-left.
const BACK_GLYPH: char = '\u{f053}';

/// Font Awesome's gear.
const GEAR_GLYPH: char = '\u{f013}';

#[cfg(test)]
mod tests {
    use super::*;

    fn moving(from: Panel, to: Panel, t: f32) -> FrontMotion {
        FrontMotion {
            from,
            to,
            t,
            anchor: Vec2::new(0.0, -80.0),
        }
    }

    #[test]
    fn the_status_band_holds_two_lines_at_every_width() {
        // `status_slot` lays the line over the sliver and the gap either
        // side of it, instead of reserving two empty lines, so a refusal
        // must still find two lines of room there. Bevy's default line
        // height is 1.2 of the font.
        for width in [390.0, 900.0, 1728.0] {
            let metrics = Metrics::of(width);
            let band = 2.0 * card_gap(metrics) + metrics.small;
            let two_lines = 2.0 * 1.2 * metrics.small;
            assert!(band >= two_lines, "{width}: band {band} < {two_lines}");
        }
    }

    #[test]
    fn the_notice_is_the_policy_s_own_words() {
        // Pinned byte for byte: the Fan Content Policy asks for this text,
        // and a notice that was tidied is not the one it asks for.
        assert_eq!(
            FAN_CONTENT_NOTICE,
            "baylee is unofficial Fan Content permitted under the Fan Content Policy. \
             Not approved/endorsed by Wizards. Portions of the materials used are \
             property of Wizards of the Coast. \u{a9}Wizards of the Coast LLC."
        );
    }

    #[test]
    fn going_into_a_gateway_grows_it_past_the_viewer_from_its_row() {
        let half = moving(Panel::Gateway, Panel::SignIn, 0.5);
        let gateway = pose(&half, Panel::Gateway, 520.0);
        let account = pose(&half, Panel::SignIn, 520.0);
        assert!(gateway.scale.x > 1.2 && gateway.over, "{gateway:?}");
        assert!(
            gateway.shift.y > 0.0,
            "the row above the centre stays put, so the panel moves down: {gateway:?}"
        );
        assert!(account.scale.x > RISE_FROM && account.scale.x < 1.0);
        assert!(gateway.alpha < 1.0 && account.alpha > 0.0);

        let end = moving(Panel::Gateway, Panel::SignIn, 1.0);
        assert!(pose(&end, Panel::Gateway, 520.0).alpha < 1e-6);
        assert_eq!(pose(&end, Panel::SignIn, 520.0).scale, Vec2::ONE);

        // Back is the same film backwards: a quarter of the way back is
        // three quarters of the way in.
        let back = moving(Panel::SignIn, Panel::Gateway, 0.25);
        let forth = moving(Panel::Gateway, Panel::SignIn, 0.75);
        for panel in [Panel::Gateway, Panel::SignIn] {
            assert_eq!(pose(&back, panel, 520.0), pose(&forth, panel, 520.0));
        }
    }

    #[test]
    fn the_tabs_turn_on_a_carousel_with_both_in_sight() {
        let half = moving(Panel::SignIn, Panel::Create, 0.5);
        let going = pose(&half, Panel::SignIn, 520.0);
        let coming = pose(&half, Panel::Create, 520.0);
        for posed in [going, coming] {
            assert!(posed.scale.x > 0.5 && posed.scale.x < 0.8, "{posed:?}");
            assert!(posed.scale.y < 1.0, "further off, so smaller: {posed:?}");
            assert!(posed.shade > 0.0 && (posed.alpha - 1.0).abs() < 1e-6);
        }
        assert!(going.shift.x < 0.0 && coming.shift.x > 0.0);
        // Their near edges meet in the middle.
        let right_of_going = going.shift.x + 260.0 * going.scale.x;
        let left_of_coming = coming.shift.x - 260.0 * coming.scale.x;
        assert!((right_of_going - left_of_coming).abs() < 1.0);

        let early = moving(Panel::SignIn, Panel::Create, 0.2);
        assert!(pose(&early, Panel::SignIn, 520.0).over);
        assert!(!pose(&early, Panel::Create, 520.0).over);
        let late = moving(Panel::SignIn, Panel::Create, 0.8);
        assert!(!pose(&late, Panel::SignIn, 520.0).over);
        assert!(pose(&late, Panel::Create, 520.0).over);
    }

    #[test]
    fn a_panel_at_rest_or_outside_the_motion_is_not_moved() {
        let rest = FrontMotion::at_rest(Panel::SignIn);
        assert_eq!(pose(&rest, Panel::SignIn, 520.0), Pose::REST);
        let door = moving(Panel::Gateway, Panel::SignIn, 0.5);
        assert_eq!(pose(&door, Panel::Create, 520.0), Pose::REST);
    }
}
