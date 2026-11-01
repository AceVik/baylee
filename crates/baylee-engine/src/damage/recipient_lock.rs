//! An instruction-local restriction is reread after every damage redirection.
use crate::{event::DamageTarget, state::GameState, text_changes::RuleContext};
use baylee_cards_dsl::Filter;
use baylee_core::ids::PlayerId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct RecipientLock {
    pub filter: &'static Filter,
    pub you: PlayerId,
    pub context: RuleContext,
}

impl RecipientLock {
    pub(super) fn matches(self, state: &GameState, recipient: DamageTarget) -> bool {
        let DamageTarget::Object(id) = recipient else {
            return false;
        };
        state.object(id).is_some_and(|object| {
            crate::eval::matches_with_context(self.filter, state, object, self.you, self.context)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::{Assignment, DamageWork};
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::prevention::{ChosenSource, RedirectDestination, Shield, ShieldKind, Shielded};
    use crate::zone::ZoneLocation;
    use baylee_core::ids::ObjectId;
    use baylee_core::types::TypeSet;
    const YOU: PlayerId = PlayerId::new(0);
    const THEM: PlayerId = PlayerId::new(1);
    fn state() -> GameState {
        GameState::from_preset(&preset(723, &[]), &SyntheticLookup::new(vec![])).unwrap()
    }
    fn creature(state: &mut GameState, owner: PlayerId) -> ObjectId {
        let name = state.names.intern("Recipient restriction fixture");
        let id = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let base = state.object_mut(id).unwrap().base_mut();
        base.types = TypeSet::CREATURE;
        base.power = Some(3);
        base.toughness = Some(8);
        id
    }
    fn work(state: &mut GameState, source: ObjectId, recipient: DamageTarget) -> DamageWork {
        let context = crate::eval::live_context(state, source);
        let mut work = DamageWork::new(
            state,
            vec![Assignment {
                source,
                source_version: None,
                recipient,
                amount: 3,
                is_combat: false,
            }],
        );
        work.recipient_lock = Some(RecipientLock {
            filter: &Filter::CREATURE,
            you: YOU,
            context,
        });
        work
    }
    #[test]
    fn recipient_restriction_leaves_player_damage_preventable_but_bypasses_creature_shield() {
        for is_creature in [false, true] {
            let mut state = state();
            let source = creature(&mut state, YOU);
            let target = creature(&mut state, THEM);
            let reference = state.source_identity(target).unwrap();
            state.shields.push(Shield {
                protects: if is_creature {
                    Shielded::Object(reference.object, reference.version)
                } else {
                    Shielded::Player(THEM)
                },
                kind: ShieldKind::Next(3),
                controller: THEM,
            });
            let recipient = if is_creature {
                DamageTarget::Object(target)
            } else {
                DamageTarget::Player(THEM)
            };
            assert!(
                work(&mut state, source, recipient)
                    .advance(&mut state)
                    .is_none()
            );
            assert_eq!(state.players[1].life, 20);
            assert_eq!(
                state.object(target).unwrap().damage,
                if is_creature { 3 } else { 0 }
            );
            assert_eq!(state.shields.len(), usize::from(is_creature));
        }
    }
    #[test]
    fn recipient_restriction_applies_after_redirection_and_stops_further_redirection() {
        let mut state = state();
        let source = creature(&mut state, YOU);
        let target = creature(&mut state, THEM);
        let alternate = creature(&mut state, THEM);
        let to = state.source_identity(target).unwrap();
        let alternate = state.source_identity(alternate).unwrap();
        let chosen = ChosenSource::new(
            &state,
            state.source_identity(source).unwrap(),
            &Filter::Any,
            THEM,
            target,
        )
        .unwrap();
        state.shields.push(Shield {
            protects: Shielded::Player(THEM),
            kind: ShieldKind::RedirectSource {
                source: chosen,
                to: RedirectDestination::Object(to),
                next: false,
            },
            controller: THEM,
        });
        state.shields.push(Shield {
            protects: Shielded::Object(to.object, to.version),
            kind: ShieldKind::Next(3),
            controller: THEM,
        });
        state.shields.push(Shield {
            protects: Shielded::Object(to.object, to.version),
            kind: ShieldKind::RedirectSource {
                source: chosen,
                to: RedirectDestination::Object(alternate),
                next: false,
            },
            controller: THEM,
        });
        assert!(
            work(&mut state, source, DamageTarget::Player(THEM))
                .advance(&mut state)
                .is_none()
        );
        assert_eq!(state.players[1].life, 20);
        assert_eq!(state.object(target).unwrap().damage, 3);
        assert_eq!(state.object(alternate.object).unwrap().damage, 0);
        assert_eq!(state.shields.len(), 3);
    }
}
