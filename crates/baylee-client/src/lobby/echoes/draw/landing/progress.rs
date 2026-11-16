#[allow(clippy::wildcard_imports)]
use super::*;
use domain::v2::run::Outcome;

pub(in crate::lobby::echoes::draw) fn market_progress(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
) {
    if let Some(run) = ui.model.run() {
        metrics(commands, root, kit, run, true);
        if !domain::v2::generation::is_run(run.shop.generator) {
            play(commands, root, kit, ui, run);
        }
    }
}

pub(in crate::lobby::echoes::draw) fn progress(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    run: &api::Run,
) {
    let lang = lang(kit);
    let band = row(commands, root, kit);
    metrics(commands, band, kit, run, false);
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

fn metrics(commands: &mut Commands, root: Entity, kit: Kit, run: &api::Run, compact: bool) {
    let lang = lang(kit);
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
        (
            glyph::COINS,
            Phrase::EchoesGold,
            run.economy.gold.to_string(),
        ),
        (
            glyph::COMPASS,
            Phrase::EchoesOrdinal,
            run.shop.stage.to_string(),
        ),
    ] {
        if compact && caption == Phrase::EchoesGold {
            continue;
        }
        if compact {
            let text = if caption == Phrase::EchoesGold {
                format!("{value} {}", caption.text(lang))
            } else {
                value.clone()
            };
            let badge = crate::lobby::parts::badge_with(commands, kit, icon, &text, tokens::INK);
            commands.entity(badge).insert(Pickable::default());
            controls::attach_tooltip(
                commands,
                badge,
                kit,
                &format!("{} · {value}", caption.text(lang)),
            );
            commands.entity(root).add_child(badge);
        } else {
            widgets::stat(commands, root, kit, icon, caption.text(lang), &value);
        }
    }
}

fn play(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui, run: &api::Run) {
    let lang = lang(kit);
    match run.phase {
        api::Phase::Shop => {
            let band = row(commands, root, kit);
            commands
                .entity(band)
                .entry::<Node>()
                .and_modify(|mut node| {
                    node.width = Val::Auto;
                    node.flex_grow = 1.0;
                    node.justify_content = JustifyContent::End;
                });
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
