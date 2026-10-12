//! Batch 48 event predicates exercised through real card actions.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

mod activator_control;
mod intrinsic_color_choice;
mod loyalty_observer;
mod observers;
mod review_edges;
mod sacrifice;

fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    for player in &mut e.state.players {
        for color in ManaColor::ALL {
            player.mana_pool.add(color, 30);
        }
    }
    e.refresh_offer();
    e
}

fn source(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    on_battlefield(e, seat, card).expect("fixture permanent")
}

fn priority(e: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, card: CardIndex) {
    priority(e, P0);
    let card = in_hand(e, P0, card).expect("fixture hand card");
    // These tests isolate event semantics, not mana generation. Refill after
    // step transitions, which legitimately empty the initial floating mana.
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}

fn ability(e: &mut Engine<RegistryLookup>, seat: PlayerId, card: CardIndex, ability_index: u32) {
    priority(e, seat);
    let source = source(e, seat, card);
    e.refresh_offer();
    e.apply(
        seat,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .unwrap();
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
