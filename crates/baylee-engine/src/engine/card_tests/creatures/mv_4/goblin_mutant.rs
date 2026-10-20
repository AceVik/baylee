//! `cards/creatures/mv_4/goblin_mutant.rs`, played.
//!
//! Oracle:
//! - "Trample"
//! - "This creature can't attack if defending player controls an untapped creature with power 3 or greater."
//! - "This creature can't block creatures with power 3 or greater."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_mutant() -> CardIndex {
    card_index("540e1371-baa9-412f-a17a-22ea833ea3e8")
}

#[test]
fn goblin_mutant_attack_and_block_restrictions_against_power_three_or_greater() {
    // Attack restriction: cannot attack if P1 controls an untapped Hill Giant (3/3).
    {
        let mut engine = lk::game(SEED, &[goblin_mutant()], &[index::HILL_GIANT], &[], &[]);
        let mutant = lk::obj(&engine, P0, goblin_mutant());
        assert!(keywords(&engine, mutant).contains(KeywordSet::TRAMPLE));
        let offered = lk::attackers_offered(&mut engine);
        assert!(
            !offered.contains(&mutant),
            "Goblin Mutant cannot attack while defending player controls an untapped power>=3 creature"
        );
    }

    // Block restriction: cannot block an attacking Hill Giant (3/3), can block Grizzly Bears (2/2).
    {
        let mut engine = lk::game(
            SEED,
            &[goblin_mutant()],
            &[index::HILL_GIANT, index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let mutant = lk::obj(&engine, P0, goblin_mutant());
        let giant = lk::obj(&engine, P1, index::HILL_GIANT);
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        pass_until(&mut engine, |e| {
            e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let offer = lk::declare(&mut engine, &[giant, bears]);
        assert!(!lk::may_block(&offer, mutant, giant));
        assert!(lk::may_block(&offer, mutant, bears));
    }
}
