//! Season-independent rank paints and open-centred portrait frames.
//! Each distinct design shares one material across every visible profile and board row.
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use std::collections::HashMap;

mod catalog;
mod promotion;
pub(crate) use catalog::embed as embed_art;
pub(crate) use promotion::Promotion;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Surface {
    Helmet,
    Cards,
    Frame,
    DeckFrame,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RankArt {
    tier: u8,
    division: u8,
    podium: u8,
    surface: Surface,
}

impl RankArt {
    pub(crate) fn badge(
        league: &str,
        division: Option<u8>,
        deck: bool,
        place: Option<u32>,
    ) -> Option<Self> {
        let tier = catalog::STYLES.iter().position(|style| {
            style.english.eq_ignore_ascii_case(league) || style.slug.eq_ignore_ascii_case(league)
        })?;
        let divisions = catalog::STYLES[tier].divisions;
        Some(Self {
            tier: tier as u8,
            division: division
                .filter(|n| (1..=divisions).contains(n))
                .unwrap_or(0),
            podium: match place {
                Some(n @ 1..=3) if tier == 10 && !deck => n as u8,
                _ => 0,
            },
            surface: if deck {
                Surface::Cards
            } else {
                Surface::Helmet
            },
        })
    }

    pub(crate) fn frame(league: &str, division: Option<u8>, place: Option<u32>) -> Option<Self> {
        let mut art = Self::badge(league, division, false, place)?;
        art.surface = Surface::Frame;
        Some(art)
    }

    pub(crate) fn deck_frame(
        league: &str,
        division: Option<u8>,
        place: Option<u32>,
    ) -> Option<Self> {
        let mut art = Self::frame(league, division, place)?;
        art.surface = Surface::DeckFrame;
        Some(art)
    }

    /// Diameter of the mounted portrait as a fraction of the engraved master.
    pub(crate) fn portrait_scale(self) -> f32 {
        match self.tier {
            0..=1 => 0.81,
            2..=4 => 0.77,
            5..=7 => 0.73,
            8..=10 => 0.67,
            _ => 0.95,
        }
    }

    fn path(self) -> String {
        let style = catalog::STYLES[usize::from(self.tier)];
        let track = if matches!(self.surface, Surface::Cards | Surface::DeckFrame) {
            "deck"
        } else {
            "player"
        };
        let suffix = if self.podium > 0 {
            ["", "-1", "-2", "-3"][usize::from(self.podium)]
        } else {
            ["", "-i", "-ii", "-iii", "-iv"][usize::from(self.division)]
        };
        let folder = if matches!(self.surface, Surface::Frame | Surface::DeckFrame) {
            "frames/"
        } else {
            ""
        };
        format!(
            "embedded://baylee_client/ranks/{folder}{track}/{}{suffix}.png",
            style.slug
        )
    }

    fn step(self) -> u8 {
        if self.division == 0 {
            0
        } else {
            catalog::STYLES[usize::from(self.tier)].divisions + 1 - self.division
        }
    }
}

#[derive(Clone, Copy, ShaderType)]
struct Params {
    // Tier, earned division step, podium, surface (helmet, fan, portrait frame).
    kind: UVec4,
    // Loop period in seconds; stationary when period or motion is zero.
    timing: Vec4,
    metal: Vec4,
    gem: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct Material {
    #[uniform(0)]
    params: Params,
    #[texture(1)]
    #[sampler(2)]
    image: Option<Handle<Image>>,
}

impl UiMaterial for Material {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/rank_art/surface.wesl".into()
    }
}

#[derive(Resource, Default)]
struct Paints {
    handles: HashMap<RankArt, Handle<Material>>,
    // Shader libraries must outlive drawing entities, including while the lobby is closed.
    _library: Handle<Shader>,
    still: Option<bool>,
}

fn linear(rgb: [u8; 3]) -> Vec4 {
    let c = Color::srgb_u8(rgb[0], rgb[1], rgb[2]).to_linear();
    Vec4::new(c.red, c.green, c.blue, 1.0)
}

fn paint(art: RankArt, server: &AssetServer, motion: f32) -> Material {
    let s = catalog::STYLES[usize::from(art.tier)];
    let (metal, gem) = match art.podium {
        3 => ([218, 155, 116], [249, 166, 171]),
        2 => ([219, 229, 240], [132, 201, 255]),
        _ => (s.metal, s.gem),
    };
    Material {
        params: Params {
            kind: UVec4::new(
                u32::from(art.tier),
                u32::from(art.step()),
                u32::from(art.podium),
                art.surface as u32,
            ),
            timing: Vec4::new(f32::from(s.period), motion, 0.0, 0.0),
            metal: linear(metal),
            gem: linear(gem),
        },
        image: (art.tier != catalog::UNRANKED).then(|| server.load(art.path())),
    }
}

fn dress(
    mut commands: Commands,
    changed: Query<(Entity, &RankArt), Changed<RankArt>>,
    mut paints: ResMut<Paints>,
    mut materials: ResMut<Assets<Material>>,
    server: Res<AssetServer>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    quality: Option<Res<crate::quality::InUse>>,
) {
    let still = crate::quality::ambient_still(
        prefs.as_ref().is_some_and(|p| p.all().reduce_motion),
        quality.as_deref(),
    );
    let motion = if still { 0.0 } else { 1.0 };
    if paints.still != Some(still) {
        for handle in paints.handles.values() {
            if let Some(mut material) = materials.get_mut(handle) {
                material.params.timing.y = motion;
            }
        }
        paints.still = Some(still);
    }
    for (entity, &art) in &changed {
        let handle = paints
            .handles
            .entry(art)
            .or_insert_with(|| materials.add(paint(art, &server, motion)))
            .clone();
        commands.entity(entity).try_insert(MaterialNode(handle));
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_message::<Promotion>();
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    embed_art(app.world().resource());
    let library = app
        .world()
        .resource::<AssetServer>()
        .load("embedded://baylee_client/rank_art/finish.wesl");
    embedded_asset!(app, "rank_art/surface.wesl");
    bevy::shader::load_shader_library!(app, "rank_art/portrait.wesl");
    app.insert_resource(Paints {
        _library: library,
        ..default()
    })
    .add_plugins(UiMaterialPlugin::<Material>::default())
    .add_systems(
        PostUpdate,
        (dress, ApplyDeferred)
            .chain()
            .before(bevy::ui::UiSystems::Prepare)
            .before(bevy::ui::UiSystems::Content)
            .before(bevy::ui::UiSystems::Layout)
            .before(bevy::ui::UiSystems::Stack)
            .before(bevy::asset::AssetEventSystems),
    );
    promotion::install(app);
}

#[cfg(test)]
mod tests;
