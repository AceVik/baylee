//! Effects carried by mana after it pays for a spell.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::mana_provenance::ManaProvenance;
use crate::object::Rider;
use baylee_cards_dsl::{Amount, CounterKind, Duration, Effect, KeywordSet, Modifier, SpendRider};

impl<L: CardLookup> Engine<L> {
    /// Payment has finished and the spell is on the stack. Delayed triggers
    /// join the normal trigger queue so they resolve above that spell.
    pub(super) fn apply_spend_riders(
        &mut self,
        player: PlayerId,
        spell: ObjectId,
        riders: &[(baylee_core::mana::RestrictedMana, ObjectId, SpendRider)],
    ) {
        for (mana, _, rider) in riders {
            if *rider == SpendRider::None {
                continue;
            }
            let Some(provenance) = self
                .state
                .restriction_info
                .get(&mana.restriction.0)
                .copied()
            else {
                continue;
            };
            let commander_casts = (player == provenance.controller)
                .then(|| {
                    self.state.commanders[usize::from(player.get())]
                        .iter()
                        .find(|commander| commander.object == spell)
                        .map(|commander| commander.casts)
                })
                .flatten();
            // Every unit carries a separate effect or delayed trigger, even
            // when a replacement made several units at once (CR 106.6a).
            for _ in 0..mana.amount {
                match *rider {
                    SpendRider::None => {}
                    SpendRider::Uncounterable => {
                        if let Some(object) = self.state.object_mut(spell)
                            && !object.riders.contains(&Rider::Uncounterable)
                        {
                            object.riders.push(Rider::Uncounterable);
                        }
                    }
                    SpendRider::Scry(amount) => self.queue_mana_scry(provenance, u32::from(amount)),
                    SpendRider::CommanderScry => {
                        if let Some(amount) = commander_casts {
                            self.queue_mana_scry(provenance, amount);
                        }
                    }
                    SpendRider::EnterCounters(amount) => self.mana_entry_counters(spell, amount),
                    SpendRider::CommanderEnterCounters => {
                        if let Some(amount) = commander_casts {
                            self.mana_entry_counters(spell, amount);
                        }
                    }
                    SpendRider::Haste => self.mana_haste(spell, provenance.controller),
                    SpendRider::Copy => self.queue_mana_copy(provenance, spell),
                }
            }
        }
    }

    /// Payment riders are captured before a sacrifice can remove the source.
    /// Mana abilities never call this: they do not create a stack object.
    pub(super) fn apply_ability_spend_riders(&mut self, player: PlayerId, ability: ObjectId) {
        let riders = self
            .state
            .object_mut(ability)
            .and_then(|object| object.paid.as_mut())
            .map(|paid| std::mem::take(&mut paid.spend_riders))
            .unwrap_or_default();
        self.apply_spend_riders(player, ability, &riders);
    }

    fn queue_mana_copy(&mut self, provenance: ManaProvenance, original: ObjectId) {
        let Some(object) = self.state.object(original) else {
            return;
        };
        let identity = Some((object.version, object.characteristics().power.unwrap_or(0)));
        self.trigger_queue.push_back(trigger::PendingTrigger {
            source: provenance.context.source,
            source_version: provenance.source_version,
            counter_source_version: None,
            related_objects: None,
            event_object_identity: identity,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            text: provenance.context.text,
            controller: provenance.controller,
            timestamp: provenance.timestamp,
            event_object: Some(original),
            event_mana_value: None,
            event_departure: None,
            event_mana: None,
            event_amount: None,
            event_player: None,
            implicit_target: None,
            synthetic_effects: Some(&MANA_COPY),
            synthetic_target: None,
            once_per_turn: false,
            chosen_mode: None,
        });
    }

    fn mana_entry_counters(&mut self, spell: ObjectId, amount: u32) {
        if amount != 0
            && let Some(object) = self.state.object_mut(spell)
        {
            object.riders.push(Rider::EntryP1P1(amount));
        }
    }

    fn mana_haste(&mut self, spell: ObjectId, controller: PlayerId) {
        let modifier = Modifier::AddKeyword(KeywordSet::HASTE);
        let timestamp = self.state.next_timestamp();
        let filter = crate::effects::EffectFilter::object(&self.state, spell);
        // A continuous effect on this spell follows it into the battlefield
        // (CR 400.7a), and participates in layer-six timestamp ordering.
        self.state
            .effects
            .register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: None,
                controller,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp,
                duration: Duration::Indefinitely,
                filter,
                modifier,
            });
    }

    fn queue_mana_scry(&mut self, provenance: ManaProvenance, amount: u32) {
        // A delayed trigger belongs to the original mana ability's controller
        // and source, including after that source leaves (CR 603.7e).
        self.trigger_queue.push_back(trigger::PendingTrigger {
            source: provenance.context.source,
            source_version: provenance.source_version,
            counter_source_version: None,
            related_objects: None,
            event_object_identity: None,
            ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
            abilities: None,
            text: provenance.context.text,
            controller: provenance.controller,
            timestamp: provenance.timestamp,
            event_object: None,
            event_mana_value: None,
            event_departure: None,
            event_mana: None,
            event_amount: Some(amount),
            event_player: None,
            implicit_target: None,
            synthetic_effects: Some(&MANA_SCRY),
            synthetic_target: None,
            once_per_turn: false,
            chosen_mode: None,
        });
    }

    /// Entry replacement effects apply once, before the first SBA and ETB
    /// triggers. A later incarnation has already discarded these markers.
    pub(super) fn apply_mana_entry_counters(&mut self, id: ObjectId, from: Zone) -> bool {
        if from != Zone::Stack {
            return false;
        }
        let amounts: SmallVec<[u32; 2]> =
            self.state.object(id).map_or_else(SmallVec::new, |object| {
                object
                    .riders
                    .iter()
                    .filter_map(|rider| match rider {
                        Rider::EntryP1P1(amount) => Some(*amount),
                        _ => None,
                    })
                    .collect()
            });
        if amounts.is_empty() {
            return false;
        }
        if let Some(object) = self.state.object_mut(id) {
            object
                .riders
                .retain(|rider| !matches!(rider, Rider::EntryP1P1(_)));
        }
        for amount in amounts {
            let Ok(amount) = u16::try_from(amount) else {
                self.state.numeric_failure = Some("mana entry counter amount exceeds u16");
                return true;
            };
            crate::replacement::put_counters(&mut self.state, id, CounterKind::P1P1, amount);
        }
        true
    }
}

static MANA_SCRY: [Effect; 1] = [Effect::Scry {
    amount: Amount::EventAmount,
}];

static MANA_COPY: [Effect; 1] = [Effect::CopyEventSpellOrAbility];
