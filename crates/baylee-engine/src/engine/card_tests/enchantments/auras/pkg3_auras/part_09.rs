//! Package 3 Aura behavioral tests — part 09 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn siege_modification() -> CardIndex {
    // oracle_id = "ac6cf9a7-732d-4ff0-9e88-aae97a2ff169"
    card_index("ac6cf9a7-732d-4ff0-9e88-aae97a2ff169")
}

fn epic_proportions() -> CardIndex {
    // oracle_id = "ad8edfdf-6915-4fc2-9226-3aae95951935"
    card_index("ad8edfdf-6915-4fc2-9226-3aae95951935")
}

fn spreading_seas() -> CardIndex {
    // oracle_id = "af32757c-9d3c-4f6c-a6fa-d5729a440382"
    card_index("af32757c-9d3c-4f6c-a6fa-d5729a440382")
}

fn dryad_s_favor() -> CardIndex {
    // oracle_id = "af740170-af87-487c-abab-a6ccdea82d97"
    card_index("af740170-af87-487c-abab-a6ccdea82d97")
}

fn mark_of_the_vampire() -> CardIndex {
    // oracle_id = "af942d30-a191-4306-846d-6c26755ca3e6"
    card_index("af942d30-a191-4306-846d-6c26755ca3e6")
}

fn dead_weight() -> CardIndex {
    // oracle_id = "b1804304-fac1-4b19-a48d-6ade9407972a"
    card_index("b1804304-fac1-4b19-a48d-6ade9407972a")
}

fn oakenform() -> CardIndex {
    // oracle_id = "b254fefb-11d2-4369-ae5c-0c3bd48ae580"
    card_index("b254fefb-11d2-4369-ae5c-0c3bd48ae580")
}

fn tainted_well() -> CardIndex {
    // oracle_id = "b35b3ea6-ae9c-4128-a47f-069d88ee49c2"
    card_index("b35b3ea6-ae9c-4128-a47f-069d88ee49c2")
}

fn twinblade_blessing() -> CardIndex {
    // oracle_id = "b3e1674f-dba1-4570-9990-e5dc643e237d"
    card_index("b3e1674f-dba1-4570-9990-e5dc643e237d")
}

fn gaea_s_embrace() -> CardIndex {
    // oracle_id = "b44e3f2e-3185-40d5-b50c-d3a1e473120c"
    card_index("b44e3f2e-3185-40d5-b50c-d3a1e473120c")
}

fn security_blockade() -> CardIndex {
    // oracle_id = "b4b397a9-85cb-4bdf-8c2e-a151a02a4208"
    card_index("b4b397a9-85cb-4bdf-8c2e-a151a02a4208")
}

fn leafdrake_roost() -> CardIndex {
    // oracle_id = "b5ff42a1-1ac4-472b-8479-5e3749845305"
    card_index("b5ff42a1-1ac4-472b-8479-5e3749845305")
}

fn tall_as_a_beanstalk() -> CardIndex {
    // oracle_id = "b638c475-778c-4251-8dac-6a18c3394ae5"
    card_index("b638c475-778c-4251-8dac-6a18c3394ae5")
}

fn sheltering_boughs() -> CardIndex {
    // oracle_id = "b6bde4eb-15f0-4e6e-9d25-459a2f75ccf2"
    card_index("b6bde4eb-15f0-4e6e-9d25-459a2f75ccf2")
}

fn stamina() -> CardIndex {
    // oracle_id = "b77217b8-22d1-4dec-a5b4-18fccfcb70e9"
    card_index("b77217b8-22d1-4dec-a5b4-18fccfcb70e9")
}

fn carapace() -> CardIndex {
    // oracle_id = "b893b554-9156-4d79-aca8-6e09c523760b"
    card_index("b893b554-9156-4d79-aca8-6e09c523760b")
}

fn cartouche_of_solidarity() -> CardIndex {
    // oracle_id = "b9176173-c4a1-45ae-8f34-bd6b31c141fe"
    card_index("b9176173-c4a1-45ae-8f34-bd6b31c141fe")
}

fn weight_of_the_underworld() -> CardIndex {
    // oracle_id = "b960d17b-3679-4dc5-9a5d-c8e7fd81fa9a"
    card_index("b960d17b-3679-4dc5-9a5d-c8e7fd81fa9a")
}

fn eternity_snare() -> CardIndex {
    // oracle_id = "b9bd1833-d4f9-46c0-bb94-331c456193fc"
    card_index("b9bd1833-d4f9-46c0-bb94-331c456193fc")
}

fn aquitect_s_defenses() -> CardIndex {
    // oracle_id = "b9dee727-8ad8-42e0-93c6-5ef91d3f7309"
    card_index("b9dee727-8ad8-42e0-93c6-5ef91d3f7309")
}

fn call_to_serve() -> CardIndex {
    // oracle_id = "bb4be6b2-507c-4d7c-bea4-010670dce3da"
    card_index("bb4be6b2-507c-4d7c-bea4-010670dce3da")
}

fn savage_silhouette() -> CardIndex {
    // oracle_id = "bc243497-d95a-44e0-b7e3-c56b4ff7cd69"
    card_index("bc243497-d95a-44e0-b7e3-c56b4ff7cd69")
}

fn serpent_skin() -> CardIndex {
    // oracle_id = "bc606d13-6c01-4f22-a514-1d0a16ec4cf6"
    card_index("bc606d13-6c01-4f22-a514-1d0a16ec4cf6")
}

fn illusory_wrappings() -> CardIndex {
    // oracle_id = "bda363b6-4be1-4415-8b3e-92882df1f630"
    card_index("bda363b6-4be1-4415-8b3e-92882df1f630")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_09() {
    // Siege Modification — {1}{R}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: siege_modification(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Red],
        host_kind: AuraHostKind::CreatureOrVehicle,
        expected_pt_delta: Some((3, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Epic Proportions — {4}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: epic_proportions(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((5, 5)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Spreading Seas — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spreading_seas(),
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
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Dryad's Favor — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dryad_s_favor(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FORESTWALK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mark of the Vampire — {3}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mark_of_the_vampire(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::LIFELINK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Dead Weight — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dead_weight(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Oakenform — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: oakenform(),
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
    // Tainted Well — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tainted_well(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Twinblade Blessing — {1}{W}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: twinblade_blessing(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::DOUBLE_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Gaea's Embrace — {2}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gaea_s_embrace(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Security Blockade — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: security_blockade(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Leafdrake Roost — {3}{G}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: leafdrake_roost(),
        primary_mana: &[
            (ManaColor::Colorless, 3),
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Green, Color::Blue],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Tall as a Beanstalk — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tall_as_a_beanstalk(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::REACH],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sheltering Boughs — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sheltering_boughs(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Stamina — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: stamina(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::VIGILANCE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Carapace — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: carapace(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Cartouche of Solidarity — {W} — Enchantment — Aura Cartouche
    verify_aura(&AuraSpec {
        card: cartouche_of_solidarity(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Weight of the Underworld — {3}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: weight_of_the_underworld(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-3, -2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Eternity Snare — {5}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: eternity_snare(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Aquitect's Defenses — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aquitect_s_defenses(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Call to Serve — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: call_to_serve(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::NonblackCreature,
        expected_pt_delta: Some((1, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Savage Silhouette — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: savage_silhouette(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Serpent Skin — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: serpent_skin(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Illusory Wrappings — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: illusory_wrappings(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: Some((0, 2)),
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
