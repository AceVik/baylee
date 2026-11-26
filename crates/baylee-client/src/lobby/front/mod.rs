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

pub(crate) mod about;
pub(crate) mod donate;
pub(crate) mod door;
pub(crate) mod faces;
pub(crate) mod keys;
pub(crate) mod links;
pub(crate) mod progress;
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
pub(crate) const FAN_CONTENT_NOTICE: &str = "Baylee is unofficial Fan Content permitted under the Fan Content Policy. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.";

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

/// Which community or web destination a front-door pill opens in the QR modal.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CommunityKind {
    WhatsApp,
    Discord,
    GitHub,
    Website,
}

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
    /// Choose a new password there, after an admin reset it (beta.8).
    NewPassword,
}

impl Panel {
    /// The panel the lobby asks for.
    fn of(state: &LobbyState) -> Self {
        if keys::on_the_list(state) {
            return Self::Gateway;
        }
        if state.lobby.gateway_chosen() {
            match state.lobby.face() {
                baylee_client_core::lobby::Face::SignIn => Self::SignIn,
                baylee_client_core::lobby::Face::Create => Self::Create,
                baylee_client_core::lobby::Face::Guest => Self::Guest,
                baylee_client_core::lobby::Face::NewPassword => Self::NewPassword,
            }
        } else {
            Self::Gateway
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self::at_rest(Panel::SignIn)
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self::at_rest(Panel::Gateway)
        }
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

/// The stage the panels stand on, for [`glide_the_stage`].
#[derive(Component)]
pub(super) struct FrontStage;

/// How long a same-panel height change (entering/leaving the authentication or
/// queue view, or revealing a status banner) glides along [`ease`].
const SAME_PANEL_GLIDE_SECONDS: f32 = 0.34;

/// How tall the stage stood when nothing moved, and any active same-panel
/// S-curve height glide.
#[derive(Resource, Default, Clone, Copy, PartialEq, Debug)]
pub(super) struct StageAtRest {
    panel: Panel,
    height: f32,
    glide_from: f32,
    glide_to: f32,
    glide_t: f32,
}

/// A panel on screen, for [`pose_front`] and [`fade_front`].
#[derive(Component)]
pub(super) struct FrontCard(pub(super) Panel);

/// Natural-height content column inside [`FrontCard`], kept `flex_shrink: 0.0`
/// so [`glide_the_stage`] can measure the panel's true target height while the
/// outer glass card and stage smoothly glide between heights.
#[derive(Component)]
pub(super) struct FrontCardInner(pub(super) Panel);

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
    let on_front =
        matches!(state.lobby.screen(), Screen::SignIn { .. }) || state.showing_gateway_hub();
    let still = prefs.all().reduce_motion || !on_front;
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
    let unauthenticated = matches!(state.lobby.screen(), Screen::SignIn { .. })
        && !state.central_ready()
        && !state.lobby.offline();
    scene.set_if_neq(crate::vista::FrontScene {
        stage: if entrance.active() {
            crate::vista::arrival(entrance.progress())
        } else {
            crate::vista::passage(motion.progress(), peak)
        },
        shown: unauthenticated || entrance.active(),
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

/// The stage's height in one frame of a motion: from the panel going to the
/// panel coming, on the motion's own curve.
fn stage_height(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * ease(t)
}

#[allow(clippy::type_complexity)]
fn measure_panel_height(
    wanted: Panel,
    inners: &Query<(&FrontCardInner, &ComputedNode)>,
    cards: &Query<(&FrontCard, &mut Node, &ComputedNode), Without<FrontStage>>,
) -> Option<f32> {
    let logical_h = |node: &ComputedNode| node.size().y * node.inverse_scale_factor();
    inners
        .iter()
        .find(|(inner, _)| inner.0 == wanted)
        .map(|(_, node)| logical_h(node))
        .filter(|h| *h > 0.0)
        .map(|h| h + 2.0)
        .or_else(|| {
            cards
                .iter()
                .find(|(card, _, _)| card.0 == wanted)
                .map(|(_, _, node)| logical_h(node))
                .filter(|h| *h > 0.0)
        })
}

#[allow(clippy::type_complexity)]
fn settle_stage_and_cards(
    panel: Panel,
    settled_h: f32,
    rest: &mut ResMut<StageAtRest>,
    stage: &mut Node,
    cards: &mut Query<(&FrontCard, &mut Node, &ComputedNode), Without<FrontStage>>,
) {
    rest.set_if_neq(StageAtRest {
        panel,
        height: settled_h,
        glide_from: settled_h,
        glide_to: settled_h,
        glide_t: 1.0,
    });
    if stage.height != Val::Auto {
        stage.height = Val::Auto;
        stage.grid_template_rows = vec![GridTrack::auto()];
    }
    for (_, mut node, _) in cards {
        if node.height != Val::Auto {
            node.height = Val::Auto;
        }
    }
}

/// Eases the stage and active card smoothly whenever height changes — both
/// between two panels (`motion.moving()`) and within the same panel (entering
/// or leaving the authentication/queue view, or showing/clearing a status banner).
///
/// The logo stands above the stage and the four community pills stand below it,
/// so holding `FrontStage` and `FrontCard` to an eased height makes the entire
/// composition glide together without a single-frame jump.
#[allow(clippy::type_complexity)]
pub(super) fn glide_the_stage(
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    motion: Res<FrontMotion>,
    mut rest: ResMut<StageAtRest>,
    inners: Query<(&FrontCardInner, &ComputedNode)>,
    mut cards: Query<(&FrontCard, &mut Node, &ComputedNode), Without<FrontStage>>,
    mut stages: Query<(&mut Node, &ComputedNode), With<FrontStage>>,
) {
    let Ok((mut stage, stage_computed)) = stages.single_mut() else {
        return;
    };
    let still = prefs.is_some_and(|p| p.all().reduce_motion);

    if !motion.moving() {
        let Some(target_h) = measure_panel_height(motion.to, &inners, &cards) else {
            // Freshly rebuilt in `Update` before `PostUpdate` layout has run:
            // hold the previous height on Frame 1 so layout never steps once.
            if !still && rest.panel == motion.to && rest.height > 0.0 {
                let held = px_fixed(rest.height);
                if stage.height != held {
                    stage.height = held;
                    stage.grid_template_rows = vec![GridTrack::px(rest.height)];
                }
                for (card, mut node, _) in &mut cards {
                    if card.0 == motion.to && node.height != held {
                        node.height = held;
                    }
                }
            }
            return;
        };

        let raw_dt = time.delta_secs();
        let dt = if raw_dt.is_finite() {
            raw_dt.clamp(0.0, 1.0 / 30.0)
        } else {
            0.0
        };
        let settled_h = if stage_computed.size().y > 0.0 && stage.height == Val::Auto {
            stage_computed.size().y * stage_computed.inverse_scale_factor()
        } else {
            target_h
        };

        if still || rest.panel != motion.to || rest.height <= 0.0 {
            settle_stage_and_cards(motion.to, settled_h, &mut rest, &mut stage, &mut cards);
            return;
        }
        if (rest.glide_to - target_h).abs() > 0.5 {
            rest.glide_from = rest.height;
            rest.glide_to = target_h;
            rest.glide_t = 0.0;
        }
        if rest.glide_t >= 1.0 || (rest.height - target_h).abs() <= 0.5 {
            settle_stage_and_cards(motion.to, settled_h, &mut rest, &mut stage, &mut cards);
            return;
        }

        rest.glide_t = (rest.glide_t + dt / SAME_PANEL_GLIDE_SECONDS).min(1.0);
        let next_h = stage_height(rest.glide_from, rest.glide_to, rest.glide_t);
        rest.height = next_h;
        let px_h = px_fixed(next_h);
        if stage.height != px_h {
            stage.height = px_h;
            stage.grid_template_rows = vec![GridTrack::px(next_h)];
        }
        for (card, mut node, _) in &mut cards {
            if card.0 == motion.to && node.height != px_h {
                node.height = px_h;
            }
        }
        return;
    }

    for (_, mut node, _) in &mut cards {
        if node.height != Val::Auto {
            node.height = Val::Auto;
        }
    }
    // The panel it stood on keeps that height, either way round, so a motion
    // turned back on itself runs back along the same numbers. The other is
    // measured the frame after it was drawn: until then the stage stands.
    let panel = |wanted: Panel| {
        if wanted == rest.panel && rest.height > 0.0 {
            return Some(rest.height);
        }
        measure_panel_height(wanted, &inners, &cards)
    };
    let from = panel(motion.from).unwrap_or(rest.height);
    let wanted = stage_height(from, panel(motion.to).unwrap_or(from), motion.t);
    if wanted > 0.0 && stage.height != px_fixed(wanted) {
        stage.height = px_fixed(wanted);
        stage.grid_template_rows = vec![GridTrack::px(wanted)];
    }
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
            let gateways = state.displayed_gateways();
            let found: Vec<(String, Vec2)> = rows
                .iter()
                .filter_map(|(press, node, at)| match press {
                    Press::Front(FrontPress::SelectGateway(index)) if node.size().y > 0.0 => {
                        gateways
                            .get(*index)
                            .map(|url| (url.clone(), (at.translation - centre) * scale))
                    }
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
    gradient: Option<BackgroundGradient>,
    border: Option<BorderColor>,
    ink: Option<Color>,
    shadow: Option<Vec<Color>>,
}

fn scale_gradient_alpha(base: &BackgroundGradient, alpha: f32) -> BackgroundGradient {
    use bevy::ui::Gradient;
    let mut scaled = base.clone();
    for g in &mut scaled.0 {
        match g {
            Gradient::Linear(l) => {
                for stop in &mut l.stops {
                    stop.color = stop.color.with_alpha(stop.color.alpha() * alpha);
                }
            }
            Gradient::Radial(r) => {
                for stop in &mut r.stops {
                    stop.color = stop.color.with_alpha(stop.color.alpha() * alpha);
                }
            }
            Gradient::Conic(c) => {
                for stop in &mut c.stops {
                    stop.color = stop.color.with_alpha(stop.color.alpha() * alpha);
                }
            }
        }
    }
    scaled
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
        Option<&mut BackgroundGradient>,
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
            let Ok((fill, gradient, border, ink, shadow, unfaded)) = nodes.get_mut(node) else {
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
                    gradient: gradient.as_deref().cloned(),
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
            if let (Some(mut gradient), Some(base)) = (gradient, unfaded.gradient.as_ref()) {
                gradient.set_if_neq(scale_gradient_alpha(base, alpha));
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
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
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
    sheens: Query<&MaterialNode<crate::shellkit::sheen::SheenMaterial>>,
    mut sheen_materials: Option<ResMut<Assets<crate::shellkit::sheen::SheenMaterial>>>,
) {
    let mut materials = materials;
    for (card, panel) in &cards {
        let alpha = pose(&motion, panel.0, 0.0).alpha;
        for node in children.iter_descendants(card) {
            if let Some(sheen_materials) = sheen_materials.as_mut()
                && let Ok(sheen) = sheens.get(node)
            {
                let want = crate::shellkit::sheen::GLOW * alpha;
                if sheen_materials
                    .get(&sheen.0)
                    .is_some_and(|m| (m.params.glow - want).abs() > 1e-5)
                    && let Some(mut material) = sheen_materials.get_mut(&sheen.0)
                {
                    material.params.glow = want;
                    if alpha < 1.0 {
                        material.params.hover = 0.0;
                    }
                }
            }
            let Some(materials) = materials.as_mut() else {
                continue;
            };
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
    let curved = 1.0 - (1.0 - alpha).powi(3);
    let density = super::dock::GROUND_DENSITY * curved;
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
    music_on: bool,
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
            FrontStage,
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
        let card = card(commands, state, panel, kit, scrolled_to, music_on);
        commands.entity(stage).add_child(card);
    }
    room
}

/// The card's elevation: deep multi-layer contact, key, and ambient halo
/// drop shadow (`surfaces::elevation_card`), carried wherever the panel goes.
pub(super) fn card_shadow() -> BoxShadow {
    crate::shellkit::surfaces::elevation_card()
}

/// Translucent 3-stop smoked cut-glass gradient for front-door form cards.
///
/// Paired with `BackgroundColor(Color::NONE)` so Bevy 0.20 renders a single
/// translucent glass quad over the animated world vista instead of stacking two.
pub(crate) fn glass_card_gradient() -> BackgroundGradient {
    use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};
    BackgroundGradient::from(
        LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.080, 0.130, 0.200, 0.60), 0.0),
            ColorStop::percent(Color::srgba(0.035, 0.068, 0.115, 0.54), 48.0),
            ColorStop::percent(Color::srgba(0.025, 0.050, 0.088, 0.64), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba),
    )
}

/// Crisp directional specular cut-glass rim for front-door form cards.
pub(crate) const fn glass_card_border() -> BorderColor {
    BorderColor {
        top: Color::srgba(0.86, 0.78, 0.62, 0.44),
        right: Color::srgba(0.58, 0.66, 0.76, 0.30),
        bottom: Color::srgba(0.46, 0.42, 0.34, 0.34),
        left: Color::srgba(0.58, 0.66, 0.76, 0.30),
    }
}

/// One panel: smoked cut-glass optics, multi-layer elevation shadow,
/// and the face it shows.
fn card(
    commands: &mut Commands,
    state: &LobbyState,
    panel: Panel,
    kit: Kit,
    scrolled_to: &Scrolled,
    music_on: bool,
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
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(CARD_RADIUS)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            glass_card_gradient(),
            glass_card_border(),
            card_shadow(),
            UiTransform::default(),
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .id();
    let inner = commands
        .spawn((
            FrontCardInner(panel),
            Node {
                width: Val::Percent(100.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(if panel == Panel::Gateway {
                    6.0
                } else if kit.m.frame == Frame::Phone {
                    8.0
                } else {
                    14.0
                }),
                padding: UiRect {
                    left: px_fixed(side),
                    right: px_fixed(side),
                    top: px_fixed(side * if panel == Panel::Gateway { 0.72 } else { 0.90 }),
                    bottom: px_fixed(side * if panel == Panel::Gateway { 0.76 } else { 0.94 }),
                },
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_child(inner);
    if panel != Panel::Gateway && progress::is_active(state) {
        progress::draw(commands, inner, state, kit);
    } else {
        match panel {
            Panel::Gateway => faces::gateway(commands, inner, state, kit, scrolled_to, music_on),
            Panel::SignIn => faces::sign_in(commands, inner, state, kit, music_on),
            Panel::Create => faces::create(commands, inner, state, kit, music_on),
            Panel::Guest => faces::guest(commands, inner, state, kit, music_on),
            Panel::NewPassword => faces::new_password(commands, inner, state, kit, music_on),
        }
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
#[allow(clippy::too_many_lines)] // header + language pills + audio card + primary settings action
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
            GlobalZIndex(crate::shellkit::tokens::z::POPOVER - 2),
            Press::Front(FrontPress::FrontMenu),
        ))
        .id();
    let menu = commands
        .spawn((
            crate::shellkit::role::Role::Opaque,
            Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_TOP + HEADER_HEIGHT + 6.0),
                right: px(side - 6.0),
                width: px_fixed(312.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(12.0),
                padding: UiRect::all(px_fixed(16.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(crate::shellkit::tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(crate::shellkit::tokens::OPAQUE),
            BorderColor::all(crate::shellkit::tokens::POPOVER_EDGE),
            crate::shellkit::surfaces::elevation_card(),
            UiTransform::default(),
            GlobalZIndex(600),
            // A press on the menu's own ground is not a press outside it.
            Press::Shared(SharedPress::PickerNothing),
        ))
        .id();
    let nipple = super::header::spawn_popover_nipple(commands);
    commands.entity(menu).insert((
        super::header::HeaderPopover {
            trigger: crate::shellkit::header::HeaderTrigger::Gear,
            nipple,
        },
        Visibility::Hidden,
    ));
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::bottom(px_fixed(4.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                ..default()
            },
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.35)),
            Pickable::IGNORE,
        ))
        .id();
    let head_left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: px_fixed(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let head_badge = commands
        .spawn((
            Node {
                width: px_fixed(28.0),
                height: px_fixed(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                ..default()
            },
            BackgroundColor(crate::shellkit::tokens::ACCENT.with_alpha(0.14)),
            BorderColor::all(crate::shellkit::tokens::ACCENT.with_alpha(0.38)),
            Pickable::IGNORE,
        ))
        .id();
    let head_icon = commands
        .spawn((
            Text::new("\u{f013}"),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(crate::shellkit::tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(head_badge).add_child(head_icon);
    let head_title = commands
        .spawn((
            Text::new(Phrase::SettingsQuickTitle.text(lang)),
            crate::hud::tf_bold(fonts, metrics.text),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(head_left)
        .add_children(&[head_badge, head_title]);
    let close_btn = commands
        .spawn((
            Role::Button,
            Node {
                width: px_fixed(28.0),
                height: px_fixed(28.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.04)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.35)),
            crate::ambience::Feel::tinting_to(
                Color::srgba(1.0, 1.0, 1.0, 0.04),
                crate::shellkit::tokens::HOVER,
            ),
            UiTransform::default(),
            Press::Front(FrontPress::FrontMenu),
        ))
        .id();
    let close_icon = commands
        .spawn((
            Text::new("\u{f00d}"),
            crate::hud::icon_tf(fonts, metrics.small),
            TextColor(palette::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(close_btn).add_child(close_icon);
    commands.entity(head).add_children(&[head_left, close_btn]);

    let lang_box = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(10.0)),
                row_gap: px_fixed(8.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.06, 0.10, 0.72)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.32)),
            Pickable::IGNORE,
        ))
        .id();
    let caption = note(commands, fonts, metrics, Phrase::Language.text(lang));
    let mut chips = Vec::new();
    for offered in Lang::ALL {
        let active = offered == lang;
        let bg = if active {
            Color::srgba(0.09, 0.22, 0.26, 0.92)
        } else {
            Color::srgba(0.06, 0.10, 0.16, 0.88)
        };
        let chip = button(
            commands,
            fonts,
            metrics,
            offered.name(),
            Press::Shared(SharedPress::PickLang(offered)),
            bg,
            true,
        );
        let flag = door::flag_icon(commands, offered);
        commands
            .entity(chip)
            .insert(crate::shellkit::surfaces::elevation_control())
            .insert_children(0, &[flag]);
        if active {
            commands.entity(chip).insert((
                BackgroundColor(bg),
                BorderColor::all(crate::shellkit::tokens::ACCENT.with_alpha(0.65)),
                crate::ambience::Feel::tinting_to(bg, crate::shellkit::tokens::HOVER),
            ));
        }
        chips.push(chip);
    }
    let languages = halves(commands, metrics, &chips);
    commands
        .entity(lang_box)
        .add_children(&[caption, languages]);

    let audio_box = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px_fixed(10.0)),
                row_gap: px_fixed(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.06, 0.10, 0.72)),
            BorderColor::all(palette::DOCK_EDGE.with_alpha(0.32)),
            Pickable::IGNORE,
        ))
        .id();
    let music = crate::music::controls(commands, fonts, metrics, lang);
    commands.entity(audio_box).add_child(music);

    let actions_col = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px_fixed(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let all = button(
        commands,
        fonts,
        metrics,
        Phrase::AllSettings.text(lang),
        Press::Settings(SettingsPress::OpenSettings),
        crate::shellkit::tokens::PRIMARY,
        true,
    );
    commands.entity(all).insert((
        BorderColor::all(crate::shellkit::tokens::PRIMARY_EDGE),
        crate::shellkit::surfaces::elevation_control(),
        crate::ambience::Feel::new(crate::shellkit::tokens::PRIMARY),
        crate::ambience::FeelShadow,
        crate::shellkit::sheen::Sheen,
    ));
    commands.entity(all).entry::<Node>().and_modify(|mut node| {
        node.justify_content = JustifyContent::Center;
    });
    let report = crate::report::button(commands, fonts, metrics, lang);
    commands.entity(actions_col).add_children(&[all, report]);
    commands
        .entity(menu)
        .add_children(&[nipple, head, lang_box, audio_box, actions_col]);
    commands.entity(holder).add_children(&[veil, menu]);
    holder
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

/// The chosen gateway's legal notice as a link: its address and the host
/// to show beside it, or `None` when the gateway has not answered or names
/// none (`/info` `impressum`, checked by `legal_address`).
pub(crate) fn impressum_link(state: &LobbyState) -> Option<(String, String)> {
    match state.probes.get(&state.gateway) {
        Some(Probe::Known(info)) => baylee_client_core::lobby::gateway_info::impressum_link(info),
        _ => None,
    }
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
    cloud: Option<&super::memorial::CloudArt>,
    music_on: bool,
    height: f32,
) {
    let phone = kit.m.frame == Frame::Phone;
    let full = door::full_colophon(kit, height);
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
                padding: UiRect::axes(
                    px_fixed(kit.m.body),
                    px_fixed(if full { kit.m.body } else { 4.0 }),
                ),
                row_gap: px_fixed(0.0),
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
    let _ = cloud;
    let logo = assets.map_or_else(Handle::default, |a| a.load("brand/baylee-logo.png"));
    let logo_width = match kit.m.frame {
        Frame::Phone => 185.0,
        Frame::Compact | Frame::Narrow => 350.0,
        Frame::Wide => 520.0,
        Frame::Vast => 580.0,
    };
    let brand = door::front_logo(commands, state, logo, logo_width, phone, full);
    let stage = stage(commands, state, cast, kit, scrolled_to, music_on);
    let community = door::community_row(commands, state, kit, full);
    let version = door::version_line(commands, state, kit);
    let info = door::info_line(commands, state, kit);
    let in_lobby = state.central_ready();
    if phone {
        // Two panes, width the plentiful axis: the logo at the left, the
        // card at the right, the community links under the logo.
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
        if in_lobby {
            commands.entity(brand).despawn();
            commands
                .entity(left)
                .add_children(&[community, version, info]);
        } else {
            let controls = door::header_controls(commands, state, kit, music_on, false);
            commands
                .entity(left)
                .add_children(&[brand, controls, community, version, info]);
        }
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
        // Flexible top and bottom spacers position the front-door stack (logo,
        // tagline, sign-in card, and the four community pills) ~116 px below
        // geometric centre when vertical room permits (+35 px + +100 px), while
        // shrinking smoothly down to zero before ever triggering a scrollbar.
        let compact = kit.m.frame == Frame::Compact;
        let top_basis = if in_lobby {
            0.0
        } else if compact {
            135.0
        } else {
            232.0
        };
        let top_spacer = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_basis: px_fixed(top_basis),
                    flex_grow: if compact { 0.0 } else { 1.0 },
                    flex_shrink: 1.0,
                    min_height: px_fixed(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let bottom_spacer = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_basis: px_fixed(if in_lobby { 12.0 } else { 0.0 }),
                    flex_grow: if compact { 0.0 } else { 1.0 },
                    flex_shrink: 1.0,
                    min_height: px_fixed(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let composition = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    row_gap: px_fixed(kit.m.gap * if full { 0.72 } else { 0.4 }),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        if in_lobby {
            commands.entity(brand).despawn();
            commands
                .entity(composition)
                .add_children(&[stage, community]);
        } else {
            let tagline = door::tagline(commands, state, kit);
            commands
                .entity(composition)
                .add_children(&[brand, tagline, stage, community]);
            let top_right = door::top_right_controls(commands, state, kit, music_on);
            commands.entity(root).add_child(top_right);
            if donate::showing(state) {
                let corner = donate::corner(commands, state, kit);
                commands.entity(root).add_child(corner);
            }
        }
        commands
            .entity(page)
            .add_children(&[top_spacer, composition, bottom_spacer]);
        commands.entity(root).add_children(&[version, info]);
    }
    let colophon = if full {
        door::full(commands, state, kit)
    } else {
        let line = door::one_line(commands, state, kit);
        let holder = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    padding: UiRect {
                        left: px_fixed(kit.m.body),
                        right: px_fixed(kit.m.body),
                        top: px_fixed(2.0),
                        bottom: px_fixed(if phone { 4.0 } else { door::FOOTER_BOTTOM }),
                    },
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(holder).add_child(line);
        holder
    };
    commands.entity(root).add_child(colophon);
    if state.front_menu {
        let veil = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px_fixed(0.0),
                    top: px_fixed(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                GlobalZIndex(crate::shellkit::tokens::z::POPOVER - 2),
                Press::Front(FrontPress::FrontMenu),
            ))
            .id();
        commands.entity(root).add_child(veil);
    }
    door::community_dialog(commands, root, state, kit);
    donate::sheet(commands, root, state, kit);
}

/// A control of the front door: the saved gateways, the account form,
/// guests, offline play and the source link.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FrontPress {
    #[cfg(not(target_arch = "wasm32"))]
    CentralSignOut,
    /// Past Central's sign-in to the device's own gateways, no account needed.
    WithoutAccount,
    /// From the device's own gateways back to Central's sign-in.
    BackToCentral,
    AddGateway,
    SelectGateway(usize),
    /// Asks, in the confirm dialog, whether a saved gateway leaves the list.
    ForgetGateway(usize),
    /// Back from the account form to the gateway form.
    LeaveGateway,
    /// Opens or closes the front door's gear menu.
    FrontMenu,
    /// Opens the QR-code modal dialog for one of the four community/web pills.
    OpenCommunityDialog(CommunityKind),
    /// Closes the QR-code modal dialog.
    CloseCommunityDialog,
    /// Opens the donation sheet (the button at the top left).
    Donate,
    /// Closes the donation sheet.
    CloseDonate,
    /// Opens one of the places to give in the browser.
    OpenGiving(baylee_client_core::funding::Platform),
    /// Swap the form between log-in and sign-up.
    ToggleRegistering,
    /// Send the sign-in form.
    Submit,
    /// Play the house AI in this process, no account needed.
    PlayOffline,
    /// Open the source address in the browser (#299).
    OpenSource,
    /// Open the community's Discord invitation in the browser.
    OpenDiscord,
    /// Open the community's `WhatsApp` invitation in the browser.
    OpenWhatsApp,
    /// Open the website's registration page, from Central's sign-in form.
    OpenCentralRegister,
    /// Open `https://baylee.app` in the browser, from Central's sign-in form.
    OpenWebsite,
    /// Open the GitHub releases changelog in the browser.
    OpenChangelog,
    /// Open one of the verified HTTPS reference links in the browser.
    OpenExternal(super::source::ExternalLink),
    /// Open the gateway's legal notice (Impressum, `/info` `impressum`) in
    /// the browser.
    OpenImpressum,
    /// Fold or unfold one of the three gateway compatibility groups.
    ToggleGatewayGroup(baylee_client_core::lobby::gateway_list::GatewayGroup),
    /// Step through the gateway sort orders (`LastUsed`, `Name`, `Address`, `Version`).
    CycleGatewaySort,
    /// Clear the gateway search box.
    ClearGatewaySearch,
    /// Re-probe every gateway in the list.
    RefreshGateways,
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
    /// Ask for the terms again after a failure.
    TermsRetry,
    /// The terms sheet's Decline and delete account: the account deletion's
    /// own confirmation (#292), over the sheet.
    TermsDecline,
    /// The terms sheet's Later: signs out, nothing accepted.
    TermsLater,
}

impl FrontPress {
    /// What a click on this control does.
    #[allow(clippy::too_many_lines)]
    pub(super) fn handle(self, cx: Cx<'_, '_, '_, '_, '_>) {
        let Cx {
            state,
            prefs,
            scrolled,
            mailbox,
            settings,
        } = cx;
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            FrontPress::CentralSignOut => {
                super::central::logout(state, prefs);
                super::central::enable(state);
            }
            FrontPress::WithoutAccount => {
                #[cfg(not(target_arch = "wasm32"))]
                super::central::without_account(state);
                // Nobody asked them yet: Central's form keeps the device's
                // addresses to itself until the player leaves it.
                for url in state.unasked_gateways() {
                    http::probe_gateway(url, mailbox);
                }
            }
            FrontPress::BackToCentral => {
                #[cfg(not(target_arch = "wasm32"))]
                super::central::back_to_sign_in(state);
            }
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
            FrontPress::FrontMenu => {
                state.header_menu = None;
                state.front_menu = !state.front_menu;
            }
            FrontPress::OpenCommunityDialog(kind) => {
                state.front_menu = false;
                state.community_dialog = Some(kind);
            }
            FrontPress::CloseCommunityDialog => {
                state.community_dialog = None;
            }
            FrontPress::Donate => {
                state.front_menu = false;
                state.community_dialog = None;
                state.giving.open = true;
            }
            FrontPress::CloseDonate => state.giving.open = false,
            FrontPress::Nothing => {}
            FrontPress::BackToSignIn => {
                state.back_a_face();
            }
            FrontPress::GuestFace => {
                // From Central's form, a gateway that takes guests too; or,
                // signed in at Central, the door a guests' gateway left open.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    super::central::guest_instead(state);
                    super::central::guest_here(state);
                }
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
            // The same confirmation, password and request as Settings'
            // Delete account: declining the terms deletes nothing else.
            FrontPress::TermsDecline => {
                if state.terms_subject == terms::Subject::Gateway {
                    state.lobby.ask_to_delete_account();
                }
            }
            FrontPress::TermsLater => {
                if let Some(ask) = state.terms.not_now(false) {
                    terms::perform(ask, state, prefs, scrolled, mailbox, settings);
                }
            }
            FrontPress::TermsRetry => {
                if let Some(ask) = state.terms.retry() {
                    terms::perform(ask, state, prefs, scrolled, mailbox, settings);
                }
            }
            press @ (FrontPress::OpenSource
            | FrontPress::OpenDiscord
            | FrontPress::OpenWhatsApp
            | FrontPress::OpenCentralRegister
            | FrontPress::OpenWebsite
            | FrontPress::OpenChangelog
            | FrontPress::OpenExternal(_)
            | FrontPress::OpenGiving(_)
            | FrontPress::OpenImpressum) => {
                if state.community_dialog.is_some() {
                    state.community_dialog = None;
                }
                super::source::open(press, state);
            }
            FrontPress::ToggleGatewayGroup(group) => {
                let slot = &mut state.gateway_collapsed[group.index()];
                *slot = !*slot;
                let visible = state.visible_gateway_indices();
                if state
                    .gateway_cursor
                    .is_some_and(|at| !visible.contains(&at))
                {
                    state.gateway_cursor = visible.first().copied();
                }
            }
            FrontPress::CycleGatewaySort => {
                state.gateway_sort = state.gateway_sort.next();
                scrolled.set(List::Gateways, 0.0);
                state.gateway_cursor = state.visible_gateway_indices().first().copied();
            }
            FrontPress::ClearGatewaySearch => {
                state.lobby.set_field(Field::GatewaySearch, "");
                scrolled.set(List::Gateways, 0.0);
                state.gateway_cursor = state.visible_gateway_indices().first().copied();
            }
            FrontPress::RefreshGateways => {
                for url in state.displayed_gateways().to_vec() {
                    state.probes.entry(url.clone()).or_insert(Probe::Asking);
                    http::probe_gateway(url, mailbox);
                }
            }
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
            FrontPress::PlayOffline => start_offline(state, prefs, scrolled, mailbox),
        }
    }
}

/// Enters the offline lobby and settles its initial local requests (`ListDecks`,
/// `ListTables`, and starter precons when the account has no decks yet) in the
/// same frame so the lobby renders populated on its very first frame.
fn start_offline(
    state: &mut LobbyState,
    prefs: &mut crate::prefs::Prefs,
    scrolled: &mut Scrolled,
    mailbox: &Mailbox,
) {
    state.gateway_epoch = state.gateway_epoch.wrapping_add(1);
    prefs.detach();
    scrolled.set(List::Table, 0.0);
    // Whatever is already here is kept. `HubPress::SignOut` is the only
    // thing that clears it, so a player coming back to offline
    // play finds the decks and the room they left.
    state
        .offline
        .get_or_insert_with(super::offline::Offline::load);
    let lang = state.lobby.lang();
    let mut request = state.lobby.play_offline();
    while let Some(req) = request {
        let Some(offline) = state.offline.as_mut() else {
            dispatch(state, mailbox, Some(req));
            break;
        };
        let event = offline.perform(req, lang);
        request = state.lobby.apply(event);
    }
    if state.lobby.decks().is_empty()
        && state.lobby.library().house.is_empty()
        && let Some(req) = state.lobby.browse_house()
    {
        if let Some(offline) = state.offline.as_mut() {
            let event = offline.perform(req, lang);
            let next = state.lobby.apply(event);
            dispatch(state, mailbox, next);
        } else {
            dispatch(state, mailbox, Some(req));
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
            "Baylee is unofficial Fan Content permitted under the Fan Content Policy. \
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

    /// The logo follows the stage: it may not step, so the height leaves
    /// where the panel going stood, arrives where the panel coming stands,
    /// and on the way never moves further in a frame than the curve allows.
    #[test]
    fn the_stage_travels_between_the_panels_heights_without_a_step() {
        let (from, to) = (452.0, 396.0);
        assert!((stage_height(from, to, 0.0) - from).abs() < 1e-3);
        assert!((stage_height(from, to, 1.0) - to).abs() < 1e-3);
        let frames = 48_u8;
        let mut last = from;
        for frame in 1..=frames {
            let t = f32::from(frame) / f32::from(frames);
            let height = stage_height(from, to, t);
            assert!(height <= last, "it goes one way");
            assert!(last - height < 2.5, "a step of {} at {t}", last - height);
            last = height;
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
