//! `cards/creatures/mv_1/orcish_captain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// Aims the Captain's ability at itself in a game of `seed` and reports
/// whether the coin was won. Both outcomes are checked against the text on
/// the way.
fn flip_at_itself(seed: u64) -> bool {
    let mut e = Duel::new(seed, forest())
        .battlefield(
            0,
            &[
                card_index("76c06de7-39ff-4b80-8e54-20910372cffd"),
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let captain = on_battlefield(&e, P0, card_index("76c06de7-39ff-4b80-8e54-20910372cffd"))
        .expect("the Captain is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("the Bears are out");
    assert_eq!(pt(&e, captain), (1, 1));

    fill_pools(&mut e);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: captain,
            ability_index: 0,
        },
    )
    .expect("{1} is paid");
    let menu = aim_at(&mut e, P0, captain);
    assert!(menu.contains(&captain), "the Captain is an Orc");
    assert!(!menu.contains(&bears), "Bears are not Orcs");
    pass_until(&mut e, stack_is_empty);

    if let Some(c) = on_battlefield(&e, P0, card_index("76c06de7-39ff-4b80-8e54-20910372cffd")) {
        assert_eq!(pt(&e, c), (3, 1), "a won flip gives +2/+0");
        pass_until(&mut e, |e| e.state().turn.active == P1);
        let c = on_battlefield(&e, P0, card_index("76c06de7-39ff-4b80-8e54-20910372cffd"))
            .expect("still out");
        assert_eq!(pt(&e, c), (1, 1), "until end of turn");
        true
    } else {
        assert!(
            in_graveyard(&e, P0, card_index("76c06de7-39ff-4b80-8e54-20910372cffd")).is_some(),
            "a lost flip gives -0/-2 to a 1/1, which dies"
        );
        false
    }
}

/// Orcish Captain (76c06de7-39ff-4b80-8e54-20910372cffd) — "{1}: Flip a coin.
/// If you win the flip, target Orc creature gets +2/+0 until end of turn. If
/// you lose the flip, it gets -0/-2 until end of turn."
#[test]
fn orcish_captain_flip_either_boosts_or_shrinks_the_orc_and_both_happen() {
    let mut won = 0;
    let mut lost = 0;
    for seed in 0..48 {
        if flip_at_itself(seed) {
            won += 1;
        } else {
            lost += 1;
        }
    }
    assert!(won > 0, "some flip is won in 48 games");
    assert!(lost > 0, "some flip is lost in 48 games");
}
