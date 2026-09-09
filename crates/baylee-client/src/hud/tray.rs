//! The zone browser, drawn.
//!
//! Every zone a choice can reach that the table cannot show: the cards the
//! engine is *showing* this seat, the stack, and every graveyard, exile pile
//! and command zone at the table. Cards, not a list of names — the tray uses
//! the same [`spawn_card_art`] the hand and the stack use, because a second
//! card renderer is how two parts of one interface start disagreeing about
//! what a card looks like.
//!
//! It opens by itself for a choice that needs it ([`Browser::wanted`]) and by
//! hand from the table — a tap on the top card of a pile opens that pile —
//! and a click on one of its cards goes through exactly the same
//! `activate_card` a click on the table does.
//!
//! There was a strip of pile chips above the sheet doing that second job,
//! drawing the local seat's graveyard, exile and command zone as counts. It is
//! gone: those three piles stand on the felt now with a real stack of cards on
//! them, and two drawings of one zone in two renderers is what the command
//! zone's own well already replaced once. The pile *is* the button.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::browser::{BrowseRow, BrowseZone, Browser};

/// The card a browser row is drawn at — smaller than a hand card, because
/// a search can put thirty of them on screen at once, large enough that the
/// art still identifies the card without the preview.
const TRAY_CARD_W: f32 = 74.0;
/// Height, keeping the 63:88 card aspect.
const TRAY_CARD_H: f32 = TRAY_CARD_W * 88.0 / 63.0;
/// Panel width: eight cards, their gaps and the padding.
///
/// It was five, and a five-wide grid pinned to the left edge is what made
/// looking through a hundred-card library a chore: nine rows of five, most of
/// them off the bottom of a panel that also sat over the seat tabs.
///
/// The sheet takes its width from [`Placement`] now, because a player can
/// resize it. This stays as the *derivation* of that default — the arithmetic
/// that says why eight columns is 690 and not a round number somebody liked —
/// and `the_default_width_is_still_eight_columns` holds the two together.
#[cfg(test)]
const TRAY_PANEL_W: f32 = 8.0 * (TRAY_CARD_W + TRAY_GAP) + 34.0;

/// The air between two cards in the grid, in both directions.
///
/// Named because two places were using it and disagreeing: the grid laid its
/// cards out at 6 and [`TRAY_PANEL_W`] derived the sheet's width from 8, so
/// "eight columns" was 22 pixels wider than eight columns and the test that
/// held the two together was holding one of them to a number the other did
/// not use. Eight is what the seat bar and the phase rail already put between
/// two buttons.
const TRAY_GAP: f32 = 8.0;

/// What stands above and below the grid on the sheet.
///
/// Measured on the running client rather than derived, because most of it is
/// text: one border and sixteen of padding, then the header, the zone tabs,
/// the filter row and three ten-pixel gaps come to **112** logical pixels
/// from the sheet's top edge to the first card's, and the bottom padding and
/// border close it with **17**. It exists so
/// [`Placement::DEFAULT_H`](baylee_client_core::browser::Placement::DEFAULT_H)
/// is a number with a reason rather than one somebody liked.
#[cfg(test)]
const TRAY_CHROME_H: f32 = 112.0 + 17.0;

/// The strip of screen the sheet is allowed into: below the seat tabs and the
/// phase rail, above the hand bar.
///
/// One function because three places need the same answer and a band computed
/// twice is a band that can disagree with itself — the overlay places the
/// sheet in it, the drag clamps against it, and a resized window re-fits to
/// it. A window that has not been created yet answers with the size the rest
/// of the overlay falls back to.
#[must_use]
pub(crate) fn band_of(windows: &Query<&Window>) -> (f32, f32) {
    let size = windows.single().map_or(Vec2::new(1200.0, 800.0), |w| {
        Vec2::new(w.width(), w.height())
    });
    (size.x, (size.y - EDGE - HAND_BAR_H).max(Placement::MIN_H))
}

/// The zone browser: a sheet laid on the felt, in the middle of the table.
///
/// Centred rather than pinned to a corner, and parchment rather than a black
/// panel, for the same reason the prompt slip is: this is the surface a
/// player *reads* — a graveyard they are looking through, a library the
/// engine is showing them — and the middle of the screen is where a stack of
/// cards goes when somebody puts one down on a real table.
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
    sheets: Option<&UiSheets>,
    mut cards: Option<&mut UiCards<'_>>,
    place: Placement,
) -> Entity {
    let rows = browser.rows(view, interaction);
    // The band: the whole window between its top edge and the hand bar,
    // painting nothing and answering no click. It used to centre its one
    // child; now it is the coordinate space that child is placed in, which is
    // what makes a remembered position mean the same thing on two screens
    // with different amounts of HUD above and below. It used to start under
    // the phase rail, a hundred and ten pixels down; there is nothing across
    // the top of the window any more, so it starts at the window's own edge
    // and the sheet has that much more room to be dragged into.
    let frame = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(EDGE),
                bottom: px(HAND_BAR_H),
                ..default()
            },
            ZIndex(3),
            Pickable::IGNORE,
        ))
        .id();
    let sheet_node = commands.spawn((
        TrayPanel,
        Node {
            position_type: PositionType::Absolute,
            left: px(place.left),
            top: px(place.top),
            width: px(place.width),
            height: px(place.height),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            padding: UiRect::all(px(16)),
            border: UiRect::all(px(1)),
            overflow: Overflow::clip(),
            border_radius: sheet_radius(),
            ..default()
        },
        BackgroundColor(palette::PARCHMENT),
        BorderColor::all(palette::PARCHMENT_EDGE),
        sheet_shadow(),
    ));
    let panel = sheet_node.id();
    commands.entity(frame).add_child(panel);
    // First child, so every row below is drawn on it — see [`sheet_surface`]
    // for why the parchment is not the panel's own image.
    if let Some(sheets) = sheets {
        let surface = commands.spawn(sheet_surface(sheets)).id();
        commands.entity(panel).add_child(surface);
    }

    // ---- header: what this is, the way out of it, and the handle ----
    //
    // The row answers the pointer now rather than ignoring it, because it is
    // what a drag takes hold of. The title inside it keeps `Pickable::IGNORE`,
    // so a press anywhere on the row that is not the close button is a press
    // on the row itself.
    let header = commands
        .spawn((
            TrayGrip,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let title = super::overlay::slip_text(
        commands,
        fonts,
        Phrase::BrowseTitle.text(lang),
        16.0,
        palette::SLIP_INK,
        false,
    );
    commands.entity(title).insert(Pickable::IGNORE);
    // The way out.
    //
    // It was a squat pill — 26.5 by 21.5 — with a 6.5 px cross adrift in the
    // middle of it, no fill, and nothing that answered the pointer: a stray
    // mark on the sheet rather than a control. Square, so the cross has a
    // centre to sit in; the glyph large enough to read as a cross; and a
    // `Feel`, because every other button in this client breathes and these
    // were the only ones that did not.
    let close = commands
        .spawn((
            TrayClose,
            Button,
            Node {
                width: px(24),
                height: px(24),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::SLIP_GHOST),
            BorderColor::all(palette::PARCHMENT_EDGE),
            Feel::new(palette::SLIP_GHOST),
            children![(
                // The icon font's own cross. Inter has no U+2715, which is
                // why the button drew as a thin bar for one build.
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, 13.0),
                TextColor(palette::SLIP_INK),
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
    // "All" carries no count: a sum of a graveyard, a stack and a reveal is a
    // number about nothing.
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
    //
    // An ordering has no filter to offer — the panel is the answer being
    // assembled, and narrowing it would hide places in it — so the row says
    // what to do instead. Everywhere else this is a field: empty and unfocused
    // it shows what it is for, focused it shows a caret, and either way it is
    // the thing a player clicks to search the pile they are looking at.
    let ordering = interaction.is_some_and(baylee_client_core::Interaction::is_ordering);
    let typing = browser.is_typing();
    let hint = if ordering {
        Phrase::BrowseOrderHint.text(lang).to_string()
    } else if typing {
        format!("{}\u{258f}", browser.filter())
    } else if browser.filter().trim().is_empty() {
        Phrase::BrowseFilter.text(lang).to_string()
    } else {
        format!("\u{201c}{}\u{201d}", browser.filter())
    };
    let filter_row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: px(6),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = typing || !browser.filter().trim().is_empty();
    let filter_fill = if typing {
        Color::srgba(0.0, 0.0, 0.0, 0.10)
    } else {
        palette::SLIP_GHOST
    };
    let filter_text = super::overlay::slip_text(
        commands,
        fonts,
        &hint,
        12.0,
        if said {
            palette::SLIP_INK
        } else {
            palette::SLIP_SOFT
        },
        false,
    );
    commands.entity(filter_text).insert(Pickable::IGNORE);
    let filter_line = commands
        .spawn((
            TrayFilter,
            Button,
            Node {
                flex_grow: 1.0,
                padding: UiRect::axes(px(7), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(filter_fill),
            Feel::new(filter_fill),
        ))
        .id();
    commands.entity(filter_line).add_child(filter_text);
    // A library is a hundred cards and a long graveyard is thirty, so "look
    // through this pile" is not a question the pile's own order answers on
    // its own. The key and the direction are two buttons because they are two
    // questions, and the arrow says which way the current one runs rather
    // than being a third state of the key.
    let sort_fill = Color::srgba(0.0, 0.0, 0.0, 0.10);
    let key_text = super::overlay::slip_text(
        commands,
        fonts,
        browser.sort().label().text(lang),
        12.0,
        palette::SLIP_INK,
        false,
    );
    commands.entity(key_text).insert(Pickable::IGNORE);
    let sort_key = commands
        .spawn((
            TraySort { reverse: false },
            Button,
            Node {
                padding: UiRect::axes(px(7), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(sort_fill),
            Feel::new(sort_fill),
        ))
        .id();
    commands.entity(sort_key).add_child(key_text);
    let dir_text = super::overlay::slip_text(
        commands,
        fonts,
        if browser.descending() {
            "\u{2193}"
        } else {
            "\u{2191}"
        },
        12.0,
        palette::SLIP_INK,
        false,
    );
    commands.entity(dir_text).insert(Pickable::IGNORE);
    let sort_dir = commands
        .spawn((
            TraySort { reverse: true },
            Button,
            Node {
                padding: UiRect::axes(px(6), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(sort_fill),
            Feel::new(sort_fill),
        ))
        .id();
    commands.entity(sort_dir).add_child(dir_text);
    commands
        .entity(filter_row)
        .add_children(&[filter_line, sort_key, sort_dir]);

    // ---- the cards ----
    //
    // The grid scrolls, and it is the grid rather than the panel: the tabs,
    // the filter and the sort control have to stay where they are while a
    // hundred-card library is scrolled past them. `Pickable` and not
    // `Pickable::IGNORE`, because the picking backend is what turns a wheel
    // into the `Pointer<Scroll>` a scrolling node listens for — the same
    // reason `dev-control` has to put the pointer over a list before it can
    // send one.
    //
    // `min_height: px(0)` beside the `flex_grow`, and it is load-bearing: a
    // flex item's default `min-height` is `auto`, so a hundred-card library
    // would size the column to its own content, push the sheet past the
    // explicit height it was given, and hand the overflow — including the
    // resize corner — to `Overflow::clip`.
    let grid = commands
        .spawn((Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(TRAY_GAP),
            row_gap: px(TRAY_GAP),
            flex_grow: 1.0,
            min_height: px(0),
            overflow: Overflow::scroll_y(),
            ..default()
        },))
        .id();
    if rows.is_empty() {
        let empty = super::overlay::slip_text(
            commands,
            fonts,
            Phrase::BrowseEmpty.text(lang),
            12.0,
            palette::SLIP_SOFT,
            false,
        );
        commands.entity(empty).insert(Pickable::IGNORE);
        commands.entity(grid).add_child(empty);
    }
    for row in &rows {
        let card = spawn_row(
            commands, lang, row, view, statics, textures, assets, fonts, faces, &mut cards,
        );
        commands.entity(grid).add_child(card);
    }

    // The corner, in the same shape and the same place the card preview's is:
    // one handle, bottom right, both axes. A second handle on every edge is
    // eight more hit targets for a gesture nobody makes on a sheet of cards.
    let corner = commands
        .spawn((
            TrayResize,
            Button,
            Node {
                position_type: PositionType::Absolute,
                right: px(6),
                bottom: px(6),
                width: px(22),
                height: px(22),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::SLIP_GHOST),
            Feel::new(palette::SLIP_GHOST),
            children![(
                Text::new(glyph::EXPAND.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(palette::SLIP_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id();

    commands
        .entity(panel)
        .add_children(&[header, tabs, filter_row, grid, corner]);
    frame
}

/// One zone tab.
fn spawn_tab(
    commands: &mut Commands,
    fonts: &UiFonts,
    zone: Option<BrowseZone>,
    label: String,
    current: bool,
) -> Entity {
    // Brass is a *light* on this sheet and not a letter: measured against
    // parchment it carries 1.9:1, which is why the current tab read fainter
    // than the ones beside it. The ink says which tab is current; the fill
    // under it says it a second time.
    let (fill, ink) = if current {
        (Color::srgba(0.0, 0.0, 0.0, 0.10), palette::SLIP_INK)
    } else {
        (palette::SLIP_GHOST, palette::SLIP_SOFT)
    };
    let text = super::overlay::slip_text(commands, fonts, &label, 12.0, ink, false);
    commands.entity(text).insert(Pickable::IGNORE);
    let tab = commands
        .spawn((
            TrayTab { zone },
            Button,
            Node {
                padding: UiRect::axes(px(7), px(3)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(fill),
            Feel::new(fill),
        ))
        .id();
    commands.entity(tab).add_child(text);
    tab
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
            palette::BRASS,
            Val::Px(0.0),
            Val::Px(0.0),
            Val::Px(2.0),
            Val::Px(9.0),
        )
    } else if row.selectable {
        BoxShadow::new(
            palette::BRASS,
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.10)),
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
                    tf(fonts, 10.0),
                    TextColor(palette::PARCHMENT_INK),
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
                BackgroundColor(palette::BRASS),
                children![(
                    Text::new(place.to_string()),
                    tf(fonts, 11.0),
                    TextColor(palette::PANEL),
                    Pickable::IGNORE,
                )],
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(badge);
    }

    // A token says so. A graveyard holds cards and tokens together and they
    // are not the same thing — a token there ceases to exist the next time
    // state-based actions are checked (CR 111.7), so a row that looked like a
    // card would be inviting a player to plan around something already gone.
    // Along the bottom edge rather than in a corner, because the top-left
    // corner is the ordering badge's and two marks fighting for one corner is
    // how a player learns to read neither.
    if row.token {
        let mark = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    right: px(0),
                    bottom: px(0),
                    height: px(13),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(palette::PANEL),
                children![(
                    Text::new(Phrase::IsToken.text(lang)),
                    tf(fonts, 9.0),
                    TextColor(palette::MUTED),
                    Pickable::IGNORE,
                )],
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(mark);
    }
    slot
}

/// What a zone tab reads. A pile belonging to a seat says whose it is,
/// because at a table of four "Graveyard" alone names nothing — and every tab
/// says how many cards are in it, which is the one thing the deleted pile
/// chips carried that nothing else on the sheet does.
///
/// The count goes in brackets rather than after a separator because the
/// sheet's typography already means something by a bracket: `slip_text` hands
/// a bracketed run to [`palette::SLIP_ASIDE`], so "Graveyard (12)" is drawn as
/// a name with a grey aside beside it and reads as one.
fn zone_label(lang: Lang, zone: BrowseZone, view: &PlayerView, statics: &GameStatic) -> String {
    let name = zone.label().text(lang).to_string();
    let named = match zone.seat() {
        None => name,
        Some(seat) if seat == view.seat => name,
        Some(seat) => {
            let who = statics.seats.iter().find(|s| s.player == seat).map_or_else(
                || Phrase::SeatNumbered.fill(lang, &[&seat.get().to_string()]),
                |s| s.display_name.clone(),
            );
            Phrase::BrowseZoneOf.fill(lang, &[&name, &who])
        }
    };
    Phrase::BrowseTabCount.fill(lang, &[&named, &zone.count_in(view).to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::prose::bracketed;
    use baylee_client_core::test_support::{ViewBuilder, printed};
    use baylee_core::ids::PlayerId;

    fn statics() -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: "g".into(),
            your_seat: PlayerId::new(0),
            seats: vec![baylee_view::SeatIdentity {
                player: PlayerId::new(1),
                display_name: "House AI".into(),
                is_ai: true,
                away: false,
                team: None,
            }],
            prints: vec![],
        }
    }

    /// The sheet a player has never moved is still eight card columns wide.
    ///
    /// `Placement::DEFAULT_W` is a number in the renderer-free half, where it
    /// can be tested but where `TRAY_CARD_W` does not exist; the arithmetic
    /// that produced it lives here. This is the seam between them, so a card
    /// resized on one side cannot silently leave the other showing seven
    /// columns and a gap.
    #[test]
    fn the_default_width_is_still_eight_columns() {
        assert!(
            (Placement::DEFAULT_W - TRAY_PANEL_W).abs() < f32::EPSILON,
            "eight columns is {TRAY_PANEL_W}, the sheet opens at {}",
            Placement::DEFAULT_W
        );
    }

    /// And it opens three whole rows tall.
    ///
    /// The same seam one axis over, and the one that had gone wrong: 520 is
    /// three rows plus sixty-five pixels, so the sheet always showed most of
    /// a fourth row that nothing could ever be put in. The tolerance is a
    /// pixel because [`TRAY_CHROME_H`] is a measurement and
    /// [`Placement::DEFAULT_H`] is a whole number.
    #[test]
    fn the_default_height_is_three_whole_rows() {
        let rows = 3.0;
        let want = TRAY_CHROME_H + rows * TRAY_CARD_H + (rows - 1.0) * TRAY_GAP;
        let off = (Placement::DEFAULT_H - want).abs();
        assert!(
            off <= 1.0,
            "three rows is {want}, the sheet opens at {} ({off} out)",
            Placement::DEFAULT_H
        );
        // And it is genuinely short of a fourth, which is the whole point.
        let four = want + TRAY_CARD_H + TRAY_GAP;
        assert!(Placement::DEFAULT_H < four - TRAY_CARD_H / 2.0);
    }

    /// The band never claims more room than the window has.
    #[test]
    fn the_band_is_what_is_left_above_the_hand() {
        // A window the size the dev harness reports.
        let tall = 1052.0 - EDGE - HAND_BAR_H;
        assert!(tall > Placement::MIN_H, "the fixture is not exercising it");
        // A window too short for a sheet still gets one: `MIN_H` wins, and a
        // sheet clamped to nothing would be a sheet that is not there.
        let cramped = (200.0f32 - EDGE - HAND_BAR_H).max(Placement::MIN_H);
        assert!((cramped - Placement::MIN_H).abs() < f32::EPSILON);
    }

    /// A tab says how many cards are in it, and says it in the one register
    /// the sheet greys.
    ///
    /// Two claims in one, because they are one decision: the count is drawn
    /// as an aside rather than as part of the name, and `slip_text` decides
    /// that by finding a bracket. A count appended with a separator would
    /// read at full ink weight and make every tab look twice as long.
    #[test]
    fn a_zone_tab_carries_its_count_as_an_aside() {
        let view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(1, 0, "Llanowar Elves", 1)])
            .with_graveyard(1, vec![printed(2, 1, "Ponder", 2)])
            .build();
        let mine = zone_label(
            Lang::En,
            BrowseZone::Graveyard(PlayerId::new(0)),
            &view,
            &statics(),
        );
        assert_eq!(mine, "Graveyard (1)", "my own pile does not say whose");

        let runs: Vec<_> = bracketed(&mine).collect();
        assert_eq!(
            runs,
            vec![("Graveyard ", false), ("(1)", true)],
            "the count is not in the aside register the sheet greys"
        );

        let theirs = zone_label(
            Lang::En,
            BrowseZone::Graveyard(PlayerId::new(1)),
            &view,
            &statics(),
        );
        assert_eq!(
            theirs, "Graveyard · House AI (1)",
            "somebody else's pile says whose, and still counts"
        );
    }

    /// Nothing on the parchment says anything in brass.
    ///
    /// `BRASS` on `PARCHMENT` measures 1.9:1 — below every legibility floor —
    /// which is why the *current* zone tab read fainter than the ones beside
    /// it. Brass keeps its job as a light: the card glow and the ordering
    /// badge, both of which sit on their own fill. The check is on the source
    /// because what is being held is a rule about the whole file, not about
    /// one node.
    #[test]
    fn the_sheet_writes_no_letters_in_brass() {
        // Assembled rather than written out, or the needle is in the
        // haystack and this test fails on its own source line.
        let ink_in = format!("TextColor(palette::{}", "BRASS");
        for line in include_str!("tray.rs").lines() {
            let code = line.split("//").next().unwrap_or(line);
            assert!(
                !code.contains(&ink_in),
                "brass is a light on this sheet, not a letter: {line}"
            );
        }
    }
}
