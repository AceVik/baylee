//! Dai Li Indoctrination, played in both modes.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

fn dai_li_indoctrination() -> CardIndex {
    card_index("5591efbc-1b2c-489a-b4f3-935c9b6bd71a")
}

#[test]
fn dai_li_indoctrination_reveals_the_hand_and_offers_only_nonland_permanents() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[dai_li_indoctrination()])
        .battlefield(0, &[swamp(), forest()])
        .hand(
            1,
            &[
                llanowar_elves(),
                sol_ring(),
                index::DARKSTEEL_CITADEL,
                lightning_bolt(),
                index::DIVINATION,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
    let elf = in_hand(&engine, them, llanowar_elves()).unwrap();
    let ring = in_hand(&engine, them, sol_ring()).unwrap();
    let bolt = in_hand(&engine, them, lightning_bolt()).unwrap();

    cast_from_hand(&mut engine, me, dai_li_indoctrination());
    let mode = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(me, PlayerAction::ChooseMode(mode)).unwrap();
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        panic!("expected an opponent target");
    };
    assert_eq!(options, &[them]);
    engine.apply(me, PlayerAction::ChoosePlayer(them)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        unreachable!("the caster has a discard choice");
    };
    assert_eq!((*player, *min, *max), (me, 1, 1));
    assert_eq!(options.len(), 2);
    assert!(options.contains(&elf));
    assert!(options.contains(&ring));
    assert!(engine.journal().entries().iter().any(|entry| {
        matches!(&entry.event, GameEvent::Revealed { player, cards }
            if *player == them && *cards == hand)
    }));
    assert!(
        engine
            .apply(
                me,
                PlayerAction::ChooseObjects {
                    objects: vec![bolt]
                }
            )
            .is_err()
    );
    engine
        .apply(
            me,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert!(in_graveyard(&engine, them, sol_ring()).is_some());
    assert!(in_hand(&engine, them, llanowar_elves()).is_some());
    assert!(in_hand(&engine, them, lightning_bolt()).is_some());
    assert!(in_hand(&engine, them, index::DIVINATION).is_some());
    assert!(in_hand(&engine, them, index::DARKSTEEL_CITADEL).is_some());
}

#[test]
fn dai_li_indoctrination_earthbends_a_land_you_control_and_brings_it_back() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[dai_li_indoctrination()])
        .battlefield(0, &[swamp(), swamp(), forest(), sol_ring()])
        .hand(1, &[lightning_bolt()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let land = on_battlefield(&engine, me, forest()).unwrap();
    let their_land = on_battlefield(&engine, them, mountain()).unwrap();
    let artifact = on_battlefield(&engine, me, sol_ring()).unwrap();
    tap_mana_where(&mut engine, me, |id| id != land && id != artifact);
    cast_with_floating(&mut engine, me, dai_li_indoctrination());
    let mode = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(me, PlayerAction::ChooseMode(mode)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected land targets");
    };
    assert!(options.contains(&land));
    assert!(!options.contains(&their_land));
    assert!(!options.contains(&artifact));
    aim_at(&mut engine, me, land);
    pass_until(&mut engine, stack_is_empty);

    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(
        types.contains(TypeSet::LAND),
        "earthbend preserves the land type"
    );
    assert!(types.contains(TypeSet::CREATURE));
    assert_eq!(pt(&engine, land), (2, 2));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 2);
    assert!(keywords(&engine, land).contains(KeywordSet::HASTE));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == them),
    );
    cast_from_hand(&mut engine, them, lightning_bolt());
    aim_at(&mut engine, them, land);
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, me, forest()).expect("the earthbent land returned");
    assert!(is_tapped(&engine, returned));
    assert_eq!(counters_on(&engine, returned, CounterKind::P1P1), 0);
    assert!(engine.journal().entries().iter().any(|entry| {
        matches!(entry.event, GameEvent::ZoneChanged {
            object, from: Zone::Graveyard, to: Zone::Battlefield, ..
        } if object == land)
    }));
}
