//! `cards/creatures/mv_4/bone_shaman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn bone_shaman() -> CardIndex {
    card_index("690ba846-964e-44ba-8637-9a1ce62f5931")
}

/// Bone Shaman (690ba846-964e-44ba-8637-9a1ce62f5931) — {2}{R}{R} 3/3 Creature — Giant Shaman:
/// - "{B}: Until end of turn, Bone Shaman gains 'Creatures dealt damage by Bone
///   Shaman this turn can't be regenerated this turn.'"
///   (`Coverage::Partial`: can't-be-regenerated-when-dealt-damage static not supported).
#[test]
fn bone_shaman_is_a_three_three_and_activates_its_black_mana_ability() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[bone_shaman(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let shaman = on_battlefield(&engine, P0, bone_shaman()).expect("Bone Shaman seated");
    assert_eq!(pt(&engine, shaman), (3, 3));

    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, bone_shaman(), 0);
    pass_until(&mut engine, stack_is_empty);

    let _blocks = attack_and_collect_blocks(&mut engine, shaman, P1);
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);
}
