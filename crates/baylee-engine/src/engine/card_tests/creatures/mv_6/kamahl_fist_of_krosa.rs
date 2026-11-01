//! `cards/creatures/mv_6/kamahl_fist_of_krosa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Kamahl, Fist of Krosa (`8b21146d-3e92-46e7-b5d9-8240fba5088e`) — {4}{G}{G} Legendary Creature — Human Druid 4/3:
/// "{G}: Target land becomes a 1/1 creature until end of turn. It's still a land.
/// {2}{G}{G}{G}: Creatures you control get +3/+3 and gain trample until end of turn."
#[test]
fn kamahl_fist_of_krosa_animates_land_and_overruns_your_creatures() {
    let kamahl_card = card_index("8b21146d-3e92-46e7-b5d9-8240fba5088e");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                kamahl_card,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let kamahl = on_battlefield(&engine, p0, kamahl_card).unwrap();
    let target_land = on_battlefield(&engine, p0, plains()).unwrap();

    tap_all_mana_but(&mut engine, p0, Some(plains()));
    // Activate {G}: animate target Plains into a 1/1 creature.
    activate(&mut engine, p0, kamahl_card, 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target_land],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let land_chars = engine
        .state()
        .object(target_land)
        .unwrap()
        .characteristics();
    assert!(land_chars.types.contains(TypeSet::CREATURE));
    assert!(land_chars.types.contains(TypeSet::LAND));
    assert_eq!(pt(&engine, target_land), (1, 1));

    // Activate {2}{G}{G}{G}: creatures you control get +3/+3 and gain trample until end of turn.
    activate(&mut engine, p0, kamahl_card, 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, kamahl), (7, 6));
    assert!(keywords(&engine, kamahl).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&engine, target_land), (4, 4));
    assert!(keywords(&engine, target_land).contains(KeywordSet::TRAMPLE));
}
