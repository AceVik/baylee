//! Actual card actions and historical identity boundaries.
#[allow(clippy::wildcard_imports)]
use super::*;

fn instant(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    priority(e, player);
    e.state.players[usize::from(player.get())]
        .mana_pool
        .add(ManaColor::Black, 1);
    let card = in_hand(e, player, index::DARK_RITUAL).unwrap();
    e.refresh_offer();
    e.apply(player, PlayerAction::CastSpell { card }).unwrap();
    pass_until(e, stack_is_empty);
}

#[test]
fn batch48_druid_counts_countered_instants_cast_before_it_entered() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::ICHNEUMON_DRUID, index::COUNTERSPELL])
        .hand(1, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    priority(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::Black, 1);
    let first = in_hand(&e, P1, index::DARK_RITUAL).unwrap();
    e.refresh_offer();
    e.apply(P1, PlayerAction::CastSpell { card: first })
        .unwrap();
    cast(&mut e, index::COUNTERSPELL);
    targets(&mut e, vec![first], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(first).unwrap().zone, Zone::Graveyard);
    cast(&mut e, index::ICHNEUMON_DRUID);
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[1].life;
    instant(&mut e, P1);
    assert_eq!(e.state.players[1].life, life - 4);
}

#[test]
fn batch48_druid_ignores_sorcery_and_own_instants_and_resets_each_turn() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ICHNEUMON_DRUID])
        .hand(0, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .hand(
            1,
            &[
                index::DARK_RITUAL,
                index::DARK_RITUAL,
                index::DARK_RITUAL,
                index::SACRED_NECTAR,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let own_life = e.state.players[0].life;
    instant(&mut e, P0);
    instant(&mut e, P0);
    assert_eq!(e.state.players[0].life, own_life);
    // First opposing instant on P0's turn is free; the ordinal resets on P1's turn.
    let life = e.state.players[1].life;
    instant(&mut e, P1);
    assert_eq!(e.state.players[1].life, life);
    next_main(&mut e, P1);
    e.state.players[1].mana_pool.add(ManaColor::White, 3);
    let sorcery = in_hand(&e, P1, index::SACRED_NECTAR).unwrap();
    e.refresh_offer();
    e.apply(P1, PlayerAction::CastSpell { card: sorcery })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[1].life;
    instant(&mut e, P1);
    assert_eq!(
        e.state.players[1].life, life,
        "sorcery is not the first instant"
    );
    instant(&mut e, P1);
    assert_eq!(e.state.players[1].life, life - 4);
}

#[test]
fn batch48_druid_instant_ordinals_are_per_opponent() {
    let p2 = PlayerId::new(2);
    let mut e = Duel::table(SEED, index::FOREST, 3)
        .battlefield(0, &[index::ICHNEUMON_DRUID])
        .hand(1, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .hand(2, &[index::DARK_RITUAL, index::DARK_RITUAL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let before = [e.state.players[1].life, e.state.players[2].life];
    instant(&mut e, P1);
    instant(&mut e, p2);
    assert_eq!([e.state.players[1].life, e.state.players[2].life], before);
    instant(&mut e, p2);
    instant(&mut e, P1);
    assert_eq!(
        [e.state.players[1].life, e.state.players[2].life],
        [before[0] - 4, before[1] - 4]
    );
}

fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}
