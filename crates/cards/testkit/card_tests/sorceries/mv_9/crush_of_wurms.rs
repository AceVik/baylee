//! `cards/sorceries/mv_9/crush_of_wurms.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn crush_of_wurms_creates_three_six_six_wurm_tokens_and_flashes_back_for_twelve_mana() {
    let crush = card_index("6c12cd05-b673-453c-9f6b-f3aa022467c1");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 21])
        .hand(0, &[crush])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for f in all_on_battlefield(&engine, p0, forest())
        .into_iter()
        .take(9)
    {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    let spell = in_hand(&engine, p0, crush).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let wurms_1: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.card.is_none() && pt(&engine, id) == (6, 6)
        })
        .collect();
    assert_eq!(wurms_1.len(), 3);
    for &wurm in &wurms_1 {
        let characteristics = engine.state().object(wurm).unwrap().characteristics();
        assert!(
            characteristics
                .colors
                .contains(baylee_core::color::Color::Green)
        );
        assert!(
            characteristics
                .subtypes
                .contains(baylee_core::generated::subtypes::creature::WURM)
        );
    }

    let remaining_forests: Vec<_> = all_on_battlefield(&engine, p0, forest())
        .into_iter()
        .filter(|&id| !is_tapped(&engine, id))
        .collect();
    for f in remaining_forests {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: f })
            .unwrap();
    }
    let gy_crush = in_graveyard(&engine, p0, crush).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_crush })
        .expect("flashback Crush of Wurms");
    pass_until(&mut engine, stack_is_empty);

    let wurms_2 = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|&&id| {
            let obj = engine.state().object(id).unwrap();
            obj.card.is_none() && pt(&engine, id) == (6, 6)
        })
        .count();
    assert_eq!(wurms_2, 6);
    assert_eq!(engine.state().object(gy_crush).unwrap().zone, Zone::Exile);
}
