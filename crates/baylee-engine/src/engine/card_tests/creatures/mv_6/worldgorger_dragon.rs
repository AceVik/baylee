//! `cards/creatures/mv_6/worldgorger_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn worldgorger_dragon_exiles_all_other_permanents_you_control_on_etb_and_returns_them_on_ltb() {
    let dragon = card_index("a628186d-b7d9-40a5-9ae2-fbc9d2a14c7c");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                swamp(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[savannah_lions()])
        .hand(0, &[dragon, vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    let opp_lions = on_battlefield(&engine, p1, savannah_lions()).expect("P1 Savannah Lions");

    for m in all_on_battlefield(&engine, p0, mountain()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let card = in_hand(&engine, p0, dragon).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();

    // Float {W}{W}{B} while Worldgorger Dragon is on the stack, before its ETB trigger exiles our lands.
    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    for s in all_on_battlefield(&engine, p0, swamp()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    let wd = on_battlefield(&engine, p0, dragon).expect("Worldgorger Dragon on battlefield");
    assert!(keywords(&engine, wd).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, wd).contains(KeywordSet::TRAMPLE));

    // All other permanents P0 controlled were exiled; P1's permanents stayed.
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
    assert!(all_on_battlefield(&engine, p0, mountain()).is_empty());
    assert_eq!(
        engine.state().object(opp_lions).unwrap().zone,
        Zone::Battlefield
    );

    // Destroy Worldgorger Dragon with Vindicate; its LTB trigger returns the exiled permanents.
    let vind = in_hand(&engine, p0, vindicate()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: vind })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wd],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 6);
}
