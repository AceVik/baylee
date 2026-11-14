//! `cards/creatures/mv_1/nightscape_apprentice.rs`, played.
//!
//! Oracle:
//! - "{U}, {T}: Put target creature you control on top of its owner's library."
//! - "{R}, {T}: Target creature gains first strike until end of turn."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nightscape_apprentice() -> CardIndex {
    card_index("429de3c3-3f2e-4d55-a04a-f414912aff3a")
}

#[test]
fn nightscape_apprentice_bounces_own_creature_to_library_top_and_grants_first_strike() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                mountain(),
                swamp(),
                nightscape_apprentice(),
                nightscape_apprentice(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::SAVANNAH_LIONS])
        .hand(0, &[nightscape_apprentice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let copies = all_on_battlefield(&engine, P0, nightscape_apprentice());
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    assert_eq!(pt(&engine, copies[0]), (1, 1));

    tap_all_mana(&mut engine, P0);

    // Ability 1: "{R}, {T}: Target creature gains first strike until end of turn."
    activate(&mut engine, P0, nightscape_apprentice(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears), "own creature is a valid target");
    assert!(
        options.contains(&lions),
        "opponent creature is also a valid target"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, bears).contains(KeywordSet::FIRST_STRIKE));

    // Ability 0: "{U}, {T}: Put target creature you control on top of its owner's library."
    let p0_lib_before = library_size(&engine, P0);
    activate(&mut engine, P0, nightscape_apprentice(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&lions),
        "must only target a creature you control"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none());
    assert_eq!(library_size(&engine, P0), p0_lib_before + 1);
    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(P0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top).unwrap().card.unwrap().index,
        index::GRIZZLY_BEARS
    );

    // Cast third copy with leftover {B}; summoning sickness bars its {T} abilities.
    cast_with_floating(&mut engine, P0, nightscape_apprentice());
    pass_until(&mut engine, stack_is_empty);
    let sick = all_on_battlefield(&engine, P0, nightscape_apprentice())
        .into_iter()
        .find(|id| !copies.contains(id))
        .unwrap();
    assert!(!ability_offered(&engine, sick, 0));
    assert!(!ability_offered(&engine, sick, 1));
}
