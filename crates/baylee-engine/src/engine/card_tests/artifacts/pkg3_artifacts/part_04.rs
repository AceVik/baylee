//! Package 3 Artifact behavioral tests — part 04 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn seer_s_lantern() -> CardIndex {
    // oracle_id = "4b3dd7e5-e0b5-4773-bb16-37b757ae74d8"
    card_index("4b3dd7e5-e0b5-4773-bb16-37b757ae74d8")
}

fn sphinx_bone_wand() -> CardIndex {
    // oracle_id = "4b9e529c-4fbd-4944-828d-7fbc8f26c991"
    card_index("4b9e529c-4fbd-4944-828d-7fbc8f26c991")
}

fn murmuring_volume() -> CardIndex {
    // oracle_id = "4baa7847-dfcc-4aa3-aa64-2deeaa3f013b"
    card_index("4baa7847-dfcc-4aa3-aa64-2deeaa3f013b")
}

fn relic_of_legends() -> CardIndex {
    // oracle_id = "4d9554de-c005-41ab-a941-39328ebff9de"
    card_index("4d9554de-c005-41ab-a941-39328ebff9de")
}

fn courier_s_capsule() -> CardIndex {
    // oracle_id = "4e694613-e2e1-4ae3-baa8-f372259e91cf"
    card_index("4e694613-e2e1-4ae3-baa8-f372259e91cf")
}

fn darksteel_brute() -> CardIndex {
    // oracle_id = "4f7d2774-2aec-4992-b35b-e03265e517af"
    card_index("4f7d2774-2aec-4992-b35b-e03265e517af")
}

fn world_map() -> CardIndex {
    // oracle_id = "51d4f95e-aad2-4e5f-9f15-a68bbdf9ab3e"
    card_index("51d4f95e-aad2-4e5f-9f15-a68bbdf9ab3e")
}

fn hierophant_s_chalice() -> CardIndex {
    // oracle_id = "52732f93-b951-43c3-bb02-c254996606e5"
    card_index("52732f93-b951-43c3-bb02-c254996606e5")
}

fn norn_s_wellspring() -> CardIndex {
    // oracle_id = "56a55f45-2cc6-407d-b212-5619c98456d9"
    card_index("56a55f45-2cc6-407d-b212-5619c98456d9")
}

fn shriekhorn() -> CardIndex {
    // oracle_id = "571c1923-790f-48b7-86d2-70c3e79fdba3"
    card_index("571c1923-790f-48b7-86d2-70c3e79fdba3")
}

fn glint_hawk_idol() -> CardIndex {
    // oracle_id = "575f032a-6ae8-4f4f-ab0b-983e7c76ad7f"
    card_index("575f032a-6ae8-4f4f-ab0b-983e7c76ad7f")
}

fn runaway_boulder() -> CardIndex {
    // oracle_id = "57f15861-82cf-4faf-827c-9bc7f4844d2a"
    card_index("57f15861-82cf-4faf-827c-9bc7f4844d2a")
}

fn medicine_bag() -> CardIndex {
    // oracle_id = "5801f773-e214-413d-968d-b84bf99f0642"
    card_index("5801f773-e214-413d-968d-b84bf99f0642")
}

fn hematite_talisman() -> CardIndex {
    // oracle_id = "586601dd-ac73-4451-848d-f21e64feb6a9"
    card_index("586601dd-ac73-4451-848d-f21e64feb6a9")
}

fn lifecrafter_s_bestiary() -> CardIndex {
    // oracle_id = "5f2a3797-28aa-4c7a-ba2b-fd243a1747fd"
    card_index("5f2a3797-28aa-4c7a-ba2b-fd243a1747fd")
}

fn scepter_of_dominance() -> CardIndex {
    // oracle_id = "60bf80f9-78b1-4869-afd9-e769b1b0f5ea"
    card_index("60bf80f9-78b1-4869-afd9-e769b1b0f5ea")
}

fn serum_tank() -> CardIndex {
    // oracle_id = "62a3f892-11cf-4064-8186-381ae0f9aa23"
    card_index("62a3f892-11cf-4064-8186-381ae0f9aa23")
}

fn dimir_cluestone() -> CardIndex {
    // oracle_id = "62f834c7-4765-4b6f-812c-7ce14e13ea0d"
    card_index("62f834c7-4765-4b6f-812c-7ce14e13ea0d")
}

fn shield_of_the_ages() -> CardIndex {
    // oracle_id = "65164450-b477-4096-8146-ebb80a709787"
    card_index("65164450-b477-4096-8146-ebb80a709787")
}

fn orochi_hatchery() -> CardIndex {
    // oracle_id = "65fb1945-ed7a-4f1b-bc6b-166cbec1d8ec"
    card_index("65fb1945-ed7a-4f1b-bc6b-166cbec1d8ec")
}

fn pit_trap() -> CardIndex {
    // oracle_id = "671c4677-fe0a-4bda-9674-f4dcd9069bfd"
    card_index("671c4677-fe0a-4bda-9674-f4dcd9069bfd")
}

fn skull_of_orm() -> CardIndex {
    // oracle_id = "67601399-089b-44fe-82db-c319729c75bf"
    card_index("67601399-089b-44fe-82db-c319729c75bf")
}

fn grindclock() -> CardIndex {
    // oracle_id = "6b7de1c0-4b97-47e1-8ea7-ba55a8d16a9b"
    card_index("6b7de1c0-4b97-47e1-8ea7-ba55a8d16a9b")
}

fn talisman_of_conviction() -> CardIndex {
    // oracle_id = "6c326439-5620-4ec6-a56a-fe9c3d5d2a46"
    card_index("6c326439-5620-4ec6-a56a-fe9c3d5d2a46")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_04() {
    // Seer's Lantern — {3} — Artifact
    verify_perm(&PermSpec {
        card: seer_s_lantern(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Sphinx-Bone Wand — {7} — Artifact
    verify_perm(&PermSpec {
        card: sphinx_bone_wand(),
        primary_mana: &[(ManaColor::Colorless, 7)],
        alt_mana: None,
        total_mana: 7,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Murmuring Volume — {3} — Artifact — Book
    verify_perm(&PermSpec {
        card: murmuring_volume(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Relic of Legends — {3} — Artifact
    verify_perm(&PermSpec {
        card: relic_of_legends(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Courier's Capsule — {1}{U} — Artifact
    verify_perm(&PermSpec {
        card: courier_s_capsule(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Blue],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Darksteel Brute — {2} — Artifact
    verify_perm(&PermSpec {
        card: darksteel_brute(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // World Map — {1} — Artifact
    verify_perm(&PermSpec {
        card: world_map(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Hierophant's Chalice — {3} — Artifact
    verify_perm(&PermSpec {
        card: hierophant_s_chalice(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Norn's Wellspring — {1}{W} — Artifact
    verify_perm(&PermSpec {
        card: norn_s_wellspring(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Shriekhorn — {1} — Artifact
    verify_perm(&PermSpec {
        card: shriekhorn(),
        primary_mana: &[(ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 1,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::Charge, 3)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Glint Hawk Idol — {2} — Artifact
    verify_perm(&PermSpec {
        card: glint_hawk_idol(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Runaway Boulder — {6} — Artifact
    verify_perm(&PermSpec {
        card: runaway_boulder(),
        primary_mana: &[(ManaColor::Colorless, 6)],
        alt_mana: None,
        total_mana: 6,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Medicine Bag — {3} — Artifact
    verify_perm(&PermSpec {
        card: medicine_bag(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Hematite Talisman — {2} — Artifact
    verify_perm(&PermSpec {
        card: hematite_talisman(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Lifecrafter's Bestiary — {3} — Artifact
    verify_perm(&PermSpec {
        card: lifecrafter_s_bestiary(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Scepter of Dominance — {1}{W}{W} — Artifact
    verify_perm(&PermSpec {
        card: scepter_of_dominance(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::White],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Serum Tank — {3} — Artifact
    verify_perm(&PermSpec {
        card: serum_tank(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Dimir Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: dimir_cluestone(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Shield of the Ages — {2} — Artifact
    verify_perm(&PermSpec {
        card: shield_of_the_ages(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Orochi Hatchery — {X}{X} — Artifact
    verify_perm(&PermSpec {
        card: orochi_hatchery(),
        primary_mana: &[(ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 4,
        snow_mana: 0,
        x_value: Some(2),
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::Charge, 2)),
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Pit Trap — {2} — Artifact
    verify_perm(&PermSpec {
        card: pit_trap(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Skull of Orm — {3} — Artifact
    verify_perm(&PermSpec {
        card: skull_of_orm(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Grindclock — {2} — Artifact
    verify_perm(&PermSpec {
        card: grindclock(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Talisman of Conviction — {2} — Artifact
    verify_perm(&PermSpec {
        card: talisman_of_conviction(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
}
