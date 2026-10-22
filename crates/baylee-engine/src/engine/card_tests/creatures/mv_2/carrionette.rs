//! `cards/creatures/mv_2/carrionette.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn carrionette() -> CardIndex {
    card_index("24315f60-731a-40bb-85ba-eedba1b011ff")
}

/// Carrionette (24315f60-731a-40bb-85ba-eedba1b011ff) — {1}{B} Creature — Skeleton:
/// "{2}{B}{B}: Exile this card and target creature unless that creature's controller pays {2}. Activate only if this card is in your graveyard."
#[test]
fn carrionette_exiles_itself_from_graveyard_and_target_creature_when_unpaid() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[carrionette(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let skeleton = on_battlefield(&engine, P0, carrionette()).unwrap();
    bury(&mut engine, &[skeleton]);
    engine.refresh_offer();
    let gy_skeleton = in_graveyard(&engine, P0, carrionette()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, carrionette(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_skeleton).unwrap().zone,
        Zone::Exile
    );
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
}
