//! A hosted model in a room's chair (`docs/protocol.md` §"Hosted
//! language-model seats"): a language model the gateway's operator runs on
//! a seat agent, which a registered host orders for an open chair from any
//! platform. This module decides what the room offers and says; the shell
//! draws it and sends the requests.
//!
//! The gateway lists every profile, the unavailable ones too, so a player
//! knows the model exists and when to come back; it refuses a guest's
//! order, and so the room offers a guest none.

use super::{Lobby, LobbyRequest};
use crate::i18n::{Lang, Phrase};
use serde::{Deserialize, Serialize};

/// One row of `GET /lobby/llm-profiles`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostedProfile {
    /// The profile's id, which an order names.
    pub id: String,
    /// What players see.
    pub label: String,
    /// Who the game data goes to.
    pub vendor: String,
    /// `api` or `cli`.
    #[serde(default)]
    pub kind: String,
    /// The model.
    #[serde(default)]
    pub model: String,
    /// `available`, `busy`, `exhausted`, … (the gateway's word).
    #[serde(default)]
    pub state: String,
    /// When an unavailable profile is expected back, Unix seconds.
    #[serde(default)]
    pub until_unix: Option<i64>,
    /// Whether it can be ordered now.
    #[serde(default)]
    pub available: bool,
}

/// What a room's listing says of a chair's hosted model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct HostedChair {
    /// The profile's id.
    pub profile: String,
    /// What players see.
    pub label: String,
    /// Who the game data goes to.
    pub vendor: String,
    /// The model.
    #[serde(default)]
    pub model: String,
    /// `starting`, `ready` or `failed`.
    pub state: String,
    /// Why it failed, for a chair it left open.
    #[serde(default)]
    pub note: Option<String>,
}

impl HostedChair {
    /// Whether the order still stands: the chair is the model's.
    #[must_use]
    pub fn standing(&self) -> bool {
        self.state != "failed"
    }

    /// Its state in words.
    #[must_use]
    pub fn said(&self, lang: Lang) -> String {
        match self.state.as_str() {
            "ready" => Phrase::HostedReady.text(lang).to_string(),
            "failed" => Phrase::HostedFailed.fill(
                lang,
                &[self
                    .note
                    .as_deref()
                    .unwrap_or(Phrase::HostedNoReason.text(lang))],
            ),
            _ => Phrase::HostedStarting.text(lang).to_string(),
        }
    }

    /// Where the game's data goes, said before and while it plays.
    #[must_use]
    pub fn data_goes(&self, lang: Lang) -> String {
        Phrase::HostedDataGoesTo.fill(lang, &[&self.vendor])
    }
}

/// One line of the hosted-model sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfferRow {
    /// The profile's id.
    pub id: String,
    /// `label · vendor`.
    pub words: String,
    /// Whether a press orders it.
    pub available: bool,
    /// Under an unavailable row: when it is back, or that it is not now.
    pub why: Option<String>,
}

/// "hh:mm" of `unix` at `offset` seconds east of UTC.
#[must_use]
pub fn clock(unix: i64, offset: i32) -> String {
    let day = (unix + i64::from(offset)).rem_euclid(86_400);
    format!("{:02}:{:02}", day / 3600, day % 3600 / 60)
}

impl Lobby {
    /// Whether this session may order a hosted model: a registered
    /// account's, at a gateway.
    #[must_use]
    pub fn may_order_hosted(&self) -> bool {
        !self.guest() && !self.offline() && self.has_a_performer()
    }

    /// The profiles the gateway last listed.
    #[must_use]
    pub fn hosted_profiles(&self) -> &[HostedProfile] {
        &self.hosted_profiles
    }

    /// Whether the gateway has answered the list since it was asked.
    #[must_use]
    pub fn hosted_listed(&self) -> bool {
        self.hosted_listed
    }

    /// The sheet's rows: available first, then the rest greyed, each group
    /// by label; an unavailable row says when it is back, if known, at
    /// `offset` seconds east of UTC.
    #[must_use]
    pub fn hosted_offer(&self, offset: i32) -> Vec<OfferRow> {
        let lang = self.lang();
        let mut rows: Vec<&HostedProfile> = self.hosted_profiles.iter().collect();
        rows.sort_by(|a, b| {
            b.available
                .cmp(&a.available)
                .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
                .then_with(|| a.id.cmp(&b.id))
        });
        rows.into_iter()
            .map(|p| OfferRow {
                id: p.id.clone(),
                words: format!("{} · {}", p.label, p.vendor),
                available: p.available,
                why: (!p.available).then(|| {
                    p.until_unix.map_or_else(
                        || Phrase::HostedNotNow.text(lang).to_string(),
                        |until| Phrase::HostedBackAt.fill(lang, &[&clock(until, offset)]),
                    )
                }),
            })
            .collect()
    }

    /// Asks the gateway which profiles it offers (`GET /lobby/llm-profiles`).
    pub fn list_hosted(&mut self) -> Option<LobbyRequest> {
        if self.busy || !self.may_order_hosted() {
            return None;
        }
        self.busy = true;
        self.refreshing = true;
        self.hosted_listed = false;
        Some(LobbyRequest::HostedProfiles)
    }

    /// The host orders `profile` for chair `seat`. Refused here for a
    /// guest, and for a profile the list does not offer now.
    pub fn order_hosted(
        &mut self,
        game_id: &str,
        seat: u32,
        profile: &str,
    ) -> Option<LobbyRequest> {
        if self.busy || !self.may_order_hosted() {
            return None;
        }
        if !self
            .hosted_profiles
            .iter()
            .any(|p| p.id == profile && p.available)
        {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::OrderHosted {
            game_id: game_id.to_string(),
            seat,
            profile: profile.to_string(),
        })
    }

    /// The host takes a hosted chair back before the game.
    pub fn cancel_hosted(&mut self, game_id: &str, seat: u32) -> Option<LobbyRequest> {
        if self.busy || !self.has_a_performer() || self.offline() {
            return None;
        }
        self.busy = true;
        self.note(Phrase::ArrangingTable);
        Some(LobbyRequest::CancelHosted {
            game_id: game_id.to_string(),
            seat,
        })
    }

    pub(super) fn hosted_listed_now(&mut self, profiles: Vec<HostedProfile>) {
        self.hosted_profiles = profiles;
        self.hosted_listed = true;
    }
}
