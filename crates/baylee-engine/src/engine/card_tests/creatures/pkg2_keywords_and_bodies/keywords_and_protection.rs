//! Keyword mechanics and protection behavioral tests for Package 2 creatures.

#[allow(clippy::wildcard_imports)]
use super::*;

fn unsummon() -> CardIndex {
    card_index("837182db-1bf3-4a2c-bd01-1af9d9873561")
}

fn murder() -> CardIndex {
    card_index("938b4e2c-88d9-4637-bc00-e228920c9a78")
}

fn giant_growth() -> CardIndex {
    card_index("5748ebf1-24e3-499d-ab7c-c2cebd462a24")
}

fn ornithopter() -> CardIndex {
    card_index("a3a98bc9-caa0-49b7-951c-fe4e4f54e4ba")
}

fn raging_goblin() -> CardIndex {
    card_index("30997b43-fc13-41d3-8064-1ccc2cb6fd2b")
}

fn melesse_spirit() -> CardIndex {
    card_index("1c4bd85b-03c2-4235-aea1-76c48520adf3")
}

fn shivan_hellkite() -> CardIndex {
    card_index("069be6a0-9545-4fdc-ad40-957fe307db7a")
}

fn archdemon_of_paliano() -> CardIndex {
    card_index("6ca0078f-d6b5-4643-b801-e7a98706f21c")
}

fn serpentine_kavu() -> CardIndex {
    card_index("cf252d71-77e7-438f-b774-28e5ab2f09f6")
}

fn vampire_noble() -> CardIndex {
    card_index("e85d957b-35a8-48a9-8d63-90e3c680e0dd")
}

fn unnatural_speed() -> CardIndex {
    card_index("fa46ced5-b701-4aa3-b4a1-36fb15b80696")
}

#[track_caller]
fn targets_offered_for(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    spell: CardIndex,
    mana: &[(ManaColor, u32)],
) -> Vec<ObjectId> {
    float(e, seat, mana);
    cast_with_floating(e, seat, spell);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected a target choice, got {:?}", e.pending());
    };
    options
}

#[test]
#[allow(clippy::too_many_lines)]
fn vigilance_creatures_attack_without_tapping() {
    let cards = [
        card_index("03e04034-48b5-4f1f-9859-2b46dedf4d8b"),
        card_index("139fb542-89fd-4b9e-87e6-ec925bcca93a"),
        card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"),
        card_index("26e86be4-52e1-47cb-b548-c434482bd3ad"),
        card_index("28cc015f-1f3e-417a-9762-de25109fee5c"),
        card_index("2977aae2-225d-4f3f-8bf7-535f542a7391"),
        card_index("325b4288-fe1b-4e29-ae13-72924b4db3dd"),
        card_index("363f5ed8-a7a0-4cde-b09d-9baa26bad347"),
        card_index("44240b1e-c104-4eb4-8ab9-0d56607ba241"),
        card_index("46f01d1c-7ab6-4a58-97ce-57a4eb99843c"),
        card_index("4db00a5e-7dbe-4387-90e9-c21ec1daa448"),
        card_index("50372c54-359c-4ee6-9e21-1e40ba53652d"),
        card_index("524d81fb-33f5-4e77-ae41-477b91e67be4"),
        card_index("5b38a1e4-a6af-4094-9392-70a87759e20d"),
        card_index("5c1a0a7a-dea4-4b8e-b9c0-8c19942a01ef"),
        card_index("640fa591-ba79-4148-86a6-9de994a9e67c"),
        card_index("69ae765d-03c9-46d9-bdad-8e5936df9616"),
        card_index("69b1b0ec-9db0-48d1-a7b5-71281aca16fe"),
        card_index("6a0419c3-4bfa-46c1-932b-fe10f2fa2ad7"),
        card_index("6db09a0a-6c8e-4829-9b55-00745f0587b5"),
        card_index("6dfcc4bb-7cd5-4b3e-ad79-3c246a3af2a8"),
        card_index("6eb74f6e-1c9d-4caf-a0d8-fa472d435346"),
        card_index("72df0b70-5c9f-44f3-96d5-81c95abc0065"),
        card_index("76f22670-6806-4124-8bbb-822ed33d2bda"),
        card_index("7dd518f9-1ddd-4950-add3-fc21f675f4dc"),
        card_index("7fc904aa-377b-4671-8308-275205f56d81"),
        card_index("8178c546-84f8-4563-a804-6489ec016660"),
        card_index("8516d8ba-b3f9-4c45-8d70-5dab476e478f"),
        card_index("913dbce2-6805-453f-a9c3-64f1cc658627"),
        card_index("9291f31d-6953-4fe1-9a88-b8dd09fde89c"),
        card_index("954836f8-c225-450a-bb51-f2adb6923ef5"),
        card_index("9a74a1dd-c36f-4900-b835-fc39dd725bde"),
        card_index("9b344428-ec35-45b9-9714-9ca58a338f5c"),
        card_index("a16f4b53-c8cf-45b0-89e7-507acd094a64"),
        card_index("a37c9b13-dc45-4b55-a63d-5314abde7aab"),
        card_index("ac230f29-6d3b-4fe1-bcb0-6ffab95a1bd7"),
        card_index("b0747a4f-d041-4750-aef5-a5d101696f65"),
        card_index("b820c29e-2428-4074-a024-9502848484d6"),
        card_index("bd73581c-c4bd-46cb-8004-75aeafa24288"),
        card_index("be37ef9d-955a-417e-bde8-bb9e0bdf3612"),
        card_index("c2bfb230-6949-4d5e-8cac-9d7dafdc5e9a"),
        card_index("c9efec35-823f-4c78-be36-d1a57d8b6f44"),
        card_index("caf1bc69-a8d7-454b-9cc6-2e4d003776a8"),
        card_index("ce31cfc4-a109-4cf5-b8f9-210267a98114"),
        card_index("d014b02f-a8b8-4b97-96e0-f588792069f0"),
        card_index("d8c70f2f-985c-4982-97eb-99da3810e048"),
        card_index("d8c7cd66-d8bf-4a99-add5-1d5651d4e025"),
        card_index("dee0b358-b9da-49b9-a47f-7907cde6d8ee"),
        card_index("e0172cad-8b93-4d1a-b442-5eced3212b17"),
        card_index("eb9a3b52-62ad-467f-b62a-0273ceee48e5"),
        card_index("eb9b0655-3ca3-4d12-8665-54ef0c2c4de2"),
        card_index("f01efb04-ad2b-46fb-852c-dfb7e523e1b7"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(50_000 + n as u64, &[card], &[], &[], &[]);
        let me = obj(&e, P0, card);
        assert!(has_kw(&e, me, KeywordSet::VIGILANCE), "row {n}");
        attack(&mut e, &[me]);
        assert!(
            !tapped(&e, me),
            "row {n}: vigilance creature tapped on attack"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn haste_creatures_can_attack_the_turn_they_are_cast() {
    let rows: &[(CardIndex, &[(ManaColor, u32)])] = &[
        (
            card_index("126844f5-3a95-48cb-80f2-d7e4b0005350"),
            &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("21dd60d5-76c8-469e-8c4e-112e377d2d59"),
            &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        ),
        (
            card_index("24d52cf1-d4f1-4b4c-a50b-87d3ba0aeebb"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("29f22168-21aa-4ca9-ab72-b09396a12c76"),
            &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("363f5ed8-a7a0-4cde-b09d-9baa26bad347"),
            &[
                (ManaColor::Colorless, 4),
                (ManaColor::White, 1),
                (ManaColor::Red, 1),
            ],
        ),
        (
            card_index("422ef4ec-863a-450e-9577-a0707bc16532"),
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        ),
        (
            card_index("47d4da1e-b1a2-4a23-9b96-c85d5d20e3ef"),
            &[(ManaColor::Colorless, 5), (ManaColor::Red, 1)],
        ),
        (
            card_index("4a678c6f-5a93-4a3b-80fd-4c0d0f9040c9"),
            &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        ),
        (
            card_index("5ba6a472-b2f3-4d8e-838c-269aaa5eb2e3"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("5c1a0a7a-dea4-4b8e-b9c0-8c19942a01ef"),
            &[
                (ManaColor::White, 1),
                (ManaColor::Blue, 1),
                (ManaColor::Red, 1),
            ],
        ),
        (
            card_index("5d45fa1f-110c-4bc8-a0fc-ed04556cf639"),
            &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
        ),
        (
            card_index("63d846e2-4bef-4911-9508-273c75a4a040"),
            &[(ManaColor::Black, 1)],
        ),
        (
            card_index("6a15fad7-8171-4bce-bb39-fe3a8175f8fa"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("6db09a0a-6c8e-4829-9b55-00745f0587b5"),
            &[(ManaColor::White, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("6fbaa853-48d4-4a3b-b22d-27415cc266d7"),
            &[(ManaColor::Colorless, 3), (ManaColor::Red, 2)],
        ),
        (
            card_index("73346137-3117-4dc3-b5a4-1aca1da54548"),
            &[(ManaColor::Colorless, 3), (ManaColor::Black, 2)],
        ),
        (
            card_index("7f240b1f-fb7b-49a3-bcaf-6348956d2a38"),
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("8f4c7419-bbbe-4ddc-83ae-fd9ea3e86c3d"),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::Black, 1),
                (ManaColor::Red, 1),
            ],
        ),
        (
            card_index("8f606acf-db16-42cd-8907-dad88c4a1d54"),
            &[
                (ManaColor::Colorless, 1),
                (ManaColor::White, 1),
                (ManaColor::Red, 1),
                (ManaColor::Green, 1),
            ],
        ),
        (
            card_index("95f20067-4471-4bfa-952d-dbbfbb2c2178"),
            &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        ),
        (
            card_index("994cf8bb-c900-448f-b055-ecc2ee6cf9e2"),
            &[
                (ManaColor::Colorless, 2),
                (ManaColor::Blue, 1),
                (ManaColor::Black, 1),
            ],
        ),
        (
            card_index("9f18a580-1c30-4f97-be06-098a763dfda8"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("a361fdd1-7f97-4aed-8e24-ab2a03387f73"),
            &[
                (ManaColor::Colorless, 3),
                (ManaColor::White, 1),
                (ManaColor::Red, 1),
            ],
        ),
        (
            card_index("aa86bce1-9cde-44d8-9455-24502221f31d"),
            &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        ),
        (
            card_index("aad16b50-a866-4871-843f-ec8813ad5dfd"),
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("b909ff27-423f-4e84-887b-7840149a85be"),
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        ),
        (
            card_index("c336eb5b-e185-4d4b-b106-3a641ba9aad1"),
            &[(ManaColor::Colorless, 1), (ManaColor::Red, 2)],
        ),
        (
            card_index("c5c1fb0f-b9cf-48a0-a4ea-e995e222e4b4"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("e050a0a3-f48e-4cba-9cd4-692a92261c9f"),
            &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        ),
        (
            card_index("e900b2b5-4bb2-482d-89ee-b5bbaf246698"),
            &[(ManaColor::Black, 2)],
        ),
        (
            card_index("ee11cecc-4772-4bb2-9af8-a2a99eb037f9"),
            &[(ManaColor::Colorless, 5), (ManaColor::Red, 1)],
        ),
        (
            card_index("fc1256e9-eb92-43b6-abfc-ce19d910a929"),
            &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        ),
    ];
    for (n, &(card, cost)) in rows.iter().enumerate() {
        let mut e = game(
            51_000 + n as u64,
            &[],
            &[],
            &[card, ids::barbary_apes()],
            &[],
        );
        float(&mut e, P0, cost);
        cast_with_floating(&mut e, P0, card);
        settle(&mut e);
        float(
            &mut e,
            P0,
            &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        );
        cast_with_floating(&mut e, P0, ids::barbary_apes());
        settle(&mut e);
        let me = obj(&e, P0, card);
        let apes = obj(&e, P0, ids::barbary_apes());
        assert!(has_kw(&e, me, KeywordSet::HASTE), "row {n}");
        let able = attackers_offered(&mut e);
        assert!(
            able.contains(&me),
            "row {n}: haste creature should be able to attack immediately"
        );
        assert!(
            !able.contains(&apes),
            "row {n}: non-haste creature has summoning sickness"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn flash_creatures_can_be_cast_on_opponents_turn() {
    let rows: &[(CardIndex, &[(ManaColor, u32)])] = &[
        (
            card_index("0f5401c2-0189-4a07-8e7b-a765ccd7c954"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("11369d8b-6443-43c3-a60f-dd920a4e8c08"),
            &[
                (ManaColor::Colorless, 1),
                (ManaColor::Blue, 1),
                (ManaColor::Green, 1),
            ],
        ),
        (
            card_index("14fa0771-cf2b-4563-98b2-ff4ee24bce21"),
            &[(ManaColor::White, 3)],
        ),
        (
            card_index("1a36d2a8-6117-4828-bbe2-bc63212d29d8"),
            &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        ),
        (
            card_index("1d5e32b1-1a2d-4b3d-80f2-f8c688b90587"),
            &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        ),
        (
            card_index("1f2d1de6-42e0-4f45-b43d-ce4980146469"),
            &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        ),
        (
            card_index("257f7c29-8857-429e-a421-9262982804e0"),
            &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        ),
        (
            card_index("263df914-aea1-48dc-b517-7d515307789e"),
            &[(ManaColor::Colorless, 4), (ManaColor::Green, 2)],
        ),
        (
            card_index("2c378f72-917c-49d4-ba68-51ebaf18d976"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        ),
        (
            card_index("379ef166-a29f-4715-8ce4-9d7d567e4e91"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("447b9412-2649-4548-8e9b-eed5e92ad92a"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        ),
        (
            card_index("508fdfb1-87d6-4342-a05a-01158f253896"),
            &[(ManaColor::White, 1)],
        ),
        (
            card_index("656513cf-1bf0-431e-8d3a-2d1ec79d0bec"),
            &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        ),
        (
            card_index("6de183ad-8cdf-4fa6-b17d-5ba1b480849a"),
            &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        ),
        (
            card_index("8003ca2d-0c1d-47c1-bcd1-4609615c7a82"),
            &[(ManaColor::Colorless, 5)],
        ),
        (
            card_index("8957f7c2-040c-4048-9f21-efa7c97682b7"),
            &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        ),
        (
            card_index("8ed27504-f86b-4ce5-9395-7029825176ab"),
            &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        ),
        (
            card_index("91585b64-f10d-47c6-a563-384ceb944710"),
            &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        ),
        (
            card_index("9bc6d4ac-6fca-4574-9cd8-10a5528575b3"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 2)],
        ),
        (
            card_index("9d57cda6-9042-44bd-9785-38de9487ff85"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        ),
        (
            card_index("a93779c5-603e-4a91-aef8-a0cd8fd1b3e9"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("b03c6720-b899-4eba-b28d-54ded03e086d"),
            &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        ),
        (
            card_index("b0747a4f-d041-4750-aef5-a5d101696f65"),
            &[(ManaColor::Colorless, 4), (ManaColor::Green, 2)],
        ),
        (
            card_index("baddf7ee-f6b9-41e3-b930-57129952cc1d"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("bd73581c-c4bd-46cb-8004-75aeafa24288"),
            &[
                (ManaColor::Colorless, 2),
                (ManaColor::White, 1),
                (ManaColor::Blue, 1),
            ],
        ),
        (
            card_index("cb7e076c-a695-4c79-9f20-498370c81380"),
            &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        ),
        (
            card_index("d38a33ae-59d1-4d08-8e14-5fa5a8c3762d"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("d684553a-f225-49af-90eb-399f55ef65df"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        ),
        (
            card_index("d9f43942-4296-417d-a180-5b4f64e4b12d"),
            &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        ),
        (
            card_index("df7580fb-4c37-4c09-8785-e320cb124621"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        ),
        (
            card_index("e5f04e8a-e9ae-4f00-8ada-f63b3454ed30"),
            &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        ),
        (
            card_index("e6d13c0d-23ed-41f5-a726-f18d4bb44c9a"),
            &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        ),
        (
            card_index("fe7a4361-561a-4bab-921e-d01bab19dad3"),
            &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        ),
        (
            card_index("fed32c9c-7458-4143-b0b2-45ca23ba25ee"),
            &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        ),
        (
            card_index("ff12dfee-7140-44bd-a62e-5f8ab98e93c6"),
            &[(ManaColor::Colorless, 4), (ManaColor::Blue, 2)],
        ),
    ];
    for (n, &(card, cost)) in rows.iter().enumerate() {
        let mut e = game(
            52_000 + n as u64,
            &[],
            &[],
            &[card, ids::barbary_apes()],
            &[],
        );
        reach_their_main_phase(&mut e, P1);
        pass_once(&mut e);
        float(&mut e, P0, cost);
        float(
            &mut e,
            P0,
            &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        );
        assert!(
            castable(&e, P0, card),
            "row {n}: flash creature castable on P1 turn"
        );
        assert!(
            !castable(&e, P0, ids::barbary_apes()),
            "row {n}: non-flash creature not castable on P1 turn"
        );
        cast_with_floating(&mut e, P0, card);
        settle(&mut e);
        assert!(
            on_battlefield(&e, P0, card).is_some(),
            "row {n}: flash creature resolved"
        );
    }
}

#[test]
fn first_strike_creatures_kill_one_toughness_attacker_before_taking_damage() {
    let cards = [
        card_index("0e11792b-7fe5-4208-aa0b-e5d09b2b65fe"),
        card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"),
        card_index("2b6aaa8b-b90d-459f-95d4-9e00ee134f8e"),
        card_index("2ef481fa-ff20-426a-af16-fbf761ea5eaf"),
        card_index("3552eea9-aee9-4555-b3cb-8e4fc5b728cc"),
        card_index("3bbfceba-9f3a-4988-a009-13c9472138b1"),
        card_index("46762944-8364-45c4-8008-e693e97ddc46"),
        card_index("4e6c71e8-aeaf-404e-84c2-c79f2a1cbe4b"),
        card_index("53390f91-a159-482f-9f64-f5adc812680b"),
        card_index("56cb7592-ad58-42dc-b568-4b7754bba53d"),
        card_index("5a7ecb5c-9b22-4d2c-b851-db92c1d3c745"),
        card_index("661b23fc-78cd-44f7-ba53-0cb89ccca41f"),
        card_index("67fba605-9cfa-499c-83e0-4fbd023bcfa0"),
        card_index("68a99e50-0e98-480b-ad25-aa2c226d24ca"),
        card_index("76c7a339-015b-4074-8e10-14d52b784f9b"),
        card_index("864c81e4-d612-4939-b4df-4b125cb8cdf2"),
        card_index("8bd301ea-32f0-4c3a-a215-c04b31fbdc7c"),
        card_index("aa7cd6e6-35a6-4c18-9d66-dc2a95206401"),
        card_index("aa86bce1-9cde-44d8-9455-24502221f31d"),
        card_index("be37ef9d-955a-417e-bde8-bb9e0bdf3612"),
        card_index("bf302113-7f2e-4ceb-a26f-1fd6e010be8a"),
        card_index("c4a99095-38cc-4fd3-b3da-c72a6ccceb25"),
        card_index("d8c70f2f-985c-4982-97eb-99da3810e048"),
        card_index("f01efb04-ad2b-46fb-852c-dfb7e523e1b7"),
        card_index("f1ba6053-14e0-46b0-b760-b72e4fa0c5f5"),
        card_index("f1d2f437-d474-4617-aea6-102daa5e5a15"),
        card_index("f8848065-7075-41d8-82f6-67ace390625a"),
        card_index("fb42bade-4bbc-4119-b6cc-71695fe43dd9"),
        card_index("fc1256e9-eb92-43b6-abfc-ce19d910a929"),
        card_index("fd8722e1-9c41-4037-ab7e-47e2aa9858a0"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(53_000 + n as u64, &[ids::barbary_apes()], &[card], &[], &[]);
        let apes = obj(&e, P0, ids::barbary_apes());
        let me = obj(&e, P1, card);
        // Make Barbary Apes a colorless 1/1 ground attacker so any first striker
        // (including Dragon's Eye Sentry with defender and creatures with protection
        // from color) can block it and kill it in the first-strike damage step.
        let state = e.dev_state_mut(P0).unwrap();
        state.object_mut(apes).unwrap().base_mut().power = Some(1);
        state.object_mut(apes).unwrap().base_mut().toughness = Some(1);
        state.object_mut(apes).unwrap().base_mut().colors = ColorSet::EMPTY;
        state.refresh_characteristics();
        assert_eq!(pt(&e, apes), (1, 1));
        assert!(has_kw(&e, me, KeywordSet::FIRST_STRIKE), "row {n}");
        attack(&mut e, &[apes]);
        block(&mut e, &[(me, apes)]);
        through_combat(&mut e);
        assert!(
            in_graveyard(&e, P0, ids::barbary_apes()).is_some(),
            "row {n}: attacker died"
        );
        assert_eq!(damage_on(&e, me), 0, "row {n}: first striker took 0 damage");
    }
}

#[test]
fn double_strike_creatures_deal_double_damage_and_win_first_strike() {
    let rows = [
        (card_index("142d2063-d692-42a6-b828-42d998914589"), 1i32),
        (card_index("1fa3839a-127e-4740-bd2e-97b18fdddedc"), 2i32),
        (card_index("2f810936-2ba6-4c2b-84a0-ff4c1deb026b"), 1i32),
        (card_index("377e4dad-866b-4e02-8c10-072bf7f000be"), 1i32),
        (card_index("44240b1e-c104-4eb4-8ab9-0d56607ba241"), 1i32),
        (card_index("679841ad-f3ac-48e2-ba11-2cf6d78642b4"), 1i32),
        (card_index("7b2bbd87-f288-44bd-a15c-1432a3e90cb3"), 1i32),
        (card_index("8f2252f6-69b9-4c56-b078-803db301a874"), 2i32),
        (card_index("a2f867cd-410d-4524-8660-38ad142f6b1f"), 1i32),
        (card_index("b5868383-a0fb-40fc-b718-f1b5441c5ce3"), 2i32),
        (card_index("bc940eda-efa8-4227-9b56-0c6fbf1398d2"), 2i32),
        (card_index("d014b02f-a8b8-4b97-96e0-f588792069f0"), 1i32),
        (card_index("fe69d9bd-2a60-4b33-a0d8-1ca18c2b6705"), 2i32),
    ];
    for (n, (card, power)) in rows.into_iter().enumerate() {
        let mut e = game(54_000 + n as u64, &[card], &[], &[], &[]);
        let me = obj(&e, P0, card);
        assert!(has_kw(&e, me, KeywordSet::DOUBLE_STRIKE), "row {n}");
        attack(&mut e, &[me]);
        block(&mut e, &[]);
        through_combat(&mut e);
        assert_eq!(
            life(&e, P1),
            20 - 2 * power,
            "row {n}: double strike unblocked damage"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn deathtouch_creatures_destroy_high_toughness_attacker_when_blocking() {
    let cards = [
        card_index("0c6a6587-2b3f-40d1-a5f2-06128da0d52e"),
        card_index("0f8e0b2c-f1a5-40ad-b8d8-7c06a012e909"),
        card_index("0fabb6e4-35cf-4e56-a27c-ff627d8e9b7a"),
        card_index("11369d8b-6443-43c3-a60f-dd920a4e8c08"),
        card_index("1551cd95-1e68-404f-be1d-cc9c7f447210"),
        card_index("1bf06342-0d76-433b-9912-8af03b7e3bc3"),
        card_index("315ddc7a-c39c-43d9-b134-23acbd865e3e"),
        card_index("3b61b6da-44ed-4766-8134-fca37afd3f2a"),
        card_index("4704ce78-b287-4038-abdd-ddb20116874d"),
        card_index("557485d9-7537-4297-a589-c29d0f1ecd98"),
        card_index("5a938371-8428-48e3-85ed-6a84a39b29c6"),
        card_index("61c3b999-4f5d-4018-a9ee-ef894fd1ef6c"),
        card_index("6b941291-1802-4ced-9869-39fa43825550"),
        card_index("6e42e7dc-acd9-4125-a90d-75fd8da204b3"),
        card_index("6edacc06-9523-4ccf-a4c2-2ac4af386b8a"),
        card_index("7d6f6ee4-77c2-4986-91c7-7dee962fc1a7"),
        card_index("8481d91e-0242-4edc-8f4a-ffa47cf2e097"),
        card_index("8957f7c2-040c-4048-9f21-efa7c97682b7"),
        card_index("913dbce2-6805-453f-a9c3-64f1cc658627"),
        card_index("9e187d43-ae75-4589-81fc-476ae95ba5b9"),
        card_index("a48d2be2-395a-4377-a2ad-51dbbc5b4e17"),
        card_index("a7a6cdfd-10ac-49d0-a0a6-8ad37e326511"),
        card_index("a93f0735-9854-487d-b646-d55e2bee00c0"),
        card_index("aed587bf-c6e5-4850-baeb-2715cb27bf4c"),
        card_index("b5a26a1b-d2be-406e-a6a9-1993045313b5"),
        card_index("b5ba87f5-3589-4a8b-ad65-2ae2523e98e8"),
        card_index("b94591b8-2e99-4983-8f36-9b52207bfd2e"),
        card_index("d38cbb37-0ddd-4768-bb05-f35b38730e44"),
        card_index("d4870995-f434-4985-a921-1dc9556cee41"),
        card_index("d6ee6cc1-902d-4f56-afa5-6fa4813bfbbc"),
        card_index("debbbdd8-c8e2-4384-bcac-dbdfb18b54fa"),
        card_index("e2714e1f-c883-41e0-bc2f-60e15462ef98"),
        card_index("ea4d30c7-7c39-46d5-9b71-a7ebc3e219a1"),
        card_index("eb9a3b52-62ad-467f-b62a-0273ceee48e5"),
        card_index("ee8f16ba-cbf0-4286-b26c-d78dd35bee17"),
        card_index("efd70846-2226-43c6-80fe-6acdd15d4246"),
        card_index("f1929f1f-6aa3-4308-bbff-cab03883ac68"),
        card_index("f394d2bf-89aa-44c1-b473-8c7a8874a9a3"),
        card_index("fbadf7a3-0d54-40fe-a763-f641dd448e56"),
        card_index("feb244f8-bcb1-44cf-9940-2719221a7309"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(55_000 + n as u64, &[ids::barbary_apes()], &[card], &[], &[]);
        let apes = obj(&e, P0, ids::barbary_apes());
        let me = obj(&e, P1, card);
        // Make Barbary Apes a 0/10 attacker so only deathtouch destroys it.
        let state = e.dev_state_mut(P0).unwrap();
        state.object_mut(apes).unwrap().base_mut().power = Some(0);
        state.object_mut(apes).unwrap().base_mut().toughness = Some(10);
        state.refresh_characteristics();
        assert!(has_kw(&e, me, KeywordSet::DEATHTOUCH), "row {n}");
        attack(&mut e, &[apes]);
        block(&mut e, &[(me, apes)]);
        through_combat(&mut e);
        assert!(
            in_graveyard(&e, P0, ids::barbary_apes()).is_some(),
            "row {n}: deathtouch blocker destroyed 0/10 attacker"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn lifelink_creatures_gain_life_equal_to_combat_damage_dealt() {
    let rows = [
        (card_index("065b8e5e-874e-4de3-9f12-41e4b1604561"), 4i32),
        (card_index("0b3b2fb1-d5fe-4d14-985a-85f13cd60d9c"), 3i32),
        (card_index("0c6a6587-2b3f-40d1-a5f2-06128da0d52e"), 2i32),
        (card_index("0e11792b-7fe5-4208-aa0b-e5d09b2b65fe"), 5i32),
        (card_index("1c385880-3eea-45c8-8e89-330ad8ad92b1"), 3i32),
        (card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"), 6i32),
        (card_index("23dd6d4a-d933-4328-8d09-75f7908ae347"), 3i32),
        (card_index("28a52ba1-95da-44e1-8ac5-0dc23c902394"), 1i32),
        (card_index("2c0b8cc3-b1c1-498d-b34c-f317c2c9468f"), 1i32),
        (card_index("31009c45-afba-45f4-a6ca-9dfdb9990e72"), 2i32),
        (card_index("316f7091-0c75-4510-a89a-6b58bb7f0627"), 2i32),
        (card_index("38c588ae-7254-4fae-aa9a-03e2a5524492"), 2i32),
        (card_index("4e6c71e8-aeaf-404e-84c2-c79f2a1cbe4b"), 2i32),
        (card_index("5101ff9e-4680-4d38-80a5-b4c21bc67356"), 2i32),
        (card_index("5a938371-8428-48e3-85ed-6a84a39b29c6"), 1i32),
        (card_index("5c05c350-1b11-4dda-9de6-01c7e0f07ffc"), 3i32),
        (card_index("5d95db27-53bb-4a20-9fb8-24998b03e767"), 3i32),
        (card_index("5e95618c-4f58-4ae9-9e5e-deffd5e210a4"), 2i32),
        (card_index("63d846e2-4bef-4911-9508-273c75a4a040"), 1i32),
        (card_index("6774c0ce-103e-471b-8de0-f325805674ca"), 2i32),
        (card_index("67f59250-d46a-4637-9493-165d23278817"), 1i32),
        (card_index("69b1b0ec-9db0-48d1-a7b5-71281aca16fe"), 2i32),
        (card_index("725dc451-cae4-4345-9741-6c58e5a1a1dc"), 3i32),
        (card_index("84487586-4f71-457a-b983-4f1f7a71deef"), 2i32),
        (card_index("894979ba-e4c7-4914-b0d3-133cbaed38c2"), 1i32),
        (card_index("8f4c7419-bbbe-4ddc-83ae-fd9ea3e86c3d"), 3i32),
        (card_index("8f606acf-db16-42cd-8907-dad88c4a1d54"), 4i32),
        (card_index("94688e62-f39f-41fd-9309-f16b715ef724"), 3i32),
        (card_index("a361fdd1-7f97-4aed-8e24-ab2a03387f73"), 4i32),
        (card_index("a939f01b-afce-4d92-a897-1a6dc3d788c6"), 1i32),
        (card_index("ad172d4e-febf-4844-9544-861d03e27ab5"), 2i32),
        (card_index("be37ef9d-955a-417e-bde8-bb9e0bdf3612"), 3i32),
        (card_index("c2bfb230-6949-4d5e-8cac-9d7dafdc5e9a"), 2i32),
        (card_index("c650a7bc-e350-44a0-a698-d4a233d66156"), 2i32),
        (card_index("cd53f7cf-e4da-4e93-8550-60556300b56c"), 1i32),
        (card_index("d218b9ad-3a6f-4bda-90c6-3f869c6c4e6d"), 1i32),
        (card_index("d56602f3-64df-4c68-a4a3-95c37c021e72"), 3i32),
        (card_index("ebd33f4c-2cc8-46e3-b367-ce5b37c2719b"), 1i32),
        (card_index("f18a1a99-8363-4edf-9807-2e23fa007ce8"), 1i32),
        (card_index("f1d2f437-d474-4617-aea6-102daa5e5a15"), 2i32),
        (card_index("f688863f-b8eb-4143-8ea5-0b234cc91bd0"), 3i32),
        (card_index("faf2f368-a7e7-4427-bc03-929f294b1e03"), 2i32),
        (card_index("fcedc288-5e27-45a2-a07b-06c7aa7a9230"), 3i32),
        (card_index("fd7abcee-d4d3-4cf6-8385-c6ad7ac218e4"), 2i32),
        (card_index("feb244f8-bcb1-44cf-9940-2719221a7309"), 2i32),
    ];
    for (n, (card, power)) in rows.into_iter().enumerate() {
        let mut e = game(56_000 + n as u64, &[card], &[], &[], &[]);
        let me = obj(&e, P0, card);
        assert!(has_kw(&e, me, KeywordSet::LIFELINK), "row {n}");
        attack(&mut e, &[me]);
        block(&mut e, &[]);
        through_combat(&mut e);
        assert_eq!(
            life(&e, P0),
            20 + power,
            "row {n}: controller gained life from lifelink"
        );
        assert_eq!(
            life(&e, P1),
            20 - power,
            "row {n}: defending player lost life"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn trample_creatures_assign_excess_combat_damage_to_defending_player() {
    let rows = [
        (card_index("04be1e69-2d5c-4931-8906-a6807b5b08ca"), 1i32),
        (card_index("08c7db90-c0cf-4482-b7ee-bb033e5996d2"), 6i32),
        (card_index("1f14d336-71ef-4e3f-b8e0-42585af8db40"), 7i32),
        (card_index("21dd60d5-76c8-469e-8c4e-112e377d2d59"), 5i32),
        (card_index("26fbfab5-f479-49f2-b36b-a3ebf964741f"), 8i32),
        (card_index("325b4288-fe1b-4e29-ae13-72924b4db3dd"), 6i32),
        (card_index("32fc8fa7-7e0a-4d4e-85cf-2f98b3fc6ecf"), 3i32),
        (card_index("34ff281a-f294-4351-9665-863a5b2e882b"), 4i32),
        (card_index("384611fb-aca2-4e2d-9125-44ea034de08b"), 4i32),
        (card_index("44240b1e-c104-4eb4-8ab9-0d56607ba241"), 2i32),
        (card_index("4fb9c040-c684-4ba8-970e-58313af5584a"), 4i32),
        (card_index("53390f91-a159-482f-9f64-f5adc812680b"), 6i32),
        (card_index("533c4956-87f8-4bae-8924-166fc2e9ec1f"), 4i32),
        (card_index("55187899-c445-48e9-80b9-235eaf42f803"), 3i32),
        (card_index("5d95db27-53bb-4a20-9fb8-24998b03e767"), 3i32),
        (card_index("60206d4e-4301-4d7a-a68e-46a2ad9fc407"), 4i32),
        (card_index("60ff591d-170d-410a-9710-510e811f9827"), 4i32),
        (card_index("62c4d349-a18e-40a2-8cc3-244c219dec7f"), 5i32),
        (card_index("6adff11d-5126-4a4b-b182-e2700d406995"), 3i32),
        (card_index("6ea0d274-564b-4785-801d-b3272a7b6bfa"), 2i32),
        (card_index("6fbaa853-48d4-4a3b-b22d-27415cc266d7"), 5i32),
        (card_index("73346137-3117-4dc3-b5a4-1aca1da54548"), 5i32),
        (card_index("782e8499-0714-4cee-869e-8ebd7dad0242"), 3i32),
        (card_index("7abbbe18-a2e5-47ce-9c65-8008e5153a4d"), 4i32),
        (card_index("7fe5cad0-8db8-4cb3-9d61-bda84477654d"), 5i32),
        (card_index("8049dbff-6b57-454a-a2e6-7e2ae39bb606"), 3i32),
        (card_index("8e4dc1fc-e412-4aa3-8793-5c4abc598521"), 4i32),
        (card_index("8f606acf-db16-42cd-8907-dad88c4a1d54"), 4i32),
        (card_index("9a74a1dd-c36f-4900-b835-fc39dd725bde"), 5i32),
        (card_index("9d448c85-9c93-4e9e-8aa0-0035fc94112e"), 5i32),
        (card_index("b0747a4f-d041-4750-aef5-a5d101696f65"), 6i32),
        (card_index("be1c2607-091a-41e7-ae4c-73c7e5689739"), 4i32),
        (card_index("ce82aa79-bb50-47bc-84f5-eae30249c7a5"), 5i32),
        (card_index("d5142292-446a-413b-85a0-be8a59cec1f8"), 3i32),
        (card_index("d52397bf-e3fb-4894-996d-7c1cc8d92603"), 3i32),
        (card_index("d8c7cd66-d8bf-4a99-add5-1d5651d4e025"), 5i32),
        (card_index("dbdd0963-b395-4cb8-ad9a-ed85a3e9f2e5"), 7i32),
        (card_index("e4c96980-31ed-494e-89db-c0a0f1fd23fd"), 6i32),
        (card_index("e873b9c2-1346-4f36-aa48-da58112792db"), 4i32),
        (card_index("f5070528-cc4c-4910-8047-1ed17efd6496"), 6i32),
        (card_index("f5f2fa96-8e87-44bc-8c82-e217706d3cab"), 3i32),
        (card_index("f66fca2b-6c44-4079-b29e-4d18ef030a0c"), 5i32),
    ];
    for (n, (card, power)) in rows.into_iter().enumerate() {
        // Include Jacques le Vert (+0/+2 to green creatures) so 8/0 Force of Savagery stays on board.
        let mut e = game(
            57_000 + n as u64,
            &[card, ids::jacques_le_vert()],
            &[ids::vampire_bats()],
            &[],
            &[],
        );
        let me = obj(&e, P0, card);
        let bats = obj(&e, P1, ids::vampire_bats());
        assert!(has_kw(&e, me, KeywordSet::TRAMPLE), "row {n}");
        attack(&mut e, &[me]);
        block(&mut e, &[(bats, me)]);
        through_combat(&mut e);
        assert!(
            in_graveyard(&e, P1, ids::vampire_bats()).is_some(),
            "row {n}: chump blocker died"
        );
        assert_eq!(
            life(&e, P1),
            20 - (power - 1),
            "row {n}: excess trample damage dealt to P1"
        );
    }
}

#[test]
fn hexproof_creatures_repel_opposing_targets_but_allow_controllers_targets() {
    let cards = [
        card_index("011b2243-b8be-471f-9d8f-824a40e599df"),
        card_index("33e87ab4-f680-4463-a48d-289fed7e196e"),
        card_index("37ad9318-6690-4a8d-bcea-9fcbca58769d"),
        card_index("385c1208-bfea-44e2-b236-4f38bc90db9f"),
        card_index("395922cc-d524-4241-a7d8-9c7959f0b9db"),
        card_index("39daa265-8d02-4449-9c45-ae8ec2b13e5b"),
        card_index("4317f4f1-d339-4940-b46e-659641035595"),
        card_index("43c449d1-51a9-45e0-8102-823a6811ff54"),
        card_index("44350127-f6b6-458f-8f2f-e57a78d83303"),
        card_index("457e4a49-c8fe-4e1d-a4b4-7a880a47f58d"),
        card_index("6175197f-cf3c-41df-9d24-b4fffa09631a"),
        card_index("86f725f6-b48b-4dcc-9a02-e258a226e31c"),
        card_index("abb39d0e-bf11-4982-b06a-ca7be16bce2c"),
        card_index("b94591b8-2e99-4983-8f36-9b52207bfd2e"),
        card_index("ccb64a70-cbdd-4991-a28d-b137032a4f4d"),
        card_index("efd70846-2226-43c6-80fe-6acdd15d4246"),
        card_index("f5f2fa96-8e87-44bc-8c82-e217706d3cab"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            58_000 + n as u64,
            &[],
            &[card, ids::barbary_apes()],
            &[murder()],
            &[giant_growth()],
        );
        let hex = obj(&e, P1, card);
        let apes = obj(&e, P1, ids::barbary_apes());
        assert!(has_kw(&e, hex, KeywordSet::HEXPROOF), "row {n}");
        let opp_opts = targets_offered_for(
            &mut e,
            P0,
            murder(),
            &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
        );
        assert!(
            opp_opts.contains(&apes),
            "row {n}: unprotected creature is targetable"
        );
        assert!(
            !opp_opts.contains(&hex),
            "row {n}: opponent cannot target hexproof creature"
        );
        aim(&mut e, &[apes], &[]);
        settle(&mut e);

        pass_once(&mut e);
        let self_opts = targets_offered_for(&mut e, P1, giant_growth(), &[(ManaColor::Green, 1)]);
        assert!(
            self_opts.contains(&hex),
            "row {n}: controller can target own hexproof creature"
        );
    }
}

#[test]
fn shroud_creatures_cannot_be_targeted_by_either_player() {
    let cards = [
        card_index("2b6aaa8b-b90d-459f-95d4-9e00ee134f8e"),
        card_index("49cade2f-92ab-48c8-aed3-d52ab3263506"),
        card_index("9cc092f9-e8f0-40ea-847d-813b70f889db"),
        card_index("dbdd0963-b395-4cb8-ad9a-ed85a3e9f2e5"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            59_000 + n as u64,
            &[ids::barbary_apes()],
            &[card],
            &[giant_growth()],
            &[giant_growth()],
        );
        let shrouded = obj(&e, P1, card);
        let apes = obj(&e, P0, ids::barbary_apes());
        assert!(has_kw(&e, shrouded, KeywordSet::SHROUD), "row {n}");
        let p0_opts = targets_offered_for(&mut e, P0, giant_growth(), &[(ManaColor::Green, 1)]);
        assert!(
            !p0_opts.contains(&shrouded),
            "row {n}: opponent cannot target shroud"
        );
        assert!(p0_opts.contains(&apes), "row {n}");
        aim(&mut e, &[apes], &[]);
        settle(&mut e);

        pass_once(&mut e);
        let p1_opts = targets_offered_for(&mut e, P1, giant_growth(), &[(ManaColor::Green, 1)]);
        assert!(
            !p1_opts.contains(&shrouded),
            "row {n}: controller cannot target shroud either"
        );
    }
}

#[test]
fn indestructible_creatures_survive_destroy_effects() {
    let cards = [card_index("f5f2fa96-8e87-44bc-8c82-e217706d3cab")];
    for (n, card) in cards.into_iter().enumerate() {
        // Controller (P0) targets own indestructible creature with Murder so hexproof does not block targeting.
        let mut e = game(60_000 + n as u64, &[card], &[], &[murder()], &[]);
        let ind = obj(&e, P0, card);
        assert!(has_kw(&e, ind, KeywordSet::INDESTRUCTIBLE), "row {n}");
        float(
            &mut e,
            P0,
            &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
        );
        cast_at(&mut e, P0, murder(), &[ind], &[]);
        assert!(
            on_battlefield(&e, P0, card).is_some(),
            "row {n}: survived Murder"
        );
    }
}

#[test]
fn changeling_creatures_carry_every_creature_subtype() {
    let cards = [
        card_index("2977aae2-225d-4f3f-8bf7-535f542a7391"),
        card_index("338f0ab6-f744-4e5a-a358-05e71b93758d"),
        card_index("384611fb-aca2-4e2d-9125-44ea034de08b"),
        card_index("60206d4e-4301-4d7a-a68e-46a2ad9fc407"),
        card_index("767c1f97-03de-474d-9112-e7521ba3b3f3"),
        card_index("930035c6-cde7-40ac-98ef-0840812037db"),
        card_index("a2b8185a-9ff2-4d65-9bdd-b060b7ce69bb"),
        card_index("f1929f1f-6aa3-4308-bbff-cab03883ac68"),
        card_index("faf2f368-a7e7-4427-bc03-929f294b1e03"),
        card_index("fe7a4361-561a-4bab-921e-d01bab19dad3"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let e = game(61_000 + n as u64, &[card], &[], &[], &[]);
        let id = obj(&e, P0, card);
        assert!(has_kw(&e, id, KeywordSet::CHANGELING), "row {n}");
        let subs = &e.state().object(id).unwrap().characteristics().subtypes;
        for required in [
            subtypes::creature::GOBLIN,
            subtypes::creature::ELF,
            subtypes::creature::SLIVER,
            subtypes::creature::ZOMBIE,
            subtypes::creature::DRAGON,
        ] {
            assert!(
                subs.contains(required),
                "row {n}: changeling missing subtype {required:?}"
            );
        }
    }
}

#[test]
fn devoid_creatures_are_colorless_in_every_zone() {
    let cards = [card_index("55187899-c445-48e9-80b9-235eaf42f803")];
    for (n, card) in cards.into_iter().enumerate() {
        let e = game(62_000 + n as u64, &[card], &[], &[card], &[]);
        let bf_id = obj(&e, P0, card);
        let hand_id = in_hand(&e, P0, card).unwrap();
        assert_eq!(
            colors_of(&e, bf_id),
            ColorSet::EMPTY,
            "row {n}: colorless on battlefield"
        );
        assert_eq!(
            colors_of(&e, hand_id),
            ColorSet::EMPTY,
            "row {n}: colorless in hand"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn protection_from_color_prevents_targeting_by_spells_of_that_color() {
    let rows = [
        (
            card_index("01d70534-7c26-4a54-9eca-d0ebbefc2159"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("02eef375-5166-4b6b-bb17-b38074a6ba53"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("04576b4d-2977-4cdf-b4a5-b0b47995e962"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("1c4bd85b-03c2-4235-aea1-76c48520adf3"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("333555d6-3fcc-4021-ab60-afc36d723877"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("33435856-ea24-4d10-84d5-6872b35b63a3"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("3552eea9-aee9-4555-b3cb-8e4fc5b728cc"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("372e2d6e-427c-4152-b936-27dcf536ca84"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("3cc99034-6eaf-4e45-9bff-57b74847343e"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("443f8dd1-333d-42c4-a286-302a9496209a"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("4a1db129-cc7c-410a-a41e-d633e30cfdad"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("535007c5-03a8-495f-a132-37960f179f96"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("58e4f299-bb32-4727-8498-a364166adc38"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("5eb29367-4ec3-4bd2-887e-13eeeecae8fa"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("5fed5377-07cf-4ad9-86fd-1efe1f182088"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("656513cf-1bf0-431e-8d3a-2d1ec79d0bec"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("67fba605-9cfa-499c-83e0-4fbd023bcfa0"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("6b26951f-57be-4524-ae0e-a102e84e81ef"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("7220769a-9d63-4211-9365-9feaf4d9a518"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("7338f964-b7d8-4491-b933-7d8b7d1b56ed"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("786afd52-0eae-4453-b1bd-cffc21699849"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("7a7f37a3-9e99-4d3d-b705-b074fb3e9b3e"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("7d086ae5-738c-4685-80ae-66bd9379d8f7"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("857524d5-9b5c-42ac-a6cf-2d9bfebd7015"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("871549e6-24c0-4fbd-8b71-ba8639b0ee47"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("8cdda36f-5e82-4c2f-8eaa-75b9bbc93a02"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("904b2caf-e083-4917-8841-95f48d90dc0c"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("9a484eb3-ae4a-465b-9dbc-f7a42a036e07"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("9a642db9-c337-481c-b260-11e01f64e68e"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("a9cbf2ed-9896-41b5-b17f-e2fcb3cb82fa"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("aa17782b-d8e7-423b-9f03-01ccb600f69a"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("aa7cd6e6-35a6-4c18-9d66-dc2a95206401"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("b318d94e-314f-4df8-a178-b5ddd756e1c4"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("b5d5b9c9-accc-4dbe-aaf4-c806f3e1c61c"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("b9be76a3-e14f-48dd-8824-17d5708ac7e0"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("c04dc6fe-06fc-4e5b-87da-6b8b262a214f"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("c208b959-d0e4-4a9a-8255-2c7cc7596767"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("cbe5c74b-1006-4c63-8b46-e06b61e6fe02"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("ce82aa79-bb50-47bc-84f5-eae30249c7a5"),
            unsummon(),
            ManaColor::Blue,
            0u32,
        ),
        (
            card_index("ce82aa79-bb50-47bc-84f5-eae30249c7a5"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("d87ab916-5d49-4840-8d5b-805af800a83c"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("e5b11e43-fc7c-4cea-a44a-8af3f124e29e"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
        (
            card_index("e5f04e8a-e9ae-4f00-8ada-f63b3454ed30"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("e80be353-8b19-42d2-9141-5353a8409c8e"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("f8c9cb9f-3a37-4004-9cde-f51ca5579e1a"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("fb3d438e-5e04-45a0-8d16-db63f0faff75"),
            path_to_exile(),
            ManaColor::White,
            0u32,
        ),
        (
            card_index("fd8722e1-9c41-4037-ab7e-47e2aa9858a0"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("fd8722e1-9c41-4037-ab7e-47e2aa9858a0"),
            lightning_bolt(),
            ManaColor::Red,
            0u32,
        ),
        (
            card_index("fe69d9bd-2a60-4b33-a0d8-1ca18c2b6705"),
            murder(),
            ManaColor::Black,
            1u32,
        ),
        (
            card_index("fe69d9bd-2a60-4b33-a0d8-1ca18c2b6705"),
            giant_growth(),
            ManaColor::Green,
            0u32,
        ),
    ];
    for (n, (card, spell, color, generic)) in rows.into_iter().enumerate() {
        let mut e = game(
            63_000 + n as u64,
            &[],
            &[card, ids::barbary_apes()],
            &[spell],
            &[],
        );
        let protected = obj(&e, P1, card);
        let apes = obj(&e, P1, ids::barbary_apes());
        let mana = if generic > 0 {
            vec![(ManaColor::Colorless, generic), (color, 2)]
        } else {
            vec![(color, 1)]
        };
        let opts = targets_offered_for(&mut e, P0, spell, &mana);
        assert!(
            opts.contains(&apes),
            "row {n}: unprotected creature is targetable"
        );
        assert!(
            !opts.contains(&protected),
            "row {n}: protected from {color:?}"
        );
    }
}

#[test]
fn protection_from_artifacts_prevents_artifact_creatures_from_blocking() {
    let cards = [
        card_index("475412c9-8960-4085-bd8f-5f8a7c7485b9"),
        card_index("6e06b5cc-444c-49c9-b08a-81ef8d97c9ab"),
        card_index("939203ec-3de4-44d8-932e-63a32e5f2d47"),
        card_index("cca29a9b-794f-4712-8e6c-36ce3da9cb8b"),
        card_index("d83d784e-e95b-48a1-9b4a-362f4fccc529"),
        card_index("de6854d1-bdb0-4b9a-b442-08d4d90a538d"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            64_000 + n as u64,
            &[card],
            &[ornithopter(), ids::vampire_bats()],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        let offer = attack(&mut e, &[attacker]);
        let thopter = obj(&e, P1, ornithopter());
        let bats = obj(&e, P1, ids::vampire_bats());
        assert!(
            !may_block(&offer, thopter, attacker),
            "row {n}: artifact creature blocked"
        );
        assert!(
            may_block(&offer, bats, attacker),
            "row {n}: non-artifact flier may block"
        );
    }
}

#[test]
fn protection_from_enchantments_prevents_aura_targeting() {
    let cards = [card_index("08d02fdb-4b3f-4f55-b5fc-6bb4d2a14b1f")];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            64_500 + n as u64,
            &[],
            &[card, ids::barbary_apes()],
            &[ids::seeker()],
            &[],
        );
        let protected = obj(&e, P1, card);
        let apes = obj(&e, P1, ids::barbary_apes());
        let opts = targets_offered_for(
            &mut e,
            P0,
            ids::seeker(),
            &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
        );
        assert!(opts.contains(&apes), "row {n}");
        assert!(
            !opts.contains(&protected),
            "row {n}: aura cannot target protection from enchantments"
        );
    }
}

#[test]
fn protection_from_creatures_makes_attacker_unblockable_by_creatures() {
    let cards = [
        card_index("a07406b7-faa1-4ed3-ab84-314c40f3f7f1"),
        card_index("b9b9edb2-62f5-499c-b2fc-359dc673c4b4"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            65_000 + n as u64,
            &[card],
            &[ids::vampire_bats(), ids::barbary_apes()],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        let offer = attack(&mut e, &[attacker]);
        assert!(
            !may_block(&offer, obj(&e, P1, ids::vampire_bats()), attacker),
            "row {n}"
        );
        assert!(
            !may_block(&offer, obj(&e, P1, ids::barbary_apes()), attacker),
            "row {n}"
        );
    }
}

#[test]
fn protection_from_lands_prevents_land_creatures_from_blocking() {
    let cards = [card_index("9c724912-5afd-4813-affb-4dbda64fd4ee")];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            65_500 + n as u64,
            &[card],
            &[ids::barbary_apes(), ids::vampire_bats()],
            &[],
            &[],
        );
        let drake = obj(&e, P0, card);
        let land_flier = obj(&e, P1, ids::barbary_apes());
        let bats = obj(&e, P1, ids::vampire_bats());
        // Make Barbary Apes a flying Land Creature on P1's board.
        let state = e.dev_state_mut(P1).unwrap();
        state.object_mut(land_flier).unwrap().base_mut().types =
            TypeSet::LAND.union(TypeSet::CREATURE);
        state.object_mut(land_flier).unwrap().base_mut().keywords = KeywordSet::FLYING;
        state.refresh_characteristics();

        let offer = attack(&mut e, &[drake]);
        assert!(
            !may_block(&offer, land_flier, drake),
            "row {n}: land creature cannot block Horizon Drake"
        );
        assert!(
            may_block(&offer, bats, drake),
            "row {n}: non-land flyer can block Horizon Drake"
        );
    }
}

#[test]
fn protection_from_creature_subtype_prevents_blocking_by_that_subtype() {
    let rows = [
        (
            card_index("0e11792b-7fe5-4208-aa0b-e5d09b2b65fe"),
            archdemon_of_paliano(),
            ids::vampire_bats(),
        ),
        (
            card_index("0e11792b-7fe5-4208-aa0b-e5d09b2b65fe"),
            shivan_hellkite(),
            ids::vampire_bats(),
        ),
        (
            card_index("64368768-77b9-4f78-adb7-951ca71dfb28"),
            raging_goblin(),
            ids::barbary_apes(),
        ),
        (
            card_index("79dcc432-e8f5-4f2b-bff8-5ae964714666"),
            serpentine_kavu(),
            ids::barbary_apes(),
        ),
        (
            card_index("87e44296-6c79-4065-aa54-33e508293a79"),
            melesse_spirit(),
            ids::barbary_apes(),
        ),
        (
            card_index("d9410a82-1b7f-4f3a-a0fd-53099d7594ec"),
            shivan_hellkite(),
            ids::vampire_bats(),
        ),
        (
            card_index("da404c9c-3fb0-418c-947f-f42d6aff6899"),
            llanowar_elves(),
            ids::barbary_apes(),
        ),
        (
            card_index("e8e2e1e9-13a6-4cfd-b9ed-2080e984844d"),
            vampire_noble(),
            ids::barbary_apes(),
        ),
    ];
    for (n, (card, blocked_sub, allowed_sub)) in rows.into_iter().enumerate() {
        let mut e = game(
            66_000 + n as u64,
            &[card],
            &[blocked_sub, allowed_sub],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        let offer = attack(&mut e, &[attacker]);
        let bad = obj(&e, P1, blocked_sub);
        let good = obj(&e, P1, allowed_sub);
        assert!(
            !may_block(&offer, bad, attacker),
            "row {n}: protected subtype blocked"
        );
        assert!(
            may_block(&offer, good, attacker),
            "row {n}: non-protected creature may block"
        );
    }
}

#[test]
fn protection_from_zombies_on_defender_prevents_zombie_combat_damage() {
    let cards = [card_index("9e2f854b-17f7-4d52-927c-abdfef86c1f8")];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            66_500 + n as u64,
            &[ids::headless_horseman()],
            &[card],
            &[],
            &[],
        );
        let zombie = obj(&e, P0, ids::headless_horseman());
        let bramble = obj(&e, P1, card);
        attack(&mut e, &[zombie]);
        block(&mut e, &[(bramble, zombie)]);
        through_combat(&mut e);
        assert!(
            in_graveyard(&e, P0, ids::headless_horseman()).is_some(),
            "row {n}: Zombie died"
        );
        assert_eq!(
            damage_on(&e, bramble),
            0,
            "row {n}: Zombie damage was prevented"
        );
    }
}

#[test]
fn protection_from_arcane_prevents_arcane_spells_from_targeting() {
    let cards = [card_index("87e44296-6c79-4065-aa54-33e508293a79")];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            66_800 + n as u64,
            &[],
            &[card, ids::barbary_apes()],
            &[unnatural_speed()],
            &[],
        );
        let protected = obj(&e, P1, card);
        let apes = obj(&e, P1, ids::barbary_apes());
        let opts = targets_offered_for(&mut e, P0, unnatural_speed(), &[(ManaColor::Red, 1)]);
        assert!(opts.contains(&apes), "row {n}");
        assert!(
            !opts.contains(&protected),
            "row {n}: Arcane spell cannot target Kitsune Riftwalker"
        );
    }
}

#[test]
fn murder_destroys_any_target_creature_at_instant_speed() {
    let mut e = game(
        67_000,
        &[],
        &[ids::headless_horseman(), ornithopter()],
        &[murder(), murder()],
        &[],
    );
    assert!(
        !castable(&e, P0, murder()),
        "Murder is not castable with an empty mana pool"
    );
    // Cast on opponent's turn (instant speed) targeting a black creature (Headless Horseman)
    // and then an artifact creature (Ornithopter).
    reach_their_main_phase(&mut e, P1);
    pass_once(&mut e);
    let horseman = obj(&e, P1, ids::headless_horseman());
    let thopter = obj(&e, P1, ornithopter());
    float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    assert!(castable(&e, P0, murder()));
    cast_at(&mut e, P0, murder(), &[horseman], &[]);
    assert_eq!(pool_total(&e, P0), 0);
    assert!(in_graveyard(&e, P1, ids::headless_horseman()).is_some());

    pass_once(&mut e);
    float(
        &mut e,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    cast_at(&mut e, P0, murder(), &[thopter], &[]);
    assert!(in_graveyard(&e, P1, ornithopter()).is_some());
}
