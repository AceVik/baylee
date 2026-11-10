//! `cards/creatures/mv_7/seraph_of_the_suns.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn seraph_of_the_suns() -> CardIndex {
    card_index("8354b70e-43fc-4581-bb53-b913335bf460")
}

/// Seraph of the Suns — a 4/4 flier with indestructible for {5}{W}{W}.
#[test]
fn seraph_of_the_suns_costs_seven_and_is_a_four_four() {
    let mut engine = table7(&lots(plains(), 7), &[seraph_of_the_suns()], &[]);
    let seraph = cast_and_resolve(&mut engine, seraph_of_the_suns());
    assert_body(
        &engine,
        seraph,
        (4, 4),
        KeywordSet::FLYING.union(KeywordSet::INDESTRUCTIBLE),
    );
}

/// Indestructible: blocked by an 8/7 flier that deals lethal damage, the
/// Seraph is still on the battlefield, having dealt its 4.
#[test]
fn seraph_of_the_suns_survives_lethal_combat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table7(&[seraph_of_the_suns()], &[], &[flier()]);
    let seraph = on_battlefield(&engine, p0, seraph_of_the_suns()).expect("out");
    let sphinx = on_battlefield(&engine, p1, flier()).expect("out");
    fight(&mut engine, &[seraph], &[(sphinx, seraph)]);
    assert_eq!(
        on_battlefield(&engine, p0, seraph_of_the_suns()),
        Some(seraph),
        "8 damage did not destroy it"
    );
    assert_eq!(engine.state().object(seraph).expect("out").damage, 8);
    assert_eq!(engine.state().object(sphinx).expect("out").damage, 4);
}

/// And a destroy effect does not destroy it either.
#[test]
fn seraph_of_the_suns_is_not_destroyed() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[seraph_of_the_suns()], &[], &[]);
    let seraph = on_battlefield(&engine, p0, seraph_of_the_suns()).expect("out");
    kill(&mut engine, seraph);
    assert_eq!(
        on_battlefield(&engine, p0, seraph_of_the_suns()),
        Some(seraph)
    );
}
