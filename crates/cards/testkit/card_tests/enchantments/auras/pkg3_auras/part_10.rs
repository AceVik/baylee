//! Package 3 Aura behavioral tests — part 10 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn conquer() -> CardIndex {
    // oracle_id = "be15e131-0753-4ceb-a36b-4f9172241080"
    card_index("be15e131-0753-4ceb-a36b-4f9172241080")
}

fn curse_of_chains() -> CardIndex {
    // oracle_id = "be951382-5173-4fb6-9dda-2c5f7fddc863"
    card_index("be951382-5173-4fb6-9dda-2c5f7fddc863")
}

fn jolrael_s_favor() -> CardIndex {
    // oracle_id = "bf71cca7-be71-4429-9cb6-1eb34ff847eb"
    card_index("bf71cca7-be71-4429-9cb6-1eb34ff847eb")
}

fn future_flight() -> CardIndex {
    // oracle_id = "bfd5b48a-698e-4ade-afb0-954ba288f9a7"
    card_index("bfd5b48a-698e-4ade-afb0-954ba288f9a7")
}

fn divine_favor() -> CardIndex {
    // oracle_id = "bffa18c3-642c-4239-b425-238599007976"
    card_index("bffa18c3-642c-4239-b425-238599007976")
}

fn mists_of_littjara() -> CardIndex {
    // oracle_id = "c0162a6c-12d9-4b84-811a-ee6e39401863"
    card_index("c0162a6c-12d9-4b84-811a-ee6e39401863")
}

fn coma_veil() -> CardIndex {
    // oracle_id = "c062bd12-34cf-4177-92d4-8ce91206989d"
    card_index("c062bd12-34cf-4177-92d4-8ce91206989d")
}

fn elephant_guide() -> CardIndex {
    // oracle_id = "c138bd7f-8751-4e96-b54a-d5082e1a491f"
    card_index("c138bd7f-8751-4e96-b54a-d5082e1a491f")
}

fn paralyzing_grasp() -> CardIndex {
    // oracle_id = "c1459d75-2bc2-4dae-8c7e-4e759a4dffc9"
    card_index("c1459d75-2bc2-4dae-8c7e-4e759a4dffc9")
}

fn dark_favor() -> CardIndex {
    // oracle_id = "c1f4a440-638a-4e3e-8a3e-af4a4a73ec91"
    card_index("c1f4a440-638a-4e3e-8a3e-af4a4a73ec91")
}

fn griffin_guide() -> CardIndex {
    // oracle_id = "c5323a43-82de-4340-8578-b3ffcc66f8fa"
    card_index("c5323a43-82de-4340-8578-b3ffcc66f8fa")
}

fn feather_of_flight() -> CardIndex {
    // oracle_id = "c61b05b8-00f1-4371-9856-c65852b4ca02"
    card_index("c61b05b8-00f1-4371-9856-c65852b4ca02")
}

fn lay_claim() -> CardIndex {
    // oracle_id = "c7111365-b96b-4d6a-8c3c-73a0d8baf641"
    card_index("c7111365-b96b-4d6a-8c3c-73a0d8baf641")
}

fn noxious_field() -> CardIndex {
    // oracle_id = "c75ec3c6-2786-4386-b758-ac03610f2a99"
    card_index("c75ec3c6-2786-4386-b758-ac03610f2a99")
}

fn military_discipline() -> CardIndex {
    // oracle_id = "c910c1c0-2b78-4e0f-bb55-e0581b2e07e8"
    card_index("c910c1c0-2b78-4e0f-bb55-e0581b2e07e8")
}

fn charmed_sleep() -> CardIndex {
    // oracle_id = "cb33edaf-b604-48b8-8b9c-d6e462a8bebb"
    card_index("cb33edaf-b604-48b8-8b9c-d6e462a8bebb")
}

fn grounded() -> CardIndex {
    // oracle_id = "cca3557c-c911-494e-95e7-cebec3442d63"
    card_index("cca3557c-c911-494e-95e7-cebec3442d63")
}

fn lingering_mirage() -> CardIndex {
    // oracle_id = "cce17310-585d-4738-8d5b-bbad65008c37"
    card_index("cce17310-585d-4738-8d5b-bbad65008c37")
}

fn zephyr_net() -> CardIndex {
    // oracle_id = "cd510cf2-df54-4024-919b-e4377cdb2be1"
    card_index("cd510cf2-df54-4024-919b-e4377cdb2be1")
}

fn mantle_of_the_wolf() -> CardIndex {
    // oracle_id = "cd69b2c7-2be7-4915-9d17-4fd400ace6b8"
    card_index("cd69b2c7-2be7-4915-9d17-4fd400ace6b8")
}

fn pursuit_of_flight() -> CardIndex {
    // oracle_id = "cdce8b86-81e3-443c-af91-f6b4f62f5b59"
    card_index("cdce8b86-81e3-443c-af91-f6b4f62f5b59")
}

fn leshrac_s_rite() -> CardIndex {
    // oracle_id = "ced4538a-4919-4ef8-8bf2-2de9e0388587"
    card_index("ced4538a-4919-4ef8-8bf2-2de9e0388587")
}

fn spectral_flight() -> CardIndex {
    // oracle_id = "d127d1e6-f3a6-4f68-bea5-d7b4123476e5"
    card_index("d127d1e6-f3a6-4f68-bea5-d7b4123476e5")
}

fn aura_of_dominion() -> CardIndex {
    // oracle_id = "d2f0a918-4435-4d43-811d-2da659086ef5"
    card_index("d2f0a918-4435-4d43-811d-2da659086ef5")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_10() {
    // Conquer — {3}{R}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: conquer(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Curse of Chains — {1}{W/U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: curse_of_chains(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)]),
        total_mana: 2,
        colors: &[Color::Blue, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Jolrael's Favor — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: jolrael_s_favor(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Future Flight — {2}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: future_flight(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 2,
        etb_taps_attached: false,
    });
    // Divine Favor — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: divine_favor(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mists of Littjara — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mists_of_littjara(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::CreatureOrVehicle,
        expected_pt_delta: Some((-3, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Coma Veil — {4}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: coma_veil(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::ArtifactOrCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Elephant Guide — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: elephant_guide(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Paralyzing Grasp — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: paralyzing_grasp(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Dark Favor — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dark_favor(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Griffin Guide — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: griffin_guide(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Feather of Flight — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: feather_of_flight(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Lay Claim — {5}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lay_claim(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 7,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Permanent,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Noxious Field — {1}{B}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: noxious_field(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Military Discipline — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: military_discipline(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Charmed Sleep — {1}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: charmed_sleep(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Grounded — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: grounded(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Lingering Mirage — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lingering_mirage(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Zephyr Net — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: zephyr_net(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::DEFENDER.union(KeywordSet::FLYING)],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mantle of the Wolf — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mantle_of_the_wolf(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((4, 4)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Pursuit of Flight — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: pursuit_of_flight(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Leshrac's Rite — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: leshrac_s_rite(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::SWAMPWALK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Spectral Flight — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spectral_flight(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Aura of Dominion — {U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aura_of_dominion(),
        primary_mana: &[(ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
