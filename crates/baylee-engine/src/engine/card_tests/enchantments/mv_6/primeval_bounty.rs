//! `cards/enchantments/mv_6/primeval_bounty.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn primeval_bounty() -> CardIndex {
    card_index("f633c16d-d943-421c-ad18-af35db9ec9fc")
}

/// "Whenever you cast a creature spell, create a 3/3 green Beast creature
/// token." Casting a noncreature spell (Dark Ritual) makes none of it.
#[test]
fn primeval_bounty_makes_a_beast_for_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[primeval_bounty(), mountain(), mountain(), mountain()])
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gray_ogre());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gray_ogre()).is_some()
    });
    let beasts = tokens_of(&engine, p0);
    assert_eq!(beasts.len(), 1, "one Beast");
    assert_eq!(pt(&engine, beasts[0]), (3, 3));
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    assert_eq!(counters_on(&engine, ogre, CounterKind::P1P1), 0);
}

/// "Whenever you cast a noncreature spell, put three +1/+1 counters on
/// target creature you control." The trigger asks for the target; the Ogre
/// becomes a 5/5 and no Beast appears.
#[test]
fn primeval_bounty_puts_three_counters_for_a_noncreature_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[primeval_bounty(), swamp(), gray_ogre()])
        .hand(0, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    cast_from_hand(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let options = aim_at(&mut engine, p0, ogre);
    assert!(options.contains(&ogre), "my creature is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, ogre, CounterKind::P1P1), 3);
    assert_eq!(pt(&engine, ogre), (5, 5));
    assert!(tokens_of(&engine, p0).is_empty(), "not a creature spell");
}

/// "Landfall — Whenever a land you control enters, you gain 3 life."
#[test]
fn primeval_bounty_gains_three_life_on_landfall() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[primeval_bounty()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 23);
}
