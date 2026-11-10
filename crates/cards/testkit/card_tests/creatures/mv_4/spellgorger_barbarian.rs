//! `cards/creatures/mv_4/spellgorger_barbarian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn spellgorger_barbarian_discards_a_card_at_random_on_etb_and_draws_a_card_on_ltb() {
    let barbarian = card_index("9c0fd5ad-fbba-4598-80cf-6540888378cb");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[barbarian, lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();

    let mountains = all_on_battlefield(&engine, p0, mountain());
    for &m in &mountains[..4] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: m })
            .unwrap();
    }
    let card = in_hand(&engine, p0, barbarian).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let sb = on_battlefield(&engine, p0, barbarian).expect("Spellgorger Barbarian on battlefield");
    // 1 card cast + 1 card discarded at random on ETB.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 1
    );

    let hand_after_etb = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before_ltb = library_size(&engine, p0);

    // Destroy Spellgorger Barbarian with Lightning Bolt to trigger its leaves-the-battlefield draw.
    engine
        .apply(
            p0,
            PlayerAction::ActivateManaAbility {
                source: mountains[4],
            },
        )
        .unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sb],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 1 Bolt cast (-1) + 1 card drawn from LTB trigger (+1) = net 0 change to hand size,
    // and library decreased by 1.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_after_etb
    );
    assert_eq!(library_size(&engine, p0), lib_before_ltb - 1);
}
