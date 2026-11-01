//! `cards/planeswalkers/mv_4/serra_the_benevolent.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn serra_the_benevolent() -> CardIndex {
    card_index("8b0e2382-dd91-42e6-aba3-5e8a15418a49")
}

#[test]
fn serra_pumps_only_own_flying_creatures_and_creates_her_angel_token() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[serra_the_benevolent(), serra_angel(), gray_ogre()])
        .battlefield(1, &[serra_angel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let serra = on_battlefield(&e, p0, serra_the_benevolent()).expect("Serra");
    let flying = on_battlefield(&e, p0, serra_angel()).expect("own flier");
    let ground = on_battlefield(&e, p0, gray_ogre()).expect("own nonflier");
    let theirs = on_battlefield(&e, p1, serra_angel()).expect("opposing flier");
    assert_eq!(counters_on(&e, serra, CounterKind::Loyalty), 4);
    activate(&mut e, p0, serra_the_benevolent(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, serra, CounterKind::Loyalty), 6);
    assert_eq!(pt(&e, flying), (5, 5));
    assert_eq!(pt(&e, ground), (2, 2));
    assert_eq!(pt(&e, theirs), (4, 4));

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(pt(&e, flying), (4, 4), "the pump ends with the turn");
    activate(&mut e, p0, serra_the_benevolent(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, serra, CounterKind::Loyalty), 3);
    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 1);
    let token = tokens[0];
    let chars = e
        .state()
        .object(token)
        .expect("Angel token")
        .characteristics();
    assert_eq!(pt(&e, token), (4, 4));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White])
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ANGEL)
    );
    assert!(keywords(&e, token).contains(KeywordSet::FLYING));
    assert!(keywords(&e, token).contains(KeywordSet::VIGILANCE));
}

#[track_caller]
fn bolt_self(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(e, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
    tap_all_mana(e, player);
    let spell = in_hand(e, player, lightning_bolt()).expect("Bolt in hand");
    let Pending::Priority {
        player: priority,
        legal,
        ..
    } = e.pending().clone()
    else {
        panic!("Bolt needs priority, got {:?}", e.pending());
    };
    assert_eq!(priority, player);
    assert!(
        legal.castable.contains(&spell),
        "Bolt is castable with one red mana, pool: {:?}",
        e.state().players[usize::from(player.get())].mana_pool
    );
    e.apply(player, PlayerAction::CastSpell { card: spell })
        .expect("cast Bolt");
    crate::engine::card_tests::legends_kit::aim(e, &[], &[player]);
}

#[test]
fn serra_emblem_floors_damage_only_while_a_creature_is_controlled() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .life(0, 2)
        .battlefield(
            0,
            &[serra_the_benevolent(), gray_ogre(), mountain(), mountain()],
        )
        .hand(0, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let serra = on_battlefield(&e, p0, serra_the_benevolent()).expect("Serra");
    e.dev_state_mut(p0)
        .expect("setup the ultimate")
        .object_mut(serra)
        .expect("Serra object")
        .counters
        .add(CounterKind::Loyalty, 3);
    e.refresh_offer();
    activate(&mut e, p0, serra_the_benevolent(), 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, serra, CounterKind::Loyalty), 1);

    bolt_self(&mut e, p0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        1,
        "a creature enables the emblem"
    );
    let ogre = on_battlefield(&e, p0, gray_ogre()).expect("the last creature");
    kill(&mut e, ogre);
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    bolt_self(&mut e, p0);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert!(e.state().players[0].life < 1, "no creature, no life floor");
}
