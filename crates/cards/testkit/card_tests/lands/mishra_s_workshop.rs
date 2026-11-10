//! `cards/lands/mishra_s_workshop.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn mishras_workshop_pays_for_an_artifact_spell_and_no_planeswalker_of_the_same_cost() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mishras_workshop(), mishras_workshop()])
        .hand(0, &[panharmonicon(), karn_the_great_creator()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `castable` is read off the pool, not off what is
    // still standing untapped.
    tap_all_mana(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let artifact = in_hand(&engine, p0, panharmonicon()).expect("Panharmonicon is in hand");
    let walker = in_hand(&engine, p0, karn_the_great_creator()).expect("Karn is in hand");
    assert!(
        legal.castable.contains(&artifact),
        "six colourless off the two Workshops, and an artifact spell may spend \
         them: {:?}",
        legal.castable
    );
    assert!(
        !legal.castable.contains(&walker),
        "\"Spend this mana only to cast artifact spells\" — a planeswalker is \
         not one, so the same six are not there for it: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, panharmonicon());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, panharmonicon()).is_some(),
        "the artifact resolved on the Workshops' restricted mana and nothing else"
    );
}
