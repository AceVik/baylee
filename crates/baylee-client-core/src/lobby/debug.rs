//! Diagnostic summaries must never expose credentials or private request payloads.

use super::{Lobby, LobbyEvent, LobbyRequest, Performer, SeatHandover};
use std::fmt;

impl fmt::Debug for SeatHandover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeatHandover")
            .field("game_id", &self.game_id)
            .field("seat", &self.seat)
            .field("seat_token", &"<redacted>")
            .field("local", &self.local)
            .finish()
    }
}

impl fmt::Debug for Performer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Nobody => "Nobody",
            Self::Gateway(_) => "Gateway(<redacted>)",
            Self::Guest(_) => "Guest(<redacted>)",
            Self::Offline => "Offline",
        })
    }
}

impl fmt::Debug for Lobby {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // An allowlist also protects future form fields and pending requests.
        f.debug_struct("Lobby")
            .field("screen", &self.screen)
            .field("focus", &self.focus)
            .field("busy", &self.busy())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for LobbyRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Exhaustiveness makes new variants choose a diagnostic label explicitly.
        // No request payload is safe to log by default, including private decks.
        let name = match self {
            Self::Library(_) => "Library",
            Self::Register { .. } => "Register",
            Self::LogIn { .. } => "LogIn",
            Self::ChangePassword { .. } => "ChangePassword",
            Self::PlayAsGuest { .. } => "PlayAsGuest",
            Self::LogOut { .. } => "LogOut",
            Self::Depart { .. } => "Depart",
            Self::ListDecks => "ListDecks",
            Self::LoadPool => "LoadPool",
            Self::LoadPrintings { .. } => "LoadPrintings",
            Self::LoadDeck { .. } => "LoadDeck",
            Self::SaveDeck { .. } => "SaveDeck",
            Self::DeleteDeck { .. } => "DeleteDeck",
            Self::DeleteAccount { .. } => "DeleteAccount",
            Self::ListGames(_) => "ListGames",
            Self::CreateGame { .. } => "CreateGame",
            Self::JoinGame { .. } => "JoinGame",
            Self::TakeSeat { .. } => "TakeSeat",
            Self::SetSeat { .. } => "SetSeat",
            Self::ConfigureRoom { .. } => "ConfigureRoom",
            Self::SetReady { .. } => "SetReady",
            Self::StartGame { .. } => "StartGame",
            Self::HandOver { .. } => "HandOver",
            Self::LeaveGame { .. } => "LeaveGame",
            Self::Rematch { .. } => "Rematch",
            Self::HostedProfiles => "HostedProfiles",
            Self::OrderHosted { .. } => "OrderHosted",
            Self::CancelHosted { .. } => "CancelHosted",
        };
        f.debug_struct(name).finish_non_exhaustive()
    }
}

impl fmt::Debug for LobbyEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Library(_) => "Library",
            Self::Registered => "Registered",
            Self::LoggedIn { .. } => "LoggedIn",
            Self::GuestIn(_) => "GuestIn",
            Self::LoggedOut => "LoggedOut",
            Self::Decks(_) => "Decks",
            Self::Pool { .. } => "Pool",
            Self::BlockedCards(_) => "BlockedCards",
            Self::Printings { .. } => "Printings",
            Self::DeckLoaded { .. } => "DeckLoaded",
            Self::DeckSaved { .. } => "DeckSaved",
            Self::DeckDeleted => "DeckDeleted",
            Self::AccountDeleted => "AccountDeleted",
            Self::Left => "Left",
            Self::Games(_) => "Games",
            Self::Moved => "Moved",
            Self::Seated(_) => "Seated",
            Self::HostedProfiles(_) => "HostedProfiles",
            Self::PasswordChangeNeeded => "PasswordChangeNeeded",
            Self::Banned { .. } => "Banned",
            Self::Failed(_) => "Failed",
            Self::GatewayLost => "GatewayLost",
        };
        f.debug_struct(name).finish_non_exhaustive()
    }
}
