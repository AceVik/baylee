//! The card that opens over a person's name (owner, 12.11.2026).
//!
//! The pointer resting on a name — in a chat line, on a contact's row, on
//! the account's own pill — brings up a small card: their disc
//! (`shellkit::avatar`), their name and tag, where a friend is, how this
//! account stands to them and when it last played with them. What it says
//! is `client-core::social::card`'s decision; this module only waits, draws
//! and moves.
//!
//! Built like `lobby::hint`: one node outside every retained tree, so a
//! pointer crossing a list draws one panel and rebuilds nothing. It differs
//! in three ways. It **waits** ([`DWELL`]) before it opens, so a pointer
//! crossing a chat does not flicker cards. Once one stands, the next name is
//! **handed** the card at once and without a second opening, and a short
//! gap between two rows ([`GRACE`]) does not take it down. And it comes and
//! goes on the HUD's own curves (`hud::motion`), about the corner it hangs
//! from, with nothing written once it stands.
//!
//! The card is never pickable: it takes no click meant for what is under it,
//! and any press puts it away. A name and a nickname are another player's or
//! this player's own text: drawn as plain text, on one line, cut to
//! [`NAME_CHARS`].

use baylee_client_core::i18n::Lang;
use baylee_client_core::social::card::{PersonCard, Relation, own_games};
use baylee_protocol::social::Shown;
use bevy::picking::events::{PointerOut, PointerOver};
use bevy::prelude::*;

use super::SocialHost;
use super::rows::ChatName;
use super::zoom::{z, zpx};
use crate::hud::{UiFonts, motion, tf, tf_bold};
use crate::shellkit::avatar::{self, PersonLink};
use crate::shellkit::tokens;

/// How long the pointer rests on a name before its card opens.
const DWELL: f32 = 0.32;

/// How long the pointer may be off every name before a standing card goes:
/// the gap between two rows of a list.
const GRACE: f32 = 0.12;

/// The card's width.
const WIDTH: f32 = 272.0;

/// The room between the card and what it hangs from.
const GAP: f32 = 8.0;

/// The room the card keeps to the window's edge.
const EDGE: f32 = 8.0;

/// The disc's width.
const DISC: f32 = 56.0;
/// How far the disc's setting stands up into the tint band.
const RISE: f32 = 24.0;

/// The most characters of a name the card draws.
const NAME_CHARS: usize = 28;

/// The card's ground, top-left to bottom-right: the row menu's own.
const GROUND_LIT: Color = Color::srgba(0.10, 0.18, 0.25, 0.98);
const GROUND_DEEP: Color = Color::srgba(0.035, 0.07, 0.10, 0.98);
/// What the disc is set in: the ground where the disc stands.
const GROUND_MID: Color = Color::srgb(0.075, 0.135, 0.19);

/// Who a card is about, and how it hangs.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Subject {
    handle: String,
    guest: bool,
    row: bool,
}

/// The name the pointer is on, and the card that stands.
#[derive(Resource, Default)]
pub(crate) struct PersonHover {
    /// The name under the pointer.
    over: Option<(Entity, Subject)>,
    /// Where the pointer came onto it.
    at: Vec2,
    /// How long the pointer has rested there.
    rested: f32,
    /// How long it has been off every name.
    away: f32,
    /// Whose card stands, and what it hangs from.
    shown: Option<(Subject, Option<Entity>)>,
    /// Bumped whenever `shown` changes.
    epoch: u64,
}

impl PersonHover {
    /// Whether nothing is hovered and nothing stands.
    fn idle(&self) -> bool {
        self.over.is_none() && self.shown.is_none()
    }

    /// The pointer came onto a name.
    fn enter(&mut self, entity: Entity, subject: Subject, at: Vec2) {
        if self.over.as_ref().is_none_or(|(e, _)| *e != entity) {
            self.rested = 0.0;
        }
        self.over = Some((entity, subject));
        self.at = at;
    }

    /// The pointer left the name it was on.
    fn leave(&mut self) {
        self.over = None;
        self.rested = 0.0;
    }

    /// A press anywhere: the card goes, and stays away until the pointer
    /// comes onto a name anew.
    fn dismiss(&mut self) {
        self.leave();
        if self.shown.take().is_some() {
            self.epoch = self.epoch.wrapping_add(1);
        }
    }

    /// One frame of waiting.
    fn tick(&mut self, dt: f32) {
        match (&self.over, &self.shown) {
            // The same person, perhaps named by another node (a list that
            // was rebuilt under the pointer): the card stays as it is.
            (Some((_, subject)), Some((standing, _))) if subject == standing => {
                self.away = 0.0;
            }
            // Another person while a card stands: handed on at once.
            (Some((entity, subject)), Some(_)) => {
                self.shown = Some((subject.clone(), Some(*entity)));
                self.away = 0.0;
                self.epoch = self.epoch.wrapping_add(1);
            }
            (Some((entity, subject)), None) => {
                self.rested += dt;
                if self.rested >= DWELL {
                    self.shown = Some((subject.clone(), Some(*entity)));
                    self.away = 0.0;
                    self.epoch = self.epoch.wrapping_add(1);
                }
            }
            (None, Some(_)) => {
                self.away += dt;
                if self.away >= GRACE {
                    self.shown = None;
                    self.epoch = self.epoch.wrapping_add(1);
                }
            }
            (None, None) => {}
        }
    }
}

/// The card.
#[derive(Component)]
pub(crate) struct PersonPanel {
    epoch: u64,
    /// 0 → 1 while it opens; 0 → 1 again while it leaves.
    t: f32,
    leaving: bool,
    /// The corner that stays still while it grows: −1 is left or top, +1 is
    /// right or bottom.
    corner: Vec2,
}

/// The disc's setting, which comes up a beat after its card.
#[derive(Component)]
pub(crate) struct PanelDisc {
    panel: Entity,
}

/// Installs the card.
pub(crate) fn install(app: &mut App) {
    app.init_resource::<PersonHover>().add_systems(
        Update,
        (hovers, stand)
            .chain()
            .run_if(resource_exists::<Messages<PointerOver>>)
            .run_if(resource_exists::<Messages<PointerOut>>)
            .run_if(resource_exists::<UiFonts>),
    );
}

/// The person an entity names, at or up to six parents above it.
fn subject_of(
    mut entity: Entity,
    links: &Query<&PersonLink>,
    names: &Query<&ChatName>,
    parents: &Query<&ChildOf>,
) -> Option<(Entity, Subject)> {
    for _ in 0..6 {
        if let Ok(link) = links.get(entity) {
            return Some((
                entity,
                Subject {
                    handle: link.handle.clone(),
                    guest: link.guest,
                    row: link.row,
                },
            ));
        }
        if let Ok(name) = names.get(entity) {
            return Some((
                entity,
                Subject {
                    handle: name.handle.clone(),
                    guest: name.guest,
                    row: false,
                },
            ));
        }
        entity = parents.get(entity).ok()?.parent();
    }
    None
}

/// Tracks the name under the pointer, and waits.
#[allow(clippy::too_many_arguments)] // a Bevy system: the pointer and what it lands on
fn hovers(
    time: Res<Time>,
    mut overs: MessageReader<PointerOver>,
    mut outs: MessageReader<PointerOut>,
    links: Query<&PersonLink>,
    names: Query<&ChatName>,
    parents: Query<&ChildOf>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    mut hover: ResMut<PersonHover>,
) {
    if hover.idle() && overs.is_empty() {
        outs.clear();
        return;
    }
    // A rebuild takes the name away under a still pointer, and no `Out`
    // says so.
    if let Some(entity) = hover.over.as_ref().map(|(entity, _)| *entity)
        && !links.contains(entity)
        && !names.contains(entity)
    {
        hover.leave();
    }
    for out in outs.read() {
        let left = subject_of(out.entity, &links, &names, &parents).map(|(e, _)| e);
        if left.is_some() && left == hover.over.as_ref().map(|(e, _)| *e) {
            hover.leave();
        }
    }
    for over in overs.read() {
        if let Some((entity, subject)) = subject_of(over.entity, &links, &names, &parents) {
            hover.enter(entity, subject, over.pointer.position);
        }
    }
    if mouse.is_some_and(|m| m.get_just_pressed().next().is_some()) {
        hover.dismiss();
        return;
    }
    hover.tick(time.delta_secs().min(0.05));
}

/// What the card hangs from.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Anchor {
    /// A laid-out node: its centre and size.
    Box {
        /// The centre.
        centre: Vec2,
        /// The size.
        size: Vec2,
        /// A row of a list: the card stands beside it.
        row: bool,
    },
    /// The pointer.
    Point(Vec2),
}

/// Where the card stands.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Standing {
    left: f32,
    /// From the top, or
    top: Option<f32>,
    /// from the bottom where it stands above its anchor.
    bottom: Option<f32>,
    corner: Vec2,
}

/// Places a card of `size` (its height an estimate) in `canvas`: beside a
/// row, on the side with room; else under its anchor, or over it where the
/// window ends first. Always inside the window's edge.
fn standing(canvas: Vec2, size: Vec2, anchor: Anchor) -> Standing {
    let (centre, half, row) = match anchor {
        Anchor::Box { centre, size, row } => (centre, size * 0.5, row),
        Anchor::Point(at) => (at + Vec2::new(8.0, 4.0), Vec2::new(0.0, 10.0), false),
    };
    let room_x = (canvas.x - size.x - EDGE).max(EDGE);
    let room_y = (canvas.y - size.y - EDGE).max(EDGE);
    if row {
        let before = centre.x - half.x - GAP - size.x;
        let (left, grows) = if before >= EDGE {
            (before, 1.0)
        } else {
            ((centre.x + half.x + GAP).min(room_x), -1.0)
        };
        return Standing {
            left,
            top: Some((centre.y - half.y).clamp(EDGE, room_y)),
            bottom: None,
            corner: Vec2::new(grows, -1.0),
        };
    }
    let left = (centre.x - half.x).clamp(EDGE, room_x);
    let under = centre.y + half.y + GAP;
    if under + size.y <= canvas.y - EDGE {
        Standing {
            left,
            top: Some(under),
            bottom: None,
            corner: Vec2::new(-1.0, -1.0),
        }
    } else {
        Standing {
            left,
            top: None,
            bottom: Some((canvas.y - (centre.y - half.y) + GAP).max(EDGE)),
            corner: Vec2::new(-1.0, 1.0),
        }
    }
}

/// The card's transform at scale `scale`, its `corner` held still.
fn held(scale: f32, corner: Vec2) -> UiTransform {
    UiTransform {
        // A percentage resolves against the node's own size
        // (`hud::motion::from_bottom`), so the corner is exact unmeasured.
        translation: Val2::new(
            Val::Percent(corner.x * 50.0 * (1.0 - scale)),
            Val::Percent(corner.y * 50.0 * (1.0 - scale)),
        ),
        scale: Vec2::splat(scale),
        ..default()
    }
}

/// The disc's scale while its card is `t` into its opening: nothing for the
/// first third, then up with the card's own overshoot.
fn disc_scale(t: f32) -> f32 {
    motion::pop(((t - 0.34) / 0.66).clamp(0.0, 1.0)).max(0.0)
}

/// A name, on one line and no longer than the card.
fn bounded(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).collect();
    if name.chars().count() <= NAME_CHARS {
        return name;
    }
    let mut cut: String = name.chars().take(NAME_CHARS - 1).collect();
    cut.push('…');
    cut
}

/// Opens, hands on and takes down the card.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one panel's whole life
fn stand(
    mut commands: Commands,
    time: Res<Time>,
    hover: Res<PersonHover>,
    host: Res<SocialHost>,
    settings: Option<Res<crate::settings::ClientSettings>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    fonts: Res<UiFonts>,
    mut panels: Query<(Entity, &mut PersonPanel, &mut UiTransform), Without<PanelDisc>>,
    mut discs: Query<(&PanelDisc, &mut UiTransform), Without<PersonPanel>>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
    ui: Option<Res<UiScale>>,
) {
    if panels.is_empty() && hover.shown.is_none() {
        return;
    }
    let still = prefs.as_ref().is_some_and(|p| p.all().reduce_motion);
    let dt = time.delta_secs().min(0.05);
    let (mut stands, mut handed) = (false, false);
    for (entity, mut panel, mut transform) in &mut panels {
        if !panel.leaving && panel.epoch != hover.epoch {
            if hover.shown.is_some() || still {
                // Handed on: the next card stands where this one stood.
                handed = hover.shown.is_some();
                commands.entity(entity).despawn();
                continue;
            }
            panel.leaving = true;
            panel.t = 0.0;
        }
        if panel.leaving {
            panel.t = motion::step(panel.t, motion::ZOOM_OUT, dt, still);
            if panel.t >= 1.0 {
                commands.entity(entity).despawn();
            } else {
                *transform = held(motion::shutting(panel.t), panel.corner);
            }
            continue;
        }
        stands = true;
        if panel.t < 1.0 {
            panel.t = motion::step(panel.t, motion::ZOOM_IN, dt, still);
            *transform = held(motion::opening(panel.t), panel.corner);
            let scale = disc_scale(panel.t);
            for (disc, mut transform) in &mut discs {
                if disc.panel == entity {
                    transform.scale = Vec2::splat(scale);
                }
            }
        }
    }
    let (Some((subject, source)), false) = (&hover.shown, stands) else {
        return;
    };
    let Some(machine) = host.machine() else {
        return;
    };
    let lang = settings.as_ref().map_or(Lang::En, |s| Lang::of(&s.lang));
    let now = super::now_ms();
    let card = machine.person_card(&subject.handle, subject.guest);
    let mut facts: Vec<String> = Vec::new();
    facts.extend(card.relation_words(lang, now));
    facts.extend(card.played_words(lang, now));
    if card.relation == Relation::Me
        && let Some(prefs) = prefs.as_ref()
    {
        let (games, wins) =
            prefs
                .all()
                .deck_stats
                .values()
                .fold((0u32, 0u32), |(games, wins), deck| {
                    (
                        games.saturating_add(deck.games_played),
                        wins.saturating_add(deck.wins),
                    )
                });
        facts.push(own_games(lang, games, wins));
    }
    let presence = card.presence.as_ref().map(|presence| {
        (
            presence.status,
            super::drawer::place_words(presence.status, presence.place.as_ref(), lang),
        )
    });

    let canvas = windows.iter().next().map_or(Vec2::new(1280.0, 800.0), |w| {
        crate::hud::scale::space(w, ui.as_deref())
    });
    let ui_scale = ui.as_deref().map_or(1.0, |scale| scale.0);
    let anchor = source
        .and_then(|source| nodes.get(source).ok())
        .filter(|(node, _)| node.size().x > 0.0 && node.size().y > 0.0)
        .map_or(Anchor::Point(hover.at / ui_scale), |(node, place)| {
            let inv = node.inverse_scale_factor() / ui_scale;
            Anchor::Box {
                centre: place.translation * inv,
                size: node.size() * inv,
                row: subject.row,
            }
        });
    #[allow(clippy::cast_precision_loss)] // a handful of lines
    let height = z(104.0)
        + if presence.is_some() { z(20.0) } else { 0.0 }
        + if facts.is_empty() {
            0.0
        } else {
            z(18.0) + z(22.0) * facts.len() as f32
        };
    let width = z(WIDTH).min(canvas.x - 2.0 * EDGE).max(120.0);
    let at = standing(canvas, Vec2::new(width, height), anchor);
    let t = if handed || still { 1.0 } else { 0.0 };
    build(
        &mut commands,
        &fonts,
        &card,
        presence,
        &facts,
        (at, width),
        PersonPanel {
            epoch: hover.epoch,
            t,
            leaving: false,
            corner: at.corner,
        },
    );
}

/// A presence's dot.
pub(crate) fn presence_ink(shown: Shown) -> Color {
    match shown {
        Shown::Online => Color::srgb(0.420, 0.816, 0.541),
        Shown::Away => crate::hud::palette::CANDLE,
        Shown::Dnd => crate::hud::palette::DANGER,
        Shown::Offline => tokens::MUTED,
    }
}

/// Draws the card.
#[allow(clippy::too_many_lines)] // one panel, top to bottom
fn build(
    commands: &mut Commands,
    fonts: &UiFonts,
    card: &PersonCard,
    presence: Option<(Shown, String)>,
    facts: &[String],
    (at, width): (Standing, f32),
    life: PersonPanel,
) -> Entity {
    let tint = baylee_client_core::avatar::monogram(&card.handle).tint;
    let (_, tint_ink, _) = avatar::palette(tint);
    let [r, g, b] = tint.rim();
    let hue = Color::srgb(r, g, b);
    let opening = life.t;
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(at.left),
                top: at.top.map_or(Val::Auto, Val::Px),
                bottom: at.bottom.map_or(Val::Auto, Val::Px),
                width: Val::Px(width),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(zpx(1.0)),
                border_radius: BorderRadius::all(zpx(12.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            held(motion::opening(opening), life.corner),
            life,
            BackgroundColor(GROUND_DEEP),
            BackgroundGradient::from(LinearGradient::new(
                145.0_f32.to_radians(),
                vec![ColorStop::auto(GROUND_LIT), ColorStop::auto(GROUND_DEEP)],
            )),
            BorderColor::all(hue.with_alpha(0.36)),
            BoxShadow(vec![
                ShadowStyle {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.70),
                    x_offset: Val::Px(0.0),
                    y_offset: zpx(6.0),
                    spread_radius: Val::Px(0.0),
                    blur_radius: zpx(22.0),
                },
                // The person's own tint, as a breath of light round the card.
                ShadowStyle {
                    color: hue.with_alpha(0.14),
                    x_offset: Val::Px(0.0),
                    y_offset: Val::Px(0.0),
                    spread_radius: zpx(1.0),
                    blur_radius: zpx(14.0),
                },
            ]),
            // Over every chat surface wherever it is stacked (`window::Stack`),
            // over a row's menu (60), under the vault's sheet (70).
            GlobalZIndex(super::window::G_PANEL + 65),
        ))
        .id();

    // A band of the person's tint, running out to the right.
    let band = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: zpx(40.0),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundGradient::from(LinearGradient::new(
                100.0_f32.to_radians(),
                vec![
                    ColorStop::auto(hue.with_alpha(0.46)),
                    ColorStop::auto(hue.with_alpha(0.10)),
                ],
            )),
        ))
        .id();

    // The disc, set into the band's foot.
    let disc = avatar::disc(commands, fonts, &card.handle, z(DISC));
    let setting = commands
        .spawn((
            PanelDisc { panel },
            Node {
                width: zpx(DISC + 6.0),
                height: zpx(DISC + 6.0),
                padding: UiRect::all(zpx(3.0)),
                flex_shrink: 0.0,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            UiTransform::from_scale(Vec2::splat(disc_scale(opening))),
            BackgroundColor(GROUND_MID),
        ))
        .add_child(disc)
        .id();
    if let Some((shown, _)) = presence {
        let dot = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: zpx(1.0),
                    bottom: zpx(1.0),
                    width: zpx(15.0),
                    height: zpx(15.0),
                    border: UiRect::all(zpx(2.5)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(presence_ink(shown)),
                BorderColor::all(GROUND_MID),
                ZIndex(1),
            ))
            .id();
        commands.entity(setting).add_child(dot);
    }

    // The name: what this account calls them, and under a nickname who
    // they are.
    let title = card.nickname.as_deref().unwrap_or(&card.name);
    let name = commands
        .spawn((
            Text::new(bounded(title)),
            tf_bold(fonts, z(16.0)),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
        ))
        .id();
    if card.nickname.is_none()
        && let Some(tag) = &card.tag
    {
        let tag = commands
            .spawn((
                TextSpan::new(format!(" #{tag}")),
                tf(fonts, z(12.5)),
                TextColor(tokens::MUTED),
            ))
            .id();
        commands.entity(name).add_child(tag);
    }
    let words = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            flex_shrink: 1.0,
            min_width: Val::Px(0.0),
            overflow: Overflow::clip(),
            // The name begins under the band whatever stands below it; a
            // nickname's third line grows the card downwards.
            margin: UiRect::top(zpx(RISE + 4.0)),
            padding: UiRect::bottom(zpx(3.0)),
            ..default()
        })
        .add_child(name)
        .id();
    if card.nickname.is_some() {
        let handle = commands
            .spawn((
                Text::new(bounded(&card.handle)),
                tf(fonts, z(12.0)),
                TextColor(tokens::MUTED),
                TextLayout::no_wrap(),
            ))
            .id();
        commands.entity(words).add_child(handle);
    }
    if let Some((shown, place)) = &presence {
        let place = commands
            .spawn((
                Text::new(bounded(place)),
                tf(fonts, z(12.5)),
                TextColor(if *shown == Shown::Offline {
                    tokens::MUTED
                } else {
                    presence_ink(*shown)
                }),
                TextLayout::no_wrap(),
            ))
            .id();
        commands.entity(words).add_child(place);
    }
    let head = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::FlexStart,
            column_gap: zpx(10.0),
            margin: UiRect::top(zpx(-RISE)),
            padding: UiRect {
                left: zpx(13.0),
                right: zpx(14.0),
                top: Val::Px(0.0),
                bottom: zpx(12.0),
            },
            ..default()
        })
        .add_children(&[setting, words])
        .id();
    commands.entity(panel).add_children(&[band, head]);

    if !facts.is_empty() {
        let list = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: zpx(5.0),
                    margin: UiRect::horizontal(zpx(14.0)),
                    padding: UiRect::axes(Val::Px(0.0), zpx(10.0)),
                    border: UiRect::top(zpx(1.0)),
                    ..default()
                },
                BorderColor::all(tokens::BORDER.with_alpha(0.32)),
            ))
            .id();
        for fact in facts {
            let line = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: zpx(8.0),
                        ..default()
                    },
                    children![
                        (
                            Node {
                                width: zpx(5.0),
                                height: zpx(5.0),
                                flex_shrink: 0.0,
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(tint_ink.with_alpha(0.85)),
                        ),
                        (
                            Text::new(fact.clone()),
                            tf(fonts, z(12.5)),
                            TextColor(tokens::INK.with_alpha(0.88)),
                        ),
                    ],
                ))
                .id();
            commands.entity(list).add_child(line);
        }
        commands.entity(panel).add_child(list);
    }
    // A card must never eat the click meant for what is under it.
    commands
        .entity(panel)
        .insert_recursive::<Children>(Pickable::IGNORE);
    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(handle: &str) -> Subject {
        Subject {
            handle: handle.into(),
            guest: false,
            row: false,
        }
    }

    /// The card waits before it opens, is handed to the next name without
    /// waiting again, stays across a gap between two rows and goes after it.
    #[test]
    fn a_card_waits_is_handed_on_and_outlasts_a_gap() {
        let (a, b) = (
            Entity::from_raw_u32(1).unwrap(),
            Entity::from_raw_u32(2).unwrap(),
        );
        let mut hover = PersonHover::default();
        hover.enter(a, subject("Alice#0001"), Vec2::ZERO);
        hover.tick(DWELL * 0.5);
        assert!(hover.shown.is_none(), "a pointer passing by opens nothing");
        hover.tick(DWELL * 0.6);
        assert_eq!(
            hover.shown.as_ref().map(|s| s.0.handle.as_str()),
            Some("Alice#0001")
        );
        let opened = hover.epoch;

        // Off the row for less than the grace, then onto the next one.
        hover.leave();
        hover.tick(GRACE * 0.5);
        assert_eq!(hover.epoch, opened, "a gap between rows takes nothing down");
        hover.enter(b, subject("Bob#0002"), Vec2::ZERO);
        hover.tick(0.001);
        assert_eq!(
            hover.shown.as_ref().map(|s| s.0.handle.as_str()),
            Some("Bob#0002")
        );
        assert_ne!(
            hover.epoch, opened,
            "the next name is handed the card at once"
        );
        let handed = hover.epoch;

        // The same person named by another node: nothing changes.
        hover.enter(a, subject("Bob#0002"), Vec2::ZERO);
        hover.tick(0.5);
        assert_eq!(hover.epoch, handed);

        hover.leave();
        hover.tick(GRACE + 0.01);
        assert!(hover.shown.is_none());
        assert!(hover.idle());
    }

    /// A press puts the card away, and the pointer still resting on the
    /// name does not bring it back.
    #[test]
    fn a_press_puts_the_card_away() {
        let a = Entity::from_raw_u32(1).unwrap();
        let mut hover = PersonHover::default();
        hover.enter(a, subject("Alice#0001"), Vec2::ZERO);
        hover.tick(DWELL + 0.01);
        assert!(hover.shown.is_some());
        hover.dismiss();
        hover.tick(1.0);
        assert!(hover.shown.is_none());
    }

    /// Wherever its anchor is, the card stands inside the window; a row has
    /// it beside it, on the side with room; a name near the window's foot
    /// has it above.
    #[test]
    fn the_card_stands_inside_the_window_and_beside_a_row() {
        let canvas = Vec2::new(1280.0, 800.0);
        let size = Vec2::new(272.0, 170.0);
        // A drawer's row at the right edge: the card stands left of it,
        // growing out of its right edge.
        let row = Anchor::Box {
            centre: Vec2::new(1110.0, 300.0),
            size: Vec2::new(320.0, 44.0),
            row: true,
        };
        let at = standing(canvas, size, row);
        assert!((at.left + size.x + GAP - 950.0).abs() < 1e-3);
        assert_eq!(at.corner, Vec2::new(1.0, -1.0));
        // The same row at the left edge: right of it.
        let at = standing(
            canvas,
            size,
            Anchor::Box {
                centre: Vec2::new(170.0, 300.0),
                size: Vec2::new(320.0, 44.0),
                row: true,
            },
        );
        assert!((at.left - (330.0 + GAP)).abs() < 1e-3);
        assert!((at.corner.x + 1.0).abs() < 1e-6);
        // A name at the window's foot: above it, hanging from its bottom.
        let at = standing(canvas, size, Anchor::Point(Vec2::new(400.0, 780.0)));
        assert!(at.top.is_none() && at.bottom.is_some());
        assert!((at.corner.y - 1.0).abs() < 1e-6);
        // Everywhere: inside the edge.
        for x in [0.0, 300.0, 640.0, 1279.0] {
            for y in [0.0, 200.0, 799.0] {
                for row in [false, true] {
                    for anchor in [
                        Anchor::Point(Vec2::new(x, y)),
                        Anchor::Box {
                            centre: Vec2::new(x, y),
                            size: Vec2::new(200.0, 40.0),
                            row,
                        },
                    ] {
                        let at = standing(canvas, size, anchor);
                        assert!(at.left >= EDGE - 1e-3, "{anchor:?}");
                        assert!(at.left + size.x <= canvas.x - EDGE + 1e-3, "{anchor:?}");
                        if let Some(top) = at.top {
                            assert!(top >= EDGE - 1e-3, "{anchor:?}");
                        }
                        if let Some(bottom) = at.bottom {
                            assert!(bottom >= EDGE - 1e-3, "{anchor:?}");
                        }
                    }
                }
            }
        }
    }

    /// The card grows out of the corner it hangs from and ends exactly in
    /// place; the disc waits a beat and ends exactly at its size.
    #[test]
    fn the_card_ends_in_place_and_the_disc_at_its_size() {
        let done = held(motion::opening(1.0), Vec2::new(1.0, -1.0));
        assert!((done.scale - Vec2::ONE).length() < 1e-5);
        assert_eq!(
            done.translation,
            Val2::new(Val::Percent(0.0), Val::Percent(0.0))
        );
        let start = held(motion::opening(0.0), Vec2::new(1.0, -1.0));
        let Val::Percent(x) = start.translation.x else {
            panic!("a percentage of the card's own size");
        };
        assert!(x > 0.0, "the right edge stays: the card is pushed right");
        assert!(disc_scale(0.0).abs() < 1e-6);
        assert!(disc_scale(0.3).abs() < 1e-6);
        assert!((disc_scale(1.0) - 1.0).abs() < 1e-5);
    }

    /// A name is one line and bounded, whatever it holds.
    #[test]
    fn a_name_is_one_bounded_line() {
        assert_eq!(bounded("Alice"), "Alice");
        assert_eq!(bounded("Al\nice\u{7}"), "Alice");
        let long = bounded(&"x".repeat(200));
        assert_eq!(long.chars().count(), NAME_CHARS);
        assert!(long.ends_with('…'));
    }
}
