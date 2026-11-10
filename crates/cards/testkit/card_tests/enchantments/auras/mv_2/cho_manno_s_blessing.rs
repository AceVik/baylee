//! `cards/enchantments/auras/mv_2/cho_manno_s_blessing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn subject_card() -> CardIndex {
    card_index("fe9f2a67-b8af-48fd-83cd-d15168d2e64f")
}

/// Cho-Manno's Blessing — {W}{W} Enchantment — Aura:
/// "Flash. Enchant creature. As this Aura enters, choose a color. Enchanted creature has protection from the chosen color. This effect doesn't remove this Aura."
#[test]
fn cho_mannos_blessing_grants_protection_from_chosen_color_without_removing_itself() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                index::GRIZZLY_BEARS,
                index::HILL_GIANT,
            ],
        )
        .hand(0, &[subject_card(), index::HOLY_STRENGTH])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, P0, subject_card());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseColor(ManaColor::White))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, subject_card()).is_some());

    cast_from_hand(&mut engine, P0, index::HOLY_STRENGTH);
    let menu = aim_at(&mut engine, P0, giant);
    assert!(!menu.contains(&bears));
}
