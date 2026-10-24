//! One line of a list (§7): a pool row, a deck row, a deck section's
//! heading. Every cell is one line, cut and never wrapped, inside a row of a
//! fixed height that clips — so the row's height is the pitch whatever its
//! words are, and two rows can never overlap (§10 #6).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::{hover_of_card, hover_of_entry, print_mark};
use baylee_client_core::deckbuilder::PoolCard;

mod deck;
mod pool;
pub(crate) use deck::deck_row;
pub(crate) use pool::pool_row;

/// A pool row's place in the shown results, for the cursor's highlight.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct PoolRowAt(pub(crate) usize);

/// A deck row's place in the drawn list, for the cursor's highlight.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct DeckRowAt(pub(crate) usize);

/// The count badge on a pool row: how many of this card the deck holds,
/// kept by `virtual_rows::in_deck` without redrawing the row.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct InDeck(pub(crate) usize);

/// The ground a row rests at.
pub(crate) const ROW_GROUND: Color = Color::srgba(1.0, 1.0, 1.0, 0.03);
/// The ground a row is lit to under the pointer.
pub(crate) const ROW_HOT: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);

/// A pool row's second line: type line · P/T · first keyword — never rules
/// text (§7: the Huntmaster reads `Creature — Human Werewolf // Creature —
/// Werewolf · 2/2 // 4/4 · Transform`).
#[must_use]
pub(crate) fn subline(card: &PoolCard) -> String {
    let mut parts = vec![card.type_line.clone()];
    if let Some(stats) = card.stats.as_ref().filter(|s| !s.is_empty()) {
        parts.push(stats.clone());
    }
    if let Some(keyword) = first_keyword(card) {
        parts.push(keyword);
    }
    parts.join(" \u{b7} ")
}

/// The card's first keyword, read off its front face's keyword lines in the
/// card's own words (the gateway's, else the compiled English Oracle): the
/// row stands for the face it is filed under, so a Huntmaster does not read
/// as its back face's Trample.
fn first_keyword(card: &PoolCard) -> Option<String> {
    let oracle: String = if card.oracle_text.is_empty() {
        baylee_cards::generated_oracle::ORACLE
            .get(card.index as usize)
            .and_then(|faces| faces.first())
            .map_or_else(String::new, |face| (*face).to_owned())
    } else {
        card.oracle_text
            .split("\n//")
            .next()
            .unwrap_or_default()
            .to_owned()
    };
    baylee_client_core::textface::keyword_chips(oracle.lines())
        .into_iter()
        .next()
}

/// The row frame every list line stands in: absolute at its top, its own
/// height, clipping.
fn frame(kit: Kit, top: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        top: px_fixed(top),
        left: px_fixed(0.0),
        width: Val::Percent(100.0),
        height: px_fixed(height),
        align_items: AlignItems::Center,
        column_gap: kit.m.px(10.0),
        padding: UiRect::axes(kit.m.px(6.0), px_fixed(0.0)),
        border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
        overflow: Overflow::clip(),
        ..default()
    }
}

/// A card's print at a height, at its own aspect.
fn print(commands: &mut Commands, hover: &crate::lobby::HoverCard, height: f32) -> Entity {
    let thumb = crate::lobby::thumbnails::spawn(commands, hover);
    commands.entity(thumb).insert(Node {
        width: px_fixed((height * 5.0 / 7.0).round()),
        height: px_fixed(height.round()),
        flex_shrink: 0.0,
        border_radius: BorderRadius::all(px_fixed(4.0)),
        ..default()
    });
    thumb
}

/// A deck section's heading: its caret, its name and count; a press folds
/// it or opens it again.
#[allow(clippy::too_many_arguments)] // one heading: where, which, and what it says
pub(crate) fn section_head(
    commands: &mut Commands,
    kit: Kit,
    lang: Lang,
    key: SectionKey,
    cards: u32,
    folded: bool,
    top: f32,
    height: f32,
) -> Entity {
    let m = kit.m;
    let row = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px_fixed(top),
                left: px_fixed(0.0),
                width: Val::Percent(100.0),
                height: px_fixed(height),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                padding: UiRect::axes(m.px(4.0), px_fixed(0.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Press::Build(BuildPress::ToggleSection(key)),
        ))
        .id();
    let caret = glyph(
        commands,
        kit,
        if folded { mark::FOLDED } else { mark::OPEN },
        m.small * 0.9,
        tokens::MUTED,
    );
    let name = caption(
        commands,
        kit,
        &format!("{} \u{b7} {cards}", key.heading(lang)),
    );
    commands.entity(row).add_children(&[caret, name]);
    row
}
