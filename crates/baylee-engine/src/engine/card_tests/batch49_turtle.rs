//! Delayed effects are exercised through actual card activations and turns.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod extra_combat;
mod history;
mod turn_edges;

fn targets(e: &mut Engine<RegistryLookup>, ids: Vec<ObjectId>) {
    let player = e.pending().asked().unwrap();
    e.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: ids,
            players: vec![],
        },
    )
    .unwrap();
}
fn main_after(e: &mut Engine<RegistryLookup>, player: PlayerId, after: u32) {
    pass_until(e, |e| {
        e.state.turn.number > after
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}
fn attack(
    e: &mut Engine<RegistryLookup>,
    player: PlayerId,
    creature: ObjectId,
    defender: PlayerId,
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player: p, .. } if *p == player),
    );
    e.apply(
        player,
        PlayerAction::DeclareAttackers {
            attackers: vec![(creature, Defender::Player(defender))],
        },
    )
    .unwrap();
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseBlockers { .. }));
}
fn blink(e: &mut Engine<RegistryLookup>, id: ObjectId, owner: PlayerId) {
    e.state
        .move_object(
            id,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}

fn turn_priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn turn_mana(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    for c in ManaColor::ALL {
        e.state.players[p.get() as usize].mana_pool.add(c, 30);
    }
    e.refresh_offer();
}
fn turn_cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    turn_priority(e, p);
    turn_mana(e, p);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn turn_walk(e: &mut Engine<RegistryLookup>) {
    turn_cast(e, P0, index::TIME_WALK);
    pass_until(e, stack_is_empty);
}
fn vault_offer(e: &Engine<RegistryLookup>) -> bool {
    matches!(
        e.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::SkipTurn { .. },
            ..
        }
    )
}
fn next_actual_main(e: &mut Engine<RegistryLookup>, accept_first_vault: &mut bool) -> PlayerId {
    let turn = e.state.turn.number;
    for _ in 0..12 {
        pass_until(e, |e| {
            vault_offer(e)
                || (e.state.turn.number > turn
                    && e.state.turn.phase == crate::turn::Phase::FirstMain
                    && matches!(e.pending(), Pending::Priority { .. }))
        });
        if vault_offer(e) {
            let p = e.pending().asked().unwrap();
            let yes = std::mem::take(accept_first_vault);
            e.apply(p, PlayerAction::YesNo(yes)).unwrap();
        } else {
            return e.state.turn.active;
        }
    }
    panic!("turn boundary did not finish");
}
fn assert_attack_offer(e: &mut Engine<RegistryLookup>, p: PlayerId, id: ObjectId, legal: bool) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..} if *player==p),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(attackers.contains(&id), legal);
    if legal {
        e.apply(
            p,
            PlayerAction::DeclareAttackers {
                attackers: vec![(id, Defender::Player(if p == P0 { P1 } else { P0 }))],
            },
        )
        .unwrap();
    } else {
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                p,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(id, Defender::Player(if p == P0 { P1 } else { P0 }))]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
        e.apply(p, PlayerAction::DeclareAttackers { attackers: vec![] })
            .unwrap();
    }
}
