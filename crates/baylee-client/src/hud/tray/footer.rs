//! The dialog's controls: its footer, the zone tabs, the view switch, and
//! the control each of them is.

#[allow(clippy::wildcard_imports)] // the tray's own vocabulary
use super::*;

/// The footer, or nothing at all when there is no question to answer.
///
/// Two buttons, and both of them send what [`PromptAction::Confirm`] sends,
/// which is the whole shape of §6's footer: **Confirm is lit only when the
/// answer is complete, and Cancel is drawn only when the minimum is zero.**
/// There is no cancel on the wire — a question that will take an empty answer
/// is answered by sending one, and a question that will not has no way out to
/// offer, so a dialog that drew the button anyway would be promising what the
/// engine cannot deliver.
#[allow(clippy::too_many_lines)] // ordinary, batch and empty-answer confirmations
pub(super) fn spawn_footer(
    commands: &mut Commands,
    fonts: &UiFonts,
    lang: Lang,
    interaction: Option<&baylee_client_core::Interaction>,
    view: &PlayerView,
) -> Option<Entity> {
    let it = interaction?;
    let (min, max) = it.bounds()?;
    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10),
                padding: UiRect::axes(px(TRAY_SIDE), px(TRAY_FOOT_PAD)),
                border: UiRect::top(px(1)),
                // The head's radius at the other end, and for the same reason
                // — this band reaches the panel's bottom two corners exactly
                // as the head reaches the top two. It is drawn only while a
                // question is standing, which is why the square corner was
                // seen at the top first.
                border_radius: BorderRadius::bottom(px(TRAY_HEAD_R)),
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
    let words = dialog_label(
        commands,
        fonts,
        if matches!(
            it.pending(),
            baylee_engine::choice::Pending::ChooseCards {
                prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown { .. },
                ..
            }
        ) {
            if it.selected().next().is_some() {
                Phrase::ConfirmFaceDown
            } else {
                Phrase::DeclineCast
            }
        } else {
            Phrase::BrowseConfirm
        }
        .text(lang),
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
            UiButton,
            PromptButton {
                // The question's own identity, as the shelf's Confirm carries
                // it (`ledge::sync`): `PromptButton::matches_decision_id`
                // drops a Confirm whose identity is not the current one, and
                // a versioned question — a copy's "choose new targets"
                // (`DecisionId::Retarget`) — never matched `None`, so the lit
                // button under a retarget's graveyard rows did nothing (#03).
                decision_id: it.decision_id(),
                action: PromptAction::Confirm,
            },
            Feel::new(palette::CANDLE),
        ));
    } else {
        commands.entity(confirm).insert(Pickable::IGNORE);
    }
    commands.entity(foot).add_child(confirm);
    // Complete, and held back by a bound the question states (a crew short
    // of its power): the question's own reason beside the unlit Confirm,
    // which is what the engine would have answered the click with.
    if let Some(fault) = it.answer_fault() {
        let why = dialog_label(
            commands,
            fonts,
            &baylee_client_core::i18n::server_message(lang, fault.reason()),
            11.0,
            palette::DIALOG_SOFT,
        );
        commands.entity(foot).add_child(why);
    }
    if let Some(baylee_engine::choice::PlayerAction::ChooseTargetBatch { count, .. }) =
        baylee_client_core::targeting::batch_answer(it, view)
    {
        let batch = spawn_control(
            commands,
            fonts,
            PromptButton {
                decision_id: None,
                action: PromptAction::TargetBatch,
            },
            &Phrase::TargetingBatch.fill(lang, &[&count.to_string()]),
            11.0,
        );
        commands.entity(foot).add_child(batch);
    }

    // Inspection has no alternative answer. An empty Mask selection already
    // labels the primary action "Do not cast", so a second decline duplicates it.
    let primary_declines = matches!(
        it.pending(),
        baylee_engine::choice::Pending::ChooseCards {
            prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown { .. },
            ..
        }
    ) && it.selected().next().is_none();
    if min == 0 && max > 0 && !primary_declines {
        let out = dialog_label(
            commands,
            fonts,
            if matches!(
                it.pending(),
                baylee_engine::choice::Pending::ChooseCards {
                    prompt: baylee_engine::choice::ChoicePrompt::CastFaceDown { .. },
                    ..
                }
            ) {
                Phrase::DeclineCast
            } else {
                Phrase::BrowseNone
            }
            .text(lang),
            13.0,
            palette::DIALOG_SOFT,
        );
        let cancel = commands
            .spawn((
                TrayNone,
                UiButton,
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

/// One zone tab: a box, and the pile's name beside it.
///
/// `live` is false while a question has pinned a tab — every other one is
/// still drawn, at the weight of something that is not a control, and none of
/// them is a button. Drawn and not hidden, because a tab that vanished would
/// be saying the graveyard is empty, and what is true is that it is no part
/// of *this* question.
///
/// **The state is in the box and not under the chip.** It was a candle fill
/// across the whole tab, which is a thing one chip at a time can say; the
/// owner asked on 14.09.2026 for a checkbox at the start of each name so that
/// ticking several merges them, and three candle chips in a row would read as
/// three panels rather than as one list. So the chip keeps the panel's own
/// dark and the box carries the accent — the same box, the same candle and
/// the same tick the rows in the list below already use, one size down.
pub(super) fn spawn_tab(
    commands: &mut Commands,
    fonts: &UiFonts,
    zone: Option<BrowseZone>,
    label: String,
    ticked: bool,
    live: bool,
) -> Entity {
    // A chip outside the question gets no surface at all and the quieter ink
    // on top: the dialog already means "a different kind of sentence" by that
    // grey, which is exactly what a tab outside the question is. It also gets
    // **no box**, for the reason a row the question will not take draws none:
    // a box that cannot be ticked is an invitation that will be refused.
    let dead = !live && !ticked;
    let (fill, ink) = if dead {
        (Color::NONE, palette::DIALOG_SOFT)
    } else {
        (palette::DIALOG, palette::DIALOG_INK)
    };
    let text = dialog_label(commands, fonts, &label, 11.0, ink);
    let tab = commands
        .spawn((
            TrayTab { zone },
            Node {
                height: px(TRAY_TAB_H),
                padding: UiRect::horizontal(px(8)),
                align_items: AlignItems::Center,
                column_gap: px(6),
                border_radius: btn_radius(),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(fill),
        ))
        .id();
    if !dead {
        let mark = spawn_tab_box(commands, fonts, ticked);
        commands.entity(tab).add_child(mark);
    }
    if live {
        commands.entity(tab).insert((UiButton, Feel::new(fill)));
    } else {
        // No `Button` and no `Feel` either: a control that lights under the
        // pointer and then refuses the click is worse than one that never
        // invited it.
        commands.entity(tab).insert(Pickable::IGNORE);
    }
    commands.entity(tab).add_child(text);
    tab
}

/// The box at the start of a zone's name.
///
/// The list's own box ([`spawn_row`]) one size down and with the same four
/// decisions made the same way: candle under a tick, a hollow outline under
/// none, the tick out of the **icon** face because neither Inter nor Alegreya
/// Sans has U+2713, and `Pickable::IGNORE` so the box never takes the press
/// meant for the chip it sits in — the trap `a-label-swallows-the-hover`
/// names.
///
/// A box on a pinned tab is drawn and is not a control — the chip around it
/// carries that — and its tick stays candle, because the question really has
/// ticked that zone and a grey tick would be saying something else.
fn spawn_tab_box(commands: &mut Commands, fonts: &UiFonts, ticked: bool) -> Entity {
    let (fill, edge) = if ticked {
        (palette::CANDLE, palette::CANDLE)
    } else {
        (Color::NONE, palette::DIALOG_SOFT)
    };
    let mark = commands
        .spawn((
            Node {
                width: px(TRAY_TAB_BOX),
                height: px(TRAY_TAB_BOX),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(3)),
                ..default()
            },
            BackgroundColor(fill),
            BorderColor::all(edge),
            Pickable::IGNORE,
        ))
        .id();
    if ticked {
        let ink = commands
            .spawn((
                Text::new(glyph::CHECK.to_string()),
                icon_tf(fonts, 7.5),
                TextColor(palette::DIALOG),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(mark).add_child(ink);
    }
    mark
}

/// One of the three view segments.
///
/// Icons and not words, and that is arithmetic rather than taste: three
/// labels beside the sort key would leave the search field about 140 px wide
/// on a sheet at its 356-pixel floor, which is a search box that can show a
/// card name and nothing a player is typing. There are no words behind them
/// either, which is the honest half: an icon with nothing behind it cannot be
/// named by a tooltip, a keyboard map or a reader, and [`ViewMode::name`]
/// says why the translated labels are written when something first asks
/// rather than now.
///
/// The three are one strip with one border round it rather than three
/// buttons in a row: they are a single question with three answers, and a
/// chosen segment says so by being the lit one. The unchosen two rest at
/// nothing, so the strip reads as one control with a mark in it.
pub(super) fn spawn_view(
    commands: &mut Commands,
    fonts: &UiFonts,
    mode: ViewMode,
    current: bool,
) -> Entity {
    let (fill, ink) = if current {
        (palette::DIALOG_LIT, palette::CANDLE)
    } else {
        (Color::NONE, palette::DIALOG_SOFT)
    };
    let mark = match mode {
        ViewMode::Detailed => glyph::VIEW_ROWS,
        ViewMode::Large => glyph::VIEW_BIG,
        ViewMode::Grid => glyph::VIEW_GRID,
    };
    let icon = commands
        .spawn((
            Text::new(mark.to_string()),
            icon_tf(fonts, 10.0),
            TextColor(ink),
            Pickable::IGNORE,
        ))
        .id();
    let button = commands
        .spawn((
            super::super::TrayView { mode },
            UiButton,
            Node {
                width: px(26),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(fill),
            // Both ends stated, because `Feel::new` shades towards white and
            // **keeps the alpha**: an unchosen segment rests at nothing and
            // would be lifted to a brighter nothing, which is a control that
            // never answers the pointer. The resize corner has the same note
            // for the same reason.
            if current {
                Feel::new(palette::DIALOG_LIT)
            } else {
                Feel::rising_to(Color::NONE, palette::DIALOG_LIT)
            },
        ))
        .id();
    commands.entity(button).add_child(icon);
    button
}

/// One control in the head's bottom row: the sort key, or the arrow beside it.
pub(super) fn spawn_control<C: Component>(
    commands: &mut Commands,
    fonts: &UiFonts,
    marker: C,
    label: &str,
    pad: f32,
) -> Entity {
    let text = dialog_label(commands, fonts, label, 11.0, palette::DIALOG_INK);
    let button = commands
        .spawn((
            marker,
            UiButton,
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
