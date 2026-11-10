//! `cards/creatures/mv_2/hazy_homunculus.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn hazy_homunculus() -> CardIndex {
    card_index("1af26f29-627c-48e5-a19d-deb43a11a846")
}

/// The defending player's lands, as the blockers are declared.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Lands {
    None,
    Untapped,
    /// Its one Forest, tapped for mana once the attack is declared.
    Tapped,
}

/// Whether the defender's Gray Ogre is offered as a blocker for the
/// Homunculus. The attacker has an untapped Forest of its own on every
/// board, so a reading of "you control" would answer the same each time.
fn ogre_may_block(lands: Lands) -> bool {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let theirs: &[CardIndex] = match lands {
        Lands::None => &[gray_ogre()],
        Lands::Untapped | Lands::Tapped => &[gray_ogre(), forest()],
    };
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[hazy_homunculus(), forest()])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let homunculus = on_battlefield(&e, p0, hazy_homunculus()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(homunculus, Defender::Player(p1))],
        },
    )
    .expect("the Homunculus attacks");
    let mut tapped = lands != Lands::Tapped;
    for _ in 0..40 {
        match e.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => {
                assert!(tapped, "the defender was given priority to tap first");
                return blockers.iter().any(|option| {
                    option.blocker == ogre && option.attackers.contains(&homunculus)
                });
            }
            Pending::Priority { player, .. } if player == p1 && !tapped => {
                assert_eq!(tap_all_mana(&mut e, p1), 1, "the defender's one Forest");
                tapped = true;
            }
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to the blockers: {other:?}"),
        }
    }
    panic!("never reached the declare-blockers question")
}

/// "This creature can't be blocked as long as defending player controls an
/// untapped land." An untapped land makes it unblockable; a tapped one
/// does not (a reading of "a land" would), and neither does no land. It
/// is a blocking restriction, asked as blockers are declared (CR 509.1b).
#[test]
fn hazy_homunculus_is_unblockable_only_while_the_defender_has_an_untapped_land() {
    assert!(!ogre_may_block(Lands::Untapped));
    assert!(ogre_may_block(Lands::Tapped));
    assert!(ogre_may_block(Lands::None));
}
