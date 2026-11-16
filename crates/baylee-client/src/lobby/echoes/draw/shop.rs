#[allow(clippy::wildcard_imports)]
use super::*;

mod detail;
mod dock;
mod options;
mod stock;
pub(super) use options::toolbar;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
    scrolled: &Scrolled,
) {
    let workspace = row(commands, root, kit);
    commands
        .entity(workspace)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.align_items = AlignItems::Start;
            node.flex_wrap = FlexWrap::Wrap;
        });
    let market = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_basis: kit.m.px(480.0),
                flex_grow: 2.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(10.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(workspace).add_child(market);
    stock::draw(commands, market, kit, ui, run, cohort);
    dock::draw(commands, market, kit, ui, run);
    let inspector = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_basis: kit.m.px(280.0),
                flex_grow: 1.0,
                min_width: px_fixed(0.0),
                row_gap: kit.m.px(10.0),
                max_height: Val::Vh(55.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            Pickable::default(),
            Scrollable(List::EchoesInspector),
            ScrollPosition(Vec2::new(0.0, scrolled.get(List::EchoesInspector))),
        ))
        .id();
    commands.entity(workspace).add_child(inspector);
    if ui.choices.item.is_some() {
        equipment::detail(commands, inspector, kit, ui, run);
    } else if ui.choices.offer.is_some() {
        detail::detail(commands, inspector, kit, ui, run, cohort);
    } else {
        equipment::attributes(commands, inspector, kit, run);
        let hint = panel(commands, inspector, kit);
        label(
            commands,
            hint,
            kit,
            Phrase::EchoesShopRules.text(lang(kit)),
            tokens::MUTED,
        );
        label(
            commands,
            hint,
            kit,
            Phrase::EchoesSelectEquipment.text(lang(kit)),
            tokens::MUTED,
        );
    }
    options::options(commands, root, kit, ui, run);
}
