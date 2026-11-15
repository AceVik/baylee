#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::catalog::{ACTIVE_SLOTS, RESERVE_SLOTS};

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    _cohort: &api::Cohort,
) {
    attributes(commands, root, kit, run);
    inventory(commands, root, kit, ui, run);
}

pub(super) fn attributes(commands: &mut Commands, root: Entity, kit: Kit, run: &api::Run) {
    let lang = lang(kit);
    let stats = panel(commands, root, kit);
    heading(commands, stats, kit, Phrase::EchoesEquipment.text(lang));
    if let Ok(stats_value) = domain::v2::catalog::attributes(&run.economy.inventory.active()) {
        let band = row(commands, stats, kit);
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
            presentation::metric(commands, band, kit, icon, phrase.text(lang), &value);
        }
    }
    label(
        commands,
        stats,
        kit,
        &Phrase::EchoesCharges.fill(
            lang,
            &[
                &run.economy.potions[0].to_string(),
                &run.economy.potions[1].to_string(),
            ],
        ),
        tokens::MUTED,
    );
}

mod inventory;
mod placement;
pub(super) use inventory::detail;
use inventory::inventory;
pub(super) use placement::{ingredients, slots};
