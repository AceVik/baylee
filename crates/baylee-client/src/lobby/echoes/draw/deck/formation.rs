//! Run-owned Commander editor. The authority validates every zone together.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::echoes::draft::DeckZone;
use domain::cards::{Card, Stack};

const PAGE: usize = 10;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    summary(commands, root, kit, ui, run);
    if let Some(id) = ui.preview {
        let preview = panel(commands, root, kit);
        let top = row(commands, preview, kit);
        heading(commands, top, kit, &words::card(id, lang(kit)));
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(top).add_child(spacer);
        control(
            commands,
            top,
            kit,
            ui,
            Phrase::ShellClose.text(lang(kit)),
            Action::ClosePreview,
            true,
        );
        card::rules(commands, preview, kit, id);
    }
    collection(commands, root, kit, ui, run, cohort);
}

fn summary(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let Some(formation) = &ui.choices.formation else {
        return;
    };
    let lang = lang(kit);
    let summary = panel(commands, root, kit);
    let top = row(commands, summary, kit);
    heading(
        commands,
        top,
        kit,
        &Phrase::EchoesRunDeckCount.fill(
            lang,
            &[
                &count(&ui.choices.deck).to_string(),
                &count(&formation.sideboard).to_string(),
            ],
        ),
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::EchoesSaveDeck.text(lang),
        Action::SaveDeck,
        !ui.choices.deck_unchanged(Some(run)) && can(ui, &ui.choices.save_deck()),
    );
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::EchoesResetDeck.text(lang),
        Action::ResetDeck,
        !ui.choices.deck_unchanged(Some(run)),
    );
    label(
        commands,
        summary,
        kit,
        Phrase::EchoesRunDeckRules.text(lang),
        tokens::MUTED,
    );
    let commander = row(commands, summary, kit);
    label(
        commands,
        commander,
        kit,
        Phrase::EchoesRunCommander.text(lang),
        tokens::GOLD,
    );
    control(
        commands,
        commander,
        kit,
        ui,
        &words::card(formation.commander, lang),
        Action::InspectCard(formation.commander),
        true,
    );
    if !ui.choices.deck_unchanged(Some(run)) && !can(ui, &ui.choices.save_deck()) {
        label(
            commands,
            summary,
            kit,
            Phrase::EchoesRunDeckInvalid.text(lang),
            tokens::GOLD,
        );
    }
}

fn collection(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = lang(kit);
    let pool = panel(commands, root, kit);
    let tabs = row(commands, pool, kit);
    for (zone, phrase) in [
        (DeckZone::Main, Phrase::EchoesRunLibrary),
        (DeckZone::Sideboard, Phrase::EchoesRunSideboard),
    ] {
        control(
            commands,
            tabs,
            kit,
            ui,
            phrase.text(lang),
            Action::DeckZone(zone),
            true,
        );
    }
    let entries: Vec<_> = cohort
        .catalog
        .cards
        .iter()
        .filter(|entry| entry.basic_land || run.cards.copies(entry.card) > 0)
        .collect();
    let pages = entries.len().div_ceil(PAGE).max(1);
    let page = ui.deck.page.min(pages - 1);
    for entry in entries.into_iter().skip(page * PAGE).take(PAGE) {
        entry_row(commands, pool, kit, ui, run, cohort, *entry);
    }
    let footer = row(commands, pool, kit);
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::EchoesRunPrevious.text(lang),
        Action::DeckPage(page.saturating_sub(1)),
        page > 0,
    );
    label(
        commands,
        footer,
        kit,
        &Phrase::EchoesRunPage.fill(lang, &[&(page + 1).to_string(), &pages.to_string()]),
        tokens::MUTED,
    );
    control(
        commands,
        footer,
        kit,
        ui,
        Phrase::EchoesRunNext.text(lang),
        Action::DeckPage(page + 1),
        page + 1 < pages,
    );
}

fn entry_row(
    commands: &mut Commands,
    pool: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
    entry: Card,
) {
    let Some(formation) = &ui.choices.formation else {
        return;
    };
    let lang = lang(kit);
    let rows = if ui.deck.zone == DeckZone::Main {
        &ui.choices.deck
    } else {
        &formation.sideboard
    };
    let band = row(commands, pool, kit);
    let copies = rows
        .iter()
        .find(|row| row.card == entry.card)
        .map_or(0, |row| row.copies);
    label(
        commands,
        band,
        kit,
        &format!("{copies}×"),
        if copies == 0 {
            tokens::MUTED
        } else {
            tokens::ACCENT
        },
    );
    control(
        commands,
        band,
        kit,
        ui,
        &words::card(entry.card, lang),
        Action::InspectCard(entry.card),
        true,
    );
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(band).add_child(spacer);
    if entry.run.is_some_and(|rules| rules.commander) && run.cards.copies(entry.card) > 0 {
        control(
            commands,
            band,
            kit,
            ui,
            Phrase::EchoesRunCommander.text(lang),
            Action::Commander(entry.card),
            run.phase == api::Phase::Shop && !ui.model.uncertain(),
        );
    }
    for (add, phrase) in [(false, Phrase::EchoesRemove), (true, Phrase::EchoesAdd)] {
        control(
            commands,
            band,
            kit,
            ui,
            phrase.text(lang),
            Action::ZoneCard(entry.card, ui.deck.zone, add),
            !ui.model.uncertain()
                && ui
                    .choices
                    .can_card_in(run, cohort, entry.card, ui.deck.zone, add),
        );
    }
}

fn count(rows: &[Stack]) -> u16 {
    rows.iter().map(|row| u16::from(row.copies)).sum()
}
