//! `cards/sorceries/mv_4/channel_the_suns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Channel the Suns` is a sorcery costing `{3}{G}` under `Coverage::Implemented`.
/// It prints "Add {W}{U}{B}{R}{G}."
/// When cast from hand off four Forests, casting consumes the floating green mana,
/// and upon resolution exactly one mana of each color ({W}, {U}, {B}, {R}, {G}) is added to the pool.
#[test]
fn channel_the_suns_adds_one_mana_of_each_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &[channel_the_suns()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, channel_the_suns());
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 5, "five mana added in total");
    assert_eq!(pool.available(ManaColor::White), 1, "contains {{W}}");
    assert_eq!(pool.available(ManaColor::Blue), 1, "contains {{U}}");
    assert_eq!(pool.available(ManaColor::Black), 1, "contains {{B}}");
    assert_eq!(pool.available(ManaColor::Red), 1, "contains {{R}}");
    assert_eq!(pool.available(ManaColor::Green), 1, "contains {{G}}");
    assert!(
        in_graveyard(&engine, p0, channel_the_suns()).is_some(),
        "Channel the Suns went to graveyard after resolving"
    );
}
