//! Run status, lives, admission and explicit retirement.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    season: &tower_api::Season,
) {
    let tower = &model.tower;
    let phase = tower.run.as_ref().map(|run| run.progress.phase);
    let title = match phase {
        None | Some(Phase::Ready) => Phrase::TowerReady,
        Some(Phase::Playing) => Phrase::TowerPlaying,
        Some(Phase::Choosing) => Phrase::TowerChoosing,
        Some(Phase::Conquered) => Phrase::TowerConquered,
        Some(Phase::Exhausted) => Phrase::TowerExhausted,
        Some(Phase::Abandoned) => Phrase::TowerAbandoned,
    };
    heading(commands, parent, kit, title.text(lang));
    summit(commands, parent, kit, lang, model);
    if let Some(run) = &tower.run {
        let lives = row(commands, parent, kit);
        for life in 0..3 {
            let remaining = life < run.progress.lives;
            let pip = crate::lobby::parts::badge(
                commands,
                kit,
                if remaining { "◆" } else { "◇" },
                if remaining {
                    tokens::ACCENT
                } else {
                    tokens::MUTED
                },
            );
            commands.entity(lives).add_child(pip);
        }
        label(
            commands,
            parent,
            kit,
            &Phrase::TowerLifeCount.fill(lang, &[&run.progress.lives.to_string()]),
            if run.progress.lives <= 1 {
                tokens::DANGER
            } else {
                tokens::ACCENT
            },
        );
        label(
            commands,
            parent,
            kit,
            &Phrase::TowerScore.fill(
                lang,
                &[
                    &run.progress.cleared.to_string(),
                    &(3 - run.progress.lives).to_string(),
                    &run.progress.turns.to_string(),
                ],
            ),
            tokens::MUTED,
        );
    }
    let live = season.theme.period.live_at(tower.now)
        && tower
            .run
            .as_ref()
            .is_none_or(|run| run.progress.competitive);
    label(
        commands,
        parent,
        kit,
        if live {
            Phrase::TowerRanked
        } else {
            Phrase::TowerPractice
        }
        .text(lang),
        tokens::MUTED,
    );
    if season.withdrawn {
        label(
            commands,
            parent,
            kit,
            Phrase::ChallengeWithdrawn.text(lang),
            tokens::DANGER,
        );
    } else if phase != Some(Phase::Choosing) {
        let play = match phase {
            None => Phrase::TowerBegin,
            Some(Phase::Playing) => Phrase::TowerResume,
            Some(Phase::Ready) => Phrase::TowerNext,
            _ => Phrase::TowerRestart,
        };
        control(
            commands,
            parent,
            kit,
            lang,
            play,
            TowerAction::Play,
            model,
            Weight::Gold,
        );
    }
    consent(commands, parent, kit, lang, model);
}

fn summit(commands: &mut Commands, parent: Entity, kit: Kit, lang: Lang, model: &State) {
    if !model
        .tower
        .run
        .as_ref()
        .is_some_and(|run| run.progress.phase == Phase::Conquered)
    {
        return;
    }
    let Some(badge) = model
        .tower
        .badges
        .iter()
        .find(|badge| badge.kind == tower_api::BadgeKind::Summit && badge.tier.is_some())
    else {
        return;
    };
    collection::medal(commands, parent, kit, badge.kind, badge.tier, 128.0);
    label(
        commands,
        parent,
        kit,
        Phrase::TowerEarnSummit.text(lang),
        tokens::GOLD,
    );
}

fn consent(commands: &mut Commands, parent: Entity, kit: Kit, lang: Lang, model: &State) {
    let tower = &model.tower;
    let phase = tower.run.as_ref().map(|run| run.progress.phase);
    let listing = if tower.listed {
        Phrase::ChallengeListed
    } else {
        Phrase::ChallengePrivate
    };
    control(
        commands,
        parent,
        kit,
        lang,
        listing,
        TowerAction::Visibility,
        model,
        Weight::Secondary,
    );
    label(
        commands,
        parent,
        kit,
        Phrase::ChallengeConsent.text(lang),
        tokens::MUTED,
    );
    if phase.is_some_and(Phase::active) {
        control(
            commands,
            parent,
            kit,
            lang,
            Phrase::TowerRetire,
            TowerAction::Abandon,
            model,
            Weight::Ghost,
        );
    }
    if tower.abandoning {
        label(
            commands,
            parent,
            kit,
            Phrase::TowerRetireConfirm.text(lang),
            tokens::DANGER,
        );
        let row = row(commands, parent, kit);
        control(
            commands,
            row,
            kit,
            lang,
            Phrase::ChallengeCancel,
            TowerAction::Cancel,
            model,
            Weight::Secondary,
        );
        control(
            commands,
            row,
            kit,
            lang,
            Phrase::ChallengeConfirm,
            TowerAction::Confirm,
            model,
            Weight::Gold,
        );
    }
}
