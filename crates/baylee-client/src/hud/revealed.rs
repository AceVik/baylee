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
//! Since 12.10.2026 (the owner: *"the revealed cards view, the hover preview
//! flickers then"*) it survives the overlay's rebuilds (`OverlayTree::reveal`):
//! the overlay swept it away on every rebuild, and the preview a pointer on
//! one of its cards opens *is* a rebuild, so the sheet stood up again, the
//! card under the pointer was a new entity, the preview closed, the pointer
//! found the new card, and round again while it rested. Its own gate redraws
//! it for its own pictures only ([`RevealRevision`]'s `arrivals`). The card
//! under the pointer wears the candle edge ([`light_the_hovered`]) and its
//! preview opens at that card's edge ([`CardLens`]); the head says how to
//! enlarge a card while nothing waits, and the foot's answer says what it
//! does: *Next* while another reveal waits, *Done* for the last.
//!
//! [`Reveals::current`]: baylee_client_core::reveals::Reveals::current

use super::{
    CardLens, EDGE, HudRoot, LogLink, MenuAction, TOP_CLEAR, UiFonts, Z_LOG, card_radius, palette,
    sheet_radius, spawn_card_art,
};
use crate::Duel;
use crate::cardmat::{CardLook, CardUiMaterial, UiCardMaterials, UiCards, finish_of};
use crate::textures::CardTextures;
use baylee_client_core::i18n::{Lang, Phrase, seat_name};
use baylee_client_core::images::{ArtSize, ImageKey};
use baylee_client_core::reveals::{Reveal, fit};
use bevy::prelude::*;

/// The widest a revealed card is drawn, in logical pixels: big enough that
/// its rules text reads, which is the point of showing it.
pub(crate) const WIDEST: f32 = 240.0;

/// The most of the window's width the sheet takes.
const MOST_ACROSS: f32 = 0.8;

/// The paper round the cards and their side rail.
const PAD: f32 = 16.0;

/// Between two cards, and between the cards and their rail.
const GAP: f32 = 14.0;

/// The right-hand rail: title, guidance and the way through the reveals.
/// Keeping those beside rather than above and below the cards leaves the
/// cards' whole height for the cards themselves.
const RAIL_W: f32 = 224.0;

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
    /// How many of its own cards' pictures have arrived, so a card whose
    /// picture lands is drawn again — and only then. It was the textures'
    /// whole epoch, which moves whenever any picture anywhere lands or is
    /// evicted, and every move stood the sheet up again under the pointer.
    arrivals: usize,
    /// What a text face is drawn from: the card texts' generation, the held
    /// face key and the text-view setting.
    faces: (u64, bool, bool),
}

/// What draws a revealed card's text face where its art is not there (the
/// table's own fallback, `face::wants_face`): an art-less gateway, the text
/// view, the held key, or art still in flight.
#[derive(bevy::ecs::system::SystemParam)]
pub struct RevealFaces<'w> {
    texts: Option<Res<'w, crate::cardtext::CardTexts>>,
    mode: Option<Res<'w, crate::face::FaceMode>>,
}

impl RevealFaces<'_> {
    fn revision(&self, settings: &crate::settings::ClientSettings) -> (u64, bool, bool) {
        (
            self.texts
                .as_deref()
                .map_or(0, crate::cardtext::CardTexts::generation),
            self.mode.as_deref().is_some_and(|m| m.held),
            settings.prefer_text_view,
        )
    }

    /// The face to draw for `link` instead of its art, or `None` for the art.
    fn face(
        &self,
        link: &LogLink,
        settings: &crate::settings::ClientSettings,
        textures: &CardTextures,
        key: ImageKey,
    ) -> Option<baylee_client_core::card_face::CardFace> {
        let held = crate::face::FaceMode::default();
        let mode = self.mode.as_deref().unwrap_or(&held);
        let card = link.card?;
        let texts = self.texts.as_deref()?;
        crate::face::wants_face(mode, settings, textures, Some(key))
            .then(|| crate::face::of_identity(card.index, card.face, texts))
            .flatten()
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
    ui: Option<Res<UiScale>>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    material_assets: Option<ResMut<Assets<CardUiMaterial>>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    faces: RevealFaces,
) {
    // Nothing over the end screen: it shows the whole log, this line too.
    let yielding = super::ledge::drawer::sheet_asked(&duel);
    let shown = duel
        .reveals
        .current()
        .filter(|_| duel.ending().is_none() && !yielding);
    let window = crate::hud::scale::window_space(&windows, ui.as_deref())
        .unwrap_or(Vec2::new(1280.0, 720.0));
    let lang = Lang::of(&settings.lang);
    let next = RevealRevision {
        number: shown.map(|r| r.number),
        waiting: duel.reveals.waiting(),
        folded: shown.is_some_and(|r| duel.reveal_fold.is_folded(Some(r.number))),
        yielding,
        lang: Some(lang),
        window: (window.x as i32, window.y as i32),
        arrivals: shown.map_or(0, |r| arrived(r, &textures)),
        faces: faces.revision(&settings),
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
            let built = faces.face(&link, &settings, &textures, key);
            let image = textures.get(key, statics, &assets);
            let card = spawn_card_art(
                commands,
                lang,
                image,
                built.as_ref(),
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

/// How many of `reveal`'s pictures are on the GPU.
fn arrived(reveal: &Reveal, textures: &CardTextures) -> usize {
    reveal
        .cards
        .iter()
        .filter_map(|shown| {
            LogLink {
                card: shown.card,
                token: shown.token,
            }
            .art(ArtSize::Normal)
        })
        .filter(|key| textures.has_arrived(*key))
        .count()
}

/// Lights the revealed card the pointer is on (`Duel::hovered_log`), so the
/// player sees which card the preview beside it is of: the candle edge a
/// zone dialog's tile wears, on the picture's own edge. Written only where
/// it changes, so a resting pointer costs nothing.
pub fn light_the_hovered(
    mut commands: Commands,
    duel: Res<Duel>,
    lenses: Query<&ChildOf, With<CardLens>>,
    cards: Query<(Entity, Has<Outline>), With<RevealCard>>,
) {
    let lit = duel
        .hovered_log
        .and_then(|hover| lenses.get(hover.span).ok())
        .map(ChildOf::parent);
    for (card, outlined) in &cards {
        let want = lit == Some(card);
        if want && !outlined {
            commands
                .entity(card)
                .insert(Outline::new(px(2), px(2), palette::CANDLE));
        } else if !want && outlined {
            commands.entity(card).remove::<Outline>();
        }
    }
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
            vec![Phrase::RevealedHint.text(paper.lang).to_string()]
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
    let rows = paper.reveal.cards.len().max(1).div_ceil(across);
    #[allow(clippy::cast_precision_loss)] // a handful of cards
    let (across_f, rows_f) = (across as f32, rows as f32);
    let row_w = across_f * width + (across_f - 1.0) * GAP;
    let grid_h = rows_f * width * 88.0 / 63.0 + (rows_f - 1.0) * GAP;
    let page = commands
        .spawn((
            Node {
                width: px(row_w + RAIL_W + GAP + 2.0 * PAD),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(GAP),
                padding: UiRect::all(px(PAD)),
                // Open at the foot, as the decision sheet is: it grows out of
                // the shelf rather than standing on it.
                border: UiRect {
                    bottom: px(0),
                    ..UiRect::all(px(1))
                },
                border_radius: BorderRadius {
                    bottom_left: px(0).into(),
                    bottom_right: px(0).into(),
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
    let grid = spawn_grid(commands, paper, (width, row_w, grid_h), card);
    let top = sheet::spawn_head(
        commands,
        fonts,
        &head,
        None,
        (MenuAction::FoldReveal, None),
        Some(MenuAction::DismissReveal),
    );
    let foot = spawn_foot(commands, paper, fonts);
    let rail = commands
        .spawn((
            Node {
                width: px(RAIL_W),
                min_width: px(RAIL_W),
                min_height: px(grid_h),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                row_gap: px(GAP),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_children(&[top, foot]);
    commands.entity(page).add_children(&[grid, rail]);
    commands.entity(band).add_child(page);
    band
}

/// The cards, large, each with the log's own link on it.
fn spawn_grid(
    commands: &mut Commands,
    paper: &Paper<'_>,
    (width, row_w, height): (f32, f32, f32),
    card: &mut dyn FnMut(&mut Commands, LogLink, f32) -> Entity,
) -> Entity {
    let grid = commands
        .spawn((
            Node {
                width: px(row_w),
                height: px(height),
                flex_shrink: 0.0,
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
                CardLens,
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

/// The foot: the close answer with `Esc`'s cap, across the side rail.
fn spawn_foot(commands: &mut Commands, paper: &Paper<'_>, fonts: &UiFonts) -> Entity {
    use super::ledge::drawer::sheet;
    let foot = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::top(px(10)),
                border: UiRect::top(px(1)),
                ..default()
            },
            BorderColor::all(palette::DOCK_EDGE),
            Pickable::IGNORE,
        ))
        .id();
    // Says what pressing it does: the next reveal while one waits, else
    // done with this one. "Close" read as a window being shut on the game.
    let label = if paper.waiting > 0 {
        Phrase::RevealNext
    } else {
        Phrase::RevealDone
    };
    let close = sheet::footer_button(
        commands,
        fonts,
        MenuAction::DismissReveal,
        label.text(paper.lang),
        paper.cancel_cap.as_deref(),
    );
    commands
        .entity(close)
        .entry::<Node>()
        .and_modify(|mut node| node.width = percent(100));
    commands.entity(foot).add_child(close);
    foot
}

/// The room the cards have in a window this big: across, at most
/// [`MOST_ACROSS`] of it inside the margins, less their own rail and the
/// paper; down, from under the report button's corner to the hand zone, less
/// only the paper. Head and foot stand beside the cards, not above and below.
pub(crate) fn room(window: Vec2) -> (f32, f32) {
    let across = (window.x * MOST_ACROSS).min(window.x - 2.0 * EDGE) - 2.0 * PAD - RAIL_W - GAP;
    let down = window.y - TOP_CLEAR - super::HAND_ZONE_H - EDGE - 2.0 * PAD;
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
