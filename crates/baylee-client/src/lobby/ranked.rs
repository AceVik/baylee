//! The public seasonal ladder and Central's cross-gateway matchmaking desk.
use super::{
    Hub, Lang, List, LobbyRequest, LobbyState, Metrics, Phrase, Press, Scrolled, orders, parts,
    press::Cx,
};
use crate::shellkit::{
    controls::{self, Kit, Live, Weight},
    focus::TabOrder,
    tokens,
};
use baylee_account::ranked::{self as api, Cursor, Format, League, Track};
use bevy::prelude::*;

mod board;
mod draw;
pub(crate) mod native;
mod queue;
mod receipt;
pub(super) use draw::draw;

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "ranked",
    stops: &[
        "back",
        "format",
        "deck",
        "deck-choice",
        "queue",
        "track",
        "league",
        "league-choice",
        "division",
        "refresh",
        "rows",
        "pager",
        "baylee",
    ],
    modal: false,
};

const FORMATS: [Format; 3] = [Format::Standard, Format::Highlander, Format::Commander];
const LEAGUES: [League; 12] = [
    League::Unranked,
    League::Paper,
    League::Wood,
    League::Iron,
    League::Bronze,
    League::Silver,
    League::Gold,
    League::Platinum,
    League::Diamond,
    League::Master,
    League::Grandmaster,
    League::Champion,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Back,
    Format(Format),
    Track(Track),
    ToggleLeagues,
    League(League),
    Division(Option<u8>),
    ToggleDecks,
    DeckPage(usize),
    Deck(uuid::Uuid),
    Join,
    Cancel,
    Resume,
    Refresh,
    Page(bool),
}

pub(crate) struct Ui {
    format: Format,
    track: Track,
    league: League,
    division: Option<u8>,
    deck_id: Option<uuid::Uuid>,
    show_decks: bool,
    deck_page: usize,
    show_leagues: bool,
    // Pin subsequent cursor pages to the season that produced the first page.
    season: Option<api::Season>,
    // For Back, store the cursor used for each visited page and its ordinal offset.
    pages: Vec<(Option<Cursor>, usize)>,
    previous: Hub,
    revision: u64,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            format: Format::Standard,
            track: Track::Player,
            league: League::Champion,
            division: None,
            deck_id: None,
            show_decks: false,
            show_leagues: false,
            season: None,
            pages: Vec::new(),
            previous: Hub::Play,
            deck_page: 0,
            revision: 0,
        }
    }
}

impl Ui {
    fn board_request(&self) -> api::Request {
        api::Request::Board {
            season: self.season.as_ref().map(|s| s.id),
            format: self.format,
            track: self.track,
            league: self.league,
            division: self.division,
            after: self.pages.last().and_then(|(c, _)| *c),
        }
    }
    fn reset_pages(&mut self) {
        self.season = None;
        self.pages.clear();
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        native::poll
            .after(super::poll)
            .before(super::ui)
            .run_if(in_state(crate::DuelPhase::Closed)),
    );
    app.add_systems(
        Update,
        (draw::patch, queue::animate, receipt::promotion)
            .chain()
            .after(super::ui)
            .run_if(in_state(crate::DuelPhase::Closed)),
    );
}

pub(super) fn refresh(state: &mut LobbyState) {
    let request = state.ranked.board_request();
    native::board(state, request);
    if state.central_ready() {
        native::refresh(state);
    }
}
pub(super) fn escape(state: &mut ResMut<LobbyState>) {
    if state.ranked.show_decks || state.ranked.show_leagues {
        let ui = &mut state.bypass_change_detection().ranked;
        ui.show_decks = false;
        ui.show_leagues = false;
        ui.revision = ui.revision.wrapping_add(1);
    } else {
        state.hub = state.ranked.previous;
    }
}

pub(super) fn press(action: Action, cx: Cx<'_, '_, '_, '_, '_>) {
    if action == Action::Open {
        if cx.state.hub == Hub::Ranked {
            return;
        }
        let previous = cx.state.hub;
        cx.state.ranked.previous = previous;
        cx.state.hub = Hub::Ranked;
        cx.state.header_menu = None;
        cx.state.front_menu = false;
        cx.state.lobby.park_caret();
        if cx.state.can_play_online() && cx.state.central_ready() {
            super::dispatch(cx.state, cx.mailbox, Some(LobbyRequest::ListDecks));
        }
        cx.state.ranked.reset_pages();
        refresh(cx.state);
        return;
    }
    if cx.state.hub != Hub::Ranked {
        return;
    }
    if action == Action::Back {
        let previous = cx.state.ranked.previous;
        cx.state.hub = previous;
        return;
    }
    // A filter, a page and the network mailbox are local to the retained page.
    let state = cx.state.bypass_change_detection();
    let ui = &mut state.ranked;
    let mut reset_pages = false;
    match action {
        Action::Open | Action::Back => return,
        Action::Format(format) if ui.format != format => {
            ui.format = format;
            ui.deck_id = None;
            ui.show_decks = false;
            ui.deck_page = 0;
            reset_pages = true;
        }
        Action::Track(track) if ui.track != track => {
            ui.track = track;
            reset_pages = true;
        }
        Action::ToggleLeagues => ui.show_leagues = !ui.show_leagues,
        Action::League(league) => {
            ui.show_leagues = false;
            if ui.league != league {
                ui.league = league;
                ui.division = None;
                reset_pages = true;
            }
        }
        Action::Division(division) if ui.division != division => {
            ui.division = division;
            reset_pages = true;
        }
        Action::ToggleDecks => ui.show_decks = !ui.show_decks,
        Action::DeckPage(page) => ui.deck_page = page,
        Action::Deck(id) => {
            ui.deck_id = Some(id);
            ui.show_decks = false;
        }
        Action::Join => {
            if !state.can_play_online()
                || !state.central_ready()
                || state.lobby.guest()
                || state.lobby.awaiting().is_some()
                || state.ranked_net.busy
            {
                return;
            }
            let format = queue::effective_format(state);
            if let Some((id, _)) = queue::selected(state) {
                native::join(state, format, &id.to_string());
            }
            return;
        }
        Action::Cancel => {
            native::cancel(state);
            return;
        }
        Action::Resume => {
            native::resume(state);
            return;
        }
        Action::Refresh => {
            refresh(state);
            return;
        }
        Action::Page(forward) => {
            if state.ranked_net.board_busy {
                return;
            }
            if forward {
                let Some(api::Response::Board {
                    season,
                    entries,
                    next: Some(cursor),
                }) = &state.ranked_net.board
                else {
                    return;
                };
                let offset = ui.pages.last().map_or(0, |(_, n)| *n) + entries.len();
                ui.season = Some(season.clone());
                ui.pages.push((Some(*cursor), offset));
            } else if ui.pages.pop().is_none() {
                return;
            }
            cx.scrolled.set(List::Ranked, 0.0);
            let request = ui.board_request();
            native::board(state, request);
        }
        _ => return,
    }
    if reset_pages {
        state.ranked.reset_pages();
        cx.scrolled.set(List::Ranked, 0.0);
        let request = state.ranked.board_request();
        native::board(state, request);
    }
    state.ranked.revision = state.ranked.revision.wrapping_add(1);
}

pub(super) fn entry(commands: &mut Commands, parent: Entity, kit: Kit) {
    let button = controls::button_compact(
        commands,
        kit,
        Some(crate::hud::glyph::TROPHY),
        Phrase::RankedTitle.text(kit.lang()),
        Weight::Secondary,
        Live::Yes,
        Press::Ranked(Action::Open),
    );
    orders::stop(commands, button, &orders::PLAY, "ranked");
    commands.entity(parent).add_child(button);
}

fn divisions(league: League) -> u8 {
    match league {
        League::Wood | League::Iron | League::Bronze | League::Silver => 3,
        League::Gold | League::Platinum | League::Diamond => 4,
        _ => 0,
    }
}
pub(crate) fn league_name(league: League, lang: Lang) -> &'static str {
    match league {
        League::Unranked => Phrase::RankedUnranked,
        League::Paper => Phrase::RankedPaper,
        League::Wood => Phrase::RankedWood,
        League::Iron => Phrase::RankedIron,
        League::Bronze => Phrase::RankedBronze,
        League::Silver => Phrase::RankedSilver,
        League::Gold => Phrase::RankedGold,
        League::Platinum => Phrase::RankedPlatinum,
        League::Diamond => Phrase::RankedDiamond,
        League::Master => Phrase::RankedMaster,
        League::Grandmaster => Phrase::RankedGrandmaster,
        League::Champion => Phrase::RankedChampion,
    }
    .text(lang)
}
pub(crate) fn format_name(format: Format, lang: Lang) -> &'static str {
    match format {
        Format::Standard => Phrase::PlayFormatStandard,
        Format::Highlander => Phrase::PlayFormatHighlander,
        Format::Commander => Phrase::FormatCommander,
    }
    .text(lang)
}
fn numeral(division: u8) -> &'static str {
    match division {
        1 => "I",
        2 => "II",
        3 => "III",
        4 => "IV",
        _ => "",
    }
}
pub(crate) fn rank_name(rank: api::Rank, lang: Lang) -> String {
    let league = league_name(rank.league, lang);
    if let Some(place) = rank.champion_place {
        format!("{league} #{place}")
    } else if let Some(division) = rank.division {
        format!("{league} {}", numeral(division))
    } else {
        league.to_string()
    }
}
