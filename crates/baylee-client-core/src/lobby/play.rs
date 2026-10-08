//! Play and its Create-table sheet (the shell design, `DESIGN-v5.md` §4,
//! §5), decided without a window: the gateway's clocks and their names, the
//! table list's filters and sort, the format warning before a Join (S-4),
//! this session's recent games, and the sheet's draft.

use super::{GameMode, GameSummary, Lobby, LobbyRequest, SeatKind};
use crate::i18n::{Lang, Phrase};
use baylee_core::preset::{RoomSeatSetup, RoomSetup};
use serde::Deserialize;

/// One clock a room may be opened at, as `GET /auth/config` lists them.
/// The first is the default (a room that names none plays it).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ClockPreset {
    /// Its wire name: what `POST /lobby/games` takes as `clock`.
    pub name: String,
    /// Seconds per decision; zero is no decision clock.
    #[serde(default)]
    pub decide_secs: u32,
    /// Seconds a seat may be gone before the house answers for it.
    #[serde(default)]
    pub reconnect_secs: u32,
    /// The gateway's one line about it, in English: shown only for a clock
    /// this client has no words of its own for.
    #[serde(default)]
    pub blurb: String,
}

/// A table's pace, as the listing says it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, serde::Serialize)]
pub struct TableClock {
    /// Seconds per decision; zero is none.
    #[serde(default)]
    pub decide_secs: u32,
    /// Seconds a seat may be gone.
    #[serde(default)]
    pub reconnect_secs: u32,
}

/// The five clocks a client of this build knows by name, with their
/// labels, used when the gateway has not said (an older one) — the same
/// five `GET /auth/config` lists today, the default (`classic`, three
/// minutes) first.
#[must_use]
pub fn known_clocks() -> Vec<ClockPreset> {
    [
        ("classic", 180, 60),
        ("casual", 600, 60),
        ("standard", 120, 60),
        ("blitz", 30, 30),
        ("untimed", 0, 60),
    ]
    .into_iter()
    .map(|(name, decide, reconnect)| ClockPreset {
        name: name.to_string(),
        decide_secs: decide,
        reconnect_secs: reconnect,
        blurb: String::new(),
    })
    .collect()
}

/// How long a decision may take, in words: "10 min", "30 s".
fn span(lang: Lang, secs: u32) -> String {
    if secs >= 60 && secs.is_multiple_of(60) {
        Phrase::ClockMinutes.fill(lang, &[&(secs / 60).to_string()])
    } else {
        Phrase::ClockSeconds.fill(lang, &[&secs.to_string()])
    }
}

/// A clock's label in the segmented control: "casual · 10 min". A name
/// this client knows is said in its language (C3-27); one it does not is
/// the wire's word.
#[must_use]
pub fn clock_label(lang: Lang, clock: &ClockPreset) -> String {
    let name = match clock.name.as_str() {
        "classic" => Phrase::ClockClassic.text(lang),
        "casual" => Phrase::ClockCasual.text(lang),
        "standard" => Phrase::ClockStandard.text(lang),
        "blitz" => Phrase::ClockBlitz.text(lang),
        "untimed" => return Phrase::ClockUntimed.text(lang).to_string(),
        other => other,
    };
    if clock.decide_secs == 0 {
        return name.to_string();
    }
    format!("{name} · {}", span(lang, clock.decide_secs))
}

/// The help line under the clock control: ours for a clock we know, the
/// gateway's blurb for one we do not.
#[must_use]
pub fn clock_help(lang: Lang, clock: &ClockPreset) -> String {
    match clock.name.as_str() {
        "classic" => Phrase::ClockClassicHelp.text(lang).to_string(),
        "casual" => Phrase::ClockCasualHelp.text(lang).to_string(),
        "standard" => Phrase::ClockStandardHelp.text(lang).to_string(),
        "blitz" => Phrase::ClockBlitzHelp.text(lang).to_string(),
        "untimed" => Phrase::ClockUntimedHelp.text(lang).to_string(),
        _ => clock.blurb.clone(),
    }
}

/// A table's pace in the room's rules rail: the clock it matches by its
/// numbers, else the bare numbers.
#[must_use]
pub fn table_clock_label(lang: Lang, clocks: &[ClockPreset], clock: TableClock) -> String {
    clocks
        .iter()
        .find(|c| c.decide_secs == clock.decide_secs && c.reconnect_secs == clock.reconnect_secs)
        .map_or_else(
            || {
                if clock.decide_secs == 0 {
                    Phrase::ClockUntimed.text(lang).to_string()
                } else {
                    span(lang, clock.decide_secs)
                }
            },
            |c| clock_label(lang, c),
        )
}

/// The table list's filter chips (§4): local over the page the feed sent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "five independent chips, each on or off"
)]
pub struct TableFilter {
    /// Only tables with a chair a person could take.
    pub open_seats: bool,
    /// Only tables without a password.
    pub no_password: bool,
    /// Leave out tables where every other chair is the house AI.
    pub hide_ai_only: bool,
    /// Only tables whose host brought a Commander deck.
    pub commander: bool,
    /// Only two-chair tables.
    pub duel: bool,
}

/// One chip of [`TableFilter`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    /// [`TableFilter::open_seats`].
    OpenSeats,
    /// [`TableFilter::no_password`].
    NoPassword,
    /// [`TableFilter::hide_ai_only`].
    HideAiOnly,
    /// [`TableFilter::commander`].
    Commander,
    /// [`TableFilter::duel`].
    Duel,
}

impl Chip {
    /// Every chip, in the row's order.
    pub const ALL: [Self; 5] = [
        Self::OpenSeats,
        Self::NoPassword,
        Self::HideAiOnly,
        Self::Commander,
        Self::Duel,
    ];

    /// Its label.
    #[must_use]
    pub const fn phrase(self) -> Phrase {
        match self {
            Self::OpenSeats => Phrase::ShellOpenSeats,
            Self::NoPassword => Phrase::ShellNoPassword,
            Self::HideAiOnly => Phrase::PlayHideAiOnly,
            Self::Commander => Phrase::FormatCommander,
            Self::Duel => Phrase::PlayDuel,
        }
    }
}

impl TableFilter {
    /// Whether a chip is on.
    #[must_use]
    pub const fn on(self, chip: Chip) -> bool {
        match chip {
            Chip::OpenSeats => self.open_seats,
            Chip::NoPassword => self.no_password,
            Chip::HideAiOnly => self.hide_ai_only,
            Chip::Commander => self.commander,
            Chip::Duel => self.duel,
        }
    }

    /// Turns a chip over.
    pub fn toggle(&mut self, chip: Chip) {
        let flag = match chip {
            Chip::OpenSeats => &mut self.open_seats,
            Chip::NoPassword => &mut self.no_password,
            Chip::HideAiOnly => &mut self.hide_ai_only,
            Chip::Commander => &mut self.commander,
            Chip::Duel => &mut self.duel,
        };
        *flag = !*flag;
    }

    /// Whether a table passes every chip that is on. The player's own
    /// table always does: hiding it would hide the way back to it.
    #[must_use]
    pub fn admits(self, game: &GameSummary) -> bool {
        if game.seated() || game.yours {
            return true;
        }
        let ai_only = game
            .seats
            .iter()
            .filter(|s| !s.host)
            .all(|s| s.kind == SeatKind::Ai);
        (!self.open_seats || game.joinable())
            && (!self.no_password || !game.locked)
            && (!self.hide_ai_only || !ai_only)
            && (!self.commander || host_format(game) == Some("commander"))
            && (!self.duel || game.seats.len() == 2)
    }
}

/// The format the host's deck plays, which is the nearest thing a waiting
/// table has to a format: the game's own is decided at the start.
#[must_use]
pub fn host_format(game: &GameSummary) -> Option<&str> {
    game.seats
        .iter()
        .find(|s| s.host)
        .map(|s| s.format.as_str())
        .filter(|f| !f.is_empty())
}

/// How the tables list is ordered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TableSort {
    /// The gateway's order: waiting first, newest first.
    #[default]
    Newest,
    /// Most open chairs first.
    OpenSeats,
    /// By name.
    Name,
}

impl TableSort {
    /// Every sort, in the menu's order.
    pub const ALL: [Self; 3] = [Self::Newest, Self::OpenSeats, Self::Name];

    /// Its name in the menu.
    #[must_use]
    pub const fn phrase(self) -> Phrase {
        match self {
            Self::Newest => Phrase::PlaySortNewest,
            Self::OpenSeats => Phrase::PlaySortOpenSeats,
            Self::Name => Phrase::PlaySortName,
        }
    }
}

/// The rows shown: indices into `games` that pass `filter`, in `sort`'s
/// order. The player's own table stands first whatever the sort.
#[must_use]
pub fn table_order(games: &[GameSummary], filter: TableFilter, sort: TableSort) -> Vec<usize> {
    let mut shown: Vec<usize> = (0..games.len())
        .filter(|i| filter.admits(&games[*i]))
        .collect();
    let open = |g: &GameSummary| g.seats.iter().filter(|s| s.open()).count();
    match sort {
        TableSort::Newest => {}
        TableSort::OpenSeats => shown.sort_by_key(|i| std::cmp::Reverse(open(&games[*i]))),
        TableSort::Name => shown.sort_by_key(|i| super::shelf::fold(&games[*i].name)),
    }
    shown.sort_by_key(|i| !(games[*i].seated() || games[*i].yours));
    shown
}

/// The line before a Join when the next game's deck does not fit the table
/// (S-4): the host brought a deck of another format. Join still works — the
/// room lets the player pick another deck.
#[must_use]
pub fn format_warning(lang: Lang, mine: &str, game: &GameSummary) -> Option<String> {
    let theirs = host_format(game)?;
    if mine.is_empty() || mine == theirs || game.seated() {
        return None;
    }
    Some(Phrase::PlayFormatWarning.fill(
        lang,
        &[
            &super::shelf::format_label(lang, mine),
            &super::shelf::format_label(lang, theirs),
        ],
    ))
}

/// How a finished game went, from this player's side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Won.
    Won,
    /// Lost.
    Lost,
    /// A draw, or an ending without a winner.
    Drawn,
}

/// One of this session's finished games (Play's Recent games): nothing of
/// it is stored, so it is gone with the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentGame {
    /// The game, which `POST /lobby/games/{id}/rematch` takes.
    pub game_id: String,
    /// When it ended, as the client's clock read it (`HH:MM`).
    pub at: String,
    /// Who else sat there, by the names the table used.
    pub opponents: Vec<String>,
    /// How it went.
    pub outcome: Outcome,
    /// The deck this player brought.
    pub deck: String,
    /// The chairs it had, for Edit and rematch.
    pub chairs: usize,
    /// The table's name.
    pub name: String,
    /// Whether it was against the house: offline, or `mode: "ai"`.
    pub house: bool,
}

/// How many recent games Play keeps.
pub const RECENT_KEPT: usize = 5;

/// The three starting templates of the Create-table sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Template {
    /// 40 life.
    #[default]
    Commander,
    /// 20 life.
    Duel,
    /// 20 life, every seat starts with one of each basic land in play.
    FiveLand,
}

impl Template {
    /// Every template, in the sheet's order.
    pub const ALL: [Self; 3] = [Self::Commander, Self::Duel, Self::FiveLand];

    /// Its card's title.
    #[must_use]
    pub const fn phrase(self) -> Phrase {
        match self {
            Self::Commander => Phrase::TemplateCommander,
            Self::Duel => Phrase::TemplateDuel,
            Self::FiveLand => Phrase::TemplateFiveLand,
        }
    }

    /// The life it starts at.
    #[must_use]
    pub const fn life(self) -> i32 {
        match self {
            Self::Commander => 40,
            Self::Duel | Self::FiveLand => 20,
        }
    }
}

/// The Create-table sheet's choices besides the two text fields (the
/// name and the password are the lobby's own [`super::Field::RoomName`]
/// and [`super::Field::RoomPassword`], so they type like every field).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDraft {
    /// Chairs, two to eight.
    pub players: usize,
    /// The template chosen.
    pub template: Template,
    /// Starting life (Adjust).
    pub life: i32,
    /// Free mulligans (Adjust).
    pub mulligans: u8,
    /// Which of the gateway's clocks, by position.
    pub clock: usize,
    /// Whether Adjust is unfolded.
    pub adjust: bool,
}

impl Default for TableDraft {
    fn default() -> Self {
        Self::of(Template::Commander, 4)
    }
}

impl TableDraft {
    /// A draft from a template.
    #[must_use]
    pub fn of(template: Template, players: usize) -> Self {
        Self {
            players: players.clamp(super::MIN_CHAIRS, super::MAX_CHAIRS),
            template,
            life: template.life(),
            mulligans: RoomSetup::default().free_mulligans,
            clock: 0,
            adjust: false,
        }
    }

    /// Picks a template: its life, the rest kept.
    pub fn pick(&mut self, template: Template) {
        self.template = template;
        self.life = template.life();
        if template == Template::Duel && self.players > 2 {
            self.players = 2;
        }
    }

    /// One more or one fewer chair, within two to eight.
    pub fn step_players(&mut self, more: bool) {
        self.players = if more {
            (self.players + 1).min(super::MAX_CHAIRS)
        } else {
            self.players.saturating_sub(1).max(super::MIN_CHAIRS)
        };
    }

    /// The room's rules this draft asks for.
    #[must_use]
    pub fn setup(&self) -> RoomSetup {
        let mut setup = RoomSetup {
            starting_life: self.life.clamp(1, 999),
            free_mulligans: self.mulligans.min(7),
            seats: Vec::new(),
        };
        if self.template == Template::FiveLand {
            setup.seats = (0..self.players)
                .map(|_| RoomSeatSetup {
                    permanents: ["Forest", "Island", "Mountain", "Plains", "Swamp"]
                        .map(str::to_string)
                        .to_vec(),
                    ..RoomSeatSetup::default()
                })
                .collect();
        }
        setup
    }

    /// The derived line under the template cards: "40 life · 1 free
    /// mulligan".
    #[must_use]
    pub fn summary(&self, lang: Lang) -> String {
        let mut line = Phrase::RulesLife.fill(lang, &[&self.life.to_string()]);
        line.push_str(" · ");
        line.push_str(
            &Phrase::counted(
                usize::from(self.mulligans),
                Phrase::RulesMulliganOne,
                Phrase::RulesMulliganMany,
            )
            .fill(lang, &[&self.mulligans.to_string()]),
        );
        if self.template == Template::FiveLand {
            line.push_str(" · ");
            line.push_str(Phrase::RulesFiveLands.text(lang));
        }
        line
    }
}

impl Lobby {
    /// The clocks the gateway offers; this build's four until it has said.
    #[must_use]
    pub fn clocks(&self) -> Vec<ClockPreset> {
        if self.clocks.is_empty() {
            known_clocks()
        } else {
            self.clocks.clone()
        }
    }

    /// What `GET /auth/config` listed as `clocks`.
    pub fn set_clocks(&mut self, clocks: Vec<ClockPreset>) {
        self.clocks = clocks;
    }

    /// This session's finished games, newest first.
    #[must_use]
    pub fn recent(&self) -> &[RecentGame] {
        &self.recent
    }

    /// Remembers a finished game for Play's Recent games.
    pub fn remember_game(&mut self, game: RecentGame) {
        self.recent.retain(|g| g.game_id != game.game_id);
        self.recent.insert(0, game);
        self.recent.truncate(RECENT_KEPT);
    }

    /// Opens a table from the Create-table sheet: its name and password
    /// from the two fields, the chairs and the clock from the draft. The
    /// rules are applied once the room is open
    /// ([`Self::take_pending_setup`]), because `POST /lobby/games` takes
    /// none.
    pub fn open_table(&mut self, draft: &TableDraft) -> Option<LobbyRequest> {
        let clock = self.clocks().get(draft.clock).map(|c| c.name.clone());
        let name = self.field(super::Field::RoomName).trim().to_string();
        let request = self.open_room(GameMode::Open, draft.players, name)?;
        let setup = draft.setup();
        if setup != RoomSetup::default() {
            self.pending_setup = Some(setup);
        }
        Some(match request {
            LobbyRequest::CreateGame {
                deck_id,
                mode,
                chairs,
                name,
                password,
                ai,
                ..
            } => LobbyRequest::CreateGame {
                deck_id,
                mode,
                chairs,
                name,
                password,
                clock,
                ai,
            },
            other => other,
        })
    }

    /// Plays the house at a difficulty (Play's hero): `mode: "ai"`, one
    /// request, the table starting at once.
    pub fn play_house(&mut self, difficulty: &str) -> Option<LobbyRequest> {
        match self.host(GameMode::Ai)? {
            LobbyRequest::CreateGame {
                deck_id,
                mode,
                chairs,
                name,
                password,
                clock,
                ..
            } => Some(LobbyRequest::CreateGame {
                deck_id,
                mode,
                chairs,
                name,
                password,
                clock,
                ai: Some(difficulty.to_string()),
            }),
            other => Some(other),
        }
    }

    /// The room's rules as the sheet's draft, for **Edit rules**: its chairs,
    /// life and mulligans, and the template they read as. `None` unless
    /// this client hosts the room it waits at.
    #[must_use]
    pub fn room_as_draft(&self) -> Option<TableDraft> {
        let draft = self.room_edit.as_ref().filter(|d| d.host)?;
        let setup = &draft.update.setup;
        let lands = !setup.seats.is_empty() && setup.seats.iter().all(|s| s.permanents.len() == 5);
        let template = match (setup.starting_life, lands) {
            (_, true) => Template::FiveLand,
            (20, false) => Template::Duel,
            _ => Template::Commander,
        };
        Some(TableDraft {
            players: draft.update.chairs,
            template,
            life: setup.starting_life,
            mulligans: setup.free_mulligans,
            clock: 0,
            adjust: false,
        })
    }

    /// **Apply** from the sheet over a room this client hosts: the chairs,
    /// life and mulligans into the room's draft and out as one update. A
    /// seat's own life or board stays; the five-land start fills the boards
    /// that are empty.
    pub fn apply_table(&mut self, table: &TableDraft) -> Option<LobbyRequest> {
        let draft = self.room_edit.as_mut().filter(|d| d.host)?;
        let update = &mut draft.update;
        update.chairs = table.players.clamp(super::MIN_CHAIRS, super::MAX_CHAIRS);
        update.setup.starting_life = table.life.clamp(1, 999);
        update.setup.free_mulligans = table.mulligans.min(7);
        update
            .setup
            .seats
            .resize_with(update.chairs, RoomSeatSetup::default);
        if table.template == Template::FiveLand {
            for seat in &mut update.setup.seats {
                if seat.permanents.is_empty() {
                    seat.permanents = ["Forest", "Island", "Mountain", "Plains", "Swamp"]
                        .map(str::to_string)
                        .to_vec();
                }
            }
        }
        self.save_room(false)
    }

    /// The Create-table sheet's rules, once the room it opened is this
    /// host's to edit: written into the room's draft and sent as one
    /// update. `None` until then, and after.
    pub fn take_pending_setup(&mut self) -> Option<LobbyRequest> {
        if self.busy || self.pending_setup.is_none() {
            return None;
        }
        let draft = self.room_edit.as_mut().filter(|d| d.host)?;
        let setup = self.pending_setup.take()?;
        draft.update.setup = setup;
        self.save_room(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::GameSeat;

    fn table(name: &str, chairs: &[(SeatKind, bool, bool, &str)], locked: bool) -> GameSummary {
        GameSummary {
            id: name.into(),
            name: name.into(),
            state: "waiting".into(),
            locked,
            seats: chairs
                .iter()
                .enumerate()
                .map(|(i, (kind, taken, host, format))| GameSeat {
                    seat: u32::try_from(i).unwrap_or(0),
                    kind: *kind,
                    taken: *taken,
                    host: *host,
                    format: (*format).into(),
                    ..GameSeat::default()
                })
                .collect(),
            ..GameSummary::default()
        }
    }

    #[test]
    fn the_chips_filter_the_page_and_the_sorts_order_it() {
        use SeatKind::{Ai, Human};
        let games = vec![
            table(
                "pod",
                &[
                    (Human, true, true, "commander"),
                    (Human, false, false, ""),
                    (Ai, false, false, ""),
                ],
                true,
            ),
            table(
                "duel",
                &[(Human, true, true, "freeform"), (Human, false, false, "")],
                false,
            ),
            table(
                "bots",
                &[(Human, true, true, "freeform"), (Ai, false, false, "")],
                false,
            ),
            table(
                "full",
                &[(Human, true, true, "commander"), (Human, true, false, "")],
                false,
            ),
        ];
        let all = TableFilter::default();
        assert_eq!(
            table_order(&games, all, TableSort::Newest),
            vec![0, 1, 2, 3]
        );
        let mut f = all;
        f.toggle(Chip::OpenSeats);
        assert_eq!(table_order(&games, f, TableSort::Newest), vec![0, 1]);
        let mut f = all;
        f.toggle(Chip::NoPassword);
        f.toggle(Chip::HideAiOnly);
        assert_eq!(table_order(&games, f, TableSort::Newest), vec![1, 3]);
        let mut f = all;
        f.toggle(Chip::Commander);
        assert_eq!(table_order(&games, f, TableSort::Newest), vec![0, 3]);
        let mut f = all;
        f.toggle(Chip::Duel);
        assert_eq!(table_order(&games, f, TableSort::Name), vec![2, 1, 3]);
        assert_eq!(table_order(&games, all, TableSort::OpenSeats)[0], 0);
    }

    /// The player's own table is never filtered away and stands first.
    #[test]
    fn my_table_survives_every_chip_and_leads() {
        let mut games = vec![
            table("other", &[(SeatKind::Human, false, false, "")], false),
            table("mine", &[(SeatKind::Human, true, true, "")], true),
        ];
        games[1].seats[0].you = true;
        let mut f = TableFilter::default();
        f.toggle(Chip::NoPassword);
        f.toggle(Chip::OpenSeats);
        assert_eq!(table_order(&games, f, TableSort::Name), vec![1, 0]);
    }

    #[test]
    fn a_deck_of_another_format_is_warned_about_before_the_join() {
        let pod = table("pod", &[(SeatKind::Human, true, true, "commander")], false);
        assert_eq!(format_warning(Lang::En, "commander", &pod), None);
        let said = format_warning(Lang::En, "freeform", &pod).expect("a warning");
        assert!(
            said.contains("Freeform") && said.contains("Commander"),
            "{said}"
        );
        let quiet = table("x", &[(SeatKind::Human, true, true, "")], false);
        assert_eq!(format_warning(Lang::En, "freeform", &quiet), None);
    }

    /// The clocks read in the player's language; a name this client does
    /// not know reads as the wire says it, with the gateway's blurb.
    #[test]
    fn clock_labels_are_german_under_de_and_the_wire_s_for_an_unknown_one() {
        let clocks = known_clocks();
        assert_eq!(clock_label(Lang::En, &clocks[0]), "classic · 3 min");
        assert_eq!(clock_label(Lang::De, &clocks[0]), "klassisch · 3 Min.");
        assert_eq!(clock_label(Lang::De, &clocks[1]), "gemütlich · 10 Min.");
        assert_eq!(clock_label(Lang::De, &clocks[3]), "Blitz · 30 s");
        assert_eq!(clock_label(Lang::De, &clocks[4]), "ohne Uhr");
        let odd = ClockPreset {
            name: "glacial".into(),
            decide_secs: 1800,
            reconnect_secs: 60,
            blurb: "half an hour".into(),
        };
        assert_eq!(clock_label(Lang::De, &odd), "glacial · 30 Min.");
        assert_eq!(clock_help(Lang::De, &odd), "half an hour");
        assert_ne!(
            clock_help(Lang::De, &clocks[0]),
            clock_help(Lang::En, &clocks[0])
        );
        assert_eq!(
            table_clock_label(
                Lang::En,
                &clocks,
                TableClock {
                    decide_secs: 120,
                    reconnect_secs: 60
                }
            ),
            "standard · 2 min"
        );
    }

    /// The Create-table sheet opens on three minutes a decision (owner,
    /// 08.10.2026), the clock a room naming none plays, and still offers
    /// every pace it offered before.
    #[test]
    fn a_new_table_is_three_minutes_a_decision() {
        let draft = TableDraft::default();
        let clocks = known_clocks();
        assert_eq!(clocks[draft.clock].name, "classic");
        assert_eq!(
            clocks[draft.clock].decide_secs,
            baylee_core::preset::HouseRules::default().decision_timeout_secs
        );
        assert_eq!(clocks[draft.clock].decide_secs, 180);
        let names: Vec<&str> = clocks.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["classic", "casual", "standard", "blitz", "untimed"]);
    }

    #[test]
    fn a_draft_makes_the_rules_its_template_names() {
        let mut d = TableDraft::default();
        assert_eq!(
            d.setup(),
            RoomSetup::default(),
            "Commander is the room's default"
        );
        d.pick(Template::Duel);
        assert_eq!((d.players, d.setup().starting_life), (2, 20));
        d.pick(Template::FiveLand);
        d.step_players(true);
        let setup = d.setup();
        assert_eq!(setup.seats.len(), 3);
        assert_eq!(setup.seats[2].permanents.len(), 5);
        for _ in 0..20 {
            d.step_players(true);
        }
        assert_eq!(d.players, 8);
        assert_eq!(
            d.summary(Lang::En),
            "20 life · 1 free mulligan · five lands in play"
        );
    }

    #[test]
    fn recent_games_keep_the_newest_five() {
        let mut lobby = Lobby::new();
        for i in 0..7 {
            lobby.remember_game(RecentGame {
                game_id: i.to_string(),
                at: "14:02".into(),
                opponents: vec![],
                outcome: Outcome::Won,
                deck: String::new(),
                chairs: 2,
                name: String::new(),
                house: true,
            });
        }
        assert_eq!(lobby.recent().len(), RECENT_KEPT);
        assert_eq!(lobby.recent()[0].game_id, "6");
    }
}
