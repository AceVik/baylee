//! Calendar panels.
#[allow(clippy::wildcard_imports)]
use super::*;

pub(super) fn catalogue(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    archive_visible: bool,
) {
    if !model.active.is_empty() {
        let panel = panel(commands, body, kit);
        heading(commands, panel, kit, Phrase::ChallengeActive.text(lang));
        for (index, attempt) in model.active.iter().enumerate() {
            let title = model
                .catalogue
                .iter()
                .find(|task| task.id == attempt.challenge)
                .map_or_else(
                    || Phrase::ChallengeResume.text(lang).to_string(),
                    |task| words(&task.description.title, lang).to_string(),
                );
            let control = button(
                commands,
                panel,
                kit,
                &title,
                Action::Select(attempt.challenge),
                model,
                Weight::Gold,
            );
            crate::lobby::orders::item(
                commands,
                control,
                &crate::lobby::orders::CHALLENGES,
                "active",
                index,
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
    let pair = row(commands, body, kit);
    commands
        .entity(pair)
        .entry::<Node>()
        .and_modify(move |mut n| {
            n.align_items = AlignItems::Stretch;
            n.flex_direction = if kit.m.frame == crate::shellkit::Frame::Phone {
                FlexDirection::Column
            } else {
                FlexDirection::Row
            };
            n.flex_wrap = FlexWrap::NoWrap;
        });
    for task in model
        .catalogue
        .iter()
        .filter(|task| task.description.period.live_at(model.now))
    {
        card(commands, pair, kit, lang, model, task, true);
    }
    badges(commands, body, kit, lang, model);
    if !archive_visible {
        return;
    }
    let archive: Vec<_> = model
        .catalogue
        .iter()
        .filter(|task| !task.description.period.live_at(model.now))
        .collect();
    if !archive.is_empty() {
        heading(commands, body, kit, Phrase::ChallengeArchive.text(lang));
    }
    for task in archive {
        card(commands, body, kit, lang, model, task, false);
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
}

fn card(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
    featured: bool,
) {
    let panel = panel(commands, body, kit);
    if featured {
        commands
            .entity(panel)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.width = Val::Auto;
                n.flex_grow = 1.0;
                n.flex_basis = px_fixed(0.0);
                n.min_width = px_fixed(0.0);
                n.min_height = kit.m.px(210.0);
                n.border = UiRect {
                    top: px_fixed(2.0),
                    ..UiRect::all(px_fixed(1.0))
                };
            });
    }
    let kind = if task.description.period.kind == api::Kind::Daily {
        Phrase::ChallengeDaily
    } else {
        Phrase::ChallengeWeekly
    };
    let accent = if task.description.period.kind == api::Kind::Daily {
        tokens::GOLD
    } else {
        tokens::ACCENT
    };
    commands.entity(panel).insert(BorderColor {
        top: accent.with_alpha(0.75),
        ..BorderColor::all(tokens::BORDER)
    });
    let top = row(commands, panel, kit);
    let caption = crate::lobby::parts::badge(commands, kit, kind.text(lang), accent);
    commands.entity(top).add_child(caption);
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    let duration = crate::lobby::parts::glyph_line(
        commands,
        kit,
        '\u{f017}',
        &Phrase::ChallengeMinutes.fill(lang, &[&task.description.expected_minutes.to_string()]),
        tokens::MUTED,
    );
    commands.entity(top).add_child(duration);
    let title = crate::lobby::parts::line(
        commands,
        kit,
        words(&task.description.title, lang),
        kit.m.text * 1.12,
        tokens::INK,
    );
    commands.entity(panel).add_child(title);
    label(
        commands,
        panel,
        kit,
        &Phrase::ChallengeTargetCompact.fill(lang, &[&task.description.target_turn.to_string()]),
        tokens::MUTED,
    );
    let space = crate::lobby::parts::grow(commands);
    commands.entity(panel).add_child(space);
    button(
        commands,
        panel,
        kit,
        Phrase::ChallengeExplore.text(lang),
        Action::Select(task.id),
        model,
        if featured {
            Weight::Primary
        } else {
            Weight::Secondary
        },
    );
}
