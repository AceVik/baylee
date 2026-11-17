//! Expandable deck and mana-curve preview inside the spoils dialog.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::types::TypeSet;

pub(super) fn draw(commands: &mut Commands, parent: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    if !ui.reward_deck_open {
        return;
    }
    let lang = lang(kit);
    let catalog = ui.model.publication().map(|cohort| &cohort.catalog);
    let panel_card = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(kit.m.px(14.0)),
                row_gap: kit.m.px(10.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.068, 0.88)),
            BorderColor::all(tokens::BORDER),
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
            ChildOf(parent),
        ))
        .id();

    mana_curve_strip(commands, panel_card, kit, run, lang);

    let deck_row = |id: baylee_core::ids::CardIndex, count: u32| {
        let print = catalog
            .and_then(|cat| cat.card(id))
            .and_then(|card| card.run)
            .and_then(|facts| facts.presentation)
            .map_or_else(baylee_core::deckrow::PrintChoice::default, |art| {
                baylee_core::deckrow::PrintChoice {
                    scryfall_id: Some(art.printing.to_string()),
                    lang: Some("en".into()),
                    finish: Some(art.finish),
                    ..default()
                }
            });
        baylee_core::deckrow::Row {
            count,
            name: words::card(id, Lang::En),
            print: crate::towerprints::choice(&print, lang).into_owned(),
            note: None,
        }
        .to_string()
    };

    if let Some(formation) = &run.cards.formation {
        let commander_rows = [deck_row(formation.commander, 1)];
        let section = crate::lobby::decks::spawn_preview_section_container(
            commands,
            kit,
            &commander_rows,
            crate::lobby::decks::DecksView::Compact,
        );
        commands.entity(panel_card).add_child(section);
    }

    let library_rows: Vec<_> = run
        .cards
        .deck
        .iter()
        .map(|stack| deck_row(stack.card, u32::from(stack.copies)))
        .collect();
    if !library_rows.is_empty() {
        let section = crate::lobby::decks::spawn_preview_section_container(
            commands,
            kit,
            &library_rows,
            crate::lobby::decks::DecksView::Compact,
        );
        commands.entity(panel_card).add_child(section);
    }
}

fn mana_curve_strip(commands: &mut Commands, parent: Entity, kit: Kit, run: &api::Run, lang: Lang) {
    let mut bins = [0_u16; 7];
    let mut lands = 0_u16;
    let mut count_card = |card: baylee_core::ids::CardIndex, copies: u16| {
        let Some(def) = baylee_cards::by_index(card) else {
            return;
        };
        let Some(face) = def.faces.first() else {
            return;
        };
        if face.types.contains(TypeSet::LAND) {
            lands = lands.saturating_add(copies);
            return;
        }
        let slot = usize::try_from(face.mana_cost.cmc()).unwrap_or(6).min(6);
        bins[slot] = bins[slot].saturating_add(copies);
    };
    if let Some(formation) = &run.cards.formation {
        count_card(formation.commander, 1);
    }
    for stack in &run.cards.deck {
        count_card(stack.card, u16::from(stack.copies));
    }
    let peak = bins.iter().copied().max().unwrap_or(1).max(1);

    let band = row(commands, parent, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.align_items = AlignItems::End;
            node.column_gap = kit.m.px(10.0);
        });

    let title = controls::label(
        commands,
        kit,
        Phrase::EchoesRewardCurveLabel.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(band).add_child(title);

    let bars = commands
        .spawn((
            Node {
                column_gap: kit.m.px(6.0),
                align_items: AlignItems::End,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(band),
        ))
        .id();
    let labels = ["0", "1", "2", "3", "4", "5", "6+"];
    for (cost_label, count) in labels.into_iter().zip(bins) {
        spawn_curve_column(commands, bars, kit, cost_label, count, peak);
    }

    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(band).add_child(spacer);
    let land_badge = crate::lobby::parts::badge_with(
        commands,
        kit,
        glyph::COMPASS,
        &format!("{lands} {}", Phrase::DecksSectionLands.text(lang)),
        tokens::ACCENT,
    );
    commands.entity(band).add_child(land_badge);
}

fn spawn_curve_column(
    commands: &mut Commands,
    bars: Entity,
    kit: Kit,
    cost_label: &str,
    count: u16,
    peak: u16,
) {
    let col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(2.0),
                min_width: kit.m.px(22.0),
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(bars),
        ))
        .id();
    let count_text = controls::label(
        commands,
        kit,
        &count.to_string(),
        kit.m.small * 0.88,
        if count > 0 {
            tokens::INK
        } else {
            tokens::MUTED
        },
    );
    commands.entity(col).add_child(count_text);
    let bar_h = if count == 0 {
        3.0
    } else {
        6.0 + (f32::from(count) / f32::from(peak)) * 24.0
    };
    commands.spawn((
        Node {
            width: kit.m.px(14.0),
            height: kit.m.px(bar_h),
            border_radius: BorderRadius::all(kit.m.px(3.0)),
            ..default()
        },
        BackgroundColor(if count > 0 {
            tokens::GOLD.with_alpha(0.82)
        } else {
            tokens::BORDER.with_alpha(0.35)
        }),
        Pickable::IGNORE,
        ChildOf(col),
    ));
    let cmc_label = controls::label(commands, kit, cost_label, kit.m.small * 0.85, tokens::MUTED);
    commands.entity(col).add_child(cmc_label);
}
