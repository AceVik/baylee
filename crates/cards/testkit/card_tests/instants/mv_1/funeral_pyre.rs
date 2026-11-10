//! `cards/instants/mv_1/funeral_pyre.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn funeral_pyre_exiles_target_graveyard_card_and_creates_a_flying_spirit_for_its_owner() {
    let funeral_pyre = card_index("02af53c1-83e6-447f-908b-2757c8aa445c");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[funeral_pyre])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let p1_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    bury(&mut engine, &[p1_bear]);
    let dead_bear = in_graveyard(&engine, p1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, p0, funeral_pyre);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![dead_bear],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(dead_bear).unwrap().zone, Zone::Exile);
    let p1_tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p1 && obj.card.is_none()
        })
        .collect();
    assert_eq!(p1_tokens.len(), 1);
    assert_eq!(pt(&engine, p1_tokens[0]), (1, 1));
    assert!(keywords(&engine, p1_tokens[0]).contains(KeywordSet::FLYING));
}
