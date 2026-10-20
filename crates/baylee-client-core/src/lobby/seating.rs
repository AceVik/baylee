//! The tables: the list and its pages, opening, joining and configuring a
//! room, the chairs, and leaving a seat.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

impl Lobby {
    /// What the next read of the table list asks for.
    #[must_use]
    pub fn query(&self) -> GameQuery {
        GameQuery {
            q: self.awaiting.as_ref().map_or_else(
                || self.search.text().trim().to_string(),
                |h| h.game_id.clone(),
            ),
            offset: if self.awaiting.is_some() {
                0
            } else {
                self.offset
            },
            limit: PAGE,
        }
    }

    /// What the table list itself asks for when shown (including when a
    /// seated player steps away from their room to browse the lobby).
    #[must_use]
    pub fn search_query(&self) -> GameQuery {
        GameQuery {
            q: self.search.text().trim().to_string(),
            offset: self.offset,
            limit: PAGE,
        }
    }

    /// Asks for the current page of the table list.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "every caller is an intent or an event handler answering \
                  Option<LobbyRequest>, and unwrapping this one would put a \
                  Some( at eight call sites to save it here"
    )]
    pub(super) fn list(&mut self) -> Option<LobbyRequest> {
        self.busy = true;
        self.refreshing = true;
        Some(LobbyRequest::ListGames(self.query()))
    }

    /// Asks for the ticket to a chair this player is in and holds nothing for.
    ///
    /// The authenticated seat discovery proves ownership and eligibility,
    /// excluding finished participation and unclaimed rematch reservations.
    /// It is independent of whichever public page or search is on screen.
    /// `awaiting` is a ticket we hold, `reclaimed` one we already asked for.
    ///
    /// Only from [`Screen::Table`]: the lobby is where a player is looking
    /// for their table. Somebody in the deck builder did not ask to be moved,
    /// and the sign-in screen has no account to ask with.
    ///
    /// And **never offline**, where the whole question is unaskable: a ticket
    /// that outlives the client is what this recovers, and offline the table
    /// itself dies with the process, so there is nothing on the other side to
    /// hand a chair back. The offline performer says exactly that and refuses
    /// in words — which is a red line in the corner of a lobby that has done
    /// nothing wrong, and is how this was found.
    pub(super) fn reclaim_a_seat(&mut self) -> Option<LobbyRequest> {
        if !matches!(self.screen, Screen::Table) || self.awaiting.is_some() || self.offline() {
            return None;
        }
        // A known update-restart chair wins only if the gateway still lists
        // it as resumable. Public rows cannot prove that an individual player
        // has not already conceded or lost a still-running multiplayer game.
        let eligible = |seat: &&resume::Seat| !self.reclaimed.iter().any(|id| id == &seat.game_id);
        let seat = self
            .resume_candidates
            .iter()
            .filter(eligible)
            .find(|seat| {
                self.resuming
                    .as_ref()
                    .is_some_and(|(id, _)| id == &seat.game_id)
            })
            .or_else(|| self.resume_candidates.iter().find(eligible))?;
        let game_id = seat.game_id.clone();
        self.resuming = Some((game_id.clone(), seat.playing));
        self.reclaimed.push(game_id.clone());
        self.busy = true;
        Some(LobbyRequest::TakeSeat { game_id })
    }

    /// Reads the list again for what the search box now says.
    ///
    /// From the first page: a search is a different list, and the row that
    /// was fourth in the old one is not the fourth in this one.
    pub fn search_again(&mut self) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.offset = 0;
        self.busy = true;
        self.refreshing = true;
        Some(LobbyRequest::ListGames(self.search_query()))
    }

    /// How many tables the search matched, of which [`Lobby::games`] is a page.
    #[must_use]
    pub fn total(&self) -> usize {
        self.total
    }

    /// Where the shown page starts in that list.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Whether there is a page after this one.
    #[must_use]
    pub fn more(&self) -> bool {
        self.offset + self.games.len() < self.total
    }

    /// Steps one page forwards or back, if there is one to step onto.
    pub fn page(&mut self, forwards: bool) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        let next = if forwards {
            if !self.more() {
                return None;
            }
            self.offset + PAGE
        } else {
            self.offset.checked_sub(PAGE)?
        };
        self.offset = next;
        self.list()
    }

    /// Re-reads decks and tables. Decks first: the answer chains into games.
    pub fn refresh(&mut self) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.refreshing = true;
        Some(LobbyRequest::ListDecks)
    }

    /// Whether the request in flight only re-reads the deck and table
    /// lists the screen already shows: a refresh, a search, a page, and the
    /// chain of list reads they set off (the hosted models a host's room
    /// asks for among them).
    ///
    /// The shell answers this one with a quiet mark on the list rather than
    /// the loading veil (§10 #7 of the shell design): a veil over a list the
    /// player is reading, every time it is re-read, is a flicker.
    #[must_use]
    pub fn refreshing(&self) -> bool {
        self.busy && self.refreshing
    }

    /// Opens a new table with the selected deck.
    pub fn host(&mut self, mode: GameMode) -> Option<LobbyRequest> {
        self.open_room(mode, 2, String::new())
    }

    /// Opens a table of a chosen size, under a chosen name, locked with
    /// whatever is in the room password box.
    pub fn open_room(
        &mut self,
        mode: GameMode,
        chairs: usize,
        name: String,
    ) -> Option<LobbyRequest> {
        let password = self.room_password.text().to_string();
        self.open_locked_room(mode, chairs, name, password)
    }

    /// Opens a table with an explicit password, for a caller that has one
    /// that did not come from the box.
    pub fn open_locked_room(
        &mut self,
        mode: GameMode,
        chairs: usize,
        name: String,
        password: String,
    ) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        if self.health.no_agents {
            self.refuse(Phrase::NoNewGames);
            return None;
        }
        let deck_id = self
            .selected()
            .and_then(|at| self.decks.get(at))
            .map(|d| d.id.clone())
            .unwrap_or_default();
        if mode == GameMode::Ai && deck_id.is_empty() {
            self.refuse(Phrase::PickADeckFirst);
            return None;
        }
        self.busy = true;
        self.room_password.clear();
        self.asked_for = Some(mode);
        self.note(Phrase::OpeningTable);
        Some(LobbyRequest::CreateGame {
            deck_id,
            mode,
            chairs: chairs.clamp(MIN_CHAIRS, MAX_CHAIRS),
            name,
            password,
            clock: None,
            ai: None,
            spectators: true,
        })
    }

    /// Sits down at somebody else's table with the selected deck.
    pub fn join(&mut self, game_id: &str) -> Option<LobbyRequest> {
        self.join_seat(game_id, None)
    }

    /// Sits down in a named chair, sending whatever is in the room password
    /// box — which a room that is not locked simply ignores.
    pub fn join_seat(&mut self, game_id: &str, seat: Option<u32>) -> Option<LobbyRequest> {
        self.join_with(game_id, seat, None)
    }

    /// Sits down at a friend's table their invite named (the beta.8 social
    /// design §11.7): the selected deck, the first free chair, and the
    /// invite in the password's place.
    pub fn join_invited(&mut self, game_id: &str, invite: &str) -> Option<LobbyRequest> {
        self.join_with(game_id, None, Some(invite.to_string()))
    }

    fn join_with(
        &mut self,
        game_id: &str,
        seat: Option<u32>,
        invite: Option<String>,
    ) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        let deck_id = self
            .selected()
            .and_then(|at| self.decks.get(at))
            .map(|d| d.id.clone())
            .unwrap_or_default();
        // An invite stands in for the password: the box is not sent with it.
        let password = if invite.is_some() {
            String::new()
        } else {
            self.room_password.text().to_string()
        };
        self.room_password.clear();
        self.busy = true;
        // A table only starts once every chair is ready and the host says go,
        // so sitting down does not begin the game — the seat screen waits
        // either way.
        self.asked_for = None;
        self.note(Phrase::SittingDown);
        Some(LobbyRequest::JoinGame {
            game_id: game_id.to_string(),
            deck_id,
            seat,
            password,
            invite,
        })
    }

    /// Asks for a chair at the next table.
    ///
    /// `game_id` is either end of the pair — the game that just ended, or the
    /// room opened from it — because the two people pressing this are looking
    /// at different screens and the gateway answers both with the same ticket.
    ///
    /// [`Lobby::asked_for`] is cleared for the reason [`Lobby::join_seat`]
    /// clears it: what comes back is a seat at a table that may not have
    /// started, and the open-table veil is a different screen from the seat
    /// one.
    pub fn rematch(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.asked_for = None;
        self.note(Phrase::PlayingAgain);
        Some(LobbyRequest::Rematch {
            game_id: game_id.to_string(),
        })
    }

    /// Remembers that the player pressed *play again* on a finished game.
    ///
    /// Nothing goes out yet; see the field of the same name.
    pub fn want_rematch(&mut self, game_id: impl Into<String>) {
        self.rematch_wanted = Some(game_id.into());
    }

    /// The game a press is still waiting to be spent on, if there is one.
    #[must_use]
    pub fn rematch_wanted(&self) -> Option<&str> {
        self.rematch_wanted.as_deref()
    }

    /// The request that press turned into, once the shell has left the table.
    pub fn take_rematch(&mut self) -> Option<LobbyRequest> {
        let game_id = self.rematch_wanted.take()?;
        self.rematch(&game_id)
    }

    /// Sends a host edit; room authority is checked again by the gateway.
    pub fn configure_room(
        &mut self,
        game_id: &str,
        update: baylee_core::preset::RoomUpdate,
    ) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::ConfigureRoom {
            game_id: game_id.to_string(),
            update,
        })
    }

    /// Says whether this player is ready to play.
    pub fn set_ready(&mut self, game_id: &str, ready: bool) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(if ready {
            Phrase::SayingReady
        } else {
            Phrase::SayingNotReady
        });
        Some(LobbyRequest::SetReady {
            game_id: game_id.to_string(),
            ready,
        })
    }

    /// Starts the room. The host's call, and only when every chair is ready.
    ///
    /// The button is hidden for anyone else and greyed until the listing says
    /// `startable`, but nothing here is a check — the gateway refuses both
    /// cases, and this is only about not offering a player a button that does
    /// nothing.
    pub fn start_room(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        if self.health.no_agents {
            self.refuse(Phrase::NoNewGames);
            return None;
        }
        self.busy = true;
        self.note(Phrase::Starting);
        Some(LobbyRequest::StartGame {
            game_id: game_id.to_string(),
        })
    }

    /// Hands the room to another chair.
    pub fn hand_over(&mut self, game_id: &str, seat: u32) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::HandingOver);
        Some(LobbyRequest::HandOver {
            game_id: game_id.to_string(),
            seat,
        })
    }

    /// Arranges one chair of a table this account hosts.
    ///
    /// Nothing is checked here that the gateway does not check again: the
    /// client hides what a player may not do, and the gateway is what makes
    /// it true.
    pub fn set_seat(
        &mut self,
        game_id: &str,
        seat: u32,
        kind: Option<SeatKind>,
        ai: Option<String>,
    ) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind,
            ai,
            deck_id: None,
            team: None,
        })
    }

    /// Moves a chair onto a side. `0` puts it back on its own.
    ///
    /// The host's, not the player's: a side is the format, and one the people
    /// at the table can change is not a format. The gateway says so again.
    pub fn seat_team(&mut self, game_id: &str, seat: u32, team: u8) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind: None,
            ai: None,
            deck_id: None,
            team: Some(team),
        })
    }

    /// Puts the selected deck in a chair — one's own, or an AI's.
    pub fn seat_deck(&mut self, game_id: &str, seat: u32) -> Option<LobbyRequest> {
        let deck_id = self.picked_deck()?;
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind: None,
            ai: None,
            deck_id: Some(deck_id),
            team: None,
        })
    }

    /// Puts the deck at `index` in a chair, picking it only when the chair
    /// is the player's own (`yours`). A deck handed to an AI chair leaves
    /// the player's pick where it was: the owner found the next game opening
    /// with the deck the house had just played.
    pub fn seat_deck_at(
        &mut self,
        game_id: &str,
        seat: u32,
        index: usize,
        yours: bool,
    ) -> Option<LobbyRequest> {
        if yours {
            self.select_deck(index);
            return self.seat_deck(game_id, seat);
        }
        if self.busy || !self.has_a_performer() {
            return None;
        }
        let deck_id = self.decks.get(index)?.id.clone();
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::SetSeat {
            game_id: game_id.to_string(),
            seat,
            kind: None,
            ai: None,
            deck_id: Some(deck_id),
            team: None,
        })
    }

    /// Gets up from a table, or closes it when this account is the host.
    pub fn leave_table(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() {
            return None;
        }
        self.busy = true;
        self.asked_for = None;
        self.note(Phrase::LeavingTable);
        Some(LobbyRequest::LeaveGame {
            game_id: game_id.to_string(),
        })
    }

    /// Leaves the seat screen without a seat, because the shell could not
    /// connect to the table it was handed.
    pub fn unseat_because(&mut self, phrase: Phrase, args: &[&str]) {
        let why = phrase.fill(self.lang, args);
        self.unseat(why);
    }

    /// The same, in words somebody else chose — the gateway's, usually.
    pub fn unseat(&mut self, why: impl Into<String>) {
        self.leave_the_seat(why.into(), Tone::Refusal);
    }

    /// The same movement, because the game is simply **over**.
    ///
    /// [`Tone`] is the only channel that tells a refusal from a note, so the
    /// two ways out of a seat cannot share one: every finished duel used to
    /// put its own ending in the corner of the lobby in the red reserved for
    /// "nothing more will happen until you do something about it". Nothing
    /// had gone wrong; the player had won.
    pub fn stand_up(&mut self, phrase: Phrase, args: &[&str]) {
        let why = phrase.fill(self.lang, args);
        self.leave_the_seat(why, Tone::Note);
    }

    /// The same, saying how the game ended rather than that it ended.
    ///
    /// [`Self::stand_up`] writes `Phrase::GameEnded` — "das Spiel ist
    /// vorbei" — which is true of every game ever played here and says
    /// nothing about the one just finished (#155). The verdict is not
    /// missing information: the end screen has been drawing it all along
    /// from this same `GameResult`.
    ///
    /// So it is worded here through the same [`crate::interaction::verdict`]
    /// the sheet uses, rather than by a second reader of `GameResult` that
    /// could come to disagree with it about who won. The reason is appended
    /// where there is one; [`crate::interaction::ending_reason`] answers
    /// `None` for a draw, because the only thing it could say there is the
    /// verdict again.
    ///
    /// The two are joined with a full stop and not with the em dash this
    /// repository reaches for by habit, because one of the verdicts already
    /// contains one: `Phrase::YourTeamWon` is "Team {0} gewinnt \u{2014} deins",
    /// and a dash joiner would render "Team 2 gewinnt \u{2014} deins \u{2014} Nur noch
    /// ein Team im Spiel". A full stop cannot collide with any of the five.
    ///
    /// `own` is the reading seat as the last view had it, and its
    /// [`crate::interaction::loss_lines`] follow (#83): why this seat went
    /// out, and that the house answered its last decision where it did. The
    /// other seats' reasons stay on the end screen, which has a line apiece
    /// for them; this is one line of status.
    pub fn stand_up_after(
        &mut self,
        result: &GameResult,
        seat: PlayerId,
        team: Option<u8>,
        own: Option<&baylee_view::SeatView>,
    ) {
        let verdict = crate::interaction::verdict(self.lang, result, seat, team);
        let why = crate::interaction::ending_reason(self.lang, result)
            .into_iter()
            .chain(
                own.map(|own| crate::interaction::loss_lines(self.lang, own, None))
                    .into_iter()
                    .flatten(),
            )
            .fold(verdict, |said, more| format!("{said}. {more}"));
        self.leave_the_seat(why, Tone::Note);
    }

    /// What both of the above do, differing only in how it should read.
    fn leave_the_seat(&mut self, why: String, tone: Tone) {
        if matches!(self.screen, Screen::Seated(_)) {
            self.screen = Screen::Table;
        }
        self.busy = false;
        self.refreshing = false;
        self.awaiting = None;
        self.asked_for = None;
        self.write(why, tone);
    }
}
