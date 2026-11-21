//! `cards/enchantments/mv_3/spirit_of_resistance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn spirit_of_resistance() -> CardIndex {
    card_index("b075b645-2e5a-4d91-9440-02db4e6f5d08")
}

/// Prevents all damage to you while you control a permanent of each of the five
/// colors (opponent's colors do not count; losing the fifth color via real
/// removal ends the shield).
#[test]
fn spirit_of_resistance_prevents_damage_only_with_all_five_colors() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[
            spirit_of_resistance(),
            festering_goblin(),
            gray_ogre(),
            llanowar_elves(),
        ],
        &[killer_whale(), orcish_artillery()],
        &[killer_whale()],
        &[lightning_bolt(), lightning_bolt(), unsummon()],
    );
    let artillery = lk::obj(&e, p1, orcish_artillery());

    // 4 colors (W, B, R, G) — opponent's Blue Whale does not satisfy the condition.
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.apply(
        p1,
        PlayerAction::ActivateAbility {
            source: artillery,
            ability_index: 0,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 18);

    // Cast fifth color (Blue Killer Whale) -> Bolt to P0 is prevented.
    lk::float(&mut e, p0, &[(ManaColor::Blue, 5)]);
    cast_with_floating(&mut e, p0, killer_whale());
    lk::settle(&mut e);
    let my_whale = lk::obj(&e, p0, killer_whale());

    e.apply(p0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p1, &[(ManaColor::Red, 2), (ManaColor::Blue, 1)]);
    let bolt1 = in_hand(&e, p1, lightning_bolt()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: bolt1 })
        .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 18);

    // P1 bounces my Blue Whale with Unsummon -> drops to 4 colors -> next Bolt deals 3.
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bounce = in_hand(&e, p1, unsummon()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: bounce })
        .unwrap();
    lk::aim(&mut e, &[my_whale], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, my_whale), Zone::Hand);

    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt2 = in_hand(&e, p1, lightning_bolt()).unwrap();
    e.apply(p1, PlayerAction::CastSpell { card: bolt2 })
        .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 15);
}
