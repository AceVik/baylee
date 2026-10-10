//! The report's references, on the renderer's side (window B): what `#`
//! offers away from a table, and a finished reference's preview.
//!
//! What may be offered at a table, and how a reference is read back out of
//! the text, is client-core's (`bugreport::refs`), held to the hidden-
//! information rule there and in gamehost's view tests. Here the compiled
//! pool stands in where there is no table — loaded on the first `#`, the
//! deck's own cards first in the builder — and a `[Name]` span, hovered
//! (or held 400 ms under a finger), shows the card's picture.
//!
//! The preview is the report's own, at `GlobalZIndex(1002)`: the table's
//! (`Duel::hovered_log`'s) stands at `G_PREVIEW_OVER_FINISH`, under the
//! sheet's scrim, and the builder's under the lobby's tree; either would be
//! drawn behind the sheet it previews for. The picture is the printing's
//! own, through the one door every picture takes (`images::image_url`).

use baylee_client_core::bugreport::refs::{self, CardRef, RefPrint, Sigil};
use baylee_client_core::images::{ArtSize, Face, image_url};
use bevy::prelude::*;

use super::ReportDesk;
use super::field::ReportLink;
use crate::lobby::LobbyState;
use crate::shellkit::InputClass;

/// How long a finger holds a reference before its preview shows (§B.3).
const HOLD_SECS: f32 = 0.4;

/// The preview's size, in logical pixels: Scryfall's `normal` at half.
const PREVIEW: Vec2 = Vec2::new(244.0, 340.0);

/// A reference the pointer is on (or a finger holds), and where.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ReportHover {
    /// The span, which the pointer leaves by.
    pub(crate) span: Entity,
    /// The card it names.
    pub(crate) card: CardRef,
    /// Where the pointer found it.
    pub(crate) at: Vec2,
    /// When a finger came down on it; `None` under a pointer.
    pub(crate) held_since: Option<f32>,
}

/// The preview's node: one at a time, outside the sheet's tree, keyed on the
/// printing it shows.
#[derive(Component, Debug)]
pub(crate) struct ReportPreview {
    /// The printing's Scryfall id and the face, as `/state` names it.
    pub(crate) key: String,
}

/// The pool's cards as `#` offers them away from a table: no object, no
/// zone, the type line beside the name, the registry's reference printing;
/// the cards of `first` (the deck being built) ahead of the rest.
#[must_use]
pub(crate) fn pool_candidates(first: &[u32]) -> Vec<CardRef> {
    let rows = baylee_cards::pool::rows();
    let card = |row: &baylee_cards::pool::PoolCard| CardRef {
        name: row.name.clone(),
        english: row.english_name.clone(),
        card: baylee_core::ids::CardIndex::new(row.index),
        face: 0,
        art: None,
        print: Some(RefPrint {
            scryfall_id: row.scryfall_id.to_string(),
            lang: "en".to_string(),
            finish: "normal".to_string(),
        }),
        object: None,
        zone: None,
        owner: None,
        kind: Some(row.type_line.clone()),
    };
    let mut out: Vec<CardRef> = first
        .iter()
        .filter_map(|index| rows.iter().find(|row| row.index == *index))
        .map(card)
        .collect();
    out.dedup_by(|a, b| a.card == b.card);
    out.extend(
        rows.iter()
            .filter(|row| !first.contains(&row.index))
            .map(card),
    );
    out
}

/// Loads the pool the first time `#` is typed away from a table (it is
/// compiled in, so this works offline too); in the builder, the deck's
/// cards first.
pub(super) fn fill_the_pool(mut desk: ResMut<ReportDesk>, lobby: Option<Res<LobbyState>>) {
    if !desk.open || desk.at_table || !desk.gathered.refs.cards.is_empty() {
        return;
    }
    let text = desk.form.text.text();
    let typing_a_card =
        refs::trigger(text, desk.form.text.cursor()).is_some_and(|t| t.sigil == Sigil::Card);
    if !typing_a_card {
        return;
    }
    let first: Vec<u32> = lobby
        .as_deref()
        .filter(|l| matches!(l.lobby.screen(), baylee_client_core::lobby::Screen::Build))
        .map(|l| {
            let deck = l.lobby.builder();
            [
                baylee_client_core::deckbuilder::Zone::Main,
                baylee_client_core::deckbuilder::Zone::Side,
            ]
            .into_iter()
            .flat_map(|zone| deck.entries(zone).iter())
            .filter_map(|entry| deck.card(entry.slot).map(|card| card.index))
            .collect()
        })
        .unwrap_or_default();
    desk.gathered.refs.cards = pool_candidates(&first);
}

/// Whether the pointer's messages exist (the picking plugin's), for
/// [`follow_the_links`] to read.
pub(super) fn pointer_messages(
    over: Option<Res<Messages<Pointer<Over>>>>,
    out: Option<Res<Messages<Pointer<Out>>>>,
    press: Option<Res<Messages<Pointer<Press>>>>,
    release: Option<Res<Messages<Pointer<Release>>>>,
) -> bool {
    over.is_some() && out.is_some() && press.is_some() && release.is_some()
}

/// Follows the pointer onto the text's references and off them again, and
/// a finger onto one and up again.
#[allow(clippy::too_many_arguments)] // a Bevy system: one reader per message
pub(super) fn follow_the_links(
    mut overs: MessageReader<Pointer<Over>>,
    mut outs: MessageReader<Pointer<Out>>,
    mut presses: MessageReader<Pointer<Press>>,
    mut releases: MessageReader<Pointer<Release>>,
    links: Query<&ReportLink>,
    input: Option<Res<InputClass>>,
    time: Res<Time>,
    mut desk: ResMut<ReportDesk>,
) {
    let touch = input.is_some_and(|i| *i == InputClass::Touch);
    let mut hover = desk.hover.clone();
    for over in overs.read() {
        if touch {
            continue;
        }
        if let Ok(link) = links.get(over.entity) {
            hover = Some(ReportHover {
                span: over.entity,
                card: link.0.clone(),
                at: over.pointer_location.position,
                held_since: None,
            });
        }
    }
    for press in presses.read() {
        if !touch {
            continue;
        }
        if let Ok(link) = links.get(press.entity) {
            hover = Some(ReportHover {
                span: press.entity,
                card: link.0.clone(),
                at: press.pointer_location.position,
                held_since: Some(time.elapsed_secs()),
            });
        }
    }
    for out in outs.read() {
        if hover
            .as_ref()
            .is_some_and(|h| h.span == out.entity && h.held_since.is_none())
        {
            hover = None;
        }
    }
    if releases.read().count() > 0 && hover.as_ref().is_some_and(|h| h.held_since.is_some()) {
        hover = None;
    }
    // A span that went with a rebuild, or a closed form, shows nothing.
    if !desk.open || hover.as_ref().is_some_and(|h| !links.contains(h.span)) {
        hover = None;
    }
    if desk.hover != hover {
        desk.hover = hover;
    }
}

/// The picture a hover shows, by the reference's printing and face.
fn picture(card: &CardRef) -> Option<(String, Option<String>)> {
    let print = card.print.as_ref()?;
    let entry = baylee_view::PrintEntry {
        scryfall_id: print.scryfall_id.clone(),
        lang: print.lang.clone(),
        finish: baylee_view::Finish::Normal,
    };
    let face = if card.face == 0 {
        Face::Front
    } else {
        Face::Back
    };
    let key = format!("{}#{}", print.scryfall_id, card.face);
    Some((key, image_url(&entry, face, ArtSize::Normal)))
}

/// Stands the hovered reference's picture beside the pointer, over the
/// sheet; takes it down when the pointer leaves.
pub(super) fn show_the_preview(
    mut commands: Commands,
    desk: Res<ReportDesk>,
    time: Res<Time>,
    assets: Option<Res<AssetServer>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    shown: Query<(Entity, &ReportPreview)>,
    ui: Option<Res<UiScale>>,
) {
    let wanted = desk.hover.as_ref().filter(|h| {
        h.held_since
            .is_none_or(|since| time.elapsed_secs() - since >= HOLD_SECS)
    });
    let wanted = wanted.and_then(|h| picture(&h.card).map(|p| (p, h.at)));
    let key = wanted.as_ref().map(|((key, _), _)| key.as_str());
    let current: Vec<(Entity, &ReportPreview)> = shown.iter().collect();
    if current.len() == usize::from(key.is_some())
        && current.iter().all(|(_, p)| Some(p.key.as_str()) == key)
    {
        return;
    }
    for (entity, _) in current {
        commands.entity(entity).despawn();
    }
    let Some(((key, url), at)) = wanted else {
        return;
    };
    // In the UI's units: over the table they are the window's over its scale.
    let scale = ui.as_deref().map_or(1.0, |ui| ui.0.max(f32::EPSILON));
    let at = at / scale;
    let window = windows.single().map_or(Vec2::new(1280.0, 800.0), |w| {
        Vec2::new(w.width(), w.height()) / scale
    });
    // Right of the pointer where it fits, else left; never off the window.
    let left = if at.x + 16.0 + PREVIEW.x <= window.x {
        at.x + 16.0
    } else {
        (at.x - 16.0 - PREVIEW.x).max(0.0)
    };
    let top = (at.y - PREVIEW.y / 2.0).clamp(0.0, (window.y - PREVIEW.y).max(0.0));
    let mut node = commands.spawn((
        ReportPreview { key },
        Node {
            position_type: PositionType::Absolute,
            left: px(left),
            top: px(top),
            width: px(PREVIEW.x),
            height: px(PREVIEW.y),
            border_radius: BorderRadius::all(px(12)),
            ..default()
        },
        BackgroundColor(crate::hud::palette::PANEL),
        GlobalZIndex(1002),
        Pickable::IGNORE,
    ));
    if let (Some(assets), Some(url)) = (assets, url) {
        node.insert(ImageNode::new(assets.load(url)));
    }
}
