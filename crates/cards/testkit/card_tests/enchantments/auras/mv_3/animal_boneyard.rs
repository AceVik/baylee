//! `cards/enchantments/auras/mv_3/animal_boneyard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn animal_boneyard() -> CardIndex {
    card_index("6fa9debf-c60e-432d-86c8-aed16556c532")
}

fn horned_turtle() -> CardIndex {
    card_index("7a4ae76b-6241-4fa8-b239-5ec941a0e7df")
}

/// Enchanted land grants "{T}, Sacrifice a creature: You gain life equal to the
/// sacrificed creature's toughness", offered only while controlling a creature.
#[test]
fn animal_boneyard_sacrifices_creature_and_gains_its_toughness_in_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[plains(), horned_turtle(), grizzly_bears()],
        &[swamp(), llanowar_elves()],
        &[animal_boneyard(), animal_boneyard()],
        &[],
    );
    let turtle = lk::obj(&e, p0, horned_turtle());
    let bear = lk::obj(&e, p0, grizzly_bears());
    let my_plains = lk::obj(&e, p0, plains());
    let opp_swamp = lk::obj(&e, p1, swamp());

    lk::float(
        &mut e,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
    );
    lk::cast_at(&mut e, p0, animal_boneyard(), &[my_plains], &[]);
    lk::float(
        &mut e,
        p0,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
    );
    lk::cast_at(&mut e, p0, animal_boneyard(), &[opp_swamp], &[]);

    // Tap enchanted Plains and sacrifice 1/4 Horned Turtle -> +4 life.
    assert!(lk::offered(
        &e,
        plains(),
        p0,
        baylee_engine::choice::GRANTED_ABILITY
    ));
    activate(&mut e, p0, plains(), baylee_engine::choice::GRANTED_ABILITY);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![turtle],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, turtle), Zone::Graveyard);
    assert_eq!(lk::life(&e, p0), 24);

    // Enchanted opponent's Swamp lets P1 sacrifice P1's Elf (+1 life), and without creatures cannot activate.
    reach_their_main_phase(&mut e, p1);
    activate(
        &mut e,
        p1,
        swamp(),
        baylee_engine::choice::granted_ability(1),
    );
    let opp_elves = lk::obj(&e, p1, llanowar_elves());
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![opp_elves],
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p1), 21);
    assert_eq!(lk::zone_of(&e, bear), Zone::Battlefield);

    // Without any creature on P1's battlefield, the untapped Swamp no longer offers the ability.
    reach_their_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    assert!(!lk::offered(
        &e,
        swamp(),
        p1,
        baylee_engine::choice::granted_ability(1)
    ));
}
