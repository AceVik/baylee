//! `cards/enchantments/auras/mv_4/barbed_field.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn barbed_field() -> CardIndex {
    card_index("45222123-f5e0-48ff-b80c-858727d155d7")
}

/// Enchanted land gains `{T}: This land deals 1 damage to any target`, able to
/// destroy a 1-toughness creature or ping a player; on an opponent's land, the
/// opponent controls the ability.
#[test]
fn barbed_field_grants_tap_ping_to_enchanted_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[mountain()],
        &[llanowar_elves(), island()],
        &[barbed_field(), barbed_field()],
        &[],
    );
    let my_mtn = lk::obj(&e, p0, mountain());
    let their_elves = lk::obj(&e, p1, llanowar_elves());
    let their_land = lk::obj(&e, p1, island());

    lk::float(&mut e, p0, &[(ManaColor::Red, 8)]);
    cast_with_floating(&mut e, p0, barbed_field());
    let opts = pass_until_targets(&mut e, p0);
    assert!(opts.contains(&my_mtn) && opts.contains(&their_land));
    assert!(!opts.contains(&their_elves));
    lk::aim(&mut e, &[my_mtn], &[]);
    lk::settle(&mut e);

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: my_mtn,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[their_elves], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, their_elves), Zone::Graveyard);
    assert!(lk::tapped(&e, my_mtn));

    lk::cast_at(&mut e, p0, barbed_field(), &[their_land], &[]);
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: their_land,
                ability_index: baylee_engine::choice::GRANTED_ABILITY,
            },
        )
        .is_err()
    );

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: my_mtn,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[], &[p1]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p1), 19);
}
