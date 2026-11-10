//! `cards/creatures/mv_5/border_patrol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Border Patrol — {4}{W}, a 1/6 Human Nomad whose only printed text is
/// vigilance. The body and the keyword are the whole card, so the board reads
/// both: five Plains pay the cost and the creature arrives as a printed 1/6,
/// and a turn later it is declared as an attacker, where vigilance
/// (CR 702.20b) is exactly the difference between a creature that stays
/// untapped and one that turns sideways for the same declaration. The one
/// point of damage is the readable half of the attack: a creature that was
/// never really in combat could not have moved the defending seat's life, and
/// a creature that had tapped to attack could not still be standing here.
#[test]
fn border_patrol_attacks_with_vigilance_and_stays_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(415, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[border_patrol()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {4}{W} off the five Plains, so the pool is empty once the creature has
    // landed and every number below is about the card and not about mana.
    cast_from_hand(&mut engine, p0, border_patrol());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, border_patrol()).is_some()
    });
    let patrol = on_battlefield(&engine, p0, border_patrol()).expect("the Patrol resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Plains are exactly {{4}}{{W}}, so nothing is left floating"
    );
    assert_eq!(pt(&engine, patrol), (1, 6), "the printed 1/6 body");
    assert!(
        types(&engine, patrol).contains(TypeSet::CREATURE),
        "and it is a creature and not merely a card on the battlefield"
    );
    assert!(
        keywords(&engine, patrol).contains(KeywordSet::VIGILANCE),
        "the one keyword the card prints reaches the permanent"
    );
    assert!(
        !is_tapped(&engine, patrol),
        "a creature that has just arrived is untapped"
    );

    // A turn, because a creature cast this turn has summoning sickness
    // (CR 302.6) and may not attack at all; which seat took the first turn is
    // the seed's business, so the walk is written to cross one either way.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&patrol),
        "an untapped creature past summoning sickness may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(patrol, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    assert!(
        !is_tapped(&engine, patrol),
        "vigilance: attacking does not tap it, where a creature without the \
         keyword would have turned sideways in this very declaration"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        19,
        "one power of combat damage reached the seat it attacked"
    );
    assert!(
        !is_tapped(&engine, patrol),
        "and it is still untapped past the damage step, so the keyword was \
         never reverted by the combat itself"
    );
    assert!(
        on_battlefield(&engine, p0, border_patrol()).is_some(),
        "and the Patrol survived its own attack"
    );
}
