//! The zone browser, drawn.
//!
//! Every zone a choice can reach that the table cannot show: the cards the
//! engine is *showing* this seat, the stack, and every graveyard, exile pile
//! and command zone at the table. Cards, not a list of names — the tray uses
//! the same [`spawn_card_art`] the hand and the stack use, because a second
//! card renderer is how two parts of one interface start disagreeing about
//! what a card looks like.
//!
//! It opens by itself for a choice that needs it ([`Browser::wanted`]) and
//! by hand from a pile chip, and a click on one of its cards goes through
//! exactly the same `activate_card` a click on the table does.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::browser::{BrowseRow, BrowseZone, Browser};

/// The card a browser row is drawn at — smaller than a hand card, because
/// a search can put thirty of them on screen at once, large enough that the
/// art still identifies the card without the preview.
const TRAY_CARD_W: f32 = 74.0;
/// Height, keeping the 63:88 card aspect.
const TRAY_CARD_H: f32 = TRAY_CARD_W * 88.0 / 63.0;
/// Panel width: five cards, their gaps and the padding.
const TRAY_PANEL_W: f32 = 5.0 * (TRAY_CARD_W + 6.0) + 18.0;

/// The browser panel, pinned left below the seat tabs.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the stores
#[allow(clippy::too_many_lines)] // header, tabs, filter and grid are one build
pub(super) fn spawn_tray(
    commands: &mut Commands,
    lang: Lang,
    browser: &Browser,
    view: &PlayerView,
    interaction: Option<&baylee_client_core::Interaction>,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    mut cards: Option<&mut UiCards<'_>>,
) -> Entity {
    let rows = browser.rows(view, interaction);
    let panel = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(TAB_H + 44.0),
                width: px(TRAY_PANEL_W),
                max_height: percent(70),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(9)),
                overflow: Overflow::clip(),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            ZIndex(3),
            overlay_shadow(),
            Pickable::IGNORE,
        ))
        .id();

    // ---- header: what this is, and the way out of it ----
    let header = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(Phrase::BrowseTitle.text(lang)),
            tf(fonts, 13.0),
            TextColor(palette::INK),
            Pickable::IGNORE,
        ))
        .id();
    let close = commands
        .spawn((
            TrayClose,
            Button,
            Node {
                padding: UiRect::axes(px(8), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            children![(
                Text::new("\u{2715}"),
                tf(fonts, 12.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(header).add_children(&[title, close]);

    // ---- the zone tabs, "All" first ----
    let tabs = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4),
                row_gap: px(4),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let mut chips = vec![spawn_tab(
        commands,
        fonts,
        None,
        Phrase::BrowseAll.text(lang).to_string(),
        browser.tab().is_none(),
    )];
    for zone in browser.zones(view) {
        chips.push(spawn_tab(
            commands,
            fonts,
            Some(zone),
            zone_label(lang, zone, view, statics),
            browser.tab() == Some(zone),
        ));
    }
    commands.entity(tabs).add_children(&chips);

    // ---- what is typed, and what an ordering wants ----
    let hint = if interaction.is_some_and(baylee_client_core::Interaction::is_ordering) {
        Phrase::BrowseOrderHint.text(lang).to_string()
    } else if browser.filter().trim().is_empty() {
        Phrase::BrowseFilter.text(lang).to_string()
    } else {
        format!("\u{201c}{}\u{201d}", browser.filter())
    };
    let filter_line = commands
        .spawn((
            Text::new(hint),
            tf(fonts, 11.0),
            TextColor(if browser.filter().trim().is_empty() {
                palette::MUTED
            } else {
                palette::ACCENT
            }),
            Pickable::IGNORE,
        ))
        .id();

    // ---- the cards ----
    let grid = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if rows.is_empty() {
        let empty = commands
            .spawn((
                Text::new(Phrase::BrowseEmpty.text(lang)),
                tf(fonts, 11.0),
                TextColor(palette::MUTED),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(grid).add_child(empty);
    }
    for row in &rows {
        let card = spawn_row(
            commands, lang, row, view, statics, textures, assets, fonts, faces, &mut cards,
        );
        commands.entity(grid).add_child(card);
    }

    commands
        .entity(panel)
        .add_children(&[header, tabs, filter_line, grid]);
    panel
}

/// One zone tab.
fn spawn_tab(
    commands: &mut Commands,
    fonts: &UiFonts,
    zone: Option<BrowseZone>,
    label: String,
    current: bool,
) -> Entity {
    commands
        .spawn((
            TrayTab { zone },
            Button,
            Node {
                padding: UiRect::axes(px(7), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if current {
                palette::PANEL_LIT
            } else {
                palette::PANEL
            }),
            children![(
                Text::new(label),
                tf(fonts, 10.0),
                TextColor(if current {
                    palette::ACCENT
                } else {
                    palette::MUTED
                }),
                Pickable::IGNORE,
            )],
        ))
        .id()
}

/// One card in the grid: its picture, its selection state, and — for an
/// ordering — the place it holds in the answer.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
#[allow(clippy::too_many_lines)] // art, plate and place badge are one build
fn spawn_row(
    commands: &mut Commands,
    lang: Lang,
    row: &BrowseRow,
    view: &PlayerView,
    statics: &GameStatic,
    textures: &mut CardTextures,
    assets: &AssetServer,
    fonts: &UiFonts,
    faces: &FaceCtx<'_>,
    cards: &mut Option<&mut UiCards<'_>>,
) -> Entity {
    // Three states, three glows, and the same vocabulary the hand bar uses:
    // gold for what the engine offered, brighter gold for what is already
    // part of the answer, nothing at all for a card being read rather than
    // chosen.
    let shadow = if row.selected {
        BoxShadow::new(
            palette::ACCENT,
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(2.0),
            Val::Px(9.0),
        )
    } else if row.selectable {
        BoxShadow::new(
            palette::ACCENT,
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(5.0),
        )
    } else {
        soft_shadow()
    };
    let slot = commands
        .spawn((
            TrayCard { object: row.id },
            Button,
            Node {
                width: px(TRAY_CARD_W),
                height: px(TRAY_CARD_H),
                flex_shrink: 0.0,
                border_radius: card_radius(TRAY_CARD_W),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::PANEL_LIT),
            shadow,
        ))
        .id();

    let built = view
        .object(row.id)
        .and_then(|o| faces.object(o, textures, row.art));
    if let Some(key) = row.art {
        let image = textures.get(key, statics, assets);
        let visual = spawn_card_art(
            commands,
            lang,
            image,
            built.as_ref(),
            TRAY_CARD_W,
            TRAY_CARD_H,
            crate::face::Detail::Compact,
            fonts,
            // No keyword sheath: nothing in the browser is on a battlefield,
            // and a card in a graveyard wearing an indestructible border
            // would be claiming something the rules do not say.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards.as_deref_mut(),
        );
        commands.entity(slot).add_child(visual);
    } else {
        // A token in a graveyard, or a card this seat may not identify.
        let plate = commands
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::all(px(4)),
                    ..default()
                },
                children![(
                    Text::new(row.name.clone()),
                    tf(fonts, 9.0),
                    TextColor(palette::INK),
                    Pickable::IGNORE,
                )],
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(plate);
    }

    // The place in an ordering, drawn over the corner. A number rather than
    // a glow, because "third" is not a brighter kind of "chosen".
    if let Some(place) = row.place {
        let badge = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: px(3),
                    left: px(3),
                    min_width: px(16),
                    height: px(16),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(8)),
                    ..default()
                },
                BackgroundColor(palette::ACCENT),
                children![(
                    Text::new(place.to_string()),
                    tf(fonts, 10.0),
                    TextColor(palette::PANEL),
                    Pickable::IGNORE,
                )],
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(badge);
    }
    slot
}

/// What a zone tab reads. A pile belonging to a seat says whose it is,
/// because at a table of four "Graveyard" alone names nothing.
fn zone_label(lang: Lang, zone: BrowseZone, view: &PlayerView, statics: &GameStatic) -> String {
    let name = zone.label().text(lang).to_string();
    match zone.seat() {
        None => name,
        Some(seat) if seat == view.seat => name,
        Some(seat) => {
            let who = statics.seats.iter().find(|s| s.player == seat).map_or_else(
                || Phrase::SeatNumbered.fill(lang, &[&seat.get().to_string()]),
                |s| s.display_name.clone(),
            );
            Phrase::BrowseZoneOf.fill(lang, &[&name, &who])
        }
    }
}

/// The pile chips: the viewing seat's own off-board zones, as counts that
/// can be clicked.
///
/// The counts were already on screen — in every seat tab — and were the one
/// place the interface said "there are seven cards here" and offered no way
/// to look at them. They are the seat's *own* piles only, because a strip
/// that listed all eight seats' would not fit; every other pile at the table
/// is one tab away inside the tray, which lists them all.
#[allow(clippy::too_many_lines)] // one flat strip of chips
pub(super) fn spawn_pile_strip(
    commands: &mut Commands,
    lang: Lang,
    browser: &Browser,
    view: &PlayerView,
    fonts: &UiFonts,
) -> Entity {
    let strip = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                top: px(TAB_H + 10.0),
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                ..default()
            },
            ZIndex(3),
            Pickable::IGNORE,
        ))
        .id();

    let me = view.seat;
    let mut chips = Vec::new();
    let pile = |zones: &Vec<Vec<baylee_view::PublicObject>>| {
        zones.get(me.get() as usize).map_or(0, Vec::len)
    };
    for (zone, count, icon) in [
        (
            BrowseZone::Graveyard(me),
            pile(&view.graveyards),
            glyph::SKULL,
        ),
        (BrowseZone::Exile(me), pile(&view.exile), glyph::EXILE),
        (BrowseZone::Command(me), pile(&view.command), glyph::COMMAND),
    ] {
        // An empty pile is not a chip. A button that opens onto nothing is
        // one the player learns to stop pressing.
        if count == 0 {
            continue;
        }
        let lit = browser.is_open() && browser.tab() == Some(zone);
        chips.push(
            commands
                .spawn((
                    PileChip { zone: Some(zone) },
                    Button,
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(4),
                        padding: UiRect::axes(px(7), px(3)),
                        border_radius: btn_radius(),
                        ..default()
                    },
                    BackgroundColor(if lit {
                        palette::PANEL_LIT
                    } else {
                        palette::PANEL
                    }),
                    soft_shadow(),
                    children![
                        (
                            Text::new(icon.to_string()),
                            icon_tf(fonts, 10.0),
                            TextColor(if lit { palette::ACCENT } else { palette::MUTED }),
                            Pickable::IGNORE,
                        ),
                        (
                            Text::new(count.to_string()),
                            tf(fonts, 11.0),
                            TextColor(if lit { palette::ACCENT } else { palette::INK }),
                            Pickable::IGNORE,
                        ),
                    ],
                ))
                .id(),
        );
    }
    // The way in when every pile is empty but the stack or a reveal is not:
    // without it a search would be the only thing that ever opened the tray.
    let all_lit = browser.is_open() && browser.tab().is_none();
    chips.push(
        commands
            .spawn((
                PileChip { zone: None },
                Button,
                Node {
                    padding: UiRect::axes(px(7), px(3)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if all_lit {
                    palette::PANEL_LIT
                } else {
                    palette::PANEL
                }),
                soft_shadow(),
                children![(
                    Text::new(Phrase::BrowseTitle.text(lang)),
                    tf(fonts, 10.0),
                    TextColor(if all_lit {
                        palette::ACCENT
                    } else {
                        palette::MUTED
                    }),
                    Pickable::IGNORE,
                )],
            ))
            .id(),
    );
    commands.entity(strip).add_children(&chips);
    strip
}
