//! `cards/enchantments/auras/mv_2/sunken_field.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn sunken_field() -> CardIndex {
    card_index("46569712-1f28-41cb-81be-fd938408c71f")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn objs(e: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            e.state()
                .object(id)
                .is_some_and(|o| o.controller == seat && o.card.is_some_and(|c| c.index == card))
        })
        .collect()
}

/// Enchanted land gains `{T}: Counter target spell unless its controller pays
/// {1}`: counters when unpaid, leaves the spell on the stack when paid.
#[test]
fn sunken_field_counters_unless_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[island(), island()],
        &[grizzly_bears()],
        &[sunken_field(), sunken_field()],
        &[savannah_lions(), lightning_bolt()],
    );
    let lands = objs(&e, p0, island());
    let (land1, land2) = (lands[0], lands[1]);
    let bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut e, p0, sunken_field());
    let opts = pass_until_targets(&mut e, p0);
    assert!(opts.contains(&land1) && !opts.contains(&bear));
    lk::aim(&mut e, &[land1], &[]);
    lk::settle(&mut e);

    lk::cast_at(&mut e, p0, sunken_field(), &[land2], &[]);

    // P1 casts Savannah Lions on P1's main phase with no spare mana -> countered.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::White, 1)]);
    let lions = in_hand(&e, p1, savannah_lions()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: lions })
        .unwrap();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: land1,
            ability_index: baylee_engine::choice::GRANTED_ABILITY,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[lions], &[]);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 1 },
                ..
            } if *player == p1
        )
    });
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, lions), Zone::Graveyard);

    // P1 casts Lightning Bolt with {1} extra floating and pays -> Bolt resolves.
    lk::float(&mut e, p1, &[(ManaColor::Red, 2)]);
    let bolt = in_hand(&e, p1, lightning_bolt()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: bolt }).unwrap();
    lk::aim(&mut e, &[], &[p0]);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: land2,
            ability_index: baylee_engine::choice::granted_ability(1),
        },
    )
    .unwrap();
    lk::aim(&mut e, &[bolt], &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p1),
    );
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 17);
}
