//! Empress Galina — {3}{U}{U} — Legendary Creature — Merfolk Noble
//! Oracle: {U}{U}, {T}: Gain control of target legendary permanent. (This effect lasts indefinitely.)

use super::*;
use baylee_core::generated::index;

#[test]
fn empress_galina_gains_control_of_target_legendary_permanent_indefinitely() {
    let galina = card_index("ea2614bc-00ad-4859-98f5-689f33357fe6");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[galina, island(), island()])
        .battlefield(1, &[index::CAPTAIN_SISAY, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let sisay_id = on_battlefield(&engine, p1, index::CAPTAIN_SISAY).unwrap();
    let bears_id = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, galina, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&sisay_id));
    assert!(!options.contains(&bears_id));

    aim_at(&mut engine, p0, sisay_id);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(sisay_id).unwrap().controller, p0);

    // Effect lasts indefinitely across turn boundaries.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().object(sisay_id).unwrap().controller, p0);
}
