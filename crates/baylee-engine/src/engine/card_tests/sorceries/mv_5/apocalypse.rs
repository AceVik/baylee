//! `cards/sorceries/mv_5/apocalypse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn apocalypse() -> CardIndex {
    card_index("82c8f5dd-563d-4fd0-bd43-7ee2001d3777")
}

/// Apocalypse (82c8f5dd-563d-4fd0-bd43-7ee2001d3777) — {2}{R}{R}{R} Sorcery:
/// "Exile all permanents. You discard your hand."
#[test]
fn apocalypse_exiles_all_permanents_and_discards_your_hand() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[apocalypse(), index::HILL_GIANT])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let bears = on_battlefield(&engine, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, P0, apocalypse());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .is_empty()
    );
    assert_eq!(engine.state().object(elf).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().object(bears).unwrap().zone, Zone::Exile);
    assert!(engine.state().zones.list(ZoneLocation::Hand(P0)).is_empty());
    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert!(in_hand(&engine, PlayerId::new(1), forest()).is_some());
    assert!(in_graveyard(&engine, PlayerId::new(1), forest()).is_none());
}
