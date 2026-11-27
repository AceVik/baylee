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
mod inline;
pub(crate) mod native;
mod participants;
mod preview;
mod queue;
mod receipt;
#[cfg(feature = "dev-control")]
pub(crate) mod visual_fixture;
pub(super) use draw::draw;
/// The account's standing in a format: its rank, placement games, Elo and
/// season record (wins, losses, draws), as far as Central has said them.
type OwnRank = (api::Rank, u8, Option<i32>, Option<(u32, u32, u32)>);

pub(crate) const ORDER: TabOrder = TabOrder {
    name: "ranked",
    stops: &[
        "back",
        "rank-format",
        "deck",
        "deck-choice",
        "preview",
        "queue",
        "format",
        "track",
        "league",
        "league-choice",
        "division",
        "refresh",
        "rows",
        "season",
        "season-choice",
        "pager",
        "baylee",
    ],
    modal: false,
};

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
    Preview,
    Join,
    Cancel,
    Resume,
    Refresh,
    Page(bool),
    Seasons,
    Season(Option<api::Id>),
}

pub(crate) struct Ui {
    format: Format,
    queue_format: Format,
    track: Track,
    league: League,
    division: Option<u8>,
    deck_id: Option<uuid::Uuid>,
    show_decks: bool,
    deck_page: usize,
    show_leagues: bool,
    // Pin subsequent cursor pages to the season that produced the first page.
    season: Option<api::Season>,
    selected_season: Option<api::Id>,
    browsing_seasons: bool,
    season_pages: Vec<api::Id>,
    // For Back, store the cursor used for each visited page and its ordinal offset.
    pages: Vec<(Cursor, usize)>,
    previous: Hub,
    revision: u64,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            format: Format::Standard,
            queue_format: Format::Standard,
            track: Track::Player,
            league: League::Champion,
            division: None,
            deck_id: None,
            show_decks: false,
            show_leagues: false,
            season: None,
            selected_season: None,
            browsing_seasons: false,
            season_pages: Vec::new(),
            pages: Vec::new(),
            previous: Hub::Play,
            deck_page: 0,
            revision: 0,
        }
    }
}

impl Ui {
    fn board_request(&self) -> api::Request {
        if self.browsing_seasons {
            return api::Request::Seasons {
                before: self.season_pages.last().copied(),
            };
        }
        api::Request::Board {
            season: self
                .selected_season
                .or_else(|| self.season.as_ref().map(|s| s.id)),
            format: self.format,
            track: self.track,
            league: self.league,
            division: self.division,
            after: self.pages.last().map(|(cursor, _)| *cursor),
            // TODO(ai-players client): the ladder filter (everyone, people, AIs).
            players: api::ai::BoardPlayers::Everyone,
        }
    }
    fn reset_pages(&mut self) {
        self.browsing_seasons = false;
        self.season_pages.clear();
        self.season = None;
        self.pages.clear();
    }

    fn turn_page(&mut self, board: Option<&api::Response>, forward: bool) -> bool {
        if !forward {
            return if self.browsing_seasons {
                self.season_pages.pop().is_some()
            } else {
                self.pages.pop().is_some()
            };
        }
        match board {
            Some(api::Response::Seasons { next: Some(id), .. }) if self.browsing_seasons => {
                self.season_pages.push(*id);
                true
            }
            Some(api::Response::Board {
                season,
                entries,
                next: Some(cursor),
            }) if !self.browsing_seasons => {
                let offset = self.pages.last().map_or(0, |(_, n)| *n) + entries.len();
                self.season = Some(season.clone());
                self.pages.push((*cursor, offset));
                true
            }
            _ => false,
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        participants::poll
            .after(crate::DuelSet::Sync)
            .before(crate::DuelSet::Present),
    );
    #[cfg(feature = "dev-control")]
    visual_fixture::install(app);
    app.add_systems(
        OnEnter(crate::DuelPhase::Finished),
        native::close_game_transport,
    )
    .add_systems(OnEnter(crate::DuelPhase::Closed), native::clear_game_marker);
    app.add_systems(Update, native::poll.after(super::poll).before(super::ui));
    app.add_systems(
        PostUpdate,
        inline::reveal_focus
            .after(bevy::ui::UiSystems::Layout)
            .run_if(in_state(crate::DuelPhase::Closed)),
    );
    app.add_systems(
        Update,
        (
            draw::patch,
            inline::patch,
            queue::animate,
            receipt::promotion,
        )
            .chain()
            .after(super::ui)
            .run_if(in_state(crate::DuelPhase::Closed)),
    );
}

pub(super) fn refresh(state: &mut LobbyState) {
    let request = state.ranked.board_request();
    native::board(state, request);
}
pub(super) fn escape(state: &mut ResMut<LobbyState>) {
    if inline::escape(state.bypass_change_detection()) {
        return;
    }
    if state.ranked.show_leagues {
        let ui = &mut state.bypass_change_detection().ranked;
        ui.show_leagues = false;
        ui.revision = ui.revision.wrapping_add(1);
    } else {
        state.hub = state.ranked.previous;
    }
}

fn open(cx: Cx<'_, '_, '_, '_, '_>) {
    if cx.state.hub == Hub::Ranked {
        return;
    }
    let previous = cx.state.hub;
    cx.state.ranked.previous = previous;
    cx.state.hub = Hub::Ranked;
    cx.state.header_menu = None;
    cx.state.front_menu = false;
    cx.state.lobby.park_caret();
    cx.state.ranked.reset_pages();
    refresh(cx.state);
    // The page's desk reads the account's decks and its queue entry as it
    // opens, as the Play tile's desk did when it unfolded.
    inline::opened(cx.state, cx.mailbox);
}

pub(super) fn press(action: Action, cx: Cx<'_, '_, '_, '_, '_>) {
    if action == Action::Open {
        open(cx);
        return;
    }
    if inline::owns(action) {
        inline::press(action, cx);
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
    if action == Action::Refresh {
        refresh(state);
        inline::refresh(state);
        state.ranked.revision = state.ranked.revision.wrapping_add(1);
        return;
    }
    // One format choice serves the ladder and the desk beside it.
    if let Action::Format(format) = action {
        inline::follow_format(state, format);
        state.ranked.revision = state.ranked.revision.wrapping_add(1);
    }
    let ui = &mut state.ranked;
    let mut reset_pages = false;
    match action {
        Action::Format(format) if ui.format != format => {
            ui.format = format;
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
        Action::Seasons => {
            ui.browsing_seasons = !ui.browsing_seasons;
            ui.season_pages.clear();
            let request = ui.board_request();
            native::board(state, request);
        }
        Action::Season(id) => {
            ui.selected_season = id;
            reset_pages = true;
        }
        Action::Page(forward) => {
            if state.ranked_net.board_busy
                || !ui.turn_page(state.ranked_net.board.as_ref(), forward)
            {
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

pub(in crate::lobby) use inline::TileCaption;

fn join(state: &mut LobbyState) {
    if !state.has_central_account() || state.lobby.awaiting().is_some() || state.ranked_net.busy {
        return;
    }
    let format = queue::effective_format(state);
    if let Some((id, _)) = queue::selected(state) {
        native::clear_preview(state);
        native::join(state, format, &id.to_string());
    }
}

pub(super) fn divisions(league: League) -> u8 {
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
