#[allow(clippy::wildcard_imports)]
use super::*;
use bevy::ui::{ColorStop, InterpolationColorSpace, LinearGradient};

const OFFER_ITEM_SIZE: f32 = 80.0;
const OFFER_STAND_HEIGHT: f32 = 124.0;
const OFFER_LEDGE_HEIGHT: f32 = 34.0;
const PRICE_NUMERAL_GOLD: Color = Color::srgb(1.0, 0.87, 0.44);
const PRICE_ICON_GOLD: Color = Color::srgb(0.98, 0.80, 0.32);

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = lang(kit);
    let top = row(commands, root, kit);
    landing::market_progress(commands, top, kit, ui);
    let info = commands
        .spawn((
            Text::new(glyph::INFO.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::default(),
            ChildOf(top),
        ))
        .id();
    controls::attach_tooltip(commands, info, kit, Phrase::EchoesRunShelfHint.text(lang));
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);

    let frozen = run.shop.frozen;
    icon_control(
        commands,
        top,
        kit,
        ui,
        words::GLYPH_SNOWFLAKE,
        None,
        if frozen {
            IconTone::Frost
        } else {
            IconTone::FrostIdle
        },
        if frozen {
            Phrase::EchoesThawShelfTip
        } else {
            Phrase::EchoesFreezeShelfTip
        }
        .text(lang),
        Action::Freeze,
        can(ui, &Command::Freeze { frozen: !frozen }),
        None,
    );
    actions(commands, top, kit, ui, run);

    let market = spawn_market_canvas(commands, root, kit, run, frozen);
    for offer in &run.shop.offers {
        offer_tile(commands, market, kit, ui, run, cohort, offer);
    }
    footer(commands, root, kit, ui, run, cohort);
}

fn market_gradient(frozen: bool) -> LinearGradient {
    if frozen {
        LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.036, 0.082, 0.130, 0.96), 0.0),
            ColorStop::percent(Color::srgba(0.052, 0.116, 0.172, 0.95), 48.0),
            ColorStop::percent(Color::srgba(0.024, 0.054, 0.092, 0.98), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba)
    } else {
        LinearGradient::to_bottom(vec![
            ColorStop::percent(Color::srgba(0.064, 0.043, 0.028, 0.96), 0.0),
            ColorStop::percent(Color::srgba(0.084, 0.056, 0.036, 0.95), 46.0),
            ColorStop::percent(Color::srgba(0.032, 0.022, 0.017, 0.98), 100.0),
        ])
        .in_color_space(InterpolationColorSpace::Srgba)
    }
}

fn spawn_market_canvas(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    run: &api::Run,
    frozen: bool,
) -> Entity {
    let lang = lang(kit);
    let gradient = market_gradient(frozen);
    let market = commands
        .spawn((
            Node {
                display: Display::Grid,
                align_items: AlignItems::End,
                align_content: AlignContent::SpaceEvenly,
                justify_content: JustifyContent::SpaceEvenly,
                grid_template_columns: vec![RepeatedGridTrack::flex(
                    if run.shop.offers.len() > 8 { 6 } else { 4 },
                    1.0,
                )],
                width: percent(100),
                min_height: kit.m.px(298.0),
                flex_grow: 1.0,
                row_gap: kit.m.px(16.0),
                column_gap: kit.m.px(16.0),
                padding: UiRect {
                    left: kit.m.px(28.0),
                    right: kit.m.px(28.0),
                    top: kit.m.px(22.0),
                    bottom: kit.m.px(18.0),
                },
                border: UiRect::all(px_fixed(1.5)),
                border_radius: BorderRadius::all(kit.m.px(12.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BackgroundGradient::from(gradient),
            BorderColor::all(if frozen {
                Color::srgba(0.56, 0.90, 1.0, 0.84)
            } else {
                Color::srgba(0.82, 0.64, 0.30, 0.32)
            }),
            if frozen {
                BoxShadow(vec![ShadowStyle {
                    color: Color::srgba(0.36, 0.82, 1.0, 0.24),
                    x_offset: px_fixed(0.0),
                    y_offset: px_fixed(0.0),
                    spread_radius: px_fixed(1.0),
                    blur_radius: px_fixed(18.0),
                }])
            } else {
                surfaces::elevation_subcard()
            },
            Place::new(run, Anchor::Shelf),
            ChildOf(root),
            Pickable::IGNORE,
        ))
        .id();

    // Subtle inner vignette when warm; GPU-clocked crystalline rime-frost overlay
    // plus floating frost badge when frozen — all absolute so toggling Freeze never
    // shifts item or shelf plank geometry by a single pixel.
    if frozen {
        towerui::shelf_frost(commands, market);
        spawn_frost_badge(commands, market, kit, lang);
    } else {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: kit.m.px(4.0),
                right: kit.m.px(4.0),
                top: kit.m.px(4.0),
                bottom: kit.m.px(4.0),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(9.0)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.98, 0.78, 0.40, 0.12)),
            Pickable::IGNORE,
            ChildOf(market),
        ));
    }

    market
}

fn spawn_frost_badge(commands: &mut Commands, market: Entity, kit: Kit, lang: Lang) {
    let badge = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: kit.m.px(8.0),
                right: kit.m.px(18.0),
                padding: UiRect::axes(kit.m.px(9.0), kit.m.px(3.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.11, 0.18, 0.92)),
            BorderColor::all(Color::srgba(0.58, 0.90, 1.0, 0.85)),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.65),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(2.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(6.0),
            }]),
            ZIndex(3),
            Pickable::IGNORE,
            ChildOf(market),
        ))
        .id();
    let frost_ink = Color::srgb(0.78, 0.96, 1.0);
    commands.spawn((
        Text::new(words::GLYPH_SNOWFLAKE.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.9),
        TextColor(frost_ink),
        Pickable::IGNORE,
        ChildOf(badge),
    ));
    let text = controls::label(
        commands,
        kit,
        Phrase::EchoesShelfFrozen.text(lang),
        kit.m.small,
        frost_ink,
    );
    commands
        .entity(text)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.small));
    commands.entity(badge).add_child(text);
}

#[allow(clippy::too_many_arguments)] // Render context plus the authoritative offer.
fn offer_tile(
    commands: &mut Commands,
    market: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
    offer: &domain::v2::shop::Offer,
) {
    let lang = lang(kit);
    let stand = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::End,
                height: kit.m.px(OFFER_STAND_HEIGHT),
                min_width: px(0),
                row_gap: kit.m.px(4.0),
                ..default()
            },
            ChildOf(market),
            Pickable::IGNORE,
        ))
        .id();
    commands
        .entity(stand)
        .insert(Place::new(run, Anchor::Offer(offer.id)));

    let offer_price = offer.goods.price(&cohort.catalog).unwrap_or(0);
    let can_buy = can(
        ui,
        &Command::Buy {
            offer: offer.id,
            delivery: domain::v2::shop::Delivery::Inventory,
        },
    );

    if offer.sold {
        sleek_empty(commands, stand, kit, OFFER_ITEM_SIZE, glyph::CHECK);
    } else {
        let hint = matches!(offer.goods, Goods::Relic(_))
            .then(|| Phrase::EchoesRunEquipmentHint.text(lang));
        let unavailable_reason = (!can_buy && offer_price > run.economy.gold)
            .then(|| Phrase::EchoesNotEnoughGold.text(lang));
        let item = object(
            commands,
            stand,
            kit,
            ui,
            offer.goods,
            None,
            hint,
            OFFER_ITEM_SIZE,
            offer_emphasis(run, offer.goods),
            false,
            Action::Purchase(offer.id),
            can_buy,
            unavailable_reason,
        );
        commands
            .entity(item)
            .insert(super::super::super::gestures::Sale::new(run, offer.id));
    }

    // Every object has its own wooden ledge; sold slots retain identical geometry.
    let ledge = commands
        .spawn((
            Node {
                width: percent(100),
                height: kit.m.px(OFFER_LEDGE_HEIGHT),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ChildOf(stand),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        towerui::Decor::Shelf,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            ..default()
        },
        ChildOf(ledge),
        Pickable::IGNORE,
    ));

    let price_badge = spawn_price_pill(
        commands,
        ledge,
        kit,
        offer_price,
        offer_price <= run.economy.gold,
    );
    if offer.sold {
        commands.entity(price_badge).insert(Visibility::Hidden);
    }
}

fn spawn_price_pill(
    commands: &mut Commands,
    ledge: Entity,
    kit: Kit,
    amount: u16,
    affordable: bool,
) -> Entity {
    let pill = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(9.0), kit.m.px(2.5)),
                column_gap: kit.m.px(4.5),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                flex_shrink: 0.0,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.92)),
            BorderColor::all(if affordable {
                Color::srgba(0.94, 0.76, 0.34, 0.84)
            } else {
                Color::srgba(0.56, 0.46, 0.32, 0.50)
            }),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgba(0.0, 0.0, 0.0, 0.74),
                x_offset: px_fixed(0.0),
                y_offset: px_fixed(2.0),
                spread_radius: px_fixed(0.0),
                blur_radius: px_fixed(6.0),
            }]),
            Pickable::IGNORE,
            ChildOf(ledge),
        ))
        .id();
    let coin_ink = if affordable {
        PRICE_ICON_GOLD
    } else {
        Color::srgb(0.68, 0.58, 0.42)
    };
    let numeral_ink = if affordable {
        PRICE_NUMERAL_GOLD
    } else {
        Color::srgb(0.76, 0.68, 0.54)
    };
    commands.spawn((
        Text::new(glyph::COINS.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.92),
        TextColor(coin_ink),
        Pickable::IGNORE,
        ChildOf(pill),
    ));
    let numeral = commands
        .spawn((
            Text::new(amount.to_string()),
            crate::hud::tf_bold(kit.fonts, kit.m.small * 1.04),
            TextColor(numeral_ink),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
            ChildOf(pill),
        ))
        .id();
    let _ = numeral;
    pill
}

fn footer(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = lang(kit);
    let footer = row(commands, root, kit);
    let purse = commands
        .spawn((
            Node {
                padding: UiRect::axes(kit.m.px(12.0), kit.m.px(5.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.90)),
            BorderColor::all(Color::srgba(0.94, 0.76, 0.34, 0.72)),
            surfaces::elevation_subcard(),
            Pickable::default(),
            Place::new(run, Anchor::Wallet),
            ChildOf(footer),
        ))
        .id();
    let _wallet = towerui::coin(commands, purse, kit, u32::from(run.economy.gold));
    controls::attach_tooltip(commands, purse, kit, Phrase::EchoesGold.text(lang));

    spare::toggle_button(commands, footer, kit, ui, run, cohort);

    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(footer).add_child(spacer);
    reroll_button(commands, footer, kit, ui, run);
}

fn reroll_copy(run: &api::Run, lang: Lang) -> (String, String, Option<&'static str>) {
    if let Some(price) = run.shop.reroll_price() {
        let tip = Phrase::EchoesRefreshPrice.fill(lang, &[&price.to_string()]);
        let reason = if run.shop.frozen {
            Some(Phrase::EchoesShelfFrozen.text(lang))
        } else if price > run.economy.gold {
            Some(Phrase::EchoesNotEnoughGold.text(lang))
        } else {
            None
        };
        (price.to_string(), tip, reason)
    } else {
        let reason = if run.shop.frozen {
            Phrase::EchoesShelfFrozen.text(lang)
        } else {
            Phrase::EchoesRefreshSpent.text(lang)
        };
        ("—".to_owned(), String::new(), Some(reason))
    }
}

fn reroll_button(commands: &mut Commands, footer: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let has_price = run.shop.reroll_price().is_some();
    let available = can(ui, &Command::Refresh {})
        && !ui.model.mutating()
        && !ui.model.uncertain()
        && ui.confirmation.is_none();
    let (badge, tooltip, disabled_reason) = reroll_copy(run, lang);
    let (rest_bg, rest_rim, rest_ink) = IconTone::Gold.colors();
    let ground = if available {
        rest_bg
    } else {
        tokens::CONTROL.with_alpha(0.55)
    };
    let rim = if available { rest_rim } else { tokens::BORDER };
    let ink = if available {
        rest_ink
    } else {
        tokens::DISABLED
    };

    let face = commands
        .spawn((
            crate::shellkit::Role::Button,
            Node {
                min_height: kit.m.px(28.0),
                padding: UiRect::axes(kit.m.px(10.0), px_fixed(0.0)),
                column_gap: kit.m.px(5.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_PILL)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(rim),
            Pickable::IGNORE,
        ))
        .id();
    commands.spawn((
        Text::new(words::GLYPH_DICE.to_string()),
        crate::hud::icon_tf(kit.fonts, kit.m.small * 0.94),
        TextColor(if available { tokens::INK } else { ink }),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    if has_price {
        commands.spawn((
            Text::new(glyph::COINS.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small * 0.88),
            TextColor(if available { PRICE_ICON_GOLD } else { ink }),
            Pickable::IGNORE,
            ChildOf(face),
        ));
    }
    commands.spawn((
        Text::new(badge),
        crate::hud::tf_bold(kit.fonts, kit.m.small),
        TextColor(ink),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
        ChildOf(face),
    ));
    if available {
        commands.entity(face).insert((
            surfaces::elevation_control(),
            crate::ambience::Feel {
                lift: 0.0,
                ..crate::ambience::Feel::new(ground)
            },
            crate::ambience::FeelBorder {
                rest: rim,
                hot: crate::ambience::lighter(rim, 0.30),
                press: tokens::GOLD,
            },
            crate::ambience::FeelShadow,
        ));
    } else {
        commands
            .entity(face)
            .insert(surfaces::elevation_control_disabled());
    }
    let wrapper = controls::hit(commands, kit, face, Press::Echoes(Action::Reroll));
    commands.entity(wrapper).insert((
        super::super::super::keys::stop(Action::Reroll, ui),
        crate::shellkit::focus::Current(false),
    ));
    if !available {
        commands.entity(wrapper).insert(controls::Disabled);
        let why = disabled_reason.unwrap_or_else(|| Phrase::EchoesUnavailable.text(lang));
        let full_tip = if tooltip.is_empty() {
            why.to_owned()
        } else {
            format!("{tooltip} — {why}")
        };
        controls::attach_tooltip(commands, wrapper, kit, &full_tip);
    } else if !tooltip.is_empty() {
        controls::attach_tooltip(commands, wrapper, kit, &tooltip);
    }
    commands.entity(footer).add_child(wrapper);
}

fn offer_emphasis(run: &api::Run, goods: Goods) -> Emphasis {
    let base = match goods {
        Goods::Relic(definition) => match workshop::affinity(&run.economy.inventory, definition) {
            (_, Some(Rarity::Uncommon)) => Emphasis::Uncommon,
            (_, Some(Rarity::Rare)) => Emphasis::Rare,
            (_, Some(Rarity::MythicRare)) => Emphasis::Mythic,
            (0, _) => Emphasis::None,
            _ => Emphasis::Owned,
        },
        _ => Emphasis::None,
    };
    base.frozen_if(run.shop.frozen)
}

fn actions(commands: &mut Commands, top: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let cta = Cta { top, kit, ui };
    if run.phase == api::Phase::Shop {
        if ui.model.hosts().is_empty() {
            cta.spawn(
                commands,
                words::GLYPH_DUEL,
                Phrase::EchoesPlay.text(lang),
                Action::Resume,
                false,
                Some(Phrase::EchoesNoHost.text(lang)),
            );
        }
        let deck_clean = ui.choices.deck_unchanged(Some(run));
        for host in ui.model.hosts() {
            let title = if ui.model.hosts().len() == 1 {
                Phrase::EchoesPlay.text(lang).to_owned()
            } else {
                Phrase::EchoesPlayAt.fill(lang, &[&host.name])
            };
            cta.spawn(
                commands,
                words::GLYPH_DUEL,
                &title,
                Action::Play(host.id),
                deck_clean,
                (!deck_clean).then(|| Phrase::EchoesUnsaved.text(lang)),
            );
        }
    } else if matches!(run.phase, api::Phase::Reserved | api::Phase::Playing) {
        cta.spawn(
            commands,
            glyph::ARROW_RIGHT,
            Phrase::TowerResume.text(lang),
            Action::Resume,
            true,
            None,
        );
    } else if matches!(run.phase, api::Phase::Verifying | api::Phase::Quarantined) {
        held(commands, &cta, run);
    }
}

/// One gold call to action in the shelf's top row.
struct Cta<'a, 'k> {
    top: Entity,
    kit: Kit<'k>,
    ui: &'a Ui,
}

impl Cta<'_, '_> {
    fn spawn(
        &self,
        commands: &mut Commands,
        icon: char,
        title: &str,
        action: Action,
        enabled: bool,
        reason: Option<&str>,
    ) {
        let ui = self.ui;
        let blocked = ui.model.mutating() || ui.model.uncertain() || ui.confirmation.is_some();
        let btn = controls::button_compact(
            commands,
            self.kit,
            Some(icon),
            title,
            Weight::Gold,
            if enabled && !blocked {
                Live::Yes
            } else {
                Live::No(reason.unwrap_or_else(|| Phrase::EchoesUnavailable.text(lang(self.kit))))
            },
            Press::Echoes(action),
        );
        commands
            .entity(btn)
            .insert(super::super::super::keys::stop(action, ui));
        commands.entity(self.top).add_child(btn);
    }
}

/// No shop while the duel is checked or held for recovery: say so, and offer
/// the one thing the player may do about a held duel.
fn held(commands: &mut Commands, cta: &Cta<'_, '_>, run: &api::Run) {
    let lang = lang(cta.kit);
    let quarantined = run.phase == api::Phase::Quarantined;
    let text = if quarantined {
        format!(
            "{} {}",
            words::phase(run.phase).text(lang),
            Phrase::EchoesQuarantinedChoice.text(lang)
        )
    } else {
        words::phase(run.phase).text(lang).to_owned()
    };
    let status = controls::label(commands, cta.kit, &text, cta.kit.m.text, tokens::MUTED);
    commands.entity(cta.top).add_child(status);
    if quarantined {
        cta.spawn(
            commands,
            words::GLYPH_FLAG,
            Phrase::TowerRetire.text(lang),
            Action::Abandon,
            can(cta.ui, &Command::Abandon {}),
            None,
        );
    }
}
