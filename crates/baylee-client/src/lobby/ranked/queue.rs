//! A single displayed deck choice, explicit queue actions, and the last settled result.
use super::*;
use crate::hud::glyph;
use api::{Outcome, QueueState};
use baylee_client_core::lobby::DeckSummary;

fn source_format(format: Format) -> &'static str {
    match format {
        Format::Standard => "standard",
        Format::Highlander => "highlander",
        Format::Commander => "commander",
    }
}
pub(super) fn effective_format(state: &LobbyState) -> Format {
    if state.ranked_net.needs_deck {
        match &state.ranked_net.queue {
            QueueState::Assigned { assignment, .. } => return assignment.format,
            QueueState::Waiting { format, .. } => return *format,
            _ => {}
        }
    }
    state.ranked.format
}
fn choices(state: &LobbyState) -> impl Iterator<Item = (uuid::Uuid, &DeckSummary)> {
    let f = source_format(effective_format(state));
    state
        .lobby
        .decks()
        .iter()
        .filter(move |d| d.format == f && !d.legality.illegal())
        .filter_map(|d| uuid::Uuid::parse_str(&d.id).ok().map(|id| (id, d)))
}
pub(super) fn selected(state: &LobbyState) -> Option<(uuid::Uuid, &DeckSummary)> {
    let chosen = state.ranked.deck_id;
    choices(state)
        .find(|(id, _)| Some(*id) == chosen)
        .or_else(|| choices(state).next())
}

#[derive(PartialEq, Eq)]
pub(super) struct Look {
    format: Format,
    eligible: bool,
    held: bool,
    busy: bool,
    recover: bool,
    played: Option<api::Id>,
    error: Option<baylee_account::Error>,
    queue: QueueState,
    deck: Option<uuid::Uuid>,
    picker: bool,
    deck_page: usize,
    own: Option<(api::Rank, u8, Option<i32>)>,
}

pub(super) fn look(state: &LobbyState, own: Option<(api::Rank, u8, Option<i32>)>) -> Look {
    Look {
        format: effective_format(state),
        eligible: state.central_ready() && !state.lobby.guest() && state.can_play_online(),
        held: state.lobby.awaiting().is_some(),
        busy: state.ranked_net.busy,
        recover: state.ranked_net.needs_deck,
        played: state.ranked_net.played,
        error: state.ranked_net.error,
        queue: state.ranked_net.queue.clone(),
        deck: state.ranked.deck_id,
        picker: state.ranked.show_decks,
        deck_page: state.ranked.deck_page,
        own,
    }
}

#[derive(Component)]
pub(super) struct WaitingSince {
    start: i64,
    seconds: Option<u64>,
}
#[derive(Component)]
pub(super) struct Pulse(u8);
const DECK_PAGE: usize = 6;

pub(super) fn draw(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    state: &LobbyState,
    look: &Look,
) {
    use draw::label;
    let lang = kit.lang();
    let title = parts::marked_line(
        commands,
        kit,
        glyph::BULLSEYE,
        Phrase::RankedQueue.text(lang),
        kit.m.head,
        tokens::INK,
    );
    commands.entity(parent).add_child(title);
    if let Some((rank, placements, elo)) = look.own {
        let group = parts::row(commands, kit, true);
        commands.entity(parent).add_child(group);
        let badge = commands
            .spawn((
                Node {
                    width: kit.m.px(42.0),
                    height: kit.m.px(42.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        if let Some(art) = crate::rank_art::RankArt::badge(
            league_name(rank.league, Lang::En),
            rank.division,
            false,
            rank.champion_place.map(u32::from),
        ) {
            commands.entity(badge).insert(art);
        }
        commands.entity(group).add_child(badge);
        label(
            commands,
            group,
            kit,
            &rank_name(rank, lang),
            kit.m.text,
            tokens::INK,
        );
        let progress = if rank.league == League::Unranked {
            Phrase::RankedPlacements.fill(lang, &[&placements.to_string()])
        } else {
            elo.map_or_else(String::new, |value| {
                Phrase::RankedElo.fill(lang, &[&value.to_string()])
            })
        };
        label(commands, group, kit, &progress, kit.m.small, tokens::ACCENT);
    }
    if !look.eligible {
        label(
            commands,
            parent,
            kit,
            if state.central_ready() && !state.can_play_online() {
                Phrase::RankedConnect
            } else {
                Phrase::RankedSignIn
            }
            .text(lang),
            kit.m.text,
            tokens::MUTED,
        );
        return;
    }
    if let Some(err) = look.error {
        let why = match err {
            baylee_account::Error::Unavailable => Phrase::RankedServiceError,
            baylee_account::Error::Busy => Phrase::RankedBusy,
            baylee_account::Error::Conflict => Phrase::RankedDeckChanged,
            baylee_account::Error::Rejected => Phrase::RankedRejected,
            baylee_account::Error::UpdateRequired => Phrase::RankedUpdate,
            baylee_account::Error::TermsRequired => Phrase::RankedTerms,
        };
        label(
            commands,
            parent,
            kit,
            why.text(lang),
            kit.m.small,
            tokens::DANGER,
        );
        draw::btn(
            commands,
            parent,
            kit,
            Phrase::ShellRetry.text(lang),
            Action::Refresh,
            ("queue", 2),
            false,
        );
    }
    match &look.queue {
        QueueState::Idle => {}
        QueueState::Waiting {
            format, queued_at, ..
        } => {
            let title = Phrase::RankedSearching.fill(lang, &[format_name(*format, lang)]);
            label(commands, parent, kit, &title, kit.m.text, tokens::ACCENT);
            let row = parts::row(commands, kit, false);
            commands.entity(parent).add_child(row);
            for idx in 0..3 {
                let dot = commands
                    .spawn((
                        Pulse(idx),
                        Node {
                            width: kit.m.px(6.0),
                            height: kit.m.px(6.0),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(tokens::ACCENT),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(row).add_child(dot);
            }
            let clock = parts::line(commands, kit, "", kit.m.small, tokens::MUTED);
            commands.entity(clock).insert(WaitingSince {
                start: *queued_at,
                seconds: None,
            });
            commands.entity(row).add_child(clock);
            if !look.busy {
                draw::btn(
                    commands,
                    parent,
                    kit,
                    Phrase::RankedCancel.text(lang),
                    Action::Cancel,
                    ("queue", 1),
                    false,
                );
            }
            if !look.recover {
                return;
            }
        }
        QueueState::Assigned { assignment, .. } => {
            let words = Phrase::RankedAssigned.fill(lang, &[format_name(assignment.format, lang)]);
            label(commands, parent, kit, &words, kit.m.text, tokens::ACCENT);
            if look.played == Some(assignment.match_id) && !look.busy {
                draw::btn(
                    commands,
                    parent,
                    kit,
                    Phrase::RankedResume.text(lang),
                    Action::Resume,
                    ("queue", 0),
                    true,
                );
            }
            if !look.recover {
                return;
            }
        }
        QueueState::Finished {
            deck,
            outcome,
            seat,
            elo,
            delta,
            format,
            after,
            ..
        } => {
            let result = match (outcome, seat) {
                (Outcome::FirstWins, 0) | (Outcome::SecondWins, 1) => Phrase::RankedWon,
                (Outcome::Draw, _) => Phrase::RankedDrawn,
                _ => Phrase::RankedLost,
            };
            let receipt = Phrase::RankedReceipt.fill(
                lang,
                &[
                    format_name(*format, lang),
                    result.text(lang),
                    &format!("{delta:+}"),
                    &elo.to_string(),
                ],
            );
            label(commands, parent, kit, &receipt, kit.m.text, tokens::GOLD);
            let progress = if deck.after.league == League::Unranked {
                Phrase::RankedPlacements.fill(lang, &[&deck.placement_games.to_string()])
            } else {
                rank_name(deck.after, lang)
            };
            let deck_receipt = Phrase::RankedDeckResult.fill(
                lang,
                &[
                    &format!("{:+}", deck.delta),
                    &deck.elo.to_string(),
                    &progress,
                ],
            );
            label(
                commands,
                parent,
                kit,
                &deck_receipt,
                kit.m.small,
                tokens::ACCENT,
            );
            label(
                commands,
                parent,
                kit,
                &rank_name(*after, lang),
                kit.m.small,
                tokens::MUTED,
            );
        }
    }
    if look.recover {
        label(
            commands,
            parent,
            kit,
            Phrase::RankedRecover.text(lang),
            kit.m.text,
            tokens::ACCENT,
        );
    }
    let selection = selected(state);
    if let Some((_, d)) = selection {
        let line = parts::caption(commands, kit, Phrase::RankedChosenDeck.text(lang));
        commands.entity(parent).add_child(line);
        let picker = parts::menu_button(
            commands,
            kit,
            &d.name,
            Weight::Secondary,
            Press::Ranked(Action::ToggleDecks),
        );
        orders::stop(commands, picker, &ORDER, "deck");
        commands.entity(parent).add_child(picker);
        if look.picker {
            let decks = choices(state).collect::<Vec<_>>();
            let page = look
                .deck_page
                .min(decks.len().saturating_sub(1) / DECK_PAGE);
            for (idx, (id, d)) in decks
                .iter()
                .skip(page * DECK_PAGE)
                .take(DECK_PAGE)
                .enumerate()
            {
                draw::btn(
                    commands,
                    parent,
                    kit,
                    &d.name,
                    Action::Deck(*id),
                    ("deck-choice", idx),
                    Some(*id) == selection.map(|(a, _)| a),
                );
            }
            if decks.len() > DECK_PAGE {
                let row = parts::row(commands, kit, true);
                commands.entity(parent).add_child(row);
                if page > 0 {
                    draw::btn(
                        commands,
                        row,
                        kit,
                        Phrase::ChallengePrevious.text(lang),
                        Action::DeckPage(page - 1),
                        ("deck-choice", DECK_PAGE),
                        false,
                    );
                }
                if (page + 1) * DECK_PAGE < decks.len() {
                    draw::btn(
                        commands,
                        row,
                        kit,
                        Phrase::ChallengeNext.text(lang),
                        Action::DeckPage(page + 1),
                        ("deck-choice", DECK_PAGE + 1),
                        false,
                    );
                }
            }
        }
    }
    let reason = if look.held {
        Some(Phrase::ChallengeLeaveTable)
    } else if selection.is_none() {
        Some(Phrase::RankedNoDeck)
    } else if look.busy {
        Some(Phrase::RankedJoining)
    } else {
        None
    };
    let join = controls::button_with_icon(
        commands,
        kit,
        glyph::BULLSEYE,
        if look.recover {
            Phrase::RankedRestore
        } else {
            Phrase::RankedStart
        }
        .text(lang),
        Weight::Primary,
        reason.map_or(Live::Yes, |r| Live::No(r.text(lang))),
        None,
        Press::Ranked(Action::Join),
    );
    if reason.is_none() {
        orders::stop(commands, join, &ORDER, "queue");
    }
    commands.entity(parent).add_child(join);
    if let Some(r) = reason {
        label(
            commands,
            parent,
            kit,
            r.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
    }
    label(
        commands,
        parent,
        kit,
        &Phrase::RankedPublicHelp.fill(lang, &[format_name(look.format, lang)]),
        kit.m.small,
        tokens::MUTED,
    );
    label(
        commands,
        parent,
        kit,
        Phrase::RankedPlacementHelp.text(lang),
        kit.m.small,
        tokens::MUTED,
    );
}

pub(super) fn animate(
    time: Res<Time>,
    prefs: Res<crate::prefs::Prefs>,
    state: Res<LobbyState>,
    mut clocks: Query<(&mut WaitingSince, &mut Text)>,
    mut dots: Query<(&Pulse, &mut BackgroundColor)>,
) {
    if clocks.is_empty() {
        return;
    }
    let now = parts::now_secs();
    for (mut marker, mut text) in &mut clocks {
        let seconds = now.saturating_sub(u64::try_from(marker.start).unwrap_or(now));
        if marker.seconds != Some(seconds) {
            marker.seconds = Some(seconds);
            text.0 = Phrase::RankedElapsed.fill(
                state.lobby.lang(),
                &[&format!("{}:{:02}", seconds / 60, seconds % 60)],
            );
        }
    }
    let still = prefs.all().reduce_motion;
    for (dot, mut color) in &mut dots {
        let next = tokens::ACCENT.with_alpha(if still {
            0.9
        } else {
            0.6 + 0.35 * (time.elapsed_secs() * 4.0 - f32::from(dot.0)).sin()
        });
        if color.0 != next {
            color.0 = next;
        }
    }
}
