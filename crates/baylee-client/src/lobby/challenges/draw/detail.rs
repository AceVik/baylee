//! Task, consent, hints and paginated independent scoreboards.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    let card = panel(commands, root, kit);
    heading(commands, card, kit, words(&task.description.title, lang));
    status(commands, card, kit, lang, model, task);
    label(
        commands,
        card,
        kit,
        words(&task.description.instructions, lang),
        tokens::INK,
    );
    label(
        commands,
        card,
        kit,
        &Phrase::ChallengeTarget.fill(
            lang,
            &[
                &task.description.target_turn.to_string(),
                &task.description.expected_minutes.to_string(),
            ],
        ),
        tokens::ACCENT,
    );
    label(
        commands,
        card,
        kit,
        Phrase::ChallengeRules.text(lang),
        tokens::MUTED,
    );
    if let Some(progress) = &model.personal {
        let best = progress
            .best_turn
            .map_or_else(|| "—".into(), |turn| turn.to_string());
        label(
            commands,
            card,
            kit,
            &Phrase::ChallengePersonal.fill(
                lang,
                &[
                    &progress.starts.to_string(),
                    &progress.failures.to_string(),
                    &best,
                ],
            ),
            tokens::INK,
        );
        if progress.completed_at.is_some() {
            label(
                commands,
                card,
                kit,
                Phrase::ChallengeCompleted.text(lang),
                tokens::ACCENT,
            );
        }
        if progress.hint_level > 0 {
            label(
                commands,
                card,
                kit,
                Phrase::ChallengeAssisted.text(lang),
                tokens::MUTED,
            );
        }
    }
    launch(commands, card, kit, lang, model, task);
    preferences(commands, card, kit, lang, model);
    rankings(commands, root, kit, lang, model);
}

fn launch(
    commands: &mut Commands,
    card: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    let actions = row(commands, card, kit);
    if !task.withdrawn && model.personal.is_some() {
        let resume = model.attempt().is_some();
        button(
            commands,
            actions,
            kit,
            if resume {
                Phrase::ChallengeResume
            } else {
                Phrase::ChallengePlay
            }
            .text(lang),
            Action::Play,
            model,
            if resume {
                Weight::Gold
            } else {
                Weight::Primary
            },
        );
        button(
            commands,
            actions,
            kit,
            Phrase::ChallengeHint.text(lang),
            Action::Hint,
            model,
            Weight::Secondary,
        );
        if resume {
            button(
                commands,
                actions,
                kit,
                Phrase::ChallengeAbandon.text(lang),
                Action::Abandon,
                model,
                Weight::Ghost,
            );
        }
    }
    button(
        commands,
        actions,
        kit,
        Phrase::ChallengeDeck.text(lang),
        Action::Deck,
        model,
        Weight::Ghost,
    );
    if model.show_deck {
        label(commands, card, kit, &task.description.deck, tokens::INK);
    }
}

fn preferences(commands: &mut Commands, card: Entity, kit: Kit, lang: Lang, model: &State) {
    if model.personal.is_some() {
        button(
            commands,
            card,
            kit,
            if model.listed {
                Phrase::ChallengeListed
            } else {
                Phrase::ChallengePrivate
            }
            .text(lang),
            Action::Visibility,
            model,
            Weight::Secondary,
        );
        label(
            commands,
            card,
            kit,
            Phrase::ChallengeConsent.text(lang),
            tokens::MUTED,
        );
    }
    if let Some(confirmation) = model.confirmation {
        let confirmation_card = panel(commands, card, kit);
        let phrase = match confirmation {
            Confirmation::Hint(_) => Phrase::ChallengeHintConfirm,
            Confirmation::Abandon(_) => Phrase::ChallengeAbandonConfirm,
        };
        label(
            commands,
            confirmation_card,
            kit,
            phrase.text(lang),
            tokens::INK,
        );
        let choices = row(commands, confirmation_card, kit);
        button(
            commands,
            choices,
            kit,
            Phrase::ChallengeCancel.text(lang),
            Action::Cancel,
            model,
            Weight::Secondary,
        );
        button(
            commands,
            choices,
            kit,
            Phrase::ChallengeConfirm.text(lang),
            Action::Confirm,
            model,
            Weight::Danger,
        );
    }
    if let Some(hint) = &model.hint {
        label(commands, card, kit, words(hint, lang), tokens::ACCENT);
    }
}

fn rankings(commands: &mut Commands, root: Entity, kit: Kit, lang: Lang, model: &State) {
    let scores = panel(commands, root, kit);
    heading(commands, scores, kit, Phrase::ChallengeBoards.text(lang));
    let tabs = row(commands, scores, kit);
    for (board, phrase) in [
        (api::Board::FirstWin, Phrase::ChallengeFirst),
        (api::Board::BestTurns, Phrase::ChallengeTurns),
        (api::Board::Sprint, Phrase::ChallengeTime),
    ] {
        button(
            commands,
            tabs,
            kit,
            phrase.text(lang),
            Action::Board(board),
            model,
            if model.board == board {
                Weight::Primary
            } else {
                Weight::Secondary
            },
        );
    }
    if model.standings.is_empty() && !model.busy {
        label(
            commands,
            scores,
            kit,
            Phrase::ChallengeNoScores.text(lang),
            tokens::MUTED,
        );
    }
    for score in &model.standings {
        let name = score.tag.map_or_else(
            || score.name.clone(),
            |tag| format!("{}#{tag:04x}", score.name),
        );
        let name = if score.own {
            Phrase::ChallengeYou.fill(lang, &[&name])
        } else {
            name
        };
        let seconds = score.milliseconds.map_or_else(
            || "—".into(),
            |ms| format!("{}.{:03}", ms / 1000, ms % 1000),
        );
        let text = Phrase::ChallengeStanding.fill(
            lang,
            &[
                &score.rank.to_string(),
                &name,
                &score.turn.to_string(),
                &score.failures.to_string(),
                &seconds,
            ],
        );
        label(
            commands,
            scores,
            kit,
            &text,
            if score.own {
                tokens::ACCENT
            } else {
                tokens::INK
            },
        );
    }
    let pager = row(commands, scores, kit);
    if model.offset > 0 {
        button(
            commands,
            pager,
            kit,
            Phrase::ChallengePrevious.text(lang),
            Action::Page(false),
            model,
            Weight::Secondary,
        );
    }
    if model.standings.len() == api::PAGE_SIZE && model.offset < 3000 {
        button(
            commands,
            pager,
            kit,
            Phrase::ChallengeNext.text(lang),
            Action::Page(true),
            model,
            Weight::Secondary,
        );
    }
}
