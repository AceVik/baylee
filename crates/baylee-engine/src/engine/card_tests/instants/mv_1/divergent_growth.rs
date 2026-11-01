//! `cards/instants/mv_1/divergent_growth.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn divergent_growth_grants_lands_you_control_tap_for_any_color_until_end_of_turn() {
    let growth = card_index("0123b162-0413-4022-acc0-e5bd061d3fb4");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[growth])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = lands_of(&engine, p0);
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: forests[0] })
        .unwrap();
    cast_with_floating(&mut engine, p0, growth);
    pass_until(&mut engine, stack_is_empty);

    let grant_id = crate::effects::granted_activated(engine.state(), forests[1])
        .next()
        .unwrap()
        .id;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: forests[1],
                ability_index: grant_id,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Blue),
        )
        .unwrap();

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(baylee_core::mana::ManaColor::Blue),
        1
    );
}
