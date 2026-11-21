//! `cards/enchantments/auras/mv_2/lavamancer_s_skill.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn lavamancer_s_skill() -> CardIndex {
    card_index("e7819a6d-a518-4400-8e30-2a528878fd8a")
}

fn fugitive_wizard() -> CardIndex {
    card_index("381fa2a9-69fc-4558-a0ac-fb99c6f8d77f")
}

/// Grants "{T}: deal 1 damage to target creature" to any enchanted creature,
/// and additionally "{T}: deal 2 damage to target creature" while enchanted is a Wizard.
#[test]
fn lavamancer_s_skill_deals_one_and_adds_two_damage_ability_for_wizards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), fugitive_wizard()],
        &[grizzly_bears(), llanowar_elves()],
        &[lavamancer_s_skill(), lavamancer_s_skill()],
        &[],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let my_wiz = lk::obj(&e, p0, fugitive_wizard());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());
    let opp_elves = lk::obj(&e, p1, llanowar_elves());

    lk::float(&mut e, p0, &[(ManaColor::Red, 4)]);
    lk::cast_at(&mut e, p0, lavamancer_s_skill(), &[my_bear], &[]);
    lk::cast_at(&mut e, p0, lavamancer_s_skill(), &[my_wiz], &[]);

    // Bear only gets the 1-damage creature ping (not the Wizard 2-damage ability).
    assert!(!lk::offered(
        &e,
        grizzly_bears(),
        p0,
        baylee_engine::choice::granted_ability(1)
    ));
    lk::use_ability(
        &mut e,
        p0,
        grizzly_bears(),
        baylee_engine::choice::GRANTED_ABILITY,
        &[opp_elves],
        &[],
    );
    assert_eq!(lk::zone_of(&e, opp_elves), Zone::Graveyard);

    // Fugitive Wizard gains the 2-damage creature ping (slot 3 after two Auras) and kills Bear.
    lk::use_ability(
        &mut e,
        p0,
        fugitive_wizard(),
        baylee_engine::choice::granted_ability(3),
        &[opp_bear],
        &[],
    );
    assert_eq!(lk::zone_of(&e, opp_bear), Zone::Graveyard);
    assert_eq!(lk::life(&e, p1), 20);
}
