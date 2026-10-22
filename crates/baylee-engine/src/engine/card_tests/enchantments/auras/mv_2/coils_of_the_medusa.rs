//! `cards/enchantments/auras/mv_2/coils_of_the_medusa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{COILS_OF_THE_MEDUSA, GRIZZLY_BEARS, HILL_GIANT};

/// `Coils of the Medusa` ({1}{B}, Enchantment — Aura) gives enchanted creature
/// +1/-1 and can be sacrificed to destroy all non-Wall creatures blocking
/// enchanted creature.
#[test]
fn coils_of_the_medusa_modifies_pt_and_sacrifice_destroys_non_wall_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[GRIZZLY_BEARS, swamp(), swamp()])
        .battlefield(1, &[HILL_GIANT])
        .hand(0, &[COILS_OF_THE_MEDUSA])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, p1, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, COILS_OF_THE_MEDUSA);
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears)],
            },
        )
        .unwrap();

    activate(&mut engine, p0, COILS_OF_THE_MEDUSA, 2);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, COILS_OF_THE_MEDUSA).is_some());
    assert!(in_graveyard(&engine, p1, HILL_GIANT).is_some());
}
