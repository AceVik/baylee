//! `cards/lands/manlands/faceless_haven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Faceless Haven is `Coverage::Implemented`.  It is a Snow Land and has:
/// index 0 — `{T}: Add {C}` (printed mana ability, in `legal.abilities`); and
/// index 1 — `{S}{S}{S}: This land becomes a 4/3 creature with vigilance and
/// all creature types until end of turn.  It's still a land.`
///
/// After animation: the permanent is both `CREATURE` and `LAND`, `4/3`, has
/// `KeywordSet::VIGILANCE`, and carries every creature type (`AllCreatureTypes`
/// modifier).  The land type is not removed.
///
/// `{S}` can be paid with one mana from a snow source (Snow Lands are snow
/// sources), so the Haven itself tapping for `{C}` (via its mana ability)
/// counts as one snow mana.  Three snow sources are needed: this test seats
/// three copies of Faceless Haven (only one is played from hand; the others
/// are on the starting battlefield, which plants them without summoning
/// sickness so their mana abilities are available).
#[test]
fn faceless_haven_animates_into_a_4_3_vigilance_creature_with_all_creature_types() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(81, forest())
        .battlefield(0, &[faceless_haven(), faceless_haven()])
        .hand(0, &[faceless_haven()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let played = play_land(&mut engine, p0, faceless_haven());

    // Three Havens are on the board; find which two are the seated ones.
    let all = all_on_battlefield(&engine, p0, faceless_haven());
    assert_eq!(all.len(), 3, "three Havens on the battlefield");

    // Tap the two seated Havens for {C} each (snow mana) via their printed
    // mana ability (index 0, in legal.abilities — no basic subtype).
    let seated: Vec<ObjectId> = all.into_iter().filter(|&id| id != played).collect();
    assert_eq!(seated.len(), 2, "two seated copies");

    for &src in &seated {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: src,
                    ability_index: 0,
                },
            )
            .expect("seated Haven taps for {C}");
        assert!(stack_is_empty(&engine), "mana ability, no stack");
    }
    // Played Haven (index 0) for the third {S}.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: played,
                ability_index: 0,
            },
        )
        .expect("played Haven taps for {C}");
    assert!(stack_is_empty(&engine), "mana ability, no stack");

    // Pool: 3 × {C} — three snow mana covering {S}{S}{S}.
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "three colourless (snow) mana in the pool"
    );

    // The played Haven is now tapped (from its own mana ability).
    // Ability index 1 ({S}{S}{S}) has no {T} in its cost, so a tapped Haven can
    // still activate it — paying the cost comes from the floating pool.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: played,
                ability_index: 1,
            },
        )
        .expect("{C}{C}{C} satisfies {S}{S}{S} from snow sources");
    pass_until(&mut engine, |e| {
        e.state()
            .object(played)
            .is_some_and(|o| o.characteristics().types.contains(TypeSet::CREATURE))
    });

    let obj = engine
        .state()
        .object(played)
        .expect("the Haven is still an object");
    let types = obj.characteristics().types;
    assert!(types.contains(TypeSet::CREATURE), "it became a creature");
    assert!(types.contains(TypeSet::LAND), "it's still a land");
    assert_eq!(pt(&engine, played), (4, 3), "4/3 as printed");
    assert!(
        obj.characteristics()
            .keywords
            .contains(KeywordSet::VIGILANCE),
        "vigilance"
    );
}
