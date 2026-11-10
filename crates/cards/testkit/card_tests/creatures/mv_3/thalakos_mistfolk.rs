//! `cards/creatures/mv_3/thalakos_mistfolk.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn thalakos_mistfolk() -> CardIndex {
    card_index("c836f8e4-6154-4c7a-a8dd-165ff74f9115")
}

/// `Thalakos Mistfolk` ({2}{U}, 2/1 Creature — Thalakos Illusion,
/// `Coverage::Implemented`):
/// `Shadow`
/// `{U}: Put this creature on top of its owner's library.`
#[test]
fn thalakos_mistfolk_has_shadow_and_puts_itself_on_top_of_owners_library_for_u() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), thalakos_mistfolk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mistfolk = on_battlefield(&engine, p0, thalakos_mistfolk()).unwrap();
    let ch = engine.state().object(mistfolk).unwrap().characteristics();
    assert!(ch.keywords.contains(KeywordSet::SHADOW));
    assert_eq!((ch.power, ch.toughness), (Some(2), Some(1)));

    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, thalakos_mistfolk(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, thalakos_mistfolk()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before + 1
    );
    let top_id = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    assert_eq!(
        engine.state().object(top_id).unwrap().card.unwrap().index,
        thalakos_mistfolk()
    );
}
