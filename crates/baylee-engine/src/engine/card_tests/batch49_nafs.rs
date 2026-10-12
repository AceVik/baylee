//! Nafs Asp debt payments, damage and actual draw-step boundaries.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_core::ids::GrantedActionId;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod damage;
mod deadlines;
mod debts;
mod multiplayer;
fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn mana(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    for c in ManaColor::ALL {
        e.state.players[p.get() as usize].mana_pool.add(c, 30);
    }
    e.refresh_offer();
}
fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e, p);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn attacks_hit(e: &mut Engine<RegistryLookup>, asps: Vec<ObjectId>) {
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: asps
                .into_iter()
                .map(|id| (id, Defender::Player(P1)))
                .collect(),
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseBlockers { .. }));
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(e, |e| {
        e.state.turn.step == crate::turn::Step::CombatEnd && stack_is_empty(e)
    });
}
fn debts(e: &Engine<RegistryLookup>) -> Vec<GrantedActionId> {
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("debt offers require priority")
    };
    legal
        .granted_actions
        .iter()
        .filter(|o| {
            matches!(
                o.effect,
                crate::choice::GrantedActionKind::CancelDelayedLifeLoss { amount: 1 }
            )
        })
        .map(|o| o.id)
        .collect()
}
fn p1_upkeep(e: &mut Engine<RegistryLookup>) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
}
fn draw_trigger(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| {
        e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Draw
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
}
fn asp(e: &Engine<RegistryLookup>) -> ObjectId {
    on_battlefield(e, P0, index::NAFS_ASP).unwrap()
}
