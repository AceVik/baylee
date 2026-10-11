//! Effects carried by mana after it pays for a spell.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::ids::NameRef;

impl<L: CardLookup> Engine<L> {
    /// Applies spend riders after a payment (uncounterable marks, scry
    /// triggers), for restricted mana and for mana whose rider named the
    /// spell.
    pub(super) fn apply_spend_riders(
        &mut self,
        player: PlayerId,
        spell: ObjectId,
        riders: &[(
            baylee_core::mana::RestrictedMana,
            ObjectId,
            baylee_cards_dsl::SpendRider,
        )],
    ) {
        // Each unit spent is its own rider: a replacement that made more of
        // the mana made more delayed triggers (CR 106.6a).
        let units = riders
            .iter()
            .flat_map(|(mana, _, rider)| std::iter::repeat_n(rider, usize::from(mana.amount)));
        for rider in units {
            match rider {
                baylee_cards_dsl::SpendRider::None => {}
                baylee_cards_dsl::SpendRider::Uncounterable => {
                    if let Some(obj) = self.state.object_mut(spell)
                        && !obj.riders.contains(&crate::object::Rider::Uncounterable)
                    {
                        obj.riders.push(crate::object::Rider::Uncounterable);
                    }
                }
                baylee_cards_dsl::SpendRider::Scry(n) => {
                    let fx: &'static [baylee_cards_dsl::Effect] =
                        if *n >= 2 { &SCRY_TWO } else { &SCRY_ONE };
                    // A copied spell has no card, and the rider it was cast
                    // with is still the rider it was cast with. The guard
                    // that used to stand here dropped the scry rather than
                    // write a handle it had nothing to put in.
                    let card = self
                        .state
                        .object(spell)
                        .and_then(|o| o.card)
                        .map(|c| c.index);
                    let name = self
                        .state
                        .object(spell)
                        .map_or(NameRef::new(0), |o| o.base.name);
                    let base = self.state.bare_base(name);
                    let id = self.state.arena.insert_with(|id| {
                        GameObject::new_ability_on_stack(
                            id,
                            player,
                            crate::object::AbilityLoc {
                                card,
                                index: baylee_core::ids::AbilityRef::SYNTHETIC,
                                source: spell,
                            },
                            SmallVec::new(),
                            base,
                        )
                    });
                    self.synthetic_fx.insert(id, fx);
                    self.state
                        .zones
                        .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
                }
            }
        }
    }
}

static SCRY_ONE: [baylee_cards_dsl::Effect; 1] = [baylee_cards_dsl::Effect::Scry {
    amount: baylee_cards_dsl::Amount::Fixed(1),
}];
static SCRY_TWO: [baylee_cards_dsl::Effect; 1] = [baylee_cards_dsl::Effect::Scry {
    amount: baylee_cards_dsl::Amount::Fixed(2),
}];
