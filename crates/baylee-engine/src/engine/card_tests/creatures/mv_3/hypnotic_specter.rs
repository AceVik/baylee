//! `cards/creatures/mv_3/hypnotic_specter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hypnotic Specter — Flying; "Whenever this creature deals damage to an
/// opponent, that player discards a card at random." An unblocked attack
/// costs the life and the random card.
#[test]
fn hypnotic_specter_makes_a_damaged_opponent_discard_at_random() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[hypnotic_specter()])
        .hand(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    let specter = on_battlefield(&engine, p0, hypnotic_specter()).expect("seated");
    assert!(keywords(&engine, specter).contains(KeywordSet::FLYING));
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    assert_eq!(hand_before, 1, "one card seeded, to be discarded");

    let blocks = attack_and_collect_blocks(&mut engine, specter, p1);
    assert!(blocks.is_empty(), "no flying or reach blocker to stop it");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 18, "2 combat damage");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before - 1,
        "the damaged opponent discarded one card at random"
    );
}

/// Veteran Bodyguard counter-check, with a real triggered attacker: an
/// unblocked Hypnotic Specter's damage is redirected onto the untapped
/// Bodyguard, and its trigger — "whenever this creature deals damage to
/// an opponent" — never fires, because the damage it actually dealt went
/// to a creature, not a player: nobody discards.
#[test]
fn an_unblocked_specters_redirected_damage_makes_nobody_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hypnotic_specter()])
        .battlefield(1, &[veteran_bodyguard()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let specter = on_battlefield(&engine, p0, hypnotic_specter()).expect("seated");
    let guard = on_battlefield(&engine, p1, veteran_bodyguard()).expect("seated");
    assert!(keywords(&engine, specter).contains(KeywordSet::FLYING));
    assert!(!is_tapped(&engine, guard), "untapped to start");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();
    assert_eq!(
        hand_before, 1,
        "one card seeded, so a discard would be visible"
    );
    let before_life = engine.state().players[1].life;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(specter, Defender::Player(p1))],
            },
        )
        .expect("the Specter attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("flying: the Bodyguard has neither flying nor reach");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        2,
        "the Specter's power, redirected onto the Bodyguard"
    );
    assert_eq!(
        engine.state().players[1].life,
        before_life,
        "not a point off the Bodyguard's controller"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand_before,
        "no opponent was dealt damage, so the trigger never fired: no discard"
    );
}

/// "At random" means the card is not chosen by anyone and is not always the
/// same: against a five-card hand of five different cards, no question is
/// asked (a chosen discard would stop the walk on one), exactly one card
/// leaves the hand, and over sixteen seeds at least three different cards
/// are the one. The same seed picks the same card twice, which is what a
/// replay needs.
#[test]
fn hypnotic_specter_discards_a_random_card_of_several() {
    let hand = [swamp(), forest(), island(), plains(), mountain()];
    let mut seen = Vec::new();
    for seed in 0..16 {
        let discarded = specter_discard(seed, &hand);
        if !seen.contains(&discarded) {
            seen.push(discarded);
        }
    }
    assert!(
        seen.len() >= 3,
        "sixteen seeds, five cards: {} different card(s) discarded, so the pick is not random",
        seen.len()
    );
    assert_eq!(
        specter_discard(7, &hand),
        specter_discard(7, &hand),
        "the same seed discards the same card"
    );
}

/// Attacks p1 with an unblocked Specter on a board where p1 holds `hand`, and
/// names the one card that went to p1's graveyard.
#[track_caller]
fn specter_discard(seed: u64, hand: &[CardIndex]) -> CardIndex {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(seed, swamp())
        .battlefield(0, &[hypnotic_specter()])
        .hand(1, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let specter = on_battlefield(&engine, p0, hypnotic_specter()).expect("seated");
    let blocks = attack_and_collect_blocks(&mut engine, specter, p1);
    assert!(
        blocks.is_empty(),
        "nothing with flying or reach to block it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        hand.len() - 1,
        "exactly one card discarded"
    );
    let gone: Vec<CardIndex> = hand
        .iter()
        .copied()
        .filter(|&card| in_graveyard(&engine, p1, card).is_some())
        .collect();
    assert_eq!(gone.len(), 1, "one card in the graveyard: {gone:?}");
    gone[0]
}
