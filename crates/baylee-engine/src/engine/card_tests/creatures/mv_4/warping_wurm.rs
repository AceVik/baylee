//! `cards/creatures/mv_4/warping_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn warping_wurm() -> CardIndex {
    card_index("c54c1517-cf7c-4afb-92f8-70c9d1591b45")
}

fn phased_out(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .status
        .contains(Status::PHASED_OUT)
}

/// Warping Wurm (`{2}{G}{U}`, `Coverage::Partial`): 1/1 Wurm with phasing that
/// phases out at the beginning of your upkeep unless you pay `{2}{G}{U}`.
/// Declining the price phases it out.
#[test]
fn warping_wurm_phases_out_when_the_upkeep_price_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[warping_wurm(), forest(), forest(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let wurm = on_battlefield(&engine, p0, warping_wurm()).unwrap();
    assert!(keywords(&engine, wurm).contains(KeywordSet::PHASING));
    // Turn 1 upkeep: decline to pay {2}{G}{U} -> Warping Wurm phases out.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(
        phased_out(&engine, wurm),
        "declining the price phases it out"
    );
    assert!(
        !lands_of(&engine, p0)
            .into_iter()
            .any(|l| is_tapped(&engine, l)),
        "nothing was paid"
    );
}

/// The other answer: the question names the printed price, paying it taps the
/// four lands and the Wurm stays phased in. The ability is "*your* upkeep": on
/// the opponent's upkeep no question comes (`pass_until` refuses an unexpected
/// one), and the Wurm is still there afterwards. On the controller's next
/// untap step the Phasing line takes it out, so no price is asked then either.
#[test]
fn warping_wurm_stays_in_when_the_price_is_paid_and_asks_only_on_its_controllers_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[warping_wurm(), forest(), forest(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    let wurm = on_battlefield(&engine, p0, warping_wurm()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "\"at the beginning of *your* upkeep\"");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{2}{G}{U}")
        },
        "the printed price"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);
    assert!(!phased_out(&engine, wurm), "paid, so it stays phased in");
    assert!(
        lands_of(&engine, p0)
            .into_iter()
            .all(|l| is_tapped(&engine, l)),
        "the {{2}}{{G}}{{U}} came out of the four lands"
    );

    // The opponent's turn: no question on their upkeep, the Wurm stays in.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !phased_out(&engine, wurm),
        "only its controller's untap phases it"
    );
    // Back on the controller's turn the Phasing line takes it out in the untap
    // step, so it is not there to ask for the price.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        phased_out(&engine, wurm),
        "phasing: out at its controller's untap"
    );
}
