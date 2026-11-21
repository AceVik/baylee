//! `cards/instants/mv_4/blind_with_anger.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "97913c3c-eec0-4510-921a-d70c022f7961"
fn blind_with_anger() -> CardIndex {
    card_index("97913c3c-eec0-4510-921a-d70c022f7961")
}

/// Blind with Anger ({3}{R}): excludes legendary creatures, untaps target
/// nonlegendary creature, steals it with haste until end of turn so it can
/// attack immediately, and returns it to its controller next turn without haste.
#[test]
fn blind_with_anger_steals_untaps_grants_haste_and_excludes_legendary() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::LLANOWAR_ELVES, index::LILIANA_THE_REPENTANT],
        &[blind_with_anger()],
        &[],
    );
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);
    let legend = lk::obj(&engine, P1, index::LILIANA_THE_REPENTANT);

    // Advance to P1's turn so P1 can tap Llanowar Elves, then back to P0.
    reach_their_main_phase(&mut engine, P1);
    let _ = lk::attack(&mut engine, &[elf]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    reach_their_main_phase(&mut engine, P0);
    assert!(
        is_tapped(&engine, elf),
        "Elf stayed tapped after attacking on P1's turn"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P0, blind_with_anger());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&elf));
    assert!(
        !options.contains(&legend),
        "legendary creature is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(!is_tapped(&engine, elf), "untapped by Blind with Anger");
    assert_eq!(
        on_battlefield(&engine, P0, index::LLANOWAR_ELVES),
        Some(elf),
        "P0 gained control of Elf"
    );
    assert!(keywords(&engine, elf).contains(KeywordSet::HASTE));

    let _ = lk::attack(&mut engine, &[elf]);
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 19, "stolen hasted Elf dealt 1 to P1");

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(
        on_battlefield(&engine, P1, index::LLANOWAR_ELVES),
        Some(elf),
        "control returned to P1 at end of turn"
    );
    assert!(!keywords(&engine, elf).contains(KeywordSet::HASTE));
}
