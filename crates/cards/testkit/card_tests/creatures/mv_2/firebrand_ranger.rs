//! Firebrand Ranger — {1}{R} — Creature — Human Soldier Ranger
//! Oracle: {G}, {T}: You may put a basic land card from your hand onto the battlefield.

use super::*;
use baylee_core::generated::index;

#[test]
fn firebrand_ranger_puts_a_basic_land_from_hand_onto_battlefield_and_rejects_nonbasic_or_creature()
{
    let ranger = card_index("4ceb49f3-a997-468d-9653-8ca5ceb2bcfb");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ranger, forest()])
        .hand(0, &[mountain(), index::CITY_OF_BRASS, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let basic_id = in_hand(&engine, p0, mountain()).unwrap();
    let nonbasic_id = in_hand(&engine, p0, index::CITY_OF_BRASS).unwrap();
    let bears_id = in_hand(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ranger, 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![nonbasic_id],
                },
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears_id],
                },
            )
            .is_err()
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![basic_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, mountain()).is_some());
}
