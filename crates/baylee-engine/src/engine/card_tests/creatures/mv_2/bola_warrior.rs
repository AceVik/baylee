//! `cards/creatures/mv_2/bola_warrior.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Bola Warrior ({1}{R} 1/1 Creature — Human Spellshaper Warrior):
/// "{R}, {T}, Discard a card: Target creature can't block this turn."
#[test]
fn bola_warrior_discards_a_card_to_prevent_target_creature_from_blocking_this_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                card_index("13f41253-364f-40a5-a7cd-84d8785b2460"),
                index::HILL_GIANT,
            ],
        )
        .hand(0, &[mountain()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let discard = in_hand(&engine, P0, mountain()).unwrap();

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    engine.refresh_offer();

    activate(
        &mut engine,
        P0,
        card_index("13f41253-364f-40a5-a7cd-84d8785b2460"),
        0,
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::CANT_BLOCK)
    );

    let options = attack_and_collect_blocks(&mut engine, giant, P1);
    assert!(
        options.iter().all(|opt| opt.blocker != bears),
        "Grizzly Bears with CANT_BLOCK is not offered as a blocker"
    );
}
