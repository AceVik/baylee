//! Language-model chairs at a table this client hosts (`docs/llm-seat.md`
//! §"A language model at your table"): the host picks, for an open chair, a
//! profile of the settings file and the model and effort over it, and this
//! client starts a seat bridge (`baylee-seat join … --tethered`) that takes
//! that chair. Before the game a change starts the chair's bridge again;
//! during it, in a debug build only, the running bridge is told and plays
//! the new mind from its next decision (`Order`).
//!
//! Every decision is `baylee_client_core::llmseat::seating` and tested
//! there; this is the shell round it: the file read, the bridges run, the
//! local server asked what models it has, and the room compared with the
//! plan once a frame (cheap: a few comparisons and no allocation while
//! nothing is to be started or stopped).
//!
//! Desktop and a gateway only: a browser and a phone have no bridge beside
//! them, and an offline table (`LocalHost`) is the house's alone.

use baylee_client_core::llmseat::models::{Resolved, listing_url, parse_listing};
pub(crate) use baylee_client_core::llmseat::seating::Phase;
use baylee_client_core::llmseat::seating::{
    ChairModel, Change, DECKS, LIVE_CHANGES, Launch, Refusal, Running, Seating, Step, bridge_args,
    endpoint, models_for, resolved,
};
use baylee_client_core::llmseat::{Profile, SeatSettings};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

/// The room a reconcile is about: what the lobby lists of it.
pub(crate) struct Room<'a> {
    /// Its id.
    pub(crate) id: &'a str,
    /// The gateway's address.
    pub(crate) gateway: &'a str,
    /// Waiting for players, or playing.
    pub(crate) phase: Phase,
    /// The chairs no one sits in and the host has left open.
    pub(crate) open: Vec<u32>,
}

/// What the host can press for a language-model chair.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LlmPress {
    /// Seat a language model here, playing the file's default profile (or
    /// its first).
    Plan,
    /// Play the profile at this place of [`TableSeats::profiles`].
    Profile(usize),
    /// Play the model at this place of [`TableSeats::models`].
    Model(usize),
    /// Think at the effort at this place of the model's efforts; `None` is
    /// the model's own.
    Effort(Option<usize>),
    /// Bring the acceptance deck at this place of [`DECKS`].
    Deck(usize),
    /// Keep the chair's model and effort as its profile's.
    Save,
    /// No language model here any more (before the game).
    Remove,
    /// The house plays this chair from its next decision (during a debug
    /// build's game).
    House,
}

/// What listings came back, by address, waiting to be taken.
type Listings = Arc<Mutex<Vec<(String, Vec<String>)>>>;

/// The settings file as last read.
struct File {
    path: String,
    settings: Result<Option<SeatSettings>, String>,
}

/// Language-model chairs of the room this client hosts.
#[derive(Default)]
pub(crate) struct TableSeats {
    seating: Seating,
    file: Option<File>,
    /// Chairs the house plays now on a debug order.
    house_now: BTreeSet<u32>,
    /// What to say under a chair: a refusal, or what its bridge said last.
    said: BTreeMap<u32, String>,
    /// The password of the room this client opened last, for its bridges
    /// (handed over in their environment, never their arguments).
    password: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    bridges: BTreeMap<u32, (u64, crate::seatbin::Bridge)>,
    /// The plan versions whose bridge did not start: not tried again.
    #[cfg(not(target_arch = "wasm32"))]
    failed: BTreeMap<u32, u64>,
    /// What a server on this machine listed, by its listing's address.
    listed: BTreeMap<String, Vec<String>>,
    /// Listings asked for, answered or not.
    asked: BTreeSet<String>,
    answers: Listings,
    /// Counts every change the in-game panel draws, so it rebuilds only on
    /// one.
    pub(crate) revision: u64,
}

/// Whether this client can seat a language model at all: a desktop, its
/// store opened (never in a test), and a bridge beside it.
pub(crate) fn available() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        static FOUND: OnceLock<bool> = OnceLock::new();
        crate::seatpanel::DESKTOP
            && crate::settings::store_is_open()
            && *FOUND.get_or_init(|| crate::seatbin::program().is_some())
    }
    #[cfg(target_arch = "wasm32")]
    false
}

/// The provider's address variable on this machine (the bridge inherits it).
fn env_base(profile: &Profile) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        profile
            .provider
            .base_env()
            .and_then(|name| std::env::var(name).ok())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = profile;
        None
    }
}

impl TableSeats {
    /// What the chair plays, when a language model is planned there.
    pub(crate) fn planned(&self, chair: u32) -> Option<&ChairModel> {
        self.seating.chair(chair).map(|p| &p.model)
    }

    /// The deck the chair brings.
    pub(crate) fn deck(&self, chair: u32) -> Option<&str> {
        self.seating.chair(chair).map(|p| p.deck.as_str())
    }

    /// Whether the house plays the chair now, on a debug order.
    pub(crate) fn house_now(&self, chair: u32) -> bool {
        self.house_now.contains(&chair)
    }

    /// Every chair a language model is planned for.
    pub(crate) fn chairs(&self) -> Vec<u32> {
        self.seating.chairs().map(|(chair, _)| chair).collect()
    }

    /// No language model plays `chair` any more; its bridge stops before
    /// the game.
    pub(crate) fn unplan(&mut self, chair: u32) {
        if self.seating.chair(chair).is_some() {
            self.seating.unplan(chair);
            self.said.remove(&chair);
            self.revision += 1;
        }
    }

    /// Remembers the password of a room this client is opening.
    pub(crate) fn remember_password(&mut self, password: &str) {
        self.password = (!password.is_empty()).then(|| password.to_string());
    }

    /// Reads the settings file again.
    pub(crate) fn read_file(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use baylee_client_core::llmseat::store::{configured_path, load};
            use baylee_client_core::userdirs::{Os, real_env};
            self.file = configured_path(Os::current(), &real_env).map(|path| File {
                settings: load(&path),
                path: path.display().to_string(),
            });
        }
    }

    /// The file's profiles, by name, in the file's order.
    pub(crate) fn profiles(&self) -> Vec<(&str, &Profile)> {
        match self.file.as_ref().map(|f| &f.settings) {
            Some(Ok(Some(settings))) => settings
                .profiles
                .iter()
                .map(|(name, profile)| (name.as_str(), profile))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Why the file gives no profile, for the line under the chair.
    pub(crate) fn file_refused(&self) -> Option<&str> {
        match self.file.as_ref().map(|f| &f.settings) {
            Some(Err(why)) => Some(why),
            _ => None,
        }
    }

    fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| p)
    }

    /// The models the chair is offered, its profile's endpoint's: known by
    /// this build, listed by a server on this machine, and the one it plays.
    pub(crate) fn models(&self, chair: u32) -> Vec<Resolved> {
        let Some(model) = self.planned(chair) else {
            return Vec::new();
        };
        let Some(profile) = self.profile(&model.profile) else {
            return Vec::new();
        };
        let base = env_base(profile);
        let listed = self.listed_for(profile, base.as_deref());
        let mut out = models_for(profile, base.as_deref(), listed);
        if !out.iter().any(|m| m.id == model.model) {
            out.push(resolved(profile, base.as_deref(), &model.model, listed));
        }
        out
    }

    /// The model the chair plays, resolved.
    pub(crate) fn current(&self, chair: u32) -> Option<Resolved> {
        let model = self.planned(chair)?;
        let profile = self.profile(&model.profile)?;
        let base = env_base(profile);
        let listed = self.listed_for(profile, base.as_deref());
        Some(resolved(profile, base.as_deref(), &model.model, listed))
    }

    fn listed_for(&self, profile: &Profile, base: Option<&str>) -> &[String] {
        listing_url(endpoint(profile, base))
            .and_then(|url| self.listed.get(&url))
            .map_or(&[], Vec::as_slice)
    }

    /// What to say under the chair.
    pub(crate) fn said(&self, chair: u32) -> Option<String> {
        if let Some(said) = self.said.get(&chair) {
            return Some(said.clone());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((_, bridge)) = self.bridges.get(&chair) {
            return bridge.last_line();
        }
        None
    }

    /// Does what a press on chair `chair` asks, `seat_is_ai` whether the
    /// gateway has the house in it now (a language model needs it open
    /// first: the caller makes it so when this says `true`).
    pub(crate) fn press(&mut self, chair: u32, press: LlmPress, phase: Phase, level: &str) -> bool {
        self.said.remove(&chair);
        self.revision += 1;
        let mut open_it = false;
        match press {
            LlmPress::Plan => {
                self.read_file();
                let first = match self.file.as_ref().map(|f| &f.settings) {
                    Some(Ok(Some(settings))) => settings
                        .default
                        .clone()
                        .filter(|d| settings.profiles.contains_key(d))
                        .or_else(|| settings.profiles.keys().next().cloned()),
                    _ => None,
                };
                if let Some(name) = first
                    && let Some(profile) = self.profile(&name).cloned()
                {
                    self.seating
                        .plan(chair, ChairModel::of(&name, &profile), None);
                    open_it = true;
                } else {
                    self.said
                        .insert(chair, no_profile_line(self.file_refused()));
                }
            }
            LlmPress::Profile(at) => {
                let picked = self
                    .profiles()
                    .get(at)
                    .map(|(name, profile)| ChairModel::of(name, profile));
                if let Some(model) = picked {
                    self.change(chair, Some(model), phase, level);
                }
            }
            LlmPress::Model(at) => {
                let models = self.models(chair);
                if let (Some(choice), Some(mut model)) =
                    (models.get(at), self.planned(chair).cloned())
                {
                    model.choose_model(choice);
                    self.change(chair, Some(model), phase, level);
                }
            }
            LlmPress::Effort(at) => {
                if let (Some(current), Some(mut model)) =
                    (self.current(chair), self.planned(chair).cloned())
                {
                    let effort = at.and_then(|at| current.efforts.get(at).copied());
                    if model.choose_effort(&current, effort) {
                        self.change(chair, Some(model), phase, level);
                    }
                }
            }
            LlmPress::Deck(at) => {
                if phase == Phase::Waiting
                    && let (Some(deck), Some(model)) = (DECKS.get(at), self.planned(chair).cloned())
                {
                    self.seating.plan(chair, model, Some(deck));
                }
            }
            LlmPress::Save => self.save(chair),
            LlmPress::Remove => {
                if phase == Phase::Waiting {
                    self.seating.unplan(chair);
                }
            }
            LlmPress::House => self.change(chair, None, phase, level),
        }
        open_it
    }

    /// Changes what the chair plays: a new plan before the game, an order
    /// to its bridge during a debug build's.
    fn change(&mut self, chair: u32, to: Option<ChairModel>, phase: Phase, level: &str) {
        let house = to.is_none();
        match self.seating.change(chair, to, level, phase, LIVE_CHANGES) {
            Ok(Change::Reseat) => {}
            Ok(Change::Order(order)) => {
                #[cfg(not(target_arch = "wasm32"))]
                let sent = self
                    .bridges
                    .get_mut(&chair)
                    .ok_or_else(|| "no bridge plays this chair".to_string())
                    .and_then(|(_, bridge)| bridge.order(&order.line()));
                #[cfg(target_arch = "wasm32")]
                let sent: Result<(), String> = Err(format!("{order:?}"));
                match sent {
                    Ok(()) if house => {
                        self.house_now.insert(chair);
                    }
                    Ok(()) => {
                        self.house_now.remove(&chair);
                    }
                    Err(why) => {
                        self.said.insert(chair, why);
                    }
                }
            }
            Err(Refusal::NotLive) => {
                self.said.insert(
                    chair,
                    "a release build changes no mind during a game".to_string(),
                );
            }
            Err(Refusal::NotOurs) => {
                self.said
                    .insert(chair, "no language model of this client plays here".into());
            }
        }
    }

    /// Keeps the chair's model and effort as its profile's, in the file.
    fn save(&mut self, chair: u32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(model) = self.planned(chair).cloned() else {
                return;
            };
            self.read_file();
            let Some(File {
                path,
                settings: Ok(Some(settings)),
            }) = self.file.as_mut()
            else {
                self.said
                    .insert(chair, "the settings file could not be read".into());
                return;
            };
            let Some(profile) = settings.profiles.get_mut(&model.profile) else {
                self.said.insert(
                    chair,
                    format!("the file has no profile «{}»", model.profile),
                );
                return;
            };
            model.save_into(profile);
            let path = std::path::PathBuf::from(&*path);
            let saved = baylee_client_core::llmseat::store::save(&path, settings);
            self.said.insert(
                chair,
                match saved {
                    Ok(()) => format!("kept as the profile «{}»", model.profile),
                    Err(why) => why,
                },
            );
            self.read_file();
        }
        #[cfg(target_arch = "wasm32")]
        let _ = chair;
    }

    /// Compares the room with the plan: enters or leaves it, starts the
    /// bridges a chair waits for and stops those whose plan moved on, and
    /// asks a server on this machine what it lists. `None` is no room of
    /// this client's: every bridge stops.
    pub(crate) fn reconcile(&mut self, room: Option<Room<'_>>) {
        self.land_listings();
        let Some(room) = room else {
            if self.seating.room().is_some() {
                self.seating.leave();
                self.house_now.clear();
                self.said.clear();
                self.revision += 1;
            }
            self.run(Phase::Waiting, &[], None, None);
            return;
        };
        if self.seating.room() != Some(room.id) {
            self.seating.enter(room.id);
            self.house_now.clear();
            self.said.clear();
            self.read_file();
            self.revision += 1;
        }
        self.ask_listings();
        self.run(room.phase, &room.open, Some(room.id), Some(room.gateway));
    }

    /// Starts and stops what [`Seating::steps`] says.
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    fn run(&mut self, phase: Phase, open: &[u32], room: Option<&str>, gateway: Option<&str>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut running: Vec<Running> = self
                .bridges
                .iter_mut()
                .map(|(chair, (version, bridge))| Running {
                    chair: *chair,
                    version: *version,
                    exited: bridge.exited(),
                })
                .collect();
            // A bridge that did not start counts as one that ended: not
            // started again for that plan, and forgotten with it.
            self.failed.retain(|chair, version| {
                self.seating
                    .chair(*chair)
                    .is_some_and(|planned| planned.version == *version)
            });
            running.extend(self.failed.iter().map(|(chair, version)| Running {
                chair: *chair,
                version: *version,
                exited: true,
            }));
            for step in self.seating.steps(phase, open, &running) {
                self.revision += 1;
                match step {
                    Step::Stop(chair) => {
                        self.bridges.remove(&chair);
                    }
                    Step::Launch(chair) => self.launch(chair, room, gateway),
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn launch(&mut self, chair: u32, room: Option<&str>, gateway: Option<&str>) {
        let (Some(room), Some(gateway)) = (room, gateway) else {
            return;
        };
        let Some(planned) = self.seating.chair(chair) else {
            return;
        };
        let (version, model, deck) = (planned.version, planned.model.clone(), planned.deck.clone());
        let Some(file) = &self.file else {
            return;
        };
        let Some(profile) = self.profile(&model.profile).cloned() else {
            self.said.insert(
                chair,
                format!("the file has no profile «{}»", model.profile),
            );
            return;
        };
        let level = "steady";
        let args = bridge_args(
            &Launch {
                room,
                gateway,
                chair,
                config: &file.path,
                deck: &deck,
                level,
            },
            &model,
            &profile,
        );
        match crate::seatbin::Bridge::start(&args, self.password.as_deref()) {
            Ok(bridge) => {
                self.said.remove(&chair);
                self.bridges.insert(chair, (version, bridge));
            }
            Err(why) => {
                // Not started again for this plan: a bridge that is not
                // there would be asked for every frame.
                self.said.insert(chair, why);
                self.failed.insert(chair, version);
            }
        }
    }

    /// Asks each planned chair's server on this machine what it lists,
    /// once per address.
    fn ask_listings(&mut self) {
        let urls: Vec<String> = self
            .seating
            .chairs()
            .filter_map(|(_, planned)| {
                let profile = self.profile(&planned.model.profile)?;
                let base = env_base(profile);
                listing_url(endpoint(profile, base.as_deref()))
            })
            .filter(|url| !self.asked.contains(url))
            .collect();
        for url in urls {
            self.asked.insert(url.clone());
            let answers = Arc::clone(&self.answers);
            let request = ehttp::Request::get(&url);
            crate::transport::fetch(request, move |result| {
                let ids = result
                    .ok()
                    .filter(|r| r.ok)
                    .and_then(|r| r.text().map(str::to_string))
                    .and_then(|body| parse_listing(&body).ok())
                    .unwrap_or_default();
                answers
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push((url, ids));
            });
        }
    }

    fn land_listings(&mut self) {
        let landed =
            std::mem::take(&mut *self.answers.lock().unwrap_or_else(PoisonError::into_inner));
        for (url, ids) in landed {
            self.listed.insert(url, ids);
            self.revision += 1;
        }
    }
}

/// The line under a chair whose file gives no profile.
fn no_profile_line(refused: Option<&str>) -> String {
    refused.map_or_else(
        || "no language-model profile yet: add one under Settings".to_string(),
        |why| format!("the settings file cannot be used: {why}"),
    )
}

/// Compares the room this client hosts with its language-model chairs,
/// every frame the lobby is up. Through `bypass_change_detection`, so a
/// quiet frame rebuilds nothing; the room page is rebuilt only when a
/// bridge was started or stopped or a listing landed.
pub(crate) fn reconcile(mut state: bevy::prelude::ResMut<crate::lobby::LobbyState>) {
    use bevy::prelude::DetectChangesMut as _;
    if !available() {
        return;
    }
    let lobby_state = state.bypass_change_detection();
    let before = lobby_state.llm.revision;
    let hosted = lobby_state
        .lobby
        .awaiting()
        .filter(|_| !lobby_state.lobby.offline())
        .and_then(|handover| {
            lobby_state
                .lobby
                .games()
                .iter()
                .find(|g| g.id == handover.game_id && g.yours)
        })
        .map(|game| {
            let phase = if game.state == "waiting" {
                Phase::Waiting
            } else {
                Phase::Playing
            };
            let open: Vec<u32> = game
                .seats
                .iter()
                .filter(|s| !s.taken && s.kind == baylee_client_core::lobby::SeatKind::Human)
                .map(|s| s.seat)
                .collect();
            (game.id.clone(), phase, open)
        });
    let gateway = lobby_state.gateway.clone();
    let room = hosted.as_ref().map(|(id, phase, open)| Room {
        id,
        gateway: &gateway,
        phase: *phase,
        open: open.clone(),
    });
    lobby_state.llm.reconcile(room);
    if lobby_state.llm.revision != before {
        state.set_changed();
    }
}
