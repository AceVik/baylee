//! Door geometry is anchored to the single 3:2 campaign image; hover changes
//! light alone, including on the four gilded doors.
use super::{
    Action, Book, api,
    draw::{ORDER, label},
};
use crate::{
    shellkit::{controls::Kit, focus::Stop, tokens},
    ui_button::UiButton,
};
use baylee_client_core::Phrase;
use bevy::{
    asset::embedded_asset, picking::hover::Hovered, prelude::*,
    render::render_resource::AsBindGroup, shader::ShaderRef, ui::Pressed,
};

#[derive(Component)]
pub(super) struct ButtonGlow;

pub(super) fn plaque(commands: &mut Commands, kit: Kit) -> Entity {
    let mut access = accesskit::Node::new(accesskit::Role::Button);
    access.set_label(Phrase::AdventCalendar.text(kit.lang()));
    let button = commands
        .spawn((
            Action::Open,
            Stop::new(super::super::orders::PLAY.name, "advent"),
            bevy::a11y::AccessibilityNode(access),
            Name::new("Winter tavern calendar"),
            Node {
                width: percent(100),
                min_height: kit.m.px(70.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(kit.m.px(9.)),
                border: UiRect::all(kit.m.px(1.)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.77, 0.59, 0.32)),
            BackgroundColor(Color::srgb(0.033, 0.17, 0.18)),
        ))
        .id();
    let tavern = commands
        .spawn((
            Text::new(Phrase::AdventTavern.text(kit.lang())),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(Color::srgb(0.94, 0.79, 0.55)),
            Pickable::IGNORE,
        ))
        .id();
    let event = commands
        .spawn((
            Text::new(Phrase::AdventName.text(kit.lang())),
            crate::hud::tf_bold(kit.fonts, kit.m.head),
            TextColor(Color::srgb(1., 0.96, 0.84)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(button).add_children(&[tavern, event]);
    button
}
#[derive(Component)]
struct Glow {
    button: Entity,
}
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct Light {
    #[uniform(0)]
    params: Vec4,
}
#[cfg(feature = "dev-control")]
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ShaderProbe<'w, 's> {
    glows: Query<'w, 's, (&'static Glow, &'static MaterialNode<Light>)>,
    materials: Option<Res<'w, Assets<Light>>>,
}
#[cfg(feature = "dev-control")]
impl ShaderProbe<'_, '_> {
    pub(crate) fn button(&self, entity: Entity) -> Option<[f32; 4]> {
        let (_, node) = self.glows.iter().find(|(glow, _)| glow.button == entity)?;
        Some(self.materials.as_ref()?.get(node.id())?.params.to_array())
    }
}
impl UiMaterial for Light {
    fn fragment_shader() -> ShaderRef {
        "embedded://baylee_client/lobby/advent/light.wesl".into()
    }
}
pub(super) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>()
        || app.get_sub_app(bevy::render::RenderApp).is_none()
    {
        return;
    }
    embedded_asset!(app, "light.wesl");
    app.add_plugins(UiMaterialPlugin::<Light>::default())
        .add_systems(Update, (attach, animate, light_doors).chain());
}

#[derive(Component, Clone, Copy)]
struct DoorLight {
    special: bool,
    open: bool,
}
impl DoorLight {
    fn edge(self) -> Color {
        if self.open {
            Color::srgb(0.55, 0.98, 0.88)
        } else if self.special {
            Color::srgb(0.99, 0.76, 0.43)
        } else {
            Color::srgba(0.96, 0.89, 0.73, 0.65)
        }
    }
}
fn light_doors(
    mut doors: Query<(
        &DoorLight,
        &Hovered,
        Has<Pressed>,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (style, hover, pressed, mut bg, mut border) in &mut doors {
        let (background, edge) = if pressed {
            (
                Color::srgba(0.56, 0.30, 0.11, 0.84),
                Color::srgb(1., 0.87, 0.62),
            )
        } else if hover.0 {
            (
                Color::srgba(0.38, 0.22, 0.10, 0.66),
                Color::srgb(1., 0.82, 0.51),
            )
        } else {
            (Color::srgba(0.055, 0.042, 0.032, 0.38), style.edge())
        };
        if bg.0 != background {
            bg.0 = background;
        }
        let color = BorderColor::all(edge);
        if *border != color {
            *border = color;
        }
    }
}
fn attach(
    mut commands: Commands,
    buttons: Query<Entity, Added<ButtonGlow>>,
    mut assets: ResMut<Assets<Light>>,
) {
    for button in &buttons {
        let e = commands
            .spawn((
                Glow { button },
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
                    ..default()
                },
                MaterialNode(assets.add(Light { params: Vec4::ZERO })),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(button).insert_child(0, e);
    }
}
fn animate(
    book: Res<Book>,
    time: Res<Time>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    interactions: Query<(&Hovered, Has<Pressed>)>,
    glows: Query<(&Glow, &MaterialNode<Light>)>,
    mut assets: ResMut<Assets<Light>>,
) {
    let motion = book.unsolved() && prefs.as_deref().is_none_or(|p| !p.all().reduce_motion);
    for (g, material) in &glows {
        let hover = match interactions.get(g.button) {
            Ok((_, true)) => 1.,
            Ok((h, false)) if h.0 => 0.55,
            _ => 0.,
        };
        let t = if motion {
            time.elapsed_secs().rem_euclid(48.)
        } else {
            0.
        };
        let params = Vec4::new(t, f32::from(motion), hover, f32::from(book.unsolved()));
        if assets
            .get(material.id())
            .is_some_and(|m| m.params != params)
            && let Some(mut m) = assets.get_mut(material.id())
        {
            m.params = params;
        }
    }
}
#[derive(Clone, Copy)]
struct Place {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}
const NUMBERS: [u8; 23] = [
    17, 3, 21, 8, 14, 1, 10, 19, 5, 23, 6, 15, 2, 12, 22, 4, 20, 9, 7, 18, 11, 13, 16,
];
fn place(index: usize) -> Place {
    match index {
        0..6 => Place {
            x: 2. + 16.3 * index as f32,
            y: 4.,
            w: 14.,
            h: 14.,
        },
        6..18 => {
            let slot = index - 6;
            Place {
                x: [3., 21., 63., 81.][slot % 4],
                y: 25. + 18. * (slot / 4) as f32,
                w: 14.,
                h: 14.,
            }
        }
        18..23 => Place {
            x: 3. + 19.8 * (index - 18) as f32,
            y: 82.,
            w: 15.,
            h: 14.,
        },
        _ => Place {
            x: 42.,
            y: 34.,
            w: 16.,
            h: 40.,
        },
    }
}
pub(super) fn mosaic(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    book: &Book,
    max_width: f32,
) {
    let Some(c) = &book.calendar else { return };
    let mosaic = commands
        .spawn((
            Node {
                width: percent(100),
                max_width: px(max_width),
                aspect_ratio: Some(1.5),
                align_self: AlignSelf::Center,
                flex_shrink: 0.,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.07, 0.12, 0.16)),
            Name::new("Advent winter tavern"),
        ))
        .id();
    commands.entity(parent).add_child(mosaic);
    if let Some(image) = c
        .hero
        .as_ref()
        .and_then(|meta| book.scoped_image(&api::AssetScope::Calendar { year: c.year }, meta))
    {
        commands
            .entity(mosaic)
            .insert(ImageNode::new(image.clone()));
    }
    for (i, n) in NUMBERS.into_iter().chain([24]).enumerate() {
        let Some(door) = book.door(n) else { continue };
        let p = place(i);
        tile(commands, mosaic, kit, door, p);
    }
    if c.hero.is_some()
        && book
            .media
            .error(api::AssetScope::Calendar { year: c.year })
            .is_some()
    {
        label(
            commands,
            parent,
            kit,
            Phrase::AdventImageFailed.text(kit.lang()),
            tokens::MUTED,
        );
        super::draw::tile_action(
            commands,
            parent,
            kit,
            Phrase::AdventRetry.text(kit.lang()),
            Action::Retry,
            "retry",
            0,
        );
    }
}
fn tile(commands: &mut Commands, mosaic: Entity, kit: Kit, door: &api::Door, p: Place) {
    let n = door.door;
    let active = matches!(
        door.state,
        api::DoorState::Open | api::DoorState::Solved | api::DoorState::Claimed
    );
    let special = matches!(n, 6 | 13 | 20 | 24);
    let style = DoorLight {
        special,
        open: door.state == api::DoorState::Open,
    };
    let name = format!(
        "{} · {}",
        Phrase::AdventDoor.fill(kit.lang(), &[&n.to_string()]),
        state_label(door.state).text(kit.lang())
    );
    let mut access = accesskit::Node::new(if active {
        accesskit::Role::Button
    } else {
        accesskit::Role::Label
    });
    access.set_label(name.clone());
    let e = commands
        .spawn((
            Name::new(name),
            bevy::a11y::AccessibilityNode(access),
            Node {
                position_type: PositionType::Absolute,
                left: percent(p.x),
                top: percent(p.y),
                width: percent(p.w),
                height: percent(p.h),
                border: UiRect::all(px(if special || style.open { 2.0 } else { 1.0 })),
                border_radius: BorderRadius::all(px(5)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BorderColor::all(style.edge()),
            BackgroundColor(Color::srgba(
                0.055,
                0.042,
                0.032,
                if active { 0.38 } else { 0.19 },
            )),
        ))
        .id();
    if active {
        commands.entity(e).insert((
            UiButton,
            style,
            Action::Door(n),
            Stop::item(ORDER.name, "door", n),
        ));
    }
    let plate = number_plate(commands, kit, door, special);
    commands.entity(e).add_child(plate);
    commands.entity(mosaic).add_child(e);
}
fn number_plate(commands: &mut Commands, kit: Kit, door: &api::Door, special: bool) -> Entity {
    let n = door.door;
    let plate = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: kit.m.px(4.),
                padding: UiRect::axes(kit.m.px(6.), kit.m.px(2.)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(if door.state == api::DoorState::Open {
                Color::srgba(0.04, 0.19, 0.17, 0.95)
            } else {
                Color::srgba(0.07, 0.04, 0.025, 0.86)
            }),
            Pickable::IGNORE,
        ))
        .id();
    let mark = if matches!(door.state, api::DoorState::Claimed | api::DoorState::Solved) {
        " ✓"
    } else {
        ""
    };
    let text = commands
        .spawn((
            Text::new(format!("{n}{mark}")),
            crate::hud::tf_bold(kit.fonts, if n == 24 { kit.m.head } else { kit.m.text }),
            TextColor(if special { tokens::GOLD } else { tokens::INK }),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(plate).add_child(text);
    if door.state == api::DoorState::Open {
        let star = commands
            .spawn((
                Text::new(crate::lobby::parts::STAR.to_string()),
                crate::hud::icon_tf(kit.fonts, kit.m.small * 0.80),
                TextColor(tokens::GOLD),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(plate).add_child(star);
    }
    plate
}
fn state_label(state: api::DoorState) -> Phrase {
    match state {
        api::DoorState::Locked => Phrase::AdventLocked,
        api::DoorState::Open => Phrase::AdventAvailable,
        api::DoorState::Solved => Phrase::AdventSolved,
        api::DoorState::Claimed => Phrase::AdventClaimed,
        api::DoorState::Expired => Phrase::AdventExpired,
    }
}

#[cfg(test)]
mod tests {
    use super::{NUMBERS, place};
    #[test]
    fn the_advent_door_shader_accepts_the_native_ui_vertex_input() {
        let prelude = "struct UiVertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };";
        crate::cardmat::tests::check_wgsl(include_str!("light.wesl"), prelude);
    }
    #[test]
    fn twenty_four_doors_are_unique_inside_the_same_image_without_overlap() {
        let mut ids = NUMBERS.to_vec();
        ids.push(24);
        ids.sort_unstable();
        assert_eq!(ids, (1..=24).collect::<Vec<_>>());
        for i in 0..24 {
            let a = place(i);
            assert!(a.x >= 0. && a.y >= 0. && a.x + a.w <= 100. && a.y + a.h <= 100.);
            for j in 0..i {
                let b = place(j);
                assert!(
                    a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y
                );
            }
        }
    }
}
