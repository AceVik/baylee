//! `cards/creatures/mv_3/barbarian_guides.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barbarian_guides() -> CardIndex {
    card_index("ef8c324a-582b-401e-add1-1985c5596908")
}

/// Barbarian Guides (ef8c324a-582b-401e-add1-1985c5596908) — {2}{R} 1/2 Creature — Human Barbarian:
/// - "{2}{R}, {T}: Choose a land type. Target creature you control gains snow
///   landwalk of the chosen type until end of turn. Return that creature to its
///   owner's hand at the beginning of the next end step."
///   (`Coverage::Partial`: snow landwalk of a chosen land type not supported;
///   targeting a creature you control and returning it to its owner's hand at the
///   beginning of the next end step is supported).
#[test]
fn barbarian_guides_targets_creature_you_control_and_returns_it_to_hand_at_next_end_step() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                barbarian_guides(),
                mountain(),
                mountain(),
                mountain(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let guides = on_battlefield(&engine, P0, barbarian_guides()).expect("Guides seated");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("Gray Ogre seated");
    let their_elf = on_battlefield(&engine, P1, llanowar_elves()).expect("opponent Elf seated");
    assert_eq!(pt(&engine, guides), (1, 2));

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, barbarian_guides(), 0);

    let menu = aim_at(&mut engine, P0, ogre);
    assert!(
        menu.contains(&ogre) && menu.contains(&guides),
        "creatures you control are legal targets"
    );
    assert!(
        !menu.contains(&their_elf),
        "opponent's creatures are excluded"
    );
    assert!(is_tapped(&engine, guides));

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "still on the battlefield before the end step"
    );

    // At the beginning of the next end step, Gray Ogre is returned to its owner's hand.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_none(),
        "returned from the battlefield at the beginning of the next end step"
    );
    assert!(
        in_hand(&engine, P0, gray_ogre()).is_some(),
        "in its owner's hand"
    );
}
