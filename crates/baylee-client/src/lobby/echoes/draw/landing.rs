#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::run::Outcome;

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

pub(super) fn progress(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    let band = row(commands, root, kit);
    let wins = run
        .history
        .iter()
        .filter(|duel| duel.outcome == Outcome::Victory)
        .count();
    let defeats = run.history.len() - wins;
    for (icon, caption, value) in [
        (glyph::CROWN, Phrase::EchoesVictories, format!("{wins}/8")),
        (
            glyph::HEART,
            Phrase::TowerLives,
            (5_usize.saturating_sub(defeats)).to_string(),
        ),
        (glyph::SUN, Phrase::EchoesGold, run.economy.gold.to_string()),
        (
            glyph::COMPASS,
            Phrase::EchoesOrdinal,
            run.shop.stage.to_string(),
        ),
    ] {
        widgets::stat(commands, band, kit, icon, caption.text(lang), &value);
    }
    let history = if run.history.is_empty() {
        root
    } else {
        row(commands, root, kit)
    };
    for (index, duel) in run.history.iter().enumerate() {
        let text = format!(
            "{} · {} · {}",
            index + 1,
            words::outcome(duel.outcome).text(lang),
            Phrase::EchoesTurns.fill(lang, &[&duel.turns.to_string()])
        );
        let badge = crate::lobby::parts::badge_with(
            commands,
            kit,
            if duel.outcome == Outcome::Victory {
                glyph::TROPHY
            } else {
                glyph::SHIELD
            },
            &text,
            tokens::INK,
        );
        commands.entity(history).add_child(badge);
    }
    play(commands, root, kit, ui, run);
}

fn play(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    match run.phase {
        api::Phase::Shop => {
            let band = row(commands, root, kit);
            label(
                commands,
                band,
                kit,
                words::phase(run.phase).text(lang),
                tokens::ACCENT,
            );
            let spacer = crate::lobby::parts::grow(commands);
            commands.entity(band).add_child(spacer);
            if !ui.choices.deck_unchanged(Some(run)) {
                label(
                    commands,
                    band,
                    kit,
                    Phrase::EchoesUnsaved.text(lang),
                    tokens::GOLD,
                );
            }
            if ui.model.hosts().is_empty() {
                label(
                    commands,
                    root,
                    kit,
                    Phrase::EchoesNoHost.text(lang),
                    tokens::MUTED,
                );
            }
            for host in ui.model.hosts() {
                control(
                    commands,
                    band,
                    kit,
                    ui,
                    &if ui.model.hosts().len() == 1 {
                        Phrase::EchoesPlay.text(lang).to_owned()
                    } else {
                        Phrase::EchoesPlayAt.fill(lang, &[&host.name])
                    },
                    Action::Play(host.id),
                    !ui.model.uncertain() && ui.choices.deck_unchanged(Some(run)),
                );
            }
            if let Some(last) = ui
                .model
                .hosts()
                .last()
                .filter(|_| ui.model.hosts().len() == api::PAGE_SIZE)
            {
                control(
                    commands,
                    root,
                    kit,
                    ui,
                    Phrase::EchoesMoreHosts.text(lang),
                    Action::Read(Read::Hosts(Some(last.id))),
                    true,
                );
            }
        }
        api::Phase::Reserved | api::Phase::Playing => {
            control(
                commands,
                root,
                kit,
                ui,
                Phrase::TowerResume.text(lang),
                Action::Resume,
                !ui.model.uncertain(),
            );
        }
        _ => {
            label(
                commands,
                root,
                kit,
                words::phase(run.phase).text(lang),
                tokens::ACCENT,
            );
        }
    }
}
