//! Oracle: whenever an opponent casts a creature spell, this deals 1 damage to that player.
#[allow(clippy::wildcard_imports)]
use super::*;
fn aether_sting() -> CardIndex {
    card_index("61105cb5-d7a1-4021-a006-dd1b947dfa68")
}
/// Checklist: own creature and opponent noncreature casts do not trigger; opponent creature cast damages that player while the spell remains on stack.
#[test]
fn only_opponent_creature_spells_trigger_damage_before_resolution() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, mountain())
        .battlefield(0, &[aether_sting(), forest()])
        .battlefield(1, &[forest(), forest(), mountain()])
        .hand(0, &[llanowar_elves()])
        .hand(1, &[lightning_bolt(), llanowar_elves()])
        .life(0, 20)
        .life(1, 12)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    cast_from_hand(&mut g, p, llanowar_elves());
    pass_until(&mut g, stack_is_empty);
    assert_eq!(
        g.state().players[0].life,
        20,
        "your creature cast is excluded"
    );

    reach_their_main_phase(&mut g, q);
    cast_from_hand(&mut g, q, lightning_bolt());
    g.apply(
        q,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p],
        },
    )
    .unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(
        g.state().players[1].life,
        12,
        "an opponent's noncreature cast is excluded"
    );
    assert_eq!(
        g.state().players[0].life,
        17,
        "only Lightning Strike's three damage was dealt"
    );

    cast_from_hand(&mut g, q, llanowar_elves());
    pass_until(&mut g, |e| e.state().players[1].life == 11);
    assert!(
        on_stack(&g, llanowar_elves()).is_some(),
        "the creature spell remains unresolved"
    );
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[1].life, 11);
    assert!(on_battlefield(&g, q, llanowar_elves()).is_some());
}
