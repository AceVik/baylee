//! `cards/lands/utility/nearby_planet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nearby Planet prints Rangeling, `This land enters tapped.`, and `When this land enters, sacrifice it unless you pay {{1}}.`
///
/// Under `Coverage::Partial`, Rangeling is omitted because the DSL cannot grant every nonbasic land type, leaving Nearby Planet without intrinsic mana abilities.
/// Playing Nearby Planet from hand enters tapped via `EnterModifier::Tapped`.
/// When the enters-battlefield trigger resolves, `Pending::YesNo` with `YesNoPrompt::PayTax { mana: 1 }` is prompted;
/// paying with `{{1}}` floating from `forest()` prevents sacrifice, spending the mana and leaving the land tapped on the battlefield.
#[test]
fn nearby_planet_enters_tapped_and_pays_tax_to_remain_on_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[nearby_planet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Float {{1}} from Forest.
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    let planet = play_land(&mut engine, p0, nearby_planet());
    assert!(
        entered_tapped(&engine, planet),
        "nearby planet enters tapped"
    );

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });

    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!("expected PayTax prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayTax { mana: 1 });

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, nearby_planet()).is_some(),
        "nearby planet remains on battlefield when tax is paid"
    );
    assert!(is_tapped(&engine, planet), "remains tapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the one mana was spent paying the tax"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.mana_abilities.contains(&planet),
        "Rangeling is omitted under `Coverage::Partial`, so Nearby Planet has no basic land type mana abilities"
    );
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == planet),
        "no activated abilities on nearby planet"
    );
}
