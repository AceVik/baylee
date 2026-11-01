//! `cards/creatures/mv_5/silver_wyvern.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn silver_wyvern() -> CardIndex {
    card_index("4baa8f05-1ef8-4770-8c66-ef5f0f02a717")
}

/// Silver Wyvern (`4baa8f05-1ef8-4770-8c66-ef5f0f02a717`) — {3}{U}{U} 4/3 Creature — Drake (Partial):
/// "Flying. {U}: Change the target of target spell or ability that targets only this creature. The new target must be a creature."
#[test]
fn silver_wyvern_redirects_single_target_spell_to_another_creature() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[silver_wyvern()],
        &[index::TERROR],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let wyvern = lk::obj(&engine, P1, silver_wyvern());

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
    );
    cast_with_floating(&mut engine, P0, index::TERROR);
    lk::aim(&mut engine, &[wyvern], &[]);
    let terror_spell = on_stack(&engine, index::TERROR).expect("Terror on stack");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut engine, P1, &[(ManaColor::Blue, 1)]);
    engine
        .apply(
            P1,
            PlayerAction::ActivateAbility {
                source: wyvern,
                ability_index: 0,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[terror_spell], &[]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        lk::aim(&mut engine, &[bears], &[]);
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none(),
        "Silver Wyvern redirected Terror onto Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P1, silver_wyvern()).is_some(),
        "Silver Wyvern survived"
    );
}

#[test]
fn cannot_redirect_a_spell_targeting_an_unrelated_creature() {
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[silver_wyvern()],
        &[index::TERROR],
        &[],
    );
    let bear = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let wyvern = lk::obj(&e, P1, silver_wyvern());
    lk::float(&mut e, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut e, P0, index::TERROR);
    lk::aim(&mut e, &[bear], &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut e, P1, &[(ManaColor::Blue, 1)]);
    let Pending::Priority { legal, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!legal.abilities.contains(&(wyvern, 0)));
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, silver_wyvern()).is_some());
}
