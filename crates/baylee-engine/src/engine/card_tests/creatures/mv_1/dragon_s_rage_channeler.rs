//! `cards/creatures/mv_1/dragon_s_rage_channeler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn dragon_s_rage_channeler_surveils_on_noncreature_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[dragon_s_rage_channeler(), plains(), plains()])
        .hand(0, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drc = on_battlefield(&engine, p0, dragon_s_rage_channeler()).expect("drc seated");
    assert_eq!(pt(&engine, drc), (1, 1));
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, lightning_greaves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Arrange { .. })
    });

    let Pending::Arrange {
        player,
        cards,
        prompt,
        piles,
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, crate::choice::ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(1));
    assert!(!cards.is_empty());

    let binned = cards[0];
    engine.apply(p0, look_answer(&cards, &[binned])).unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, forest()).is_some(),
        "surveiled card was placed into the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, lightning_greaves()).is_some(),
        "lightning greaves resolved"
    );
    assert_eq!(
        pt(&engine, drc),
        (1, 1),
        "one graveyard type does not enable delirium"
    );
    assert!(!keywords(&engine, drc).contains(KeywordSet::FLYING));
}

#[test]
fn channeler_delirium_requires_four_types_and_forces_an_attack() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for enabled in [false, true] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[dragon_s_rage_channeler()])
            .hand(
                0,
                &[
                    index::ORNITHOPTER,
                    forest(),
                    island(),
                    swords_to_plowshares(),
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        for card in [index::ORNITHOPTER, forest(), island()]
            .into_iter()
            .chain(enabled.then_some(swords_to_plowshares()))
        {
            let id = in_hand(&e, p0, card).unwrap();
            e.dev_state_mut(p0)
                .unwrap()
                .move_object(
                    id,
                    ZoneLocation::Graveyard(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        e.refresh_offer();
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        let drc = on_battlefield(&e, p0, dragon_s_rage_channeler()).unwrap();
        assert_eq!(pt(&e, drc), if enabled { (3, 3) } else { (1, 1) });
        assert_eq!(keywords(&e, drc).contains(KeywordSet::FLYING), enabled);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers { required, .. } = e.pending() else {
            panic!("attackers")
        };
        assert_eq!(required.contains(&drc), enabled);
        let answer = e.apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] });
        assert_eq!(answer.is_err(), enabled);
        if enabled {
            e.apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(drc, Defender::Player(PlayerId::new(1)))],
                },
            )
            .unwrap();
        }
    }
}
