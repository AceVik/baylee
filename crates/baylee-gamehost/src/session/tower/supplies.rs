use super::{
    CounterKind, GameEvent, ObjectId, PlayerId, Refused, Session, TowerSupply, ZoneLocation,
};

#[derive(Clone, Copy)]
pub(super) struct Source {
    pub(super) id: ObjectId,
    version: u32,
    charges: u16,
}
pub(super) struct Sources(pub(super) [[Option<Source>; 2]; 2]);

impl Sources {
    pub(super) fn of(session: &Session) -> Self {
        let state = session.engine.state();
        Self(std::array::from_fn(|seat| {
            std::array::from_fn(|kind| {
                let supply = if kind == 0 {
                    TowerSupply::HealingPotion
                } else {
                    TowerSupply::ManaPotion
                };
                let definition = baylee_cards::tokens::tower::definition(supply);
                state
                    .zones
                    .list(ZoneLocation::Command(PlayerId::new(
                        u8::try_from(seat).expect("two seats"),
                    )))
                    .iter()
                    .find_map(|id| {
                        let object = state.object(*id)?;
                        object
                            .token
                            .filter(|token| std::ptr::eq(*token, definition))
                            .map(|_| Source {
                                id: *id,
                                version: object.version,
                                charges: object.counters.get(CounterKind::Charge),
                            })
                    })
            })
        }))
    }

    pub(super) fn consumed(&self, session: &Session) -> Result<[[u8; 2]; 2], Refused> {
        let mut paid = [[0_u16; 2]; 2];
        for entry in session.engine.journal().entries() {
            if let GameEvent::CounterChanged {
                object,
                version,
                kind: CounterKind::Charge,
                old,
                new,
                ..
            } = entry.event
            {
                for (seat, sources) in self.0.iter().enumerate() {
                    for (kind, source) in sources.iter().enumerate() {
                        if let Some(source) = source
                            && source.id == object
                            && source.version == version
                        {
                            // Original command-zone charges can only be spent, never replenished.
                            if old != source.charges - paid[seat][kind]
                                || old.checked_sub(1) != Some(new)
                            {
                                return Err(Refused::Invalid);
                            }
                            paid[seat][kind] += 1;
                        }
                    }
                }
            }
        }
        let mut consumed = [[0; 2]; 2];
        for (out, paid) in consumed
            .iter_mut()
            .flatten()
            .zip(paid.into_iter().flatten())
        {
            *out = u8::try_from(paid).map_err(|_| Refused::Invalid)?;
        }
        Ok(consumed)
    }
}
