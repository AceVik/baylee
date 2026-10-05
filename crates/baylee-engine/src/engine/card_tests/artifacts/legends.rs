//! Artifacts of the Arabian Nights / Antiquities / Legends pass (45c5bbd7),
//! each played from its Oracle text.

#[allow(clippy::wildcard_imports)]
use super::super::legends_kit::*;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

/// Arena of the Ancients: "Legendary creatures don't untap during their
/// controllers' untap steps. When this artifact enters, tap all legendary
/// creatures."
#[test]
fn arena_of_the_ancients_taps_every_legend_and_keeps_them_tapped() {
    let mut e = game(
        901,
        &[ids::jasmine_boreal(), ids::barbary_apes()],
        &[ids::tobias_andrion()],
        &[ids::arena_of_the_ancients()],
        &[],
    );
    let (mine, apes, theirs) = (
        obj(&e, P0, ids::jasmine_boreal()),
        obj(&e, P0, ids::barbary_apes()),
        obj(&e, P1, ids::tobias_andrion()),
    );
    assert!(!tapped(&e, mine) && !tapped(&e, theirs));
    float(&mut e, P0, &[(ManaColor::Colorless, 3)]);
    cast_at(&mut e, P0, ids::arena_of_the_ancients(), &[], &[]);
    assert!(tapped(&e, mine), "my legend is tapped on entry");
    assert!(tapped(&e, theirs), "an opponent's legend is tapped too");
    assert!(!tapped(&e, apes), "a nonlegendary creature is not");

    // Through the opponent's turn and back to mine: neither legend untaps.
    reach_their_main_phase(&mut e, P1);
    assert!(
        tapped(&e, theirs),
        "it did not untap in its controller's untap step"
    );
    reach_their_main_phase(&mut e, P0);
    assert!(tapped(&e, mine), "nor did mine in my untap step");
}

/// The five Mana Batteries: "{2}, {T}: Put a charge counter on this artifact.
/// {T}, Remove any number of charge counters from this artifact: Add {C},
/// then add an additional {C} for each charge counter removed this way."
#[test]
fn the_mana_batteries_store_a_charge_and_pay_it_back_with_one_extra() {
    let batteries = [
        (ids::white_mana_battery(), ManaColor::White),
        (ids::blue_mana_battery(), ManaColor::Blue),
        (ids::black_mana_battery(), ManaColor::Black),
        (ids::red_mana_battery(), ManaColor::Red),
        (ids::green_mana_battery(), ManaColor::Green),
    ];
    for (n, (battery, color)) in batteries.into_iter().enumerate() {
        let mut e = game(910 + n as u64, &[battery], &[], &[], &[]);
        let b = obj(&e, P0, battery);
        let charge =
            |e: &Engine<RegistryLookup>| counters_on(e, b, baylee_cards_dsl::CounterKind::Charge);
        float(&mut e, P0, &[(ManaColor::Colorless, 2)]);
        use_ability(&mut e, P0, battery, 0, &[], &[]);
        assert_eq!(charge(&e), 1, "{{2}},{{T}} puts a charge counter on it");
        assert!(tapped(&e, b));
        assert_eq!(pool_total(&e, P0), 0, "the {{2}} was paid");

        // Next turn it has untapped: remove the one counter for two mana of
        // its colour.
        reach_their_main_phase(&mut e, P1);
        reach_their_main_phase(&mut e, P0);
        assert!(!tapped(&e, b));
        activate(&mut e, P0, battery, 1);
        if let Pending::ChooseNumber { player, max, .. } = e.pending().clone() {
            assert_eq!(max, 1, "only one counter can be removed");
            e.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
        }
        assert_eq!(charge(&e), 0);
        assert_eq!(
            pool_of(&e, P0, color),
            2,
            "{{c}} plus one more for the counter"
        );
        assert_eq!(pool_total(&e, P0), 2);
    }
}

/// With no counter removed the second ability still adds the one mana.
#[test]
fn a_mana_battery_with_no_counters_taps_for_one() {
    let mut e = game(920, &[ids::black_mana_battery()], &[], &[], &[]);
    activate(&mut e, P0, ids::black_mana_battery(), 1);
    if let Pending::ChooseNumber { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::ChooseNumber(0)).unwrap();
    }
    assert_eq!(pool_of(&e, P0, ManaColor::Black), 1);
}

/// Horn of Deafening: "{2}, {T}: Prevent all combat damage that would be
/// dealt by target creature this turn."
#[test]
fn horn_of_deafening_prevents_the_targets_combat_damage() {
    let play = |use_horn: bool| {
        let mut e = game(
            930,
            &[ids::barbary_apes()],
            &[ids::horn_of_deafening()],
            &[],
            &[],
        );
        let apes = obj(&e, P0, ids::barbary_apes());
        attack(&mut e, &[apes]);
        block(&mut e, &[]);
        if use_horn {
            float(&mut e, P1, &[(ManaColor::Colorless, 2)]);
            to_step(&mut e, P1, Step::DeclareBlockers);
            use_ability(&mut e, P1, ids::horn_of_deafening(), 0, &[apes], &[]);
        }
        through_combat(&mut e);
        life(&e, P1)
    };
    assert_eq!(play(false), 18, "control: the Apes hit for 2");
    assert_eq!(play(true), 20, "the Horn prevented all of it");
}
