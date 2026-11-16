//! A closed expedition remains a memento; it is never an actionable shop.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let summary = panel(commands, root, kit);
    presentation::feature(
        commands,
        summary,
        kit,
        crate::challenge_art::Scene::Tower,
        words::phase(run.phase).text(lang(kit)),
    );
    landing::progress(commands, summary, kit, ui, run);
    label(
        commands,
        summary,
        kit,
        Phrase::EchoesRunFinished.text(lang(kit)),
        tokens::MUTED,
    );
    let items = row(commands, summary, kit);
    for item in &run.economy.inventory.relics {
        if !matches!(item.slot, Slot::Active(_)) {
            continue;
        }
        let name = words::goods(Goods::Relic(item.definition), lang(kit));
        let description = words::run_effect(item.definition, lang(kit));
        let item = crate::towerui::button(
            commands,
            kit,
            crate::towerui::Item {
                goods: Goods::Relic(item.definition),
                name: &name,
                description: &description,
                caption: None,
                size: 76.0,
                emphasis: crate::itemart::Emphasis::None,
            },
            Live::Yes,
            (),
        );
        commands.entity(items).add_child(item);
    }
    control(
        commands,
        summary,
        kit,
        ui,
        Phrase::EchoesNewRun.text(lang(kit)),
        Action::Overview,
        true,
    );
}
