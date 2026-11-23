//! Event cosmetics are scoped to the current participant. Ranked never opens
//! the host origin, while an ordinary guest uses only the original home seat.
use super::{
    LobbyState,
    advent::{DecodeCasual, ForgetMatch, LoadScoped},
    central,
    ranked::native::RankedSocket,
};
use crate::{DuelPhase, event_cosmetics::MatchLooks, net::SeatTicket};
use baylee_account::{
    Error, GatewayHttp,
    advent::{self, AssetMeta, AssetScope, Equipment, Id, Kind, SeatLooks},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::Zeroizing;

/// Run on the image thread. Each page occupies a single account-worker job so
/// presence, gameplay and chat can run between pages even on a slow connection.
pub(super) fn load_asset(
    account: &central::Handle,
    scope: AssetScope,
    meta: &AssetMeta,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, Error> {
    meta.validate()?;
    let (actual, bytes) = baylee_account::sdk::advent::read_asset(scope, None, |request| {
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Conflict);
        }
        let response = account.call(move |a| a.advent(request))?;
        if let advent::Response::Asset { page } = &response
            && page.asset != *meta
        {
            return Err(Error::Conflict);
        }
        Ok(response)
    })?;
    if cancelled.load(Ordering::Acquire) || !account.is_active() || actual != *meta {
        return Err(Error::Conflict);
    }
    let bytes = bytes.ok_or(Error::Rejected)?;
    baylee_account::check_advent_image(meta, &bytes)?;
    Ok(bytes)
}

#[derive(Resource)]
pub(super) struct HomeSeat {
    http: Arc<GatewayHttp>,
    origin: String,
    epoch: u64,
    game: Uuid,
    bearer: Zeroizing<String>,
}

impl HomeSeat {
    pub(super) fn matches(&self, origin: &str, epoch: u64, game: Uuid, bearer: &str) -> bool {
        self.origin == origin
            && self.epoch == epoch
            && self.game == game
            && self.bearer.as_str() == bearer
    }
    pub(super) fn from_ticket(ticket: &SeatTicket, epoch: u64) -> Option<Self> {
        let game = Uuid::parse_str(&ticket.game_id).ok()?;
        if ticket.watch
            || game.get_version_num() != 7
            || game.get_variant() != uuid::Variant::RFC4122
            || ticket.seat_token.len() != 64
            || !ticket
                .seat_token
                .bytes()
                .all(|b| matches!(b,b'0'..=b'9'|b'a'..=b'f'))
        {
            return None;
        }
        let http = if ticket.gateway.starts_with("https://") {
            GatewayHttp::live(&ticket.gateway)
        } else {
            GatewayHttp::development(&ticket.gateway)
        }
        .ok()?;
        Some(Self {
            origin: ticket.gateway.clone(),
            epoch,
            game,
            http: Arc::new(http),
            bearer: Zeroizing::new(ticket.seat_token.clone()),
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Ranked(Id, u64),
    Home(Id, u64),
}
impl Scope {
    const fn id(self) -> Id {
        match self {
            Self::Ranked(id, _) | Self::Home(id, _) => id,
        }
    }
}

type Slot = (u8, Kind, AssetMeta);
enum Reply {
    Looks {
        seats: Box<[SeatLooks; 8]>,
        presented: bool,
    },
    File {
        slot: Slot,
        bytes: Vec<u8>,
    },
}
type Answer = Result<Reply, Error>;

#[derive(Resource, Default)]
struct Bridge {
    scope: Option<Scope>,
    account: Option<central::Handle>,
    inbox: Option<Mutex<mpsc::Receiver<Answer>>>,
    active: Arc<AtomicBool>,
    next: Option<Instant>,
    look_sent: bool,
    known: Vec<Slot>,
    queue: VecDeque<Slot>,
    delivered: Vec<Slot>,
}

#[derive(SystemParam)]
struct Outputs<'w> {
    looks: ResMut<'w, MatchLooks>,
    forget: MessageWriter<'w, ForgetMatch>,
    load: MessageWriter<'w, LoadScoped>,
    decode: MessageWriter<'w, DecodeCasual>,
}

impl Bridge {
    fn forget(&mut self, out: &mut Outputs) {
        if let Some(scope) = self.scope {
            out.forget.write(ForgetMatch(scope.id()));
        }
        self.known.clear();
        self.queue.clear();
        self.delivered.clear();
        *out.looks = MatchLooks::default();
    }

    fn reset(&mut self, next: Option<Scope>, account: Option<central::Handle>, out: &mut Outputs) {
        self.active.store(false, Ordering::Release);
        self.forget(out);
        *self = Self {
            scope: next,
            account,
            active: Arc::new(AtomicBool::new(true)),
            ..Default::default()
        };
    }

    fn apply(&mut self, scope: Scope, answer: Answer, out: &mut Outputs, now: Instant) {
        let table = scope.id();
        match answer {
            Ok(Reply::Looks { seats, presented }) => {
                self.look_sent |= presented;
                let mut slots = Vec::new();
                for (seat, looks) in (0u8..8).zip(seats.iter()) {
                    if let Some(a) = looks.sleeve {
                        slots.push((seat, Kind::Sleeve, a));
                    }
                    if let Some(a) = looks.playmat {
                        slots.push((seat, Kind::Playmat, a));
                    }
                }
                if self.known != slots {
                    // A revoked or replaced slot must also drop its private texture.
                    if self.known.iter().any(|s| !slots.contains(s)) {
                        self.forget(out);
                    }
                    *out.looks = MatchLooks::participant(table, slots.iter().copied());
                    self.known = slots;
                }
                for &(seat, kind, asset) in &self.known {
                    let slot = (seat, kind, asset);
                    match scope {
                        Scope::Ranked(..) => {
                            out.load.write(LoadScoped {
                                scope: AssetScope::Equipped {
                                    match_id: table,
                                    seat,
                                    kind,
                                },
                                asset,
                            });
                        }
                        Scope::Home(..)
                            if !self.delivered.contains(&slot) && !self.queue.contains(&slot) =>
                        {
                            self.queue.push_back(slot);
                        }
                        Scope::Home(..) => {}
                    }
                }
                self.next = Some(now + Duration::from_secs(5));
            }
            Ok(Reply::File {
                slot: (seat, kind, asset),
                bytes,
            }) if self.known.contains(&(seat, kind, asset)) => {
                out.decode.write(DecodeCasual {
                    scope: AssetScope::Equipped {
                        match_id: table,
                        seat,
                        kind,
                    },
                    asset,
                    bytes,
                });
                self.delivered.push((seat, kind, asset));
            }
            Ok(Reply::File { .. }) => {}
            Err(_) => {
                self.forget(out);
                self.next = Some(now + Duration::from_secs(5));
            }
        }
    }

    fn dispatch(
        &mut self,
        scope: Scope,
        home: Option<&HomeSeat>,
        look: Option<Equipment>,
        now: Instant,
    ) {
        let metadata = self.next.is_none_or(|time| now >= time);
        if !metadata && self.queue.is_empty() {
            return;
        }
        let (send, receive) = mpsc::sync_channel(1);
        let sent = match scope {
            Scope::Ranked(match_id, _) => {
                let Some(account) = &self.account else { return };
                let look = look.filter(|_| !self.look_sent);
                account.submit(move |a| {
                    // Dressing failures are cosmetic; a participant can still
                    // see other authorized seats and play the game.
                    let presented = look.is_none_or(|look| {
                        matches!(
                            a.advent(advent::Request::Present { match_id, look }),
                            Ok(advent::Response::Presented)
                        )
                    });
                    let result = a
                        .advent(advent::Request::TableLooks { match_id })
                        .and_then(|r| {
                            let advent::Response::TableLooks { seats } = r else {
                                return Err(Error::Rejected);
                            };
                            let mut all = [SeatLooks::default(); 8];
                            all[..2].copy_from_slice(&seats);
                            Ok(Reply::Looks {
                                seats: Box::new(all),
                                presented,
                            })
                        });
                    let _ = send.send(result);
                })
            }
            Scope::Home(..) => {
                let Some(home) = home else { return };
                let http = home.http.clone();
                let game = home.game;
                let token = home.bearer.clone();
                let active = self.active.clone();
                let slot = if metadata {
                    None
                } else {
                    self.queue.pop_front()
                };
                std::thread::Builder::new()
                    .name("event-table-art".into())
                    .spawn(move || {
                        let result = match slot {
                            None => {
                                http.advent_table_looks(game, &token)
                                    .map(|seats| Reply::Looks {
                                        seats: Box::new(seats),
                                        presented: false,
                                    })
                            }
                            Some(slot @ (seat, kind, asset)) => http
                                .advent_table_asset(game, &token, seat, kind, &asset, || {
                                    active.load(Ordering::Acquire)
                                })
                                .map(|bytes| Reply::File { slot, bytes }),
                        };
                        let _ = send.send(result);
                    })
                    .is_ok()
            }
        };
        if sent {
            self.inbox = Some(Mutex::new(receive));
        } else {
            self.next = Some(now + Duration::from_secs(1));
        }
    }
}

fn poll(
    state: Res<LobbyState>,
    phase: Res<State<DuelPhase>>,
    ranked: Option<Res<RankedSocket>>,
    home: Option<Res<HomeSeat>>,
    mut bridge: ResMut<Bridge>,
    mut out: Outputs,
) {
    let playing = matches!(*phase.get(), DuelPhase::Opening | DuelPhase::Playing);
    let account = state.central_handle().filter(central::Handle::is_active);
    let scope = if playing {
        if let Some(r) = ranked.as_deref() {
            account
                .as_ref()
                .map(|_| Scope::Ranked(r.match_id, r.generation))
        } else {
            home.as_deref().filter(|h|
                matches!(state.lobby.screen(),baylee_client_core::lobby::Screen::Seated(s)
                    if !s.local && !s.watching() && Uuid::try_parse(&s.game_id).is_ok_and(|g|h.matches(&state.gateway,state.gateway_epoch,g,&s.seat_token))))
                .map(|h|Scope::Home(*h.game.as_bytes(),h.epoch))
        }
    } else {
        None
    };
    let same_account = match (&bridge.account, &account) {
        (None, None) => true,
        (Some(a), Some(b)) => a.same_session(b),
        _ => false,
    };
    let replaced_home = matches!(scope, Some(Scope::Home(..)))
        && home.as_ref().is_some_and(DetectChanges::is_changed);
    if replaced_home
        || scope != bridge.scope
        || !same_account && (bridge.scope.is_some() || scope.is_some())
    {
        bridge.reset(scope, account, &mut out);
    }
    let Some(scope) = scope else { return };
    let now = Instant::now();
    if let Some(receive) = bridge.inbox.as_mut() {
        let result = match receive.get_mut().map(|rx| rx.try_recv()) {
            Ok(Ok(answer)) => answer,
            Ok(Err(mpsc::TryRecvError::Empty)) => return,
            _ => Err(Error::Unavailable),
        };
        bridge.inbox = None;
        bridge.apply(scope, result, &mut out, now);
    }
    bridge.dispatch(
        scope,
        home.as_deref(),
        ranked.as_deref().and_then(|r| r.look),
        now,
    );
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Bridge>()
        .init_resource::<MatchLooks>()
        .add_systems(Update, poll);
    app.add_systems(OnEnter(DuelPhase::Finished), clear_home);
    app.add_systems(OnEnter(DuelPhase::Closed), clear_home);
}
fn clear_home(mut commands: Commands) {
    commands.remove_resource::<HomeSeat>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PlayerId;

    #[test]
    fn ordinary_art_uses_only_the_origin_epoch_and_capability_of_its_actual_home_seat() {
        let game = Uuid::now_v7();
        let mut t = SeatTicket {
            gateway: "http://127.0.0.1:49134".into(),
            game_id: game.to_string(),
            seat: PlayerId::new(0),
            seat_token: "a".repeat(64),
            watch: false,
        };
        let h = HomeSeat::from_ticket(&t, 4).unwrap();
        assert!(h.matches(&t.gateway, 4, game, &t.seat_token));
        assert!(!h.matches("http://127.0.0.1:49135", 4, game, &t.seat_token));
        assert!(!h.matches(&t.gateway, 5, game, &t.seat_token));
        assert!(!h.matches(&t.gateway, 4, Uuid::now_v7(), &t.seat_token));
        assert!(!h.matches(&t.gateway, 4, game, &"b".repeat(64)));
        t.watch = true;
        assert!(HomeSeat::from_ticket(&t, 4).is_none());
        t.watch = false;
        t.gateway = "http://foreign.invalid".into();
        assert!(HomeSeat::from_ticket(&t, 4).is_none());
    }
}
