//! The dialog's tree, built: the panel, its head, tabs, list and foot.

use super::footer::{spawn_control, spawn_footer, spawn_tab, spawn_tab_fact, spawn_view};
use super::rows::{spawn_rows, zone_label};
#[allow(clippy::wildcard_imports)] // the tray's own vocabulary
use super::*;

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
    maximised: bool,
    (mode, syntax_open): (ViewMode, bool),
    scroll_y: f32,
    lit: &[(ObjectId, crate::Reach)],
    facts: Option<&baylee_client_core::cardquery::PoolFacts>,
    lists: Option<&baylee_client_core::cardquery::ListShelf>,
    trigger_sheet: Option<&baylee_client_core::triggerorder::sheet::Sheet>,
) -> Entity {
    // The catalog reaching the panel's own decisions, which is the half
    // `Browser` cannot do for itself: it decides in `baylee-client-core`,
    // which links neither the card registry nor the gateway's text. Bound
    // here rather than handed out by a constructor the way
    // [`crate::cardart::registry`] is, because this one closes over the view
    // and the texts and so has nowhere to live but the call.
    let shown =
        |object: &baylee_view::PublicObject| Some(crate::face::name_of(object, view, faces.texts));
    let hand_name = |card: &baylee_view::HandObject| {
        let text = faces.texts.face(card.card.index, card.card.face);
        Some(baylee_client_core::card_face::shown_name(&card.name, text.as_ref()).to_string())
    };
    let names = Names {
        shown: &shown,
        hand: &hand_name,
        facts,
        lists,
    };
    let mut rows = browser.rows(view, interaction, names);
    // Own-hand arrangement rows are not browser tabs but use the same wording.
    for row in rows
        .iter_mut()
        .filter(|row| row.zone == baylee_client_core::BrowseZone::Hand)
    {
        if let Some(card) = baylee_client_core::decision::known_cards(view).find(|h| h.id == row.id)
            && let Some(name) = hand_name(card)
        {
            row.name = name;
        }
    }
    // The sheet's own cards are on no table and in no board model — a library
    // search lists a hundred that nothing else is drawing — so the texture
    // cache is told about them here or by nobody. Before this, the rows a
    // player was reading were the oldest thing in the cache and the first a
    // fetch would have thrown away.
    let on_the_sheet: Vec<baylee_client_core::images::ImageKey> =
        rows.iter().filter_map(|row| row.art).collect();
    textures.touch_visible(&on_the_sheet);
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
    let compact = browser.compact_targets(view, interaction);
    let mode = if compact { ViewMode::Detailed } else { mode };
    // The band: the whole window between its top edge and the hand zone,
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
            TrayBand,
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(TOP_CLEAR),
                bottom: px(HAND_ZONE_H),
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
                border_radius: BorderRadius::all(px(TRAY_RADIUS)),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            palette::elevation_sheet(),
        ))
        .id();
    commands.entity(frame).add_child(panel);

    // ---- the head: what this is, what it is showing, and what to type ----
    //
    // It carries the panel's own radius at the top, less the one pixel of
    // border it sits inside — the same arithmetic and the same reason as
    // [`super::sheet_surface`], which is the other place a child has to be
    // concentric with the corner it is drawn in.
    //
    // Without it the panel's two top corners were square. `Overflow::clip()`
    // clips **rectangularly** in `bevy_ui`, so a square child inside a
    // rounded parent paints over the parent's corner *and* over the arc of
    // its border, and what is left is two straight lines that stop short of
    // each other. Measured in the running client at 3008 x 1630 before the
    // change: the fill in the corner read `DIALOG_LIT` (38, 33, 25) — the
    // head's colour, not the panel's (28, 25, 19) — and the 1 px line was
    // absent over exactly the 14 px of the radius. The panel's *bottom*
    // corners, where no child reaches, drew a clean arc the whole time,
    // which is what says the radius was never the thing that was wrong.
    let head = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(TRAY_HEAD_GAP),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_HEAD_PAD)),
                border: UiRect::bottom(px(1)),
                border_radius: BorderRadius::top(px(TRAY_HEAD_R)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(palette::DIALOG_LIT),
            BorderColor::all(palette::DIALOG_LINE),
            Pickable::IGNORE,
        ))
        .id();

    // The title row answers the pointer, because it is what a drag takes hold
    // of. Everything inside it that is not a control keeps `Pickable::IGNORE`,
    // so a press on the bare row is a press on the row itself.
    //
    // **The tabs are in it, and the word "Zonen" is gone.** The owner asked
    // for both on 14.09.2026 — the chips say what the panel is far better
    // than a label repeating the name of the thing the player just opened,
    // and a title bar with nothing but a title in it is a row of air. They
    // were deliberately *out* of this row before, because dragging a tab
    // sideways would have carried the sheet with it; that is now settled
    // where the minimise button already settles it — `tray_drag` lets the
    // control claim the press before the row it stands on does.
    // The order of triggers has nothing to filter: the rows are the answer.
    let tools = if compact || trigger_sheet.is_some() {
        None
    } else {
        let title_row = commands
            .spawn((
                TrayGrip,
                Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    column_gap: px(TRAY_GAP),
                    // At least a line, and as many as the tabs wrap onto:
                    // the list below takes what is left of the sheet.
                    min_height: px(TRAY_TITLE_H),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .id();
        // The two ways the sheet changes size, in the order every window on this
        // player's desktop puts them: down into the tray, then out to full.
        //
        // Drawn only on a sheet the player opened by hand, which is the corner's
        // rule one control along and is there for the same measurement: on a
        // sheet a *question* opened the minimise button fired, `Browser::follow`
        // put the sheet straight back, and it was a control that lit under the
        // pointer and left the screen exactly as it was. The maximise button
        // inherits the rule for a different reason with the same shape — a sheet
        // a question opened reads no stored rectangle, so there is nothing for it
        // to write and nothing to come back to.
        //
        // The owner asked for the pair on 19.09.2026: *"Der maximieren Button
        // wandert neben den minimieren Button"*. It used to be the resize
        // corner's second job, found by a click that travelled less than
        // `input::tray_drag`'s `TAP_SLOP` — which is a gesture nothing on the
        // sheet said was there.
        let close = (!browser.for_choice()).then(|| {
            let row = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        // On the first line of tabs, however many there are.
                        align_self: AlignSelf::FlexStart,
                        column_gap: px(HEAD_BTN_GAP),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    // The row is furniture; the two buttons in it are the
                    // controls. A press on the gap between them is a press on the
                    // title bar, which is what starts a drag.
                    Pickable::IGNORE,
                ))
                .id();
            let down = head_button(commands, fonts, glyph::MINIMISE);
            commands.entity(down).insert(TrayMinimise);
            // Two marks, not one that means both: a button offering to maximise a
            // sheet that already fills the band is the same lie as a lit control
            // that refuses its gesture. `window-restore` is the pair's other half
            // in the shipped icon font and draws as two overlapping frames.
            let out = head_button(
                commands,
                fonts,
                if maximised {
                    glyph::RESTORE
                } else {
                    glyph::MAXIMISE
                },
            );
            commands.entity(out).insert(TrayMaximise);
            commands.entity(row).add_children(&[down, out]);
            row
        });
        // ---- the zone tabs, "All" first, then a group for every seat ----
        //
        // They take the row's slack and the minimise button keeps its 22 px,
        // which is why the tabs grow and it does not. `min_width` of zero is the
        // half a flex row always needs: without it a row of eight piles refuses
        // to shrink below the width of its own chips and pushes the button off
        // the sheet. On a question's sheet there is no button and the tabs have
        // the row to themselves, which is the one case where that slack is free.
        //
        // **They wrap.** The row was one line cut at the sheet's edge, and at
        // four seats the last seat's piles were tabs nobody could reach
        // (played 12.11.2026). A seat's tabs stay together and go to the next
        // line together.
        let tabs = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(TRAY_SEAT_GAP),
                    row_gap: px(4),
                    align_items: AlignItems::Center,
                    min_height: px(TRAY_TAB_H),
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_width: px(0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // "All" carries no count: a sum of a graveyard, a stack and a reveal is a
        // number about nothing. Its box is ticked when no other one is, which is
        // the empty set's meaning drawn rather than a fourth state
        // ([`Browser::shows_every_zone`]).
        //
        // A question that lives in one zone pins the tab to it (W3): nothing in
        // the row is a button then, "All" included.
        let pinned = browser.locked();
        let live = pinned.is_none();
        let mut chips = vec![spawn_tab(
            commands,
            fonts,
            None,
            Phrase::BrowseAll.text(lang).to_string(),
            browser.shows_every_zone(),
            live,
        )];
        // What no seat owns stands with "All": what is being looked at, the
        // stack, the battlefield a question asks about.
        let zones = browser.zones(view);
        for zone in zones.iter().copied().filter(|zone| zone.seat().is_none()) {
            chips.push(spawn_tab(
                commands,
                fonts,
                Some(zone),
                zone_label(lang, zone, view, statics, browser, interaction),
                browser.is_ticked(zone) || pinned == Some(zone),
                live,
            ));
        }
        // Every seat is a group, mine first: its name once, a tab for each
        // of its piles with something in it, and how many cards it holds in
        // its library and its hand, which nobody may look through and anybody
        // may count: numbers and not tabs (CR 401.2, 401.3, 402.3). A seat
        // with empty piles is still here with its numbers: the sheet says
        // where every card at the table is (the owner, 12.11.2026).
        let seats = view.seats.len();
        for step in 0..seats {
            let at = (view.seat.get() as usize + step) % seats;
            let Some(seat) = view.seats.get(at) else {
                continue;
            };
            let who = if seat.player == view.seat {
                Phrase::You.text(lang).to_string()
            } else {
                seat_name(lang, Some(statics), seat.player)
            };
            let group = commands
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: px(4),
                        height: px(TRAY_TAB_H),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let name = dialog_label(commands, fonts, &who, 11.0, palette::DIALOG_SOFT);
            commands.entity(group).add_child(name);
            for zone in zones
                .iter()
                .copied()
                .filter(|zone| zone.seat() == Some(seat.player))
            {
                let count = browser.tab_count(zone, view, interaction);
                let tab = spawn_tab(
                    commands,
                    fonts,
                    Some(zone),
                    Phrase::BrowseTabCount
                        .fill(lang, &[zone.label().text(lang), &count.to_string()]),
                    browser.is_ticked(zone) || pinned == Some(zone),
                    live,
                );
                commands.entity(group).add_child(tab);
            }
            let facts = [
                (glyph::LIBRARY, seat.library_count),
                (glyph::HAND, seat.hand_count),
            ]
            .map(|(icon, count)| spawn_tab_fact(commands, fonts, icon, count));
            commands.entity(group).add_children(&facts);
            chips.push(group);
        }
        commands.entity(tabs).add_children(&chips);
        commands.entity(title_row).add_child(tabs);
        if let Some(close) = close {
            commands.entity(title_row).add_child(close);
        }

        // ---- what is typed, how it is sorted, and how much is answered ----
        //
        // An ordering has no filter to offer — the panel is the answer being
        // assembled, and narrowing it would hide places in it — so the row says
        // what to do instead. Everywhere else this is a field: empty and unfocused
        // it shows what it is for, focused it shows a caret, and either way it is
        // the thing a player clicks to search the pile they are looking at.
        let ordering = interaction.is_some_and(baylee_client_core::Interaction::is_ordering);
        let typing = browser.is_typing();
        // A field being typed into draws itself out of its own segments below; a
        // field at rest is one line saying what it holds or what it is for.
        let hint = if ordering {
            Some(Phrase::BrowseOrderHint.text(lang).to_string())
        } else if typing {
            None
        } else if browser.filter().trim().is_empty() {
            Some(Phrase::BrowseFilter.text(lang).to_string())
        } else {
            Some(format!("\u{201c}{}\u{201d}", browser.filter()))
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
        let ink = if said {
            palette::DIALOG_INK
        } else {
            palette::DIALOG_SOFT
        };
        let filter_text: Vec<Entity> = match &hint {
            Some(words) => vec![dialog_text(commands, fonts, words, 11.5, ink)],
            None => filter_runs(commands, fonts, browser, ink),
        };
        // The magnifier the deck builder's box wears, in this register's ink. It
        // is what says the box is a *search* before a word has been typed into
        // it, and it is deliberately the quiet ink even while the field holds the
        // keyboard: it is a label on the box, not part of what is written in it.
        let lens = commands
            .spawn((
                Text::new(glyph::MAGNIFIER.to_string()),
                icon_tf(fonts, 10.5),
                TextColor(palette::DIALOG_SOFT),
                Node {
                    flex_shrink: 0.0,
                    margin: UiRect::right(px(TRAY_GAP - 4.0)),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        // A sunk field: the ring that says where the typing goes is the border
        // turning candle, not a second fill.
        let filter_line = commands
            .spawn((
                TrayFilter,
                UiButton,
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
        // The `?` syntax help and the gear live *inside* the box, which is what
        // says they are about what the box holds. Either stays lit while its
        // sheet is open, because neither sheet has a frame of its own to say so.
        let building = browser.builder().is_some();
        let help = commands
            .spawn((
                super::TraySyntaxHelp,
                UiButton,
                crate::hud::Hint(Phrase::BuildSyntaxTitle.text(lang).to_string()),
                Node {
                    width: px(TRAY_CTRL_H - 8.0),
                    height: px(TRAY_CTRL_H - 8.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_shrink: 0.0,
                    // `auto` on the left pushes the `[?, ⚙]` pair to the far end
                    // of the box so typing never walks them along.
                    margin: UiRect::left(Val::Auto),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if syntax_open {
                    palette::CANDLE_WASH
                } else {
                    Color::NONE
                }),
                Feel::tinting_to(
                    if syntax_open {
                        palette::CANDLE_WASH
                    } else {
                        Color::NONE
                    },
                    palette::CANDLE_WASH_LIT,
                ),
            ))
            .observe(toggle_tray_syntax)
            .id();
        let question = commands
            .spawn((
                Text::new(glyph::QUESTION_CIRCLE.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(if syntax_open {
                    palette::CANDLE
                } else {
                    palette::DIALOG_SOFT
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(help).add_child(question);
        let gear = commands
            .spawn((
                TrayGear,
                UiButton,
                Node {
                    width: px(TRAY_CTRL_H - 8.0),
                    height: px(TRAY_CTRL_H - 8.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_shrink: 0.0,
                    margin: UiRect::left(px(2.0)),
                    border_radius: btn_radius(),
                    ..default()
                },
                BackgroundColor(if building {
                    palette::CANDLE_WASH
                } else {
                    Color::NONE
                }),
                Feel::tinting_to(
                    if building {
                        palette::CANDLE_WASH
                    } else {
                        Color::NONE
                    },
                    palette::CANDLE_WASH_LIT,
                ),
            ))
            .id();
        let cog = commands
            .spawn((
                Text::new(glyph::GEAR.to_string()),
                icon_tf(fonts, 11.0),
                TextColor(if building {
                    palette::CANDLE
                } else {
                    palette::DIALOG_SOFT
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(gear).add_child(cog);
        let active_filters = browser.builder().map_or_else(
            || crate::filterui::query_active_count(browser.filter()),
            crate::filterui::active_count,
        );
        if let Some(badge) = crate::filterui::gear_badge(
            commands,
            fonts,
            active_filters,
            crate::filterui::Register::TRAY,
        ) {
            commands.entity(badge).entry::<Node>().and_modify(|mut n| {
                n.top = px(-3.0);
                n.right = px(-4.0);
            });
            commands.entity(gear).add_child(badge);
        }
        commands.entity(filter_line).add_child(lens);
        commands.entity(filter_line).add_children(&filter_text);
        commands.entity(filter_line).add_children(&[help, gear]);
        // A library is a hundred cards and a long graveyard is thirty, so "look
        // through this pile" is not a question the pile's own order answers on
        // its own. The key and the direction are two buttons because they are two
        // questions, and the arrow says which way the current one runs rather
        // than being a third state of the key.
        //
        // Not while an arrangement is being built, though: the order on the
        // sheet is then the answer, `Browser::rows` sorts by nothing else, and
        // a key that lit under the pointer and moved no card would be a control
        // refusing its gesture.
        let sorting = (!ordering).then(|| {
            let key = spawn_control(
                commands,
                fonts,
                TraySort { reverse: false },
                browser.sort().label().text(lang),
                9.0,
            );
            let direction = spawn_control(
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
            [key, direction]
        });
        // And after them, the three shapes the same rows can be drawn in. They
        // sit at the right end so that every "how it is shown" control is one
        // cluster and the search field keeps the growing left — and they are
        // three buttons rather than a fourth cycling one, for the reason
        // [`super::TrayView`] gives: a sort key is a ring of equivalent answers
        // and a view is a shape you are looking at.
        // Each segment carries its own 1 px border and concentric outer corner
        // (`spawn_view`) so every segment resolves to the full `TRAY_CTRL_H`
        // (28 px) button height and `bevy_ui`'s rectangular `Overflow::clip()`
        // never paints a square segment fill over a rounded outer arc.
        let views = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    height: percent(100),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let segments: Vec<Entity> = ViewMode::ALL
            .into_iter()
            .map(|each| spawn_view(commands, fonts, each, each == mode))
            .collect();
        commands.entity(views).add_children(&segments);
        commands.entity(controls).add_child(filter_line);
        if let Some(sorting) = sorting {
            commands.entity(controls).add_children(&sorting);
        }
        commands.entity(controls).add_child(views);
        // The tally. The engine names a minimum and a maximum, so the dialog can
        // say how far along the answer is — and a panel with no question in it (a
        // graveyard opened by hand) says nothing rather than "0 of 0".
        if let Some((min, max)) = answering.and_then(baylee_client_core::Interaction::bounds)
            && max > 0
        {
            let chosen = answering.map_or(0, baylee_client_core::Interaction::declared);
            let words = if min == max {
                Phrase::BrowseTallyExact.fill(lang, &[&chosen.to_string(), &max.to_string()])
            } else {
                Phrase::BrowseTallyUpTo.fill(lang, &[&chosen.to_string(), &max.to_string()])
            };
            let tally = dialog_text(commands, fonts, &words, 10.5, palette::DIALOG_SOFT);
            commands.entity(controls).add_child(tally);
        }

        Some((title_row, controls))
    };

    // Two rows, where it was three: the tabs went up into the title row —
    // and a third when the gear is open, which is the builder. It sits under
    // the controls rather than over the list, because it is what the box in
    // that row holds: a panel floating over the cards would be a second
    // window, and this is a mode of the field above it.
    if let Some((title, _)) = tools {
        commands.entity(head).add_child(title);
    }
    if let Some(answering) = answering {
        let mut context =
            crate::choices::target_question(answering, view, lang, faces.texts, Some(statics));
        if compact
            && !matches!(
                answering.pending(),
                baylee_engine::choice::Pending::ChooseTargets {
                    reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
                    ..
                }
            )
        {
            // Full card rules remain in the card preview; this sheet answers the target choice.
            context.truncate(1);
        }
        for text in context {
            let line = target_context_line(commands, fonts, &text);
            commands.entity(head).add_child(line);
        }
        if compact
            && !matches!(
                answering.pending(),
                baylee_engine::choice::Pending::ChooseTargets {
                    reason: baylee_engine::choice::TargetPrompt::Retarget { .. },
                    ..
                }
            )
        {
            let line = target_context_line(
                commands,
                fonts,
                &answering.prompt_in(view).headline(
                    lang,
                    baylee_client_core::Turn::of(view.active, view.seat),
                    Some(statics),
                    false,
                ),
            );
            commands.entity(head).add_child(line);
        }
    }
    if let Some((_, controls)) = tools {
        commands.entity(head).add_child(controls);
    }
    if let Some(panel_state) = browser.builder() {
        let veil = commands
            .spawn((
                TrayGear,
                UiButton,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-4000.0),
                    right: px(-4000.0),
                    top: px(-4000.0),
                    bottom: px(-4000.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                GlobalZIndex(crate::shellkit::tokens::z::POPOVER - 1),
            ))
            .id();
        let top_px =
            place.top + TRAY_HEAD_PAD * 2.0 + TRAY_TITLE_H + TRAY_HEAD_GAP + TRAY_CTRL_H + 6.0;
        let built = crate::filterui::floating_popover(
            commands,
            fonts,
            panel_state,
            names.surface(),
            // The zone dialog filters once. Its tabs pick a zone rather than
            // narrowing one, so there is no second truth to declare.
            None,
            lang,
            crate::filterui::Register::TRAY,
            top_px,
        );
        commands
            .entity(built)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.left = px(place.left + TRAY_SIDE);
                node.right = Val::Auto;
                node.width = px((place.width - 2.0 * TRAY_SIDE).clamp(300.0, 460.0));
            });
        commands.entity(frame).add_children(&[veil, built]);
    } else if syntax_open {
        let sheet = spawn_syntax_help(commands, fonts, lang, names.surface());
        commands.entity(head).add_child(sheet);
    }

    // ---- the list ----
    //
    // It scrolls, and it is the list rather than the panel: the tabs, the
    // search field and the sort control have to stay where they are while a
    // hundred-card library is scrolled past them. `Pickable` and not
    // `Pickable::IGNORE`, because the picking backend is what turns a wheel
    // into the `PointerScroll` a scrolling node listens for — the same
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
                min_width: px(0),
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
            super::super::Scrolls,
            TrayScroll,
            ScrollPosition(Vec2::new(0.0, scroll_y)),
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
        // Empty because nothing could be asked is not the pile being empty.
        let lacking = baylee_client_core::cardsearch::lacking(
            &baylee_client_core::cardquery::parse(browser.filter()),
            names.surface(),
        );
        let words = dialog_text(
            commands,
            fonts,
            lacking.unwrap_or(Phrase::BrowseEmpty).text(lang),
            11.5,
            palette::DIALOG_SOFT,
        );
        commands.entity(empty).add_child(words);
        commands.entity(list).add_child(empty);
    }
    let grid = GridCtx {
        browser,
        interaction,
        view,
        statics,
        fonts,
        // The measure the tiles share: the sheet's own width less the gutter
        // the list keeps on both sides. The panel's border is inside that
        // width already — it is a `border`, not a margin — so it is not
        // subtracted a second time.
        measure: place.width - 2.0 * TRAY_SIDE,
        headed: false,
        lit,
    };
    match answering.and_then(baylee_client_core::Interaction::arrangement) {
        // The order of triggers has its own rows: what a trigger will do is
        // the whole of the question, and a card's cost and type say nothing
        // about it.
        Some(arrangement) if trigger_sheet.is_some() => triggers::spawn_sheet(
            commands,
            list,
            lang,
            trigger_sheet.expect("the arm's guard"),
            arrangement,
            &rows,
            grid,
            faces,
            textures,
            assets,
            &mut cards,
        ),
        // An arrangement is drawn pile by pile, whatever the view: the piles
        // are the answer, and a list that ran them together would number
        // every card twice.
        Some(arrangement) => arrange::spawn_piles(
            commands,
            list,
            lang,
            arrangement,
            &rows,
            mode,
            grid,
            faces,
            textures,
            assets,
            &mut cards,
        ),
        None => spawn_rows(
            commands,
            list,
            lang,
            &rows,
            mode,
            GridCtx {
                // Headed runs only when the panel is showing more than one
                // pile: a tile cannot say which zone it came from, and the
                // list view says it in a badge on every row. With one tab
                // ticked the tabs above have already said it, and a heading
                // repeating that tab would be a line of chrome over every
                // grid the panel ever draws.
                headed: rows
                    .iter()
                    .map(|row| row.zone)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    > 1,
                ..grid
            },
            faces,
            textures,
            assets,
            &mut cards,
        ),
    }

    // ---- the footer ----
    let foot = spawn_footer(commands, fonts, lang, answering, view);

    // The corner, in the same shape and the same place the card preview's is:
    // one handle, bottom right, both axes. A second handle on every edge is
    // eight more hit targets for a gesture nobody makes on a dialog.
    //
    // It **resizes, and only that**. It used to maximise as well, on a press
    // and release that travelled less than `TAP_SLOP` between them — an
    // unwritten gesture whose whole justification was the mark it wore
    // ("the corner draws a ⤢ — so it is read as a maximise button and has to
    // be one"). The mark moved to a button in the head that says it out loud,
    // so the gesture went with it and the corner wears [`glyph::RESIZE`]
    // instead: two arrowheads pointing apart, which is the one thing left
    // that it does.
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
                UiButton,
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
                    Text::new(glyph::RESIZE.to_string()),
                    icon_tf(fonts, 10.0),
                    TextColor(palette::DIALOG_SOFT),
                    Pickable::IGNORE,
                )],
            ))
            .id()
    });

    let bar = crate::hud::ledge::log::scrollbar(
        commands,
        list,
        (
            palette::DOCK_EDGE.with_alpha(0.35),
            palette::CANDLE.with_alpha(0.8),
        ),
        crate::hud::ledge::log::at_rest,
    );
    let bar_bottom = if corner.is_some() { 26.0 } else { 4.0 };
    commands
        .entity(bar)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.margin = UiRect::new(px(2), px(4), px(4), px(bar_bottom));
        });
    commands.entity(bar).insert((
        crate::hud::ledge::log::OnDemandScrollbar(list),
        Visibility::Hidden,
    ));
    let body = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_height: px(0),
                width: percent(100),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&[list, bar])
        .id();

    commands.entity(panel).add_children(&[head, body]);
    if let Some(foot) = foot {
        commands.entity(panel).add_child(foot);
    }
    if let Some(corner) = corner {
        commands.entity(panel).add_child(corner);
    }
    frame
}

fn toggle_tray_syntax(
    mut click: On<bevy::picking::events::PointerClick>,
    mut commands: Commands,
    mut duel: ResMut<crate::Duel>,
    syntax: Option<Res<super::TraySyntax>>,
) {
    click.propagate(false);
    let next = !syntax.is_some_and(|s| s.open);
    if next {
        duel.browser.close_builder();
    }
    commands.insert_resource(super::TraySyntax { open: next });
}

/// Which `SYNTAX` examples apply to a given `cardquery::Surface`.
pub(in crate::hud) fn syntax_for_surface(
    surface: baylee_client_core::cardquery::Surface<'_>,
) -> Vec<(&'static str, Phrase)> {
    use baylee_client_core::cardquery::Key;
    let has_oracle = surface.keys.contains(&Key::Oracle);
    let has_facts = surface.pool_facts().is_some();
    let has_lists = surface.lists().is_some();
    crate::buildui::pool::SYNTAX
        .iter()
        .copied()
        .filter(|(example, _)| match *example {
            "o:draw" | "id<=wu" | "m:{G}{G}" | "is:partial" => has_oracle,
            "r:rare" | "s:m21" | "f:modern" | "banned:commander" => has_facts,
            "list:\"My Commander\"" => has_lists,
            _ => true,
        })
        .collect()
}

fn spawn_syntax_help(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    surface: baylee_client_core::cardquery::Surface<'_>,
) -> Entity {
    let sheet = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(2.0),
                padding: UiRect::axes(px(10.0), px(8.0)),
                margin: UiRect::top(px(6.0)),
                border: UiRect::all(px(1)),
                border_radius: btn_radius(),
                max_height: px(220.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(palette::DIALOG),
            BorderColor::all(palette::DIALOG_LINE),
            super::super::Scrolls,
            ScrollPosition::default(),
        ))
        .id();
    let title = dialog_label(
        commands,
        fonts,
        Phrase::BuildSyntaxTitle.text(lang),
        11.0,
        palette::DIALOG_INK,
    );
    commands.entity(sheet).add_child(title);
    for (example, meaning) in syntax_for_surface(surface) {
        let row = commands
            .spawn((
                UiButton,
                Node {
                    width: percent(100),
                    min_height: px(24.0),
                    align_items: AlignItems::Center,
                    column_gap: px(10.0),
                    padding: UiRect::axes(px(6.0), px(2.0)),
                    border_radius: btn_radius(),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Feel::tinting_to(Color::NONE, palette::CANDLE_WASH_LIT),
            ))
            .observe(
                move |mut click: On<bevy::picking::events::PointerClick>,
                      mut commands: Commands,
                      mut duel: ResMut<crate::Duel>| {
                    click.propagate(false);
                    let current = duel.browser.filter().to_owned();
                    let next = if current.is_empty() || current.ends_with(' ') {
                        format!("{current}{example}")
                    } else {
                        format!("{current} {example}")
                    };
                    duel.browser.set_filter(&next);
                    duel.browser.start_typing();
                    commands.insert_resource(super::TraySyntax { open: false });
                },
            )
            .id();
        let code = commands
            .spawn((
                Text::new(example),
                tf_bold(fonts, 11.0),
                TextColor(palette::CANDLE),
                TextLayout::no_wrap(),
                Node {
                    width: px(140.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let what = dialog_text(
            commands,
            fonts,
            meaning.text(lang),
            10.5,
            palette::DIALOG_SOFT,
        );
        commands.entity(row).add_children(&[code, what]);
        commands.entity(sheet).add_child(row);
    }
    sheet
}
