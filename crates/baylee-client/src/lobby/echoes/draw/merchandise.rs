//! Shared item surfaces: one semantic hit target, with the picture inside it.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn image(commands: &mut Commands, root: Entity, kit: Kit, goods: Goods, size: f32) {
    let image = commands
        .spawn((
            crate::itemart::Portrait(goods),
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(image);
}

pub(super) fn surface(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    action: Action,
) -> Entity {
    let selected = words::selected(action, ui);
    let ground = if selected {
        tokens::PRIMARY
    } else {
        tokens::CONTROL
    };
    let face = commands
        .spawn((
            crate::shellkit::role::Role::Button,
            Node {
                width: percent(100),
                min_width: px_fixed(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: kit.m.px(4.0),
                padding: UiRect::all(kit.m.px(6.0)),
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(kit.m.px(8.0)),
                ..default()
            },
            BackgroundColor(ground),
            BorderColor::all(if selected {
                tokens::ACCENT
            } else {
                tokens::BORDER
            }),
            crate::ambience::Feel::new(ground),
            Pickable::IGNORE,
        ))
        .id();
    let hit = controls::hit(commands, kit, face, Press::Echoes(action));
    commands
        .entity(hit)
        .insert((
            super::super::keys::stop(action, ui),
            crate::shellkit::focus::Current(selected),
        ))
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = percent(100);
            node.height = percent(100);
            node.min_width = px_fixed(0.0);
            node.align_self = AlignSelf::Stretch;
            node.align_items = AlignItems::Stretch;
        });
    let available = available(ui, action);
    if !available {
        commands.entity(hit).insert(controls::Disabled);
    }
    let description = if available {
        description(ui, action, lang(kit))
    } else {
        Phrase::EchoesUnavailable.text(lang(kit)).to_owned()
    };
    controls::attach_tooltip(commands, hit, kit, &description);
    commands.entity(root).add_child(hit);
    face
}

fn description(ui: &Ui, action: Action, lang: Lang) -> String {
    let Some(run) = ui.model.run() else {
        return String::new();
    };
    match action {
        Action::Place(slot) => format!(
            "{} · {}",
            words::slot(slot, lang),
            run.economy
                .inventory
                .relics
                .iter()
                .find(|item| item.slot == slot)
                .map_or_else(
                    || Phrase::EchoesSlotEmpty.text(lang).to_owned(),
                    |item| words::goods(Goods::Relic(item.definition), lang)
                )
        ),
        Action::Select(id) => run
            .economy
            .inventory
            .relics
            .iter()
            .find(|item| item.id == id)
            .map_or_else(String::new, |item| {
                format!(
                    "{} · {}",
                    words::goods(Goods::Relic(item.definition), lang),
                    words::slot(item.slot, lang)
                )
            }),
        // Full rules live in the scrollable inspector; compact hover labels must
        // not grow an unwrapped effect paragraph beyond the stock or viewport.
        Action::Offer(id) => run
            .shop
            .offers
            .iter()
            .find(|offer| offer.id == id)
            .map_or_else(String::new, |offer| {
                format!(
                    "{}{}",
                    words::goods(offer.goods, lang),
                    if offer.pinned {
                        format!("\n{}", Phrase::EchoesUnpin.text(lang))
                    } else {
                        String::new()
                    }
                )
            }),
        _ => String::new(),
    }
}

fn available(ui: &Ui, action: Action) -> bool {
    !ui.model.busy()
        && !ui.model.uncertain()
        && ui.confirmation.is_none()
        && ui.model.run().is_some_and(|run| {
            run.phase == api::Phase::Shop
                && match action {
                    Action::Offer(id) => run
                        .shop
                        .offers
                        .iter()
                        .any(|offer| offer.id == id && !offer.sold),
                    Action::Select(id) => run
                        .economy
                        .inventory
                        .relics
                        .iter()
                        .any(|item| item.id == id),
                    _ => true,
                }
        })
}

pub(super) fn grid(commands: &mut Commands, root: Entity, kit: Kit, columns: u16) -> Entity {
    let grid = commands
        .spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                grid_template_columns: vec![RepeatedGridTrack::flex(columns, 1.0)],
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(6.0),
                flex_shrink: 0.0,
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(grid);
    grid
}

pub(super) fn caption(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    text: &str,
    color: Color,
) -> Entity {
    let text = commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.small),
            TextColor(color),
            TextLayout {
                justify: Justify::Center,
                linebreak: LineBreak::WordOrCharacter,
            },
            Node {
                width: percent(100),
                max_width: percent(100),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(text);
    text
}
