//! `cards/enchantments/mv_2/lifetap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lifetap: "Whenever a Forest an opponent controls becomes tapped, you
/// gain 1 life." Tapping the opponent's Forest for mana gains a life; the
/// same seat tapping its own Forest does not.
#[test]
fn lifetap_gains_life_when_an_opponents_forest_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let lifetap = lifetap();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[lifetap, island(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest");

    let before = engine.state().players[0].life;
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: their_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before + 1,
        "gained 1 life off the opponent's Forest"
    );

    reach_their_main_phase(&mut engine, p0);
    let own_land = on_battlefield(&engine, p0, forest()).expect("p0's own Forest");
    let before = engine.state().players[0].life;
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: own_land })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        before,
        "not the caster's own Forest"
    );
}
