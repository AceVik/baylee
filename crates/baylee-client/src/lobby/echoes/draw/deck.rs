#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
    cohort: &api::Cohort,
) {
    let lang = lang(kit);
    let card = panel(commands, root, kit);
    let count: u16 = ui
        .choices
        .deck
        .iter()
        .map(|entry| u16::from(entry.copies))
        .sum();
    heading(
        commands,
        card,
        kit,
        &Phrase::EchoesDeckCount.fill(lang, &[&count.to_string()]),
    );
    label(
        commands,
        card,
        kit,
        Phrase::EchoesDeckRules.text(lang),
        tokens::MUTED,
    );
    let top = row(commands, card, kit);
    control(
        commands,
        top,
        kit,
        ui,
        Phrase::EchoesSaveDeck.text(lang),
        Action::SaveDeck,
        !ui.choices.deck_unchanged(Some(run))
            && can(
                ui,
                &Command::Edit {
                    deck: ui.choices.deck.clone(),
                },
            ),
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
    for entry in &cohort.catalog.cards {
        let owned = run.cards.copies(entry.card);
        if !entry.basic_land && owned == 0 {
            continue;
        }
        let copies = ui
            .choices
            .deck
            .iter()
            .find(|stack| stack.card == entry.card)
            .map_or(0, |stack| stack.copies);
        let band = row(commands, card, kit);
        let supply = if entry.basic_land {
            "∞".into()
        } else {
            owned.to_string()
        };
        label(
            commands,
            band,
            kit,
            &format!("{copies} / {supply}  {}", words::card(entry.card, lang)),
            tokens::INK,
        );
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(band).add_child(spacer);
        control(
            commands,
            band,
            kit,
            ui,
            Phrase::EchoesRemove.text(lang),
            Action::Card(entry.card, false),
            run.phase == api::Phase::Shop && copies > 0 && !ui.model.uncertain(),
        );
        control(
            commands,
            band,
            kit,
            ui,
            Phrase::EchoesAdd.text(lang),
            Action::Card(entry.card, true),
            run.phase == api::Phase::Shop
                && count < domain::cards::DECK_MAX
                && (entry.basic_land || copies < owned)
                && !ui.model.uncertain(),
        );
    }
}
