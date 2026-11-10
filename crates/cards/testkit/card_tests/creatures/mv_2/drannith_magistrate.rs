//! `cards/creatures/mv_2/drannith_magistrate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drannith Magistrate: "Your opponents can't cast spells from anywhere
/// other than their hands."
///
/// The zone is the whole sentence, so the test is one board read twice: a
/// Sol Ring in the opponent's hand and a commander in their command zone,
/// with four Swamps floating that pay for either of them. Without the
/// Magistrate both are offered; with it, the hand card alone — which is
/// what makes this a statement about the *zone* and not about the price or
/// about commanders.
#[test]
fn drannith_magistrate_leaves_an_opponent_their_hand_and_nothing_else() {
    let p1 = PlayerId::new(1);
    for magistrate in [false, true] {
        let mut board = vec![plains(), plains()];
        if magistrate {
            board.push(drannith_magistrate());
        }
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .commander(1, &[sheoldred_the_apocalypse()])
            .battlefield(1, &[swamp(), swamp(), swamp(), swamp()])
            .hand(1, &[quiet_artifact()])
            .start();
        keep_mulligans(&mut engine);
        reach_their_main_phase(&mut engine, p1);
        tap_all_mana(&mut engine, p1);
        assert_eq!(
            engine.state().players[1]
                .mana_pool
                .available(ManaColor::Black),
            4,
            "{{2}}{{B}}{{B}} for the commander and {{1}} for the Sol Ring are \
             both floating, so the price refuses neither"
        );

        let ring = in_hand(&engine, p1, quiet_artifact()).expect("the Sol Ring is in hand");
        let boss = engine
            .state()
            .zones
            .list(ZoneLocation::Command(p1))
            .first()
            .copied()
            .expect("the commander is in the command zone");
        let Pending::Priority { player, legal } = engine.pending().clone() else {
            panic!("expected p1's priority, got {:?}", engine.pending())
        };
        assert_eq!(player, p1, "p1's own main phase");
        assert!(
            legal.castable.contains(&ring),
            "a card in hand is castable either way, Magistrate {magistrate}"
        );
        assert_eq!(
            legal.castable.contains(&boss),
            !magistrate,
            "the command zone is \"anywhere other than their hands\", \
             Magistrate {magistrate}"
        );
    }
}
