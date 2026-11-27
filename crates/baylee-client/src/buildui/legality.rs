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

/// The overview's status bar at the bottom of the deck column: matches the
/// pool footer's minimum height (`38px`) when clean, wraps its headline and
/// primary advisory cleanly without mid-word clipping when long, carries all
/// advisories in its hover tooltip, and keeps the "Format and list" button
/// intact down to 1280×720.
#[allow(clippy::too_many_lines)] // cohesive status bar with standing, primary problem, count, and check trigger
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
                min_height: m.px(38.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: m.px(3.0),
                padding: UiRect::axes(m.px(12.0), m.px(6.0)),
                border: UiRect::new(px_fixed(3.0), px_fixed(1.0), px_fixed(1.0), px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.52)),
            BorderColor::all(ink.with_alpha(0.55)),
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
        ))
        .id();
    let mut tip_parts = vec![overview.headline.clone()];
    tip_parts.extend(extra_notes.iter().map(|(text, _)| text.clone()));
    if !extra_notes.is_empty() {
        commands.entity(bar).insert((
            crate::lobby::HoverHint(tip_parts.join(" \u{b7} ")),
            Pickable::default(),
        ));
    }

    let top_row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let lead = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let sign_node = glyph(commands, kit, mark_ch, m.small, ink);
    let headline = commands
        .spawn((
            Text::new(overview.headline.as_str()),
            tf_bold(kit.fonts, m.small),
            TextColor(ink),
            TextLayout::new(Justify::Left, bevy::text::LineBreak::WordBoundary),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(lead).add_children(&[sign_node, headline]);

    let trail = commands
        .spawn((
            Node {
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                column_gap: m.px(8.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let count = commands
        .spawn((
            Text::new(overview.count.as_str()),
            tf(kit.fonts, m.small * 0.94),
            TextColor(tokens::MUTED),
            TextLayout::no_wrap(),
            Node {
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let open = link(
        commands,
        kit,
        if deck.limited().is_some() {
            Phrase::EchoesDeckCheck.text(lang)
        } else {
            Phrase::CheckOpen.text(lang)
        },
        (Press::Build(BuildPress::OpenCheck), stop("check")),
    );
    commands.entity(open).entry::<Node>().and_modify(|mut n| {
        n.flex_shrink = 0.0;
    });
    commands.entity(trail).add_children(&[count, open]);
    commands.entity(top_row).add_children(&[lead, trail]);
    commands.entity(bar).add_child(top_row);

    if let Some((first_msg, first_ink)) = extra_notes.first() {
        let note_text = if extra_notes.len() > 1 {
            format!("{first_msg} (+{})", extra_notes.len() - 1)
        } else {
            first_msg.clone()
        };
        let note = commands
            .spawn((
                Text::new(note_text),
                tf_bold(kit.fonts, m.small * 0.92),
                TextColor(*first_ink),
                TextLayout::new(Justify::Left, bevy::text::LineBreak::WordBoundary),
                Node {
                    width: Val::Percent(100.0),
                    min_width: px_fixed(0.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(bar).add_child(note);
    }

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
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(card).add_children(children);
    card
}

/// The sheet: what the deck is held to, and everything the check found.
/// Uses [`surfaces::sheet_box_headed_listed`] with an icon badge, standing
/// subtitle, top-right `×` close button, and empty footer.
#[allow(clippy::too_many_lines)] // format section + variants section + banlist section + standing section
pub(super) fn sheet(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let lang = env.lang();
    let deck = env.deck();
    let overview = deck.overview(lang);
    let (mark_ch, ink) = sign(overview.standing);
    let sub_line = format!("{} \u{b7} {}", overview.headline, overview.count);

    // ---- Body Sections
    let mut body = Vec::new();

    if deck.limited().is_some() {
        body.push(surfaces::prose(
            commands,
            kit,
            Phrase::EchoesRunDeckRules.text(lang),
            false,
        ));
    } else {
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
                    surfaces::elevation_subcard(),
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
            super::pool::elevate_icon_button(commands, del_btn);
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

    surfaces::sheet_box_headed_listed(
        commands,
        kit,
        SheetWidth::Medium,
        surfaces::SheetHeader::new(Phrase::CheckTitle.text(lang))
            .icon('\u{f24e}')
            .subtitle_colored(&sub_line, ink)
            .close((Press::Build(BuildPress::CloseCheck), sheet_stop("close"))),
        &body,
        &[],
        crate::lobby::Scrollable(crate::lobby::List::Check),
    )
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
