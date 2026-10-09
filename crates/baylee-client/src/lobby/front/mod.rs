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

pub(crate) mod door;
pub(crate) mod faces;
pub(crate) mod keys;
pub(crate) mod terms;

use super::keyboard::choose_gateway;
use super::press::Cx;
use super::systems::keep_gateways;
#[allow(clippy::wildcard_imports)] // the lobby widget vocabulary
use super::*;
use crate::frontal::FrontalMaterial;
use crate::shellkit::controls::Kit;
use crate::shellkit::metrics::px_fixed;
use crate::shellkit::role::Role;
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

/// The card's width (§3, §2.7): 720 × the text step on a desktop (600
/// until the owner found the forms squeezed, 09.10.2026), the body's width
/// on a tablet (up to 960 × the step) and on the smallest window, and its
/// column beside the logo on a phone.
fn card_width(kit: Kit) -> (Val, Val) {
    match kit.m.frame {
        Frame::Compact | Frame::Phone => (Val::Percent(100.0), Val::Percent(100.0)),
        Frame::Narrow => (Val::Percent(100.0), kit.m.px(960.0)),
        Frame::Wide | Frame::Vast => (kit.m.px(CARD_WIDTH), Val::Percent(100.0)),
    }
}

/// The card's width on a desktop at the default step.
pub(super) const CARD_WIDTH: f32 = 720.0;

/// The four panels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(super) enum Panel {
    /// Choose a gateway.
    #[default]
    Gateway,
    /// Sign in there.
    SignIn,
    /// Create an account there.
    Create,
    /// Play there as a guest.
    Guest,
}

impl Panel {
    /// The panel the lobby asks for.
    fn of(state: &LobbyState) -> Self {
        if state.lobby.gateway_chosen() {
            match state.lobby.face() {
                baylee_client_core::lobby::Face::SignIn => Self::SignIn,
                baylee_client_core::lobby::Face::Create => Self::Create,
                baylee_client_core::lobby::Face::Guest => Self::Guest,
            }
        } else {
            Self::Gateway
        }
    }

    /// Each panel's own leather: a panel fades on its own, and the leather's
    /// density is a uniform of its material, which is one per slot.
    fn slot(self) -> u8 {
        match self {
            Self::Gateway => 1,
            Self::SignIn => 10,
            Self::Create => 11,
            Self::Guest => 12,
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

/// The primary action of the currently drawn account form.
#[derive(Component)]
pub(super) struct AccountSubmit;

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
    entrance: Res<super::entrance::Entrance>,
    mut scene: ResMut<crate::vista::FrontScene>,
) {
    let peak = if motion.to == Panel::Gateway {
        crate::vista::HAZE_OUT
    } else {
        crate::vista::HAZE_IN
    };
    scene.set_if_neq(crate::vista::FrontScene {
        stage: if entrance.active() {
            crate::vista::arrival(entrance.progress())
        } else {
            crate::vista::passage(motion.progress(), peak)
        },
        shown: matches!(state.lobby.screen(), Screen::SignIn { .. }) || entrance.active(),
        entering: entrance.active(),
        portal: entrance.progress(),
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
        // 0 with signing in up, 1 with the other face (creating an
        // account, playing as a guest) up.
        let round = if motion.to == Panel::SignIn {
            1.0 - eased
        } else {
            eased
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
                    Press::Front(FrontPress::SelectGateway(index)) if node.size().y > 0.0 => state
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
                commands.entity(node).try_insert(fresh.clone());
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

/// A fading button owns its material; its original alpha is retained so a
/// reversed passage restores it without multiplying yesterday's fade.
#[derive(Component)]
pub(super) struct FadedPrimary(f32);

/// Primary buttons normally share a material. Clone it only when a panel
/// fades, so its surface follows the text without fading another panel or
/// a stationary button that happens to share the original handle.
#[allow(clippy::type_complexity)]
pub(super) fn fade_primary_surfaces(
    mut commands: Commands,
    motion: Res<FrontMotion>,
    cards: Query<(Entity, &FrontCard)>,
    children: Query<&Children>,
    mut surfaces: Query<(
        Entity,
        &mut MaterialNode<crate::ambience::AmbienceMaterial>,
        Option<&FadedPrimary>,
    )>,
    materials: Option<ResMut<Assets<crate::ambience::AmbienceMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (card, panel) in &cards {
        let alpha = pose(&motion, panel.0, 0.0).alpha;
        for node in children.iter_descendants(card) {
            let Ok((entity, mut handle, faded)) = surfaces.get_mut(node) else {
                continue;
            };
            let base = if let Some(faded) = faded {
                faded.0
            } else {
                if alpha >= 1.0 {
                    continue;
                }
                let Some(material) = materials.get(&handle.0).cloned() else {
                    continue;
                };
                let base = material.params.low.w;
                handle.0 = materials.add(material);
                commands.entity(entity).try_insert(FadedPrimary(base));
                base
            };
            if let Some(mut material) = materials.get_mut(&handle.0) {
                material.params.low.w = base * alpha;
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
    kit: Kit,
    scrolled_to: &Scrolled,
) -> Entity {
    let (width, max_width) = card_width(kit);
    let room = commands
        .spawn((
            Node {
                width,
                max_width,
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
                width: Val::Percent(100.0),
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
        let card = card(commands, state, panel, kit, scrolled_to);
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
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(2.0),
            spread_radius: px_fixed(0.0),
            blur_radius: px_fixed(6.0),
        },
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.50),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(12.0),
            spread_radius: px_fixed(-2.0),
            blur_radius: px_fixed(30.0),
        },
    ])
}

/// One panel: the dock's leather and its inlays, the card's own shadow,
/// and the face it shows.
fn card(
    commands: &mut Commands,
    state: &LobbyState,
    panel: Panel,
    kit: Kit,
    scrolled_to: &Scrolled,
) -> Entity {
    let side = kit.m.pad * 1.2;
    let card = commands
        .spawn((
            FrontCard(panel),
            Role::Panel,
            Node {
                grid_row: GridPlacement::start(1),
                grid_column: GridPlacement::start(1),
                width: Val::Percent(100.0),
                align_self: AlignSelf::Center,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(if kit.m.frame == Frame::Phone {
                    8.0
                } else {
                    14.0
                }),
                // The foot's inlays stand clear of the tooled line, and the
                // last row stands clear of them.
                padding: UiRect {
                    left: px_fixed(side),
                    right: px_fixed(side),
                    top: px_fixed(side * 0.8),
                    bottom: px_fixed(side + super::dock::INLAY_LIFT),
                },
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(CARD_RADIUS)),
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
        Panel::Gateway => faces::gateway(commands, card, state, kit, scrolled_to),
        Panel::SignIn => faces::sign_in(commands, card, state, kit),
        Panel::Create => faces::create(commands, card, state, kit),
        Panel::Guest => faces::guest(commands, card, state, kit),
    }
    let shade = commands
        .spawn((
            FrontShade,
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(0.0),
                right: px_fixed(0.0),
                top: px_fixed(0.0),
                bottom: px_fixed(0.0),
                border_radius: BorderRadius::all(px_fixed(CARD_RADIUS - 1.0)),
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

/// The gear's menu, under the gear: the language, by each language's own
/// name, and the way to the whole settings screen.
///
/// A veil over the rest of the page closes it when pressed, as any press
/// outside a menu does.
pub(super) fn gear_menu(
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
                left: px_fixed(0.0),
                right: px_fixed(0.0),
                top: px_fixed(0.0),
                bottom: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let veil = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px_fixed(-4000.0),
                right: px_fixed(-4000.0),
                top: px_fixed(-4000.0),
                bottom: px_fixed(-4000.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            GlobalZIndex(590),
            Press::Front(FrontPress::FrontMenu),
        ))
        .id();
    let menu = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_TOP + HEADER_HEIGHT + 4.0),
                right: px(side - 6.0),
                width: px_fixed(240.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(metrics.gap),
                padding: UiRect::all(px(metrics.pad * 0.8)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT.with_alpha(1.0)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.45)),
            soft_shadow(),
            GlobalZIndex(600),
            // A press on the menu's own ground is not a press outside it.
            Press::Shared(SharedPress::PickerNothing),
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
            Press::Shared(SharedPress::PickLang(offered)),
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
    let music = crate::music::controls(commands, fonts, metrics, lang);
    let all = button(
        commands,
        fonts,
        metrics,
        Phrase::AllSettings.text(lang),
        Press::Settings(SettingsPress::OpenSettings),
        palette::PANEL,
        true,
    );
    commands.entity(all).entry::<Node>().and_modify(|mut node| {
        node.justify_content = JustifyContent::Center;
    });
    let report = crate::report::button(commands, fonts, metrics, lang);
    commands
        .entity(menu)
        .add_children(&[caption, languages, rule, music, report, all]);
    commands.entity(holder).add_children(&[veil, menu]);
    holder
}

/// A hairline across a panel.
fn rule(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                width: percent(100),
                height: px_fixed(1.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DOCK_EDGE.with_alpha(0.30)),
            Pickable::IGNORE,
        ))
        .id()
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
                node.flex_basis = px_fixed(0.0);
                node.flex_grow = 1.0;
                node.min_width = px_fixed(0.0);
                node.justify_content = JustifyContent::Center;
            });
        commands.entity(halves).add_child(control);
    }
    halves
}

/// The front door (§3): the logo, the tagline and the card, the text row
/// under it, the colophon at the foot, and the About sheet when it is up.
///
/// A phone sets the logo small at the left and the card at the right,
/// rising to the top so every field ends above the keyboard (§2.7); every
/// other class stacks them, centred.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the whole door, by size class
pub(super) fn front_door(
    commands: &mut Commands,
    root: Entity,
    state: &LobbyState,
    cast: &FrontCast,
    kit: Kit,
    scrolled_to: &Scrolled,
    assets: Option<&AssetServer>,
    music_on: bool,
    height: f32,
) {
    let phone = kit.m.frame == Frame::Phone;
    let page = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                min_height: px_fixed(0.0),
                flex_grow: 1.0,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::all(px_fixed(kit.m.body)),
                row_gap: px_fixed(kit.m.gap),
                ..default()
            },
            Scrollable(List::Table),
            ScrollPosition(Vec2::new(0.0, scrolled_to.get(List::Table))),
            Role::Scroll,
            // Empty space between fields must receive wheel/swipe gestures too.
            Pickable::default(),
        ))
        .id();
    commands
        .entity(root)
        .remove::<(Scrollable, ScrollPosition)>();
    commands
        .entity(root)
        .entry::<Node>()
        .and_modify(|mut node| node.overflow = Overflow::clip());
    // A page taller than the window scrolls, and a bar at its edge says so
    // (owner, 09.10.2026); one that fits shows none.
    let bar = super::scrollbars::attach(commands, root, page, faces::field_metrics(kit));
    commands
        .entity(bar)
        .insert(super::scrollbars::OnlyWhenNeeded)
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    let logo = assets.map_or_else(Handle::default, |a| a.load("brand/baylee-logo.png"));
    let logo_width = match kit.m.frame {
        Frame::Phone => 120.0,
        Frame::Compact | Frame::Narrow => 180.0,
        Frame::Wide | Frame::Vast => 300.0,
    };
    let brand = commands
        .spawn((
            ImageNode::new(logo),
            Node {
                width: px_fixed(logo_width),
                max_width: Val::Percent(100.0),
                // Never more than a sixth of a short window's height: the
                // card and the notices come first.
                max_height: Val::Vh(if phone { 30.0 } else { 15.0 }),
                aspect_ratio: Some(1942.0 / 809.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let stage = stage(commands, state, cast, kit, scrolled_to);
    let text_row = door::text_row(commands, state, kit, music_on);
    let version = door::version_line(commands, state, kit);
    if phone {
        // Two panes, width the plentiful axis: the logo at the left, the
        // card at the right, the text row under the logo.
        let panes = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    column_gap: px_fixed(kit.m.gap),
                    align_items: AlignItems::FlexStart,
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let left = commands
            .spawn((
                Node {
                    width: px_fixed(logo_width + 8.0),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: px_fixed(kit.m.gap),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(left)
            .add_children(&[brand, text_row, version]);
        commands
            .entity(stage)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.flex_grow = 1.0;
                node.flex_shrink = 1.0;
                node.min_width = px_fixed(0.0);
            });
        commands.entity(panes).add_children(&[left, stage]);
        commands.entity(page).add_child(panes);
    } else {
        let tagline = door::tagline(commands, state, kit);
        // Auto margins consume spare height, but collapse to zero when the
        // form needs to scroll; `justify-content: center` would hide its top
        // on overflow.
        let composition = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    row_gap: px_fixed(kit.m.gap),
                    margin: UiRect::vertical(if kit.m.frame == Frame::Compact {
                        px_fixed(0.0)
                    } else {
                        Val::Auto
                    }),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands
            .entity(composition)
            .add_children(&[brand, tagline, stage, text_row, version]);
        commands.entity(page).add_child(composition);
    }
    let colophon = if door::full_colophon(kit, height) {
        // The source offer stands in the top corner, its QR with it.
        let corner = door::source_corner(commands, state, kit);
        commands.entity(root).add_child(corner);
        door::full(commands, state, kit)
    } else {
        let line = door::one_line(commands, state, kit);
        let holder = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::axes(px_fixed(kit.m.body), kit.m.px(4.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(line);
        holder
    };
    commands.entity(root).add_child(colophon);
}

/// A control of the front door: the saved gateways, the account form,
/// guests, offline play and the source link.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FrontPress {
    AddGateway,
    SelectGateway(usize),
    /// Asks, in the confirm dialog, whether a saved gateway leaves the list.
    ForgetGateway(usize),
    /// Back from the account form to the gateway form.
    LeaveGateway,
    /// Opens or closes the front door's gear menu.
    FrontMenu,
    /// Swap the form between log-in and sign-up.
    ToggleRegistering,
    /// Send the sign-in form.
    Submit,
    /// Play the house AI in this process, no account needed.
    PlayOffline,
    /// Open the source address in the browser (#299).
    OpenSource,
    /// Nothing (a control drawn while the form is busy).
    Nothing,
    /// `‹ Back` on the create-account and guest faces.
    BackToSignIn,
    /// The sign-in face's Play as guest: the guest's face, or a kept
    /// guest at once.
    GuestFace,
    /// Ask an unreachable gateway again.
    RetryGateway,
    /// Open (`true`) or close the About sheet.
    About(bool),
    /// The terms sheet's Accept and continue.
    TermsAccept,
    /// The terms sheet's Not now (a guest's second press signs it out).
    TermsNotNow,
    /// A guest stays on the terms sheet.
    TermsStay,
    /// Ask for the terms again after a failure.
    TermsRetry,
    /// The terms sheet's Decline and delete account: the account deletion's
    /// own confirmation (#292), over the sheet.
    TermsDecline,
}

impl FrontPress {
    /// What a click on this control does.
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            FrontPress::AddGateway => {
                if let Some(url) = state.check_gateway() {
                    http::probe_gateway(url, mailbox);
                }
            }
            FrontPress::SelectGateway(index) => choose_gateway(state, prefs, mailbox, index),
            FrontPress::ForgetGateway(index) => {
                if let Some(url) = state.gateways.get(index) {
                    state.confirmation = Some(confirm::Destructive::ForgetGateway(url.clone()));
                }
            }
            FrontPress::LeaveGateway => state.leave_gateway(),
            FrontPress::FrontMenu => state.front_menu = !state.front_menu,
            FrontPress::Nothing => {}
            FrontPress::BackToSignIn => {
                state.lobby.back_to_sign_in();
            }
            FrontPress::GuestFace => {
                let request = state.lobby.open_guest_face();
                if state.lobby.guest() {
                    let gateway = state.gateway.clone();
                    state.uses.record(&gateway);
                    if let Some(settings) = settings.as_mut() {
                        keep_gateways(state, settings);
                    }
                }
                dispatch(state, mailbox, request);
            }
            FrontPress::RetryGateway => {
                let url = state.gateway.clone();
                state.probes.insert(url.clone(), Probe::Asking);
                http::probe_gateway(url, mailbox);
                http::probe_registration(state, mailbox);
            }
            FrontPress::About(open) => {
                if state.about_open != open {
                    state.about_open = open;
                    scrolled.set(List::About, 0.0);
                }
            }
            FrontPress::TermsAccept => {
                if let Some(ask) = state.terms.accept() {
                    terms::perform(ask, state, prefs, scrolled, mailbox, settings);
                }
            }
            FrontPress::TermsNotNow => {
                let guest = state.lobby.guest();
                if let Some(ask) = state.terms.not_now(guest) {
                    terms::perform(ask, state, prefs, scrolled, mailbox, settings);
                }
            }
            FrontPress::TermsStay => state.terms.stay(),
            // The same confirmation, password and request as Settings'
            // Delete account: declining the terms deletes nothing else.
            FrontPress::TermsDecline => state.lobby.ask_to_delete_account(),
            FrontPress::TermsRetry => {
                if let Some(ask) = state.terms.retry() {
                    terms::perform(ask, state, prefs, scrolled, mailbox, settings);
                }
            }
            FrontPress::OpenSource => super::source::open(state),
            FrontPress::ToggleRegistering => state.lobby.toggle_registering(),
            FrontPress::Submit => {
                let request = state.lobby.submit();
                dispatch(state, mailbox, request);
            }
            // Not a duel any more. Offline is the lobby with a different
            // performer behind it, so this opens the *table screen* with the
            // player's own decks in it and a room to arrange. What the button
            // skips is still the sign-in; what it no longer skips is choosing
            // who you are playing and with what.
            FrontPress::PlayOffline => {
                state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
                prefs.detach();
                scrolled.set(List::Table, 0.0);
                // Whatever is already here is kept. `HubPress::SignOut` is the only
                // thing that clears it, so a player coming back to offline
                // play finds the decks and the room they left.
                state
                    .offline
                    .get_or_insert_with(super::offline::Offline::load);
                let request = state.lobby.play_offline();
                dispatch(state, mailbox, request);
            }
        }
    }
}

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
    fn the_notice_is_the_policy_s_own_words() {
        // Every word and punctuation mark stays pinned. Sentence line breaks
        // only format the quoted notice; they may not rewrite its wording.
        assert_eq!(
            FAN_CONTENT_NOTICE
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
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
    fn button_surfaces_fade_independently_and_recover_on_reversal() {
        use crate::ambience::{AmbienceMaterial, AmbienceParams};
        let mut app = App::new();
        app.init_resource::<Assets<AmbienceMaterial>>()
            .insert_resource(moving(Panel::Gateway, Panel::SignIn, 0.0))
            .add_systems(Update, fade_primary_surfaces);
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AmbienceMaterial>>()
            .add(AmbienceMaterial {
                params: AmbienceParams {
                    low: Vec4::new(0.1, 0.2, 0.3, 0.8),
                    high: Vec4::ONE,
                    energy: 1.0,
                    seed: 0.0,
                    aspect: 3.0,
                    pad: 1.0,
                },
            });
        let mut panels = Vec::new();
        for panel in [Panel::Gateway, Panel::SignIn] {
            let parent = app.world_mut().spawn(FrontCard(panel)).id();
            let surface = app
                .world_mut()
                .spawn((MaterialNode(handle.clone()), ChildOf(parent)))
                .id();
            panels.push((panel, surface));
        }
        // Repeated frames, including a reversal: clones must neither compound
        // the fade nor allocate another material each frame.
        for t in [0.0, 0.35, 0.55, 0.8, 0.55, 0.35, 0.0] {
            *app.world_mut().resource_mut::<FrontMotion>() =
                moving(Panel::Gateway, Panel::SignIn, t);
            app.update();
            let materials = app.world().resource::<Assets<AmbienceMaterial>>();
            for &(panel, entity) in &panels {
                let actual = app
                    .world()
                    .get::<MaterialNode<AmbienceMaterial>>(entity)
                    .unwrap();
                let alpha = materials.get(&actual.0).unwrap().params.low.w;
                let expected = 0.8 * pose(app.world().resource::<FrontMotion>(), panel, 0.0).alpha;
                assert!(
                    (alpha - expected).abs() < 1e-6,
                    "{panel:?} at {t}: {alpha} != {expected}"
                );
            }
            assert!((materials.get(&handle).unwrap().params.low.w - 0.8).abs() < 1e-6);
            assert!(materials.len() <= 3, "only one clone per fading surface");
        }
    }

    #[test]
    fn a_panel_at_rest_or_outside_the_motion_is_not_moved() {
        let rest = FrontMotion::at_rest(Panel::SignIn);
        assert_eq!(pose(&rest, Panel::SignIn, 520.0), Pose::REST);
        let door = moving(Panel::Gateway, Panel::SignIn, 0.5);
        assert_eq!(pose(&door, Panel::Create, 520.0), Pose::REST);
    }
}
