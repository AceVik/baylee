#[allow(clippy::wildcard_imports)]
use super::*;
mod progress;
pub(super) use progress::{market_progress, progress};

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    saved(commands, body, kit, ui);
    enrollment(commands, body, kit, ui);
}

fn saved(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let runs = panel(commands, body, kit);
    heading(commands, runs, kit, Phrase::EchoesRuns.text(lang));
    if ui.model.runs().is_empty() {
        label(
            commands,
            runs,
            kit,
            Phrase::EchoesNoRuns.text(lang),
            tokens::MUTED,
        );
    }
    for run in ui.model.runs() {
        let text = format!(
            "{} · {} · {}",
            words::created(run.id, lang),
            words::phase(run.phase).text(lang),
            Phrase::EchoesStage.fill(lang, &[&run.shop.stage.to_string()])
        );
        control(
            commands,
            runs,
            kit,
            ui,
            &text,
            Action::Read(Read::Run(run.id)),
            !ui.model.uncertain() && ui.choices.deck_unchanged(ui.model.run()),
        );
    }
    let paging = row(commands, runs, kit);
    control(
        commands,
        paging,
        kit,
        ui,
        Phrase::Refresh.text(lang),
        Action::Read(Read::Runs(None)),
        true,
    );
    if let Some(last) = ui
        .model
        .runs()
        .last()
        .filter(|_| ui.model.runs().len() == api::PAGE_SIZE)
    {
        control(
            commands,
            paging,
            kit,
            ui,
            Phrase::ChallengeOlder.text(lang),
            Action::Read(Read::Runs(Some(last.id))),
            true,
        );
    }
}

fn enrollment(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let band = row(commands, body, kit);
    for cohort in ui.model.cohorts() {
        let card = tile(commands, band, kit);
        presentation::feature(
            commands,
            card,
            kit,
            crate::challenge_art::Scene::Tower,
            Phrase::EchoesNewRun.text(lang),
        );
        label(
            commands,
            card,
            kit,
            &Phrase::EchoesPublished.fill(lang, &[&words::created(cohort.id, lang)]),
            tokens::MUTED,
        );
        label(
            commands,
            card,
            kit,
            Phrase::EchoesStarterLead.text(lang),
            tokens::MUTED,
        );
        let choices = row(commands, card, kit);
        for (index, starter) in cohort.starters.iter().enumerate() {
            let choice = tile(commands, choices, kit);
            commands
                .entity(choice)
                .entry::<Node>()
                .and_modify(move |mut node| {
                    node.flex_basis = if kit.m.frame == crate::shellkit::Frame::Phone {
                        percent(100)
                    } else {
                        percent(30)
                    };
                });
            let number = u8::try_from(index).unwrap_or(0);
            let label_text = Phrase::EchoesStarter.fill(lang, &[&(number + 1).to_string()]);
            heading(commands, choice, kit, &label_text);
            for stack in &starter.deck {
                label(
                    commands,
                    choice,
                    kit,
                    &format!("{} × {}", stack.copies, words::card(stack.card, lang)),
                    tokens::MUTED,
                );
            }
            let space = crate::lobby::parts::grow(commands);
            commands.entity(choice).add_child(space);
            control(
                commands,
                choice,
                kit,
                ui,
                &label_text,
                Action::Starter(cohort.id, number),
                !ui.model.uncertain() && ui.choices.deck_unchanged(ui.model.run()),
            );
        }
    }
    if ui.loaded && ui.model.cohorts().is_empty() {
        label(
            commands,
            body,
            kit,
            Phrase::EchoesNoEnrollment.text(lang),
            tokens::MUTED,
        );
    }
    enrollment_footer(commands, body, kit, ui);
}

fn enrollment_footer(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let privacy = panel(commands, body, kit);
    if ui.withdrawn {
        label(
            commands,
            privacy,
            kit,
            Phrase::EchoesWithdrawn.text(lang),
            tokens::ACCENT,
        );
    }
    control(
        commands,
        privacy,
        kit,
        ui,
        Phrase::EchoesWithdraw.text(lang),
        Action::Withdraw,
        !ui.model.uncertain(),
    );
    if let Some(last) = ui
        .model
        .cohorts()
        .last()
        .filter(|_| ui.model.cohorts().len() == api::PAGE_SIZE)
    {
        control(
            commands,
            body,
            kit,
            ui,
            Phrase::ChallengeOlder.text(lang),
            Action::Read(Read::Cohorts(Some(last.id))),
            true,
        );
    }
}
