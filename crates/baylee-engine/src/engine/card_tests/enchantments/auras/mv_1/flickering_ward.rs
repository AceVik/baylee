//! `cards/enchantments/auras/mv_1/flickering_ward.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn flickering_ward() -> CardIndex {
    card_index("3e2ae59a-40a1-46ca-8f24-3231ebd4e4eb")
}

/// Flickering Ward (`3e2ae59a-40a1-46ca-8f24-3231ebd4e4eb`) — {W} Enchantment — Aura:
/// "Enchant creature. As this Aura enters, choose a color. Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura. {W}: Return this Aura to its owner's hand."
#[test]
fn flickering_ward_chooses_color_on_entry_and_bounces_for_white() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), index::GRIZZLY_BEARS])
        .hand(0, &[flickering_ward()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, flickering_ward());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Even with White chosen, Flickering Ward is not removed by its own protection effect.
    assert!(on_battlefield(&engine, P0, flickering_ward()).is_some());

    // Pay {W} to return Flickering Ward to its owner's hand.
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, flickering_ward(), 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, flickering_ward()).is_none());
    assert!(in_hand(&engine, P0, flickering_ward()).is_some());
}

#[test]
fn flickering_ward_protects_from_its_chosen_color_and_bouncing_ends_the_protection() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[],
        &[
            flickering_ward(),
            index::HOLY_STRENGTH,
            index::HOLY_STRENGTH,
        ],
        &[],
    );
    let bears = lk::obj(&e, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&e, P0, index::HILL_GIANT);
    lk::float(&mut e, P0, &[(ManaColor::White, 4)]);
    cast_with_floating(&mut e, P0, flickering_ward());
    lk::aim(&mut e, &[bears], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    e.apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    lk::settle(&mut e);
    assert!(on_battlefield(&e, P0, flickering_ward()).is_some());
    cast_with_floating(&mut e, P0, index::HOLY_STRENGTH);
    let menu = aim_at(&mut e, P0, giant);
    assert!(!menu.contains(&bears));
    lk::settle(&mut e);
    lk::use_ability(&mut e, P0, flickering_ward(), 2, &[], &[]);
    lk::cast_at(&mut e, P0, index::HOLY_STRENGTH, &[bears], &[]);
    assert_eq!(pt(&e, bears), (3, 4));
}
