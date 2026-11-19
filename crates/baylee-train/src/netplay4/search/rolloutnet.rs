//! A search whose rollouts another, cheaper net plays (`eval4 --rollout-net`):
//! the searching net decides at the root, ranks its answers and gives the
//! prior, as without it; inside the worlds every chair is played by the
//! rollout net, from a memory of its own that has watched the game beside
//! the searching net's. Where a rollout ends, its leaf is judged as
//! [`RolloutLeaf4`] says.
//!
//! The rollout net is a [`NetPlayer4`] like the searching one, on a batched
//! session of its own, so its rollout players are twins of it
//! ([`NetPlayer4::twin_on_shared_net`], or [`NetPlayer4::lockstep_twins`]).

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use std::collections::BTreeMap;

use super::{Answer4, NetPlayer4, PlayerSnapshot4, RootDecision4, SearchPlayer4, ViewContext4};
use crate::features4::SourceKind4;

/// How a rollout's leaf is scored when another net plays the rollout.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloutLeaf4 {
    /// The rollout net's own value of the seat's latest observation: no
    /// step of the searching net in a world.
    Student,
    /// The searching net's value of the seat's last frame, observed once
    /// from its memory at the root: one step of it a rollout.
    Jump,
    /// The searching net's value with its memory moved by every frame the
    /// seat sees in the rollout, as a search whose rollouts it plays itself
    /// scores them: one step of it a frame.
    Follow,
}

impl RolloutLeaf4 {
    /// Every leaf, in order of cost.
    pub const ALL: [Self; 3] = [Self::Student, Self::Jump, Self::Follow];

    /// The name run metadata, reports and the command line use.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Student => "student",
            Self::Jump => "jump",
            Self::Follow => "follow",
        }
    }
}

impl std::str::FromStr for RolloutLeaf4 {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> anyhow::Result<Self> {
        Self::ALL
            .into_iter()
            .find(|leaf| leaf.name() == text)
            .ok_or_else(|| {
                anyhow::anyhow!("a rollout leaf is student, jump or follow, not {text:?}")
            })
    }
}

/// A net a search plays with: lent by the caller at the root, owned by a
/// rollout player.
enum Held<'a> {
    Lent(&'a mut NetPlayer4),
    Owned(Box<NetPlayer4>),
}

impl Held<'_> {
    const fn get(&self) -> &NetPlayer4 {
        match self {
            Self::Lent(net) => net,
            Self::Owned(net) => net,
        }
    }

    fn get_mut(&mut self) -> &mut NetPlayer4 {
        match self {
            Self::Lent(net) => net,
            Self::Owned(net) => net,
        }
    }
}

/// Both nets' per-game state at a root.
pub struct RolloutSnapshot4 {
    net: PlayerSnapshot4,
    rollout: PlayerSnapshot4,
}

/// The searching net and its rollout net as one [`SearchPlayer4`]: at the
/// root the searching net answers; a rollout player made from it plays
/// every chair with the rollout net and scores its leaf as `leaf` says.
pub struct RolloutNet4<'a> {
    /// The searching net: the root's decision, prior and ranking; in a
    /// rollout player, the leaf's judge (absent when the leaf is the
    /// rollout net's own).
    net: Option<Held<'a>>,
    /// The rollout net, which has watched the game beside `net`.
    rollout: Held<'a>,
    leaf: RolloutLeaf4,
    /// Whether this is a rollout player (every chair is the rollout net's)
    /// or the root's.
    in_rollout: bool,
    /// The seat the rollout searches for, once the rollout names it
    /// ([`SearchPlayer4::reset_other_seats_for_rollout`]); the leaf judge
    /// watches only its frames.
    seat: Option<PlayerId>,
    /// For [`RolloutLeaf4::Jump`]: each seat's latest view in this rollout.
    last_views: BTreeMap<u8, PlayerView>,
}

impl<'a> RolloutNet4<'a> {
    /// The root's player: `net` searches, `rollout` plays its rollouts, each
    /// leaf scored as `leaf` says. Both must read the same views.
    ///
    /// # Errors
    /// When the two nets read different views of the table.
    pub fn new(
        net: &'a mut NetPlayer4,
        rollout: &'a mut NetPlayer4,
        leaf: RolloutLeaf4,
    ) -> anyhow::Result<Self> {
        anyhow::ensure!(
            <NetPlayer4 as SearchPlayer4>::view_context(net)
                == <NetPlayer4 as SearchPlayer4>::view_context(rollout),
            "the rollout net reads other views than the searching net"
        );
        Ok(Self {
            net: Some(Held::Lent(net)),
            rollout: Held::Lent(rollout),
            leaf,
            in_rollout: false,
            seat: None,
            last_views: BTreeMap::new(),
        })
    }

    /// The searching net; a rollout player whose leaf is the rollout net's
    /// own holds none.
    fn net(&self) -> &NetPlayer4 {
        self.net
            .as_ref()
            .map(Held::get)
            .expect("the root holds the searching net")
    }

    fn net_mut(&mut self) -> &mut NetPlayer4 {
        self.net
            .as_mut()
            .map(Held::get_mut)
            .expect("the root holds the searching net")
    }

    /// The net that answers here: the rollout net in a rollout, the
    /// searching net at the root.
    fn acting(&mut self) -> &mut NetPlayer4 {
        if self.in_rollout {
            self.rollout.get_mut()
        } else {
            self.net_mut()
        }
    }

    /// A rollout player on `rollout`, with the searching net's twin in
    /// `snapshot`'s state as the leaf's judge where the leaf needs one;
    /// `None` when the searching net lends no twin.
    fn rollout_player_of(&self, rollout: NetPlayer4, snapshot: &RolloutSnapshot4) -> Option<Self> {
        let net = match self.leaf {
            RolloutLeaf4::Student => None,
            RolloutLeaf4::Jump | RolloutLeaf4::Follow => Some(Held::Owned(Box::new(
                self.net().twin_on_shared_net(&snapshot.net)?,
            ))),
        };
        Some(Self {
            net,
            rollout: Held::Owned(Box::new(rollout)),
            leaf: self.leaf,
            in_rollout: true,
            seat: None,
            last_views: BTreeMap::new(),
        })
    }
}

impl SearchPlayer4 for RolloutNet4<'_> {
    type Snapshot = RolloutSnapshot4;

    fn snapshot(&self) -> RolloutSnapshot4 {
        RolloutSnapshot4 {
            net: self.net().snapshot(),
            rollout: self.rollout.get().snapshot(),
        }
    }
    fn restore(&mut self, snapshot: &RolloutSnapshot4) {
        if let Some(net) = self.net.as_mut() {
            net.get_mut().restore(&snapshot.net);
        }
        self.rollout.get_mut().restore(&snapshot.rollout);
        self.last_views.clear();
    }
    fn view_context(&self) -> ViewContext4 {
        <NetPlayer4 as SearchPlayer4>::view_context(self.rollout.get())
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        if !self.in_rollout {
            // At the root both nets watch, as at the table.
            self.net_mut().observe(view)?;
            return self.rollout.get_mut().observe(view);
        }
        self.rollout.get_mut().observe(view)?;
        if self.seat.is_some_and(|seat| seat != view.seat) {
            return Ok(());
        }
        match self.leaf {
            RolloutLeaf4::Student => {}
            RolloutLeaf4::Jump => {
                self.last_views.insert(view.seat.get(), view.clone());
            }
            RolloutLeaf4::Follow => self.net_mut().observe(view)?,
        }
        Ok(())
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        if self.in_rollout && self.leaf == RolloutLeaf4::Student {
            self.rollout.get().observed_value(seat)
        } else {
            self.net().observed_value(seat)
        }
    }
    fn leaf_value(&mut self, seat: PlayerId) -> Option<f32> {
        if self.in_rollout && self.leaf == RolloutLeaf4::Jump {
            // The searching net sees the seat's last frame once, from its
            // memory at the root. Observing it again changes nothing.
            if let Some(view) = self.last_views.get(&seat.get()).cloned()
                && let Err(error) = self.net_mut().observe(&view)
            {
                // As a failed rollout observation: the leaf has no value.
                eprintln!("[search] the jump leaf could not be observed: {error:#}");
                return None;
            }
        }
        self.observed_value(seat)
    }
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        <NetPlayer4 as SearchPlayer4>::decide(self.acting(), view, pending, teams)
    }
    fn conclude(&mut self, view: &PlayerView, decision: &RootDecision4, action: &PlayerAction) {
        <NetPlayer4 as SearchPlayer4>::conclude(self.acting(), view, decision, action);
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4 {
        <NetPlayer4 as SearchPlayer4>::answer(self.acting(), view, pending, teams)
    }
    fn reseed_sampling(&mut self, seed: u64) {
        self.rollout.get_mut().reseed_sampling(seed);
        if let Some(net) = self.net.as_mut() {
            net.get_mut().reseed_sampling(seed);
        }
    }
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4) {
        <NetPlayer4 as SearchPlayer4>::attribute_to_search(self.net_mut(), pending, from);
    }
    fn reset_other_seats_for_rollout(&mut self, searching_seat: PlayerId) {
        self.seat = Some(searching_seat);
        self.rollout
            .get_mut()
            .reset_other_seats_for_rollout(searching_seat);
    }
    fn lends_rollout_players(&self) -> bool {
        self.rollout.get().is_on_shared_net()
            && (self.leaf == RolloutLeaf4::Student || self.net().is_on_shared_net())
    }
    fn rollout_player(&self, snapshot: &RolloutSnapshot4) -> Option<Self> {
        let rollout = self.rollout.get().twin_on_shared_net(&snapshot.rollout)?;
        self.rollout_player_of(rollout, snapshot)
    }
    fn lockstep_rollout_players(
        &self,
        snapshot: &RolloutSnapshot4,
        count: usize,
    ) -> Option<Vec<Self>> {
        let rollouts = self
            .rollout
            .get()
            .lockstep_twins(&snapshot.rollout, count)?;
        rollouts
            .into_iter()
            .map(|rollout| self.rollout_player_of(rollout, snapshot))
            .collect()
    }
    fn plays_its_own_rollouts(&self) -> bool {
        false
    }
}
