//! `cards/lands/tapland/boseiju_who_shelters_all.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Boseiju, Who Shelters All: "Boseiju enters tapped." / "{T}, Pay 2 life: Add {C}. If that mana is spent on an instant or sorcery spell, that spell can't be countered."
/// Under `Coverage::Implemented`, Boseiju enters tapped when played from hand.
/// Paying 2 life and tapping produces one colorless mana that carries the rider.
#[test]
fn boseiju_enters_tapped_and_pays_life_for_mana_with_a_rider() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .hand(0, &[boseiju_who_shelters_all()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let boseiju = play_land(&mut engine, p0, boseiju_who_shelters_all());
    assert!(entered_tapped(&engine, boseiju));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, boseiju));

    let before_life = engine.state().players[0].life;
    activate(&mut engine, p0, boseiju_who_shelters_all(), 0);

    assert_eq!(engine.state().players[0].life, before_life - 2);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "ordinary {{C}}: the rider restricts nothing (#232)"
    );
    assert!(pool.restricted().is_empty());
    assert_eq!(pool.ridden().len(), 1, "the unit carries its rider");
    assert_eq!(pool.ridden()[0].color, ManaColor::Colorless);
    assert!(is_tapped(&engine, boseiju));
}

/// Boseiju's {C} pays for a creature spell, and leaves it counterable
/// (#232). Read as "Spend this mana only", it paid for instants and
/// sorceries alone.
#[test]
fn boseiju_pays_for_a_creature_spell_and_leaves_it_counterable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, forest())
        .battlefield(0, &[boseiju_who_shelters_all(), forest()])
        .hand(0, &[sakura_tribe_elder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, boseiju_who_shelters_all(), 0);
    activate(&mut engine, p0, forest(), 0);
    cast_with_floating(&mut engine, p0, sakura_tribe_elder());
    let elder = on_stack(&engine, sakura_tribe_elder()).expect("the Elder is cast");
    assert!(
        !engine
            .state()
            .object(elder)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable)),
        "a creature spell is not what the rider names"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// Boseiju's {C} makes a sorcery it pays for uncounterable.
#[test]
fn boseiju_makes_a_sorcery_it_pays_for_uncounterable() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(111, island())
        .battlefield(0, &[boseiju_who_shelters_all(), island(), island()])
        .hand(0, &[counsel_of_the_soratami()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    activate(&mut engine, p0, boseiju_who_shelters_all(), 0);
    activate(&mut engine, p0, island(), 0);
    activate(&mut engine, p0, island(), 0);
    cast_with_floating(&mut engine, p0, counsel_of_the_soratami());
    let counsel = on_stack(&engine, counsel_of_the_soratami()).expect("Counsel is cast");
    assert!(
        engine
            .state()
            .object(counsel)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
    );
}
