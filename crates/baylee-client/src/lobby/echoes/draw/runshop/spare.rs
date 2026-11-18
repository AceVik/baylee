//! Sell unused nonbasic cards from the run collection directly for rarity-based Gold.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn toggle_button(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = kit.lang();
    let sellable = baylee_client_core::echoes::draft::sellable_cards(run, cohort);
    let count = sellable.len();
    let badge = if count > 0 {
        Phrase::EchoesSpareCardsToggle.fill(lang, &[&count.to_string()])
    } else {
        "0".to_owned()
    };
    let tone = if ui.spare_cards_open {
        IconTone::Active
    } else if count > 0 {
        IconTone::Gold
    } else {
        IconTone::Neutral
    };
    let available = run.phase == api::Phase::Shop && (count > 0 || ui.spare_cards_open);
    let tip = if count > 0 {
        Phrase::EchoesSpareCardsTitle.text(lang)
    } else {
        ""
    };
    icon_control(
        commands,
        parent,
        kit,
        ui,
        glyph::LIBRARY,
        Some(&badge),
        tone,
        tip,
        Action::ToggleSpareCards,
        available,
        (count == 0).then(|| Phrase::EchoesSpareCardsNoneTip.text(lang)),
    );
}

pub(super) fn draw_panel(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    if !ui.spare_cards_open || run.phase != api::Phase::Shop {
        return;
    }
    let sellable = baylee_client_core::echoes::draft::sellable_cards(run, cohort);
    if sellable.is_empty() {
        return;
    }
    let lang = kit.lang();
    let frame = towerui::frame(commands, root, kit);
    commands
        .entity(frame)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.padding = UiRect::axes(kit.m.px(18.0), kit.m.px(16.0));
            node.row_gap = kit.m.px(10.0);
        });

    let head = row(commands, frame, kit);
    commands.spawn((
        Text::new(glyph::COINS.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small),
        TextColor(tokens::GOLD),
        Pickable::IGNORE,
        ChildOf(head),
    ));
    let title = controls::label(
        commands,
        kit,
        Phrase::EchoesSpareCardsTitle.text(lang),
        kit.m.text,
        tokens::INK,
    );
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(head).add_child(title);
    label(
        commands,
        head,
        kit,
        Phrase::EchoesSpareCardsLead.text(lang),
        tokens::MUTED,
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(head).add_child(spacer);
    icon_control(
        commands,
        head,
        kit,
        ui,
        glyph::CLOSE,
        None,
        IconTone::Neutral,
        Phrase::ShellClose.text(lang),
        Action::ToggleSpareCards,
        true,
        None,
    );

    let list = row(commands, frame, kit);
    commands
        .entity(list)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.display = Display::Grid;
            node.grid_template_columns = vec![RepeatedGridTrack::flex(
                if kit.m.frame == crate::shellkit::Frame::Phone {
                    1
                } else {
                    3
                },
                1.0,
            )];
            node.column_gap = kit.m.px(10.0);
            node.row_gap = kit.m.px(8.0);
            node.align_items = AlignItems::Stretch;
        });

    let deck_clean = ui.choices.deck_unchanged(Some(run));
    for (card_id, gold) in sellable {
        spare_card_row(commands, list, kit, ui, cohort, card_id, gold, deck_clean);
    }
}

#[allow(clippy::too_many_arguments)]
fn spare_card_row(
    commands: &mut Commands,
    list: Entity,
    kit: Kit,
    ui: &Ui,
    cohort: &api::Cohort,
    card_id: baylee_core::ids::CardIndex,
    gold: u16,
    deck_clean: bool,
) {
    let lang = kit.lang();
    let rarity = baylee_cards::sides::rarity(card_id)
        .unwrap_or(baylee_client_core::deckbuilder::Rarity::Common);
    let rarity_color = crate::lobby::decks::rarity_dot_color(rarity);
    let hover = card::hover(card_id, Some(&cohort.catalog), lang);
    let row_card = commands
        .spawn((
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(6.0)),
                column_gap: kit.m.px(8.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(9.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.048, 0.068, 0.82)),
            BorderColor::all(rarity_color.with_alpha(0.45)),
            surfaces::elevation_subcard(),
            hover,
            Pickable::default(),
            ChildOf(list),
        ))
        .id();
    commands.spawn((
        Node {
            width: kit.m.px(8.0),
            height: kit.m.px(8.0),
            flex_shrink: 0.0,
            border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
            ..default()
        },
        BackgroundColor(rarity_color),
        Pickable::IGNORE,
        ChildOf(row_card),
    ));
    commands.spawn((
        Text::new(words::card(card_id, lang)),
        crate::hud::tf_bold(kit.fonts, kit.m.small),
        TextColor(tokens::INK),
        Node {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_width: px_fixed(0.0),
            ..default()
        },
        TextLayout {
            linebreak: LineBreak::WordOrCharacter,
            ..default()
        },
        Pickable::IGNORE,
        ChildOf(row_card),
    ));
    let sell_cmd = Command::SellCard { card: card_id };
    let can_sell = deck_clean && can(ui, &sell_cmd);
    icon_control(
        commands,
        row_card,
        kit,
        ui,
        glyph::COINS,
        Some(&format!("+{gold}g")),
        IconTone::Gold,
        &Phrase::EchoesSellCardTip.fill(lang, &[&gold.to_string()]),
        Action::SellCard(card_id),
        can_sell,
        (!deck_clean).then(|| Phrase::EchoesSellSaveFirstTip.text(lang)),
    );
}
