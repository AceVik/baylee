//! Full pending destruction state, including immutable battlefield lookback.
use super::{Hasher, hash_characteristics, hash_object};
use std::hash::Hash;

pub(super) fn hash_choices(h: &mut Hasher, choices: &crate::sba::DamageClearingChoices) {
    choices.entries().count().hash(h);
    for attempt in choices.entries() {
        attempt.reference.hash(h);
        attempt.effect.hash(h);
        attempt.regeneratable.hash(h);
        attempt.by_effect.hash(h);
        attempt.departure.is_some().hash(h);
        if let Some(departure) = &attempt.departure {
            departure.subtypes.hash(h);
            departure.event.hash(h);
            departure.damage.is_some().hash(h);
            if let Some(death) = &departure.damage {
                hash_death(h, death);
            }
        }
    }
}

pub(super) fn hash_death(h: &mut Hasher, death: &crate::damage_history::DamageDeath) {
    death.seq.hash(h);
    hash_object(h, &death.victim);
    hash_characteristics(h, death.victim.characteristics());
    death.sources.len().hash(h);
    for observer in &death.sources {
        hash_object(h, &observer.object);
        hash_characteristics(h, observer.object.characteristics());
        observer.grants.hash(h);
        observer.text.hash(h);
        observer.times.hash(h);
    }
}
