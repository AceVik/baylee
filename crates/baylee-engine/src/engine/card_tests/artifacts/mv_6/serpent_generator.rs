//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn serpent_generator_poison_triggers_on_any_positive_player_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::SERPENT_GENERATOR])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, index::SERPENT_GENERATOR, 0);
    pass_until(&mut e, stack_is_empty);
    let snake = crate::engine::card_tests::token_helpers::named(&e, "Snake")[0];
    assert_eq!(pt(&e, snake), (1, 1));
    assert!(
        e.state()
            .object(snake)
            .unwrap()
            .characteristics()
            .colors
            .is_colorless()
    );
    for (recipient, amount, expected) in [(p1, 0, 0), (p1, 3, 1), (p0, 1, 1)] {
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e, snake, recipient, amount, false,
        );
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, |e| {
            stack_is_empty(e) && matches!(e.pending(),Pending::Priority{player,..}if *player==p0)
        });
        assert_eq!(e.state().players[recipient.get() as usize].poison, expected);
    }
}
