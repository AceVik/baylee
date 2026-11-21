//! `cards/artifacts/mv_7/summoning_station.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn summoning_station() -> CardIndex {
    card_index("01bba3b8-6372-437e-ab34-ae2c8105cd28")
}

fn pincher_tokens(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == seat
                    && o.card.is_none()
                    && o.characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::PINCHER)
            })
        })
        .collect()
}

/// Summoning Station (01bba3b8-6372-437e-ab34-ae2c8105cd28) — {7} Artifact:
/// "{T}: Create a 2/2 colorless Pincher creature token."
/// "Whenever an artifact is put into a graveyard from the battlefield, you may
///  untap this artifact."
///
/// Creates a 2/2 colorless non-artifact Pincher token, and triggers optional
/// untap whenever either your or an opponent's artifact goes from the
/// battlefield to a graveyard (verifying both declining and accepting).
#[test]
fn summoning_station_creates_pincher_and_untaps_when_any_artifact_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[summoning_station(), lotus_petal()])
        .battlefield(1, &[lotus_petal()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let station = on_battlefield(&engine, p0, summoning_station()).unwrap();

    // 1. Tap Summoning Station -> creates a 2/2 colorless non-artifact Pincher token.
    activate(&mut engine, p0, summoning_station(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, station));

    let tokens = pincher_tokens(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let pincher = engine.state().object(tokens[0]).unwrap();
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(pincher.characteristics().colors.is_empty(), "colorless");
    assert!(
        !pincher
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ARTIFACT),
        "Pincher token is a creature, not an artifact"
    );

    // 2. Sacrifice own Lotus Petal -> triggers Summoning Station -> decline -> stays tapped.
    activate(&mut engine, p0, lotus_petal(), 0);
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, station),
        "declined untap keeps Station tapped"
    );

    // 3. On p1's turn, p1 sacrifices their Lotus Petal -> triggers for p0 -> accept -> untaps!
    reach_their_main_phase(&mut engine, p1);
    activate(&mut engine, p1, lotus_petal(), 0);
    engine
        .apply(p1, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.pending().asked(), Some(p0));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, station),
        "opponent's artifact going to graveyard untaps Summoning Station"
    );
}
