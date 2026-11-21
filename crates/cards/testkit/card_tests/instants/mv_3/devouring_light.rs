//! `cards/instants/mv_3/devouring_light.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5334dc24-040e-4094-80f4-8d1b9de01230"
fn devouring_light() -> CardIndex {
    card_index("5334dc24-040e-4094-80f4-8d1b9de01230")
}

/// Devouring Light ({1}{W}{W}, Convoke): can be paid completely by convoking
/// two white creatures and a green creature with 0 mana, targets ONLY attacking
/// or blocking creatures (excluding non-combat creatures), and exiles the target.
#[test]
fn devouring_light_convokes_and_exiles_attacking_or_blocking_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            index::SAVANNAH_LIONS,
            index::WHITE_KNIGHT,
            index::GRIZZLY_BEARS,
        ],
        &[index::HILL_GIANT, index::LLANOWAR_ELVES],
        &[devouring_light()],
        &[],
    );
    let lions = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let knight = lk::obj(&engine, P0, index::WHITE_KNIGHT);
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);
    reach_their_main_phase(&mut engine, P1);

    // P1 attacks with Hill Giant (leaving Elf back); P0 blocks with Grizzly Bears.
    let _ = lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(bears, giant)]);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // P0 has 0 mana; convokes Lions ({W}) + Knight ({W}) + Bears ({1}, untapped blocker!).
    cast_with_floating(&mut engine, P0, devouring_light());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&giant), "attacking Giant is legal");
    assert!(options.contains(&bears), "blocking Bears is legal");
    assert!(!options.contains(&elf), "non-combat Elf is excluded");
    assert!(!options.contains(&lions), "non-combat Lions is excluded");
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[lions, knight, bears], &[]);
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, lions) && is_tapped(&engine, knight) && is_tapped(&engine, bears));
    assert_eq!(engine.state().object(giant).unwrap().zone, Zone::Exile);
    assert!(in_graveyard(&engine, P1, index::HILL_GIANT).is_none());
}
