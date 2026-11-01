//! Armadillo Cloak — {1}{G}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2 and has trample.
//! Oracle: Whenever enchanted creature deals damage, you gain that much life.

use super::*;

#[test]
fn armadillo_cloak_grants_plus_two_plus_two_trample_and_damage_lifegain_trigger() {
    let cloak = card_index("15397a59-70a8-441f-abf7-230405a1602b");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[forest(), plains(), forest(), grizzly_bears()])
        .hand(0, &[cloak])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, cloak);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    assert!(keywords(&engine, bears).contains(KeywordSet::TRAMPLE));

    // Attack with enchanted Grizzly Bears: 4 combat damage triggers Armadillo Cloak for 4 life gained.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(engine.state().players[0].life, 24);
}
