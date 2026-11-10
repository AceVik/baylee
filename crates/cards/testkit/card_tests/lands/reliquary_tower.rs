//! `cards/lands/reliquary_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Reliquary Tower removes the cleanup discard, and the control is the whole
/// test.
///
/// "You have no maximum hand size" is a sentence with no visible effect until
/// a hand is too big, and at that moment the *absence* of a question is what
/// it does. A test that only played the Tower would assert nothing an empty
/// implementation fails, so the same nine-card hand is walked into the same
/// cleanup step twice: without the Tower the engine asks for two cards, with
/// it the step goes past in silence.
///
/// Nine and not eight, because a count of one would pass against a
/// `hand_modifier` that was off by one in the other direction.
#[test]
fn reliquary_tower_takes_the_cleanup_discard_away_and_nothing_else_does() {
    let p0 = PlayerId::new(0);
    let hand = [
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
    ];

    for tower in [false, true] {
        let mut builder = Duel::new(4_820 + u64::from(tower), forest()).hand(0, &hand);
        if tower {
            builder = builder.battlefield(0, &[reliquary_tower()]);
        }
        let mut engine = builder.start();
        keep_mulligans(&mut engine);
        // Onto this seat's own turn first: the cleanup step that asks is the
        // *active* player's, and a loop started on the opponent's turn walks
        // straight past the only question it is looking for.
        reach_main_phase(&mut engine, p0);

        // Walk this seat's whole turn out, answering nothing but priority, and
        // watch for the one question the sentence is about.
        let mut asked = None;
        for _ in 0..400 {
            match engine.pending().clone() {
                Pending::DiscardChoice { player, count } if player == p0 => {
                    asked = Some((
                        count,
                        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
                    ));
                    break;
                }
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                // A turn walked out end to end passes through its combat
                // phase, and a question left unanswered stops the walk one
                // phase short of the step it was aimed at. Attacking with
                // nobody is the answer that changes nothing else.
                Pending::ChooseAttackers { player, .. } => {
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                        .expect("declining to attack is always legal");
                }
                _ => break,
            }
            if engine.state().turn.active != p0 {
                break;
            }
        }

        let held = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
        if tower {
            assert_eq!(
                asked, None,
                "with the Tower out there is no maximum hand size, so the \
                 cleanup step asks nothing"
            );
            assert!(
                held > 7,
                "the control half is only worth anything over a hand that is \
                 too big, and this one holds {held}"
            );
        } else {
            let (count, at_cleanup) = asked.expect(
                "a hand of nine over a maximum of seven is asked to discard, \
                 and this is the question the Tower exists to remove",
            );
            assert_eq!(
                usize::from(count),
                at_cleanup - 7,
                "the count is the overflow over CR 514.1's seven, read off \
                 the hand as it stood rather than off an assumption about \
                 whether the turn drew a card"
            );
        }
    }
}
