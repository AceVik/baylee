//! A non-modal reward in its own layout root. The caller emits once for a local settled match.
use super::{RankArt, catalog, linear};
use crate::{DuelPhase, hud, lobby::LobbyState, shellkit::tokens};
use baylee_client_core::i18n::Phrase;
use bevy::{
    asset::embedded_asset, prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef,
};

#[derive(Message, Clone)]
pub(crate) struct Promotion {
    pub before: RankArt,
    pub after: RankArt,
    /// Localized format and earned rank from the result receipt.
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Division,
    League,
    Elite,
}

impl Promotion {
    pub(super) fn step(&self) -> Option<Step> {
        let (before, after) = (self.before, self.after);
        if before.surface != after.surface || after.tier == catalog::UNRANKED {
            return None;
        }
        let league = before.tier == catalog::UNRANKED || after.tier > before.tier;
        let podium = after.tier == before.tier
            && after.podium > 0
            && (before.podium == 0 || after.podium < before.podium);
        if podium || (league && after.tier >= 8) {
            Some(Step::Elite)
        } else if league {
            Some(Step::League)
        } else if before.tier == after.tier && after.step() > before.step() {
            Some(Step::Division)
        } else {
            None
        }
    }
}

impl Step {
    fn duration(self) -> f32 {
        match self {
            Self::Division => 3.2,
            Self::League => 4.5,
            Self::Elite => 5.5,
        }
    }

    fn size(self) -> f32 {
        match self {
            Self::Division => 64.0,
            Self::League => 96.0,
            Self::Elite => 116.0,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct Halo {
    #[uniform(0)]
    tint: Vec4,
    #[uniform(1)]
    motion: Vec4,
}

impl UiMaterial for Halo {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/rank_art/corona.wesl".into()
    }
}

#[derive(Component)]
struct Celebrating {
    since: f64,
    step: Step,
    halo: Handle<Halo>,
}

#[derive(Component)]
struct Card;

#[derive(Resource, Default)]
struct Pending(std::collections::VecDeque<(Promotion, Step)>);

pub(super) fn install(app: &mut App) {
    embedded_asset!(app, "corona.wesl");
    app.add_plugins(UiMaterialPlugin::<Halo>::default())
        .init_resource::<Pending>()
        .add_systems(OnExit(DuelPhase::Closed), |mut queue: ResMut<Pending>| {
            queue.0.clear();
        })
        .add_systems(
            Update,
            (receive, animate)
                .chain()
                .run_if(in_state(DuelPhase::Closed)),
        );
}

#[allow(clippy::too_many_arguments)]
fn receive(
    mut commands: Commands,
    mut incoming: MessageReader<Promotion>,
    mut pending: ResMut<Pending>,
    state: Res<LobbyState>,
    fonts: Option<Res<hud::UiFonts>>,
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
    mut halos: ResMut<Assets<Halo>>,
    active: Query<Entity, With<Celebrating>>,
) {
    let Some(fonts) = fonts else {
        return;
    };
    // A match can promote both the player and their deck. Present the two in sequence.
    for promotion in incoming.read() {
        if let Some(step) = promotion.step()
            && pending.0.len() < 6
        {
            pending.0.push_back((promotion.clone(), step));
        }
    }
    if !active.is_empty() || pending.0.is_empty() {
        return;
    }
    let Some((promotion, step)) = pending.0.pop_front() else {
        return;
    };

    let lang = state.lobby.lang();
    let title = match step {
        Step::Division => Phrase::RankPromotionDivision,
        Step::League => Phrase::RankPromotionLeague,
        Step::Elite => Phrase::RankPromotionElite,
    }
    .text(lang);
    let still = crate::quality::ambient_still(
        prefs.as_ref().is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    let art = promotion.after;
    let style = catalog::STYLES[usize::from(art.tier)];
    let accent = Color::srgb_u8(style.metal[0], style.metal[1], style.metal[2]);
    let halo = halos.add(Halo {
        tint: linear(style.gem),
        motion: Vec4::new(f32::from(step as u8), if still { 0.0 } else { 1.0 }, 0., 0.),
    });
    spawn_card(
        &mut commands,
        &fonts,
        &promotion,
        step,
        title,
        time.elapsed_secs_f64(),
        accent,
        halo,
    );
}

#[allow(clippy::too_many_arguments)]
fn spawn_card(
    commands: &mut Commands,
    fonts: &hud::UiFonts,
    promotion: &Promotion,
    step: Step,
    title: &str,
    since: f64,
    accent: Color,
    halo: Handle<Halo>,
) {
    let art = promotion.after;
    let size = step.size();
    let outer = commands
        .spawn((
            Celebrating {
                since,
                step,
                halo: halo.clone(),
            },
            DespawnOnExit(DuelPhase::Closed),
            GlobalZIndex(tokens::z::TOAST - 1),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                top: percent(12),
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let panel = commands
        .spawn((
            Card,
            ChildOf(outer),
            Node {
                width: Val::Px(332.0),
                max_width: percent(92),
                padding: UiRect::all(Val::Px(12.0)),
                column_gap: Val::Px(10.0),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(tokens::RADIUS_PANEL)),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE.with_alpha(0.98)),
            BorderColor::all(accent.with_alpha(0.72)),
            Pickable::IGNORE,
        ))
        .id();
    let disc = commands
        .spawn((
            ChildOf(panel),
            Node {
                width: Val::Px(size),
                height: Val::Px(size),
                flex_shrink: 0.,
                ..default()
            },
            MaterialNode(halo),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        ChildOf(disc),
        art,
        Node {
            position_type: PositionType::Absolute,
            left: percent(10),
            top: percent(10),
            width: percent(80),
            height: percent(80),
            ..default()
        },
        Pickable::IGNORE,
    ));
    let words = commands
        .spawn((
            ChildOf(panel),
            Node {
                min_width: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        ChildOf(words),
        Text::new(title),
        hud::tf(fonts, 15.0),
        TextColor(accent),
        Pickable::IGNORE,
    ));
    commands.spawn((
        ChildOf(words),
        Text::new(&promotion.label),
        hud::tf_serif(fonts, 22.0, 600),
        TextColor(tokens::INK),
        Pickable::IGNORE,
    ));
}

fn animate(
    mut commands: Commands,
    time: Res<Time>,
    panels: Query<(Entity, &Celebrating, &Children)>,
    mut cards: Query<&mut UiTransform, With<Card>>,
    mut halos: ResMut<Assets<Halo>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let still = crate::quality::ambient_still(
        prefs.as_ref().is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    for (entity, panel, children) in &panels {
        let motion = if still { 0.0 } else { 1.0 };
        if halos
            .get(&panel.halo)
            .is_some_and(|h| (h.motion.y - motion).abs() > f32::EPSILON)
            && let Some(mut halo) = halos.get_mut(&panel.halo)
        {
            halo.motion.y = motion;
        }
        let age = (time.elapsed_secs_f64() - panel.since) as f32;
        if age >= panel.step.duration() {
            commands.entity(entity).despawn();
            continue;
        }
        let pose = if still {
            UiTransform::default()
        } else {
            let entrance = (age / 0.48).clamp(0., 1.);
            let exit = ((panel.step.duration() - age) / 0.32).clamp(0., 1.);
            let scale = match panel.step {
                Step::Division => 0.055,
                Step::League => 0.105,
                Step::Elite => 0.145,
            };
            let arriving = (1.0 - entrance).powi(3);
            let departing = (1.0 - exit).powi(2);
            UiTransform {
                scale: Vec2::splat(1.0 - scale * arriving - 0.045 * departing),
                // Move through the card's own clip. Text and neighbouring controls keep
                // their layout throughout, including the last frame of the announcement.
                translation: Val2::new(Val::ZERO, percent(105.0 * (arriving - departing))),
                ..default()
            }
        };
        if let Some(&child) = children.first()
            && let Ok(mut transform) = cards.get_mut(child)
        {
            transform.set_if_neq(pose);
        }
    }
}
