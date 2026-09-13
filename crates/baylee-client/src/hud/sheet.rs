//! The ability sheet: what a permanent can do, on parchment, beside the card.
//!
//! It replaces a row of buttons in the prompt bar. Those buttons carried the
//! ability's *cost* as their whole label — `{2}, {T}` — because a button on a
//! bar has room for four words, so a player read what an ability charged and
//! never what it did, and the bar was at the bottom of the window while the
//! permanent was on the table.
//!
//! What is here is a sheet of paper laid beside the card it belongs to: the
//! permanent's name, a numbered row per thing it can do with the ability's own
//! printed sentence on it, and the cost as pips on the right where a cost
//! belongs. `docs/redesign-proposal.md` §7 is the design.
//!
//! # Two trees, because two things change at different rates
//!
//! [`sync_ability_sheet`] builds it when what it *says* changes and
//! [`place_ability_sheet`] moves it every frame, which is the split the seat
//! bars already use and for a sharper version of the same reason: a card
//! glides. Nothing on this table is positioned directly — `table::sync_scene`
//! writes a `Motion` target and `table::glide` moves the card there — so a
//! sheet anchored to where the card is *going* arrives before the card does,
//! and a sheet anchored to where it was drawn last frame follows it honestly.
//!
//! The placement reads the rig the camera was set from rather than the
//! camera's propagated `GlobalTransform`, for the reason in
//! [`crate::hud::seatbar`]: `bevy_ui` runs `UiSystems::Layout` before
//! `TransformSystems::Propagate`, so a `Node` written from the propagated
//! transform is a frame stale.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;
use baylee_client_core::abilitysheet;
use baylee_client_core::card_face::TextBlock;
use bevy::text::LetterSpacing;

/// How wide the sheet is.
///
/// Wide enough for a printed sentence at 13 px without the rows turning into
/// paragraphs — about forty characters, which covers most of what an
/// activated ability says — and narrow enough that a sheet standing beside a
/// permanent in the middle lane does not reach either edge of the window.
const SHEET_W: f32 = 300.0;

/// The air between the card and the sheet's near edge.
const SHEET_GAP: f32 = 10.0;

/// How close to the window's edge the sheet may come.
const SHEET_MARGIN: f32 = 12.0;

/// The digit roundel's diameter.
const ROUNDEL: f32 = 21.0;

/// The wash under the row that is armed — [`palette::BRASS`] at 16%.
///
/// It sits on an opaque sheet and not on felt, which is the whole reason a
/// translucent fill is allowed here at all: parchment over dark cloth goes
/// grey, and parchment over parchment is warmer parchment.
const ARMED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.16);

/// The wash under an armed row with the pointer on it.
///
/// Brighter than [`ARMED_WASH`] and not dimmer: `Feel` crossfades from the
/// resting colour to this one, so a hot colour below the base would make
/// hovering the row a player has already committed to *take light away*.
const ARMED_HOT: Color = Color::srgba(0.788, 0.635, 0.153, 0.24);

/// The wash under the row the keyboard is on.
///
/// The same claim at half the weight, because the two are different claims
/// about the same row: the cursor is *where a key would land* and the arming
/// is *what a key has already done*.
const PICKED_WASH: Color = Color::srgba(0.788, 0.635, 0.153, 0.08);

/// The parent of the sheet, so one despawn clears it.
#[derive(Component)]
pub struct AbilitySheetRoot;

/// The sheet, and where it was last put.
#[derive(Component)]
pub struct AbilitySheet {
    /// The permanent it belongs to.
    pub object: ObjectId,
    /// The corner it is already at, so a camera standing still costs one
    /// comparison instead of a relayout of the whole sheet.
    placed: Option<Vec2>,
}

/// The tenth row, which turns the page.
#[derive(Component)]
pub struct SheetPager;

/// What the sheet is currently saying, so it is rebuilt only when that
/// changes.
///
/// The options themselves are fingerprinted by their labels rather than
/// stored: the list is rebuilt from `LegalActions` on every read anyway, and
/// what the sheet draws is exactly what those labels say.
#[derive(Resource, Default)]
pub struct SheetRevision {
    object: Option<ObjectId>,
    page: usize,
    pick: usize,
    armed: Option<usize>,
    fingerprint: Vec<String>,
    lang: Option<Lang>,
    /// How many printings had text when this was drawn.
    ///
    /// The same term `HudRevision` carries and for the same reason: what a
    /// row *says* is the card text, which arrives over the network after the
    /// sheet can already be opened. Nothing else in the fingerprint moves
    /// when it lands, so a sheet opened first would keep showing the fallback
    /// label for as long as it stood.
    texts: usize,
}

/// Which row of `options` the armed deed is, if any of them is.
///
/// By the action and not by an index, for the reason [`crate::Deed::Ability`]
/// carries the action: the list is rebuilt every frame and an index stored
/// across one of those rebuilds names whatever moved into that position.
fn armed_row(
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
) -> Option<usize> {
    let armed = duel.armed.as_ref()?;
    if armed.object != object {
        return None;
    }
    let crate::Deed::Ability(action) = &armed.deed else {
        return None;
    };
    options.iter().position(|o| &o.action == action)
}

/// Builds the sheet when what it says changes.
#[allow(clippy::too_many_arguments)]
pub fn sync_ability_sheet(
    mut commands: Commands,
    duel: Res<Duel>,
    mut revision: ResMut<SheetRevision>,
    existing: Query<Entity, With<AbilitySheetRoot>>,
    fonts: Res<UiFonts>,
    sheets: Res<UiSheets>,
    faces: Res<crate::cardtext::CardTexts>,
    settings: Res<crate::settings::ClientSettings>,
) {
    let lang = Lang::of(&settings.lang);
    let open = duel
        .ability_menu
        .and_then(|object| Some((object, ability_options(&duel, lang, object)?)))
        .filter(|(_, options)| options.len() > 1);

    let Some((object, options)) = open else {
        // Closed. Guarded on the revision so an interface with no sheet in it
        // does not run a query and a despawn loop on every frame of the game.
        if revision.object.is_some() {
            *revision = SheetRevision::default();
            for entity in &existing {
                commands.entity(entity).despawn();
            }
        }
        return;
    };

    let page = abilitysheet::clamp(options.len(), duel.ability_page);
    let armed = armed_row(&duel, object, &options);
    let fingerprint: Vec<String> = options.iter().map(|o| o.label.clone()).collect();
    if revision.object == Some(object)
        && revision.page == page
        && revision.pick == duel.ability_pick
        && revision.armed == armed
        && revision.lang == Some(lang)
        && revision.fingerprint == fingerprint
        && revision.texts == faces.len()
    {
        return;
    }
    revision.object = Some(object);
    revision.page = page;
    revision.pick = duel.ability_pick;
    revision.armed = armed;
    revision.lang = Some(lang);
    revision.fingerprint = fingerprint;
    revision.texts = faces.len();

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    let root = commands
        .spawn((
            AbilitySheetRoot,
            crate::table::DuelStage,
            Node {
                width: percent(100),
                height: percent(100),
                ..default()
            },
            // The felt between the card and the sheet is not the sheet.
            Pickable::IGNORE,
            // This is a **root**, not a child of `HudRoot`, so the number
            // does not mean what the four in `hud::Z_SLIP` means. bevy's
            // `ui_stack_system` sorts roots by `(GlobalZIndex, ZIndex)` and
            // then walks each subtree, so a root at `(0, 4)` stands over the
            // whole of a root at `(0, 0)` — this sheet is over every part of
            // the overlay, the prompt slip and the hover preview included,
            // which is *not* what it should be and is why the number is
            // written down here with what it actually does. Re-homing it
            // needs the third retained tree to be ordered against the first,
            // which is a decision of its own and not this one's to take.
            ZIndex(4),
        ))
        .id();
    let sheet = spawn_sheet(
        &mut commands,
        &fonts,
        &sheets,
        &faces,
        lang,
        &duel,
        object,
        &options,
        page,
        armed,
    );
    commands.entity(root).add_child(sheet);
}

/// One sheet, built.
#[allow(clippy::too_many_arguments)]
fn spawn_sheet(
    commands: &mut Commands,
    fonts: &UiFonts,
    sheets: &UiSheets,
    faces: &crate::cardtext::CardTexts,
    lang: Lang,
    duel: &Duel,
    object: ObjectId,
    options: &[crate::abilities::AbilityOption],
    page: usize,
    armed: Option<usize>,
) -> Entity {
    let sheet = commands
        .spawn((
            AbilitySheet {
                object,
                placed: None,
            },
            Node {
                position_type: PositionType::Absolute,
                width: px(SHEET_W),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(16), px(13)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(6)),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT),
            BorderColor::all(palette::PARCHMENT_EDGE),
            BoxShadow::new(
                palette::SHEET_SHADOW,
                Val::Px(0.0),
                Val::Px(6.0),
                Val::Px(0.0),
                Val::Px(18.0),
            ),
            // Deliberately *not* `Pickable::IGNORE`. The root is, and the
            // grain is, so the felt around the sheet still belongs to the
            // table — but a click on the paper itself has to land on the
            // paper. Falling through would reach the card underneath and
            // re-run `activate_card`, which puts the page and the cursor
            // back to the top of the list a player was reading.
        ))
        .id();
    commands.spawn(sheet_surface(sheets)).insert(ChildOf(sheet));

    spawn_head(commands, fonts, lang, duel, object, sheet);

    // ---- the rows --------------------------------------------------------
    for at in abilitysheet::rows(options.len(), page) {
        let digit = digit_of(at).unwrap_or('?');
        let row = spawn_row(
            commands,
            fonts,
            faces,
            duel,
            object,
            at,
            &options[at],
            digit,
            armed == Some(at),
            duel.ability_pick == at,
        );
        commands.entity(sheet).add_child(row);
    }
    if abilitysheet::paged(options.len()) {
        let row = spawn_pager(
            commands,
            fonts,
            lang,
            page,
            abilitysheet::pages(options.len()),
        );
        commands.entity(sheet).add_child(row);
    }

    spawn_foot(commands, fonts, lang, armed, sheet);
    sheet
}

/// The permanent's name, and a line saying what the list under it is.
fn spawn_head(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    duel: &Duel,
    object: ObjectId,
    sheet: Entity,
) {
    let name = duel
        .view
        .as_ref()
        .and_then(|v| v.object(object))
        .map_or_else(String::new, |o| o.name.clone());
    let head = commands
        .spawn((
            Text::new(name),
            tf(fonts, 16.0),
            TextColor(palette::SLIP_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(head);
    let sub = commands
        .spawn((
            Text::new(Phrase::WhatItCanDo.text(lang).to_uppercase()),
            tf(fonts, 10.0),
            // The one place in the interface with letter-spacing, and it is
            // what makes ten uppercase characters read as a label rather
            // than as a shout. Its own component in Bevy 0.19 and not a
            // field on `TextFont`.
            LetterSpacing::Rem(0.08),
            TextColor(palette::SLIP_SOFT),
            Node {
                margin: UiRect::top(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(sub);
    let hair = rule(commands, 10.0);
    commands.entity(sheet).add_child(hair);
}

/// The rule under the rows, and the one line that says which key does what.
///
/// Two halves and one separator: what the digit on the armed row would do if
/// it were pressed again, and the way out. With nothing armed the first half
/// names the gesture instead, because a sheet that said only "Esc closes"
/// would be a list with no stated way to answer it.
fn spawn_foot(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    armed: Option<usize>,
    sheet: Entity,
) {
    let hair = rule(commands, 8.0);
    commands.entity(sheet).add_child(hair);
    let hint = match armed.and_then(digit_of) {
        Some(digit) => Phrase::SheetPressAgain.fill(lang, &[&digit.to_string()]),
        None => Phrase::SheetDigitPicks.text(lang).to_string(),
    };
    let foot = commands
        .spawn((
            Text::new(format!(
                "{hint} \u{b7} {}",
                Phrase::SheetEscCloses.text(lang)
            )),
            tf(fonts, 10.5),
            TextColor(palette::SLIP_ASIDE),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(sheet).add_child(foot);
}

/// The digit drawn on the row at `at`, wherever on its page that is.
fn digit_of(at: usize) -> Option<char> {
    u32::try_from(at % abilitysheet::PAGE + 1)
        .ok()
        .and_then(|place| char::from_digit(place, 10))
}

/// A hairline across the sheet, with `above` pixels of air over it.
fn rule(commands: &mut Commands, above: f32) -> Entity {
    commands
        .spawn((
            Node {
                height: px(1),
                margin: UiRect::new(px(0), px(0), px(above), px(2)),
                ..default()
            },
            BackgroundColor(palette::PARCHMENT_EDGE),
            Pickable::IGNORE,
        ))
        .id()
}

/// One row: the roundel, what the ability does, what it costs.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
fn spawn_row(
    commands: &mut Commands,
    fonts: &UiFonts,
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    index: usize,
    option: &crate::abilities::AbilityOption,
    digit: char,
    armed: bool,
    picked: bool,
) -> Entity {
    let wash = if armed {
        ARMED_WASH
    } else if picked {
        PICKED_WASH
    } else {
        Color::NONE
    };
    let row = commands
        .spawn((
            // The *whole row* is the button, not the roundel on it: a target
            // 21 pixels across beside a sentence that is not clickable is a
            // row a player aims at and misses.
            crate::hud::AbilityButton { index },
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                padding: UiRect::axes(px(4), px(6)),
                margin: UiRect::horizontal(px(-4)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(wash),
            Feel::rising_to(wash, if armed { ARMED_HOT } else { PICKED_WASH }),
        ))
        .id();

    let roundel = commands
        .spawn((
            Node {
                width: px(ROUNDEL),
                height: px(ROUNDEL),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(ROUNDEL / 2.0)),
                ..default()
            },
            BackgroundColor(if armed {
                palette::BRASS
            } else {
                palette::SLIP_GHOST
            }),
            BorderColor::all(if armed {
                palette::PARCHMENT_INK
            } else {
                palette::PARCHMENT_SOFT
            }),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(digit.to_string()),
            tf(fonts, 11.0),
            // Dark on gold in both states. White on brass fails contrast, and
            // an armed row is the one a player is about to commit to.
            TextColor(palette::PARCHMENT_INK),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(roundel).add_child(glyph);
    commands.entity(row).add_child(roundel);

    let says = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    // What the ability *does*, if the card's text is here to say it, and what
    // it costs if it is not: the fallback label is `printed_label`'s answer,
    // which is the cost.
    let printed = row_text(faces, duel, object, option);
    let blocks = printed
        .clone()
        .unwrap_or_else(|| vec![TextBlock::Rules(option.label.clone())]);
    for block in blocks {
        let (words, colour) = match &block {
            TextBlock::Rules(t) => (t.clone(), palette::SLIP_INK),
            TextBlock::Reminder(t) => (t.clone(), palette::SLIP_ASIDE),
        };
        let line = crate::manaui::spawn_rich(commands, fonts, &words, 13.0, colour);
        commands.entity(says).add_child(line);
    }
    commands.entity(row).add_child(says);

    // The pip column is skipped where it would print the row's own words a
    // second time. Offline there is no card text at all, so every printed
    // ability falls back to its cost — and a row reading `{2}, {T}` on the
    // left and `{2}, {T}` on the right is the duplication the sentence-first
    // layout exists to remove, not a cost drawn where a cost belongs.
    let repeats = printed.is_none() && option.cost.as_deref() == Some(option.label.as_str());
    if let Some(cost) = option.cost.as_deref().filter(|_| !repeats) {
        let pips = crate::manaui::spawn_rich(commands, fonts, cost, 13.0, palette::SLIP_SOFT);
        commands.entity(row).add_child(pips);
    }
    row
}

/// The tenth row.
fn spawn_pager(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    page: usize,
    pages: usize,
) -> Entity {
    let row = commands
        .spawn((
            SheetPager,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(11),
                padding: UiRect::axes(px(4), px(6)),
                margin: UiRect::horizontal(px(-4)),
                border_radius: BorderRadius::all(px(4)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Feel::rising_to(Color::NONE, PICKED_WASH),
        ))
        .id();
    let roundel = commands
        .spawn((
            Node {
                width: px(ROUNDEL),
                height: px(ROUNDEL),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(ROUNDEL / 2.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(palette::PARCHMENT_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    let glyph = commands
        .spawn((
            Text::new(abilitysheet::PAGER.to_string()),
            tf(fonts, 11.0),
            TextColor(palette::SLIP_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(roundel).add_child(glyph);
    commands.entity(row).add_child(roundel);
    let says = commands
        .spawn((
            Text::new(
                Phrase::SheetMorePage.fill(lang, &[&(page + 1).to_string(), &pages.to_string()]),
            ),
            tf(fonts, 12.0),
            TextColor(palette::SLIP_SOFT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(says);
    row
}

/// What a row says: the ability's own printed sentence, or nothing.
///
/// Three things have to line up before the sentence can be drawn and any of
/// them may be missing — the generated table has to know which sentence it is,
/// the permanent has to still carry a printing, and that printing's text has
/// to have arrived, which it has not offline. That is the same triple
/// `stack_sentence` walks.
///
/// `None` rather than the fallback itself, because the *caller* has to know
/// which of the two it got: the fallback label is what the ability costs, and
/// the row draws that again on the right as pips.
///
/// The cost is cut off the front of a real sentence for the same reason — see
/// [`abilitysheet::effect`](baylee_client_core::abilitysheet::effect).
fn row_text(
    faces: &crate::cardtext::CardTexts,
    duel: &Duel,
    object: ObjectId,
    option: &crate::abilities::AbilityOption,
) -> Option<Vec<TextBlock>> {
    let text = option.printed?;
    let print = duel.view.as_ref()?.object(object)?.card?.print;
    let card = faces.get(print, text.face)?;
    let blocks =
        baylee_client_core::card_face::sentence_blocks(&card.oracle_text, text.line, text.of)?;
    let blocks = abilitysheet::effect(blocks);
    (!blocks.is_empty()).then_some(blocks)
}

/// Follows the card with the sheet that is already built.
///
/// Every frame, because the card is gliding — see the module header. The
/// write is guarded on where the sheet already is, so a card standing still
/// costs one comparison and no relayout.
pub fn place_ability_sheet(
    shown: Res<crate::table::ShownRig>,
    windows: Query<&Window>,
    cards: Query<(&crate::table::CardVisual, &Transform)>,
    mut sheet: Query<(&mut AbilitySheet, &mut Node, &bevy::ui::ComputedNode)>,
) {
    let Ok((mut sheet, mut node, computed)) = sheet.single_mut() else {
        return;
    };
    let (Some(rig), Ok(window)) = (shown.rig(), windows.single()) else {
        return;
    };
    let size = Vec2::new(window.width(), window.height());
    let lens = crate::table::Lens::new(rig, size);
    let Some((mid, card)) = cards
        .iter()
        .find(|(visual, _)| visual.object == sheet.object)
        .and_then(|(_, at)| crate::table::card_box(&lens, at))
    else {
        // The card is not on the table — it left, or the camera cannot see
        // it. Hidden rather than despawned, the way a seat bar is: the sheet
        // is closed by the input path and not by the camera.
        if node.display != Display::None {
            sheet.placed = None;
            node.display = Display::None;
        }
        return;
    };
    // `ComputedNode` is bevy_ui's own layout, which is a frame old here for
    // the reason the module header gives. A frame is nothing to a sheet that
    // stands for as long as a player is reading it, and it is the only thing
    // that knows how tall a sheet of text came out.
    let height = computed.size().y * computed.inverse_scale_factor;
    let corner = corner_for(mid, card, height, size);
    if sheet.placed == Some(corner) && node.display == Display::Flex {
        return;
    }
    sheet.placed = Some(corner);
    node.display = Display::Flex;
    node.left = px(corner.x);
    node.top = px(corner.y);
}

/// Where the sheet's top-left corner goes.
///
/// `mid` and `card` are the permanent's projected centre and the box it
/// covers ([`crate::table::card_box`]), `height` is how tall the sheet came
/// out and `window` is the canvas.
///
/// Above the card by default, because that is where a sheet covers the fewest
/// other permanents: a board is read from the far edge towards the player's
/// own hand, so the space above a card holds what has already been read.
/// Below only when there is no room above — and *clamped* rather than allowed
/// to hang off either edge, because a sheet half outside the window is a list
/// with rows the player cannot see and cannot click.
fn corner_for(mid: Vec2, card: Vec2, height: f32, window: Vec2) -> Vec2 {
    let above = mid.y - card.y / 2.0 - SHEET_GAP - height;
    let top = if above >= SHEET_MARGIN {
        above
    } else {
        (mid.y + card.y / 2.0 + SHEET_GAP).min(window.y - SHEET_MARGIN - height)
    };
    Vec2::new(
        (mid.x - SHEET_W / 2.0).clamp(
            SHEET_MARGIN,
            (window.x - SHEET_MARGIN - SHEET_W).max(SHEET_MARGIN),
        ),
        top.max(SHEET_MARGIN),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A duel's window, measured off the running client.
    const WINDOW: Vec2 = Vec2::new(1728.0, 1052.0);
    /// A permanent on a duel's battlefield is about this many logical pixels
    /// wide, and the box keeps the 63:88 card aspect.
    const CARD: Vec2 = Vec2::new(47.0, 65.6);

    #[test]
    fn the_sheet_sits_over_the_card_it_belongs_to() {
        let mid = Vec2::new(864.0, 600.0);
        let at = corner_for(mid, CARD, 210.0, WINDOW);
        assert!(
            (at.x + SHEET_W / 2.0 - mid.x).abs() < 0.5,
            "centred on the card: {at} against {mid}"
        );
        assert!(
            at.y + 210.0 <= mid.y - CARD.y / 2.0,
            "and clear of its top edge: the sheet ends at {} and the card \
             starts at {}",
            at.y + 210.0,
            mid.y - CARD.y / 2.0
        );
    }

    /// A permanent near the top of the window — an opponent's board — has no
    /// room above it, and the sheet goes under the card rather than off the
    /// screen.
    #[test]
    fn a_sheet_with_no_room_above_the_card_goes_below_it() {
        let mid = Vec2::new(864.0, 90.0);
        let at = corner_for(mid, CARD, 210.0, WINDOW);
        assert!(
            at.y >= mid.y + CARD.y / 2.0,
            "below the card: {} against {}",
            at.y,
            mid.y + CARD.y / 2.0
        );
        assert!(at.y + 210.0 <= WINDOW.y, "and still inside the window");
    }

    /// Neither edge of the window may cut a row off. A card in the corner is
    /// the ordinary case, not a contrived one: a seat's lands sit at the end
    /// of a lane.
    #[test]
    fn a_sheet_beside_a_card_at_the_edge_stays_inside_the_window() {
        for x in [0.0, 20.0, 1700.0, WINDOW.x] {
            let at = corner_for(Vec2::new(x, 600.0), CARD, 210.0, WINDOW);
            assert!(
                at.x >= SHEET_MARGIN && at.x + SHEET_W <= WINDOW.x - SHEET_MARGIN,
                "at x={x} the sheet's corner is {at}"
            );
        }
    }

    /// A sheet taller than the window it is drawn in still starts inside it.
    /// Nine rows of wrapped rules text on a short window is the shape, and
    /// the alternative is a header scrolled off the top edge.
    #[test]
    fn a_sheet_taller_than_the_window_still_starts_inside_it() {
        let at = corner_for(
            Vec2::new(400.0, 300.0),
            CARD,
            900.0,
            Vec2::new(900.0, 500.0),
        );
        assert!(at.y >= SHEET_MARGIN, "the top is on screen: {at}");
        assert!(at.x >= SHEET_MARGIN, "and so is the left edge");
    }
}

/// The placer, run.
///
/// "Declared but never wired" is a bug this client has shipped before, and a
/// system that follows a *card* cannot be checked by reading it: what it has
/// to get right is the round trip from a `Transform` on the table, through
/// the rig the camera was just set from, to a `Node` on the overlay.
#[cfg(test)]
mod running {
    use super::*;
    use crate::table::{CameraRig, Canvas, CardVisual, ShownRig, TableCamera, apply_camera_rig};
    use baylee_client_core::layout::TableLayout;
    use baylee_core::ids::PlayerId;

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// An app with the placer, a camera aimed at a duel, a window, and one
    /// card standing in the local seat's creature lane.
    ///
    /// [`apply_camera_rig`] is in the schedule rather than a [`ShownRig`]
    /// being written by hand, because the eased rig is the thing the placer
    /// has to read: a sheet projected from the *target* rig would lead the
    /// camera through every pan.
    fn harness(anchor: ObjectId) -> (App, Entity, Vec2, Vec2) {
        let mut app = App::new();
        let window = app.world_mut().spawn(Window::default()).id();
        let size = {
            let w = app
                .world()
                .entity(window)
                .get::<Window>()
                .expect("a window");
            Vec2::new(w.width(), w.height())
        };
        let canvas = Canvas::hud(size);
        let seats: Vec<_> = (0..2).map(PlayerId::new).collect();
        let table = TableLayout::new(&seats, canvas.aspect(), None);
        let slot = *table.local().expect("a local seat");
        let at = slot.lane_center(baylee_client_core::layout::LaneKind::Creatures);
        app.insert_resource(CameraRig::home(&table, canvas))
            .init_resource::<ShownRig>()
            .init_resource::<Time>()
            .init_resource::<crate::prefs::Prefs>()
            .add_systems(Update, (apply_camera_rig, place_ability_sheet).chain());
        app.world_mut().spawn((TableCamera, Transform::default()));
        app.world_mut().spawn((
            CardVisual {
                object: obj(1),
                count: 1,
            },
            Transform::from_translation(crate::table::to_world(at, crate::table::CARD_LIFT))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ));
        let sheet = app
            .world_mut()
            .spawn((
                AbilitySheet {
                    object: anchor,
                    placed: None,
                },
                Node::default(),
                bevy::ui::ComputedNode {
                    size: Vec2::new(SHEET_W, 200.0),
                    ..default()
                },
            ))
            .id();
        (app, sheet, at, size)
    }

    #[test]
    fn the_sheet_is_put_over_the_card_it_is_anchored_to() {
        let (mut app, sheet, at, size) = harness(obj(1));
        app.update();

        let rig = app
            .world()
            .resource::<ShownRig>()
            .rig()
            .expect("the camera was applied");
        // Projected a second time here, from the table point rather than
        // from the card's corners, so this is not the placer's own
        // arithmetic read back to itself.
        let drawn = crate::table::Lens::new(rig, size)
            .project(at)
            .expect("the lane is in front of the eye");

        let node = app.world().entity(sheet).get::<Node>().expect("a node");
        assert_eq!(node.display, Display::Flex, "the sheet is shown");
        let Val::Px(left) = node.left else {
            panic!("the sheet was never placed: {:?}", node.left)
        };
        let Val::Px(top) = node.top else {
            panic!("no top: {:?}", node.top)
        };
        assert!(
            (left + SHEET_W / 2.0 - drawn.x).abs() < 1.0,
            "the sheet is centred on the card: its middle is {} and the card \
             is drawn at {}",
            left + SHEET_W / 2.0,
            drawn.x
        );
        assert!(
            top + 200.0 < drawn.y && top > 0.0,
            "and stands above it, inside the window: {top} + 200 against {}",
            drawn.y
        );
    }

    /// A sheet whose permanent is not on the table is hidden rather than left
    /// wherever it was last drawn. That is the same answer a seat bar gives
    /// for a shelf the camera cannot see, and for the same reason: the sheet
    /// is closed by the input path and not by the camera.
    #[test]
    fn a_sheet_whose_card_is_not_drawn_is_put_away() {
        let (mut app, sheet, _, _) = harness(obj(9));
        app.update();
        let node = app.world().entity(sheet).get::<Node>().expect("a node");
        assert_eq!(node.display, Display::None);
    }

    /// Nothing on parchment is written in brass.
    ///
    /// `BRASS` on `PARCHMENT` measures 1.9:1 — below every legibility floor —
    /// which is how the zone browser's current tab came to read *fainter*
    /// than the ones beside it. Brass keeps its job as a light: the roundel
    /// on an armed row, the ordering badge, the card glow, each of which sits
    /// on its own fill. The check is on the source because what is being held
    /// is a rule about a whole surface rather than about one node.
    ///
    /// It used to live in `hud::tray` and scan that one file. The tray is a
    /// dark panel now (`docs/redesign-proposal.md` §1.3: parchment is a sheet
    /// you read from, a panel is a place you work in), so the rule moved to
    /// where the parchment actually is — and it reads both surfaces, which is
    /// strictly more than it ever did.
    #[test]
    fn the_parchment_writes_no_letters_in_brass() {
        // Assembled rather than written out, or the needle is in the
        // haystack and this test fails on its own source line.
        let ink_in = format!("TextColor(palette::{}", "BRASS");
        for (what, source) in [
            ("the ability sheet", include_str!("sheet.rs")),
            ("the prompt slip", include_str!("overlay.rs")),
        ] {
            for line in source.lines() {
                let code = line.split("//").next().unwrap_or(line);
                assert!(
                    !code.contains(&ink_in),
                    "brass is a light on {what}, not a letter: {line}"
                );
            }
        }
    }
}
