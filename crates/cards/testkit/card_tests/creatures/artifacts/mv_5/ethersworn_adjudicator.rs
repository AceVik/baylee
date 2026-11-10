//! `cards/creatures/artifacts/mv_5/ethersworn_adjudicator.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ethersworn_adjudicator() -> CardIndex {
    card_index("ca2a7669-4932-4a78-8a6c-8558b7c05ba5")
}

/// Ethersworn Adjudicator — "{1}{W}{B}, {T}: Destroy target creature or
/// enchantment." A creature and an enchantment are both legal, an artifact
/// is not, and the Adjudicator taps to pay.
#[test]
fn ethersworn_adjudicator_destroys_a_creature_or_an_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for victim in [gray_ogre(), sylvan_library()] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[ethersworn_adjudicator(), plains(), swamp(), forest()])
            .battlefield(1, &[gray_ogre(), sylvan_library(), sol_ring()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let adj = on_battlefield(&engine, p0, ethersworn_adjudicator()).expect("seated");
        assert_eq!(pt(&engine, adj), (4, 4));
        assert!(keywords(&engine, adj).contains(KeywordSet::FLYING));

        tap_all_mana(&mut engine, p0);
        activate(&mut engine, p0, ethersworn_adjudicator(), 1);
        let options = pass_until_targets(&mut engine, p0);
        let target = on_battlefield(&engine, p1, victim).expect("their permanent");
        let ring = on_battlefield(&engine, p1, sol_ring()).expect("their artifact");
        assert!(options.contains(&target));
        assert!(!options.contains(&ring), "an artifact is neither");
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![target],
                },
            )
            .expect("a legal target");
        pass_until(&mut engine, stack_is_empty);
        assert!(in_graveyard(&engine, p1, victim).is_some(), "destroyed");
        assert!(is_tapped(&engine, adj), "{{T}} is part of the cost");
        assert!(on_battlefield(&engine, p1, sol_ring()).is_some());
    }
}

/// "{2}{U}: Untap this creature": after attacking, three Islands untap it.
#[test]
fn ethersworn_adjudicator_untaps_for_two_and_blue() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ethersworn_adjudicator(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let adj = on_battlefield(&engine, p0, ethersworn_adjudicator()).expect("seated");
    attack_alone(&mut engine, p0, adj, p1);
    through_combat(&mut engine);
    assert!(is_tapped(&engine, adj));

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, ethersworn_adjudicator(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, adj), "untapped");
}
