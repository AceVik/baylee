//! `cards/creatures/mv_8/denizen_of_the_deep.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn denizen_of_the_deep() -> CardIndex {
    card_index("fc1bb5b9-5e36-4154-b683-e62f4f66f2b6")
}

/// On entering, Denizen of the Deep returns every other creature you control to
/// its owner's hand while keeping itself, your noncreature permanents, and
/// opposing creatures on the battlefield.
#[test]
fn denizen_of_the_deep_returns_each_other_creature_you_control_on_entering() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                grizzly_bears(),
                llanowar_elves(),
                sol_ring(),
            ],
        )
        .battlefield(1, &[craw_wurm()])
        .hand(0, &[denizen_of_the_deep()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    let spell = in_hand(&engine, p0, denizen_of_the_deep()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let serpent = on_battlefield(&engine, p0, denizen_of_the_deep()).expect("stays in play");
    assert_eq!(pt(&engine, serpent), (11, 11));
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
    assert!(in_hand(&engine, p0, grizzly_bears()).is_some());
    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
    assert!(
        on_battlefield(&engine, p0, sol_ring()).is_some(),
        "noncreature permanents remain on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p1, craw_wurm()).is_some(),
        "opponent's creatures remain on the battlefield"
    );
}
