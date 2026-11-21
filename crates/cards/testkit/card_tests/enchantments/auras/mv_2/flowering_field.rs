//! `cards/enchantments/auras/mv_2/flowering_field.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn flowering_field() -> CardIndex {
    card_index("40155cfb-078a-4359-9551-9c71266d8a1c")
}

/// Enchants only lands and grants `{T}: Prevent the next 1 damage that would be
/// dealt to any target this turn` to the enchanted land (tested against both
/// player burn and creature damage).
#[test]
fn flowering_field_prevents_next_one_damage_to_player_and_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[plains(), grizzly_bears()],
        &[orcish_artillery()],
        &[flowering_field()],
        &[lightning_bolt()],
    );
    let land = lk::obj(&e, p0, plains());
    let bear = lk::obj(&e, p0, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    cast_with_floating(&mut e, p0, flowering_field());
    let opts = pass_until_targets(&mut e, p0);
    assert!(opts.contains(&land));
    assert!(!opts.contains(&bear));
    lk::aim(&mut e, &[land], &[]);
    lk::settle(&mut e);

    // Shield p0 from 1 of Lightning Bolt's 3 damage -> p0 lands on 18, not 17.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert!(lk::tapped(&e, land));

    e.apply(p0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    let bolt = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt }).unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 18);

    // Next turn: shield Grizzly Bears, then Orcish Artillery deals 1 instead of 2.
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: land,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);

    e.apply(p0, PlayerAction::PassPriority).unwrap();
    activate(&mut e, p1, orcish_artillery(), 0);
    lk::aim(&mut e, &[bear], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, bear), 1);
}
