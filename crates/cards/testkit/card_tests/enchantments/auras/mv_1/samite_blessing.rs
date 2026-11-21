//! `cards/enchantments/auras/mv_1/samite_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn samite_blessing() -> CardIndex {
    card_index("a524ae69-2594-467d-af0a-27b9984d4300")
}

/// Enchanted creature gains `{T}` to prevent the next damage from a chosen
/// source to target creature this turn; a second hit from the same source is
/// not prevented.
#[test]
fn samite_blessing_prevents_next_damage_from_chosen_source_to_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), shock_troops()],
        &[],
        &[samite_blessing()],
        &[lightning_bolt(), lightning_bolt()],
    );
    let host = lk::obj(&e, p0, grizzly_bears());
    let patient = lk::obj(&e, p0, shock_troops());

    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut e, p0, samite_blessing(), &[host], &[]);

    // P1 casts Bolt 1 at Shock Troops; P0 activates granted {T} choosing Bolt 1 -> prevented.
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p1, &[(ManaColor::Red, 2)]);
    let bolt1 = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt1 })
        .unwrap();
    lk::aim(&mut e, &[patient], &[]);

    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: host,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[patient], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    let bolt_src = baylee_core::ids::DamageSourceRef {
        object: bolt1,
        version: e.state().object(bolt1).expect("bolt on stack").version,
    };
    assert!(options.contains(&bolt_src));
    e.apply(
        p0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: bolt_src,
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert_eq!(lk::zone_of(&e, patient), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, patient), 0);
    assert!(lk::tapped(&e, host));

    // A second Bolt in the same turn kills Shock Troops (shield applied only once).
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt2 = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt2 })
        .unwrap();
    lk::aim(&mut e, &[patient], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, patient), Zone::Graveyard);
}

/// Enchanting an opponent's creature grants the activated ability to that
/// creature's controller, not the Aura's controller.
#[test]
fn samite_blessing_on_opponents_creature_grants_ability_to_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[], &[grizzly_bears()], &[samite_blessing()], &[]);
    let their_bear = lk::obj(&e, p1, grizzly_bears());
    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    lk::cast_at(&mut e, p0, samite_blessing(), &[their_bear], &[]);

    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: their_bear,
                ability_index: baylee_engine::choice::GRANTED_ABILITY,
            },
        )
        .is_err()
    );
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(lk::offered(
        &e,
        grizzly_bears(),
        p1,
        baylee_engine::choice::GRANTED_ABILITY
    ));
}
