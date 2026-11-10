//! `cards/lands/restricted/moonring_island.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moonring Island prints `This land enters tapped`, `{T}: Add {U}`, and `{U}, {T}: Look at the top card
/// of target player's library. Activate only if you control two or more blue permanents.`
/// The card is marked `Coverage::Partial` because peeking at another player's library is not supported.
/// Moonring Island enters tapped, untaps on the following turn cycle, and with two `island` permanents
/// present offers only ability index 0 for `{U}` and never the library inspection ability.
#[test]
fn moonring_island_enters_tapped_and_taps_for_blue_without_peek_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[moonring_island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, moonring_island());
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

    activate(&mut engine, p0, moonring_island(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}

/// "Activate only if you control two or more blue permanents": the opponent's
/// blue permanents do not count toward your two, however many there are, and
/// two of your own do.
#[test]
fn moonring_island_counts_only_the_blue_permanents_you_control() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for (mine, theirs, offered) in [
        (0, 2, false),
        (0, 3, false),
        (1, 1, false),
        (1, 2, false),
        (2, 0, true),
        (1, 0, false),
    ] {
        let mut ours = vec![moonring_island()];
        ours.extend(std::iter::repeat_n(
            index::MERFOLK_OF_THE_PEARL_TRIDENT,
            mine,
        ));
        let mut e = Duel::new(SEED, island())
            .battlefield(0, &ours)
            .battlefield(
                1,
                &std::iter::repeat_n(index::MERFOLK_OF_THE_PEARL_TRIDENT, theirs)
                    .collect::<Vec<_>>(),
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, moonring_island()).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Blue, 1);
        e.refresh_offer();
        assert!(!is_tapped(&e, land));
        let Pending::Priority { legal, .. } = e.pending().clone() else {
            panic!("priority")
        };
        assert_eq!(
            legal.abilities.contains(&(land, 1)),
            offered,
            "{mine} blue permanents of yours and {theirs} of theirs, {{U}} floating"
        );
        if !offered {
            let hash = e.snapshot_hash();
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: land,
                        ability_index: 1,
                    },
                )
                .is_err(),
                "{mine} of yours and {theirs} of theirs: refused"
            );
            assert_eq!(e.snapshot_hash(), hash);
        }
    }
}
