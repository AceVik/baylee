//! `cards/enchantments/mv_3/orim_s_prayer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit::life;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn orim_s_prayer() -> CardIndex {
    card_index("f387b1c6-25a6-4fff-90d3-fff027b247a5")
}

/// Orim's Prayer (`f387b1c6-25a6-4fff-90d3-fff027b247a5`) — {1}{W}{W} Enchantment:
/// "Whenever one or more creatures attack you, you gain 1 life for each attacking creature."
#[test]
fn orim_s_prayer_gains_one_life_per_attacking_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .hand(0, &[orim_s_prayer()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, orim_s_prayer());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0)), (giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(life(&engine, P0), 22);
}

#[test]
fn your_own_attack_does_not_gain_life() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[orim_s_prayer(), index::GRIZZLY_BEARS],
        &[],
        &[],
        &[],
    );
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    lk::attack(&mut e, &[bears]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(life(&e, P0), 20);
}
