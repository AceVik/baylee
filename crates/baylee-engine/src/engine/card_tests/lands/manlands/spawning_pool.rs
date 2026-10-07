//! `cards/lands/manlands/spawning_pool.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spawning Pool: "This land enters tapped." / "{T}: Add {B}." / "{1}{B}:
/// This land becomes a 1/1 black Skeleton creature with '{B}: Regenerate
/// this creature' until end of turn. It's still a land."
///
/// The regeneration here is not an ability the card has — it is one an
/// animation *grants*, which is a different door: it arrives as a
/// `Modifier::GrantActivated` and is offered under the synthetic
/// `choice::GRANTED_ABILITY` index rather than at a place in the card's own
/// list. So the assertion has to be made twice over: the printed ability 1
/// is what animates, and the thing it hands the land is what makes a
/// shield.
///
/// Three Swamps, because the animation eats `{1}{B}` and the grant then asks
/// for a `{B}` of its own — and `mana_pay::pay_any` settles the generic half
/// out of `ManaColor::ALL` in order, so it takes the black before it reaches
/// the green standing beside it. Two Swamps leave the grant unaffordable and
/// the offer would then be missing for the wrong reason.
#[test]
fn spawning_pool_animates_into_a_skeleton_that_can_regenerate_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(217, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), forest()])
        .hand(0, &[spawning_pool()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pool = play_land(&mut engine, p0, spawning_pool());
    assert!(entered_tapped(&engine, pool));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, pool));

    tap_mana_except(&mut engine, p0, pool);
    activate(&mut engine, p0, spawning_pool(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, pool), (1, 1));
    let types = engine.state().object(pool).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, pool));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal
            .abilities
            .contains(&(pool, crate::choice::GRANTED_ABILITY)),
        "the animation granted an activated ability, and it is on offer: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pool,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("a black is still floating for the grant");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(pool)
            .expect("the land is still a land")
            .regeneration_shields,
        1,
        "the granted ability regenerates the thing that was granted it"
    );
}
