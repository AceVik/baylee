//! `cards/sorceries/mv_4/relentless_assault.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn relentless_assault() -> CardIndex {
    card_index("dc1c0e8c-d0c1-445c-968f-7dec91e5d5fc")
}

/// `Relentless Assault` ({2}{R}{R}, Sorcery, `Coverage::Partial`) untaps all
/// creatures that attacked this turn.
#[test]
fn relentless_assault_untaps_creatures_that_attacked_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                GRIZZLY_BEARS,
                HILL_GIANT,
            ],
        )
        .hand(0, &[relentless_assault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    // The Giant stays home but is tapped (it did not attack this turn).
    engine
        .dev_state_mut(p0)
        .unwrap()
        .object_mut(giant)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(is_tapped(&engine, bear));

    cast_from_hand(&mut engine, p0, relentless_assault());
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_tapped(&engine, bear), "the attacker untapped");
    assert!(
        is_tapped(&engine, giant),
        "a creature that did not attack stays tapped"
    );
}
