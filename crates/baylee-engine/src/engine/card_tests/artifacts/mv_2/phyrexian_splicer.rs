//! `cards/artifacts/mv_2/phyrexian_splicer.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn phyrexian_splicer() -> CardIndex {
    card_index("4433a5fd-2daa-440a-b9cc-4ad028358cbc")
}

/// Phyrexian Splicer (`4433a5fd-2daa-440a-b9cc-4ad028358cbc`) — {2} Artifact (Partial):
/// "{2}, {T}, Choose flying, first strike, trample, or shadow: Until end of turn, target creature with the chosen ability loses it and another target creature gains it."
#[test]
fn phyrexian_splicer_moves_flying_between_two_creatures() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                phyrexian_splicer(),
                forest(),
                forest(),
                index::AIR_ELEMENTAL,
                index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let air = on_battlefield(&engine, P0, index::AIR_ELEMENTAL).unwrap();
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, phyrexian_splicer(), 0);
    aim_at(&mut engine, P0, air);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(air)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
    assert!(
        engine
            .state()
            .object(bears)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}
