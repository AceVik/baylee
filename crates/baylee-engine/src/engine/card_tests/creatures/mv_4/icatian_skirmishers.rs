//! `cards/creatures/mv_4/icatian_skirmishers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn icatian_skirmishers() -> CardIndex {
    card_index("8ddbe82b-7fc9-4b2c-85de-dc747adbb3ca")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Skirmishers, Bears and Lions attack together; the Skirmishers band with
/// `band` (or with nobody). Returns the engine with the attack trigger
/// resolved, and the three attackers.
fn attack_banding_with(
    band: impl Fn(ObjectId, ObjectId) -> Vec<ObjectId>,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[icatian_skirmishers(), grizzly_bears(), savannah_lions()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let skirmishers =
        on_battlefield(&engine, p0, icatian_skirmishers()).expect("the Skirmishers are out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("the Lions are out");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: [skirmishers, bears, lions]
                    .into_iter()
                    .map(|a| (a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("expected the band question, got {:?}", engine.pending())
    };
    assert_eq!(prompt, ChoicePrompt::Band { with: skirmishers });
    assert!(
        options.contains(&bears) && options.contains(&lions),
        "both may band with a creature that has banding: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: band(bears, lions),
            },
        )
        .expect("a legal band");
    assert!(
        !stack_is_empty(&engine),
        "the attack trigger is on the stack"
    );
    pass_until(&mut engine, stack_is_empty);
    (engine, skirmishers, bears, lions)
}

/// Icatian Skirmishers (8ddbe82b-7fc9-4b2c-85de-dc747adbb3ca) — "First
/// strike; banding (...) / Whenever this creature attacks, all creatures
/// banded with it gain first strike until end of turn."
#[test]
fn icatian_skirmishers_gives_its_band_mate_first_strike_and_no_one_else() {
    let (engine, skirmishers, bears, lions) = attack_banding_with(|bears, _| vec![bears]);
    let own = keywords(&engine, skirmishers);
    assert!(own.contains(KeywordSet::FIRST_STRIKE) && own.contains(KeywordSet::BANDING));
    assert!(
        engine.state().combat.band_of(bears).is_some()
            && engine.state().combat.band_of(bears) == engine.state().combat.band_of(skirmishers),
        "the Bears are banded with the Skirmishers"
    );
    assert!(
        keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE),
        "the banded creature gains first strike"
    );
    assert!(
        !keywords(&engine, lions).contains(KeywordSet::FIRST_STRIKE),
        "an attacker outside the band does not"
    );
}

/// With nobody banded the trigger still resolves and gives nothing to anyone.
#[test]
fn icatian_skirmishers_attacking_alone_in_its_band_gives_nobody_first_strike() {
    let (engine, _skirmishers, bears, lions) = attack_banding_with(|_, _| vec![]);
    assert!(!keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));
    assert!(!keywords(&engine, lions).contains(KeywordSet::FIRST_STRIKE));
}
