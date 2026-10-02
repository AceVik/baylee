//! Actual damage history and death-time lookback, independent of marked damage.

use baylee_cards_dsl::{AbilityDef, Modifier, Trigger, TriggerZone};
use baylee_core::ids::ObjectId;

use crate::event::{DamageTarget, GameEvent};
use crate::object::GameObject;
use crate::state::{CardLookup, GameState};
use crate::trigger::PendingTrigger;
use crate::zone::Zone;

/// A positive damage event's two object identities (CR 400.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DamageRecord {
    source: ObjectId,
    source_version: u32,
    recipient: ObjectId,
    recipient_version: u32,
}

/// One source as it existed immediately before a damaged object's death.
#[derive(Clone, Debug)]
pub(crate) struct DamageObserver {
    pub object: GameObject,
    pub grants: Vec<Modifier>,
    pub times: u32,
}

/// Immutable lookback even if either card moves again before collection.
#[derive(Clone, Debug)]
pub(crate) struct DamageDeath {
    pub seq: u64,
    pub victim: GameObject,
    pub sources: Vec<DamageObserver>,
}

/// A simultaneous move retains both public event data and rules lookback.
#[derive(Clone, Debug)]
pub(crate) struct BattlefieldDeparture {
    pub event: crate::event::Departure,
    pub damage: Option<DamageDeath>,
}

impl GameState {
    /// Recover the identity at an event even if its object moved again in
    /// the same resolution before the trigger scan.
    pub(crate) fn event_object_identity(&self, id: ObjectId, seq: u64) -> Option<(u32, i16)> {
        for entry in self.journal.entries().iter().skip(seq as usize) {
            if matches!(entry.event, GameEvent::ZoneChanged { object, from: Zone::Battlefield, .. } if object == id)
                && let Some(departure) = &entry.departure
            {
                return Some((departure.version, departure.power));
            }
        }
        self.object_or_departed(id)
            .map(|o| (o.version, o.characteristics().power.unwrap_or(0)))
    }

    pub(crate) fn has_damage_history(&self, recipient: ObjectId) -> bool {
        self.per_turn
            .permanent_damage
            .iter()
            .any(|d| d.recipient == recipient)
    }

    pub(crate) fn live_damage_pairs(&self, position: &impl Fn(ObjectId) -> u32) -> Vec<(u32, u32)> {
        let mut pairs: Vec<_> = self
            .per_turn
            .permanent_damage
            .iter()
            .filter(|d| {
                [
                    (d.source, d.source_version),
                    (d.recipient, d.recipient_version),
                ]
                .into_iter()
                .all(|(id, version)| {
                    self.object(id)
                        .is_some_and(|o| o.zone == Zone::Battlefield && o.version == version)
                })
            })
            .map(|d| (position(d.source), position(d.recipient)))
            .collect();
        pairs.sort_unstable();
        pairs
    }

    /// Journal actual positive permanent damage after prevention/redirection,
    /// remembering both incarnations. Combat and resolving effects share it.
    pub(crate) fn record_permanent_damage(
        &mut self,
        source: ObjectId,
        source_version: Option<u32>,
        recipient: ObjectId,
        amount: u32,
        is_combat: bool,
    ) {
        if amount == 0 {
            return;
        }
        if let Some(target) = self
            .object(recipient)
            .filter(|o| o.zone == Zone::Battlefield)
            && let Some(source_version) =
                source_version.or_else(|| self.object(source).map(|o| o.version))
        {
            let damage = DamageRecord {
                source,
                source_version,
                recipient,
                recipient_version: target.version,
            };
            if !self.per_turn.permanent_damage.contains(&damage) {
                self.per_turn.permanent_damage.push(damage);
            }
        }
        self.journal.record(GameEvent::DamageDealt {
            source: Some(source),
            target: DamageTarget::Object(recipient),
            amount,
            is_combat,
        });
    }

    /// CR 603.10a: read the source's ability, controller and identity before
    /// any member of a simultaneous departure leaves. No ability had to
    /// exist when the damage was dealt, and removing marked damage (including
    /// CR 514.2 cleanup) does not erase the historical fact for this turn.
    pub(crate) fn damage_death_snapshot(&self, recipient: ObjectId) -> Option<DamageDeath> {
        let victim = self.object(recipient)?;
        let sources: Vec<_> = self
            .per_turn
            .permanent_damage
            .iter()
            .filter(|damage| {
                damage.recipient == recipient && damage.recipient_version == victim.version
            })
            .filter_map(|damage| {
                let object = self.object(damage.source).filter(|o| {
                    o.zone == Zone::Battlefield
                        && o.version == damage.source_version
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                })?;
                let grants = self
                    .effects
                    .iter()
                    .filter(|fx| {
                        matches!(
                            fx.modifier,
                            Modifier::GrantTriggered {
                                trigger: Trigger::DiesAfterDamageByThis(_),
                                ..
                            }
                        ) && crate::effects::applies_to(self, fx, object)
                    })
                    .map(|fx| fx.modifier)
                    .collect();
                Some(DamageObserver {
                    object: object.clone(),
                    grants,
                    times: crate::trigger::times_triggered(
                        self,
                        baylee_cards_dsl::TriggerEventKind::Any,
                        object.id,
                    ),
                })
            })
            .collect();
        (!sources.is_empty()).then(|| DamageDeath {
            seq: 0,
            victim: victim.clone(),
            sources,
        })
    }
}

/// This trigger consumes captured deaths, never a later object's abilities.
pub(crate) fn collect(
    state: &GameState,
    lookup: &impl CardLookup,
    from_seq: u64,
    out: &mut Vec<PendingTrigger>,
) {
    for death in state.damage_deaths.iter().filter(|d| d.seq > from_seq) {
        for observer in &death.sources {
            let source = &observer.object;
            let list = source.ability_list(lookup);
            for (index, ability) in list.abilities.iter().enumerate() {
                let (AbilityDef::Triggered {
                    trigger,
                    zone,
                    condition,
                    once_per_turn,
                    ..
                }
                | AbilityDef::ModalTriggered {
                    trigger,
                    zone,
                    condition,
                    once_per_turn,
                    ..
                }) = ability
                else {
                    continue;
                };
                if *zone != TriggerZone::Battlefield
                    || !crate::eval::intervening_if(state, *condition, source.controller, source.id)
                    || !matches_death(trigger, state, death, source)
                {
                    continue;
                }
                let mut pending = pending(death, source);
                pending.ability_index = index as u32;
                pending.abilities = Some(list);
                pending.once_per_turn = *once_per_turn;
                out.extend(std::iter::repeat_n(pending, observer.times as usize));
            }
            for grant in &observer.grants {
                let Modifier::GrantTriggered {
                    trigger,
                    effects,
                    target,
                } = grant
                else {
                    continue;
                };
                if matches_death(trigger, state, death, source) {
                    let mut pending = pending(death, source);
                    pending.synthetic_effects = Some(effects);
                    pending.synthetic_target = *target;
                    out.extend(std::iter::repeat_n(pending, observer.times as usize));
                }
            }
        }
    }
}

fn matches_death(
    trigger: &Trigger,
    state: &GameState,
    death: &DamageDeath,
    source: &GameObject,
) -> bool {
    let Trigger::DiesAfterDamageByThis(filter) = trigger else {
        return false;
    };
    crate::eval::matches(filter, state, &death.victim, source.controller, source.id)
}

fn pending(death: &DamageDeath, source: &GameObject) -> PendingTrigger {
    PendingTrigger {
        source: source.id,
        source_version: Some(source.version),
        event_object_identity: Some((
            death.victim.version,
            death.victim.characteristics().power.unwrap_or(0),
        )),
        counter_source_version: None,
        ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
        abilities: None,
        controller: source.controller,
        timestamp: source.timestamp,
        event_object: Some(death.victim.id),
        event_mana_value: Some(death.victim.characteristics().mana_value()),
        event_departure: Some((
            death.victim.controller,
            death.victim.characteristics().toughness.unwrap_or(0),
        )),
        event_mana: None,
        event_damage: None,
        implicit_target: None,
        synthetic_effects: None,
        synthetic_target: None,
        once_per_turn: false,
        chosen_mode: None,
    }
}
