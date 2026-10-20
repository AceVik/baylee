//! The account and the session: playing offline, signing out, deleting
//! the account, what the gateway says of this player, and going back to a
//! table.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

impl Lobby {
    /// Goes to the table screen with no account behind it.
    ///
    /// Every screen from here on asks the same questions and reads the same
    /// answers; what differs is only who performs the requests, and that is
    /// the shell's to know. The one thing the lobby learns is that somebody
    /// will — see [`Performer`].
    ///
    /// [`Lobby::token`] stays `None` on purpose, and is a different question
    /// from this one: it is what the shell reads to decide whether to attach
    /// the account's keymap and standing orders, and there is no account here
    /// to attach.
    pub fn play_offline(&mut self) -> Option<LobbyRequest> {
        self.performer = Performer::Offline;
        self.screen = Screen::Table;
        self.note(Phrase::PlayingOffline);
        self.busy = true;
        Some(LobbyRequest::ListDecks)
    }

    /// Signs out: the account is forgotten here, and the request that ends
    /// its session on the gateway is handed back — for a guest, the end of
    /// the guest (#269), which the shell asks the player about first. Offline
    /// there is nothing to end.
    pub fn sign_out(&mut self) -> Option<LobbyRequest> {
        let ending = match &self.performer {
            Performer::Gateway(token) | Performer::Guest(token) => Some(LobbyRequest::LogOut {
                token: token.clone(),
            }),
            Performer::Nobody | Performer::Offline => None,
        };
        if self.guest() {
            self.kept_guest = None;
        }
        self.forget_the_session();
        ending
    }

    /// The request that tells the gateway this player left the table on
    /// purpose ([`LobbyRequest::Depart`]); `None` offline and signed out,
    /// where there is nobody to tell.
    #[must_use]
    pub fn depart(&self) -> Option<LobbyRequest> {
        match &self.performer {
            Performer::Gateway(token) | Performer::Guest(token) => Some(LobbyRequest::Depart {
                token: token.clone(),
            }),
            Performer::Nobody | Performer::Offline => None,
        }
    }

    /// Opens the confirmation that deletes the signed-in account (#292).
    ///
    /// Only a gateway account or a guest has one to delete; offline or
    /// signed out this does nothing. The caret goes into the password box,
    /// which a guest's confirmation neither draws nor asks for.
    pub fn ask_to_delete_account(&mut self) {
        if !matches!(self.performer, Performer::Gateway(_) | Performer::Guest(_)) {
            return;
        }
        self.deleting = Some(Deletion {
            refusal: None,
            sent: false,
            was: self.focus,
        });
        self.account_password.clear();
        self.revealed = None;
        self.focus = Field::AccountPassword;
        self.focus_epoch += 1;
    }

    /// Whether the session was opened through Baylee Central. Set by the
    /// shell when it signs in there; any other sign-in clears it.
    pub fn set_central_bound(&mut self, central: bool) {
        self.central_bound = central;
    }

    /// Whether the gateway session was opened through Baylee Central.
    #[must_use]
    pub fn central_bound(&self) -> bool {
        self.central_bound
    }

    /// Whether confirming the deletion names the password again: a gateway
    /// account's does, a guest's and a Central-bound one's do not (their
    /// session is all they have here, and a Central-bound one was just
    /// checked at Central).
    #[must_use]
    pub fn deletion_needs_password(&self) -> bool {
        matches!(self.performer, Performer::Gateway(_)) && !self.central_bound
    }

    /// The deletion being confirmed, if one is.
    #[must_use]
    pub fn deleting_account(&self) -> Option<&Deletion> {
        self.deleting.as_ref()
    }

    /// What the confirmation names the account by: a guest's handle, or the
    /// name the account signed in with.
    #[must_use]
    pub fn account_name(&self) -> &str {
        match &self.kept_guest {
            Some(kept) if self.guest() => &kept.handle,
            _ => self.username.text().trim(),
        }
    }

    /// Closes the confirmation, deleting nothing. The caret goes back where
    /// it was, and the typed password goes.
    pub fn cancel_account_deletion(&mut self) {
        if let Some(deletion) = self.deleting.take() {
            self.focus = deletion.was;
        }
        self.account_password.clear();
        self.revealed = None;
    }

    /// Sends the deletion the player has confirmed (#292).
    ///
    /// A registered account's password is asked again, because a session
    /// left signed in on somebody else's machine is not enough
    /// (`docs/protocol.md` §"Deleting an account (#292)"). A guest has none,
    /// and its session is enough.
    pub fn delete_account(&mut self) -> Option<LobbyRequest> {
        if self.busy {
            return None;
        }
        let needs_password = self.deletion_needs_password();
        let deletion = self.deleting.as_mut()?;
        let password = match self.performer {
            Performer::Gateway(_) if !needs_password => None,
            Performer::Gateway(_) if self.account_password.is_empty() => {
                deletion.refusal = Some(
                    Phrase::DeleteAccountNeedsPassword
                        .text(self.lang)
                        .to_string(),
                );
                return None;
            }
            Performer::Gateway(_) => Some(self.account_password.text().to_string()),
            Performer::Guest(_) => None,
            Performer::Nobody | Performer::Offline => return None,
        };
        deletion.refusal = None;
        deletion.sent = true;
        self.busy = true;
        Some(LobbyRequest::DeleteAccount { password })
    }

    /// The gateway no longer knows the session held (a `401`). A guest's
    /// ending is the guest's (#269): it is dropped here, and the player is
    /// told, since the one they come back as next is a new one.
    ///
    /// Not a sign-out the player chose (the shell design, §2.5): the front
    /// door says the session ended, and what they were writing is kept —
    /// the builder's draft and the pool it is built from, which the same
    /// gateway hands the next sign-in unchanged.
    pub fn session_ended(&mut self) {
        let guest = self.guest();
        if guest {
            self.kept_guest = None;
        }
        let builder = std::mem::take(&mut self.builder);
        let pool_requested = self.pool_requested;
        self.forget_the_session();
        if guest {
            self.refuse(Phrase::GuestEnded);
        } else {
            self.builder = builder;
            self.pool_requested = pool_requested;
            self.note(Phrase::ShellSessionEnded);
        }
    }

    /// `GET /me`'s answer. Returns whether it changed anything.
    pub fn set_me(&mut self, me: strips::Me) -> bool {
        if self.me.as_ref() == Some(&me) {
            return false;
        }
        self.me = Some(me);
        true
    }

    /// What `GET /me` said, once it has.
    #[must_use]
    pub fn me(&self) -> Option<&strips::Me> {
        self.me.as_ref()
    }

    /// `GET /lobby/stats`'s answer. Returns whether it changed anything.
    pub fn set_stats(&mut self, stats: strips::LobbyStats) -> bool {
        if self.stats == Some(stats) {
            return false;
        }
        self.stats = Some(stats);
        true
    }

    /// What `GET /lobby/stats` said last, once it has.
    #[must_use]
    pub fn stats(&self) -> Option<&strips::LobbyStats> {
        self.stats.as_ref()
    }

    /// Watch a game being played (`docs/protocol.md` §"Spectators"): the
    /// shell dials the watch socket with this account's session. Nothing
    /// is asked of the gateway here; the ticket request is the ask.
    pub fn watch_table(&mut self, game_id: &str) -> bool {
        if self.busy || self.offline() || !matches!(self.screen, Screen::Table) {
            return false;
        }
        let Some(token) = self.token().map(str::to_string) else {
            return false;
        };
        self.screen = Screen::Seated(SeatHandover {
            game_id: game_id.to_string(),
            seat: SeatHandover::WATCH,
            seat_token: token,
            local: false,
        });
        true
    }

    /// Return to a game being played that holds this player's chair (the
    /// seated strip's Return): ask for the seat again, as a client that
    /// restarted does.
    pub fn return_to_game(&mut self, game_id: &str) -> Option<LobbyRequest> {
        if self.busy || self.offline() || !matches!(self.screen, Screen::Table) {
            return None;
        }
        self.busy = true;
        Some(LobbyRequest::TakeSeat {
            game_id: game_id.to_string(),
        })
    }

    /// A client restarted into an update goes back to its chair at
    /// `game_id`: a table being played (`playing`) or a waiting room. Asked
    /// for at the first listing after signing in, before any other chair
    /// ([`Lobby::reclaim_a_seat`]), with a ticket the gateway hands the
    /// account afresh; the old one died with the old client.
    pub fn resume_seat(&mut self, game_id: &str, playing: bool) {
        self.resuming = Some((game_id.to_string(), playing));
        self.reclaimed.retain(|id| id != game_id);
    }

    /// A client restarted into an update sits down again at the house's
    /// table it rebuilt from its record: offline, at that seat, so that the
    /// end of the game comes back to the offline lobby as it would have.
    pub fn resume_offline_table(&mut self, handover: SeatHandover) {
        self.performer = Performer::Offline;
        self.busy = false;
        self.note(Phrase::TakingTheSeat);
        self.screen = Screen::Seated(handover);
    }

    /// Forgets the account and everything it bought.
    pub(super) fn forget_the_session(&mut self) {
        self.me = None;
        self.stats = None;
        // Which hosted models are open is the account's own.
        self.hosted_profiles.clear();
        self.hosted_listed = false;
        self.hosted_asked_room = None;
        self.deleting = None;
        self.account_password.clear();
        self.close_library();
        self.copied_deck = None;
        self.use_once_listed = None;
        // The shell flushes a waiting deletion before it signs out; one
        // that is left here belongs to an account that is gone.
        self.staged_delete = None;
        self.recent.clear();
        self.pending_setup = None;
        self.performer = Performer::Nobody;
        self.decks.clear();
        self.games.clear();
        self.reclaimed.clear();
        self.resume_candidates.clear();
        self.discover_seats = false;
        self.resuming = None;
        self.deck = None;
        self.busy = false;
        self.refreshing = false;
        self.awaiting = None;
        self.asked_for = None;
        self.rematch_wanted = None;
        self.password.clear();
        self.password_again.clear();
        self.revealed = None;
        self.focus = Field::Username;
        self.caret_parked = false;
        self.guest_face = false;
        self.screen = Screen::SignIn { registering: false };
        self.builder.forget_pool();
        self.pool_requested = false;
        self.note(Phrase::SignedOut);
    }
}
