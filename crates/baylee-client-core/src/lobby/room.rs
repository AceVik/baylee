//! Editable room setup. A draft is local until the host applies it.
use super::{Field, Lobby, LobbyRequest};
use baylee_core::preset::{RoomSeatSetup, RoomSetup, RoomUpdate};

#[derive(Clone, Debug)]
pub(super) struct Draft {
    id: String,
    pub(super) host: bool,
    pub update: RoomUpdate,
}

/// A bounded adjustment to one room setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adjustment {
    /// Add/remove one chair.
    Chairs(bool),
    /// Change shared life by the given amount.
    Life(i32),
    /// Change the free redraw count by one.
    Mulligans(bool),
    /// Change one seat's life by the given amount; zero resets its override.
    SeatLife(u8, i32),
    /// Load a named starting template: Commander, duel, or accelerated.
    Template(u8),
}

impl Lobby {
    pub(super) fn sync_room_edit(&mut self) {
        let Some(ticket) = self.awaiting.as_ref() else {
            self.room_edit = None;
            return;
        };
        let Some(game) = self.games.iter().find(|g| g.id == ticket.game_id) else {
            return;
        };
        if self
            .room_edit
            .as_ref()
            .is_some_and(|d| d.id == game.id && d.host == game.yours)
        {
            return;
        }
        let game = game.clone();
        self.set_field(Field::RoomName, &game.name);
        self.room_password.clear();
        for at in 0..8 {
            let names = game
                .setup
                .seats
                .get(at)
                .map(|s| s.permanents.join("; "))
                .unwrap_or_default();
            self.set_field(Field::RoomBoard(at as u8), &names);
        }
        self.room_edit = Some(Draft {
            id: game.id,
            host: game.yours,
            update: RoomUpdate {
                name: game.name,
                chairs: game.seats.len(),
                password: None,
                setup: game.setup,
            },
        });
    }

    /// The local host draft; visitors read the authoritative summary instead.
    #[must_use]
    pub fn room_draft(&self) -> Option<&RoomUpdate> {
        self.room_edit.as_ref().map(|d| &d.update)
    }

    /// Whether the host is looking at unapplied rules or text edits.
    #[must_use]
    pub fn room_dirty(&self) -> bool {
        let Some(draft) = self.room_edit.as_ref().filter(|d| d.host) else {
            return false;
        };
        let Some(game) = self.games.iter().find(|g| g.id == draft.id) else {
            return false;
        };
        if draft.update.chairs != game.seats.len()
            || self.field(Field::RoomName).trim() != game.name
            || !self.field(Field::RoomPassword).is_empty()
            || draft.update.setup.starting_life != game.setup.starting_life
            || draft.update.setup.free_mulligans != game.setup.free_mulligans
        {
            return true;
        }
        (0..draft.update.chairs).any(|at| {
            let before = game.setup.seats.get(at).cloned().unwrap_or_default();
            let after = draft
                .update
                .setup
                .seats
                .get(at)
                .cloned()
                .unwrap_or_default();
            before.life != after.life
                || before.permanents
                    != self
                        .field(Field::RoomBoard(at as u8))
                        .split(';')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
        })
    }

    /// Edits a bounded setting without sending a partial room update.
    pub fn adjust_room(&mut self, change: Adjustment) {
        let Some(draft) = self.room_edit.as_mut().filter(|d| d.host) else {
            return;
        };
        let update = &mut draft.update;
        match change {
            Adjustment::Chairs(more) => {
                update.chairs = if more {
                    (update.chairs + 1).min(8)
                } else {
                    update.chairs.saturating_sub(1).max(2)
                };
                update.setup.seats.truncate(update.chairs);
            }
            Adjustment::Life(delta) => {
                update.setup.starting_life = (update.setup.starting_life + delta).clamp(1, 999);
            }
            Adjustment::Mulligans(more) => {
                update.setup.free_mulligans = if more {
                    (update.setup.free_mulligans + 1).min(7)
                } else {
                    update.setup.free_mulligans.saturating_sub(1)
                }
            }
            Adjustment::SeatLife(at, delta) => {
                update
                    .setup
                    .seats
                    .resize_with(update.chairs, RoomSeatSetup::default);
                if let Some(seat) = update.setup.seats.get_mut(usize::from(at)) {
                    seat.life = (delta != 0).then(|| {
                        (seat.life.unwrap_or(update.setup.starting_life) + delta).clamp(1, 999)
                    });
                }
            }
            Adjustment::Template(template) => {
                update.setup = RoomSetup {
                    starting_life: if template == 1 { 20 } else { 40 },
                    ..RoomSetup::default()
                };
                for at in 0..8 {
                    self.room_boards[at].clear();
                    if template == 2 {
                        self.set_field(
                            Field::RoomBoard(at as u8),
                            "Forest; Island; Mountain; Plains; Swamp",
                        );
                    }
                }
            }
        }
    }

    /// Applies the host's full draft. Blank password preserves an existing lock.
    pub fn save_room(&mut self, remove_password: bool) -> Option<LobbyRequest> {
        let draft = self.room_edit.as_ref().filter(|d| d.host)?;
        let id = draft.id.clone();
        let mut update = draft.update.clone();
        update.name = self.field(Field::RoomName).trim().to_string();
        let password = self.field(Field::RoomPassword).to_string();
        update.password = if remove_password {
            Some(String::new())
        } else {
            (!password.is_empty()).then_some(password)
        };
        update
            .setup
            .seats
            .resize_with(update.chairs, RoomSeatSetup::default);
        for (at, seat) in update.setup.seats.iter_mut().enumerate() {
            seat.permanents = self
                .field(Field::RoomBoard(at as u8))
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        let request = self.configure_room(&id, update);
        self.room_saving = request.is_some();
        request
    }
}
