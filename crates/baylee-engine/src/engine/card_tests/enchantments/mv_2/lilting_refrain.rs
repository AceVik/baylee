#[allow(clippy::wildcard_imports)]
use super::*;

fn lilting_refrain() -> CardIndex {
    card_index("f91a71b7-9bc0-4609-a312-ffede52dc0ad")
}

#[test]
fn lilting_refrain_adds_verse_counter_and_sacrifices_to_counter_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[grizzly_bears()])
        .battlefield(0, &[lilting_refrain(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, grizzly_bears());
    let spell_id = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .last()
        .copied()
        .expect("spell on stack");

    activate(&mut engine, p0, lilting_refrain(), 1);
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        aim_at(&mut engine, p0, spell_id);
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
}

#[test]
fn lilting_refrain_offers_payment_to_the_target_spells_controller() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for pay in [false, true] {
        let mut e = lk::game_with(
            SEED,
            &[lilting_refrain()],
            &[],
            &[],
            &[grizzly_bears()],
            |e| lk::yes_no(e, true),
        );
        reach_their_main_phase(&mut e, p1);
        lk::float(&mut e, p1, &[(ManaColor::Green, 3)]);
        cast_with_floating(&mut e, p1, grizzly_bears());
        let spell = *e
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .last()
            .unwrap();
        pass_until(
            &mut e,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
        );
        activate(&mut e, p0, lilting_refrain(), 1);
        lk::aim(&mut e, &[spell], &[]);
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        assert_eq!(e.pending().asked(), Some(p1));
        lk::yes_no(&mut e, pay);
        lk::settle(&mut e);
        assert_eq!(on_battlefield(&e, p1, grizzly_bears()).is_some(), pay);
        assert_eq!(lk::pool_of(&e, p1, ManaColor::Green), u32::from(!pay));
        assert!(in_graveyard(&e, p0, lilting_refrain()).is_some());
    }
}
