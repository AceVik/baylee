//! Ancient Kavu — {3}{R} — Creature — Kavu
//! Oracle: {2}: This creature becomes colorless until end of turn.

use super::*;
use baylee_core::color::ColorSet;

#[test]
fn ancient_kavu_activated_ability_makes_it_colorless_until_end_of_turn() {
    let kavu = card_index("598a7347-b772-4839-a02f-b930076c1354");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[kavu])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, kavu);
    pass_until(&mut engine, stack_is_empty);

    let kavu_id = on_battlefield(&engine, p0, kavu).expect("Ancient Kavu on board");
    assert_eq!(pt(&engine, kavu_id), (3, 3));
    assert!(
        !engine
            .state()
            .object(kavu_id)
            .unwrap()
            .characteristics()
            .colors
            .is_empty()
    );

    activate(&mut engine, p0, kavu, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(kavu_id)
            .unwrap()
            .characteristics()
            .colors,
        ColorSet::EMPTY
    );
}
