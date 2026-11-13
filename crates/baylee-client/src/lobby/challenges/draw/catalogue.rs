//! Calendar panels.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn catalogue(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    label(
        commands,
        body,
        kit,
        Phrase::ChallengeIntro.text(lang),
        tokens::INK,
    );
    if !model.active.is_empty() {
        let panel = panel(commands, body, kit);
        heading(commands, panel, kit, Phrase::ChallengeActive.text(lang));
        for attempt in &model.active {
            let title = model
                .catalogue
                .iter()
                .find(|task| task.id == attempt.challenge)
                .map_or_else(
                    || Phrase::ChallengeResume.text(lang).to_string(),
                    |task| words(&task.description.title, lang).to_string(),
                );
            button(
                commands,
                panel,
                kit,
                &title,
                Action::Select(attempt.challenge),
                model,
                Weight::Gold,
            );
        }
    }
    if model.catalogue.is_empty() && model.loaded {
        label(
            commands,
            body,
            kit,
            Phrase::ChallengeEmpty.text(lang),
            tokens::MUTED,
        );
    }
    for task in &model.catalogue {
        let panel = panel(commands, body, kit);
        let kind = if task.description.period.kind == api::Kind::Daily {
            Phrase::ChallengeDaily
        } else {
            Phrase::ChallengeWeekly
        };
        label(commands, panel, kit, kind.text(lang), tokens::ACCENT);
        heading(commands, panel, kit, words(&task.description.title, lang));
        status(commands, panel, kit, lang, model, task);
        label(
            commands,
            panel,
            kit,
            words(&task.description.instructions, lang),
            tokens::INK,
        );
        button(
            commands,
            panel,
            kit,
            Phrase::Continue.text(lang),
            Action::Select(task.id),
            model,
            Weight::Secondary,
        );
    }
    if model.before.is_some() {
        button(
            commands,
            body,
            kit,
            Phrase::ChallengeOlder.text(lang),
            Action::Older,
            model,
            Weight::Secondary,
        );
    }
    badges(commands, body, kit, lang, model);
}
