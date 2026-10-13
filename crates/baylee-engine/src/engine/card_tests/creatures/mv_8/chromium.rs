//! Printed card behavior through the real engine.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn chromium_counts_current_blockers_on_resolution() {
    assert_printed_rampage(index::CHROMIUM, 2);
}

/// "At the beginning of your upkeep, sacrifice Chromium unless you pay
/// {W}{U}{B}": paying keeps it and spends exactly {W}{U}{B}; declining
/// sacrifices it and spends nothing.
#[test]
fn chromium_upkeep_is_kept_by_paying_and_sacrificed_by_declining() {
    let p0 = PlayerId::new(0);
    for pay in [true, false] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::CHROMIUM])
            .start();
        keep_mulligans(&mut e);
        for c in [ManaColor::White, ManaColor::Blue, ManaColor::Black] {
            give_mana(&mut e, p0, c, 1);
        }
        pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let chromium = e
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .find(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == index::CHROMIUM))
            });
        let pool = &e.state().players[0].mana_pool;
        let left = [ManaColor::White, ManaColor::Blue, ManaColor::Black].map(|c| pool.available(c));
        if pay {
            assert!(chromium.is_some(), "paying {{W}}{{U}}{{B}} keeps Chromium");
            assert_eq!(
                left,
                [0, 0, 0],
                "the payment spends exactly one white, one blue and one black"
            );
        } else {
            assert!(
                chromium.is_none(),
                "declining the payment sacrifices Chromium"
            );
            assert_eq!(
                e.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
                1,
                "the sacrificed Chromium is in its owner's graveyard"
            );
            assert_eq!(left, [1, 1, 1], "declining spends no mana");
        }
    }
}
