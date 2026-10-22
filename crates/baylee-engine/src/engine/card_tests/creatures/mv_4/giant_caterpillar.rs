//! `cards/creatures/mv_4/giant_caterpillar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_caterpillar() -> CardIndex {
    card_index("43a515f2-b53a-4bb4-bb6e-0d79c9162359")
}

/// `Giant Caterpillar` ({3}{G}, 3/3 Insect, `Coverage::Implemented`):
/// `{G}, Sacrifice this creature: Create a 1/1 green Insect creature token with
/// flying named Butterfly at the beginning of the next end step.`
#[test]
fn giant_caterpillar_sacrifices_to_create_butterfly_token_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[giant_caterpillar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, giant_caterpillar());
    pass_until(&mut engine, stack_is_empty);
    let caterpillar = on_battlefield(&engine, p0, giant_caterpillar()).unwrap();

    activate(&mut engine, p0, giant_caterpillar(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(caterpillar).unwrap().zone,
        Zone::Graveyard
    );

    // Advance past P0's end step into P1's main phase; the Butterfly token is on the battlefield.
    reach_their_main_phase(&mut engine, p1);
    let butterfly = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p0 && obj.characteristics().types.contains(TypeSet::CREATURE)
        })
        .expect("Butterfly token created at next end step");
    assert_eq!(pt(&engine, butterfly), (1, 1));
    assert!(keywords(&engine, butterfly).contains(KeywordSet::FLYING));
}
