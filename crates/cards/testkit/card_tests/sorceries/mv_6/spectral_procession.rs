//! `cards/sorceries/mv_6/spectral_procession.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "376271a6-80f0-47a6-990a-5ec5517aae42"
fn spectral_procession() -> CardIndex {
    card_index("376271a6-80f0-47a6-990a-5ec5517aae42")
}

/// Spectral Procession: "Create three 1/1 white Spirit creature tokens with
/// flying." Printed `{2/W}{2/W}{2/W}`.
///
/// Three Plains pay it, one white each.
#[test]
fn spectral_procession_costs_three_white_and_makes_three_flying_spirits() {
    let mut engine = table(975, &[spectral_procession()], &[plains(); 3], &[], &[]);
    cast_plain(&mut engine, spectral_procession());
    let spirits = my_tokens(&engine);
    assert_eq!(spirits.len(), 3);
    for token in spirits {
        assert_eq!(pt(&engine, token), (1, 1));
        assert!(
            keywords_of(&engine, token).contains(KeywordSet::FLYING),
            "flying"
        );
    }
}

/// Six Forests pay it as well, two generic for each hybrid symbol; with five
/// it is not offered at all.
#[test]
fn spectral_procession_can_be_paid_with_six_generic_mana() {
    let mut engine = table(976, &[spectral_procession()], &[forest(); 5], &[], &[]);
    tap_all_mana(&mut engine, me());
    let card = in_hand(&engine, me(), spectral_procession()).expect("in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "five generic mana is one short of {{2/W}}{{2/W}}{{2/W}}"
    );

    let mut engine = table(977, &[spectral_procession()], &[forest(); 6], &[], &[]);
    cast_plain(&mut engine, spectral_procession());
    assert_eq!(my_tokens(&engine).len(), 3, "six generic mana were enough");
}
