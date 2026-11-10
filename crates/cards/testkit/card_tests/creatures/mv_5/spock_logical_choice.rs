#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("a877820f-8999-4dfe-a50d-09f63c885ce3")
}

// Scenario: printed vigilance leaves Spock untapped after attacking beside a
// normal attacker; tapping later draws exactly one card, without mana cost.
#[test]
fn spock_attacks_untapped_then_taps_to_draw_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[subject(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let spock = on_battlefield(&e, p0, subject()).unwrap();
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    assert!(keywords(&e, spock).contains(KeywordSet::VIGILANCE));
    pass_until(&mut e, |game| {
        matches!(game.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(spock, Defender::Player(p1)), (elf, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert!(!is_tapped(&e, spock));
    assert!(is_tapped(&e, elf));
    pass_until(
        &mut e,
        |game| matches!(game.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let before = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut e, p0, subject(), 0);
    assert!(is_tapped(&e, spock));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert!(library_size(&e, p0) > 0);
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: spock,
                ability_index: 0
            }
        )
        .is_err()
    );
}

#[test]
fn freshly_cast_spock_cannot_pay_his_tap_cost() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[subject()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    cast_from_hand(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    let spock = on_battlefield(&e, p0, subject()).unwrap();
    assert!(keywords(&e, spock).contains(KeywordSet::VIGILANCE));
    assert!(library_size(&e, p0) > 0);
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: spock,
                ability_index: 0
            }
        )
        .is_err()
    );
    assert!(!is_tapped(&e, spock));
}
