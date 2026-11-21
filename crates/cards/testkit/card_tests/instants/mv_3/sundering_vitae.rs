//! `cards/instants/mv_3/sundering_vitae.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "132c8a62-00bc-4b89-b775-099d8909f200"
fn sundering_vitae() -> CardIndex {
    card_index("132c8a62-00bc-4b89-b775-099d8909f200")
}

/// Sundering Vitae ({2}{G}, Convoke): can be cast for 0 mana by convoking
/// one green and two other creatures, and destroys target artifact or
/// enchantment (excluding non-artifact creatures).
#[test]
fn sundering_vitae_convokes_and_destroys_target_artifact_or_enchantment() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT, index::ORNITHOPTER],
        &[sol_ring(), index::GLORIOUS_ANTHEM, index::LLANOWAR_ELVES],
        &[sundering_vitae()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let thopter = lk::obj(&engine, P0, index::ORNITHOPTER);
    let ring = lk::obj(&engine, P1, sol_ring());
    let anthem = lk::obj(&engine, P1, index::GLORIOUS_ANTHEM);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    // 0 mana floating; convoke Bears ({G}) + Giant ({1}) + Ornithopter ({1}).
    cast_with_floating(&mut engine, P0, sundering_vitae());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ring) && options.contains(&anthem));
    assert!(!options.contains(&elf), "non-artifact creature excluded");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![anthem],
                players: vec![],
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[bears, giant, thopter], &[]);
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, bears) && is_tapped(&engine, giant) && is_tapped(&engine, thopter));
    assert!(in_graveyard(&engine, P1, index::GLORIOUS_ANTHEM).is_some());
    assert!(on_battlefield(&engine, P1, sol_ring()).is_some());
}
