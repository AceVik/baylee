//! `cards/sorceries/mv_3/nature_s_cloak.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn nature_s_cloak() -> CardIndex {
    card_index("bb493c95-5f6d-405b-aac6-c56fe6b6f42f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Nature's Cloak (bb493c95-5f6d-405b-aac6-c56fe6b6f42f) — {2}{G} Sorcery:
/// "Green creatures you control gain forestwalk until end of turn."
#[test]
fn nature_s_cloak_grants_forestwalk_only_to_controlled_green_creatures_until_eot() {
    let mut engine = lk::game(
        SEED,
        &[
            forest(),
            forest(),
            forest(),
            grizzly_bears(),
            savannah_lions(),
        ],
        &[forest(), aurochs()],
        &[nature_s_cloak()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let my_lions = lk::obj(&engine, P0, savannah_lions());
    let their_beast = lk::obj(&engine, P1, aurochs());

    cast_from_hand(&mut engine, P0, nature_s_cloak());
    lk::settle(&mut engine);

    assert!(keywords(&engine, my_bears).contains(KeywordSet::FORESTWALK));
    assert!(!keywords(&engine, my_lions).contains(KeywordSet::FORESTWALK));
    assert!(!keywords(&engine, their_beast).contains(KeywordSet::FORESTWALK));

    let offer = lk::attack(&mut engine, &[my_bears, my_lions]);
    assert!(
        !lk::may_block(&offer, their_beast, my_bears),
        "defender controls a Forest, so the forestwalking green creature is unblockable"
    );
    assert!(
        lk::may_block(&offer, their_beast, my_lions),
        "white creature did not gain forestwalk"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(their_beast, my_bears)],
                },
            )
            .is_err()
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    reach_their_main_phase(&mut engine, P1);
    assert!(!keywords(&engine, my_bears).contains(KeywordSet::FORESTWALK));
}

/// Nature's Cloak — forestwalk does not stop blocking when the defender
/// controls no Forest.
#[test]
fn nature_s_cloak_allows_blocking_when_defender_controls_no_forest() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), grizzly_bears()],
        &[island(), aurochs()],
        &[nature_s_cloak()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_beast = lk::obj(&engine, P1, aurochs());

    cast_from_hand(&mut engine, P0, nature_s_cloak());
    lk::settle(&mut engine);

    let offer = lk::attack(&mut engine, &[my_bears]);
    assert!(lk::may_block(&offer, their_beast, my_bears));
    lk::block(&mut engine, &[(their_beast, my_bears)]);
}
