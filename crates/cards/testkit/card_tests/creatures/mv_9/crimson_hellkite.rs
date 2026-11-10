//! `cards/creatures/mv_9/crimson_hellkite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::SERRA_ANGEL;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn crimson_hellkite() -> CardIndex {
    card_index("37a1f340-10d5-407e-bf2b-cbbbb6cab985")
}

/// Crimson Hellkite (37a1f340-10d5-407e-bf2b-cbbbb6cab985) — {6}{R}{R}{R} 6/6:
/// "Flying. {X}, {T}: This creature deals X damage to target creature. Spend
/// only red mana on X."
///
/// The damage ability is left off: "spend only red mana on X" cannot be said
/// for an activated ability, and with any colour paying X it would be stronger
/// than printed. This pins that it is not offered even with plenty of mana;
/// when the restriction exists this test has to move to the printed behaviour.
#[test]
fn crimson_hellkite_flies_and_offers_no_x_damage_ability() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                crimson_hellkite(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[SERRA_ANGEL])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let hellkite = on_battlefield(&engine, P0, crimson_hellkite()).unwrap();
    assert!(keywords(&engine, hellkite).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, P0);
    assert!(
        !ability_offered(&engine, hellkite, 0),
        "no X ability is printed on the card as it stands"
    );
    assert!(on_battlefield(&engine, P1, SERRA_ANGEL).is_some());
}
