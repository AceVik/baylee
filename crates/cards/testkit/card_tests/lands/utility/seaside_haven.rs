//! `cards/lands/utility/seaside_haven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seaside Haven: "{W}{U}, {T}, Sacrifice a Bird: Draw a card." The Elves
/// are not a Bird, so only the Raptor is on offer for the sacrifice.
#[test]
fn seaside_haven_sacrifices_a_bird_to_draw() {
    let p0 = PlayerId::new(0);
    let haven = card_index("4adc39dd-8de1-4298-947c-ff666ec3adeb");
    let mut engine = Duel::new(2302, forest())
        .battlefield(
            0,
            &[haven, plains(), island(), umara_raptor(), llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let h = on_battlefield(&engine, p0, haven).expect("haven");
    let bird = on_battlefield(&engine, p0, umara_raptor()).expect("bird");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf");
    tap_mana_where(&mut engine, p0, |id| id != h && id != bird && id != elf);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, haven, 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected the sacrifice, got {:?}", engine.pending())
    };
    assert!(options.contains(&bird));
    assert!(!options.contains(&elf), "an Elf is no Bird");
    unf_aim_and_pay(&mut engine, p0, None, Some(bird));

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1
    );
    assert!(in_graveyard(&engine, p0, umara_raptor()).is_some());
}
