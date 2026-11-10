//! Package 3 Artifact behavioral tests — part 02 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn dromoka_monument() -> CardIndex {
    // oracle_id = "1c587200-02cc-4c5f-86fe-d13cc7bdad8b"
    card_index("1c587200-02cc-4c5f-86fe-d13cc7bdad8b")
}

fn golgari_signet() -> CardIndex {
    // oracle_id = "1cf51f50-24e4-48d0-95b3-1dad3ffa4bf5"
    card_index("1cf51f50-24e4-48d0-95b3-1dad3ffa4bf5")
}

fn cogworker_s_puzzleknot() -> CardIndex {
    // oracle_id = "2065e9d0-c6e9-4270-898e-9822b532d467"
    card_index("2065e9d0-c6e9-4270-898e-9822b532d467")
}

fn coldsteel_heart() -> CardIndex {
    // oracle_id = "21d700e8-0255-418e-96a0-6fe05b3f836d"
    card_index("21d700e8-0255-418e-96a0-6fe05b3f836d")
}

fn instant_ramen() -> CardIndex {
    // oracle_id = "2283e409-c6c7-4de9-899b-2b3caea5f35e"
    card_index("2283e409-c6c7-4de9-899b-2b3caea5f35e")
}

fn protomatter_powder() -> CardIndex {
    // oracle_id = "22d57bc8-e473-47c5-8e48-c0fb98e66948"
    card_index("22d57bc8-e473-47c5-8e48-c0fb98e66948")
}

fn marble_chalice() -> CardIndex {
    // oracle_id = "236f318e-9bd1-4eb0-8b19-af015b3a74ea"
    card_index("236f318e-9bd1-4eb0-8b19-af015b3a74ea")
}

fn team_transmitter() -> CardIndex {
    // oracle_id = "23714aed-0b26-4e21-b290-36c21815471c"
    card_index("23714aed-0b26-4e21-b290-36c21815471c")
}

fn bag_end_banquet() -> CardIndex {
    // oracle_id = "2522a9dd-bcf3-4b84-abff-52b9e6dbde34"
    card_index("2522a9dd-bcf3-4b84-abff-52b9e6dbde34")
}

fn golden_egg() -> CardIndex {
    // oracle_id = "25977c02-e2d4-4afd-b12d-37ce4d58f453"
    card_index("25977c02-e2d4-4afd-b12d-37ce4d58f453")
}

fn levitating_statue() -> CardIndex {
    // oracle_id = "259ca16d-7753-402b-bcd0-2425500dee99"
    card_index("259ca16d-7753-402b-bcd0-2425500dee99")
}

fn azorius_cluestone() -> CardIndex {
    // oracle_id = "27e04c41-f42c-4d60-8a71-ec2d7c326f64"
    card_index("27e04c41-f42c-4d60-8a71-ec2d7c326f64")
}

fn darksteel_relic() -> CardIndex {
    // oracle_id = "2bda1fb1-528c-4e66-89f1-bb1a85e9181e"
    card_index("2bda1fb1-528c-4e66-89f1-bb1a85e9181e")
}

fn izzet_signet() -> CardIndex {
    // oracle_id = "2fda4fe7-8b0c-489c-a000-6d358e614e34"
    card_index("2fda4fe7-8b0c-489c-a000-6d358e614e34")
}

fn sunstone() -> CardIndex {
    // oracle_id = "300ad0f7-92a7-450d-a9de-b658836fe3cb"
    card_index("300ad0f7-92a7-450d-a9de-b658836fe3cb")
}

fn necrogen_spellbomb() -> CardIndex {
    // oracle_id = "34993bd9-53bf-46b2-bc20-b6daf82382d9"
    card_index("34993bd9-53bf-46b2-bc20-b6daf82382d9")
}

fn lunatic_pandora() -> CardIndex {
    // oracle_id = "34c65c7f-33ab-4f91-84ce-d0c8f4fc5832"
    card_index("34c65c7f-33ab-4f91-84ce-d0c8f4fc5832")
}

fn indatha_crystal() -> CardIndex {
    // oracle_id = "3617c4ec-dc70-4fa8-94f3-fbc48abcaeea"
    card_index("3617c4ec-dc70-4fa8-94f3-fbc48abcaeea")
}

fn ghoulcaller_s_bell() -> CardIndex {
    // oracle_id = "36e1485d-3b8c-4c23-9d12-6937fd216b25"
    card_index("36e1485d-3b8c-4c23-9d12-6937fd216b25")
}

fn manifold_key() -> CardIndex {
    // oracle_id = "370c37f7-2484-40e8-8fd9-c10cf6f5d5f2"
    card_index("370c37f7-2484-40e8-8fd9-c10cf6f5d5f2")
}

fn eye_of_vecna() -> CardIndex {
    // oracle_id = "371a7bbf-5890-46d8-ae57-f020c103b465"
    card_index("371a7bbf-5890-46d8-ae57-f020c103b465")
}

fn the_underworld_cookbook() -> CardIndex {
    // oracle_id = "37d5c720-d090-44bc-91df-0a7bab79a03e"
    card_index("37d5c720-d090-44bc-91df-0a7bab79a03e")
}

fn sacred_armory() -> CardIndex {
    // oracle_id = "37d74df9-cf87-4b82-842a-3941064078fb"
    card_index("37d74df9-cf87-4b82-842a-3941064078fb")
}

fn manhole_cover() -> CardIndex {
    // oracle_id = "38f7ff90-4cdf-4403-ad10-1400e05c32e9"
    card_index("38f7ff90-4cdf-4403-ad10-1400e05c32e9")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_02() {
    // Dromoka Monument — {3} — Artifact
    verify_perm(&PermSpec {
        card: dromoka_monument(),
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
    // Golgari Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: golgari_signet(),
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
    // Cogworker's Puzzleknot — {2} — Artifact
    verify_perm(&PermSpec {
        card: cogworker_s_puzzleknot(),
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
    // Coldsteel Heart — {2} — Snow Artifact
    verify_perm(&PermSpec {
        card: coldsteel_heart(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: true,
        choose_color: true,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Instant Ramen — {2} — Artifact — Food
    verify_perm(&PermSpec {
        card: instant_ramen(),
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
        etb_draws: 1,
    });
    // Protomatter Powder — {2}{U} — Artifact
    verify_perm(&PermSpec {
        card: protomatter_powder(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Marble Chalice — {2}{W} — Artifact
    verify_perm(&PermSpec {
        card: marble_chalice(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
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
    // Team Transmitter — {3} — Artifact
    verify_perm(&PermSpec {
        card: team_transmitter(),
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
    // Bag End Banquet — {6} — Artifact
    verify_perm(&PermSpec {
        card: bag_end_banquet(),
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
    // Golden Egg — {2} — Artifact — Food
    verify_perm(&PermSpec {
        card: golden_egg(),
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
        etb_draws: 1,
    });
    // Levitating Statue — {2} — Artifact
    verify_perm(&PermSpec {
        card: levitating_statue(),
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
    // Azorius Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: azorius_cluestone(),
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
    // Darksteel Relic — {0} — Artifact
    verify_perm(&PermSpec {
        card: darksteel_relic(),
        primary_mana: &[],
        alt_mana: None,
        total_mana: 0,
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
    // Izzet Signet — {2} — Artifact
    verify_perm(&PermSpec {
        card: izzet_signet(),
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
    // Sunstone — {3} — Artifact
    verify_perm(&PermSpec {
        card: sunstone(),
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
    // Necrogen Spellbomb — {1} — Artifact
    verify_perm(&PermSpec {
        card: necrogen_spellbomb(),
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
    // Lunatic Pandora — {1} — Legendary Artifact
    verify_perm(&PermSpec {
        card: lunatic_pandora(),
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
    // Indatha Crystal — {3} — Artifact
    verify_perm(&PermSpec {
        card: indatha_crystal(),
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
    // Ghoulcaller's Bell — {1} — Artifact
    verify_perm(&PermSpec {
        card: ghoulcaller_s_bell(),
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
    // Manifold Key — {1} — Artifact
    verify_perm(&PermSpec {
        card: manifold_key(),
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
    // Eye of Vecna — {2} — Legendary Artifact
    verify_perm(&PermSpec {
        card: eye_of_vecna(),
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
    // The Underworld Cookbook — {1} — Artifact — Book
    verify_perm(&PermSpec {
        card: the_underworld_cookbook(),
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
    // Sacred Armory — {2} — Artifact
    verify_perm(&PermSpec {
        card: sacred_armory(),
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
    // Manhole Cover — {2} — Artifact
    verify_perm(&PermSpec {
        card: manhole_cover(),
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
