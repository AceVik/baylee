//! `cards/artifacts/mv_3/chimeric_sphere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::subtypes;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chimeric_sphere() -> CardIndex {
    card_index("b4aae26a-44a4-4ef0-9a1d-14a07a0865a3")
}

/// The Sphere and four Plains at P0's first main phase, every land tapped
/// for mana: two activations' worth floating.
fn board() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[chimeric_sphere(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let sphere = on_battlefield(&e, P0, chimeric_sphere()).expect("the Sphere is out");
    assert!(
        !types(&e, sphere).contains(TypeSet::CREATURE),
        "an artifact and nothing more until it is animated"
    );
    tap_all_mana(&mut e, P0);
    (e, sphere)
}

fn is_construct(e: &Engine<RegistryLookup>, sphere: ObjectId) -> bool {
    e.state()
        .object(sphere)
        .expect("the Sphere is there")
        .characteristics()
        .subtypes
        .contains(subtypes::creature::CONSTRUCT)
}

fn flies(e: &Engine<RegistryLookup>, sphere: ObjectId) -> bool {
    keywords(e, sphere).contains(KeywordSet::FLYING)
}

/// Chimeric Sphere (b4aae26a-44a4-4ef0-9a1d-14a07a0865a3) — "{2}: Until end
/// of turn, this artifact becomes a 2/1 Construct artifact creature with
/// flying." It is still an artifact, and it is a creature only until end of
/// turn.
#[test]
fn chimeric_sphere_becomes_a_two_one_flying_construct_until_end_of_turn() {
    let (mut e, sphere) = board();
    activate(&mut e, P0, chimeric_sphere(), 0);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, sphere), (2, 1));
    assert!(flies(&e, sphere), "with flying");
    assert!(is_construct(&e, sphere), "a Construct");
    assert!(types(&e, sphere).contains(TypeSet::CREATURE));
    assert!(
        types(&e, sphere).contains(TypeSet::ARTIFACT),
        "an artifact creature"
    );

    // Until end of turn: P1's first main phase finds a plain artifact again.
    reach_their_main_phase(&mut e, P1);
    assert!(
        !types(&e, sphere).contains(TypeSet::CREATURE),
        "no longer a creature"
    );
    assert!(!flies(&e, sphere), "no flying left");
    assert!(types(&e, sphere).contains(TypeSet::ARTIFACT));
}

/// Chimeric Sphere — "{2}: Until end of turn, this artifact becomes a 3/2
/// Construct artifact creature and loses flying." Played alone it becomes a
/// 3/2 Construct that never had flying.
#[test]
fn chimeric_sphere_second_ability_becomes_a_three_two_construct() {
    let (mut e, sphere) = board();
    activate(&mut e, P0, chimeric_sphere(), 1);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(pt(&e, sphere), (3, 2));
    assert!(!flies(&e, sphere));
    assert!(is_construct(&e, sphere));
    assert!(types(&e, sphere).contains(TypeSet::CREATURE));
    assert!(types(&e, sphere).contains(TypeSet::ARTIFACT));

    reach_their_main_phase(&mut e, P1);
    assert!(!types(&e, sphere).contains(TypeSet::CREATURE));
}

/// Chimeric Sphere — the later of the two wins, in either order (CR 613.7):
/// the 3/2 form "loses flying" the 2/1 form gave, and the 2/1 form activated
/// after it gives flying back.
#[test]
fn chimeric_sphere_the_later_activation_decides_size_and_flying() {
    let (mut e, sphere) = board();
    activate(&mut e, P0, chimeric_sphere(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, sphere), (2, 1));
    assert!(flies(&e, sphere));

    activate(&mut e, P0, chimeric_sphere(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, sphere), (3, 2), "the second form is the later one");
    assert!(!flies(&e, sphere), "and it loses the flying the first gave");

    // The pool is spent: both {2} are paid for, and a third is not on offer.
    assert_eq!(e.state().players[0].mana_pool.total(), 0);

    let (mut e, sphere) = board();
    activate(&mut e, P0, chimeric_sphere(), 1);
    pass_until(&mut e, stack_is_empty);
    activate(&mut e, P0, chimeric_sphere(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, sphere), (2, 1), "now the 2/1 form is the later one");
    assert!(flies(&e, sphere), "and it flies");
}
