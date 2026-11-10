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

/// "Whenever an opponent casts a creature spell, this enchantment deals 1
/// damage to that player": the opponent's creature spell hurts them, their
/// noncreature spell does not.
#[test]
fn aether_sting_hurts_an_opponent_who_casts_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aether_sting()])
        .hand(1, &[sol_ring(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    float(&mut engine, p1, ManaColor::Green, 1);
    cast_with_floating(&mut engine, p1, sol_ring());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p1),
        20,
        "an artifact spell is no creature spell"
    );

    float(&mut engine, p1, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    assert!(!stack_is_empty(&engine), "the spell and the trigger wait");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "1 damage to the caster");
    assert_eq!(
        life_of(&engine, p0),
        20,
        "the Sting's controller is not hurt"
    );
}

/// The Sting's own controller casting a creature spell is not "an opponent".
#[test]
fn aether_sting_ignores_my_own_creature_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aether_sting()])
        .hand(0, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Green, 2);
    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 20, "no damage to me");
    assert_eq!(life_of(&engine, p1), 20, "nor to the opponent");
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
}
