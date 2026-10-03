//! Resolved prevention/redirection shields and chosen damage sources.
//!
//! The shared `damage` procedure applies these alongside standing effects
//! using affected-player choices (CR 615.7, 616.1). Shields have stable IDs
//! and provenance, and last until cleanup unless consumed first.

use crate::event::DamageTarget;
use crate::object::ObjectKind;
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

mod store;
pub use store::{ShieldOrigin, ShieldStore};

/// What a shield stands in front of.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Shielded {
    /// A player.
    Player(PlayerId),
    /// A permanent, as the object it was when the shield was made: the id
    /// and the object's version at that moment (CR 400.7).
    Object(ObjectId, u32),
    /// Anything damage can be dealt to (Fog).
    Everything,
}

/// How a shield prevents, and how much of it is left.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ShieldKind {
    /// "Prevent the next N damage" (CR 615.7): each 1 damage prevented
    /// takes 1 off, and the shield is gone at 0.
    Next(u32),
    /// "Prevent all combat damage that would be dealt this turn" (Fog):
    /// every combat damage event, never used up.
    AllCombat,
    /// "The next time a red source of your choice would deal damage to you
    /// this turn, prevent that damage" (CR 615.8): the next instance of
    /// damage from the chosen source, however much, and then it is gone.
    NextFrom {
        /// The source it waits for.
        source: ChosenSource,
        /// How much of that instance is still dealt (Forcefield's "all but
        /// 1"); 0 prevents all of it.
        all_but: u32,
        /// Its controller gains the life it prevented (CR 615.5).
        gain_life: bool,
        /// Only combat damage from that source.
        combat_only: bool,
    },
    /// "The next time a source of your choice would deal damage to target
    /// creature this turn, that source deals that damage to you instead"
    /// (Jade Monolith): not a prevention shield but a redirection one
    /// (CR 614.9), made by a resolved ability like the rest and used up the
    /// same way as a finite prevention effect.
    RedirectNextFrom {
        /// The source it waits for.
        source: ChosenSource,
        /// The player the damage is dealt to instead.
        to: PlayerId,
    },
}

/// The source a "source of your choice" shield waits for (CR 609.7a), and
/// what it must still be when it would deal the damage (CR 609.7b, 615.9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ChosenSource {
    /// The object chosen.
    pub id: ObjectId,
    /// Its version when it was chosen (CR 400.7).
    pub version: u32,
    /// It was a spell on the stack: the permanent it becomes as it resolves
    /// is the same source (CR 609.7a).
    pub was_spell: bool,
    /// What it had to be to be chosen, and must still be.
    pub filter: &'static Filter,
    /// The shield's controller, "you" to `filter`.
    pub you: PlayerId,
    /// The object whose ability made the shield, "this" to `filter`.
    pub this: ObjectId,
}

impl ChosenSource {
    /// Match the actual incarnation dealing damage, including retained LKI
    /// after its card has left and returned (CR 400.7, 609.7a–b).
    pub(crate) fn deals_as(&self, state: &GameState, source: &crate::object::GameObject) -> bool {
        source.id == self.id
            && (source.version == self.version
                || (self.was_spell
                    && source.kind == ObjectKind::Permanent
                    && source.version == self.version + 1))
            && crate::eval::matches_projected(
                self.filter,
                state,
                source,
                source.characteristics(),
                self.you,
                self.this,
            )
    }
    /// `chosen` as it is now, picked by `you` for the ability of `this`.
    #[must_use]
    pub fn new(
        state: &GameState,
        chosen: ObjectId,
        filter: &'static Filter,
        you: PlayerId,
        this: ObjectId,
    ) -> Option<Self> {
        let obj = state.object_or_departed(chosen)?;
        let obj = if matches!(obj.zone, Zone::Battlefield | Zone::Stack) {
            obj
        } else {
            // A departed source named by a waiting ability is the object
            // as it last existed, not the card now in its new zone.
            state
                .damage_sources
                .iter()
                .rev()
                .find(|was| was.id == chosen)
                .unwrap_or(obj)
        };
        Some(Self {
            id: chosen,
            version: obj.version,
            was_spell: obj.zone == Zone::Stack && obj.kind == ObjectKind::Spell,
            filter,
            you,
            this,
        })
    }
}

/// What a player may choose as "a source of your choice" matching
/// `filter` (CR 609.7a): a permanent, a spell on the stack, and the source
/// of an ability on the stack even where that source has since gone (its
/// last known characteristics answer `filter` then).
///
/// Not offered: an object only a waiting replacement or prevention effect
/// or a delayed trigger refers to, a face-up object in the command zone,
/// and a token that has ceased to exist — none of which a card in the pool
/// deals damage from today.
#[must_use]
pub fn source_options(
    state: &GameState,
    filter: &'static Filter,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| crate::eval::matches(filter, state, o, you, this))
        })
        .collect();
    for &id in state.zones.list(ZoneLocation::Stack) {
        let Some(obj) = state.object(id) else {
            continue;
        };
        match obj.kind {
            ObjectKind::Spell => {
                if crate::eval::matches(filter, state, obj, you, this) {
                    out.push(id);
                }
            }
            ObjectKind::AbilityOnStack => {
                let Some(from) = obj.ability.as_ref().map(|loc| loc.source) else {
                    continue;
                };
                let Some(source) = state.object(from) else {
                    continue;
                };
                let fits = if matches!(source.zone, Zone::Battlefield | Zone::Stack) {
                    crate::eval::matches(filter, state, source, you, this)
                } else {
                    state.last_known_characteristics(from).is_some_and(|was| {
                        crate::eval::matches_projected(filter, state, source, was, you, this)
                    })
                };
                if fits {
                    out.push(from);
                }
            }
            _ => {}
        }
    }
    out.sort();
    out.dedup();
    out
}

/// One prevention shield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Shield {
    /// Whom it protects.
    pub protects: Shielded,
    /// How it prevents.
    pub kind: ShieldKind,
    /// The player who controlled the spell or ability that made it.
    pub controller: PlayerId,
}

impl Shielded {
    /// Whether damage to `recipient` is damage to what this shields.
    pub(crate) fn covers(self, state: &GameState, recipient: DamageTarget) -> bool {
        match (self, recipient) {
            (Self::Everything, _) => true,
            (Self::Player(p), DamageTarget::Player(q)) => p == q,
            (Self::Object(id, version), DamageTarget::Object(target)) => {
                id == target && state.object(target).is_some_and(|o| o.version == version)
            }
            _ => false,
        }
    }
}

/// Whether `recipient` is something a redirection may move damage from or
/// to: a player, or a creature on the battlefield (CR 614.9).
pub(crate) fn still_a_creature(state: &GameState, recipient: DamageTarget) -> bool {
    match recipient {
        DamageTarget::Player(_) => true,
        DamageTarget::Object(id) => state.object(id).is_some_and(|obj| {
            obj.zone == Zone::Battlefield
                && obj
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
        }),
    }
}
