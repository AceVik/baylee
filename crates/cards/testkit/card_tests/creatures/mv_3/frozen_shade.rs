//! `cards/creatures/mv_3/frozen_shade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Frozen Shade is `{2}{B}` for a 0/1 and its whole printed text is one
/// activated ability with no tap and no target: "{B}: This creature gets +1/+1
/// until end of turn." None of that can be read off the card file — that the
/// price is a real black mana, that the pump lands on the Shade and not on the
/// creature beside it, and that it is *until end of turn* — so the board is
/// five Swamps and one Elf. Five Swamps pay the `{2}{B}` and leave exactly two
/// activations of black floating, which is what makes "one `{B}` is one +1/+1"
/// an exact claim rather than a guess, and the Elf is the bystander the pump's
/// `Filter::This` has to decline.
#[test]
fn frozen_shade_pumps_itself_for_one_black_and_not_the_creature_beside_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[frozen_shade()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The Elf is kept back for two reasons at once: it is the bystander the
    // pump must decline, and it is a mana source whose own `{G}` would make
    // every count below a claim about mana nothing on the board accounted for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, frozen_shade());
    pass_until(&mut engine, stack_is_empty);
    let shade = on_battlefield(&engine, p0, frozen_shade()).expect("the Shade resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(
        pt(&engine, shade),
        (0, 1),
        "the shaded body the card prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Swamps pay `{{2}}{{B}}` and exactly two black are left floating"
    );

    // Ability 0 is the only thing the card prints.
    activate(&mut engine, p0, frozen_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, shade), (1, 2), "one {{B}} is one +1/+1");

    activate(&mut engine, p0, frozen_shade(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, shade),
        (2, 3),
        "and a second activation stacks on the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "each activation spent one black, and the pool held nothing else"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the pump is `Filter::This`: the creature beside it never grew"
    );

    // "Until end of turn": the pumps are gone by the opponent's next main
    // phase, which is past the cleanup step that removes them (CR 514.2).
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, shade),
        (0, 1),
        "both pumps fell off with the turn they were made in"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and the Elf never moved");
}
