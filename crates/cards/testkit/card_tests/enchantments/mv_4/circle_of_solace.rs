//! `cards/enchantments/mv_4/circle_of_solace.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn circle_of_solace() -> CardIndex {
    card_index("14ebedd1-7a2d-413b-9b76-8610511292a4")
}

/// ETB chooses a creature type; {1}{W} prevents only the NEXT time a creature of
/// the chosen type would deal damage to you this turn (not non-chosen or 2nd hits).
#[test]
fn circle_of_solace_prevents_next_damage_from_chosen_creature_type_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[],
        &[llanowar_elves(), llanowar_elves(), grizzly_bears()],
        &[circle_of_solace()],
        &[lightning_bolt()],
    );

    lk::float(
        &mut e,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
    );
    cast_with_floating(&mut e, p0, circle_of_solace());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::ELF),
    )
    .unwrap();
    lk::settle(&mut e);

    reach_their_main_phase(&mut e, p1);
    let opp_elves = all_on_battlefield(&e, p1, llanowar_elves());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    // P1 passes priority so P0 can activate Circle of Solace; Lightning Bolt still hits P0.
    lk::pass_once(&mut e);
    lk::float(
        &mut e,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    lk::use_ability(&mut e, p0, circle_of_solace(), 0, &[], &[]);
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p1, lightning_bolt(), &[], &[p0]);
    assert_eq!(lk::life(&e, p0), 17);

    // 1st Elf's combat damage is prevented; Bear (not Elf) deals 2 -> 15.
    lk::attack(&mut e, &[opp_elves[0], opp_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 15);
}
