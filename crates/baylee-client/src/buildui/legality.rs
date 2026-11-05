//! The deck's standing on the screen: the line under the list, the sheet that
//! chooses what it is held to, and the word a row wears.
//!
//! `DeckBuilder::overview` decides; this draws. A standing is read by its
//! mark as well as its ink, because colour alone says nothing to a good part
//! of the people looking at it, and [`Standing::Unknown`] never wears what
//! legal wears: a deck nobody could check is not a deck that passed.

#[allow(clippy::wildcard_imports)] // the builder's own vocabulary
use super::*;
use crate::shellkit::surfaces::{self, SheetWidth};
use baylee_cardtext::Standing;
use baylee_client_core::deckbuilder::legality::Line;
use baylee_client_core::deckcheck::format_name;

/// A standing's mark and ink.
fn sign(standing: Standing) -> (char, Color) {
    match standing {
        Standing::Legal => (mark::CHECK, tokens::ACCENT),
        Standing::Illegal => (mark::WARN, tokens::DANGER),
        Standing::Unknown => (mark::HELP, tokens::GOLD),
    }
}

/// The overview's sleek single-row bar at the bottom of the deck column:
/// matches the pool footer's height (`38px`) so the pool list and the deck
/// list end on the exact same horizontal line.
#[allow(clippy::too_many_lines)] // one cohesive status bar with standing, primary problem, count, and check trigger
pub(super) fn foot(commands: &mut Commands, holder: Entity, env: &Env) {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let overview = deck.overview(lang);
    let (mark_ch, ink) = sign(overview.standing);

    let mut extra_notes: Vec<(String, Color)> = Vec::new();
    for problem in &overview.problems {
        extra_notes.push((problem.text.clone(), tokens::DANGER));
    }
    for problem in deck.problems(lang) {
        let color = if problem.blocking {
            tokens::DANGER
        } else {
            tokens::GOLD
        };
        if !extra_notes.iter().any(|(t, _)| t == &problem.message) {
            extra_notes.push((problem.message, color));
        }
    }
    let shaky = deck.counts().shaky;
    if shaky > 0 {
        let said = if shaky == 1 {
            Phrase::BuildShakyOne.text(lang).to_string()
        } else {
            Phrase::BuildShakyMany.fill(lang, &[&shaky.to_string()])
        };
        extra_notes.push((said, tokens::GOLD));
    }
    for missing in deck.missing() {
        extra_notes.push((Phrase::DroppedCards.fill(lang, &[missing]), tokens::DANGER));
    }

    let bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: m.px(38.0),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: m.px(8.0),
                padding: UiRect::axes(m.px(12.0), px_fixed(0.0)),
                border: UiRect::new(px_fixed(3.0), px_fixed(1.0), px_fixed(1.0), px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.52)),
            BorderColor::all(ink.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    if !extra_notes.is_empty() {
        let tip = extra_notes
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<Vec<_>>()
            .join(" \u{b7} ");
        commands
            .entity(bar)
            .insert((crate::lobby::HoverHint(tip), Pickable::default()));
    }

    let sign_node = glyph(commands, kit, mark_ch, m.small, ink);
    let headline = cell(commands, kit, &overview.headline, m.small, ink, true);
    commands.entity(headline).insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
    commands.entity(bar).add_children(&[sign_node, headline]);

    if let Some((first_msg, first_ink)) = extra_notes.first() {
        let sep = words(commands, kit, "\u{b7}", m.small, tokens::MUTED);
        let note = cell(commands, kit, first_msg, m.small * 0.92, *first_ink, true);
        commands.entity(bar).add_children(&[sep, note]);
    }

    let gap = spring(commands);
    let count = cell(
        commands,
        kit,
        &overview.count,
        m.small * 0.94,
        tokens::MUTED,
        false,
    );
    commands.entity(count).insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
    let open = link(
        commands,
        kit,
        Phrase::CheckOpen.text(lang),
        (Press::Build(BuildPress::OpenCheck), stop("check")),
    );
    commands.entity(open).insert(Node {
        flex_shrink: 0.0,
        ..default()
    });
    commands.entity(bar).add_children(&[gap, count, open]);
    commands.entity(holder).add_child(bar);
}

/// One problem: a press on its card where it has one, else its words.
fn problem_line(commands: &mut Commands, kit: Kit, problem: &Line, id: &'static str) -> Entity {
    match problem.slot {
        Some(slot) => link(
            commands,
            kit,
            &problem.text,
            (Press::Build(BuildPress::CheckCard(slot)), stop(id)),
        ),
        None => cell(
            commands,
            kit,
            &problem.text,
            kit.m.small,
            tokens::DANGER,
            true,
        ),
    }
}

/// The word a deck row wears, drawn as a small plate of danger ink.
pub(super) fn badge(commands: &mut Commands, kit: Kit, word: &str) -> Entity {
    let m = kit.m;
    let plate = commands
        .spawn((
            Node {
                padding: UiRect::axes(m.px(6.0), m.px(1.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                flex_shrink: 0.0,
                ..default()
            },
            BorderColor::all(tokens::DANGER),
            Pickable::IGNORE,
        ))
        .id();
    let words = controls::label(commands, kit, word, m.small * 0.9, tokens::DANGER);
    commands.entity(plate).add_child(words);
    plate
}

/// A styled section box inside the Format & List dialog.
fn section_card(commands: &mut Commands, kit: Kit, children: &[Entity]) -> Entity {
    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(8.0),
                padding: UiRect::all(kit.m.px(12.0)),
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

/// The sheet: what the deck is held to, and everything the check found.
/// Has an upgraded header lockup with a top-right `×` close button and no bottom footer.
#[allow(clippy::too_many_lines)] // header lockup + format section + banlist section + standing section
pub(super) fn sheet(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let overview = deck.overview(lang);
    let (mark_ch, ink) = sign(overview.standing);
    let phone = m.frame == Frame::Phone;
    let wide = match SheetWidth::Medium {
        SheetWidth::Small => 560.0,
        SheetWidth::Medium => 720.0,
        SheetWidth::Large => 960.0,
    };

    let surface = commands
        .spawn((
            Role::Opaque,
            Node {
                width: if phone {
                    Val::Percent(100.0)
                } else {
                    m.px(wide)
                },
                max_width: Val::Percent(100.0),
                max_height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(if phone {
                    0.0
                } else {
                    tokens::RADIUS_PANEL
                })),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tokens::OPAQUE),
            BackgroundGradient::from(LinearGradient::to_bottom(vec![
                Color::srgba(0.11, 0.16, 0.22, 0.99).into(),
                tokens::OPAQUE.into(),
            ])),
            BorderColor::all(tokens::PRIMARY_EDGE.with_alpha(0.55)),
            surfaces::popover_shadow(),
        ))
        .id();

    // ---- Upgraded Dialog Header with Icon Badge, Title + Subtitle, and Top-Right × Button
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: m.px(12.0),
                padding: UiRect::axes(px_fixed(m.pad), m.px(14.0)),
                border: UiRect::bottom(px_fixed(1.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.45)),
            BorderColor::all(tokens::BORDER.with_alpha(0.75)),
            Pickable::IGNORE,
        ))
        .id();
    let head_left = commands
        .spawn((
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(12.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let badge_box = commands
        .spawn((
            Node {
                width: m.px(38.0),
                height: m.px(38.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(10.0)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(tokens::SELECTED),
            BorderColor::all(tokens::ACCENT.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    let badge_icon = glyph(commands, kit, '\u{f24e}', m.text, tokens::ACCENT);
    commands.entity(badge_box).add_child(badge_icon);

    let title_col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: m.px(2.0),
                flex_shrink: 1.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let title_text = commands
        .spawn((
            Text::new(Phrase::CheckTitle.text(lang)),
            tf_bold(kit.fonts, m.head * 1.05),
            TextColor(tokens::INK),
            TextShadow {
                offset: Vec2::new(0.0, 1.5),
                color: Color::srgba(0.0, 0.0, 0.0, 0.65),
            },
            Pickable::IGNORE,
        ))
        .id();
    let sub_text = cell(
        commands,
        kit,
        &format!("{} \u{b7} {}", overview.headline, overview.count),
        m.small,
        ink,
        false,
    );
    commands
        .entity(title_col)
        .add_children(&[title_text, sub_text]);
    commands
        .entity(head_left)
        .add_children(&[badge_box, title_col]);

    let close_btn = icon_button(
        commands,
        kit,
        mark::CLOSE,
        false,
        (Press::Build(BuildPress::CloseCheck), sheet_stop("close")),
    );
    commands.entity(head).add_children(&[head_left, close_btn]);

    // ---- Body Sections
    let mut body = Vec::new();

    // 1. Format Section Card
    let chosen = deck.format();
    let chips: Vec<Entity> = deck
        .format_choices()
        .iter()
        .enumerate()
        .map(|(at, key)| {
            controls::chip(
                commands,
                kit,
                format_name(key, lang),
                key == chosen,
                None,
                false,
                (
                    Press::Build(BuildPress::SetFormat(at)),
                    sheet_stop("format"),
                ),
            )
        })
        .collect();
    let fmt_head = caption(commands, kit, Phrase::CheckFormat.text(lang));
    let fmt_wrap = wrapped(commands, kit, &chips);
    body.push(section_card(commands, kit, &[fmt_head, fmt_wrap]));

    // 1b. Multi-Format Deck Variants Section Card
    let deck_key = deck.editing().unwrap_or("new");
    let variants = env.prefs.variants_for_deck(deck_key);
    let var_title = caption(commands, kit, Phrase::BuildVariantsTitle.text(lang));
    let save_var_label = Phrase::BuildVariantSave.fill(lang, &[format_name(chosen, lang)]);
    let save_var_btn = controls::chip(
        commands,
        kit,
        &save_var_label,
        false,
        None,
        false,
        (
            Press::Build(BuildPress::SaveFormatVariant),
            sheet_stop("save-variant"),
        ),
    );
    let head_gap = spring(commands);
    let var_head_row = line(commands, kit, &[var_title, head_gap, save_var_btn]);
    let var_desc = surfaces::prose(commands, kit, Phrase::BuildVariantsNote.text(lang), true);
    let mut var_items = vec![var_head_row, var_desc];
    for (idx, variant) in variants.iter().enumerate() {
        let report = deck.validate_variant(variant);
        let (v_mark, v_ink) = sign(report.standing());
        let main_cnt: u32 = variant
            .cards
            .iter()
            .filter_map(|r| baylee_core::deckrow::parse(r).ok())
            .map(|r| r.count)
            .sum();
        let side_cnt: u32 = variant
            .sideboard
            .iter()
            .filter_map(|r| baylee_core::deckrow::parse(r).ok())
            .map(|r| r.count)
            .sum();
        let status_word = match report.standing() {
            Standing::Legal => Phrase::BuildVariantLegal.text(lang).to_string(),
            Standing::Illegal => {
                Phrase::BuildVariantIssues.fill(lang, &[&report.problems.len().to_string()])
            }
            Standing::Unknown => Phrase::BuildVariantUnchecked.text(lang).to_string(),
        };
        let row_box = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    column_gap: m.px(8.0),
                    padding: UiRect::axes(m.px(10.0), m.px(6.0)),
                    border: UiRect::all(px_fixed(1.0)),
                    border_radius: BorderRadius::all(px_fixed(8.0)),
                    ..default()
                },
                BackgroundColor(if variant.format == chosen {
                    tokens::SELECTED.with_alpha(0.55)
                } else {
                    tokens::CONTROL.with_alpha(0.55)
                }),
                BorderColor::all(v_ink.with_alpha(0.45)),
                Pickable::IGNORE,
            ))
            .id();
        let v_sign = glyph(commands, kit, v_mark, m.small, v_ink);
        let v_name = cell(
            commands,
            kit,
            format_name(&variant.format, lang),
            m.small,
            tokens::INK,
            true,
        );
        let v_meta = cell(
            commands,
            kit,
            &format!("{main_cnt} / {side_cnt} \u{b7} {status_word}"),
            m.small * 0.9,
            v_ink,
            false,
        );
        let load_chip = controls::chip(
            commands,
            kit,
            Phrase::BuildVariantActivate.text(lang),
            variant.format == chosen,
            None,
            false,
            Press::Build(BuildPress::LoadFormatVariant(idx)),
        );
        let del_btn = icon_button(
            commands,
            kit,
            '\u{f1f8}',
            false,
            Press::Build(BuildPress::DeleteFormatVariant(idx)),
        );
        let row_gap = spring(commands);
        commands
            .entity(row_box)
            .add_children(&[v_sign, v_name, v_meta, row_gap, load_chip, del_btn]);
        var_items.push(row_box);
    }
    body.push(section_card(commands, kit, &var_items));

    // 2. Banlist Section Card
    let mut list_items = Vec::new();
    super::lists::picker(commands, env, &mut list_items);
    if !list_items.is_empty() {
        body.push(section_card(commands, kit, &list_items));
    }

    // 3. Standing & Problems Section Card
    let mut status_items = Vec::new();
    let sign_node = glyph(commands, kit, mark_ch, m.text, ink);
    let headline = cell(commands, kit, &overview.headline, m.text, ink, true);
    let gap = spring(commands);
    let count = cell(
        commands,
        kit,
        &overview.count,
        m.small,
        tokens::MUTED,
        false,
    );
    status_items.push(line(commands, kit, &[sign_node, headline, gap, count]));
    for problem in &overview.problems {
        status_items.push(problem_line(commands, kit, problem, "sheet-problem"));
    }
    for problem in deck.problems(lang) {
        let color = if problem.blocking {
            tokens::DANGER
        } else {
            tokens::GOLD
        };
        status_items.push(cell(
            commands,
            kit,
            &problem.message,
            m.small,
            color,
            problem.blocking,
        ));
    }
    for notice in &overview.notices {
        status_items.push(surfaces::prose(commands, kit, notice, false));
    }
    body.push(section_card(commands, kit, &status_items));

    let scroller = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                min_height: px_fixed(0.0),
                padding: UiRect::all(px_fixed(m.pad)),
                row_gap: px_fixed(m.gap),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            crate::lobby::Scrollable(crate::lobby::List::Check),
        ))
        .id();
    commands.entity(scroller).add_children(&body);

    let band = crate::lobby::scrollbars::band_when_needed(commands, scroller, phone);
    commands.entity(surface).add_children(&[head, band]);
    surface
}

/// Chips in rows that wrap.
pub(super) fn wrapped(commands: &mut Commands, kit: Kit, chips: &[Entity]) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_wrap: FlexWrap::Wrap,
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_children(chips);
    row
}
