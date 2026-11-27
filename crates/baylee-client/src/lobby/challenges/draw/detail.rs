//! One task: what it asks, where the player stands, the way in, and the rankings.
//!
//! The task's own card reads top to bottom as a player decides: the task,
//! their progress, the one primary action with its two lesser ones, then
//! consent and the long explanations. The rankings stand beside it as a table.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::hud::tf_bold;
use baylee_client_core::challenges::presentation as formatting;

/// The height of the task's landscape, at the default text step.
const BANNER: f32 = 104.0;

pub(super) fn draw(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    let stacked = !layout::wide(kit);
    let columns = layout::split(commands, root, kit, stacked);
    let card = layout::section(commands, columns, kit);
    layout::share(commands, card, stacked, 1.2);
    let board = layout::section(commands, columns, kit);
    layout::share(commands, board, stacked, 1.0);
    for section in [card, board] {
        commands
            .entity(section)
            .entry::<Node>()
            .and_modify(move |mut n| {
                n.padding = UiRect::axes(px_fixed(kit.m.pad), kit.m.px(14.0));
                n.row_gap = kit.m.px(8.0);
            });
    }
    brief(commands, card, kit, lang, model, task);
    progress(commands, card, kit, lang, model);
    launch(commands, card, kit, lang, model, task);
    answers(commands, card, kit, lang, model);
    privacy(commands, card, kit, lang, model);
    more(commands, card, kit, lang, model, task);
    rankings(commands, board, kit, lang, model);
}

/// The task itself: its landscape, kind and state, its name, what to do.
fn brief(
    commands: &mut Commands,
    card: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    let period = task.description.period;
    layout::banner(
        commands,
        card,
        kit,
        presentation::scene(period.kind),
        if layout::tight(kit) {
            BANNER * 0.6
        } else {
            BANNER
        },
    );
    let top = row(commands, card, kit);
    layout::kind_badge(commands, top, kit, lang, period.kind);
    let (state, ink) = if task.withdrawn {
        (Phrase::ChallengeWithdrawn, tokens::DANGER)
    } else if model.now < period.ends_at {
        (Phrase::ChallengeLive, tokens::MUTED)
    } else {
        (Phrase::ChallengeArchive, tokens::MUTED)
    };
    label(commands, top, kit, state.text(lang), ink);
    layout::title(
        commands,
        card,
        kit,
        words(&task.description.title, lang),
        kit.m.h1,
    );
    layout::prose(
        commands,
        card,
        kit,
        words(&task.description.instructions, lang),
        tokens::INK,
    );
    let goal = layout::inset(commands, card, kit);
    let target = crate::lobby::parts::glyph_line(
        commands,
        kit,
        crate::hud::glyph::BULLSEYE,
        &Phrase::ChallengeTarget.fill(
            lang,
            &[
                &task.description.target_turn.to_string(),
                &task.description.expected_minutes.to_string(),
            ],
        ),
        tokens::ACCENT,
    );
    commands.entity(goal).add_child(target);
    presentation::calendar(commands, goal, kit, lang, period);
    if !task.withdrawn {
        presentation::countdown(commands, goal, kit, lang, period.ends_at);
    }
}

fn progress(commands: &mut Commands, card: Entity, kit: Kit, lang: Lang, model: &State) {
    let Some(progress) = &model.personal else {
        return;
    };
    layout::caption(
        commands,
        card,
        kit,
        Phrase::ChallengeSectionProgress.text(lang),
    );
    let best = progress
        .best_turn
        .map_or_else(|| "—".into(), |turn| turn.to_string());
    let band = layout::split(commands, card, kit, false);
    for (icon, caption, value) in [
        (
            crate::hud::glyph::PLAY,
            Phrase::ChallengeStatStarts,
            progress.starts.to_string(),
        ),
        (
            crate::hud::glyph::CLOSE,
            Phrase::ChallengeStatFailures,
            progress.failures.to_string(),
        ),
        (crate::hud::glyph::FINISH, Phrase::ChallengeStatBest, best),
    ] {
        crate::lobby::parts::stat(commands, band, kit, icon, &value, caption.text(lang));
    }
    if let Some(at) = progress.completed_at {
        let message = format!(
            "{} · {} UTC",
            Phrase::ChallengeCompleted.text(lang),
            formatting::date(at, lang)
        );
        let completed = crate::lobby::parts::glyph_line(
            commands,
            kit,
            crate::hud::glyph::CHECK,
            &message,
            tokens::ACCENT,
        );
        commands.entity(card).add_child(completed);
    }
    if progress.hint_level > 0 {
        let assisted = crate::lobby::parts::glyph_line(
            commands,
            kit,
            crate::hud::glyph::INFO,
            Phrase::ChallengeAssisted.text(lang),
            tokens::MUTED,
        );
        commands.entity(card).add_child(assisted);
    }
}

/// The one primary action, the hint beside it, and abandoning at the far end.
fn launch(
    commands: &mut Commands,
    card: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    if task.withdrawn || model.personal.is_none() {
        return;
    }
    let actions = row(commands, card, kit);
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
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(actions).add_child(spacer);
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

/// What an action just asked or revealed: the confirmation, a seen hint.
fn answers(commands: &mut Commands, card: Entity, kit: Kit, lang: Lang, model: &State) {
    if let Some(confirmation) = model.confirmation {
        let ask = layout::inset(commands, card, kit);
        commands
            .entity(ask)
            .insert(BorderColor::all(tokens::DANGER.with_alpha(0.6)));
        let phrase = match confirmation {
            Confirmation::Hint(_) => Phrase::ChallengeHintConfirm,
            Confirmation::Abandon(_) => Phrase::ChallengeAbandonConfirm,
        };
        layout::prose(commands, ask, kit, phrase.text(lang), tokens::INK);
        let choices = row(commands, ask, kit);
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
        let seen = layout::inset(commands, card, kit);
        commands
            .entity(seen)
            .insert(BorderColor::all(tokens::ACCENT.with_alpha(0.6)));
        layout::prose(commands, seen, kit, words(hint, lang), tokens::INK);
    }
}

/// Consent stays beside its control: what listing means, and the switch.
fn privacy(commands: &mut Commands, card: Entity, kit: Kit, lang: Lang, model: &State) {
    if model.personal.is_none() {
        return;
    }
    layout::rule(commands, card);
    layout::caption(commands, card, kit, Phrase::ChallengePrivacy.text(lang));
    let stacked = layout::tight(kit);
    let line = layout::split(commands, card, kit, stacked);
    commands.entity(line).entry::<Node>().and_modify(|mut n| {
        n.align_items = AlignItems::Center;
    });
    let consent = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_shrink: 1.0,
                flex_basis: px_fixed(0.0),
                min_width: px_fixed(0.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(line).add_child(consent);
    if stacked {
        commands
            .entity(consent)
            .entry::<Node>()
            .and_modify(|mut n| {
                n.width = percent(100);
                n.flex_basis = Val::Auto;
            });
    }
    layout::prose(
        commands,
        consent,
        kit,
        Phrase::ChallengeConsent.text(lang),
        tokens::MUTED,
    );
    button(
        commands,
        line,
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
}

/// The long reads, folded: the supplied deck and how a task is scored.
fn more(
    commands: &mut Commands,
    card: Entity,
    kit: Kit,
    lang: Lang,
    model: &State,
    task: &api::Challenge,
) {
    layout::rule(commands, card);
    let folds = row(commands, card, kit);
    button(
        commands,
        folds,
        kit,
        Phrase::ChallengeDeck.text(lang),
        Action::Deck,
        model,
        Weight::Ghost,
    );
    button(
        commands,
        folds,
        kit,
        Phrase::ChallengeRulesToggle.text(lang),
        Action::Rules,
        model,
        Weight::Ghost,
    );
    if model.show_deck {
        let list = layout::inset(commands, card, kit);
        layout::caption(commands, list, kit, Phrase::ChallengeDeck.text(lang));
        let deck = supplied_deck_text(&task.description.deck, lang);
        layout::prose(commands, list, kit, &deck, tokens::INK);
    }
    if model.show_rules {
        let rules = layout::inset(commands, card, kit);
        layout::caption(
            commands,
            rules,
            kit,
            Phrase::ChallengeRulesToggle.text(lang),
        );
        for phrase in [Phrase::ChallengeRules, Phrase::ChallengeBadgeRules] {
            layout::prose(commands, rules, kit, phrase.text(lang), tokens::MUTED);
        }
    }
}

fn rankings(commands: &mut Commands, scores: Entity, kit: Kit, lang: Lang, model: &State) {
    presentation::title(
        commands,
        scores,
        kit,
        crate::hud::glyph::TROPHY,
        Phrase::ChallengeBoards.text(lang),
    );
    board_tabs(commands, scores, kit, lang, model);
    standings(commands, scores, kit, lang, model);
    if !model.standings.is_empty() {
        let space = crate::lobby::parts::grow(commands);
        commands.entity(scores).add_child(space);
    }
    pager(commands, scores, kit, lang, model);
}

/// The three rankings as equal tabs; the one shown is the primary.
fn board_tabs(commands: &mut Commands, scores: Entity, kit: Kit, lang: Lang, model: &State) {
    let tabs = layout::split(commands, scores, kit, false);
    let mut tab_kit = kit;
    tab_kit.m.text = kit.m.small;
    for (board, phrase) in [
        (api::Board::FirstWin, Phrase::ChallengeFirst),
        (api::Board::BestTurns, Phrase::ChallengeTurns),
        (api::Board::Sprint, Phrase::ChallengeTime),
    ] {
        let tab = button(
            commands,
            tabs,
            tab_kit,
            phrase.text(lang),
            Action::Board(board),
            model,
            if model.board == board {
                Weight::Primary
            } else {
                Weight::Secondary
            },
        );
        layout::share(commands, tab, false, 1.0);
    }
}

/// The ranking as one table: the column names once, then a line per score.
fn standings(commands: &mut Commands, scores: Entity, kit: Kit, lang: Lang, model: &State) {
    let table = Columns::of(kit);
    table.line(
        commands,
        scores,
        kit,
        [
            Phrase::ChallengeStatRank.text(lang),
            Phrase::ChallengeStatPlayer.text(lang),
            Phrase::ChallengeStatTurns.text(lang),
            Phrase::ChallengeStatFailures.text(lang),
            Phrase::ChallengeStatTime.text(lang),
        ],
        Tone::Head,
        None,
    );
    if model.standings.is_empty() && !model.blocking() {
        // No score yet: the ranking says so in its middle, under a cup,
        // as every empty list of the lobby does.
        let empty = crate::shellkit::states::empty(
            commands,
            kit,
            crate::hud::glyph::TROPHY,
            Phrase::ChallengeNoScores.text(lang),
            None,
        );
        commands.entity(empty).entry::<Node>().and_modify(|mut n| {
            n.width = percent(100);
            n.flex_grow = 1.0;
            n.justify_content = JustifyContent::Center;
        });
        commands.entity(scores).add_child(empty);
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
        table.line(
            commands,
            scores,
            kit,
            [
                &score.rank.to_string(),
                &name,
                &score.turn.to_string(),
                &score.failures.to_string(),
                &formatting::elapsed(score.milliseconds),
            ],
            if score.own { Tone::Own } else { Tone::Other },
            usize::try_from(score.rank).ok(),
        );
    }
}

/// The ranking's pages: back on the left, on on the right.
fn pager(commands: &mut Commands, scores: Entity, kit: Kit, lang: Lang, model: &State) {
    let previous = model.offset > 0;
    let next = model.standings.len() == api::PAGE_SIZE && model.offset < 3000;
    if previous || next {
        let pager = row(commands, scores, kit);
        if previous {
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
        let spacer = crate::lobby::parts::grow(commands);
        commands.entity(pager).add_child(spacer);
        if next {
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
}

/// Which line of the ranking table is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    /// The column names.
    Head,
    /// Another player's score.
    Other,
    /// The player's own score.
    Own,
}

/// The ranking table's columns: rank, player, turns, failures, time. The
/// player's name takes the room the four fixed columns leave.
#[derive(Clone, Copy)]
struct Columns([f32; 5]);

impl Columns {
    fn of(kit: Kit) -> Self {
        Self(if layout::tight(kit) {
            [44.0, 0.0, 48.0, 88.0, 84.0]
        } else {
            [60.0, 0.0, 64.0, 124.0, 116.0]
        })
    }

    fn line(
        self,
        commands: &mut Commands,
        root: Entity,
        kit: Kit,
        cells: [&str; 5],
        tone: Tone,
        place: Option<usize>,
    ) {
        let line = commands
            .spawn((
                Node {
                    width: percent(100),
                    min_width: px_fixed(0.0),
                    min_height: if tone == Tone::Head {
                        Val::Auto
                    } else {
                        px_fixed(kit.m.control)
                    },
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    // Two steps between columns: a right-aligned number never
                    // meets the next column's name in a long German heading.
                    column_gap: kit.m.space(2),
                    padding: UiRect::axes(kit.m.space(2), kit.m.space(0)),
                    border: UiRect::bottom(px_fixed(1.0)),
                    border_radius: if tone == Tone::Own {
                        BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL))
                    } else {
                        BorderRadius::ZERO
                    },
                    ..default()
                },
                BorderColor::all(tokens::BORDER),
                BackgroundColor(if tone == Tone::Own {
                    tokens::ACCENT.with_alpha(0.12)
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            ))
            .id();
        for (index, (text, width)) in cells.into_iter().zip(self.0).enumerate() {
            if index == 0
                && let Some(place) = place
            {
                let cell = crate::lobby::parts::place_cell(
                    commands,
                    kit,
                    place,
                    kit.m.px(width),
                    kit.m.text,
                    tokens::INK,
                );
                commands.entity(line).add_child(cell);
                continue;
            }
            let name = index == 1;
            let font = match tone {
                Tone::Head => tf_bold(kit.fonts, kit.m.small),
                Tone::Own if name => tf_bold(kit.fonts, kit.m.text),
                Tone::Other | Tone::Own => tf(kit.fonts, kit.m.text),
            };
            let ink = match tone {
                Tone::Head => tokens::MUTED,
                Tone::Own if name => tokens::ACCENT,
                Tone::Other | Tone::Own => tokens::INK,
            };
            let text = if tone == Tone::Head {
                text.to_uppercase()
            } else {
                text.to_string()
            };
            let cell = commands
                .spawn((
                    Text::new(text),
                    font,
                    TextColor(ink),
                    TextLayout::no_wrap().with_justify(if index < 2 {
                        Justify::Left
                    } else {
                        Justify::Right
                    }),
                    Node {
                        width: if name { Val::Auto } else { kit.m.px(width) },
                        flex_grow: if name { 1.0 } else { 0.0 },
                        flex_shrink: if name { 1.0 } else { 0.0 },
                        flex_basis: if name { px_fixed(0.0) } else { Val::Auto },
                        min_width: px_fixed(0.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(line).add_child(cell);
        }
        commands.entity(root).add_child(line);
    }
}

/// Formats a challenge's supplied deck list for players by stripping raw
/// internal `[deck:…]` headers and comment lines, and rendering section markers
/// as plain headings.
fn supplied_deck_text(raw: &str, lang: Lang) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if out.last().is_some_and(|prev| !prev.is_empty()) {
                out.push("");
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let tag = inner.trim().to_ascii_lowercase();
            if tag == "deck" || tag.starts_with("deck:") {
                continue;
            }
            if tag == "commander" {
                if out.last().is_some_and(|prev| !prev.is_empty()) {
                    out.push("");
                }
                out.push(Phrase::BuildCommander.text(lang));
                continue;
            }
            if tag == "sideboard" {
                if out.last().is_some_and(|prev| !prev.is_empty()) {
                    out.push("");
                }
                out.push(Phrase::BuildTabSide.text(lang));
                continue;
            }
        }
        out.push(trimmed);
    }
    while out.last().is_some_and(|line| line.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_deck_text_strips_internal_deck_header_and_formats_sections() {
        let raw = "[deck:Crossroads]\n24 Mountain\n16 Forest\n4 Grizzly Bears\n";
        assert_eq!(
            supplied_deck_text(raw, Lang::En),
            "24 Mountain\n16 Forest\n4 Grizzly Bears"
        );
        assert_eq!(
            supplied_deck_text("4 Lightning Bolt", Lang::En),
            "4 Lightning Bolt"
        );
        let with_sections = "# comment\n[deck:Test]\n\n4 Lightning Bolt\n\n[sideboard]\n2 Abrade\n[commander]\n1 Ragavan\n";
        assert_eq!(
            supplied_deck_text(with_sections, Lang::En),
            format!(
                "4 Lightning Bolt\n\n{}\n2 Abrade\n\n{}\n1 Ragavan",
                Phrase::BuildTabSide.text(Lang::En),
                Phrase::BuildCommander.text(Lang::En),
            )
        );
    }
}
