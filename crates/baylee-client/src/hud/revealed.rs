//! The sheet that holds up cards another seat revealed (owner, 07.10.2026).
//!
//! `baylee_client_core::reveals` decides which cards and for how long; this
//! draws [`Reveals::current`] and nothing else. The cards are drawn the way
//! the log's own links preview them (#300): from the printing the line named
//! ([`LogLink::art`]) and not from an object on the table, because a card
//! revealed out of a library or a hand is no object this seat's view holds.
//!
//! Since 08.10.2026 (the owner: *"the same sheet style as discard/target
//! selection"*) it is the decision sheet's paper with the sheet's own parts
//! (`ledge::drawer::sheet`): a head saying *"Bo reveals"* and how many more
//! wait, with the fold and a close cross; the cards large, each carrying the
//! log's own [`LogLink`] so the pointer on one opens the table's preview of
//! that printing; and a foot with the close answer and `Esc`'s cap. The
//! fold (`Duel::reveal_fold`, keyed on the reveal's number) folds it to the
//! sheet's pill over the shelf, and the next reveal stands up open.
//!
//! Since 10.10.2026 (the owner: *"like the target selection for a cast, it
//! should move down into the decision area … and must not close by itself"*)
//! it stands where the decision sheet does, grown out of the shelf at the
//! bottom (`ledge::drawer::root_node`), open at its foot, its shadow cast
//! upward, on the log's rung ([`Z_LOG`]). It has no clock: it stays until
//! the cross, the foot's answer or `Esc` (`input::answering`'s ladder) put
//! it away. While a decision sheet of this seat's own stands in that place
//! the reveal steps aside, still waiting, and comes back once the question is
//! answered: it never covers what this seat has to answer, and a zone dialog
//! and the hover preview stand over it. It does not move, so `reduce_motion`
//! has nothing to hold still; nothing is drawn on a print, and every card
//! shows its own picture whole.
//!
//! [`Reveals::current`]: baylee_client_core::reveals::Reveals::current

use super::{
    EDGE, HudRoot, LogLink, MenuAction, TOP_CLEAR, UiFonts, Z_LOG, card_radius, palette,
    sheet_radius, spawn_card_art,
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

/// The head's height, kept out of the cards' room.
const HEAD_H: f32 = 34.0;

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

/// The folded reveal: a press opens it again.
#[derive(Component)]
pub struct RevealPill;

/// What the sheet was last drawn from.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub struct RevealRevision {
    /// Which reveal, by its number; `None` for none.
    number: Option<u64>,
    /// How many wait behind it, which the head says.
    waiting: usize,
    /// Folded to its pill.
    folded: bool,
    /// Stepped aside for a decision sheet standing in its place.
    yielding: bool,
    lang: Option<Lang>,
    /// The window, in whole pixels.
    window: (i32, i32),
    /// The textures' arrivals, so a card whose picture lands is drawn again.
    arrivals: u64,
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
    settings: Res<crate::settings::ClientSettings>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
) {
    // Nothing over the end screen: it shows the whole log, this line too.
    let yielding = super::ledge::drawer::sheet_asked(&duel);
    let shown = duel
        .reveals
        .current()
        .filter(|_| duel.ending().is_none() && !yielding);
    let window = windows.single().map_or(Vec2::new(1280.0, 720.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    let lang = Lang::of(&settings.lang);
    let next = RevealRevision {
        number: shown.map(|r| r.number),
        waiting: duel.reveals.waiting(),
        folded: shown.is_some_and(|r| duel.reveal_fold.is_folded(Some(r.number))),
        yielding,
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
            folded: next.folded,
            cancel_cap: prefs.as_deref().and_then(|p| {
                p.keymap()
                    .chords(baylee_client_core::prefs::Action::Cancel)
                    .first()
                    .map(baylee_client_core::prefs::Chord::display)
            }),
        },
        &fonts,
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
    /// Folded to its pill.
    folded: bool,
    /// `Esc`'s cap, for the foot's close.
    cancel_cap: Option<String>,
}

/// The sheet: a full-width band over the shelf, where the decision sheet
/// stands, that the pointer passes through,
/// and in it the decision sheet's own paper — head ("Bo reveals", how many
/// more wait, the fold and the close cross), the cards, and a foot with the
/// close answer and `Esc`'s cap. Folded, the band holds the sheet's pill.
fn spawn_sheet(
    commands: &mut Commands,
    paper: &Paper<'_>,
    fonts: &UiFonts,
    card: &mut dyn FnMut(&mut Commands, LogLink, f32) -> Entity,
) -> Entity {
    use super::ledge::drawer::sheet;
    let band = commands
        .spawn((
            RevealSheet,
            super::ledge::drawer::root_node(),
            ZIndex(Z_LOG),
            Pickable::IGNORE,
        ))
        .id();
    let head = sheet::Head {
        title: Phrase::RevealedBy.fill(paper.lang, &[&paper.name]),
        detail: if paper.waiting > 0 {
            vec![Phrase::RevealedWaiting.fill(paper.lang, &[&paper.waiting.to_string()])]
        } else {
            Vec::new()
        },
        source: None,
        folded: paper.folded,
    };
    if paper.folded {
        let pill = sheet::spawn_pill(
            commands,
            fonts,
            &head,
            None,
            (MenuAction::FoldReveal, None),
            UiRect::ZERO,
        );
        commands.entity(pill).insert(RevealPill);
        commands.entity(band).add_child(pill);
        return band;
    }
    let room = room(paper.window);
    let (width, across) = fit(paper.reveal.cards.len(), room, GAP, WIDEST);
    #[allow(clippy::cast_precision_loss)] // a handful of cards
    let row_w = across as f32 * width + (across.saturating_sub(1)) as f32 * GAP;
    let page = commands
        .spawn((
            Node {
                width: px(row_w + 2.0 * PAD),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(px(PAD)),
                row_gap: px(GAP),
                // Open at the foot, as the decision sheet is: it grows out of
                // the shelf rather than standing on it.
                border: UiRect {
                    bottom: px(0),
                    ..UiRect::all(px(1))
                },
                border_radius: BorderRadius {
                    bottom_left: px(0),
                    bottom_right: px(0),
                    ..sheet_radius()
                },
                ..default()
            },
            BackgroundColor(palette::DOCK_GROUND),
            BorderColor::all(palette::DOCK_EDGE),
            // Upward, onto the table, as the decision sheet's.
            BoxShadow(vec![ShadowStyle {
                color: palette::SHADOW,
                x_offset: px(0.0),
                y_offset: px(-6.0),
                spread_radius: px(0.0),
                blur_radius: px(18.0),
            }]),
        ))
        .id();
    let top = sheet::spawn_head(
        commands,
        fonts,
        &head,
        None,
        (MenuAction::FoldReveal, None),
        Some(MenuAction::DismissReveal),
    );
    let grid = spawn_grid(commands, paper, (width, row_w), card);
    let foot = spawn_foot(commands, paper, fonts, row_w);
    commands.entity(page).add_children(&[top, grid, foot]);
    commands.entity(band).add_child(page);
    band
}

/// The cards, large, each with the log's own link on it.
fn spawn_grid(
    commands: &mut Commands,
    paper: &Paper<'_>,
    (width, row_w): (f32, f32),
    card: &mut dyn FnMut(&mut Commands, LogLink, f32) -> Entity,
) -> Entity {
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
        let link = LogLink {
            card: shown.card,
            token: shown.token,
        };
        let picture = card(commands, link, width);
        commands.entity(picture).insert(RevealCard);
        // On top of the picture, the log's own link: the pointer on it
        // opens the table's large preview of that printing, as a [link] in
        // the log does (`hover_log_links`), and a press answers nothing.
        let lens = commands
            .spawn((
                link,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ))
            .id();
        commands.entity(picture).add_child(lens);
        commands.entity(grid).add_child(picture);
    }
    grid
}

/// The foot: the close answer with `Esc`'s cap.
fn spawn_foot(commands: &mut Commands, paper: &Paper<'_>, fonts: &UiFonts, row_w: f32) -> Entity {
    use super::ledge::drawer::sheet;
    let foot = commands
        .spawn((
            Node {
                width: px(row_w),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let close = sheet::footer_button(
        commands,
        fonts,
        MenuAction::DismissReveal,
        Phrase::ShellClose.text(paper.lang),
        paper.cancel_cap.as_deref(),
    );
    commands.entity(foot).add_child(close);
    foot
}

/// The room the cards have in a window this big: across, at most
/// [`MOST_ACROSS`] of it inside the margins; down, from under the report
/// button's corner to the hand zone, less the paper and its head.
pub(crate) fn room(window: Vec2) -> (f32, f32) {
    let across = (window.x * MOST_ACROSS).min(window.x - 2.0 * EDGE) - 2.0 * PAD;
    let down = window.y
        - TOP_CLEAR
        - super::HAND_ZONE_H
        - EDGE
        - 2.0 * PAD
        - HEAD_H
        - super::ledge::drawer::sheet::FOOT_H
        - 2.0 * GAP;
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
