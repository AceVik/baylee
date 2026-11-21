//! `cards/artifacts/mv_4/paradise_plume.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn paradise_plume() -> CardIndex {
    card_index("665e52f1-901b-49a1-bfbd-ee38bdee8983")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

fn cast_plume_choosing(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
    color: ManaColor,
) -> ObjectId {
    let plume_card = in_hand(engine, p0, paradise_plume()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: plume_card })
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseColor(color))
        .expect("choose color for Paradise Plume");
    pass_until(engine, stack_is_empty);
    on_battlefield(engine, p0, paradise_plume()).unwrap()
}

fn cast_bolt_at(engine: &mut Engine<RegistryLookup>, caster: PlayerId, target: PlayerId) {
    let bolt = in_hand(engine, caster, lightning_bolt()).unwrap();
    engine
        .apply(caster, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            caster,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

/// Paradise Plume (665e52f1-901b-49a1-bfbd-ee38bdee8983) — {4} Artifact:
/// "As this artifact enters, choose a color."
/// "Whenever a player casts a spell of the chosen color, you may gain 1 life."
/// "{T}: Add one mana of the chosen color."
///
/// Chooses red on entry, taps for {R}, triggers optionally on your red spells
/// (accept + decline), and triggers on the opponent's red spells too.
#[test]
fn paradise_plume_chooses_color_makes_mana_and_triggers_for_both_players() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[paradise_plume(), lightning_bolt(), lightning_bolt()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let plume = cast_plume_choosing(&mut engine, p0, ManaColor::Red);

    // {T}: Add one mana of the chosen color ({R}).
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: plume,
                ability_index: 1,
            },
        )
        .expect("tap Paradise Plume for {R}");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1
    );

    // Accept life gain on own red spell (20 -> 21), then decline on second (stays 21).
    cast_bolt_at(&mut engine, p0, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 21);

    cast_bolt_at(&mut engine, p0, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 21);

    // Opponent casts a red spell at p0: Paradise Plume triggers (+1 then -3 = 19).
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    cast_bolt_at(&mut engine, p1, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 19);
}

/// Non-chosen-color and colorless spells do NOT trigger Paradise Plume.
#[test]
fn paradise_plume_ignores_off_color_and_colorless_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[paradise_plume(), grizzly_bears(), sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_plume_choosing(&mut engine, p0, ManaColor::Blue);

    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bears })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let ring = in_hand(&engine, p0, sol_ring()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: ring })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        life(&engine, p0),
        20,
        "no trigger for green or colorless spells"
    );
}
