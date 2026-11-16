#[allow(clippy::wildcard_imports)]
use super::*;
mod progress;
mod starter;
pub(super) use progress::{market_progress, progress};

pub(super) fn draw(commands: &mut Commands, body: Entity, kit: Kit, ui: &Ui) {
    if let Some(id) = ui.preview {
        card::preview(commands, body, kit, id);
        control(
            commands,
            body,
            kit,
            ui,
            Phrase::ShellClose.text(lang(kit)),
            Action::ClosePreview,
            true,
        );
    }
    enrollment(commands, body, kit, ui);
    saved(commands, body, kit, ui);
    enrollment_footer(commands, body, kit, ui);
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
        let run_format = cohort
            .starters
            .iter()
            .all(|starter| starter.formation.is_some());
        let card = tile(commands, band, kit);
        presentation::title(
            commands,
            card,
            kit,
            glyph::CROWN,
            Phrase::EchoesNewRun.text(lang),
        );
        if !run_format {
            label(
                commands,
                card,
                kit,
                &Phrase::EchoesPublished.fill(lang, &[&words::created(cohort.id, lang)]),
                tokens::MUTED,
            );
        }
        label(
            commands,
            card,
            kit,
            Phrase::EchoesStarterLead.text(lang),
            tokens::MUTED,
        );
        let choices = choices(commands, card, kit, run_format);
        for (index, starter) in cohort.starters.iter().enumerate() {
            let number = u8::try_from(index).unwrap_or(0);
            if let Some(formation) = &starter.formation {
                starter::draw(
                    commands,
                    choices,
                    kit,
                    ui,
                    cohort.id,
                    number,
                    formation.commander,
                );
                continue;
            }
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
        if run_format {
            label(
                commands,
                card,
                kit,
                Phrase::EchoesRunStarterFormat.text(lang),
                tokens::MUTED,
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
}

fn choices(commands: &mut Commands, card: Entity, kit: Kit, run_format: bool) -> Entity {
    let choices = row(commands, card, kit);
    if run_format {
        commands
            .entity(choices)
            .entry::<Node>()
            .and_modify(move |mut node| {
                node.display = Display::Grid;
                node.grid_template_columns = vec![RepeatedGridTrack::minmax(
                    GridTrackRepetition::AutoFill,
                    MinTrackSizingFunction::Px(kit.m.scaled(210.0)),
                    MaxTrackSizingFunction::Fraction(1.0),
                )];
                node.align_items = AlignItems::Stretch;
            });
    }
    choices
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
