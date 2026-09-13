//! The zone browser, drawn.
//!
//! Every zone a choice can reach that the table cannot show: the cards the
//! engine is *showing* this seat, the stack, and every graveyard, exile pile
//! and command zone at the table.
//!
//! It opens by itself for a choice that needs it ([`Browser::wanted`]) and by
//! hand from the table — a tap on the top card of a pile opens that pile —
//! and a click on one of its rows goes through exactly the same
//! `activate_card` a click on the table does.
//!
//! # Why it is a panel and not a sheet
//!
//! `docs/redesign-proposal.md` §1.3 draws the line: **parchment is a sheet
//! you read from, a panel is a place you work in.** This was parchment, and a
//! grid of ten card columns, on the argument that a graveyard is something a
//! player *reads*. It is not: §6 draws a checkbox, a tally and a Confirm, and
//! that is work. So it is a dark panel with a list in it — each row a
//! checkbox, a thumbnail, a name, the cost in pips, the type line and the
//! zone it is in — and the chosen row goes candle, not teal.
//!
//! The grid is what the list replaces, and the reason is the same one that
//! made the grid ten columns wide: a fetchland offers the whole library. A
//! grid answers "show me more at once" by growing sideways, which is the axis
//! that buys nothing here — a name, a cost and a type line fit in one measure
//! and everything past it is blank. A list grows *down*, which is where a
//! hundred cards are, and it can say the three things about a card that a
//! player searching a library is actually reading.
//!
//! There was a strip of pile chips above the sheet doing the by-hand job,
//! drawing the local seat's graveyard, exile and command zone as counts. It is
//! gone: those three piles stand on the felt now with a real stack of cards on
//! them, and two drawings of one zone in two renderers is what the command
//! zone's own well already replaced once. The pile *is* the button.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::browser::{BrowseRow, BrowseZone, Browser};

/// The thumbnail on a row.
///
/// Small on purpose: it is there to be *recognised*, not read — the name is
/// beside it in full and the preview is a hover away. It is what sets the row
/// height, being the tallest thing in one.
const TRAY_THUMB_W: f32 = 30.0;
/// Its height, keeping the 63:88 card aspect.
const TRAY_THUMB_H: f32 = TRAY_THUMB_W * 88.0 / 63.0;
/// The air above and below the thumbnail in a row.
const TRAY_ROW_PAD: f32 = 7.0;
/// One row, which is the unit the whole sheet is measured in.
const TRAY_ROW_H: f32 = TRAY_THUMB_H + 2.0 * TRAY_ROW_PAD;
/// The gutter every band of the sheet keeps at its left and right.
///
/// The rows carry it themselves rather than the panel carrying it for them,
/// which is what lets a chosen row's wash run from edge to edge: a highlight
/// that stopped short of the border would read as a chip lying on the list
/// rather than as the row being chosen.
const TRAY_SIDE: f32 = 16.0;
/// The air between two things in a row, and between two controls.
const TRAY_GAP: f32 = 11.0;
/// The checkbox. Also the width of the gutter a row that cannot be chosen
/// leaves empty, so the names stay in one column.
const TRAY_BOX: f32 = 15.0;
/// One mana pip on a row.
const TRAY_PIP: f32 = 15.0;
/// What a cost is given: four pips and the air between them.
///
/// Four rather than the longest cost in the pool, because this is the width
/// the *sheet* is derived from and a cost longer than four pips simply pushes
/// the name's measure in. Most printed costs are three or four symbols.
const TRAY_COST_W: f32 = 4.0 * TRAY_PIP + 3.0 * 2.0;
/// What one character of the row's prose is worth, as a fraction of its size.
///
/// The same estimate `hud::stack` budgets a stack entry's name with. It is an
/// estimate and it is allowed to be: what it sizes is a *measure*, and a name
/// a little longer than one simply takes a little of the slack beside it.
#[cfg(test)]
const TRAY_CH: f32 = 0.52;
/// The name's size.
const TRAY_NAME_SIZE: f32 = 12.5;
/// The type line's.
const TRAY_TYPE_SIZE: f32 = 10.5;
/// The zone badge's.
const TRAY_BADGE_SIZE: f32 = 9.0;
/// The zone badge: the longest zone word this client has — `Kommandozone`,
/// which sets at 68.8 px in Inter at [`TRAY_BADGE_SIZE`] — plus its padding
/// and border.
///
/// **Measured in the shipped face, not estimated.** [`TRAY_CH`] is a mean
/// over mixed-case English prose and holds there to within a percent; a
/// German compound of round wide letters runs 0.64 per character, and the
/// estimate cut the last three letters off every badge on the panel.
const TRAY_BADGE_W: f32 = 68.8 + 12.0;
/// Thirty characters of name — `Sea Gate Loremaster` and room to spare.
#[cfg(test)]
const TRAY_NAME_W: f32 = 30.0 * TRAY_CH * TRAY_NAME_SIZE;
/// A type line's measure: `Legendary Planeswalker — Aminatou` at
/// [`TRAY_TYPE_SIZE`], which is 185.1 px in Inter.
///
/// Measured rather than estimated for the reason [`TRAY_BADGE_W`] gives — a
/// long type line is all supertype, type and em dash, which is wider than the
/// mean this estimate is taken over — and it is a *measure* rather than a
/// fit: a longer one is clipped at its end, not wrapped.
const TRAY_TYPE_W: f32 = 185.1;

/// The panel's default width: **one row**.
///
/// Its fixed furniture — the checkbox, the thumbnail, four pips of cost and
/// the zone badge — plus a measure for each of the two pieces of prose, and
/// the gutters and gaps that hold them apart. Anything longer than a measure
/// takes the slack in the middle, which is where a list wants it.
///
/// The sheet takes its width from [`Placement`] now, because a player can
/// resize it. This stays as the *derivation* of that default — the arithmetic
/// that says why the number is what it is — and
/// `the_default_width_is_one_whole_row` holds the two together.
#[cfg(test)]
const TRAY_PANEL_W: f32 = 2.0 * TRAY_SIDE
    + TRAY_BOX
    + TRAY_THUMB_W
    + TRAY_NAME_W
    + TRAY_COST_W
    + TRAY_TYPE_W
    + TRAY_BADGE_W
    + 5.0 * TRAY_GAP
    + 2.0;

/// The title row: the sheet's name and the way out of it.
const TRAY_TITLE_H: f32 = 24.0;
/// One zone tab.
const TRAY_TAB_H: f32 = 22.0;
/// The search field, the sort key and the arrow beside it.
const TRAY_CTRL_H: f32 = 28.0;
/// A footer button.
const TRAY_FOOT_H: f32 = 34.0;
/// The air between the head's three rows.
const TRAY_HEAD_GAP: f32 = 8.0;
/// The head band's own padding, above and below.
const TRAY_HEAD_PAD: f32 = 12.0;
/// The footer band's.
const TRAY_FOOT_PAD: f32 = 11.0;

/// What stands above and below the list.
///
/// Arithmetic rather than a measurement, unlike the grid's chrome that came
/// before it: every row of the head and the footer is given an explicit
/// height here, so there is no text line box left to guess at. It exists so
/// [`Placement::DEFAULT_H`](baylee_client_core::browser::Placement::DEFAULT_H)
/// is a number with a reason rather than one somebody liked.
#[cfg(test)]
const TRAY_CHROME_H: f32 =
    // the head: its border, its padding, and three rows with air between them
    1.0 + 2.0 * TRAY_HEAD_PAD + TRAY_TITLE_H + TRAY_TAB_H + TRAY_CTRL_H + 2.0 * TRAY_HEAD_GAP
    // the footer: its border, its padding, one button
    + 1.0 + 2.0 * TRAY_FOOT_PAD + TRAY_FOOT_H
    // and the panel's own border, top and bottom
    + 2.0;

/// How many rows the sheet opens showing.
///
/// The half is the point: a row cut through by the bottom edge is what says
/// the list continues, and it says it without a scrollbar. A grid was cut to
/// four *whole* rows for the opposite reason — most of a fifth row of cards
/// was space nothing could ever be put in.
#[cfg(test)]
const TRAY_ROWS: f32 = 8.5;

/// The strip of screen the sheet is allowed into: below the seat tabs and the
/// phase rail, above the hand bar.
///
/// One function because three places need the same answer and a band computed
/// twice is a band that can disagree with itself — the overlay places the
/// sheet in it, the drag clamps against it, and a resized window re-fits to
/// it. A window that has not been created yet answers with the size the rest
/// of the overlay falls back to.
pub(crate) fn band_of(windows: &Query<&Window>) -> (f32, f32) {
    let (w, h) = windows
        .single()
        .map_or((1280.0, 720.0), |window| (window.width(), window.height()));
    (w, (h - EDGE - HAND_BAR_H).max(Placement::MIN_H))
}

/// How fast the veil rises, as the rate of `1 - e^(-rate·dt)`.
///
/// Nine, against the fourteen every button in this client hovers at: 90% of
/// the way in `ln(10)/9`, about a quarter of a second. A hover answers the
/// pointer and may be quick; a veil changes the whole scene and reads as an
/// accident if it simply appears — but it has to be settled before the eye has
/// finished reading the dialog's title, and anything past a third of a second
/// is the player waiting.
///
/// It rises and never falls on screen: the dialog is a retained tree and goes
/// the instant it is answered, so the veil goes with it. The number still eases
/// back down with nothing to draw, which is what makes the *next* question fade
/// in from nothing rather than snapping from wherever the last one stopped.
const VEIL_RATE: f32 = 9.0;

/// The veil over the table, and the number behind it.
///
/// One full-window node, painting [`palette::TABLE_VEIL`] at whatever fraction the
/// fade has reached, answering no click at all. `Pickable::IGNORE` is the
/// whole of W2's scope: the owner asked for darkening, not for blocking, and
/// a veil that swallowed clicks would be making a claim the model does not
/// make — [`Browser::dims_the_table`] is drawn from `locked`, and even a
/// locked question leaves the board worth *pointing* at. A click that lands
/// here falls through to `input::pointer`'s "nothing interactive" branch,
/// which clears the preview, which is what a click on the table's empty felt
/// has always done.
///
/// It is the window and not [`band_of`]'s strip, because the hand bar is the
/// one thing under it a player might otherwise still reach for, and a question
/// whose every answer is in the dialog is exactly the question the hand cannot
/// answer.
///
/// It is spawned fully clear and painted by [`dim_the_table`], which runs
/// after the rebuild in the same frame: the fade lives in [`Veil`] rather than
/// on this node, and reading it here would mean a seventeenth system parameter
/// on `sync_overlay`, which already carries sixteen.
pub(super) fn spawn_veil(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            TableVeil,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
            BackgroundColor(veil_at(0.0)),
            ZIndex(Z_VEIL),
            Pickable::IGNORE,
        ))
        .id()
}

/// [`palette::TABLE_VEIL`] at `lit` of its alpha.
fn veil_at(lit: f32) -> Color {
    palette::TABLE_VEIL.with_alpha(palette::TABLE_VEIL.alpha() * lit.clamp(0.0, 1.0))
}

/// Eases the veil towards where the browser says it should be, and paints it.
///
/// After `sync_overlay` deliberately, for the reason [`spawn_veil`] gives: a
/// veil spawned this frame is spawned clear, and this is what gives it its
/// colour before anything is drawn. The number itself is eased whether a veil
/// exists or not, which is what lets it fall back to nothing while there is
/// nothing on screen to fall.
///
/// `reduce_motion` takes the whole step at once, the way [`Feel`] does — the
/// veil is still drawn, it simply arrives.
pub(crate) fn dim_the_table(
    time: Res<Time>,
    duel: Res<crate::Duel>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut veil: ResMut<Veil>,
    mut nodes: Query<&mut BackgroundColor, With<TableVeil>>,
) {
    let still = prefs.is_some_and(|p| p.all().reduce_motion);
    let target = if duel.browser.dims_the_table() {
        1.0
    } else {
        0.0
    };
    let step = if still {
        1.0
    } else {
        1.0 - (-VEIL_RATE * time.delta_secs()).exp()
    };
    veil.lit += (target - veil.lit) * step;
    if (veil.lit - target).abs() < 0.001 {
        veil.lit = target;
    }
    let colour = veil_at(veil.lit);
    for mut background in &mut nodes {
        background.0 = colour;
    }
}

/// A line of a dialog's prose.
///
/// The same bracket rule the parchment sheets use — `prose::bracketed` greys
/// what a sentence says in brackets — in the dialog's own two inks. It is not
/// `overlay::slip_text`: that one carries a warm shadow to lift ink off
/// parchment and greys its asides in [`palette::SLIP_ASIDE`], and both of
/// those belong to the sheet rather than to this panel. Ink on a dark ground
/// needs no shadow to be a stroke.
fn dialog_text(
    commands: &mut Commands,
    fonts: &UiFonts,
    text: &str,
    size: f32,
    ink: Color,
) -> Entity {
    let line = commands
        .spawn((
            Text::default(),
            tf(fonts, size),
            TextColor(ink),
            // Every line on this dialog stands in a band of a fixed height,
            // so a line that wrapped would have its second half cut off by
            // the row it is in. Too long is clipped at the end instead, which
            // at least says which card it is.
            TextLayout::linebreak(bevy::text::LineBreak::NoWrap),
            Pickable::IGNORE,
        ))
        .id();
    for (run, aside) in baylee_client_core::prose::bracketed(text) {
        let span = commands
            .spawn((
                TextSpan::new(run.to_string()),
                tf(fonts, size),
                TextColor(if aside { palette::DIALOG_SOFT } else { ink }),
            ))
            .id();
        commands.entity(line).add_child(span);
    }
    line
}

/// The zone browser: a dialog over the table, in the middle of it.
///
/// Centred rather than pinned to a corner, because that is where a stack of
/// cards goes when somebody puts one down on a real table — and a dark panel
/// rather than parchment, for the reason the module doc gives.
#[allow(clippy::too_many_arguments)] // a panel, a view, and the stores
#[allow(clippy::too_many_lines)] // head, tabs, controls, list and footer are one build
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
    place: Placement,
) -> Entity {
    let rows = browser.rows(view, interaction);
    // The question *this* sheet answers, which is not every question there
    // might be: a graveyard opened by hand while the engine asks about the
    // battlefield holds none of the answer, and grew a tally and a Confirm for
    // it anyway. `Browser::answers_here` carries the whole argument, including
    // why the prompt slip reads the same predicate and draws no second
    // Confirm behind this one. Everything else on the sheet — the rows, the
    // place numbers — still takes the interaction whole: a row is drawn as
    // selected because it *is*, whatever surface the send belongs to.
    let answering = browser
        .answers_here(interaction)
        .then_some(interaction)
        .flatten();
    // The band: the whole window between its top edge and the hand bar,
    // painting nothing and answering no click. It is the coordinate space the
    // sheet is placed in, which is what makes a remembered position mean the
    // same thing on two screens with different amounts of HUD above and below.
    //
    // Five, above [`spawn_veil`]'s three and the prompt slip's four. The
    // overlay's whole order is stated in one place — `overlay::sync_overlay`'s
    // doc — because a `ZIndex` is local to a parent's children and two
    // siblings that share one are settled by the order they were spawned in,
    // which is how the seat bars once came to be drawn through this dialog.
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
            ZIndex(Z_SHEET),
            Pickable::IGNORE,
        ))
        .id();
    let panel = commands
        .spawn((
            TrayPanel,
            Node {
                position_type: PositionType::Absolute,
                left: px(place.left),
                top: px(place.top),
                width: px(place.width),
                height: px(place.height),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px(1)),
                overflow: Overflow::clip(),
                border_radius: sheet_radius(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            sheet_shadow(),
        ))
        .id();
    commands.entity(frame).add_child(panel);

    // ---- the head: what this is, what it is showing, and what to type ----
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(TRAY_HEAD_GAP),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_HEAD_PAD)),
                border: UiRect::bottom(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();

    // The title row answers the pointer, because it is what a drag takes hold
    // of. The title inside it keeps `Pickable::IGNORE`, so a press anywhere on
    // the row that is not the close button is a press on the row itself — and
    // the tabs are deliberately *not* in it, or dragging a tab sideways would
    // carry the whole sheet with it.
    let title_row = commands
        .spawn((
            TrayGrip,
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                height: px(TRAY_TITLE_H),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    let title = dialog_text(
        commands,
        fonts,
        Phrase::BrowseTitle.text(lang),
        13.0,
        palette::DIALOG_SOFT,
    );
    // The way out. Square, so the cross has a centre to sit in, and with a
    // `Feel`, because every other button in this client breathes.
    let close = commands
        .spawn((
            TrayClose,
            Button,
            Node {
                width: px(22),
                height: px(22),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::new(palette::DIALOG),
            children![(
                // The icon font's own cross. Inter has no U+2715, which is
                // why the button drew as a thin bar for one build.
                Text::new(glyph::CLOSE.to_string()),
                icon_tf(fonts, 12.0),
                TextColor(palette::DIALOG_SOFT),
                Pickable::IGNORE,
            )],
        ))
        .id();
    commands.entity(title_row).add_children(&[title, close]);

    // ---- the zone tabs, "All" first ----
    let tabs = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(4),
                align_items: AlignItems::Center,
                height: px(TRAY_TAB_H),
                overflow: Overflow::clip(),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // "All" carries no count: a sum of a graveyard, a stack and a reveal is a
    // number about nothing.
    // A question that lives in one zone pins the tab to it (W3): every other
    // tab, "All" included, is drawn and is not a button.
    let pinned = browser.locked();
    let mut chips = vec![spawn_tab(
        commands,
        fonts,
        None,
        Phrase::BrowseAll.text(lang).to_string(),
        browser.tab().is_none(),
        pinned.is_some(),
    )];
    for zone in browser.zones(view) {
        chips.push(spawn_tab(
            commands,
            fonts,
            Some(zone),
            zone_label(lang, zone, view, statics),
            browser.tab() == Some(zone),
            pinned.is_some_and(|p| p != zone),
        ));
    }
    commands.entity(tabs).add_children(&chips);

    // ---- what is typed, how it is sorted, and how much is answered ----
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
    let controls = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                height: px(TRAY_CTRL_H),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let said = typing || !browser.filter().trim().is_empty();
    let filter_text = dialog_text(
        commands,
        fonts,
        &hint,
        11.5,
        if said {
            palette::DIALOG_INK
        } else {
            palette::DIALOG_SOFT
        },
    );
    // A sunk field: the ring that says where the typing goes is the border
    // turning candle, not a second fill.
    let filter_line = commands
        .spawn((
            TrayFilter,
            Button,
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                height: percent(100),
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(9)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(if typing {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
            Feel::new(palette::DIALOG),
        ))
        .id();
    commands.entity(filter_line).add_child(filter_text);
    // A library is a hundred cards and a long graveyard is thirty, so "look
    // through this pile" is not a question the pile's own order answers on
    // its own. The key and the direction are two buttons because they are two
    // questions, and the arrow says which way the current one runs rather
    // than being a third state of the key.
    let sort_key = spawn_control(
        commands,
        fonts,
        TraySort { reverse: false },
        browser.sort().label().text(lang),
        9.0,
    );
    let sort_dir = spawn_control(
        commands,
        fonts,
        TraySort { reverse: true },
        if browser.descending() {
            "\u{2193}"
        } else {
            "\u{2191}"
        },
        8.0,
    );
    commands
        .entity(controls)
        .add_children(&[filter_line, sort_key, sort_dir]);
    // The tally. The engine names a minimum and a maximum, so the dialog can
    // say how far along the answer is — and a panel with no question in it (a
    // graveyard opened by hand) says nothing rather than "0 of 0".
    if let Some((min, max)) = answering.and_then(baylee_client_core::Interaction::bounds) {
        let chosen = answering.map_or(0, baylee_client_core::Interaction::declared);
        let words = if min == max {
            Phrase::BrowseTallyExact.fill(lang, &[&chosen.to_string(), &max.to_string()])
        } else {
            Phrase::BrowseTallyUpTo.fill(lang, &[&chosen.to_string(), &max.to_string()])
        };
        let tally = dialog_text(commands, fonts, &words, 10.5, palette::DIALOG_SOFT);
        commands.entity(controls).add_child(tally);
    }

    commands
        .entity(head)
        .add_children(&[title_row, tabs, controls]);

    // ---- the list ----
    //
    // It scrolls, and it is the list rather than the panel: the tabs, the
    // search field and the sort control have to stay where they are while a
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
    let list = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_height: px(0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            // The two halves the sentence above only claimed. An overflow
            // clips and nothing else — Bevy moves the content when
            // `ScrollPosition` changes and nothing changes it on its own —
            // so until `hud::scrolls` existed this list ended at the bottom
            // of the sheet with the rest of the library behind it, and the
            // wheel that should have reached it zoomed the table.
            super::Scrolls,
            ScrollPosition::default(),
        ))
        .id();
    if rows.is_empty() {
        let empty = commands
            .spawn((
                Node {
                    padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_ROW_PAD * 2.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let words = dialog_text(
            commands,
            fonts,
            Phrase::BrowseEmpty.text(lang),
            11.5,
            palette::DIALOG_SOFT,
        );
        commands.entity(empty).add_child(words);
        commands.entity(list).add_child(empty);
    }
    for row in &rows {
        let node = spawn_row(
            commands, lang, row, view, statics, textures, assets, fonts, faces, &mut cards,
        );
        commands.entity(list).add_child(node);
    }

    // ---- the footer ----
    let foot = spawn_footer(commands, fonts, lang, answering);

    // The corner, in the same shape and the same place the card preview's is:
    // one handle, bottom right, both axes. A second handle on every edge is
    // eight more hit targets for a gesture nobody makes on a dialog.
    //
    // Drawn only on a sheet the player arranged. A sheet a *question* opened
    // is centred and reads no stored rectangle, so `input::tray_drag` returns
    // before it ever reaches this handle — and then the rule the pinned tabs
    // and the unlit Confirm already obey applies here too: a control that
    // lights under the pointer and refuses the gesture is worse than no
    // control at all.
    let corner = (!browser.for_choice()).then(|| {
        commands
            .spawn((
                TrayResize,
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(4),
                    bottom: px(4),
                    width: px(22),
                    height: px(22),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                // `Feel::new` shades towards white and **keeps the alpha**, so
                // a control resting at nothing is lifted to a brighter nothing
                // and never answers the pointer. The hot end is stated here
                // for exactly the reason [`Feel::hot`] exists.
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
                children![(
                    Text::new(glyph::EXPAND.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(palette::DIALOG_SOFT),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    });

    commands.entity(panel).add_children(&[head, list]);
    if let Some(foot) = foot {
        commands.entity(panel).add_child(foot);
    }
    if let Some(corner) = corner {
        commands.entity(panel).add_child(corner);
    }
    frame
}

/// The footer, or nothing at all when there is no question to answer.
///
/// Two buttons, and both of them send what [`PromptAction::Confirm`] sends,
/// which is the whole shape of §6's footer: **Confirm is lit only when the
/// answer is complete, and Cancel is drawn only when the minimum is zero.**
/// There is no cancel on the wire — a question that will take an empty answer
/// is answered by sending one, and a question that will not has no way out to
/// offer, so a dialog that drew the button anyway would be promising what the
/// engine cannot deliver.
fn spawn_footer(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    interaction: Option<&baylee_client_core::Interaction>,
) -> Option<Entity> {
    let it = interaction?;
    let (min, _max) = it.bounds()?;
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_FOOT_PAD)),
                border: UiRect::top(px(1)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();

    // Lit only when the answer is complete. An unlit Confirm is not a button
    // at all — no `Button`, no `Feel`, `Pickable::IGNORE` — for the reason a
    // pinned zone tab is not one: a control that lights under the pointer and
    // then refuses the click is worse than one that never invited it.
    let ready = it.can_confirm();
    let confirm = commands
        .spawn((
            Node {
                height: px(TRAY_FOOT_H),
                padding: UiRect::horizontal(px(18)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                ..default()
            },
            BackgroundColor(if ready {
                palette::CANDLE
            } else {
                palette::DIALOG
            }),
            BorderColor::all(if ready {
                palette::CANDLE
            } else {
                palette::DIALOG_LINE
            }),
        ))
        .id();
    let words = dialog_text(
        commands,
        fonts,
        Phrase::BrowseConfirm.text(lang),
        13.0,
        if ready {
            palette::DIALOG
        } else {
            palette::DIALOG_SOFT
        },
    );
    commands.entity(confirm).add_child(words);
    if ready {
        commands.entity(confirm).insert((
            Button,
            PromptButton {
                action: PromptAction::Confirm,
            },
            Feel::new(palette::CANDLE),
        ));
    } else {
        commands.entity(confirm).insert(Pickable::IGNORE);
    }
    commands.entity(foot).add_child(confirm);

    if min == 0 {
        let out = dialog_text(
            commands,
            fonts,
            Phrase::BrowseCancel.text(lang),
            13.0,
            palette::DIALOG_SOFT,
        );
        let cancel = commands
            .spawn((
                TrayCancel,
                Button,
                Node {
                    height: px(TRAY_FOOT_H),
                    padding: UiRect::horizontal(px(14)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                // A ghost button that fills in under the pointer rather than
                // one that lightens: see the resize corner for why a rest of
                // `Color::NONE` has to state its hot end.
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT),
            ))
            .id();
        commands.entity(cancel).add_child(out);
        commands.entity(foot).add_child(cancel);
    }
    Some(foot)
}

/// One zone tab.
///
/// `locked` is the question having pinned a tab: every other one is still
/// drawn, at the weight of something that is not a control, and is not a
/// button. Drawn and not hidden, because a tab that vanished would be saying
/// the graveyard is empty — and what is true is that it is no part of *this*
/// question.
fn spawn_tab(
    commands: &mut Commands,
    fonts: &UiFonts,
    zone: Option<BrowseZone>,
    label: String,
    current: bool,
    locked: bool,
) -> Entity {
    // Candle under the current tab, and the panel's own dark for the ink on
    // it: a lit chip is the one place on this dialog where the accent is a
    // *fill*, which is what keeps the list underneath quiet.
    let (fill, ink) = if current {
        (palette::CANDLE, palette::DIALOG)
    } else if locked {
        // No surface at all under it, and the quieter ink on top: the dialog
        // already means "a different kind of sentence" by that grey, which is
        // exactly what a tab outside the question is.
        (Color::NONE, palette::DIALOG_SOFT)
    } else {
        (palette::DIALOG, palette::DIALOG_INK)
    };
    let text = dialog_text(commands, fonts, &label, 11.0, ink);
    let tab = commands
        .spawn((
            TrayTab { zone },
            Node {
                height: px(TRAY_TAB_H),
                padding: UiRect::horizontal(px(8)),
                align_items: AlignItems::Center,
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
        ))
        .id();
    if locked {
        // No `Button` and no `Feel` either: a control that lights under the
        // pointer and then refuses the click is worse than one that never
        // invited it.
        commands.entity(tab).insert(Pickable::IGNORE);
    } else {
        commands.entity(tab).insert((Button, Feel::new(fill)));
    }
    commands.entity(tab).add_child(text);
    tab
}

/// One control in the head's bottom row: the sort key, or the arrow beside it.
fn spawn_control<C: Component>(
    commands: &mut Commands,
    fonts: &UiFonts,
    marker: C,
    label: &str,
    pad: f32,
) -> Entity {
    let text = dialog_text(commands, fonts, label, 11.0, palette::DIALOG_INK);
    let button = commands
        .spawn((
            marker,
            Button,
            Node {
                height: percent(100),
                padding: UiRect::horizontal(px(pad)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::new(palette::DIALOG),
        ))
        .id();
    commands.entity(button).add_child(text);
    button
}

/// One row of the list: a checkbox, a thumbnail, the name, the cost in pips,
/// the type line and the pile it is in.
#[allow(clippy::too_many_arguments)] // the row, the view, and the stores
#[allow(clippy::too_many_lines)] // six columns in one build
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
    // Candle, and a wash of it rather than a fill: a chosen row is still a row
    // being read. The tick and the ink carry the claim.
    //
    // The hot end is stated both ways round, because `Feel`'s own hover keeps
    // a colour's alpha (see [`Feel::hot`]): an unchosen row rests at nothing
    // and would be lifted to a brighter nothing, and a chosen one rests at a
    // tenth and would be lifted to a paler tenth. A hundred rows that did not
    // answer the pointer is the whole list not answering it.
    let (fill, hot) = if row.selected {
        (palette::CANDLE_WASH, palette::CANDLE_WASH_LIT)
    } else {
        (Color::NONE, palette::DIALOG_LIT)
    };
    let slot = commands
        .spawn((
            TrayCard { object: row.id },
            Button,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(TRAY_GAP),
                width: percent(100),
                // Explicit rather than whatever the thumbnail happens to make
                // it: the sheet's whole height is counted in rows, so a row
                // whose height was an accident of its tallest child would put
                // that arithmetic one text metric away from being wrong.
                height: px(TRAY_ROW_H),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_ROW_PAD)),
                border: UiRect::bottom(px(1)),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(palette::DIALOG_LINE),
            Feel::rising_to(fill, hot),
        ))
        .id();

    // ---- the box, or the place in an ordering ----
    //
    // A number rather than a tick for an ordering, because "third" is not a
    // brighter kind of "chosen" — and it stands in the box's own column, so a
    // list of ordered cards reads down the same gutter a list of ticked ones
    // does. A row the question will not take leaves the column empty rather
    // than drawing a box that cannot be ticked.
    //
    // Four states decided before the node is spawned rather than patched into
    // it afterwards: a radius is a field of `Node` and not a component of its
    // own, so "insert a rounder corner" would mean writing the whole `Node`
    // back over itself.
    let (fill, edge, radius, glyph_in) = if row.place.is_some() {
        (
            palette::CANDLE,
            palette::CANDLE,
            TRAY_BOX / 2.0,
            row.place.map(|place| place.to_string()),
        )
    } else if row.selected {
        (
            palette::CANDLE,
            palette::CANDLE,
            3.0,
            Some(glyph::CHECK.to_string()),
        )
    } else if row.selectable {
        (Color::NONE, palette::DIALOG_SOFT, 3.0, None)
    } else {
        (Color::NONE, Color::NONE, 3.0, None)
    };
    let mark = commands
        .spawn((
            Node {
                width: px(TRAY_BOX),
                height: px(TRAY_BOX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(radius)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Pickable::IGNORE,
        ))
        .id();
    if let Some(inside) = glyph_in {
        // The ordering's number is set in the text face and the tick in the
        // icon one, because a tick is a glyph Inter does not have.
        let face = if row.place.is_some() {
            tf(fonts, 9.5)
        } else {
            icon_tf(fonts, 9.0)
        };
        let ink = commands
            .spawn((
                Text::new(inside),
                face,
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(mark).add_child(ink);
    }
    commands.entity(slot).add_child(mark);

    // ---- the picture ----
    //
    // `built` is deliberately not passed to it: a built face draws the name,
    // the cost and the type line onto the card, and at thirty pixels wide all
    // three would be a grey smear. The row says those three things beside it,
    // in letters a person can read.
    let thumb = if let Some(key) = row.art {
        let image = textures.get(key, statics, assets);
        spawn_card_art(
            commands,
            lang,
            image,
            None,
            TRAY_THUMB_W,
            TRAY_THUMB_H,
            crate::face::Detail::Compact,
            fonts,
            // No keyword sheath: nothing in the browser is on a battlefield,
            // and a card in a graveyard wearing an indestructible border
            // would be claiming something the rules do not say.
            CardLook::art(key, finish_of(statics, Some(key)), 0),
            cards.as_deref_mut(),
        )
    } else {
        // A token in a graveyard, or a card this seat may not identify.
        commands
            .spawn((
                Node {
                    width: px(TRAY_THUMB_W),
                    height: px(TRAY_THUMB_H),
                    flex_shrink: 0.0,
                    border_radius: card_radius(TRAY_THUMB_W),
                    ..default()
                },
                BackgroundColor(palette::DIALOG_LIT),
            ))
            .id()
    };
    commands.entity(thumb).insert(Pickable::IGNORE);
    commands.entity(slot).add_child(thumb);

    // ---- the name ----
    //
    // The one thing on the row that grows, with `flex_basis: 0` beside it:
    // grow alone divides only the slack left after every fixed column, which
    // on a narrow sheet is nothing at all.
    let name = dialog_text(
        commands,
        fonts,
        &row.name,
        TRAY_NAME_SIZE,
        palette::DIALOG_INK,
    );
    commands.entity(name).insert(Node {
        flex_grow: 1.0,
        flex_basis: px(0),
        min_width: px(0),
        overflow: Overflow::clip(),
        ..default()
    });
    commands.entity(slot).add_child(name);

    // ---- the cost, drawn and not spelled ----
    //
    // The face is what carries it: a `BrowseRow` has the projected mana
    // *value*, which is what the sort key reads, and a number is not a price.
    let built = view.object(row.id).map(|o| faces.facts(o));
    let pips = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(2),
                flex_shrink: 0.0,
                // A reserve rather than a fit, so the type lines beside them
                // start in one column down the whole list. A cost longer than
                // four pips takes the room it needs and pushes the name in,
                // which is the right way round: the name has the slack.
                min_width: px(TRAY_COST_W),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for symbol in built.iter().flat_map(|face| face.cost.iter()) {
        let pip = crate::manaui::spawn_pip(
            commands,
            fonts,
            baylee_client_core::manapip::pip(*symbol),
            TRAY_PIP,
        );
        commands.entity(pips).add_child(pip);
    }
    commands.entity(slot).add_child(pips);

    // ---- the type line ----
    let types = built
        .as_ref()
        .map_or_else(String::new, |f| f.type_line.clone());
    let type_line = dialog_text(
        commands,
        fonts,
        &types,
        TRAY_TYPE_SIZE,
        palette::DIALOG_SOFT,
    );
    commands.entity(type_line).insert(Node {
        width: px(TRAY_TYPE_W),
        flex_shrink: 0.0,
        overflow: Overflow::clip(),
        ..default()
    });
    commands.entity(slot).add_child(type_line);

    // ---- which pile it is in ----
    //
    // The bare zone word, with no seat on it: at a table of four the tab above
    // already says whose pile is being looked through, and a seat name in a
    // badge this size is a smear. A token says so here instead — a graveyard
    // holds cards and tokens together and they are not the same thing, since a
    // token ceases to exist the next time state-based actions are checked (CR
    // 111.7), so a row that looked like a card would invite a player to plan
    // around something already gone.
    let badge_words = if row.token {
        Phrase::IsToken.text(lang).to_string()
    } else {
        row.zone.label().text(lang).to_string()
    };
    let badge_text = dialog_text(
        commands,
        fonts,
        &badge_words,
        TRAY_BADGE_SIZE,
        palette::DIALOG_SOFT,
    );
    let badge = commands
        .spawn((
            Node {
                width: px(TRAY_BADGE_W),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(px(5)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(badge).add_child(badge_text);
    commands.entity(slot).add_child(badge);

    slot
}

/// What a zone tab reads. A pile belonging to a seat says whose it is,
/// because at a table of four "Graveyard" alone names nothing — and every tab
/// says how many cards are in it, which is the one thing the deleted pile
/// chips carried that nothing else on the sheet does.
///
/// The count goes in brackets rather than after a separator because the
/// dialog's typography already means something by a bracket: [`dialog_text`]
/// hands a bracketed run to [`palette::DIALOG_SOFT`], so "Graveyard (12)" is
/// drawn as a name with a grey aside beside it and reads as one.
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

    /// The sheet a player has never moved is exactly one row wide.
    ///
    /// `Placement::DEFAULT_W` is a number in the renderer-free half, where it
    /// can be tested but where a row's columns do not exist; the arithmetic
    /// that produced it lives here. This is the seam between them, so a column
    /// widened on one side cannot silently leave the other with a name that
    /// no longer has its measure.
    #[test]
    fn the_default_width_is_one_whole_row() {
        let off = (Placement::DEFAULT_W - TRAY_PANEL_W).abs();
        assert!(
            off <= 1.0,
            "a row is {TRAY_PANEL_W}, the sheet opens at {} ({off} out)",
            Placement::DEFAULT_W
        );
    }

    /// And it opens showing half of a ninth row.
    ///
    /// The same seam one axis over, and the half is the whole point: a list
    /// cut off at a row boundary looks like a list that ends there.
    #[test]
    fn the_default_height_shows_half_a_row() {
        let want = TRAY_CHROME_H + TRAY_ROWS * TRAY_ROW_H;
        let off = (Placement::DEFAULT_H - want).abs();
        assert!(
            off <= 1.0,
            "{TRAY_ROWS} rows is {want}, the sheet opens at {} ({off} out)",
            Placement::DEFAULT_H
        );
        // Genuinely half: the list is cut through a row rather than between
        // two, which is what says there is more below.
        let spare = (Placement::DEFAULT_H - TRAY_CHROME_H) % TRAY_ROW_H;
        assert!(
            spare > TRAY_ROW_H * 0.25 && spare < TRAY_ROW_H * 0.75,
            "the bottom row is cut at {spare} of {TRAY_ROW_H}, which reads as a whole one"
        );
    }

    /// The smallest sheet still has a row in it worth reading.
    ///
    /// Both floors are one row's arithmetic: the width is the fixed columns
    /// plus ten characters of name, and the height is the chrome plus two
    /// whole rows. A sheet dragged smaller than either is a sheet with no
    /// list left in it.
    #[test]
    fn the_floor_is_a_row_that_can_still_be_read() {
        let fixed = TRAY_PANEL_W - TRAY_NAME_W - TRAY_TYPE_W;
        let ten = 10.0 * TRAY_CH * TRAY_NAME_SIZE;
        assert!(
            (Placement::MIN_W - (fixed + ten)).abs() <= 1.0,
            "ten characters of name is {}, the floor is {}",
            fixed + ten,
            Placement::MIN_W
        );
        let two = TRAY_CHROME_H + 2.0 * TRAY_ROW_H;
        assert!(
            (Placement::MIN_H - two).abs() <= 1.0,
            "two rows is {two}, the floor is {}",
            Placement::MIN_H
        );
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
    /// the dialog greys.
    ///
    /// Two claims in one, because they are one decision: the count is drawn
    /// as an aside rather than as part of the name, and [`dialog_text`]
    /// decides that by finding a bracket. A count appended with a separator
    /// would read at full ink weight and make every tab look twice as long.
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
            "the count is not in the aside register the dialog greys"
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

    /// The footer is exactly what the question allows, and nothing more.
    ///
    /// Three claims, one per state, and they are the whole of §6's footer.
    /// **Confirm is a control only when the answer is complete** — an unlit
    /// one carries no `Button` at all, for the reason a pinned zone tab
    /// carries none: a thing that lights under the pointer and then refuses
    /// the click is worse than one that never invited it. And **Cancel is
    /// drawn only when the minimum is zero**, because there is no cancel on
    /// the wire: it is `Interaction::confirm` sending an empty answer, so a
    /// question that will not take one has no way out to offer.
    #[test]
    fn the_footer_offers_only_what_the_question_allows() {
        use baylee_core::ids::{ObjectId, PlayerId};
        use baylee_engine::choice::{ChoicePrompt, Pending};

        fn asked(min: u8, picks: &[u32]) -> baylee_client_core::Interaction {
            let mut it = baylee_client_core::Interaction::new(
                Pending::ChooseCards {
                    player: PlayerId::new(0),
                    options: (1..4).map(|n| ObjectId::new(n, 0)).collect(),
                    min,
                    max: 3,
                    prompt: ChoicePrompt::SearchLibrary,
                },
                PlayerId::new(0),
            );
            for id in picks {
                it.toggle(ObjectId::new(*id, 0));
            }
            it
        }

        /// Builds one footer and reports `(confirm is a control, cancel is drawn)`.
        fn footer_of(it: &baylee_client_core::Interaction) -> (bool, bool) {
            let mut app = App::new();
            let fonts = UiFonts {
                text: Handle::default(),
                italic: Handle::default(),
                icons: Handle::default(),
                mana: Handle::default(),
            };
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let foot = {
                let mut commands = Commands::new(&mut queue, app.world());
                spawn_footer(&mut commands, &fonts, Lang::En, Some(it)).expect("a question has one")
            };
            queue.apply(app.world_mut());
            let kids: Vec<_> = app
                .world()
                .entity(foot)
                .get::<Children>()
                .expect("a footer has buttons")
                .iter()
                .collect();
            let lit = kids
                .iter()
                .any(|e| app.world().entity(*e).contains::<PromptButton>());
            let out = kids
                .iter()
                .any(|e| app.world().entity(*e).contains::<TrayCancel>());
            (lit, out)
        }

        assert_eq!(
            footer_of(&asked(1, &[])),
            (false, false),
            "an incomplete answer offered a Confirm, or a way out that does \
             not exist"
        );
        assert_eq!(
            footer_of(&asked(1, &[1])),
            (true, false),
            "a complete answer could not be sent"
        );
        assert_eq!(
            footer_of(&asked(0, &[])),
            (true, true),
            "a question that takes an empty answer drew no way out"
        );
    }

    /// W2's whole claim, written as an order: the veil goes over what answers
    /// nothing and under everything that does.
    ///
    /// One assertion rather than five literals in four files, because that is
    /// the failure it guards. A `ZIndex` is local to a parent's children, so
    /// these five mean something only *against each other* — a veil written
    /// as a 3 beside a prompt slip that had never been given a number at all
    /// is a dim drawn over the sentence stating the question.
    ///
    /// In `const` blocks, so the order is checked when the crate is *built*
    /// and not when its tests are run. It is still a named test because the
    /// rule wants somewhere to be written down in words, and because a
    /// constant that silently stopped being compared would be no rule at all.
    #[test]
    fn the_veil_lies_over_the_table_and_under_the_question() {
        const {
            assert!(
                Z_STACK < Z_VEIL && Z_HAND < Z_VEIL,
                "the stack and the hand answer nothing here and go dark with the table"
            );
            assert!(
                Z_VEIL < Z_SLIP,
                "the slip is the sentence saying what the question is"
            );
            assert!(Z_SLIP < Z_SHEET, "the dialog is what the slip is about");
            assert!(
                Z_SHEET < Z_PREVIEW,
                "a card held up to the light is held over whatever raised it"
            );
        }
    }

    /// The fade rises to exactly the veil's own alpha and falls back to
    /// nothing — and it survives the rebuild that every tick of a checkbox
    /// causes, which is the whole reason the number is not on the node.
    #[test]
    fn the_veil_rises_while_the_question_stands_and_falls_when_it_is_answered() {
        use baylee_client_core::test_support::{ViewBuilder, printed};
        use baylee_core::ids::{ObjectId, PlayerId};
        use baylee_engine::choice::{ChoicePrompt, Pending};
        use std::time::Duration;

        fn tick(app: &mut App) {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(16));
            app.update();
        }
        fn alpha(app: &mut App) -> f32 {
            let mut found = app
                .world_mut()
                .query_filtered::<&BackgroundColor, With<TableVeil>>();
            found.iter(app.world()).next().expect("a veil").0.alpha()
        }
        fn raise(app: &mut App) {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            {
                let mut commands = Commands::new(&mut queue, app.world());
                spawn_veil(&mut commands);
            }
            queue.apply(app.world_mut());
        }

        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<Veil>()
            .init_resource::<crate::Duel>()
            .add_systems(Update, dim_the_table);
        raise(&mut app);
        assert!(alpha(&mut app).abs() < 1e-6, "it is spawned clear");

        // A search, through the same door the client uses: cards shown, every
        // answer among them.
        let view = ViewBuilder::new(2)
            .with_looking_at((10..13).map(|s| printed(s, 0, "Forest", 1)).collect())
            .build();
        let search = baylee_client_core::Interaction::new(
            Pending::ChooseCards {
                player: PlayerId::new(0),
                options: (10..13).map(|n| ObjectId::new(n, 0)).collect(),
                min: 1,
                max: 1,
                prompt: ChoicePrompt::SearchLibrary,
            },
            PlayerId::new(0),
        );
        app.world_mut()
            .resource_mut::<crate::Duel>()
            .browser
            .follow(&view, Some(&search));
        for _ in 0..3 {
            tick(&mut app);
        }
        let rising = alpha(&mut app);
        assert!(
            rising > 0.0 && rising < palette::TABLE_VEIL.alpha(),
            "three frames in it should be on its way and not there yet: {rising}"
        );

        // A tick of a checkbox rebuilds the whole overlay, veil included.
        let standing: Vec<_> = {
            let mut found = app.world_mut().query_filtered::<Entity, With<TableVeil>>();
            found.iter(app.world()).collect()
        };
        for entity in standing {
            app.world_mut().entity_mut(entity).despawn();
        }
        raise(&mut app);
        tick(&mut app);
        assert!(
            alpha(&mut app) > rising,
            "the rebuilt veil started again from nothing — the fade is on the \
             node instead of in `Veil`"
        );

        for _ in 0..60 {
            tick(&mut app);
        }
        assert!(
            (alpha(&mut app) - palette::TABLE_VEIL.alpha()).abs() < 1e-4,
            "it settles at the veil's own alpha, not a hair under it"
        );

        // Answered. The number falls whether or not a node is left to paint.
        app.world_mut().resource_mut::<crate::Duel>().browser = Browser::new();
        for _ in 0..60 {
            tick(&mut app);
        }
        assert!(
            alpha(&mut app).abs() < 1e-6,
            "and is clear again for the next one"
        );
    }

    /// Nothing on the dialog is drawn in the teal the redesign retires.
    ///
    /// `palette::ACCENT` is what "this is asking you something" used to be
    /// said in, and §1 gives that job to candle at two energies. The check is
    /// on the source because what is being held is a rule about the whole
    /// file, not about one node — and it is the counterpart of
    /// `the_sheet_writes_no_letters_in_brass`, which holds the same kind of
    /// rule over the two parchment surfaces.
    #[test]
    fn the_dialog_says_nothing_in_teal() {
        // Assembled rather than written out, or the needle is in the
        // haystack and this test fails on its own source line.
        let teal = format!("palette::{}", "ACCENT");
        for line in include_str!("tray.rs").lines() {
            let code = line.split("//").next().unwrap_or(line);
            assert!(
                !code.contains(&teal),
                "the accent is the teal §1 retires: {line}"
            );
        }
    }
}
