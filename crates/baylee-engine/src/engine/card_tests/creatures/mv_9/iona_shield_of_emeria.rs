//! `cards/creatures/mv_9/iona_shield_of_emeria.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("5e816472-561d-43d1-ad28-80a829e2a91a")
}

#[test]
fn iona_choice_locks_opponents_chosen_color_but_not_her_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                mountain(),
            ],
        )
        .hand(0, &[subject(), lightning_bolt()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, |g| {
        matches!(g.pending(), Pending::ChooseColor { .. })
    });
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let iona = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, iona).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut e, p1);
    tap_all_mana(&mut e, p1);
    assert!(
        !priority_offer(&e)
            .castable
            .contains(&in_hand(&e, p1, lightning_bolt()).unwrap())
    );
    reach_their_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    assert!(
        priority_offer(&e)
            .castable
            .contains(&in_hand(&e, p0, lightning_bolt()).unwrap())
    );
}
