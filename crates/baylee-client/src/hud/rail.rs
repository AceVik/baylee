//! The phase rail down the side of the screen, and the combat line that
//! reads out of it.

#[allow(clippy::wildcard_imports)] // the HUD's own vocabulary
use super::*;

/// Icon and the short rail label for a rail row.
fn row_visual(row: RailRow) -> (char, &'static str) {
    match row {
        RailRow::Untap => ('\u{f185}', "UNT"),
        RailRow::Upkeep => ('\u{f0ad}', "UPK"),
        RailRow::Draw => ('\u{f063}', "DRW"),
        RailRow::Main1 => ('\u{f024}', "M1"),
        RailRow::CombatBegin => ('\u{f71d}', "CBT"),
        RailRow::Attackers => ('\u{f70c}', "ATK"),
        RailRow::Blockers => ('\u{f3ed}', "BLK"),
        RailRow::Damage => ('\u{f6e2}', "DMG"),
        RailRow::CombatEnd => ('\u{f11e}', "EOC"),
        RailRow::Main2 => ('\u{f024}', "M2"),
        RailRow::EndStep => ('\u{f253}', "END"),
        RailRow::Cleanup => ('\u{f51a}', "CLN"),
    }
}

/// The rail's width.
pub const RAIL_W: f32 = 56.0;

/// The phase rail, split in two priority-control sections: the phases of
/// *opponents'* turns on top, your own (and teammates') phases at the
/// bottom, the turn number between them, and the autopilot buttons at the
/// foot behind a separator. Spans exactly from the player bar's bottom
/// to the hand bar's top. Row size and font scale with the available
/// height — they only shrink when the space runs out.
#[allow(clippy::too_many_lines)] // two sections + foot, one flat build
#[allow(clippy::too_many_arguments)] // one rail, drawn from everything it shows
pub(super) fn spawn_phase_rail(
    commands: &mut Commands,
    lang: Lang,
    view: &PlayerView,
    orders: &baylee_client_core::automation::PhaseOrders,
    autopilot: Option<AutoPilot>,
    fonts: &UiFonts,
    window_h: f32,
    statics: Option<&GameStatic>,
) -> Entity {
    use baylee_client_core::automation::RailSide;

    let rail = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(0),
                top: px(TAB_H),
                bottom: px(HAND_BAR_H),
                width: px(RAIL_W),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(px(4), px(6)),
                ..default()
            },
            BackgroundColor(palette::PANEL),
            Pickable::IGNORE,
        ))
        .id();

    // Responsive row size: only shrinks when the space runs out.
    let available = (window_h - TAB_H - HAND_BAR_H).max(240.0);
    let reserved = 2.0 * 14.0   // section headers
        + 20.0                  // turn number
        + 1.0 + 8.0             // separator + its margin
        + 2.0 * 30.0 + 4.0      // autopilot buttons + gap
        + 12.0; // rail padding
    let row_h = ((available - reserved) / 24.0).clamp(16.0, 30.0);
    let icon_size = (row_h * 0.42).clamp(8.0, 12.0);
    let label_size = (row_h * 0.30).clamp(6.5, 9.0);
    let show_label = row_h >= 21.0;

    let current = RailRow::current(view.phase, view.step);
    let active_is_mine = same_team(statics, view.active, view.seat);
    let current_side = if active_is_mine {
        RailSide::Mine
    } else {
        RailSide::Theirs
    };

    let spawn_section = |commands: &mut Commands, side: RailSide, header: &str| {
        let section = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        let head = commands
            .spawn((Text::new(header), tf(fonts, 9.0), TextColor(palette::MUTED)))
            .id();
        commands.entity(section).add_child(head);
        for (row, skipped) in orders.rows_for(side) {
            let is_current = row == current && side == current_side;
            let is_selected = orders.selected() == Some((side, row));
            let (icon, short) = row_visual(row);
            let mut button_children = vec![
                commands
                    .spawn((
                        Text::new(icon.to_string()),
                        icon_tf(fonts, icon_size),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ))
                    .id(),
            ];
            if show_label {
                let label = commands
                    .spawn((
                        Text::new(short),
                        tf(fonts, label_size),
                        TextColor(if is_current {
                            palette::INK
                        } else {
                            palette::MUTED
                        }),
                    ))
                    .id();
                button_children.push(label);
            }
            let button = commands
                .spawn((
                    PhaseButton { side, row },
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        height: px(row_h),
                        border: UiRect::all(px(if is_selected || is_current { 2.0 } else { 1.0 })),
                        border_radius: btn_radius(),
                        width: percent(100),
                        ..default()
                    },
                    BackgroundColor(if skipped {
                        palette::ORDER_SKIP
                    } else {
                        palette::ORDER_GO
                    }),
                    BorderColor::all(if is_selected {
                        palette::ACCENT
                    } else if is_current {
                        palette::INK
                    } else {
                        palette::PANEL
                    }),
                ))
                .id();
            for child in button_children {
                commands.entity(button).add_child(child);
            }
            commands.entity(section).add_child(button);
        }
        section
    };

    // Top: the opponents' phases.
    let theirs = spawn_section(commands, RailSide::Theirs, Phrase::RailOpponent.text(lang));
    commands.entity(rail).add_child(theirs);

    // Middle: the turn number between the two sections.
    let turn = commands
        .spawn((
            Node {
                justify_content: JustifyContent::Center,
                padding: UiRect::axes(px(0), px(3)),
                ..default()
            },
            Pickable::IGNORE,
            children![(
                Text::new(format!("T{}", view.turn)),
                tf(fonts, 12.0),
                TextColor(palette::INK),
            )],
        ))
        .id();
    commands.entity(rail).add_child(turn);

    // Bottom: your own (and teammates') phases.
    let mine = spawn_section(commands, RailSide::Mine, Phrase::RailYou.text(lang));
    commands.entity(rail).add_child(mine);

    // Foot: separator, then the autopilot buttons with even padding.
    let separator = commands
        .spawn((
            Node {
                height: px(1),
                width: percent(100),
                margin: UiRect::axes(px(0), px(4)),
                ..default()
            },
            BackgroundColor(palette::DEAD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(rail).add_child(separator);

    let foot = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(2)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (kind, icon, engaged) in [
        (
            RailButton::NextPhase,
            glyph::STEP,
            matches!(autopilot, Some(AutoPilot::ToNextPhase { .. })),
        ),
        (
            RailButton::EndTurn,
            glyph::FAST,
            matches!(autopilot, Some(AutoPilot::ToNextTurn { .. })),
        ),
    ] {
        let button = commands
            .spawn((
                kind,
                Node {
                    padding: UiRect::all(px(6)),
                    border: UiRect::all(px(1)),
                    border_radius: btn_radius(),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(palette::PANEL_LIT),
                BorderColor::all(if engaged {
                    palette::ACCENT
                } else {
                    palette::PANEL
                }),
                children![(
                    Text::new(icon.to_string()),
                    icon_tf(fonts, 13.0),
                    TextColor(if engaged {
                        palette::ACCENT
                    } else {
                        palette::INK
                    }),
                )],
            ))
            .id();
        commands.entity(foot).add_child(button);
    }
    commands.entity(rail).add_child(foot);

    rail
}

/// The combat line: where the next declaration points, and how many stand.
///
/// `None` when there is nothing to aim — a two-player game with no
/// planeswalkers has exactly one thing to attack, and a line saying so every
/// combat would be noise. The declaration count is still worth saying, so the
/// line survives that case whenever anything has been declared.
#[must_use]
pub(super) fn combat_line(
    interaction: &baylee_client_core::Interaction,
    view: &PlayerView,
    statics: Option<&GameStatic>,
    lang: Lang,
) -> Option<String> {
    let (position, count) = interaction.focus_position()?;
    let declared = interaction.declared();
    let aiming = count > 1;
    if !aiming && declared == 0 {
        return None;
    }
    let aim = aiming.then(|| {
        let target = match interaction.combat_focus() {
            CombatFocus::Defender(Defender::Player(p)) => statics.map_or_else(
                || Phrase::ASeat.text(lang).to_string(),
                |s| s.seat_name(p).to_string(),
            ),
            CombatFocus::Defender(Defender::Planeswalker(o)) | CombatFocus::Attacker(o) => {
                view.object(o).map_or_else(
                    || Phrase::APermanent.text(lang).to_string(),
                    |o| o.name.clone(),
                )
            }
            CombatFocus::None => Phrase::AimingAtNothing.text(lang).to_string(),
        };
        Phrase::AimedAt.fill(
            lang,
            &[&target, &(position + 1).to_string(), &count.to_string()],
        )
    });
    let standing =
        (declared > 0).then(|| Phrase::DeclaredCount.fill(lang, &[&declared.to_string()]));
    Some(
        [aim, standing]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("  ·  "),
    )
}

/// Whether two seats play on the same side (same team when teams are
/// set; identical seats otherwise).
#[must_use]
pub fn same_team(statics: Option<&GameStatic>, a: PlayerId, b: PlayerId) -> bool {
    if a == b {
        return true;
    }
    let team_of = |p: PlayerId| {
        statics
            .and_then(|s| s.seats.iter().find(|i| i.player == p))
            .and_then(|i| i.team)
    };
    matches!((team_of(a), team_of(b)), (Some(ta), Some(tb)) if ta == tb)
}
