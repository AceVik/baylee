//! Turn the Earth, played from hand and via flashback.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn turn_the_earth_shuffles_cards_from_any_graveyard_and_can_flash_back_with_no_targets() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let turn_the_earth = card_index("ebb445f7-75da-4af4-9153-93d419c838e0");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 3])
        .hand(0, &[turn_the_earth, island()])
        .hand(1, &[plains(), unsummon(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);
    let own_card = hand_to_graveyard(&mut engine, me, island());
    let other_cards = [
        hand_to_graveyard(&mut engine, them, plains()),
        hand_to_graveyard(&mut engine, them, unsummon()),
    ];

    cast_from_hand(&mut engine, me, turn_the_earth);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending()
    else {
        panic!("expected the spell to offer graveyard targets");
    };
    assert_eq!((*min, *max), (0, 3));
    let chosen = vec![own_card, other_cards[0], other_cards[1]];
    assert_eq!(options.len(), chosen.len());
    assert!(chosen.iter().all(|card| options.contains(card)));
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: chosen,
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(me))
            .contains(&own_card)
    );
    assert!(other_cards.iter().all(|card| {
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(them))
            .contains(card)
    }));
    assert_eq!(engine.state().players[0].life, 22);
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);

    let spell = in_graveyard(&engine, me, turn_the_earth).unwrap();
    let untouched = hand_to_graveyard(&mut engine, them, llanowar_elves());
    engine
        .apply(me, PlayerAction::CastSpell { card: spell })
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCastMode { .. }) {
        let flashback = choose_cast_kind(&engine, CastModeKind::Flashback);
        engine
            .apply(me, PlayerAction::ChooseMode(flashback))
            .unwrap();
    }
    assert_eq!(pass_until_targets(&mut engine, me), vec![untouched]);
    engine
        .apply(
            me,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 24);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(engine.state().object(spell).unwrap().zone, Zone::Exile);
    assert_eq!(
        engine.state().object(untouched).unwrap().zone,
        Zone::Graveyard
    );
}
