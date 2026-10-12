//! Magosi's turn skips and counter-funded extra turns, played.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod countered;
mod magosi;
mod magosi_skips;

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
            && matches!(e.pending(),Pending::Priority{player:p,..} if *p==player)
    });
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
fn magosi_skip(e: &mut Engine<RegistryLookup>) {
    turn_priority(e, P0);
    turn_mana(e, P0);
    activate(e, P0, index::MAGOSI_THE_WATERVEIL, 1);
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
