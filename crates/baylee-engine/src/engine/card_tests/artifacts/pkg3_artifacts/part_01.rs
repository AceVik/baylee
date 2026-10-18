//! Package 3 Artifact behavioral tests — part 01 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn static_orb() -> CardIndex {
    // oracle_id = "0004ebd0-dfd6-4276-b4a6-de0003e94237"
    card_index("0004ebd0-dfd6-4276-b4a6-de0003e94237")
}

fn courier_s_briefcase() -> CardIndex {
    // oracle_id = "00cd3bf9-0dd6-4ec0-a841-1839a71604ab"
    card_index("00cd3bf9-0dd6-4ec0-a841-1839a71604ab")
}

fn tumble_magnet() -> CardIndex {
    // oracle_id = "0481adcd-b66d-432e-80ba-3a4718bb3a7d"
    card_index("0481adcd-b66d-432e-80ba-3a4718bb3a7d")
}

fn nutrient_block() -> CardIndex {
    // oracle_id = "04a0382f-7e4e-4450-ae70-18999379d875"
    card_index("04a0382f-7e4e-4450-ae70-18999379d875")
}

fn unstable_obelisk() -> CardIndex {
    // oracle_id = "060ae1bb-956a-4264-b405-a33151f31493"
    card_index("060ae1bb-956a-4264-b405-a33151f31493")
}

fn spare_supplies() -> CardIndex {
    // oracle_id = "088fb5dc-65eb-42bf-8a32-76fa6b79b79f"
    card_index("088fb5dc-65eb-42bf-8a32-76fa6b79b79f")
}

fn orzhov_cluestone() -> CardIndex {
    // oracle_id = "09650a76-05c4-40a0-b861-d146b3ba87d6"
    card_index("09650a76-05c4-40a0-b861-d146b3ba87d6")
}

fn simic_cluestone() -> CardIndex {
    // oracle_id = "0b65050d-a9cd-4b6c-9114-af89bfa6682f"
    card_index("0b65050d-a9cd-4b6c-9114-af89bfa6682f")
}

fn silumgar_monument() -> CardIndex {
    // oracle_id = "0b88268c-7fcb-4fd2-96cd-f93b37413283"
    card_index("0b88268c-7fcb-4fd2-96cd-f93b37413283")
}

fn implement_of_examination() -> CardIndex {
    // oracle_id = "0baf75b3-cc02-47cc-873a-8d5f17da0ec9"
    card_index("0baf75b3-cc02-47cc-873a-8d5f17da0ec9")
}

fn malachite_talisman() -> CardIndex {
    // oracle_id = "0be4877d-72ff-4951-aa52-f4f816abc844"
    card_index("0be4877d-72ff-4951-aa52-f4f816abc844")
}

fn fireforger_s_puzzleknot() -> CardIndex {
    // oracle_id = "0de18ba2-84a7-4e07-84b2-771761d77aaf"
    card_index("0de18ba2-84a7-4e07-84b2-771761d77aaf")
}

fn dreamstone_hedron() -> CardIndex {
    // oracle_id = "0e2575be-c596-4c8c-bf07-7941ca065721"
    card_index("0e2575be-c596-4c8c-bf07-7941ca065721")
}

fn futurist_forge() -> CardIndex {
    // oracle_id = "0e5f97b5-bb8c-4688-a141-5db4d4eeccf0"
    card_index("0e5f97b5-bb8c-4688-a141-5db4d4eeccf0")
}

fn potion_of_healing() -> CardIndex {
    // oracle_id = "11fb3fee-ae84-4bf3-b834-63d3b8987df3"
    card_index("11fb3fee-ae84-4bf3-b834-63d3b8987df3")
}

fn imi_statue() -> CardIndex {
    // oracle_id = "1201520a-d76a-4c8a-be5c-ff37ffd2c024"
    card_index("1201520a-d76a-4c8a-be5c-ff37ffd2c024")
}

fn firdoch_core() -> CardIndex {
    // oracle_id = "126c75c0-414b-4e55-8747-2b5566ce716f"
    card_index("126c75c0-414b-4e55-8747-2b5566ce716f")
}

fn prophetic_prism() -> CardIndex {
    // oracle_id = "134a9877-5cfb-4a2a-a0f0-930dce45f58b"
    card_index("134a9877-5cfb-4a2a-a0f0-930dce45f58b")
}

fn stuffed_bear() -> CardIndex {
    // oracle_id = "1402c116-194e-49ca-a8c1-09ca659c220c"
    card_index("1402c116-194e-49ca-a8c1-09ca659c220c")
}

fn talisman_of_creativity() -> CardIndex {
    // oracle_id = "14d2979d-5728-42d7-a027-0eb1f754655d"
    card_index("14d2979d-5728-42d7-a027-0eb1f754655d")
}

fn trigon_of_mending() -> CardIndex {
    // oracle_id = "17b1c870-ef94-49af-8f9b-a8765f1b54db"
    card_index("17b1c870-ef94-49af-8f9b-a8765f1b54db")
}

fn vial_of_dragonfire() -> CardIndex {
    // oracle_id = "17be934e-1e33-4b95-a984-242fa018e280"
    card_index("17be934e-1e33-4b95-a984-242fa018e280")
}

fn pristine_talisman() -> CardIndex {
    // oracle_id = "1b3d7fce-e9fe-4176-9d9a-472415826cdd"
    card_index("1b3d7fce-e9fe-4176-9d9a-472415826cdd")
}

fn astral_cornucopia() -> CardIndex {
    // oracle_id = "1bc42024-52da-4d93-8b47-544f0a4a72a1"
    card_index("1bc42024-52da-4d93-8b47-544f0a4a72a1")
}

#[test]
#[allow(clippy::too_many_lines)]
fn artifact_pkg3_part_01() {
    // Static Orb — {3} — Artifact
    verify_perm(&PermSpec {
        card: static_orb(),
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
    // Courier's Briefcase — {1}{G} — Artifact — Treasure
    verify_perm(&PermSpec {
        card: courier_s_briefcase(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[Color::Green],
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 0,
    });
    // Tumble Magnet — {3} — Artifact
    verify_perm(&PermSpec {
        card: tumble_magnet(),
        primary_mana: &[(ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 3,
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
    // Nutrient Block — {1} — Artifact — Food
    verify_perm(&PermSpec {
        card: nutrient_block(),
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
    // Unstable Obelisk — {3} — Artifact
    verify_perm(&PermSpec {
        card: unstable_obelisk(),
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
    // Spare Supplies — {2} — Artifact
    verify_perm(&PermSpec {
        card: spare_supplies(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
        snow_mana: 0,
        x_value: None,
        colors: &[],
        enters_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        your_creature_pt: None,
        your_creature_kw: None,
        etb_draws: 1,
    });
    // Orzhov Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: orzhov_cluestone(),
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
    // Simic Cluestone — {3} — Artifact
    verify_perm(&PermSpec {
        card: simic_cluestone(),
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
    // Silumgar Monument — {3} — Artifact
    verify_perm(&PermSpec {
        card: silumgar_monument(),
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
    // Implement of Examination — {3} — Artifact
    verify_perm(&PermSpec {
        card: implement_of_examination(),
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
    // Malachite Talisman — {2} — Artifact
    verify_perm(&PermSpec {
        card: malachite_talisman(),
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
    // Fireforger's Puzzleknot — {2} — Artifact
    verify_perm(&PermSpec {
        card: fireforger_s_puzzleknot(),
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
    // Dreamstone Hedron — {6} — Artifact
    verify_perm(&PermSpec {
        card: dreamstone_hedron(),
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
    // Futurist Forge — {1}{U} — Artifact
    verify_perm(&PermSpec {
        card: futurist_forge(),
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
        etb_draws: 1,
    });
    // Potion of Healing — {1}{W} — Artifact
    verify_perm(&PermSpec {
        card: potion_of_healing(),
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
        etb_draws: 1,
    });
    // Imi Statue — {3} — Artifact
    verify_perm(&PermSpec {
        card: imi_statue(),
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
    // Firdoch Core — {3} — Kindred Artifact — Shapeshifter
    verify_perm(&PermSpec {
        card: firdoch_core(),
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
    // Prophetic Prism — {2} — Artifact
    verify_perm(&PermSpec {
        card: prophetic_prism(),
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
    // Stuffed Bear — {2} — Artifact
    verify_perm(&PermSpec {
        card: stuffed_bear(),
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
    // Talisman of Creativity — {2} — Artifact
    verify_perm(&PermSpec {
        card: talisman_of_creativity(),
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
    // Trigon of Mending — {2} — Artifact
    verify_perm(&PermSpec {
        card: trigon_of_mending(),
        primary_mana: &[(ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Vial of Dragonfire — {2} — Artifact
    verify_perm(&PermSpec {
        card: vial_of_dragonfire(),
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
    // Pristine Talisman — {3} — Artifact
    verify_perm(&PermSpec {
        card: pristine_talisman(),
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
    // Astral Cornucopia — {X}{X}{X} — Artifact
    verify_perm(&PermSpec {
        card: astral_cornucopia(),
        primary_mana: &[(ManaColor::Colorless, 6)],
        alt_mana: None,
        total_mana: 6,
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
}
