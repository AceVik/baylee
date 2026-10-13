//! `cards/lands/utility/helios_one.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// With no mana available, HELIOS One can only pay for its colorless tap.
#[test]
fn helios_one_without_mana_only_offers_its_free_colorless_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4102, forest()).hand(0, &[helios_one()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, helios_one());
    assert!(
        !entered_tapped(&engine, land),
        "nothing on the card enters it tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land drop spends nothing, so whatever is in the pool next is the land's"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending());
    };
    let offered = deeds(&legal, &[land]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "the energy activations need mana that this fixture cannot pay: {offered:?}"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the only mana source on the battlefield is HELIOS One"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{C}}, as printed");
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}

/// "Destroy target nonland permanent with mana value X. Activate only as a
/// sorcery." At X = 0 a token (mana value 0) is a target and a land (mana
/// value 0) is not; the ability is not offered with a spell on the stack or
/// in combat.
#[test]
fn helios_one_at_zero_targets_the_token_not_the_land_and_only_as_a_sorcery() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = kit::game(
        4103,
        &[helios_one()],
        &[forest()],
        &[index::SPROUT, index::SPROUT],
        &[],
    );
    let forest_theirs = kit::obj(&e, kit::P1, forest());

    // The first Saproling is the target that makes the timing checks honest:
    // a legal target exists while the second Sprout waits on the stack.
    kit::float(&mut e, p0, &[(ManaColor::Green, 2)]);
    cast_with_floating(&mut e, p0, index::SPROUT);
    pass_until(&mut e, stack_is_empty);
    let first = tokens_of(&e, p0);
    assert_eq!(first.len(), 1, "the first Sprout made one Saproling");

    cast_with_floating(&mut e, p0, index::SPROUT);
    assert!(!stack_is_empty(&e), "the second Sprout is on the stack");
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 3)]);
    assert!(
        !kit::offered(&e, helios_one(), p0, 2),
        "\"Activate only as a sorcery\": not with a spell on the stack, though {{3}} stands in the pool"
    );

    pass_until(&mut e, stack_is_empty);
    assert!(
        kit::offered(&e, helios_one(), p0, 2),
        "main phase, empty stack, {{3}} floating, zero energy: offered"
    );

    kit::to_step(&mut e, p0, crate::turn::Step::CombatBegin);
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 3)]);
    assert!(
        !kit::offered(&e, helios_one(), p0, 2),
        "\"Activate only as a sorcery\": not in combat"
    );

    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    kit::float(&mut e, p0, &[(ManaColor::Colorless, 3)]);
    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 2, "two Saprolings in all");
    activate(&mut e, p0, helios_one(), 2);
    if let Pending::ChooseNumber { min, max, .. } = e.pending().clone() {
        assert_eq!((min, max), (0, 0), "no energy to pay, so X can only be 0");
        e.apply(p0, PlayerAction::ChooseNumber(0)).unwrap();
    }
    let Pending::ChooseTargets { mut options, .. } = e.pending().clone() else {
        panic!("expected a target choice, got {:?}", e.pending());
    };
    assert!(
        !options.contains(&forest_theirs),
        "\"nonland permanent\": a land of mana value 0 is not a target at X = 0"
    );
    options.sort();
    let mut expected = tokens.clone();
    expected.sort();
    assert_eq!(
        options, expected,
        "the tokens (mana value 0) are the only targets at X = 0"
    );
    kit::aim(&mut e, &[tokens[0]], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(tokens[0])
            .is_none_or(|o| o.zone != Zone::Battlefield),
        "the chosen Saproling was destroyed"
    );
    assert_eq!(
        e.state().object(tokens[1]).unwrap().zone,
        Zone::Battlefield,
        "the other Saproling was not"
    );
}
