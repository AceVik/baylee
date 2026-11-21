//! `cards/enchantments/mv_5/gratuitous_violence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn gratuitous_violence() -> CardIndex {
    card_index("7c340a39-4ee0-4ba1-bb66-6674f8020fda")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

/// Creatures you control deal double combat and noncombat damage to permanents
/// and players; non-creature spells you control and opponent creatures do not double.
#[test]
fn gratuitous_violence_doubles_your_creatures_combat_and_noncombat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[gratuitous_violence(), grizzly_bears(), prodigal_sorcerer()],
        &[horned_turtle(), llanowar_elves()],
        &[lightning_bolt()],
        &[],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let opp_turtle = lk::obj(&e, p1, horned_turtle());

    // Noncombat damage from your creature (1 -> 2) kills neither, leaves 2 damage on 1/4 Turtle.
    lk::use_ability(&mut e, p0, prodigal_sorcerer(), 0, &[opp_turtle], &[]);
    assert_eq!(lk::damage_on(&e, opp_turtle), 2);

    // Your Lightning Bolt is NOT a creature -> deals normal 3 (20 -> 17).
    lk::float(&mut e, p0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut e, p0, lightning_bolt(), &[], &[p1]);
    assert_eq!(lk::life(&e, p1), 17);

    // Combat damage from your 2/2 Bear doubles to 4 (17 -> 13).
    lk::attack(&mut e, &[my_bear]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 13);

    // Opponent's 1/1 Elf does NOT double (20 -> 19).
    reach_their_main_phase(&mut e, p1);
    let opp_elves = lk::obj(&e, p1, llanowar_elves());
    lk::attack(&mut e, &[opp_elves]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p0), 19);
}
