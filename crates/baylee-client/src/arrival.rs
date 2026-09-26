//! Readiness belongs to the scene, not to receipt of its first packet.
//! Keep an opaque screen across camera handoff, prepare the real scene below,
//! then acknowledge actual render preparation before scheduling its reveal.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use baylee_client_core::{
    i18n::{Lang, Phrase, Refusal},
    lobby::gateway_info::Probe,
};
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        mesh::RenderMesh,
        render_asset::RenderAssets,
        render_resource::{AsBindGroup, CachedPipelineState, PipelineCache, ShaderType},
        texture::GpuImage,
    },
    shader::ShaderRef,
};

use crate::{Duel, DuelCommand, DuelPhase, InstalledHost, hud::UiFonts};

const FLIGHT: f64 = baylee_protocol::TABLE_ENTRANCE_MS as f64 / 1000.0;
const INK: Color = Color::srgb(0.82, 0.77, 0.65);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Destination {
    Login,
    Table,
    Hidden,
}

#[derive(Resource)]
pub(crate) struct Journey {
    destination: Destination,
    departure: Option<f64>,
    ready: u32,
    total: u32,
    error: Option<Refusal>,
    since: Option<f64>,
    milestone: u8,
    revision: u64,
}

impl Default for Journey {
    fn default() -> Self {
        Self {
            destination: Destination::Hidden,
            departure: None,
            ready: 0,
            total: 0,
            error: None,
            since: None,
            milestone: 0,
            revision: 0,
        }
    }
}

impl Journey {
    pub(crate) fn active(&self) -> bool {
        self.destination != Destination::Hidden
    }

    pub(crate) fn synchronize(&mut self, ready: u32, total: u32, starts_in: Option<f64>, now: f64) {
        self.ready = ready;
        self.total = total;
        self.departure = starts_in.map(|wait| now + wait);
    }

    #[cfg(all(feature = "dev-control", not(target_arch = "wasm32")))]
    pub(crate) fn diagnostic(&self, now: f64) -> serde_json::Value {
        serde_json::json!({
            "destination": match self.destination { Destination::Login => "login", Destination::Table => "table", Destination::Hidden => "hidden" },
            "milestone": self.milestone, "ready": self.ready, "total": self.total,
            "departure": self.departure, "progress": self.progress(now), "error": self.error.as_ref().map(|e| e.text(Lang::En)),
        })
    }

    pub(crate) fn changed_scene(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub(crate) fn fail(&mut self, reason: String) {
        self.error = Some(Refusal::Verbatim(reason));
        self.departure = None;
    }

    fn fail_local(&mut self, reason: Phrase) {
        self.error = Some(Refusal::Said(reason));
        self.departure = None;
    }

    fn progress(&self, now: f64) -> f32 {
        self.departure
            .map_or(0.0, |at| ((now - at) / FLIGHT).clamp(0.0, 1.0) as f32)
    }
}

/// A generation-tagged request prevents a render acknowledgement from a
/// previous scene from releasing a newly opened table. Shared atomics only
/// carry feedback; no main-world resources are touched by the render thread.
#[derive(Resource, Clone, ExtractResource, Default)]
struct Prepared {
    generation: u64,
    scene_revision: u64,
    // Pin the submitted snapshot until the cover goes away: a speculative
    // preload evicted during extraction must not strand an acknowledgement.
    images: Vec<Handle<Image>>,
    meshes: Vec<Handle<Mesh>>,
    requested: bool,
    acknowledged: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
}

impl Prepared {
    fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.requested = false;
        self.images.clear();
        self.meshes.clear();
        self.failed.store(false, Ordering::Relaxed);
    }

    fn ready(&self) -> bool {
        self.requested && self.acknowledged.load(Ordering::Acquire) == self.generation
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct PassageMaterial {
    #[uniform(0)]
    params: PassageParams,
}

#[derive(ShaderType, Clone, Copy, Default)]
struct PassageParams {
    // aspect, time, flight, reduced motion
    scene: Vec4,
    // actual preparation milestone, table/login, unused
    state: Vec4,
}

impl UiMaterial for PassageMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/shaders/loading.wgsl".into()
    }
}

#[derive(Component)]
struct Cover;
#[derive(Component)]
struct Words(u8);
#[derive(Component)]
struct Exit;

/// Startup belongs to the standalone shell. An embedded duel must not cover
/// its host application's screen before anyone asks to open a table.
pub(crate) fn start_login(app: &mut App) {
    if let Some(mut journey) = app.world_mut().get_resource_mut::<Journey>() {
        journey.destination = Destination::Login;
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>() || app.get_sub_app(RenderApp).is_none() {
        return;
    }
    embedded_asset!(app, "shaders/loading.wgsl");
    app.init_resource::<Journey>()
        .init_resource::<Prepared>()
        .add_plugins((
            UiMaterialPlugin::<PassageMaterial>::default(),
            ExtractResourcePlugin::<Prepared>::default(),
        ))
        .add_systems(Startup, spawn)
        .add_systems(OnEnter(DuelPhase::Opening), begin_table)
        .add_systems(OnEnter(DuelPhase::Closed), close_table)
        .add_systems(Update, cancel)
        .add_systems(
            PostUpdate,
            (cover_requests, paint)
                .chain()
                .before(bevy::ui::UiSystems::Prepare),
        )
        // After all presentation systems and deferred spawns. The scene is
        // rendered while covered, so specialization is warmed for real.
        .add_systems(PostUpdate, prepare.after(bevy::ui::UiSystems::Layout));
    app.sub_app_mut(RenderApp)
        .add_systems(Render, acknowledge.after(RenderSystems::Render));
}

// Read the lobby after its complete update. Even the frame that grants a
// seat is covered, before NextState switches cameras on the following frame.
fn cover_requests(
    lobby: Option<Res<crate::lobby::LobbyState>>,
    phase: Res<State<DuelPhase>>,
    mut journey: ResMut<Journey>,
    mut prepared: ResMut<Prepared>,
) {
    if *phase.get() != DuelPhase::Closed {
        return;
    }
    let seated = lobby.is_some_and(|l| {
        matches!(
            l.lobby.screen(),
            baylee_client_core::lobby::Screen::Seated(_)
        )
    });
    if seated && journey.destination != Destination::Table {
        *journey = Journey {
            destination: Destination::Table,
            ..default()
        };
        prepared.reset();
    } else if !seated && journey.destination == Destination::Table {
        journey.destination = Destination::Hidden;
        prepared.reset();
    }
}

fn begin_table(mut journey: ResMut<Journey>, mut prepared: ResMut<Prepared>) {
    *journey = Journey {
        destination: Destination::Table,
        ..default()
    };
    prepared.reset();
}

fn close_table(mut journey: ResMut<Journey>, mut prepared: ResMut<Prepared>) {
    if journey.destination == Destination::Table {
        journey.destination = Destination::Hidden;
        prepared.reset();
    }
}

fn acknowledge(
    request: Res<Prepared>,
    images: Res<RenderAssets<GpuImage>>,
    meshes: Res<RenderAssets<RenderMesh>>,
    pipelines: Res<PipelineCache>,
    mut stable: Local<(u64, u8)>,
) {
    if !request.requested {
        *stable = (0, 0);
        return;
    }
    let failed = pipelines
        .pipelines()
        .any(|p| matches!(&p.state, CachedPipelineState::Err(error) if !matches!(error, bevy::shader::ShaderCacheError::ShaderNotLoaded(_) | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable)));
    request.failed.store(failed, Ordering::Relaxed);
    let ready = !failed
        && pipelines.waiting_pipelines().next().is_none()
        && request
            .images
            .iter()
            .all(|id| images.get(id.id()).is_some())
        && request
            .meshes
            .iter()
            .all(|id| meshes.get(id.id()).is_some());
    if stable.0 != request.generation || !ready {
        *stable = (request.generation, 0);
    }
    if ready {
        stable.1 = stable.1.saturating_add(1);
        // Covers extraction, texture upload, specialization, layout and a
        // submitted frame. This counts prepared frames, not elapsed time.
        if stable.1 >= 3 {
            request
                .acknowledged
                .store(request.generation, Ordering::Release);
        }
    }
}

fn fonts_ready(fonts: &UiFonts, assets: &AssetServer) -> Result<bool, ()> {
    let mut ready = true;
    for font in [
        &fonts.text,
        &fonts.medium,
        &fonts.bold,
        &fonts.italic,
        &fonts.medium_italic,
        &fonts.serif,
        &fonts.serif_italic,
        &fonts.icons,
        &fonts.mana,
    ] {
        if matches!(
            assets.get_load_state(font.id()),
            Some(bevy::asset::LoadState::Failed(_))
        ) {
            return Err(());
        }
        ready &= assets.is_loaded_with_dependencies(font.id());
    }
    Ok(ready)
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)] // startup readiness includes fonts, login art and destination assets
fn prepare(
    mut journey: ResMut<Journey>,
    mut prepared: ResMut<Prepared>,
    fonts: Res<UiFonts>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    vista_art: Option<Res<crate::vista::VistaArt>>,
    meshes: Query<&Mesh3d>,
    textures: Res<crate::textures::CardTextures>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    mut duel: ResMut<Duel>,
    host: Option<ResMut<InstalledHost>>,
    mut next: ResMut<NextState<DuelPhase>>,
    phase: Res<State<DuelPhase>>,
    time: Res<Time<Real>>,
) {
    if !journey.active()
        || journey.error.is_some()
        || (journey.destination == Destination::Table && *phase.get() == DuelPhase::Closed)
    {
        return;
    }
    if prepared.scene_revision != journey.revision {
        prepared.reset();
        prepared.scene_revision = journey.revision;
    }
    let now = time.elapsed_secs_f64();
    let since = *journey.since.get_or_insert(now);
    if now - since > 125.0 && journey.departure.is_none() {
        journey.fail_local(Phrase::ArrivalTimeout);
        return;
    }
    let art_ready = if journey.destination == Destination::Login {
        vista_art
            .as_ref()
            .map_or(Ok(true), |art| art.ready(&assets))
    } else {
        Ok(true)
    };
    let Ok(fonts_ok) =
        fonts_ready(&fonts, &assets).and_then(|fonts| art_ready.map(|art| fonts && art))
    else {
        journey.fail_local(Phrase::ArrivalAssetsFailed);
        return;
    };
    let data_ok = match journey.destination {
        Destination::Login => lobby.as_ref().is_none_or(|lobby| {
            lobby.probes.values().all(|p| !matches!(p, Probe::Asking))
                && lobby.auth_probes.load(Ordering::Relaxed) == 0
        }),
        Destination::Table => {
            duel.statics.is_some() && duel.view.as_ref().is_some_and(|v| textures.table_ready(v))
        }
        Destination::Hidden => return,
    };
    journey.milestone = if !fonts_ok {
        0
    } else if !data_ok {
        1
    } else {
        2
    };
    if !fonts_ok || !data_ok {
        // A changed view during a reconnect must earn a new acknowledgement.
        if prepared.requested {
            prepared.reset();
        }
        return;
    }
    if !prepared.requested {
        prepared.reset();
        let resident: Vec<_> = images
            .iter()
            .filter(|(_, image)| {
                image
                    .asset_usage
                    .contains(bevy::asset::RenderAssetUsages::RENDER_WORLD)
            })
            .map(|(id, _)| id)
            .collect();
        prepared.images = resident
            .into_iter()
            .filter_map(|id| images.get_strong_handle(id))
            .collect();
        prepared.meshes = meshes.iter().map(|mesh| mesh.0.clone()).collect();
        prepared.requested = true;
        return;
    }
    if prepared.failed.load(Ordering::Relaxed) {
        journey.fail_local(Phrase::ArrivalGraphicsFailed);
        return;
    }
    if !prepared.ready() {
        return;
    }
    journey.milestone = 3;
    if journey.destination == Destination::Login {
        journey.departure.get_or_insert(now);
    } else {
        if !duel.ready_sent
            && let Some(mut host) = host
        {
            if !duel.curtain_up {
                host.0.ready();
            }
            duel.ready_sent = true;
        }
        // In-process games and late reconnects already have their curtain.
        // Network starts are timed by Preparing instead, before Curtain.
        if duel.curtain_up {
            journey.departure.get_or_insert(now);
        }
    }
    if journey.progress(now) >= 1.0
        && (journey.destination == Destination::Login || duel.curtain_up)
    {
        if journey.destination == Destination::Table {
            next.set(DuelPhase::Playing);
        }
        journey.destination = Destination::Hidden;
        prepared.reset();
    }
}

fn spawn(mut commands: Commands, mut materials: ResMut<Assets<PassageMaterial>>) {
    commands
        .spawn((
            Cover,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Vw(5.0), Val::Vh(6.0)),
                ..default()
            },
            GlobalZIndex(500),
            BackgroundColor(Color::srgb(0.009, 0.013, 0.029)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                MaterialNode(materials.add(PassageMaterial { params: default() })),
                Pickable::IGNORE,
            ));
            root.spawn((
                Words(4),
                Text::new("B A Y L E E"),
                TextFont {
                    font_size: 24.0.into(),
                    ..default()
                },
                TextColor(INK),
                Pickable::IGNORE,
            ));
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(71.0),
                    width: Val::Percent(90.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|words| {
                for (index, size) in [(0, 30.0), (1, 18.0), (2, 13.0)] {
                    words.spawn((
                        Words(index),
                        Text::new(""),
                        TextFont {
                            font_size: size.into(),
                            ..default()
                        },
                        TextColor(INK),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                }
            });
            root.spawn((
                Exit,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(22.0), Val::Px(12.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(0.65, 0.52, 0.35, 0.35)),
                BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.6)),
            ))
            .with_children(|button| {
                button.spawn((
                    Words(3),
                    Text::new(""),
                    TextFont {
                        font_size: 15.0.into(),
                        ..default()
                    },
                    TextColor(INK),
                    Pickable::IGNORE,
                ));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn paint(
    journey: Res<Journey>,
    time: Res<Time<Real>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    fonts: Option<Res<UiFonts>>,
    assets: Res<AssetServer>,
    lobby: Option<Res<crate::lobby::LobbyState>>,
    mut root: Query<(&mut Visibility, &mut BackgroundColor), With<Cover>>,
    surface: Query<(&ComputedNode, &MaterialNode<PassageMaterial>)>,
    mut materials: ResMut<Assets<PassageMaterial>>,
    mut words: Query<(&Words, &mut Text, &mut TextFont, &mut TextColor)>,
    mut exit: Query<&mut Visibility, (With<Exit>, Without<Cover>)>,
) {
    let Ok((mut visible, mut backdrop)) = root.single_mut() else {
        return;
    };
    visible.set_if_neq(if journey.active() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    if !journey.active() {
        return;
    }
    let now = time.elapsed_secs_f64();
    let progress = journey.progress(now);
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let table = journey.destination == Destination::Table;
    let lang = settings.map_or(Lang::En, |s| Lang::of(&s.lang));
    for (size, handle) in &surface {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params = PassageParams {
                scene: Vec4::new(
                    size.size().x / size.size().y.max(1.0),
                    if still { 0.0 } else { now as f32 },
                    progress,
                    f32::from(still),
                ),
                state: Vec4::new(f32::from(journey.milestone), f32::from(table), 0.0, 0.0),
            };
        }
    }
    // Opaque even before the shader has compiled. Reveal only after readiness.
    backdrop.0 =
        Color::srgb(0.009, 0.013, 0.029).with_alpha(if progress > 0.0 { 0.0 } else { 1.0 });
    let title = if journey.error.is_some() {
        Phrase::ArrivalHeld
    } else if table {
        Phrase::ArrivalTable
    } else {
        Phrase::ArrivalLogin
    };
    let status = status(&journey, progress, lang);
    for (word, mut text, mut font, mut color) in &mut words {
        let value = match word.0 {
            0 => title.text(lang).to_string(),
            1 => status.clone(),
            2 => if table {
                Phrase::ArrivalTableSteps
            } else {
                Phrase::ArrivalLoginSteps
            }
            .text(lang)
            .to_string(),
            4 => "B A Y L E E".to_string(),
            _ => if lobby.is_some() {
                Phrase::ArrivalLeave
            } else {
                Phrase::ArrivalLeaveTable
            }
            .text(lang)
            .to_string(),
        };
        text.set_if_neq(Text::new(value));
        if let Some(fonts) = &fonts {
            let handle = if word.0 == 0 {
                &fonts.serif
            } else {
                &fonts.text
            };
            let wanted = if assets.is_loaded_with_dependencies(handle.id()) {
                handle.clone().into()
            } else {
                default()
            };
            if font.font != wanted {
                font.font = wanted;
            }
        }
        color.0 = INK.with_alpha(
            (1.0 - progress * 3.0).clamp(0.0, 1.0) * if word.0 == 2 { 0.5 } else { 1.0 },
        );
    }
    for mut visibility in &mut exit {
        *visibility = if table && progress == 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn status(journey: &Journey, progress: f32, lang: Lang) -> String {
    if let Some(error) = &journey.error {
        return error.text(lang);
    }
    if progress > 0.0 {
        return Phrase::ArrivalPortal.text(lang).into();
    }
    let table = journey.destination == Destination::Table;
    if journey.milestone == 3 && table && journey.total > 0 {
        return Phrase::ArrivalPlayers.fill(
            lang,
            &[&journey.ready.to_string(), &journey.total.to_string()],
        );
    }
    match (table, journey.milestone) {
        (_, 0) => Phrase::ArrivalWorld,
        (false, 1) => Phrase::ArrivalGateways,
        (true, 1) => Phrase::ArrivalCards,
        (_, 2) => Phrase::ArrivalGraphics,
        _ => Phrase::ArrivalReady,
    }
    .text(lang)
    .into()
}

fn cancel(
    journey: Res<Journey>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: MessageWriter<DuelCommand>,
    buttons: Query<&Interaction, (With<Exit>, Changed<Interaction>)>,
) {
    if journey.destination == Destination::Table
        && (keys.just_pressed(KeyCode::Escape)
            || buttons.iter().any(|i| *i == Interaction::Pressed))
    {
        commands.write(DuelCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_render_acknowledgements_do_not_release_the_next_table() {
        let mut prepared = Prepared::default();
        prepared.reset();
        prepared.requested = true;
        prepared
            .acknowledged
            .store(prepared.generation, Ordering::Release);
        assert!(prepared.ready());
        prepared.reset();
        prepared.requested = true;
        assert!(!prepared.ready());
    }

    #[test]
    #[allow(clippy::float_cmp)] // the exact endpoints produced by clamp
    fn synchronization_cancels_and_reschedules_a_flight_without_accumulating_time() {
        let mut journey = Journey {
            destination: Destination::Table,
            ..default()
        };
        journey.synchronize(2, 2, Some(1.5), 10.0);
        assert_eq!(journey.progress(11.0), 0.0);
        assert!((journey.progress(12.325) - 0.5).abs() < 0.0001);
        journey.synchronize(1, 2, None, 12.0);
        assert_eq!(journey.progress(20.0), 0.0);
        journey.synchronize(2, 2, Some(1.5), 20.0);
        assert_eq!(journey.progress(23.15), 1.0);
    }

    #[test]
    fn loading_shader_is_valid() {
        crate::cardmat::tests::check_wgsl(
            include_str!("shaders/loading.wgsl"),
            "
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};",
        );
    }
}
