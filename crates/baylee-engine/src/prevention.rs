//! Prevention shields (CR 615): damage that would be dealt and is not.
//!
//! A spell or ability that says "prevent" leaves a shield behind as it
//! resolves (CR 615.1, 615.3), and the shield waits for damage. Every writer
//! of damage asks [`apply`] how much of what it is about to deal still gets
//! through, at the one point where prevention belongs: after the damage is
//! known and before anything is marked, lost or journalled (CR 615.4 — a
//! shield cannot reach back to damage already dealt). Four writers do that
//! today, two in `combat` and two in `resolve::life`; a damage path that
//! skips this function deals damage no shield can see.
//!
//! What is here is the *resolved* kind of prevention — "the next 3 damage",
//! "all combat damage this turn". The standing kind a permanent's static
//! ability states (Maze of Ith's `Modifier::PreventDamageToIt`, protection)
//! is asked by each writer before this, is never used up, and is not a
//! shield.
//!
//! Every shield here lasts for the turn: all of them say "this turn", and
//! the cleanup step ends them with every other such effect (CR 514.2).

use crate::event::DamageTarget;
use crate::state::GameState;
use baylee_core::ids::{ObjectId, PlayerId};

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
    fn covers(self, state: &GameState, recipient: DamageTarget) -> bool {
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

/// The order shields are offered damage in.
///
/// CR 616.1 (and 615.7's last sentence, for one shield and several sources
/// at once) gives that choice to the affected player or the controller of
/// the affected permanent. The engine does not ask. It applies the shields
/// in the one order that player would always choose, where there is one:
///
/// 1. A shield that prevents everything and is never used up (Fog) — it
///    costs nothing to apply, and every other shield it saves stays for
///    later damage.
/// 2. Shields with an amount, oldest first. Two such shields on one
///    recipient spend the same total whichever goes first, and what is left
///    of them afterwards protects the same recipient either way.
///
/// That is the whole list today, and the order is dominant for it. Shields
/// whose use is a trade — one that pays life back for what it prevents
/// against one that is never used up, say — are where the missing question
/// would matter, and each such kind has to be placed here with the reason
/// the place is the one the player would pick.
fn rank(shield: &Shield) -> u8 {
    match shield.kind {
        ShieldKind::AllCombat => 0,
        ShieldKind::Next(_) => 1,
    }
}

/// How much of `amount` damage from `source` to `recipient` is still dealt
/// once the shields in front of it have prevented what they prevent.
///
/// Uses the shields up as it goes: an emptied "next N" is removed. Damage
/// that can't be prevented (CR 615.12, `unpreventable`) passes every shield
/// untouched and reduces none of them.
pub fn apply(
    state: &mut GameState,
    source: ObjectId,
    recipient: DamageTarget,
    amount: u32,
    is_combat: bool,
) -> u32 {
    if amount == 0 || state.shields.is_empty() {
        return amount;
    }
    if crate::combat::unpreventable(state, source, is_combat) {
        return amount;
    }
    let mut order: Vec<usize> = (0..state.shields.len())
        .filter(|&i| state.shields[i].protects.covers(state, recipient))
        .collect();
    order.sort_by_key(|&i| (rank(&state.shields[i]), i));
    let mut left = amount;
    let mut spent: Vec<usize> = Vec::new();
    for i in order {
        if left == 0 {
            break;
        }
        match &mut state.shields[i].kind {
            ShieldKind::AllCombat => {
                if is_combat {
                    left = 0;
                }
            }
            ShieldKind::Next(n) => {
                let prevented = left.min(*n);
                *n -= prevented;
                left -= prevented;
                if *n == 0 {
                    spent.push(i);
                }
            }
        }
    }
    spent.sort_unstable();
    for i in spent.into_iter().rev() {
        state.shields.remove(i);
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use crate::zone::ZoneLocation;
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
    use baylee_core::types::TypeSet;

    struct NoCards;
    impl CardLookup for NoCards {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            None
        }
    }

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);

    fn board() -> (GameState, ObjectId) {
        let seat = || SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let mut state = GameState::from_preset(
            &GamePreset {
                format: FormatId::Freeform,
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: vec![seat(), seat()],
            },
            &NoCards,
        )
        .expect("an empty board");
        let name = state.names.intern("Test Creature");
        let id = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(2);
        b.toughness = Some(2);
        (state, id)
    }

    fn shield(protects: Shielded, kind: ShieldKind) -> Shield {
        Shield {
            protects,
            kind,
            controller: P0,
        }
    }

    /// CR 615.7: "the next 3" prevents 3 in all, across events, and only
    /// to what it shields.
    #[test]
    fn a_shield_prevents_its_amount_and_is_used_up() {
        let (mut state, creature) = board();
        state
            .shields
            .push(shield(Shielded::Player(P1), ShieldKind::Next(3)));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P0), 2, false),
            2
        );
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 2, false),
            0
        );
        assert_eq!(state.shields[0].kind, ShieldKind::Next(1));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 2, true),
            1
        );
        assert!(state.shields.is_empty(), "used up");
    }

    /// A shield on a permanent is on that object: once it has left and
    /// come back, the new object is not shielded (CR 400.7).
    #[test]
    fn a_shield_on_a_permanent_is_on_that_object() {
        let (mut state, creature) = board();
        let version = state.object(creature).unwrap().version;
        state.shields.push(shield(
            Shielded::Object(creature, version),
            ShieldKind::Next(1),
        ));
        state.object_mut(creature).unwrap().version += 1;
        assert_eq!(
            apply(
                &mut state,
                creature,
                DamageTarget::Object(creature),
                1,
                false
            ),
            1
        );
        assert_eq!(state.shields.len(), 1, "not used on a different object");
    }

    /// Fog prevents combat damage only, to anything, and first: a "next 2"
    /// beside it is still whole afterwards.
    #[test]
    fn fog_prevents_combat_damage_and_spares_the_other_shields() {
        let (mut state, creature) = board();
        state
            .shields
            .push(shield(Shielded::Player(P1), ShieldKind::Next(2)));
        state
            .shields
            .push(shield(Shielded::Everything, ShieldKind::AllCombat));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 5, true),
            0
        );
        assert_eq!(state.shields[0].kind, ShieldKind::Next(2));
        assert_eq!(
            apply(&mut state, creature, DamageTarget::Player(P1), 5, false),
            3
        );
        assert_eq!(state.shields.len(), 1, "the amount is spent, Fog stays");
    }
}
