//! `cards/creatures/mv_2/krovikan_elementalist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krovikan_elementalist() -> CardIndex {
    card_index("43e6fc44-8b23-4bff-91da-936019efe84b")
}

/// Krovikan Elementalist (43e6fc44-8b23-4bff-91da-936019efe84b) — {B}{B} 1/1 Creature — Human Wizard:
/// - "{2}{R}: Target creature gets +1/+0 until end of turn."
/// - "{U}{U}: Target creature you control gains flying until end of turn. Sacrifice it at the beginning of the next end step."
#[test]
fn krovikan_elementalist_pumps_any_creature_and_grants_your_creature_flying_then_sacrifices_at_end_step()
 {
    let mut engine = lk::game(
        SEED,
        &[krovikan_elementalist(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);

    // Ability 0 ("{2}{R}: Target creature gets +1/+0 until end of turn") can target any creature, including P1's.
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
    );
    activate(&mut engine, P0, krovikan_elementalist(), 0);
    let options0 = aim_at(&mut engine, P0, giant);
    assert!(
        options0.contains(&bears) && options0.contains(&giant),
        "{{2}}{{R}} ability can target any creature on the battlefield"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, giant), (4, 3));

    // Ability 1 ("{U}{U}: Target creature you control gains flying until end of turn. Sacrifice it at the beginning of the next end step")
    // can target only P0's creatures and excludes P1's Hill Giant.
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    activate(&mut engine, P0, krovikan_elementalist(), 1);
    let options1 = aim_at(&mut engine, P0, bears);
    assert!(options1.contains(&bears));
    assert!(
        !options1.contains(&giant),
        "opponent's creature is excluded from 'target creature you control'"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, bears).contains(KeywordSet::FLYING),
        "Grizzly Bears gains flying until end of turn"
    );

    // Advance to P1's main phase (past P0's end step): Grizzly Bears was sacrificed at P0's end step,
    // and Hill Giant's +1/+0 wore off at end of turn.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "Grizzly Bears was sacrificed at the beginning of the next end step"
    );
    assert_eq!(pt(&engine, giant), (3, 3));
}
