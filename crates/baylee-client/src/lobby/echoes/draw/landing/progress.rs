#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::run::Outcome;

pub(in crate::lobby::echoes::draw) fn progress(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
) {
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
