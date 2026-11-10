//! Thundercloud Elemental — `{5}{U}{U}` 3/4 Flying Creature — Elemental (SCG).
//! Oracle: "Flying. {3}{U}: Tap all creatures with toughness 2 or less. {3}{U}: All other creatures lose flying until end of turn."

use super::*;

fn thundercloud_elemental() -> CardIndex {
    card_index("24b44a35-496a-48e9-91ee-98b327d867e4")
}

#[test]
fn thundercloud_elemental_taps_small_toughness_creatures_and_strips_flying_from_others() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = thundercloud_elemental();
    let mut engine = table7(
        &lots(island(), 15),
        &[card],
        &[grizzly_bears(), craw_wurm(), flier()],
    );

    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands[..7].contains(&id));
    cast_with_floating(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let elem = on_battlefield(&engine, p0, card).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();
    let sphinx = on_battlefield(&engine, p1, flier()).unwrap();

    assert!(keywords(&engine, elem).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, sphinx).contains(KeywordSet::FLYING));

    // Ability 0 ({3}{U}): tap all creatures with toughness 2 or less.
    tap_mana_where(&mut engine, p0, |id| islands[7..11].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elem,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(wurm)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(elem)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Ability 1 ({3}{U}): all other creatures lose flying until end of turn.
    tap_mana_where(&mut engine, p0, |id| islands[11..15].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elem,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "other creature loses flying"
    );
    assert!(
        keywords(&engine, elem).contains(KeywordSet::FLYING),
        "Thundercloud Elemental keeps its own flying"
    );
}
