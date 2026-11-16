//! One run workspace: equipped items, shelf, then the paged inventory.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::{itemart::Emphasis, towerui};
use baylee_client_core::echoes::workshop;

mod bag;
mod equipment;
mod shelf;
#[cfg(test)]
mod tests;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let workspace = row(commands, root, kit);
    commands
        .entity(workspace)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.align_items = AlignItems::Stretch;
            node.flex_wrap = FlexWrap::Wrap;
            node.flex_grow = 1.0;
        });
    let equipment = towerui::frame(commands, workspace, kit);
    commands
        .entity(equipment)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_basis = kit.m.px(260.0);
            node.flex_grow = 0.0;
        });
    equipment::draw(commands, equipment, kit, ui, run);
    let market = towerui::frame(commands, workspace, kit);
    commands
        .entity(market)
        .entry::<Node>()
        .and_modify(move |mut node| {
            node.flex_basis = kit.m.px(600.0);
            node.flex_grow = 1.0;
        });
    shelf::draw(commands, market, kit, ui, run, cohort);
    bag::draw(commands, root, kit, ui, run);
}

fn stats(commands: &mut Commands, root: Entity, kit: Kit, run: &api::Run) {
    let stats = row(commands, root, kit);
    commands
        .entity(stats)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.justify_content = JustifyContent::Center;
        });
    if let Ok(stats_value) = domain::v2::catalog::run_attributes(&run.economy.inventory.active()) {
        for (icon, phrase, value) in [
            (
                glyph::HEART,
                Phrase::EchoesStartingLife,
                stats_value.life.to_string(),
            ),
            (
                glyph::HAND,
                Phrase::EchoesOpeningCards,
                stats_value.cards.to_string(),
            ),
            (
                glyph::LIBRARY,
                Phrase::EchoesHandLimit,
                stats_value.maximum_hand.to_string(),
            ),
            (
                glyph::REFRESH,
                Phrase::EchoesMulligans,
                stats_value.free_mulligans.to_string(),
            ),
        ] {
            presentation::metric(commands, stats, kit, icon, phrase.text(lang(kit)), &value);
        }
    }
}

fn help_heading(commands: &mut Commands, root: Entity, kit: Kit, title: Phrase, help: Phrase) {
    let band = row(commands, root, kit);
    commands
        .entity(band)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.width = Val::Auto;
            node.flex_wrap = FlexWrap::NoWrap;
        });
    let title = controls::label(
        commands,
        kit,
        title.text(lang(kit)),
        kit.m.text,
        tokens::INK,
    );
    commands
        .entity(title)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.text));
    commands.entity(band).add_child(title);
    let info = commands
        .spawn((
            Text::new(glyph::INFO.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::default(),
            ChildOf(band),
        ))
        .id();
    controls::attach_tooltip(commands, info, kit, help.text(lang(kit)));
}

fn grid(commands: &mut Commands, parent: Entity, kit: Kit, columns: u16) -> Entity {
    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: percent(100),
                column_gap: kit.m.px(6.0),
                row_gap: kit.m.px(8.0),
                grid_template_columns: vec![RepeatedGridTrack::flex(columns, 1.0)],
                ..default()
            },
            ChildOf(parent),
            Pickable::IGNORE,
        ))
        .id()
}

#[allow(clippy::too_many_arguments)]
fn object(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    goods: Goods,
    caption: Option<&str>,
    size: f32,
    emphasis: Emphasis,
    action: Action,
    available: bool,
) -> Entity {
    let description = match goods {
        Goods::Relic(definition) => words::run_effect(definition, lang(kit)),
        _ => words::effect(goods, lang(kit)),
    };
    let name = words::goods(goods, lang(kit));
    let live =
        if available && !ui.model.busy() && !ui.model.uncertain() && ui.confirmation.is_none() {
            Live::Yes
        } else {
            Live::No(Phrase::EchoesUnavailable.text(lang(kit)))
        };
    let button = towerui::button(
        commands,
        kit,
        towerui::Item {
            goods,
            name: &name,
            description: &description,
            caption,
            size,
            emphasis,
        },
        live,
        Press::Echoes(action),
    );
    commands
        .entity(button)
        .insert(super::super::keys::stop(action, ui));
    commands.entity(root).add_child(button);
    button
}
