//! The calendar: what is open now, what else there is, what was earned, what is past.
//!
//! In reading order: an attempt left open elsewhere, the Daily and the Weekly
//! as two equal cards, the two expeditions as two equal cards, the permanent
//! badges, and the archive as one list.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::challenges::presentation as formatting;

/// The height of a current task's landscape, at the default text step.
const BANNER: f32 = 132.0;

pub(super) fn catalogue(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    archive_visible: bool,
) {
    let live: Vec<_> = model
        .catalogue
        .iter()
        .filter(|task| task.description.period.live_at(model.now))
        .collect();
    elsewhere(commands, body, kit, lang, model, &live);
    if model.catalogue.is_empty() && model.loaded {
        let section = layout::section(commands, body, kit);
        let empty = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::CALENDAR,
            Phrase::ChallengeEmpty.text(lang),
            None,
        );
        commands.entity(section).add_child(empty);
    }
    let stacked = layout::tight(kit);
    if !live.is_empty() {
        let pair = layout::split(commands, body, kit, stacked);
        for task in &live {
            let card = current(commands, pair, kit, lang, model, task);
            layout::share(commands, card, stacked, 1.0);
        }
    }
    if archive_visible {
        let pair = layout::split(commands, body, kit, stacked);
        let echoes = crate::lobby::echoes::entry(commands, pair, kit, lang);
        layout::share(commands, echoes, stacked, 1.0);
        let tower = super::tower::entry(commands, pair, kit, lang, model);
        layout::share(commands, tower, stacked, 1.0);
    }
    badges(commands, body, kit, lang, model);
    if archive_visible {
        archive(commands, body, kit, lang, model);
    }
}

/// Attempts still open on a task that is not one of the current cards: the
/// way back to them leads the page. A current task's own card offers it.
fn elsewhere(
    commands: &mut Commands,
    body: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    live: &[&api::Challenge],
) {
    let open: Vec<_> = model
        .active
        .iter()
        .enumerate()
        .filter(|(_, attempt)| !live.iter().any(|task| task.id == attempt.challenge))
        .collect();
    if open.is_empty() {
        return;
    }
    let section = layout::section(commands, body, kit);
    layout::caption(commands, section, kit, Phrase::ChallengeActive.text(lang));
    for (index, attempt) in open {
        let title = model
            .catalogue
            .iter()
            .find(|task| task.id == attempt.challenge)
            .map_or_else(
                || Phrase::ChallengeResume.text(lang).to_string(),
                |task| words(&task.description.title, lang).to_string(),
            );
        let line = row(commands, section, kit);
        commands.entity(line).entry::<Node>().and_modify(|mut n| {
            n.flex_wrap = FlexWrap::NoWrap;
        });
        let name = name_cell(commands, kit, &title);
        commands.entity(line).add_child(name);
        let control = button(
            commands,
            line,
            kit,
            Phrase::ChallengeResume.text(lang),
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

/// One of the tasks open now: its landscape, what it is, what it asks, and
/// the one way in.
fn current(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) -> Entity {
    let period = task.description.period;
    let card = layout::section(commands, root, kit);
    layout::banner(
        commands,
        card,
        kit,
        presentation::scene(period.kind),
        if layout::tight(kit) {
            BANNER * 0.7
        } else {
            BANNER
        },
    );
    let top = row(commands, card, kit);
    layout::kind_badge(commands, top, kit, lang, period.kind);
    let spacer = crate::lobby::parts::grow(commands);
    commands.entity(top).add_child(spacer);
    let duration = crate::lobby::parts::glyph_line(
        commands,
        kit,
        crate::hud::glyph::HOURGLASS,
        &Phrase::ChallengeMinutes.fill(lang, &[&task.description.expected_minutes.to_string()]),
        tokens::MUTED,
    );
    commands.entity(top).add_child(duration);
    layout::title(
        commands,
        card,
        kit,
        words(&task.description.title, lang),
        kit.m.head,
    );
    for (icon, text) in [
        (
            crate::hud::glyph::BULLSEYE,
            Phrase::ChallengeTargetCompact.fill(lang, &[&task.description.target_turn.to_string()]),
        ),
        (
            crate::hud::glyph::CLOCK,
            format!(
                "{} · {} UTC",
                Phrase::ChallengeCloses.text(lang),
                formatting::date(period.ends_at, lang)
            ),
        ),
    ] {
        let line = crate::lobby::parts::glyph_line(commands, kit, icon, &text, tokens::MUTED);
        commands.entity(card).add_child(line);
    }
    presentation::countdown(commands, card, kit, lang, period.ends_at);
    let space = crate::lobby::parts::grow(commands);
    commands.entity(card).add_child(space);
    let resume = model
        .active
        .iter()
        .any(|attempt| attempt.challenge == task.id);
    button(
        commands,
        card,
        kit,
        if resume {
            Phrase::ChallengeResume
        } else {
            Phrase::ChallengeExplore
        }
        .text(lang),
        Action::Select(task.id),
        model,
        if resume {
            Weight::Gold
        } else {
            Weight::Primary
        },
    );
    card
}

/// Past tasks as one list: kind, title, when it closed, the way in.
fn archive(commands: &mut Commands, body: Entity, kit: Kit, lang: Lang, model: &State) {
    let past: Vec<_> = model
        .catalogue
        .iter()
        .filter(|task| !task.description.period.live_at(model.now))
        .collect();
    if past.is_empty() && model.before.is_none() {
        return;
    }
    let section = layout::section(commands, body, kit);
    heading(
        commands,
        section,
        kit,
        Phrase::ChallengeArchiveTitle.text(lang),
    );
    layout::prose(
        commands,
        section,
        kit,
        Phrase::ChallengeArchive.text(lang),
        tokens::MUTED,
    );
    for task in past {
        layout::rule(commands, section);
        let period = task.description.period;
        let line = row(commands, section, kit);
        commands.entity(line).entry::<Node>().and_modify(|mut n| {
            n.flex_wrap = FlexWrap::NoWrap;
        });
        layout::kind_badge(commands, line, kit, lang, period.kind);
        let name = name_cell(commands, kit, words(&task.description.title, lang));
        commands.entity(line).add_child(name);
        if !layout::tight(kit) {
            let closed = crate::lobby::parts::glyph_line(
                commands,
                kit,
                crate::hud::glyph::CLOCK,
                &format!("{} UTC", formatting::date(period.ends_at, lang)),
                tokens::MUTED,
            );
            commands.entity(line).add_child(closed);
        }
        button(
            commands,
            line,
            kit,
            Phrase::ChallengeExplore.text(lang),
            Action::Select(task.id),
            model,
            Weight::Secondary,
        );
    }
    if model.before.is_some() {
        layout::rule(commands, section);
        let foot = row(commands, section, kit);
        button(
            commands,
            foot,
            kit,
            Phrase::ChallengeOlder.text(lang),
            Action::Older,
            model,
            Weight::Ghost,
        );
    }
}

/// A row's name: body size, one line, taking the room the row has left.
fn name_cell(commands: &mut Commands, kit: Kit, text: &str) -> Entity {
    commands
        .spawn((
            Text::new(text),
            tf(kit.fonts, kit.m.text),
            TextColor(tokens::INK),
            TextLayout::no_wrap(),
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}
