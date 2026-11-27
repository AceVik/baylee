//! A single displayed deck choice, explicit queue actions, and the last settled result.
use super::draw::{btn, label};
use super::{
    Action, Format, Kit, Lang, League, Live, LobbyState, OwnRank, Phrase, Press, Weight, api,
    controls, format_name, league_name, orders, parts, preview, rank_name, tokens,
};
use crate::hud::glyph;
use api::{Outcome, QueueState};
use baylee_client_core::aiplayers::Opponents;
use baylee_client_core::lobby::DeckSummary;
use bevy::prelude::*;

pub(super) fn effective_format(state: &LobbyState) -> Format {
    match state.ranked_net.shown_queue() {
        QueueState::Assigned { assignment, .. } => assignment.format,
        QueueState::Waiting { format, .. } => *format,
        _ => state.ranked.queue_format,
    }
}
fn choices(state: &LobbyState) -> impl Iterator<Item = (uuid::Uuid, &DeckSummary)> {
    let f = effective_format(state).as_str();
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
#[allow(clippy::struct_excessive_bools)] // independent worker, table and picker states
pub(super) struct Look {
    waiting_consent: bool,
    format: Format,
    pub(super) eligible: bool,
    held: bool,
    busy: bool,
    recover: bool,
    played: Option<api::Id>,
    error: Option<baylee_account::Error>,
    pub(super) queue: QueueState,
    pub(super) deck: Option<(uuid::Uuid, String)>,
    // Only the six visible deck labels and the next-page sentinel can affect the picker.
    options: Vec<(uuid::Uuid, String)>,
    picker: bool,
    deck_page: usize,
    own: Option<OwnRank>,
    preview: preview::Look,
    /// Whom the account lets fill its searches; `None` until it chooses.
    pub(super) opponents: Option<Opponents>,
    /// Whether the settled game shown was a first win against its AI.
    first_win: bool,
}

/// The queue as the desk draws it. The countdown and the search's clock
/// are patched in place (`aidesk::animate`), so neither a new fill-in time
/// nor a search moving to another choice redraws the desk.
fn drawn_queue(state: &LobbyState) -> QueueState {
    let mut queue = state.ranked_net.shown_queue().clone();
    if let QueueState::Waiting {
        ai_fill_in_at,
        queued_at,
        key,
        ..
    } = &mut queue
    {
        *ai_fill_in_at = None;
        *queued_at = 0;
        *key = [0; 16];
    }
    queue
}

pub(super) fn look(state: &LobbyState, own: Option<OwnRank>) -> Look {
    Look {
        waiting_consent: state.ranked_net.waiting_consent(),
        format: effective_format(state),
        eligible: state.has_central_account(),
        held: state.lobby.awaiting().is_some(),
        // A search moving to another choice is still the search it was.
        busy: state.ranked_net.busy && !state.ranked_net.switching(),
        recover: state.ranked_net.needs_deck,
        played: state.ranked_net.played,
        error: state.ranked_net.error,
        opponents: super::choice::current(state),
        first_win: matches!(
            state.ranked_net.shown_queue(),
            QueueState::Finished { match_id, .. } if state.ranked.first_win == Some(*match_id)
        ),
        queue: drawn_queue(state),
        deck: selected(state).map(|(id, d)| (id, d.name.clone())),
        options: if state.ranked.show_decks {
            choices(state)
                .skip(state.ranked.deck_page.saturating_mul(DECK_PAGE))
                .take(DECK_PAGE + 1)
                .map(|(id, d)| (id, d.name.clone()))
                .collect()
        } else {
            Vec::new()
        },
        picker: state.ranked.show_decks,
        deck_page: state.ranked.deck_page,
        own,
        preview: preview::look(state),
    }
}

/// The search's clock, and the seconds it last wrote.
#[derive(Component)]
pub(super) struct WaitingSince {
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
    own_rank(commands, parent, kit, look);
    if !look.eligible {
        label(
            commands,
            parent,
            kit,
            Phrase::RankedSignIn.text(lang),
            kit.m.text,
            tokens::MUTED,
        );
        return;
    }
    error(commands, parent, kit, look);
    if !show_status(commands, parent, kit, look) {
        return;
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
    deck_picker(commands, parent, kit, state, look);
    if look.deck.is_some() && !look.recover {
        preview::draw(commands, parent, kit, &look.preview);
        super::aidesk::opponents(commands, parent, kit, look.opponents);
    }
    join_button(commands, parent, kit, look);
}

/// The account's standing in the format: its badge, rank and Elo, the
/// season's record, and while it is placed a bar of the ten placement games.
fn own_rank(commands: &mut Commands, parent: Entity, kit: Kit, look: &Look) {
    let lang = kit.lang();
    let Some(OwnRank {
        rank,
        placements,
        elo,
        record,
        career,
    }) = look.own
    else {
        return;
    };
    let group = parts::row(commands, kit, false);
    commands.entity(group).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Percent(100.0);
    });
    commands.entity(parent).add_child(group);
    let badge = commands
        .spawn((
            Node {
                width: kit.m.px(56.0),
                height: kit.m.px(56.0),
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
    let words = super::draw::column(commands, group, kit);
    commands.entity(words).entry::<Node>().and_modify(|mut n| {
        n.flex_grow = 1.0;
        n.row_gap = Val::Px(2.0);
    });
    let name = parts::line(
        commands,
        kit,
        &rank_name(rank, lang),
        kit.m.head,
        tokens::INK,
    );
    commands
        .entity(name)
        .insert(crate::hud::tf_bold(kit.fonts, kit.m.head));
    commands.entity(words).add_child(name);
    let placing = rank.league == League::Unranked;
    let figures = if placing {
        Phrase::RankedPlacements.fill(lang, &[&placements.to_string()])
    } else {
        let elo = elo.map(|value| Phrase::RankedElo.fill(lang, &[&value.to_string()]));
        let record = record.map(|(wins, losses, draws)| {
            Phrase::RankedRecord.fill(
                lang,
                &[&wins.to_string(), &losses.to_string(), &draws.to_string()],
            )
        });
        elo.into_iter()
            .chain(record)
            .collect::<Vec<_>>()
            .join(" · ")
    };
    if !figures.is_empty() {
        label(commands, words, kit, &figures, kit.m.small, tokens::ACCENT);
    }
    if placing {
        let bar = parts::meter(
            commands,
            kit,
            f32::from(placements.min(10)) / 10.0,
            Val::Percent(100.0),
            tokens::ACCENT,
        );
        commands.entity(words).add_child(bar);
    }
    figures_row(commands, parent, kit, record, career);
}

/// The standing's figures in three equal cells under the rank (owner,
/// 25.11.2026: "Statistiken"): the season's win rate, its games, and the
/// career's games over every season. Only once a game has been played: a
/// row of dashes says nothing the placement meter does not.
fn figures_row(
    commands: &mut Commands,
    parent: Entity,
    kit: Kit,
    record: Option<(u32, u32, u32)>,
    career: Option<api::Record>,
) {
    let lang = kit.lang();
    let season = record.map_or(0, |(wins, losses, draws)| {
        wins.saturating_add(losses).saturating_add(draws)
    });
    let career = career.map_or(season, |career| career.games.max(season));
    if career == 0 {
        return;
    }
    let row = parts::row(commands, kit, false);
    commands.entity(row).entry::<Node>().and_modify(|mut n| {
        n.width = Val::Percent(100.0);
        n.flex_wrap = FlexWrap::NoWrap;
    });
    commands.entity(parent).add_child(row);
    let rate = record
        .and_then(|(wins, _, _)| super::win_rate(wins, season))
        .map_or_else(
            || "—".to_owned(),
            |rate| {
                if lang == Lang::De {
                    format!("{rate}\u{a0}%")
                } else {
                    format!("{rate}%")
                }
            },
        );
    for (icon, value, name) in [
        (glyph::PERCENT, rate, Phrase::RankedStatWinRate),
        (glyph::FINISH, season.to_string(), Phrase::RankedStatSeason),
        (glyph::HISTORY, career.to_string(), Phrase::RankedStatCareer),
    ] {
        parts::stat(commands, row, kit, icon, &value, name.text(lang));
    }
}

fn error(commands: &mut Commands, parent: Entity, kit: Kit, look: &Look) {
    let lang = kit.lang();
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
        btn(
            commands,
            parent,
            kit,
            Phrase::ShellRetry.text(lang),
            Action::Refresh,
            (&super::ORDER, "queue", 2),
            false,
        );
    }
}

fn show_status(commands: &mut Commands, parent: Entity, kit: Kit, look: &Look) -> bool {
    let lang = kit.lang();
    match &look.queue {
        QueueState::Idle => {}
        QueueState::Waiting { format, .. } => {
            waiting(commands, parent, kit, *format);
            // The countdown's room stands from the first second, so its
            // coming and going moves nothing.
            super::aidesk::fill_in_slot(commands, parent, kit);
            // Leave stays where it is while its answer comes, greyed.
            let leave = controls::button_compact(
                commands,
                kit,
                None,
                Phrase::RankedCancel.text(lang),
                Weight::Secondary,
                if look.busy {
                    Live::No(Phrase::RankedLeaving.text(lang))
                } else {
                    Live::Yes
                },
                Press::Ranked(Action::Cancel),
            );
            if !look.busy {
                orders::item(commands, leave, &super::ORDER, "queue", 1);
            }
            commands.entity(parent).add_child(leave);
            super::aidesk::opponents(commands, parent, kit, look.opponents);
            if !look.recover {
                return false;
            }
        }
        QueueState::Assigned { assignment, .. } => {
            let words = if look.waiting_consent {
                Phrase::RankedHostsWaiting.text(lang).to_owned()
            } else {
                Phrase::RankedAssigned.fill(lang, &[format_name(assignment.format, lang)])
            };
            label(commands, parent, kit, &words, kit.m.text, tokens::ACCENT);
            if let Some(badge) = &assignment.opponent_ai {
                super::aidesk::opponent_line(commands, parent, kit, Phrase::RankedOpponent, badge);
            }
            if look.played == Some(assignment.match_id) && !look.busy {
                btn(
                    commands,
                    parent,
                    kit,
                    Phrase::RankedResume.text(lang),
                    Action::Resume,
                    (&super::ORDER, "queue", 0),
                    true,
                );
            }
            if !look.recover {
                return false;
            }
        }
        QueueState::Finished {
            deck,
            outcome,
            seat,
            elo,
            delta,
            format,
            before,
            after,
            opponent_ai,
            ..
        } => {
            let deck_progress = if deck.after.league == League::Unranked {
                Phrase::RankedPlacements.fill(lang, &[&deck.placement_games.to_string()])
            } else {
                rank_name(deck.after, lang)
            };
            let deck_receipt = Phrase::RankedDeckResult.fill(
                lang,
                &[
                    &format!("{:+}", deck.delta),
                    &deck.elo.to_string(),
                    &deck_progress,
                ],
            );
            result_card(
                commands,
                parent,
                kit,
                Settled {
                    format: *format,
                    result: result(*outcome, *seat),
                    elo: *elo,
                    delta: *delta,
                    before: *before,
                    after: *after,
                    deck: &deck_receipt,
                    opponent: opponent_ai.as_ref(),
                    first_win: look.first_win,
                },
            );
        }
    }

    true
}

/// A settled game from this seat's side.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Won,
    Lost,
    Drawn,
}

fn result(outcome: Outcome, seat: u8) -> Verdict {
    match (outcome, seat) {
        (Outcome::FirstWins, 0) | (Outcome::SecondWins, 1) => Verdict::Won,
        (Outcome::Draw, _) => Verdict::Drawn,
        _ => Verdict::Lost,
    }
}

impl Verdict {
    fn phrase(self) -> Phrase {
        match self {
            Self::Won => Phrase::RankedWon,
            Self::Lost => Phrase::RankedLost,
            Self::Drawn => Phrase::RankedDrawn,
        }
    }

    /// Its glyph and ink: the cup in gold, the cross in danger, the
    /// handshake in muted ink.
    fn look(self) -> (char, Color) {
        match self {
            Self::Won => (glyph::TROPHY, tokens::GOLD),
            Self::Lost => (glyph::CLOSE, tokens::DANGER),
            Self::Drawn => ('\u{f2b5}', tokens::MUTED),
        }
    }
}

pub(super) fn receipt(
    lang: Lang,
    format: Format,
    outcome: Outcome,
    seat: u8,
    elo: i32,
    delta: i32,
) -> String {
    Phrase::RankedReceipt.fill(
        lang,
        &[
            format_name(format, lang),
            result(outcome, seat).phrase().text(lang),
            &format!("{delta:+}"),
            &elo.to_string(),
        ],
    )
}

/// What the result card says about a settled game.
struct Settled<'a> {
    format: Format,
    result: Verdict,
    elo: i32,
    delta: i32,
    before: api::Rank,
    after: api::Rank,
    /// The deck's line, already in words.
    deck: &'a str,
    /// The opponent's badge, when it was an AI player.
    opponent: Option<&'a baylee_client_core::aiplayers::AiBadge>,
    /// Whether it was the first win against that AI.
    first_win: bool,
}

/// A rank's small badge, as the result card shows it before and after.
fn rank_badge(commands: &mut Commands, kit: Kit, rank: api::Rank) -> Entity {
    let node = commands
        .spawn((
            Node {
                width: kit.m.px(34.0),
                height: kit.m.px(34.0),
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
        commands.entity(node).insert(art);
    }
    node
}

/// The last settled game as a card: the result in its colour, the rank
/// before and after it, the Elo it moved and where it stands, the deck's line.
fn result_card(commands: &mut Commands, parent: Entity, kit: Kit, settled: Settled<'_>) {
    let lang = kit.lang();
    let (icon, ink) = settled.result.look();
    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.space(1),
                padding: UiRect::all(kit.m.space(2)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(tokens::CONTROL.with_alpha(0.45)),
            BorderColor::all(ink.with_alpha(0.55)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(parent).add_child(card);
    let head = parts::row(commands, kit, false);
    commands.entity(card).add_child(head);
    let title = parts::marked_line(
        commands,
        kit,
        icon,
        settled.result.phrase().text(lang),
        kit.m.head,
        ink,
    );
    let grow = parts::grow(commands);
    let format = parts::line(
        commands,
        kit,
        format_name(settled.format, lang),
        kit.m.small,
        tokens::MUTED,
    );
    commands.entity(head).add_children(&[title, grow, format]);
    let line = parts::row(commands, kit, false);
    commands.entity(card).add_child(line);
    let from = rank_badge(commands, kit, settled.before);
    let arrow = commands
        .spawn((
            Text::new(glyph::ARROW_RIGHT.to_string()),
            crate::hud::icon_tf(kit.fonts, kit.m.small),
            TextColor(tokens::MUTED),
            Pickable::IGNORE,
        ))
        .id();
    let to = rank_badge(commands, kit, settled.after);
    let name = parts::line(
        commands,
        kit,
        &rank_name(settled.after, lang),
        kit.m.text,
        tokens::INK,
    );
    let grow = parts::grow(commands);
    let moved = match settled.delta.cmp(&0) {
        std::cmp::Ordering::Greater => tokens::GAIN,
        std::cmp::Ordering::Less => tokens::DANGER,
        std::cmp::Ordering::Equal => tokens::MUTED,
    };
    let delta = parts::badge(
        commands,
        kit,
        &format!(
            "{:+} · {}",
            settled.delta,
            Phrase::RankedElo.fill(lang, &[&settled.elo.to_string()])
        ),
        moved,
    );
    commands
        .entity(line)
        .add_children(&[from, arrow, to, name, grow, delta]);
    if let Some(badge) = settled.opponent {
        super::aidesk::opponent_line(commands, card, kit, Phrase::RankedAgainst, badge);
        // The first win against this AI, as this device counted it.
        if settled.first_win {
            let mark = parts::marked_line(
                commands,
                kit,
                glyph::TROPHY,
                Phrase::RankedFirstWin.text(lang),
                kit.m.small,
                tokens::GOLD,
            );
            commands.entity(card).add_child(mark);
        }
    }
    label(
        commands,
        card,
        kit,
        settled.deck,
        kit.m.small,
        tokens::ACCENT,
    );
}

/// The search's head: what is searched for, three breathing dots and how
/// long it has run (`aidesk::animate` writes the clock, from the start of
/// the person's search, which a move to another choice keeps).
fn waiting(commands: &mut Commands, parent: Entity, kit: Kit, format: Format) {
    let lang = kit.lang();
    let title = Phrase::RankedSearching.fill(lang, &[format_name(format, lang)]);
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
    commands
        .entity(clock)
        .insert((WaitingSince { seconds: None }, super::aidesk::counting(kit)));
    commands.entity(row).add_child(clock);
}

fn deck_picker(commands: &mut Commands, parent: Entity, kit: Kit, state: &LobbyState, look: &Look) {
    let lang = kit.lang();
    if let Some((selected, name)) = &look.deck {
        let line = parts::caption(commands, kit, Phrase::RankedChosenDeck.text(lang));
        commands.entity(parent).add_child(line);
        let picker = parts::menu_button(
            commands,
            kit,
            name,
            Weight::Secondary,
            Press::Ranked(Action::ToggleDecks),
        );
        orders::stop(commands, picker, &super::ORDER, "deck");
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
                btn(
                    commands,
                    parent,
                    kit,
                    &d.name,
                    Action::Deck(*id),
                    (&super::ORDER, "deck-choice", idx),
                    id == selected,
                );
            }
            if decks.len() > DECK_PAGE {
                let row = parts::row(commands, kit, true);
                commands.entity(parent).add_child(row);
                if page > 0 {
                    btn(
                        commands,
                        row,
                        kit,
                        Phrase::ChallengePrevious.text(lang),
                        Action::DeckPage(page - 1),
                        (&super::ORDER, "deck-choice", DECK_PAGE),
                        false,
                    );
                }
                if (page + 1) * DECK_PAGE < decks.len() {
                    btn(
                        commands,
                        row,
                        kit,
                        Phrase::ChallengeNext.text(lang),
                        Action::DeckPage(page + 1),
                        (&super::ORDER, "deck-choice", DECK_PAGE + 1),
                        false,
                    );
                }
            }
        }
    }
}

fn join_button(commands: &mut Commands, parent: Entity, kit: Kit, look: &Look) {
    let lang = kit.lang();
    if look.deck.is_none() && !look.held {
        // No deck to play yet: the desk says so in its middle and leads to
        // the deck library, instead of a grey button and a line beside it.
        let to_decks = controls::button_with_icon(
            commands,
            kit,
            glyph::LIBRARY,
            Phrase::ShellDecks.text(lang),
            Weight::Secondary,
            Live::Yes,
            None,
            Press::Hub(crate::lobby::HubPress::Tab(crate::lobby::Hub::Decks)),
        );
        orders::stop(commands, to_decks, &super::ORDER, "queue");
        let empty = crate::shellkit::states::empty(
            commands,
            kit,
            glyph::LIBRARY,
            Phrase::RankedNoDeck.text(lang),
            Some(to_decks),
        );
        commands.entity(parent).add_child(empty);
    } else {
        let reason = if look.held {
            Some(Phrase::ChallengeLeaveTable)
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
            orders::stop(commands, join, &super::ORDER, "queue");
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
    }
    label(
        commands,
        parent,
        kit,
        &Phrase::RankedPublicHelp.fill(lang, &[format_name(look.format, lang)]),
        kit.m.small,
        tokens::MUTED,
    );
    // How placement works matters while one is being placed, not after.
    if look
        .own
        .is_none_or(|own| own.rank.league == League::Unranked)
    {
        label(
            commands,
            parent,
            kit,
            Phrase::RankedPlacementHelp.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
    }
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
    // From the start of the person's search on Central's clock: a move to
    // another choice keeps it, and a clock that runs off Central's does not
    // shift it.
    let now = state.ranked_net.central_now();
    let since = state.ranked_net.searching_since().unwrap_or(now);
    let seconds = u64::try_from(now.saturating_sub(since)).unwrap_or(0);
    for (mut marker, mut text) in &mut clocks {
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
