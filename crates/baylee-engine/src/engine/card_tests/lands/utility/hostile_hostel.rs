//! `cards/lands/utility/hostile_hostel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hostile Hostel // Creeping Inn: "{T}: Add {C}." / "{1}, {T}, Sacrifice a creature: Put a soul counter on this land..."
/// Under `Coverage::Partial`, the soul-counter transform and linked-exile attack trigger are omitted.
/// Playing the front face enters untapped and immediately taps for colorless mana.
#[test]
fn hostile_hostel_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(227, forest())
        .hand(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hostile_hostel());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, hostile_hostel(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// Hostile Hostel // Creeping Inn: the land is colorless, and the back face
/// prints no mana cost and a black color indicator ("an object is each color
/// denoted by its color indicator", CR 202.2e). Without the indicator the
/// turned-over Creeping Inn stayed colorless.
#[test]
fn creeping_inn_is_black_after_the_flip() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8304, forest())
        .battlefield(0, &[hostile_hostel()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hostel = on_battlefield(&e, p0, hostile_hostel()).expect("Hostile Hostel is on the table");
    let colors =
        |e: &Engine<RegistryLookup>| e.state().object(hostel).unwrap().characteristics().colors;
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::EMPTY,
        "the land is colorless"
    );

    let def = baylee_cards::by_index(hostile_hostel()).unwrap();
    assert!(e.dev_state_mut(p0).unwrap().transform(hostel, def, 1));
    assert_eq!(
        e.state().object(hostel).unwrap().face_index,
        1,
        "Creeping Inn is up"
    );
    assert_eq!(
        colors(&e),
        baylee_core::color::ColorSet::of(baylee_core::color::Color::Black),
        "Creeping Inn has no mana cost and a black color indicator, so it is black and only black (CR 202.2e)"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn hostile_hostel_sacrifices_three_creatures_to_transform_and_creeping_inn_drains_on_attack() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(8305, forest())
        .battlefield(
            0,
            &[
                hostile_hostel(),
                forest(),
                ondu_cleric(),
                ondu_cleric(),
                ondu_cleric(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let hostel = on_battlefield(&e, p0, hostile_hostel()).unwrap();
    for expected_souls in 1..=3 {
        e.dev_state_mut(p0)
            .unwrap()
            .object_mut(hostel)
            .unwrap()
            .status
            .remove(crate::object::Status::TAPPED);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 1);
        e.refresh_offer();
        let victim = on_battlefield(&e, p0, ondu_cleric()).unwrap();
        activate(&mut e, p0, hostile_hostel(), 1);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        let souls = e
            .state()
            .object(hostel)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::SOUL);
        if expected_souls < 3 {
            assert_eq!(souls, expected_souls);
            assert_eq!(e.state().object(hostel).unwrap().face_index, 0);
        } else {
            assert_eq!(souls, 0, "3rd soul counter removes all soul counters");
            assert_eq!(
                e.state().object(hostel).unwrap().face_index,
                1,
                "transforms into Creeping Inn"
            );
            assert!(!is_tapped(&e, hostel), "and untaps it");
        }
    }

    // Attack with Creeping Inn; exile a creature card from graveyard to drain 1 life.
    let gy_creature =
        in_graveyard(&e, p0, ondu_cleric()).expect("sacrificed creature in graveyard");
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(hostel, Defender::Player(PlayerId::new(1)))],
        },
    )
    .unwrap();
    pass_until(&mut e, |eng| matches!(eng.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        panic!("expected graveyard creature choice");
    };
    assert!(options.contains(&gy_creature));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![gy_creature],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(gy_creature).unwrap().zone, Zone::Exile);
    assert_eq!(
        e.state().players[0].life,
        21,
        "you gain X life (X = 1 exiled creature)"
    );
    assert_eq!(
        e.state().players[1].life,
        19,
        "each opponent loses X life (X = 1 exiled creature)"
    );

    // Also exercise Creeping Inn's {4}: This creature phases out.
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, hostile_hostel(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(hostel)
            .unwrap()
            .status
            .contains(crate::object::Status::PHASED_OUT)
    );
}
