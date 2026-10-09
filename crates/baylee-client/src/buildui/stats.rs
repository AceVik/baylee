//! The Stats tab (§7, `mocks/v5-builder-stats-1920.png`): the curve by mana
//! value with one type lit by its chip, the colour pips' shares, the type
//! counts, the land share and Draw seven. Colour sources against what the
//! costs need is round 2 (S-17).

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::hover_of_card;
use crate::shellkit::focus::Current;
use baylee_client_core::deckbuilder::STAT_GROUPS;

/// The tallest a curve bar gets, before the text step.
const CURVE_HEIGHT: f32 = 110.0;

/// The numbers, as one column.
#[allow(clippy::too_many_lines)] // five blocks, top to bottom
pub(crate) fn draw(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let ui = env.ui();
    let column = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(12.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let counts = deck.counts();
    if counts.main == 0 {
        let said = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::LIBRARY,
            Phrase::BuildStatsEmpty.text(lang),
            None,
        );
        commands.entity(column).add_child(said);
        return column;
    }

    // ---- the type chips that light the curve
    let chips = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(6.0),
                row_gap: m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (i, group) in STAT_GROUPS.iter().enumerate() {
        let on = ui.lit == Some(*group);
        let chip = controls::chip(
            commands,
            kit,
            group.label().text(lang),
            on,
            None,
            on,
            (
                Press::Build(BuildPress::SetLit(if on { None } else { Some(*group) })),
                Stop::item(BUILDER, "lit", u8::try_from(i).unwrap_or(0)),
                Current(on),
            ),
        );
        commands.entity(chips).add_child(chip);
    }
    commands.entity(column).add_child(chips);

    // ---- the curve
    let mut heading = format!(
        "{} \u{b7} {}",
        Phrase::BuildManaCurve.text(lang),
        Phrase::BuildCurveNote.text(lang)
    );
    if let Some(group) = ui.lit {
        heading.push_str(" \u{b7} ");
        heading.push_str(&Phrase::BuildCurveLit.fill(lang, &[group.label().text(lang)]));
    }
    let head = cell(commands, kit, &heading, m.small, tokens::MUTED, false);
    commands.entity(column).add_child(head);
    let curve = deck.curve_lit(ui.lit);
    let tallest = curve.iter().map(|(all, _)| *all).max().unwrap_or(0).max(1);
    let bars = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: px_fixed(m.scaled(CURVE_HEIGHT) + m.small * 3.0),
                align_items: AlignItems::FlexEnd,
                column_gap: m.px(8.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (bucket, (all, lit)) in curve.iter().copied().enumerate() {
        let cmc = u32::try_from(bucket).unwrap_or(0);
        let col = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: px_fixed(0.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    row_gap: m.px(2.0),
                    ..default()
                },
                Press::Build(BuildPress::SetCmc(cmc)),
            ))
            .id();
        let n = words(
            commands,
            kit,
            &if all == 0 {
                String::new()
            } else {
                all.to_string()
            },
            m.small,
            tokens::INK,
        );
        let full = m.scaled(CURVE_HEIGHT);
        let height = 3.0 + (full - 3.0) * f32::from(all) / f32::from(tallest);
        let bar = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: px_fixed(height),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    border_radius: BorderRadius::all(px_fixed(3.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL),
                Pickable::IGNORE,
            ))
            .id();
        if all > 0 {
            let share = if ui.lit.is_some() {
                f32::from(lit) / f32::from(all)
            } else {
                1.0
            };
            let fill = commands
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(share * 100.0),
                        ..default()
                    },
                    BackgroundColor(tokens::ACCENT.with_alpha(0.75)),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(bar).add_child(fill);
        }
        let label = words(
            commands,
            kit,
            &if bucket + 1 == CURVE_BUCKETS {
                format!("{cmc}+")
            } else {
                cmc.to_string()
            },
            m.small,
            tokens::MUTED,
        );
        commands.entity(col).add_children(&[n, bar, label]);
        commands.entity(bars).add_child(col);
    }
    commands.entity(column).add_child(bars);

    // ---- the colour pips' shares
    let pips = deck.pips();
    let total: u32 = pips.iter().map(|p| u32::from(*p)).sum();
    if total > 0 {
        let head = cell(
            commands,
            kit,
            &format!(
                "{} \u{b7} {}",
                Phrase::BuildColourPips.text(lang),
                Phrase::BuildPipsNote.text(lang)
            ),
            m.small,
            tokens::MUTED,
            false,
        );
        commands.entity(column).add_child(head);
        let strip = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: m.px(26.0),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for (at, count) in pips.iter().copied().enumerate() {
            if count == 0 {
                continue;
            }
            let letter = "WUBRG".chars().nth(at).unwrap_or('C');
            let share = f32::from(count) / total as f32;
            let part = commands
                .spawn((
                    Node {
                        width: Val::Percent(share * 100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(mana_tone(letter)),
                    Pickable::IGNORE,
                ))
                .id();
            let said = commands
                .spawn((
                    Text::new(format!("{letter} {:.0}%", share * 100.0)),
                    tf_bold(kit.fonts, m.small * 0.9),
                    TextColor(crate::shellkit::tokens::INK_ON_LIGHT),
                    TextLayout::no_wrap(),
                    Node {
                        min_width: px_fixed(0.0),
                        flex_shrink: 1.0,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(part).add_child(said);
            commands.entity(strip).add_child(part);
        }
        commands.entity(column).add_child(strip);
    }

    // ---- the type counts
    let head = caption(commands, kit, Phrase::BuildByType.text(lang));
    commands.entity(column).add_child(head);
    let grid = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(16.0),
                row_gap: m.px(4.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (group, n) in deck.group_counts() {
        let name = cell(
            commands,
            kit,
            group.label().text(lang),
            m.text,
            tokens::INK,
            false,
        );
        commands.entity(name).insert(Node {
            flex_grow: 1.0,
            min_width: px_fixed(0.0),
            overflow: Overflow::clip_x(),
            ..default()
        });
        let count = words(commands, kit, &n.to_string(), m.text, tokens::INK);
        let pair = commands
            .spawn((
                Node {
                    width: Val::Percent(30.0),
                    min_width: m.px(140.0),
                    flex_grow: 1.0,
                    align_items: AlignItems::Center,
                    column_gap: m.px(8.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(pair).add_children(&[name, count]);
        commands.entity(grid).add_child(pair);
    }
    commands.entity(column).add_child(grid);

    // ---- the land share
    let share = f64::from(counts.lands) / f64::from(counts.main.max(1));
    let label = caption(commands, kit, Phrase::GroupLands.text(lang));
    let percent = (share * 100.0).round() as u32;
    let big = words(commands, kit, &format!("{percent} %"), m.head, tokens::INK);
    let of = words(
        commands,
        kit,
        &Phrase::BuildLandShare.fill(lang, &[&counts.lands.to_string(), &counts.main.to_string()]),
        m.small,
        tokens::MUTED,
    );
    let track = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                height: m.px(8.0),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent((share * 100.0) as f32),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(tokens::GOLD),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(fill);
    let lands = line(commands, kit, &[label, big, of, track]);
    commands.entity(column).add_child(lands);

    // ---- Draw seven
    let draw = controls::button(
        commands,
        kit,
        Phrase::BuildDrawSeven.text(lang),
        Weight::Primary,
        Live::Yes,
        None,
        (Press::Build(BuildPress::DrawSeven), stop("draw")),
    );
    let note = cell(
        commands,
        kit,
        Phrase::BuildDrawNote.text(lang),
        m.small,
        tokens::MUTED,
        false,
    );
    let draw_line = line(commands, kit, &[draw, note]);
    commands.entity(column).add_child(draw_line);
    if !ui.hand.is_empty() {
        let hand = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    column_gap: m.px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for slot in &ui.hand {
            let Some(card) = deck.card(*slot) else {
                continue;
            };
            let hover = hover_of_card(card);
            let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
            let height = m.scaled(78.0);
            commands.entity(thumb).insert((
                Node {
                    width: px_fixed((height * 5.0 / 7.0).round()),
                    height: px_fixed(height.round()),
                    flex_shrink: 0.0,
                    border_radius: BorderRadius::all(px_fixed(4.0)),
                    ..default()
                },
                hover,
                Pickable::default(),
                Press::Build(BuildPress::Inspect(*slot)),
            ));
            commands.entity(hand).add_child(thumb);
        }
        commands.entity(column).add_child(hand);
    }
    column
}
