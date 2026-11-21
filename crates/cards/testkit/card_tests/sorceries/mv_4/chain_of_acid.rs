//! `cards/sorceries/mv_4/chain_of_acid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn chain_of_acid() -> CardIndex {
    card_index("0b1a27bd-bb98-44f8-8357-666fabfeabf0")
}

/// Chain of Acid (0b1a27bd-bb98-44f8-8357-666fabfeabf0) — {3}{G} Sorcery:
/// "Destroy target noncreature permanent. Then that permanent's controller may
/// copy this spell and may choose a new target for that copy."
#[test]
fn chain_of_acid_destroys_noncreature_and_lets_its_controller_copy_back() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), forest(), forest(), island(), aurochs()],
        &[sol_ring(), mirri_s_guile(), fangren_hunter()],
        &[chain_of_acid()],
        &[],
    );
    let my_land = lk::obj(&engine, P0, island());
    let my_creature = lk::obj(&engine, P0, aurochs());
    let their_ring = lk::obj(&engine, P1, sol_ring());
    let their_ench = lk::obj(&engine, P1, mirri_s_guile());
    let their_creature = lk::obj(&engine, P1, fangren_hunter());

    cast_from_hand(&mut engine, P0, chain_of_acid());
    let menu = aim_at(&mut engine, P0, their_ring);
    assert!(
        menu.contains(&their_ring) && menu.contains(&their_ench) && menu.contains(&my_land),
        "artifacts, enchantments, and lands are legal noncreature targets"
    );
    assert!(!menu.contains(&my_creature) && !menu.contains(&their_creature));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(lk::zone_of(&engine, their_ring), Zone::Graveyard);
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(player, P1, "destroyed permanent's controller may copy");
    lk::yes_no(&mut engine, true);

    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::YesNo { .. } => lk::yes_no(&mut engine, true),
            Pending::ChooseTargets { player, .. } => {
                assert_eq!(player, P1);
                lk::aim(&mut engine, &[my_land], &[]);
                break;
            }
            _ => break,
        }
    }

    // P0 declines copying back after my_land is destroyed.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(lk::zone_of(&engine, my_land), Zone::Graveyard);
    let Pending::YesNo { player: p0_ask, .. } = engine.pending().clone() else {
        unreachable!();
    };
    assert_eq!(p0_ask, P0);
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);
}
