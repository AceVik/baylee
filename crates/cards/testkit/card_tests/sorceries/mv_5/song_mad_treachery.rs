//! `cards/sorceries/mv_5/song_mad_treachery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Song-Mad Treachery` // `Song-Mad Ruins` (`Coverage::Implemented`): "Gain control of target
/// creature until end of turn. Untap that creature. It gains haste until end of turn. // This
/// land enters tapped. {T}: Add {R}."
///
/// Under `Coverage::Implemented`, the front-face sorcery executes all three clauses in sequence:
/// it gains control of an opponent's creature via `Modifier::GainControl`, untaps that creature
/// via `Effect::UntapTarget`, and grants it `KeywordSet::HASTE` until end of turn. The test
/// lets the opponent tap their creature during their turn, casts `Song-Mad Treachery` on the
/// following turn, and verifies control transfer, the untap state, and granted haste.
#[test]
fn song_mad_treachery_steals_untaps_and_grants_haste_to_opponent_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(815, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[song_mad_treachery()])
        .start();
    keep_mulligans(&mut engine);

    // Advance to p1's main phase so Llanowar Elves loses summoning sickness and taps for mana.
    reach_their_main_phase(&mut engine, p1);
    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls the creature");
    tap_all_mana(&mut engine, p1);
    assert!(is_tapped(&engine, victim), "p1 tapped the elf for mana");

    // Advance to p0's main phase: only p0's permanents untap, leaving the victim tapped.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        is_tapped(&engine, victim),
        "victim remains tapped across the turn boundary"
    );
    assert_eq!(
        engine.state().object(victim).map(|o| o.controller),
        Some(p1),
        "p1 still controls the victim before the spell resolves"
    );

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, song_mad_treachery());

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target selection, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&victim),
        "the opponent's creature is a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(victim).map(|o| o.controller),
        Some(p0),
        "p0 gained control of the creature"
    );
    assert!(
        !is_tapped(&engine, victim),
        "the creature was untapped by the spell"
    );
    assert!(
        keywords(&engine, victim).contains(KeywordSet::HASTE),
        "the creature gained haste"
    );
    assert!(
        in_graveyard(&engine, p0, song_mad_treachery()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}
