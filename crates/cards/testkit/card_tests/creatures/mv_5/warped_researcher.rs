//! `cards/creatures/mv_5/warped_researcher.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
#[allow(clippy::wildcard_imports)]
use baylee_card_testkit::card_tests::legends_kit::*;

fn warped_researcher() -> CardIndex {
    card_index("069862f9-f051-40cc-84f2-c0208873a34c")
}

fn lonely_sandbar() -> CardIndex {
    card_index("765863c8-1be0-4bb1-9e9c-db7701cffde3")
}

fn putrid_imp() -> CardIndex {
    card_index("7192ff07-9a84-49ad-a40c-ce255ce21544")
}

/// Warped Researcher gains flying and shroud until end of turn whenever any
/// player cycles a card (preventing even its controller from targeting it),
/// ignores plain discards, and loses both keywords at cleanup.
#[test]
fn warped_researcher_gains_flying_and_shroud_on_any_cycle_and_expires() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), forest(), warped_researcher(), putrid_imp()])
        .battlefield(1, &[island()])
        .hand(0, &[lonely_sandbar(), giant_growth(), swamp()])
        .hand(1, &[lonely_sandbar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let researcher = on_battlefield(&engine, p0, warped_researcher()).expect("Researcher seated");
    let imp = on_battlefield(&engine, p0, putrid_imp()).expect("Imp seated");
    assert!(!keywords(&engine, researcher).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, researcher).contains(KeywordSet::SHROUD));

    // Plain discard via Putrid Imp does not trigger Warped Researcher.
    let swamp_card = in_hand(&engine, p0, swamp()).expect("Swamp in hand");
    activate(&mut engine, p0, putrid_imp(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swamp_card],
            },
        )
        .expect("discard Swamp");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(!keywords(&engine, researcher).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, researcher).contains(KeywordSet::SHROUD));

    // P0 cycles Lonely Sandbar -> Researcher gains flying and shroud.
    let u = on_battlefield(&engine, p0, island()).expect("Island");
    tap_mana_where(&mut engine, p0, |id| id == u);
    activate(&mut engine, p0, lonely_sandbar(), 1);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(keywords(&engine, researcher).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, researcher).contains(KeywordSet::SHROUD));

    // Shroud prevents even controller P0 from targeting Warped Researcher with Giant Growth.
    let g = on_battlefield(&engine, p0, forest()).expect("Forest");
    tap_mana_where(&mut engine, p0, |id| id == g);
    cast_with_floating(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets for Giant Growth: {:?}",
            engine.pending()
        );
    };
    assert!(
        !options.contains(&researcher),
        "shroud keeps Warped Researcher untargetable even by its controller"
    );
    assert!(options.contains(&imp));
    aim(&mut engine, &[imp], &[]);
    pass_until(&mut engine, stack_is_empty);

    // Cleanup removes flying and shroud; P1 cycling on P1's turn grants both again.
    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, researcher).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, researcher).contains(KeywordSet::SHROUD));

    tap_all_mana(&mut engine, p1);
    activate(&mut engine, p1, lonely_sandbar(), 1);
    pass_until(&mut engine, |e| at_rest(e, p1));
    assert!(keywords(&engine, researcher).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, researcher).contains(KeywordSet::SHROUD));
}
