//! The sheet that holds up cards another seat revealed (owner, 07.10.2026).
//!
//! `baylee_client_core::reveals` decides which cards and for how long; this
//! draws [`Reveals::current`] and nothing else. The cards are drawn the way
//! the log's own links preview them (#300): from the printing the line named
//! ([`LogLink::art`]) and not from an object on the table, because a card
//! revealed out of a library or a hand is no object this seat's view holds.
//! The paper is the preview's slip, parchment with the slip's inks, since a
//! reveal is something read rather than worked in.
//!
//! It stands at the top of the window, centred, on the log's rung
//! ([`Z_LOG`]): a sheet only showing the game. A zone dialog answering a
//! question stands over it and the hover preview over that, so it never
//! covers what this seat has to answer, and no key but `Esc` is its. A press
//! on it puts it away, and so does `Esc` (`input::answering`'s ladder). It
//! does not move, so `reduce_motion` has nothing to hold still; nothing is
//! drawn on a print, and every card shows its own picture whole.
//!
//! [`Reveals::current`]: baylee_client_core::reveals::Reveals::current

use super::{
    EDGE, HudRoot, LogLink, MenuAction, MenuButton, TOP_CLEAR, UiFonts, UiSheets, Z_LOG,
    card_radius, glyph, icon_tf, palette, sheet_radius, sheet_shadow, sheet_surface,
    spawn_card_art, tf, tf_bold,
};
use crate::Duel;
use crate::cardmat::{CardLook, CardUiMaterial, UiCardMaterials, UiCards, finish_of};
use crate::textures::CardTextures;
use baylee_client_core::i18n::{Lang, Phrase, seat_name};
use baylee_client_core::images::ArtSize;
use baylee_client_core::reveals::{Reveal, fit};
use bevy::prelude::*;

/// The widest a revealed card is drawn, in logical pixels: big enough that
/// its rules text reads, which is the point of showing it.
pub(crate) const WIDEST: f32 = 240.0;

/// The most of the window's width the sheet takes.
const MOST_ACROSS: f32 = 0.8;

/// The paper round the cards.
const PAD: f32 = 12.0;

/// Between two cards.
const GAP: f32 = 10.0;

/// The head's text size.
const HEAD_PT: f32 = 13.0;

/// The head's height, kept out of the cards' room.
const HEAD_H: f32 = 26.0;

/// The rung is the claim, so it is checked where it is made: a sheet only
/// showing the game stands under a dialog answering a question and under the
/// hover preview.
const _: () = assert!(
    Z_LOG < super::Z_SHEET && Z_LOG < super::Z_PREVIEW,
    "a reveal never stands over what this seat has to answer"
);

/// The sheet.
#[derive(Component)]
pub struct RevealSheet;

/// One card on it, for whoever counts them (a test, `/state`).
#[derive(Component)]
pub struct RevealCard;

/// What the sheet was last drawn from.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RevealRevision {
    /// Which reveal, by its number; `None` for none.
    number: Option<u64>,
    /// How many wait behind it, which the head says.
    waiting: usize,
    lang: Option<Lang>,
    /// The window, in whole pixels.
    window: (i32, i32),
    /// The textures' arrivals, so a card whose picture lands is drawn again.
    arrivals: u64,
}

/// Runs the reveals' clock on the game's own time, so a paused clock
/// (`dev-control`'s `/pause`) holds a reveal up as long as it is paused.
pub fn tick(time: Res<Time>, mut duel: ResMut<Duel>) {
    let now = time.elapsed_secs_f64();
    // Asked first, through `Res`'s side of the borrow: a write every frame
    // would mark the duel changed on every frame for nothing.
    if duel.reveals.due(now) {
        duel.reveals.tick(now);
    }
}

/// Draws the reveal standing, and takes the sheet away when none is.
#[allow(clippy::too_many_arguments)] // a Bevy system: every one is an injection
pub fn sync(
    mut commands: Commands,
    duel: Res<Duel>,
    roots: Query<Entity, With<HudRoot>>,
    standing: Query<Entity, With<RevealSheet>>,
    mut revision: ResMut<RevealRevision>,
    mut textures: ResMut<CardTextures>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    fonts: Res<UiFonts>,
    sheets: Option<Res<UiSheets>>,
    settings: Res<crate::settings::ClientSettings>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
) {
    // Nothing over the end screen: it shows the whole log, this line too.
    let shown = duel.reveals.current().filter(|_| duel.ending().is_none());
    let window = windows.single().map_or(Vec2::new(1280.0, 720.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let lang = Lang::of(&settings.lang);
    #[allow(clippy::cast_possible_truncation)] // a window's size in pixels
    let next = RevealRevision {
        number: shown.map(|r| r.number),
        waiting: duel.reveals.waiting(),
        lang: Some(lang),
        window: (window.x as i32, window.y as i32),
        arrivals: if shown.is_some() { textures.epoch() } else { 0 },
    };
    if *revision == next && standing.is_empty() == shown.is_none() {
        return;
    }
    *revision = next;
    for sheet in &standing {
        commands.entity(sheet).despawn();
    }
    let (Some(reveal), Some(statics), Ok(root)) = (shown, duel.statics.as_ref(), roots.single())
    else {
        return;
    };
    let mut cards = match (ui_materials, material_assets) {
        (Some(cache), Some(assets)) => Some((cache, assets)),
        _ => None,
    };
    let mut cards = cards.as_mut().map(|(cache, assets)| UiCards {
        cache: cache.as_mut(),
        assets: assets.as_mut(),
    });
    let sheet = spawn_sheet(
        &mut commands,
        &Paper {
            reveal,
            waiting: duel.reveals.waiting(),
            name: seat_name(lang, Some(statics), reveal.player),
            lang,
            window,
        },
        &fonts,
        sheets.as_deref(),
        &mut |commands, link, width| {
            let height = width * 88.0 / 63.0;
            let Some(key) = link.art(ArtSize::Normal) else {
                return blank(commands, width, height);
            };
            let image = textures.get(key, statics, &assets);
            let card = spawn_card_art(
                commands,
                lang,
                image,
                None,
                width,
                height,
                crate::face::Detail::Full,
                &fonts,
                CardLook::art(key, finish_of(statics, Some(key))),
                cards.as_mut(),
                &crate::face::Widths::of(None),
            );
            commands.entity(card).insert((
                Node {
                    width: px(width),
                    height: px(height),
                    flex_shrink: 0.0,
                    border_radius: card_radius(width),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            card
        },
    );
    commands.entity(root).add_child(sheet);
}

/// What one sheet says.
struct Paper<'a> {
    reveal: &'a Reveal,
    waiting: usize,
    name: String,
    lang: Lang,
    window: Vec2,
}

/// The sheet: a full-width band at the top that the pointer passes through,
/// and in it the paper, which is a button that puts it away.
fn spawn_sheet(
    commands: &mut Commands,
    paper: &Paper<'_>,
    fonts: &UiFonts,
    sheets: Option<&UiSheets>,
    card: &mut dyn FnMut(&mut Commands, LogLink, f32) -> Entity,
) -> Entity {
    let room = room(paper.window);
    let (width, across) = fit(paper.reveal.cards.len(), room, GAP, WIDEST);
    #[allow(clippy::cast_precision_loss)] // a handful of cards
    let row_w = across as f32 * width + (across.saturating_sub(1)) as f32 * GAP;
    let band = commands
        .spawn((
            RevealSheet,
            Node {
                position_type: PositionType::Absolute,
                top: px(TOP_CLEAR),
                left: px(0),
                right: px(0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            ZIndex(Z_LOG),
            Pickable::IGNORE,
        ))
        .id();
    let page = commands
        .spawn((
            Button,
            MenuButton {
                action: MenuAction::DismissReveal,
            },
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(PAD)),
                row_gap: px(GAP),
                border_radius: sheet_radius(),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT),
            sheet_shadow(),
        ))
        .id();
    if let Some(sheets) = sheets {
        commands.entity(page).with_child(sheet_surface(sheets));
    }
    let head = head(commands, paper, fonts, row_w);
    let grid = commands
        .spawn((
            Node {
                width: px(row_w),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(GAP),
                row_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for shown in &paper.reveal.cards {
        let picture = card(
            commands,
            LogLink {
                card: shown.card,
                token: shown.token,
            },
            width,
        );
        commands.entity(picture).insert(RevealCard);
        commands.entity(grid).add_child(picture);
    }
    commands.entity(page).add_children(&[head, grid]);
    commands.entity(band).add_child(page);
    band
}

/// "Bo reveals", how many more wait, and the cross.
fn head(commands: &mut Commands, paper: &Paper<'_>, fonts: &UiFonts, width: f32) -> Entity {
    let who = commands
        .spawn((
            Text::new(Phrase::RevealedBy.fill(paper.lang, &[&paper.name])),
            tf_bold(fonts, HEAD_PT),
            TextColor(palette::SLIP_INK),
            Pickable::IGNORE,
        ))
        .id();
    let row = commands
        .spawn((
            Node {
                width: px(width),
                height: px(HEAD_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_child(who)
        .id();
    if paper.waiting > 0 {
        let more = commands
            .spawn((
                Text::new(Phrase::RevealedWaiting.fill(paper.lang, &[&paper.waiting.to_string()])),
                tf(fonts, HEAD_PT - 1.0),
                TextColor(palette::SLIP_SOFT),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(row).add_child(more);
    }
    let gap = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let cross = commands
        .spawn((
            Text::new(glyph::CLOSE.to_string()),
            icon_tf(fonts, HEAD_PT - 2.0),
            TextColor(palette::SLIP_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(&[gap, cross]);
    row
}

/// The room the cards have in a window this big: across, at most
/// [`MOST_ACROSS`] of it inside the margins; down, from under the report
/// button's corner to the hand zone, less the paper and its head.
pub(crate) fn room(window: Vec2) -> (f32, f32) {
    let across = (window.x * MOST_ACROSS).min(window.x - 2.0 * EDGE) - 2.0 * PAD;
    let down = window.y - TOP_CLEAR - super::HAND_ZONE_H - EDGE - 2.0 * PAD - HEAD_H - GAP;
    (across.max(0.0), down.max(0.0))
}

/// A card with no picture to draw: a token whose art is not in this build.
fn blank(commands: &mut Commands, width: f32, height: f32) -> Entity {
    commands
        .spawn((
            Node {
                width: px(width),
                height: px(height),
                flex_shrink: 0.0,
                border_radius: card_radius(width),
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            Pickable::IGNORE,
        ))
        .id()
}

#[cfg(test)]
mod tests;
