//! Package 3 Aura behavioral tests — part 04 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn chronic_flooding() -> CardIndex {
    // oracle_id = "474db5db-914e-474d-b0c2-7987d5986dc3"
    card_index("474db5db-914e-474d-b0c2-7987d5986dc3")
}

fn squire_s_devotion() -> CardIndex {
    // oracle_id = "4770ad4a-a346-4108-8f61-a077155d4bd1"
    card_index("4770ad4a-a346-4108-8f61-a077155d4bd1")
}

fn cartouche_of_knowledge() -> CardIndex {
    // oracle_id = "488696c2-b0c7-49be-a5a1-e833b6fd8751"
    card_index("488696c2-b0c7-49be-a5a1-e833b6fd8751")
}

fn viridian_harvest() -> CardIndex {
    // oracle_id = "48dc8f34-8c73-4155-8a3d-b26a35260e6b"
    card_index("48dc8f34-8c73-4155-8a3d-b26a35260e6b")
}

fn necromantic_thirst() -> CardIndex {
    // oracle_id = "494895db-3da1-4a19-b7a5-ea47817f4e4f"
    card_index("494895db-3da1-4a19-b7a5-ea47817f4e4f")
}

fn swashbuckling() -> CardIndex {
    // oracle_id = "4992144f-7a36-4f30-9e8a-4355288b50c5"
    card_index("4992144f-7a36-4f30-9e8a-4355288b50c5")
}

fn wolfkin_bond() -> CardIndex {
    // oracle_id = "49e22e88-d4d9-4781-81e3-bf77b76b132b"
    card_index("49e22e88-d4d9-4781-81e3-bf77b76b132b")
}

fn scourge_of_the_nobilis() -> CardIndex {
    // oracle_id = "4a1126a3-f8ce-4790-aa51-1e3f4b63eb18"
    card_index("4a1126a3-f8ce-4790-aa51-1e3f4b63eb18")
}

fn lightning_talons() -> CardIndex {
    // oracle_id = "4a2a7eca-3477-4858-9dab-ba80d4d704cd"
    card_index("4a2a7eca-3477-4858-9dab-ba80d4d704cd")
}

fn madcap_skills() -> CardIndex {
    // oracle_id = "4a9dc9ef-a603-467a-aeed-32cc111394b0"
    card_index("4a9dc9ef-a603-467a-aeed-32cc111394b0")
}

fn aspect_of_gorgon() -> CardIndex {
    // oracle_id = "4b65276e-7d03-4700-834c-48178fe98353"
    card_index("4b65276e-7d03-4700-834c-48178fe98353")
}

fn so_shiny() -> CardIndex {
    // oracle_id = "4bc34109-2050-4736-8247-ba06f0a9114a"
    card_index("4bc34109-2050-4736-8247-ba06f0a9114a")
}

fn mark_of_the_oni() -> CardIndex {
    // oracle_id = "4ce00fdf-c4da-492a-81c9-8c93dd0bf620"
    card_index("4ce00fdf-c4da-492a-81c9-8c93dd0bf620")
}

fn audacity() -> CardIndex {
    // oracle_id = "4eb8d877-ee46-4bbe-99da-fee9e238ca58"
    card_index("4eb8d877-ee46-4bbe-99da-fee9e238ca58")
}

fn lifelink() -> CardIndex {
    // oracle_id = "4fe8316a-9cff-43e9-a1d5-993a0c9daf3a"
    card_index("4fe8316a-9cff-43e9-a1d5-993a0c9daf3a")
}

fn tin_street_market() -> CardIndex {
    // oracle_id = "5102b7c8-6886-4004-9992-416d22b4fce7"
    card_index("5102b7c8-6886-4004-9992-416d22b4fce7")
}

fn entangling_vines() -> CardIndex {
    // oracle_id = "51e7405e-a714-4c76-8913-10d626d2361e"
    card_index("51e7405e-a714-4c76-8913-10d626d2361e")
}

fn debilitating_injury() -> CardIndex {
    // oracle_id = "52eab77d-9a07-4e14-8872-72681d3b3d0e"
    card_index("52eab77d-9a07-4e14-8872-72681d3b3d0e")
}

fn silken_strength() -> CardIndex {
    // oracle_id = "53a59ba9-f4e9-460b-928a-ce6d7ae883f7"
    card_index("53a59ba9-f4e9-460b-928a-ce6d7ae883f7")
}

fn untamed_hunger() -> CardIndex {
    // oracle_id = "550abd9b-9aae-4812-9bc0-7f131d6dcde8"
    card_index("550abd9b-9aae-4812-9bc0-7f131d6dcde8")
}

fn contaminated_ground() -> CardIndex {
    // oracle_id = "55e04860-f4f5-445b-81f2-b500fa9b456a"
    card_index("55e04860-f4f5-445b-81f2-b500fa9b456a")
}

fn favor_of_the_overbeing() -> CardIndex {
    // oracle_id = "56568fc4-7133-40e2-833c-29ecc1f953d6"
    card_index("56568fc4-7133-40e2-833c-29ecc1f953d6")
}

fn steal_enchantment() -> CardIndex {
    // oracle_id = "567a3fa7-5ca9-4812-b81d-4953c401c83d"
    card_index("567a3fa7-5ca9-4812-b81d-4953c401c83d")
}

fn power_taint() -> CardIndex {
    // oracle_id = "585a1c82-8a6c-4770-be9c-3a4916b74a9f"
    card_index("585a1c82-8a6c-4770-be9c-3a4916b74a9f")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_04() {
    // Chronic Flooding — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: chronic_flooding(),
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
    // Squire's Devotion — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: squire_s_devotion(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::LIFELINK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Cartouche of Knowledge — {1}{U} — Enchantment — Aura Cartouche
    verify_aura(&AuraSpec {
        card: cartouche_of_knowledge(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Viridian Harvest — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: viridian_harvest(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Artifact,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Necromantic Thirst — {2}{B}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: necromantic_thirst(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Swashbuckling — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: swashbuckling(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::HASTE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Wolfkin Bond — {4}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: wolfkin_bond(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Scourge of the Nobilis — {2}{R/W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: scourge_of_the_nobilis(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 2), (ManaColor::White, 1)]),
        total_mana: 3,
        colors: &[Color::Red, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Lightning Talons — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lightning_talons(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Madcap Skills — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: madcap_skills(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MENACE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Aspect of Gorgon — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aspect_of_gorgon(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::DEATHTOUCH],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // So Shiny — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: so_shiny(),
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
    // Mark of the Oni — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mark_of_the_oni(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Audacity — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: audacity(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Lifelink — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lifelink(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::LIFELINK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Tin Street Market — {4}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tin_street_market(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Entangling Vines — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: entangling_vines(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::TappedCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Debilitating Injury — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: debilitating_injury(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-2, -2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Silken Strength — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: silken_strength(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::CreatureOrVehicle,
        expected_pt_delta: Some((1, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::REACH],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Untamed Hunger — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: untamed_hunger(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MENACE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Contaminated Ground — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: contaminated_ground(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Favor of the Overbeing — {1}{G/U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: favor_of_the_overbeing(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)]),
        total_mana: 2,
        colors: &[Color::Green, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Steal Enchantment — {U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: steal_enchantment(),
        primary_mana: &[(ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Enchantment,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Power Taint — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: power_taint(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Enchantment,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
