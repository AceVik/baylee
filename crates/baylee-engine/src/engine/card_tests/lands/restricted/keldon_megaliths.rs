//! `cards/lands/restricted/keldon_megaliths.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keldon Megaliths prints `This land enters tapped`, `{T}: Add {R}`, and the Hellbent ability
/// `{1}{R}, {T}: This land deals 1 damage to any target. Activate only if you have no cards in hand.`
/// The card is marked `Coverage::Partial` because `Condition` lacks a condition for having no cards in hand.
/// Keldon Megaliths arrives tapped upon play, untaps on the subsequent turn cycle, and with an empty hand
/// and mana available offers only ability index 0 for `{R}`, omitting the Hellbent damage ability.
#[test]
fn keldon_megaliths_enters_tapped_and_taps_for_red_without_hellbent() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[keldon_megaliths()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, keldon_megaliths());
    assert!(entered_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0)]);

    activate(&mut engine, p0, keldon_megaliths(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// Keldon Megaliths: "Hellbent — {1}{R}, {T}: This land deals 1 damage to
/// any target. Activate only if you have no cards in hand." The word
/// "hellbent" is an ability word with no rules meaning, so what is asserted
/// is the sentence it stands for: the ability is absent with a card in hand
/// and, on an empty hand, kills the 1/1 it is pointed at.
#[test]
fn keldon_megaliths_burns_only_on_an_empty_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut holding = Duel::new(9207, forest())
        .battlefield(0, &[keldon_megaliths(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut holding);
    reach_main_phase(&mut holding, p0);
    let megaliths =
        on_battlefield(&holding, p0, keldon_megaliths()).expect("the Megaliths are seated");
    tap_mana_except(&mut holding, p0, megaliths);
    let Pending::Priority { legal, .. } = holding.pending().clone() else {
        panic!("expected priority, got {:?}", holding.pending())
    };
    assert!(
        !legal.abilities.contains(&(megaliths, 1)),
        "a card in hand is not hellbent: {:?}",
        legal.abilities
    );

    let mut empty = Duel::new(9208, forest())
        .battlefield(0, &[keldon_megaliths(), mountain(), mountain()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut empty);
    reach_main_phase(&mut empty, p0);
    let megaliths =
        on_battlefield(&empty, p0, keldon_megaliths()).expect("the Megaliths are seated");
    assert!(
        !is_tapped(&empty, megaliths),
        "a board the harness seats is standing: \"enters tapped\" is a \
         replacement on the way in, and this land never came in"
    );
    assert!(
        empty.state().zones.list(ZoneLocation::Hand(p0)).is_empty(),
        "turn one, and seat zero takes no draw (CR 103.8a) — crossing a turn \
         to untap would have put a card in this hand and answered the \
         condition for the wrong reason"
    );
    let elf = on_battlefield(&empty, p1, llanowar_elves()).expect("their Elf is seated");
    tap_mana_except(&mut empty, p0, megaliths);
    activate(&mut empty, p0, keldon_megaliths(), 1);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = empty.pending().clone()
    else {
        panic!(
            "\"any target\" is a target choice, got {:?}",
            empty.pending()
        )
    };
    assert!(options.contains(&elf), "the Elf is one of the objects");
    assert!(
        player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice"
    );
    empty
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf came out of the menu");
    pass_until(&mut empty, stack_is_empty);
    assert!(
        on_battlefield(&empty, p1, llanowar_elves()).is_none(),
        "one damage is lethal to a 1/1 (CR 704.5g)"
    );
}
