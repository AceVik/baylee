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
        self.room_boards
            .iter_mut()
            .for_each(super::TextBuffer::clear);
        if self.room_print_target.take().is_some() {
            self.builder.close_picker();
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
        draft.update.setup != game.setup
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
                update
                    .setup
                    .seats
                    .resize_with(update.chairs, RoomSeatSetup::default);
                if template == 2 {
                    for seat in &mut update.setup.seats {
                        seat.permanents = ["Forest", "Island", "Mountain", "Plains", "Swamp"]
                            .map(str::to_string)
                            .to_vec();
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
        let request = self.configure_room(&id, update);
        self.room_saving = request.is_some();
        request
    }
}

impl Lobby {
    /// Permanent search suggestions, bounded to keep the dropdown readable.
    #[must_use]
    pub fn room_matches(&self, seat: u8) -> Vec<usize> {
        let query = self.field(Field::RoomBoard(seat)).trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        self.builder
            .pool()
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.kinds.iter().any(|k| {
                    [
                        "Creature",
                        "Artifact",
                        "Enchantment",
                        "Land",
                        "Planeswalker",
                        "Battle",
                    ]
                    .contains(&k.as_str())
                }) && (c.name.to_lowercase().contains(&query)
                    || c.english_name.to_lowercase().contains(&query))
            })
            .take(8)
            .map(|(at, _)| at)
            .collect()
    }

    fn room_seat_mut(&mut self, seat: u8) -> Option<&mut RoomSeatSetup> {
        let d = self.room_edit.as_mut().filter(|d| d.host)?;
        d.update
            .setup
            .seats
            .resize_with(d.update.chairs, RoomSeatSetup::default);
        d.update.setup.seats.get_mut(usize::from(seat))
    }

    /// Adds one independent copy, preserving per-copy printing and counters.
    pub fn room_add_card(&mut self, seat: u8, slot: usize) {
        if !self.room_matches(seat).contains(&slot) {
            return;
        }
        let Some(card) = self.builder.card(slot) else {
            return;
        };
        let name = card.english_name.clone();
        if let Some(s) = self.room_seat_mut(seat)
            && s.permanents.len() < 32
        {
            s.permanents.push(name);
            s.counters.resize_with(s.permanents.len(), Vec::new);
            self.set_field(Field::RoomBoard(seat), "");
        }
    }

    /// Removes a copy together with its counters.
    pub fn room_remove_card(&mut self, seat: u8, at: usize) {
        if let Some(s) = self.room_seat_mut(seat)
            && at < s.permanents.len()
        {
            s.permanents.remove(at);
            if at < s.counters.len() {
                s.counters.remove(at);
            }
        }
        self.room_print_target = None;
        self.builder.close_picker();
    }

    /// Parses the persisted printing choice of one starting card.
    #[must_use]
    pub fn room_card(&self, seat: u8, at: usize) -> Option<baylee_core::deckrow::Row> {
        let line = self
            .room_draft()?
            .setup
            .seats
            .get(usize::from(seat))?
            .permanents
            .get(at)?;
        baylee_core::deckrow::parse(&format!("1 {line}")).ok()
    }

    /// Opens the existing catalog picker without editing a deck.
    pub fn room_pick_print(&mut self, seat: u8, at: usize) -> Option<LobbyRequest> {
        self.room_edit.as_ref().filter(|d| d.host)?;
        let row = self.room_card(seat, at)?;
        let slot = self.builder.slot_of(&row.name)?;
        self.room_print_target = Some((seat, at));
        self.builder.open_choice_picker(slot, row.print)
    }

    /// Routes catalog confirmation to the room when it owns the picker.
    pub fn room_confirm_print(&mut self) -> bool {
        let Some((seat, at)) = self.room_print_target.take() else {
            return false;
        };
        let print = self.builder.picked_choice();
        if let Some(mut row) = self.room_card(seat, at) {
            row.print = print;
            if let Some(s) = self.room_seat_mut(seat)
                && let Some(line) = s.permanents.get_mut(at)
            {
                *line = row.to_string().trim_start_matches("1 ").to_string();
            }
        }
        self.builder.close_picker();
        true
    }

    /// Closes a room-owned printing dialog without applying its draft.
    pub fn room_close_print(&mut self) {
        self.room_print_target = None;
        self.builder.close_picker();
    }

    /// Adds a counter type; the server validates the engine vocabulary too.
    pub fn room_add_counter(&mut self, seat: u8, at: usize) {
        let kind = self.field(Field::RoomCounter).trim().to_ascii_lowercase();
        if baylee_engine::object::CounterKind::from_setup_name(&kind).is_none() {
            return;
        }
        if let Some(s) = self.room_seat_mut(seat)
            && at < s.permanents.len()
        {
            s.counters.resize_with(s.permanents.len(), Vec::new);
            let counters = &mut s.counters[at];
            if counters.len() < 32 && !counters.iter().any(|c| c.kind == kind) {
                counters.push(baylee_core::preset::StartingCounter { kind, amount: 1 });
                self.set_field(Field::RoomCounter, "");
            }
        }
    }

    /// Edits an initial counter count; zero removes that type.
    pub fn room_counter_step(&mut self, seat: u8, at: usize, counter: usize, delta: i16) {
        if let Some(s) = self.room_seat_mut(seat)
            && let Some(cs) = s.counters.get_mut(at)
            && let Some(c) = cs.get_mut(counter)
        {
            c.amount = c.amount.saturating_add_signed(delta).min(999);
            if c.amount == 0 {
                cs.remove(counter);
            }
        }
    }
}
