//! The official host has already started this game before it grants its seat.
use super::{Lobby, Screen, SeatHandover};

impl Lobby {
    /// Enter a verified official challenge seat after gateway admission.
    pub fn enter_challenge(&mut self, seat: SeatHandover) {
        self.busy = false;
        self.discover_seats = false;
        self.awaiting = None;
        self.asked_for = None;
        self.screen = Screen::Seated(seat);
    }
}
