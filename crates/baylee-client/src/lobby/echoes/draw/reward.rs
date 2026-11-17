//! Five authoritative offers over the shop; selection never spends or grants locally.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::deckbuilder::Rarity as CardRarity;
use baylee_client_core::echoes::draft::{RewardGold, reward_card_duplicate_gold, reward_gold};
use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};

mod deck_preview;

const LUMINOUS_GOLD: Color = Color::srgb(1.0, 0.87, 0.44);

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    scrolled: &Scrolled,
) {
    let Some(reward) = &run.reward else {
        return;
    };
    let lang = lang(kit);
    let body = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(14.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();

    let gold_info = reward_gold(run, &ui.choices.rewards);
    reward_toolbar(commands, body, kit, ui, run, gold_info);
    if ui.reward_gold_open
        && let Some(gold) = gold_info
    {
        gold_breakdown_card(commands, body, kit, run, reward, gold);
    }

    let shelf = row(commands, body, kit);
    commands
        .entity(shelf)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.justify_content = JustifyContent::Center;
            node.align_items = AlignItems::Stretch;
            node.column_gap = kit.m.px(12.0);
            node.row_gap = kit.m.px(12.0);
        });
    for &id in &reward.cards {
        candidate(commands, shelf, kit, ui, run, reward.take, id);
    }
    if reward.cards.is_empty() {
        label(
            commands,
            body,
            kit,
            Phrase::EchoesRewardEmpty.text(lang),
            tokens::MUTED,
        );
    }

    deck_preview::draw(commands, body, kit, ui, run);

    let footer = selection_footer(commands, kit, ui, reward.take);
    let header = surfaces::SheetHeader::new(Phrase::EchoesRewardReady.text(lang))
        .icon_colored(glyph::TROPHY, tokens::GOLD)
        .subtitle(Phrase::EchoesRewardLead.text(lang));
    let surface = surfaces::sheet_box_headed_listed(
        commands,
        kit,
        surfaces::SheetWidth::Large,
        header,
        &[body],
        &[footer],
        (
            Scrollable(List::EchoesInspector),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::EchoesInspector))),
        ),
    );
    commands
        .entity(surface)
        .insert(Press::Shared(crate::lobby::SharedPress::PickerNothing));
    let scrim = surfaces::sheet(commands, surface);
    commands.entity(root).add_child(scrim);
}

fn reward_toolbar(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    gold_info: Option<RewardGold>,
) {
    let lang = lang(kit);
    let bar = row(commands, body, kit);
    commands.entity(bar).entry::<Node>().and_modify(|mut node| {
        node.justify_content = JustifyContent::SpaceBetween;
        node.align_items = AlignItems::Center;
    });

    if let Some(gold) = gold_info {
        spawn_gold_toggle(commands, bar, kit, ui, gold);
    }

    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(bar).add_child(spacer);

    let deck_total: u16 = run
        .cards
        .deck
        .iter()
        .map(|entry| u16::from(entry.copies))
        .sum::<u16>()
        + u16::from(run.cards.formation.is_some());
    let deck_btn = controls::button_compact(
        commands,
        kit,
        Some(glyph::LIBRARY),
        &Phrase::EchoesRewardViewDeck.fill(lang, &[&deck_total.to_string()]),
        if ui.reward_deck_open {
            Weight::Primary
        } else {
            Weight::Secondary
        },
        if ui.model.mutating() {
            Live::No(Phrase::EchoesUnavailable.text(lang))
        } else {
            Live::Yes
        },
        Press::Echoes(Action::ToggleRewardDeck),
    );
    commands.entity(deck_btn).insert((
        super::super::keys::stop(Action::ToggleRewardDeck, ui),
        crate::shellkit::focus::Current(ui.reward_deck_open),
    ));
    commands.entity(bar).add_child(deck_btn);
}

fn spawn_gold_toggle(commands: &mut Commands, bar: Entity, kit: Kit, ui: &Ui, gold: RewardGold) {
    let lang = lang(kit);
    let ground = if ui.reward_gold_open {
        Color::srgba(0.16, 0.12, 0.04, 0.94)
    } else {
        Color::srgba(0.08, 0.06, 0.03, 0.90)
    };
    let rim = Color::srgba(
        0.95,
        0.78,
        0.34,
        if ui.reward_gold_open { 0.92 } else { 0.72 },
    );
    let face = commands
        .spawn((
            crate::shellkit::Role::Button,
            Node {
                min_height: kit.m.px(30.0),
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(4.0)),
                column_gap: kit.m.px(7.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(rim),
            surfaces::elevation_control(),
            crate::ambience::Feel {
                lift: 0.0,
                ..crate::ambience::Feel::new(ground)
            },
            crate::ambience::FeelBorder {
                rest: rim,
                hot: LUMINOUS_GOLD,
                press: tokens::GOLD,
            },
            crate::ambience::FeelShadow,
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        Text::new(glyph::COINS.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small),
        TextColor(tokens::GOLD),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    let earned_text =
        Phrase::EchoesRewardEarnedGold.fill(lang, &[&gold.total_with_duplicates.to_string()]);
    commands.spawn((
        Text::new(earned_text),
        crate::hud::tf_bold(kit.fonts, kit.m.small * 1.05),
        TextColor(LUMINOUS_GOLD),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    let chevron = if ui.reward_gold_open {
        glyph::CHEVRON_UP
    } else {
        glyph::CHEVRON_DOWN
    };
    commands.spawn((
        Text::new(chevron.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.78),
        TextColor(tokens::MUTED),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    let wrapper = controls::hit(commands, kit, face, Press::Echoes(Action::ToggleRewardGold));
    commands.entity(wrapper).insert((
        super::super::keys::stop(Action::ToggleRewardGold, ui),
        crate::shellkit::focus::Current(ui.reward_gold_open),
    ));
    controls::attach_tooltip(
        commands,
        wrapper,
        kit,
        Phrase::EchoesRewardGoldBreakdownTip.text(lang),
    );
    commands.entity(bar).add_child(wrapper);
}

fn spawn_breakdown_row(
    commands: &mut Commands,
    box_card: Entity,
    kit: Kit,
    lang: Lang,
    source: &str,
    amount: u16,
    highlight: bool,
) {
    let line = row(commands, box_card, kit);
    commands
        .entity(line)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::SpaceBetween;
        });
    label(
        commands,
        line,
        kit,
        source,
        if highlight {
            tokens::INK
        } else {
            tokens::MUTED
        },
    );
    let value = controls::label(
        commands,
        kit,
        &format!("+{amount} {}", Phrase::EchoesGold.text(lang)),
        kit.m.small,
        if amount > 0 {
            LUMINOUS_GOLD
        } else {
            tokens::MUTED
        },
    );
    commands
        .entity(value)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(line).add_child(value);
}

fn gold_breakdown_card(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    run: &api::Run,
    reward: &api::Reward,
    gold: RewardGold,
) {
    let lang = lang(kit);
    let box_card = commands
        .spawn((
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(kit.m.px(14.0), kit.m.px(10.0)),
                row_gap: kit.m.px(6.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.88)),
            BorderColor::all(Color::srgba(0.92, 0.74, 0.34, 0.45)),
            surfaces::elevation_subcard(),
            Pickable::IGNORE,
            ChildOf(body),
        ))
        .id();

    spawn_breakdown_row(
        commands,
        box_card,
        kit,
        lang,
        Phrase::EchoesRewardSourceDuel.text(lang),
        gold.base_duel,
        true,
    );
    let relic_label = if gold.equipped_acorns > 0 {
        format!(
            "{} (×{})",
            Phrase::EchoesRewardSourceRelics.text(lang),
            gold.equipped_acorns
        )
    } else {
        Phrase::EchoesRewardSourceRelics.text(lang).to_owned()
    };
    spawn_breakdown_row(
        commands,
        box_card,
        kit,
        lang,
        &relic_label,
        gold.relic_bonus,
        gold.relic_bonus > 0,
    );

    let any_owned_candidate = reward
        .cards
        .iter()
        .any(|&card| reward_card_duplicate_gold(run, card).is_some());
    if gold.duplicate_total > 0 || any_owned_candidate {
        let dup_label = format!(
            "{} (+{}g)",
            Phrase::EchoesRewardSourceDuplicates.text(lang),
            gold.duplicate_rate
        );
        spawn_breakdown_row(
            commands,
            box_card,
            kit,
            lang,
            &dup_label,
            gold.duplicate_total,
            gold.duplicate_total > 0,
        );
    }
}

fn selection_footer(commands: &mut Commands, kit: Kit, ui: &Ui, take: u8) -> Entity {
    let lang = lang(kit);
    let selected_count = ui.choices.rewards.len();
    let required = usize::from(take);
    let ready = selected_count == required;
    let allowed = ready
        && !ui.model.mutating()
        && can(
            ui,
            &Command::Claim {
                cards: ui.choices.rewards.clone(),
            },
        );
    let label_text = if ready {
        Phrase::EchoesRewardClaim.text(lang).to_owned()
    } else {
        Phrase::EchoesRewardClaimProgress
            .fill(lang, &[&selected_count.to_string(), &required.to_string()])
    };
    let need_more =
        Phrase::EchoesRewardNeedMore.fill_count(lang, required.saturating_sub(selected_count), &[]);
    let live = if allowed {
        Live::Yes
    } else if !ready {
        Live::No(&need_more)
    } else {
        Live::No(Phrase::EchoesUnavailable.text(lang))
    };
    let confirm = controls::button_with_icon(
        commands,
        kit,
        if ready { glyph::CROWN } else { glyph::CHECK },
        &label_text,
        Weight::Gold,
        live,
        None,
        Press::Echoes(Action::ClaimReward),
    );
    commands
        .entity(confirm)
        .insert(super::super::keys::stop(Action::ClaimReward, ui))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.min_width = kit.m.px(320.0);
            node.max_width = kit.m.px(420.0);
            node.width = percent(100);
        });
    let holder = commands
        .spawn((
            Node {
                width: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(holder).add_child(confirm);
    holder
}

fn rarity_visuals(
    rarity: CardRarity,
    selected: bool,
) -> (&'static str, Color, LinearGradient, BoxShadow, bool) {
    let rarity_color = crate::lobby::decks::rarity_dot_color(rarity);
    let rarity_name = match rarity {
        CardRarity::Common => "Common",
        CardRarity::Uncommon => "Uncommon",
        CardRarity::Rare => "Rare",
        CardRarity::Mythic => "Mythic Rare",
        CardRarity::Special | CardRarity::Bonus => "Special",
    };
    let ornate = matches!(rarity, CardRarity::Rare | CardRarity::Mythic) || selected;
    if selected {
        let rim = Color::srgb(1.0, 0.86, 0.40);
        let grad = LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.18, 0.14, 0.05, 0.96), 0.0),
            ColorStop::percent(Color::srgba(0.08, 0.07, 0.04, 0.96), 55.0),
            ColorStop::percent(Color::srgba(0.04, 0.05, 0.07, 0.98), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba);
        let shadow = BoxShadow(vec![
            ShadowStyle {
                color: Color::srgba(1.0, 0.82, 0.28, 0.42),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(1.5),
                blur_radius: px_fixed(18.0),
            },
            ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.65),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(8.0),
                spread_radius: px_fixed(-1.0),
                blur_radius: px_fixed(20.0),
            },
        ]);
        return (rarity_name, rim, grad, shadow, ornate);
    }
    let (top_tint, glow_alpha, blur) = match rarity {
        CardRarity::Mythic => (Color::srgba(0.20, 0.08, 0.04, 0.95), 0.34, 16.0),
        CardRarity::Rare => (Color::srgba(0.16, 0.12, 0.04, 0.95), 0.28, 14.0),
        CardRarity::Uncommon => (Color::srgba(0.06, 0.11, 0.16, 0.95), 0.16, 10.0),
        CardRarity::Common | CardRarity::Special | CardRarity::Bonus => {
            (Color::srgba(0.05, 0.08, 0.11, 0.95), 0.08, 8.0)
        }
    };
    let rim = rarity_color.with_alpha(match rarity {
        CardRarity::Mythic => 0.92,
        CardRarity::Rare => 0.85,
        CardRarity::Uncommon => 0.68,
        CardRarity::Common | CardRarity::Special | CardRarity::Bonus => 0.46,
    });
    let grad = LinearGradient::to_bottom(vec![
        ColorStop::percent(top_tint, 0.0),
        ColorStop::percent(Color::srgba(0.04, 0.065, 0.095, 0.96), 55.0),
        ColorStop::percent(Color::srgba(0.03, 0.05, 0.075, 0.98), 100.0),
    ])
    .in_color_space(InterpolationColorSpace::Srgba);
    let shadow = BoxShadow(vec![
        ShadowStyle {
            color: rarity_color.with_alpha(glow_alpha),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(0.0),
            spread_radius: px_fixed(0.5),
            blur_radius: px_fixed(blur),
        },
        ShadowStyle {
            color: Color::srgba(0.0, 0.0, 0.0, 0.58),
            x_offset: px_fixed(0.0),
            y_offset: px_fixed(6.0),
            spread_radius: px_fixed(-1.0),
            blur_radius: px_fixed(16.0),
        },
    ]);
    (rarity_name, rim, grad, shadow, ornate)
}

#[allow(clippy::too_many_arguments)]
fn spawn_candidate_face(
    commands: &mut Commands,
    kit: Kit,
    rarity: CardRarity,
    selected: bool,
    disabled: bool,
) -> (Entity, &'static str, Color) {
    let rarity_color = crate::lobby::decks::rarity_dot_color(rarity);
    let (rarity_name, border_rest, gradient, shadow, ornate) = rarity_visuals(rarity, selected);
    let face = commands
        .spawn((
            crate::shellkit::Role::Tile,
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(2.0)),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                padding: UiRect::all(kit.m.px(12.0)),
                row_gap: kit.m.px(7.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(gradient),
            BorderColor::all(border_rest),
            shadow,
            UiTransform::default(),
            Pickable::IGNORE,
        ))
        .id();
    if !disabled {
        commands.entity(face).insert((
            crate::ambience::Feel::tinting_to(Color::NONE, tokens::HOVER),
            crate::ambience::FeelBorder {
                rest: border_rest,
                hot: if selected {
                    LUMINOUS_GOLD
                } else {
                    crate::ambience::lighter(rarity_color, 0.25)
                },
                press: tokens::GOLD,
            },
        ));
        if ornate {
            commands.entity(face).insert(crate::shellkit::sheen::Sheen);
        }
    }
    (face, rarity_name, rarity_color)
}

fn candidate(
    commands: &mut Commands,
    shelf: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    take: u8,
    id: baylee_core::ids::CardIndex,
) {
    let lang = lang(kit);
    let selected = ui.choices.rewards.contains(&id);
    let rarity = baylee_cards::sides::rarity(id).unwrap_or(CardRarity::Common);
    let full_selection = !selected && take > 1 && ui.choices.rewards.len() >= usize::from(take);
    let disabled = ui.model.mutating() || ui.model.uncertain() || full_selection;
    let (face, rarity_name, rarity_color) =
        spawn_candidate_face(commands, kit, rarity, selected, disabled);

    let hover = card::hover(
        id,
        ui.model.publication().map(|cohort| &cohort.catalog),
        lang,
    );
    let image = crate::lobby::thumbnails::spawn_sized(
        commands,
        &hover,
        kit.m.px(122.0),
        kit.m.px(170.0),
        kit.m.px(8.0),
    );
    commands.entity(image).insert(surfaces::elevation_card());
    commands.entity(face).add_child(image);

    let name = surfaces::prose(commands, kit, &words::card(id, lang), false);
    commands.entity(name).insert((
        crate::hud::tf_bold(kit.fonts, kit.m.small * 1.04),
        TextLayout::justify(Justify::Center),
    ));
    commands.entity(face).add_child(name);

    let rarity_line = controls::label(commands, kit, rarity_name, kit.m.small * 0.88, rarity_color);
    commands.entity(face).add_child(rarity_line);

    if let Some(dup_gold) = reward_card_duplicate_gold(run, id) {
        spawn_duplicate_badge(commands, face, kit, lang, dup_gold);
    }

    let space = crate::lobby::parts::grow(commands);
    commands.entity(face).add_child(space);
    selected_crest(commands, face, kit, selected);

    let action = Action::Reward(id);
    let wrapper = controls::hit(commands, kit, face, Press::Echoes(action));
    commands
        .entity(wrapper)
        .insert((
            super::super::keys::stop(action, ui),
            crate::shellkit::focus::Current(selected),
            hover,
        ))
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_basis = px_fixed(0.0);
            node.flex_grow = 1.0;
            node.min_width = kit.m.px(148.0);
            node.max_width = kit.m.px(184.0);
            node.align_self = AlignSelf::Stretch;
            node.align_items = AlignItems::Stretch;
        });
    if disabled {
        commands.entity(wrapper).insert(controls::Disabled);
        if full_selection {
            controls::attach_tooltip(
                commands,
                wrapper,
                kit,
                Phrase::EchoesRewardMaxSelected.text(lang),
            );
        }
    }
    commands.entity(shelf).add_child(wrapper);
}

fn spawn_duplicate_badge(
    commands: &mut Commands,
    face: Entity,
    kit: Kit,
    lang: Lang,
    dup_gold: u16,
) {
    let pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(8.0), kit.m.px(2.0)),
                column_gap: kit.m.px(4.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.90)),
            BorderColor::all(Color::srgba(0.94, 0.76, 0.34, 0.75)),
            Pickable::IGNORE,
            ChildOf(face),
        ))
        .id();
    commands.spawn((
        Text::new(glyph::COINS.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.85),
        TextColor(tokens::GOLD),
        Pickable::IGNORE,
        ChildOf(pill),
    ));
    let text = controls::label(
        commands,
        kit,
        &Phrase::EchoesRewardDuplicateShort.fill(lang, &[&dup_gold.to_string()]),
        kit.m.small * 0.9,
        LUMINOUS_GOLD,
    );
    commands
        .entity(text)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small * 0.9));
    commands.entity(pill).add_child(text);
}

/// Reserved fixed-height crest slot: hidden when unselected (no noisy "Wählen" pill,
/// zero vertical layout shift), luminous golden crest when selected.
fn selected_crest(commands: &mut Commands, face: Entity, kit: Kit, selected: bool) {
    let lang = lang(kit);
    let slot = commands
        .spawn((
            Node {
                min_height: kit.m.px(25.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            ChildOf(face),
        ))
        .id();
    let pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(10.0), kit.m.px(3.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.18, 0.13, 0.03, 0.94)),
            BorderColor::all(Color::srgb(1.0, 0.86, 0.40)),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(1.0, 0.82, 0.28, 0.35),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(0.0),
                spread_radius: px_fixed(0.5),
                blur_radius: px_fixed(8.0),
            }]),
            if selected {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            Pickable::IGNORE,
            ChildOf(slot),
        ))
        .id();
    commands.spawn((
        Text::new(glyph::CHECK.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.88),
        TextColor(LUMINOUS_GOLD),
        Pickable::IGNORE,
        ChildOf(pill),
    ));
    let caption = controls::label(
        commands,
        kit,
        Phrase::EchoesRewardSelected.text(lang),
        kit.m.small,
        LUMINOUS_GOLD,
    );
    commands
        .entity(caption)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(pill).add_child(caption);
}
