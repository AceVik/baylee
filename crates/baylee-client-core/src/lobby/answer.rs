//! What each answer from the gateway does to the lobby ([`Lobby::apply`]).

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

impl Lobby {
    /// Feeds back the outcome of a request, and returns the next one the
    /// lobby wants made. Ending a request always clears [`Lobby::busy`] —
    /// chaining sets it again in the same breath.
    pub fn apply(&mut self, event: LobbyEvent) -> Option<LobbyRequest> {
        let next = self.answer(event);
        // A refresh stays one while its answers chain into further list
        // reads, and ends with the first answer that asks for anything else.
        if !matches!(
            next,
            Some(
                LobbyRequest::ListGames(_) | LobbyRequest::ListDecks | LobbyRequest::HostedProfiles
            )
        ) {
            self.refreshing = false;
        }
        next
    }

    /// [`Lobby::apply`] without the refresh bookkeeping.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per event, read top to bottom: splitting it would \
                  hide which events chain into another request"
    )]
    fn answer(&mut self, event: LobbyEvent) -> Option<LobbyRequest> {
        self.busy = false;
        // Any answer from the gateway, a refusal included, is the gateway
        // answering. Said first, so the event's own sentence can follow it.
        if self.health.unreachable && !matches!(event, LobbyEvent::GatewayLost) {
            self.health.unreachable = false;
            self.note(Phrase::GatewayBack);
        }
        match event {
            LobbyEvent::GatewayLost => {
                self.room_saving = false;
                self.health.unreachable = true;
                self.refuse(Phrase::GatewayUnreachable);
                None
            }
            LobbyEvent::Library(reply) => self.library_reply(reply),
            // Sign-up hands back no token, so the credentials that are still
            // in the form go straight into a log-in.
            LobbyEvent::Registered => {
                self.note(Phrase::AccountCreated);
                // Spent: a key admits an account once, and one left in the
                // box would only be sent again with the next new one.
                self.invite_key.clear();
                self.busy = true;
                Some(LobbyRequest::LogIn {
                    username: self.username.text().trim().to_string(),
                    password: self.password.text().to_string(),
                })
            }
            LobbyEvent::LoggedIn { token, username } => {
                // Whoever signed in, the shell says again whether it was
                // through Central (`set_central_bound`), after this event.
                self.central_bound = false;
                self.performer = Performer::Gateway(token);
                self.discover_seats = true;
                self.reclaimed.clear();
                self.resume_candidates.clear();
                self.password_change = None;
                self.password.clear();
                self.password_again.clear();
                self.screen = Screen::Table;
                // Signed in with an address, the player is told the name the
                // account was given, once (#269): the address stops working
                // at the end of 2026. The field takes the name either way, so
                // the name is what this device remembers.
                let by_address = self.username.text().contains('@');
                match username {
                    Some(name) if by_address => {
                        self.write(Phrase::YourUsernameIs.fill(self.lang, &[&name]), Tone::Note);
                        self.set_field(Field::Username, &name);
                    }
                    Some(name) => {
                        self.note(Phrase::SignedIn);
                        self.set_field(Field::Username, &name);
                    }
                    None => self.note(Phrase::SignedIn),
                }
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::GuestIn(kept) => {
                self.enter_as_guest(kept);
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::LoggedOut => None,
            LobbyEvent::HostedProfiles(profiles) => {
                self.hosted_listed_now(profiles);
                None
            }
            LobbyEvent::Decks(decks) => {
                // The selection follows its deck, not its place: the list is
                // ordered newest save first, so a save moves a deck.
                let chosen = self
                    .deck
                    .and_then(|i| self.decks.get(i))
                    .map(|d| d.id.clone());
                self.decks = decks;
                let at = |id: &str| self.decks.iter().position(|d| d.id == id);
                self.deck = self
                    .use_once_listed
                    .take()
                    .and_then(|id| at(&id))
                    .or_else(|| chosen.and_then(|id| at(&id)))
                    .or_else(|| (!self.decks.is_empty()).then_some(0));
                if self
                    .staged_delete
                    .as_ref()
                    .is_some_and(|id| !self.decks.iter().any(|d| &d.id == id))
                {
                    self.staged_delete = None;
                }
                if let Some(deck_id) = self.copied_deck.take() {
                    self.busy = true;
                    return Some(LobbyRequest::LoadDeck { deck_id });
                }
                if std::mem::take(&mut self.discover_seats) {
                    self.busy = true;
                    return Some(LobbyRequest::ListSeats);
                }
                self.list()
            }
            LobbyEvent::Seats(listing) => {
                self.resume_candidates = listing.seats;
                self.reclaim_a_seat().or_else(|| self.list())
            }
            LobbyEvent::Pool { cards, has_text } => {
                self.builder.set_pool(cards, has_text);
                self.clear_status();
                None
            }
            LobbyEvent::BlockedCards(blocked) => {
                self.builder.set_blocked(blocked);
                None
            }
            LobbyEvent::Printings {
                card,
                printings,
                from_catalog,
            } => {
                self.builder.set_printings(card, printings, from_catalog);
                None
            }
            LobbyEvent::DeckLoaded {
                id,
                name,
                cards,
                sideboard,
                commanders,
            } => {
                self.builder
                    .load(&id, &name, &cards, &sideboard, &commanders);
                self.clear_status();
                // The pool may not have arrived yet — the rows are held by
                // name until it does, which is why loading is safe either way.
                self.needs_pool()
            }
            LobbyEvent::DeckSaved { deck_id } => {
                self.note(Phrase::DeckSaved);
                self.builder.saved(deck_id.as_deref());
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::DeckDeleted => {
                self.note(Phrase::DeckDeleted);
                self.busy = true;
                Some(LobbyRequest::ListDecks)
            }
            LobbyEvent::Left => {
                // The seat we were holding at that table is gone with it, so
                // nothing is being waited for any more.
                self.awaiting = None;
                self.clear_status();
                self.list()
            }
            LobbyEvent::Moved => {
                if self.room_saving {
                    self.room_edit = None;
                    self.room_saving = false;
                }
                self.list()
            }
            LobbyEvent::Games(listing) => {
                self.health.no_agents = !listing.agents_available;
                let kept_own = (!self.search.text().trim().is_empty())
                    .then(|| {
                        self.awaiting.as_ref().and_then(|h| {
                            (!listing.games.iter().any(|g| g.id == h.game_id))
                                .then(|| self.games.iter().find(|g| g.id == h.game_id).cloned())
                                .flatten()
                        })
                    })
                    .flatten();
                self.games = listing.games;
                if let Some(own) = kept_own {
                    self.games.insert(0, own);
                }
                self.total = listing.total;
                self.sync_room_edit();
                self.offset = listing.offset;
                // A seat becomes usable when its table starts playing, which
                // is a different moment from being given the seat — see
                // [`LobbyEvent::Seated`] below. This is the one place that
                // knows the difference, because the listing is what carries
                // the table's state.
                let started = self.awaiting.as_ref().is_some_and(|h| {
                    self.games
                        .iter()
                        .any(|g| g.id == h.game_id && g.state == "playing")
                });
                if started && let Some(handover) = self.awaiting.take() {
                    self.note(Phrase::TakingTheSeat);
                    self.screen = Screen::Seated(handover);
                }
                // Tables close while somebody is reading page three of them.
                // A page past the end of the list is an empty screen with a
                // Back button, which is a worse answer than the first page.
                if self.offset > 0 && self.offset >= self.total {
                    self.offset = 0;
                    return self.list();
                }
                // If discovery found multiple chairs, another may be tried
                // after a stale one was refused. Public ownership alone is
                // insufficient: an eliminated player still owns that chair.
                // A host's room offers the gateway's models from the start
                // (D5): the list is asked as the room opens, behind what a
                // seat needs first.
                self.reclaim_a_seat()
                    .or_else(|| self.awaiting.is_some().then(|| self.needs_pool()).flatten())
                    .or_else(|| self.hosted_for_room())
            }
            LobbyEvent::Seated(handover) => {
                // **A seat is not a game**, and this used to ask only whether
                // the seat was one we had *opened*: `!= Some(Open)` put a
                // table against the house and a room somebody else is
                // hosting in the same branch and sent both straight to the
                // duel. For the house that is right — `mode: "ai"` orders an
                // engine before it answers. For a room it is not: a room
                // starts on two statements by two people (this seat's own
                // `ready`, then the host's `start`), so a player who sat down
                // at one was shown a duel with no game behind it — the sky,
                // two seat mats and nothing else — and no way back to the
                // lobby to give the `ready` it was waiting for. The owner
                // found it by sitting down at a table I was hosting.
                //
                // `join_seat` has said the right thing in a comment the whole
                // time ("sitting down does not begin the game — the seat
                // screen waits either way") and then cleared `asked_for`,
                // which is what put it in the branch that does not wait.
                match self.asked_for.take() {
                    // The house is already at the table.
                    Some(GameMode::Ai) => {
                        self.note(Phrase::TakingTheSeat);
                        self.screen = Screen::Seated(handover);
                        None
                    }
                    // Ours, and empty. Offline nobody is coming — every other
                    // chair is the house already — so what the table waits
                    // for is the player arranging it, and saying "waiting for
                    // an opponent" there would be a sentence about a person
                    // who does not exist.
                    Some(GameMode::Open) => {
                        self.note(if self.offline() {
                            Phrase::TableOpenHouse
                        } else {
                            Phrase::TableOpen
                        });
                        self.awaiting = Some(handover);
                        self.list()
                    }
                    // Somebody else's room, a rematch, or a chair handed back
                    // to a client that restarted. The first two are seats at
                    // a table that has not started, and the player's next
                    // move is in the lobby rather than at the table.
                    //
                    // The third is not: a ticket for a game that is **already
                    // playing** has a game behind it right now, so there is
                    // nothing to wait for and telling that player to go and
                    // press Bereit would be advice about a button that is no
                    // longer there. The listing we are holding is what knows
                    // the difference, and it is fresh — asking for the ticket
                    // is what it answered.
                    None => {
                        // A restart knows which it left: a table or a room.
                        let resumed = self
                            .resuming
                            .take_if(|(id, _)| *id == handover.game_id)
                            .map(|(_, playing)| playing);
                        let live = resumed.unwrap_or_else(|| {
                            self.games
                                .iter()
                                .any(|g| g.id == handover.game_id && g.state == "playing")
                        });
                        if live {
                            self.note(Phrase::TakingTheSeat);
                            self.screen = Screen::Seated(handover);
                            None
                        } else {
                            self.note(Phrase::YouAreSeated);
                            self.awaiting = Some(handover);
                            self.list()
                        }
                    }
                }
            }
            LobbyEvent::AccountDeleted => {
                if self.guest() {
                    self.kept_guest = None;
                }
                self.forget_the_session();
                self.note(Phrase::AccountDeleted);
                None
            }
            LobbyEvent::PasswordChangeNeeded => {
                // The admin's password proved the account; it is kept to
                // send beside the player's own, and the boxes are for that.
                self.password_change = Some(self.password.text().to_string());
                self.password.clear();
                self.password_again.clear();
                self.guest_face = false;
                self.screen = Screen::SignIn { registering: false };
                // The face says why itself (`Phrase::PasswordWasReset`).
                self.clear_status();
                self.focus_on(Field::Password);
                None
            }
            LobbyEvent::Banned { reason, until } => {
                let said = ban_sentence(self.lang, &reason, until.as_deref());
                self.password_change = None;
                // Signed out, but a guest kept here stays kept: its session
                // is the guest, and it is good again when the ban ends.
                if matches!(self.performer, Performer::Gateway(_) | Performer::Guest(_)) {
                    self.forget_the_session();
                } else {
                    self.password.clear();
                    self.password_again.clear();
                }
                self.write(said, Tone::Refusal);
                None
            }
            LobbyEvent::Failed(why) => {
                self.room_saving = false;
                // A refused deletion leaves the session good: the refusal is
                // the confirmation's, and the typed password goes with it.
                if let Some(deletion) = self.deleting.as_mut().filter(|d| d.sent) {
                    deletion.sent = false;
                    deletion.refusal = Some(why.clone());
                    self.account_password.clear();
                }
                self.write(why, Tone::Refusal);
                // A failed fetch may have been the pool's; letting it be asked
                // for again costs one request and un-wedges the builder.
                self.pool_requested = self.builder.loaded();
                None
            }
        }
    }
}
