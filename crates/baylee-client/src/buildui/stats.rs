//! The Stats tab (§7): KPI overview cards (average mana value, land share,
//! instant-speed interaction, Central play/win record & achievement rank),
//! interactive mana curve, colour pips vs. mana sources, card type and rarity
//! breakdowns, hypergeometric opening-hand odds, and the 7-card sample hand
//! simulator.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::lobby::hover_of_card;
use crate::shellkit::focus::Current;
use baylee_client_core::deckbuilder::{Rarity, STAT_GROUPS};

/// The tallest a curve bar gets, before the text step.
const CURVE_HEIGHT: f32 = 104.0;

/// Wraps a statistics section in a styled elevated card.
fn stat_card(commands: &mut Commands, kit: Kit, children: &[Entity]) -> Entity {
    let m = kit.m;
    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(8.0),
                padding: UiRect::all(m.px(12.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.42)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_children(children);
    card
}

/// Spawns a compact KPI tile for the top metrics row.
fn kpi_tile(
    commands: &mut Commands,
    kit: Kit,
    icon_ch: char,
    accent: Color,
    label_txt: &str,
    value_txt: &str,
    sub_txt: &str,
) -> Entity {
    let m = kit.m;
    let box_node = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: m.px(130.0),
                min_width: m.px(120.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(3.0),
                padding: UiRect::axes(m.px(10.0), m.px(8.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.55)),
            BorderColor::all(accent.with_alpha(0.42)),
            Pickable::IGNORE,
        ))
        .id();
    let ico = glyph(commands, kit, icon_ch, m.small * 0.9, accent);
    let lbl = cell(
        commands,
        kit,
        label_txt,
        m.small * 0.86,
        tokens::MUTED,
        true,
    );
    let top_row = line(commands, kit, &[ico, lbl]);
    let val = cell(commands, kit, value_txt, m.head * 0.98, tokens::INK, true);
    let sub = cell(commands, kit, sub_txt, m.small * 0.86, accent, false);
    commands.entity(box_node).add_children(&[top_row, val, sub]);
    box_node
}

/// Hypergeometric probability `P(X = k)` when drawing `n` cards from a deck of
/// `pop` cards containing `successes` target cards.
fn hypergeom_pmf(pop: u32, successes: u32, n: u32, k: u32) -> f64 {
    if k > n || k > successes || (n - k) > pop.saturating_sub(successes) || pop < n {
        return 0.0;
    }
    let comb = |total: u32, choose: u32| -> f64 {
        let choose = choose.min(total.saturating_sub(choose));
        let mut acc = 1.0_f64;
        for i in 1..=choose {
            acc *= f64::from(total - choose + i) / f64::from(i);
        }
        acc
    };
    (comb(successes, k) * comb(pop - successes, n - k)) / comb(pop, n)
}

/// The numbers, as one column of rich cards.
#[allow(clippy::too_many_lines)] // complete statistics dashboard, read top to bottom
pub(crate) fn draw(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let german = lang == Lang::De;
    let deck = env.deck();
    let ui = env.ui();
    let column = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: m.px(10.0),
                flex_shrink: 0.0,
                padding: UiRect::bottom(m.px(8.0)),
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

    // ---- Compute detailed deck statistics from main deck entries
    let mut nonland_count = 0_u32;
    let mut total_cmc = 0_u32;
    let mut instant_speed = 0_u32;
    let mut legendary_count = 0_u32;
    let mut early_plays = 0_u32; // non-land MV <= 2
    let mut rarity_counts = [0_u32; 4]; // Common, Uncommon, Rare, Mythic
    let mut mana_sources = [0_u32; 5]; // W U B R G sources

    for entry in deck.entries(Zone::Main) {
        let Some(card) = deck.card(entry.slot) else {
            continue;
        };
        let n = u32::from(entry.count);
        let is_land = card.is("Land");
        if !is_land {
            nonland_count += n;
            total_cmc += card.cmc * n;
            if card.cmc <= 2 {
                early_plays += n;
            }
        }
        if card.is("Instant")
            || card.oracle_text.contains("Flash")
            || card.oracle_text.contains("Aufblitzen")
        {
            instant_speed += n;
        }
        if card.type_line.contains("Legendary") || card.type_line.contains("Legendär") {
            legendary_count += n;
        }
        match deck.entry_rarity(entry) {
            Rarity::Common => rarity_counts[0] += n,
            Rarity::Uncommon => rarity_counts[1] += n,
            Rarity::Rare => rarity_counts[2] += n,
            Rarity::Mythic | Rarity::Special | Rarity::Bonus => rarity_counts[3] += n,
        }
        if is_land || card.oracle_text.contains("Add {") || card.oracle_text.contains("Erzeuge {") {
            for (idx, ch) in "WUBRG".chars().enumerate() {
                if card.identity.contains(ch) {
                    mana_sources[idx] += n;
                }
            }
        }
    }

    let avg_cmc = if nonland_count > 0 {
        f64::from(total_cmc) / f64::from(nonland_count)
    } else {
        0.0
    };
    let land_share = f64::from(counts.lands) / f64::from(counts.main.max(1));
    let land_pct = (land_share * 100.0).round() as u32;
    let instant_pct =
        ((f64::from(instant_speed) / f64::from(nonland_count.max(1))) * 100.0).round() as u32;

    let deck_key = deck.editing().unwrap_or("new");
    let record = env.prefs.stats_for_deck(deck_key);
    let tier = record.rank_tier();
    let tier_lvl = tier.level();
    let tier_title = tier.title(lang);
    let win_pct = record.win_rate_pct().unwrap_or(0);

    // ---- 1. Top KPI Row
    let kpi_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(8.0),
                row_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let kpi_mv = kpi_tile(
        commands,
        kit,
        '\u{f51e}',
        tokens::ACCENT,
        if german {
            "Ø MANABETRAG"
        } else {
            "AVG MANA VALUE"
        },
        &format!("{avg_cmc:.2}"),
        &if german {
            format!("\u{03a3} {total_cmc} Mana \u{b7} {nonland_count} Zauber")
        } else {
            format!("\u{03a3} {total_cmc} mana \u{b7} {nonland_count} spells")
        },
    );
    let kpi_lands = kpi_tile(
        commands,
        kit,
        '\u{f6fc}',
        tokens::GOLD,
        if german {
            "LÄNDER-ANTEIL"
        } else {
            "LAND SHARE"
        },
        &format!("{land_pct} %"),
        &Phrase::BuildLandShare.fill(lang, &[&counts.lands.to_string(), &counts.main.to_string()]),
    );
    let kpi_tempo = kpi_tile(
        commands,
        kit,
        '\u{f0e7}',
        Color::srgb(0.45, 0.78, 0.96),
        if german {
            "INSTANT-TEMPO"
        } else {
            "INSTANT SPEED"
        },
        &format!("{instant_speed} ({instant_pct}%)"),
        &if german {
            format!("{legendary_count} Legendäre Karten")
        } else {
            format!("{legendary_count} legendary cards")
        },
    );
    let kpi_record = kpi_tile(
        commands,
        kit,
        '\u{f091}',
        if tier_lvl >= 5 {
            tokens::GOLD
        } else {
            tokens::ACCENT
        },
        if german {
            "BILANZ & RANG"
        } else {
            "RECORD & RANK"
        },
        &if record.games_played > 0 {
            format!("{}S / {}Sp ({win_pct}%)", record.wins, record.games_played)
        } else if german {
            "0 Spiele".to_string()
        } else {
            "0 games".to_string()
        },
        &format!("Stufe {tier_lvl}/11 \u{b7} {tier_title}"),
    );
    commands
        .entity(kpi_row)
        .add_children(&[kpi_mv, kpi_lands, kpi_tempo, kpi_record]);
    commands.entity(column).add_child(kpi_row);

    // ---- 2. Mana Curve Card
    let mut heading = format!(
        "{} \u{b7} {}",
        Phrase::BuildManaCurve.text(lang),
        Phrase::BuildCurveNote.text(lang)
    );
    if let Some(group) = ui.lit {
        heading.push_str(" \u{b7} ");
        heading.push_str(&Phrase::BuildCurveLit.fill(lang, &[group.label().text(lang)]));
    }
    let curve_head = caption(commands, kit, &heading);

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
        let height = 4.0 + (full - 4.0) * f32::from(all) / f32::from(tallest);
        let bar = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: px_fixed(height),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    border_radius: BorderRadius::all(px_fixed(4.0)),
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
                    BackgroundColor(tokens::ACCENT.with_alpha(0.82)),
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
    let curve_card = stat_card(commands, kit, &[curve_head, chips, bars]);
    commands.entity(column).add_child(curve_card);

    // ---- 3. Colour Pips vs. Mana Sources Card
    let pips = deck.pips();
    let total_pips: u32 = pips.iter().map(|p| u32::from(*p)).sum();
    let total_sources: u32 = mana_sources.iter().copied().sum();
    if total_pips > 0 || total_sources > 0 {
        let pips_head = caption(
            commands,
            kit,
            &format!(
                "{} \u{b7} {}",
                Phrase::BuildColourPips.text(lang),
                if german {
                    "Symbole in Kosten vs. Mana-Quellen"
                } else {
                    "Cost pips vs. mana sources"
                }
            ),
        );
        let mut pip_items = vec![pips_head];
        if total_pips > 0 {
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
                let share = f32::from(count) / total_pips as f32;
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
                let font = tf_bold(kit.fonts, m.small * 0.9);
                let mark_size = match font.font_size {
                    bevy::text::FontSize::Px(size) => size,
                    _ => m.small,
                };
                let glyph_opt = crate::manaui::glyph_of(letter);
                let said = commands
                    .spawn((
                        Text::new(if glyph_opt.is_some() {
                            String::new()
                        } else {
                            letter.to_string()
                        }),
                        font.clone(),
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
                if let Some(g) = glyph_opt {
                    let mark = commands
                        .spawn((
                            TextSpan::new(g.to_string()),
                            crate::manaui::mana_tf(kit.fonts, mark_size),
                            TextColor(crate::shellkit::tokens::INK_ON_LIGHT),
                            ChildOf(said),
                        ))
                        .id();
                    crate::manaui::ink_span(commands, mark, &g.to_string(), mark_size);
                }
                commands.spawn((
                    TextSpan::new(format!(" {:.0}%", share * 100.0)),
                    font,
                    TextColor(crate::shellkit::tokens::INK_ON_LIGHT),
                    ChildOf(said),
                ));
                commands.entity(part).add_child(said);
                commands.entity(strip).add_child(part);
            }
            pip_items.push(strip);
        }

        let balance_row = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: m.px(8.0),
                    row_gap: m.px(6.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        for (at, letter) in "WUBRG".chars().enumerate() {
            let p_cnt = pips[at];
            let s_cnt = mana_sources[at];
            if p_cnt == 0 && s_cnt == 0 {
                continue;
            }
            let badge = commands
                .spawn((
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: m.px(6.0),
                        padding: UiRect::axes(m.px(8.0), m.px(4.0)),
                        border: UiRect::all(px_fixed(1.0)),
                        border_radius: BorderRadius::all(px_fixed(6.0)),
                        ..default()
                    },
                    BackgroundColor(tokens::CONTROL.with_alpha(0.55)),
                    BorderColor::all(mana_tone(letter).with_alpha(0.65)),
                    Pickable::IGNORE,
                ))
                .id();
            let disc = crate::manaui::spawn_identity(
                commands,
                kit.fonts,
                &letter.to_string(),
                m.scaled(16.0),
                px_fixed(0.0),
            );
            let detail = cell(
                commands,
                kit,
                &if german {
                    format!("{p_cnt} Pips \u{b7} {s_cnt} Quellen")
                } else {
                    format!("{p_cnt} pips \u{b7} {s_cnt} sources")
                },
                m.small * 0.88,
                tokens::INK,
                false,
            );
            commands.entity(badge).add_children(&[disc, detail]);
            commands.entity(balance_row).add_child(badge);
        }
        pip_items.push(balance_row);
        let pip_card = stat_card(commands, kit, &pip_items);
        commands.entity(column).add_child(pip_card);
    }

    // ---- 4. Card Types & Rarity Breakdown Card
    let type_head = caption(commands, kit, Phrase::BuildByType.text(lang));
    let mut type_items = vec![type_head];
    for (group, n) in deck.group_counts() {
        let share = (f64::from(n) / f64::from(counts.main.max(1))) * 100.0;
        let name = cell(
            commands,
            kit,
            group.label().text(lang),
            m.small,
            tokens::INK,
            false,
        );
        commands.entity(name).insert(Node {
            width: m.px(110.0),
            flex_shrink: 0.0,
            ..default()
        });
        let track = commands
            .spawn((
                Node {
                    flex_grow: 1.0,
                    height: m.px(7.0),
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
                    width: Val::Percent(share as f32),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(if group == baylee_client_core::deckbuilder::Group::Land {
                    tokens::GOLD
                } else {
                    tokens::ACCENT.with_alpha(0.8)
                }),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(fill);
        let count_lbl = cell(
            commands,
            kit,
            &format!("{n} ({share:.0}%)"),
            m.small * 0.9,
            tokens::MUTED,
            true,
        );
        commands.entity(count_lbl).insert(Node {
            width: m.px(64.0),
            justify_content: JustifyContent::FlexEnd,
            flex_shrink: 0.0,
            ..default()
        });
        type_items.push(line(commands, kit, &[name, track, count_lbl]));
    }

    let rarity_head = caption(
        commands,
        kit,
        if german {
            "Seltenheits-Verteilung"
        } else {
            "Rarity distribution"
        },
    );
    type_items.push(rarity_head);
    let rarity_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: m.px(8.0),
                row_gap: m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    for (idx, rarity) in [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Mythic,
    ]
    .into_iter()
    .enumerate()
    {
        let cnt = rarity_counts[idx];
        let pill = commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: m.px(6.0),
                    padding: UiRect::axes(m.px(8.0), m.px(4.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(6.0)),
                    ..default()
                },
                BackgroundColor(tokens::CONTROL.with_alpha(0.55)),
                BorderColor::all(tokens::BORDER),
                Pickable::IGNORE,
            ))
            .id();
        let dot = super::rows::pool::rarity_dot(commands, kit, rarity, lang);
        let txt = cell(
            commands,
            kit,
            &format!("{} \u{b7} {cnt}", rarity_label(rarity, german)),
            m.small * 0.88,
            tokens::INK,
            cnt > 0,
        );
        commands.entity(pill).add_children(&[dot, txt]);
        commands.entity(rarity_row).add_child(pill);
    }
    type_items.push(rarity_row);
    let type_card = stat_card(commands, kit, &type_items);
    commands.entity(column).add_child(type_card);

    // ---- 5. Opening Hand Odds (Hypergeometric) & Sample Hand Simulator Card
    let pop = counts.main;
    let p_2plus_lands = if pop >= 7 {
        (2..=7)
            .map(|k| hypergeom_pmf(pop, counts.lands, 7, k))
            .sum::<f64>()
            * 100.0
    } else {
        0.0
    };
    let p_2to4_lands = if pop >= 7 {
        (2..=4)
            .map(|k| hypergeom_pmf(pop, counts.lands, 7, k))
            .sum::<f64>()
            * 100.0
    } else {
        0.0
    };
    let p_early_play = if pop >= 7 {
        (1.0 - hypergeom_pmf(pop, early_plays, 7, 0)) * 100.0
    } else {
        0.0
    };

    let odds_head = caption(
        commands,
        kit,
        if german {
            "Starthand-Chancen (7 Karten) & Simulator"
        } else {
            "Opening hand odds (7 cards) & simulator"
        },
    );
    let odds_text = cell(
        commands,
        kit,
        &if german {
            format!(
                "\u{2265} 2 Länder: {p_2plus_lands:.0}% \u{b7} 2\u{2013}4 Länder: {p_2to4_lands:.0}% \u{b7} Spielzug MV \u{2264} 2: {p_early_play:.0}%"
            )
        } else {
            format!(
                "\u{2265} 2 lands: {p_2plus_lands:.0}% \u{b7} 2\u{2013}4 lands: {p_2to4_lands:.0}% \u{b7} MV \u{2264} 2 play: {p_early_play:.0}%"
            )
        },
        m.small,
        tokens::ACCENT,
        true,
    );

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
    let mut sim_items = vec![odds_head, odds_text, draw_line];

    if !ui.hand.is_empty() {
        let mut hand_lands = 0_u32;
        let hand = commands
            .spawn((
                super::sample_hand::SampleHand,
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
            if card.is("Land") {
                hand_lands += 1;
            }
            let hover = hover_of_card(card);
            let thumb = crate::lobby::thumbnails::spawn(commands, &hover);
            let height = m.scaled(82.0);
            commands.entity(thumb).insert((
                Node {
                    width: px_fixed((height * 5.0 / 7.0).round()),
                    height: px_fixed(height.round()),
                    flex_shrink: 0.0,
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(4.0)),
                    ..default()
                },
                BorderColor::all(tokens::BORDER),
                hover,
                Pickable::default(),
                Press::Build(BuildPress::Inspect(*slot)),
            ));
            commands.entity(hand).add_child(thumb);
        }
        let hand_spells = u32::try_from(ui.hand.len())
            .unwrap_or(0)
            .saturating_sub(hand_lands);
        let hand_summary = cell(
            commands,
            kit,
            &if german {
                format!("Gezogene Hand: {hand_lands} Länder \u{b7} {hand_spells} Nichtländer")
            } else {
                format!("Drawn hand: {hand_lands} lands \u{b7} {hand_spells} non-lands")
            },
            m.small * 0.9,
            tokens::GOLD,
            true,
        );
        sim_items.push(hand_summary);
        sim_items.push(hand);
    }
    let sim_card = stat_card(commands, kit, &sim_items);
    commands.entity(column).add_child(sim_card);
    column
}
