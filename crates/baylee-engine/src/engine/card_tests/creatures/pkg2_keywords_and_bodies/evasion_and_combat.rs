//! Evasion, blocking, defender, and mandatory-attack behavioral tests for Package 2.

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_spider() -> CardIndex {
    card_index("e740ce2f-2134-473c-afa1-1b6d2d1e38ef")
}

fn clockwork_beast() -> CardIndex {
    card_index("eb97c8db-ac6c-476c-b14d-87785e9c82f0")
}

fn soltari_foot_soldier() -> CardIndex {
    card_index("6ccb3ac8-0509-431b-a196-d3cbf4fa80ae")
}

#[test]
#[allow(clippy::too_many_lines)]
fn attacking_fliers_are_blocked_only_by_flying_or_reach() {
    let cards = [
        card_index("011b2243-b8be-471f-9d8f-824a40e599df"),
        card_index("02eef375-5166-4b6b-bb17-b38074a6ba53"),
        card_index("08d02fdb-4b3f-4f55-b5fc-6bb4d2a14b1f"),
        card_index("0b3b2fb1-d5fe-4d14-985a-85f13cd60d9c"),
        card_index("0dc8bdbb-996b-47d7-a5e5-7bdb6d9280fe"),
        card_index("0e11792b-7fe5-4208-aa0b-e5d09b2b65fe"),
        card_index("0f51ab3d-aae9-4d8d-a881-e8835dcd7866"),
        card_index("0fabb6e4-35cf-4e56-a27c-ff627d8e9b7a"),
        card_index("0feeeb19-7c90-4009-9a71-226a443b5c3f"),
        card_index("11369d8b-6443-43c3-a60f-dd920a4e8c08"),
        card_index("11f77158-0240-4882-94a4-6bd6b3d94bcc"),
        card_index("126844f5-3a95-48cb-80f2-d7e4b0005350"),
        card_index("12a9a9a4-6b8c-48a1-833f-457a7ece8e7e"),
        card_index("1a36d2a8-6117-4828-bbe2-bc63212d29d8"),
        card_index("1c34b2e4-e600-4688-86cb-8a1c11fdc770"),
        card_index("1c4bd85b-03c2-4235-aea1-76c48520adf3"),
        card_index("1c96772b-463c-4655-836b-cf6e9dc6319f"),
        card_index("22936eb4-8712-4ba7-94da-e124487acda8"),
        card_index("28a52ba1-95da-44e1-8ac5-0dc23c902394"),
        card_index("2c378f72-917c-49d4-ba68-51ebaf18d976"),
        card_index("2ef481fa-ff20-426a-af16-fbf761ea5eaf"),
        card_index("2f4b5831-57a2-4778-90c1-0b8ec49afd46"),
        card_index("2f7305c1-51d2-4698-89d5-c64a57ecff80"),
        card_index("2f9de785-a80a-4356-a4b4-0e0243e8ea2d"),
        card_index("33435856-ea24-4d10-84d5-6872b35b63a3"),
        card_index("36a600fe-91b4-46a0-97ee-0cea2b63ec8e"),
        card_index("372e2d6e-427c-4152-b936-27dcf536ca84"),
        card_index("376d90f6-2ceb-4d9e-a04d-3dd5bafd1e8b"),
        card_index("379ef166-a29f-4715-8ce4-9d7d567e4e91"),
        card_index("37ad9318-6690-4a8d-bcea-9fcbca58769d"),
        card_index("38c588ae-7254-4fae-aa9a-03e2a5524492"),
        card_index("3bbfceba-9f3a-4988-a009-13c9472138b1"),
        card_index("3d529824-8e82-4c44-8e28-ed3419740c11"),
        card_index("4317f4f1-d339-4940-b46e-659641035595"),
        card_index("438ee544-78a2-4e32-9343-70838b60ac16"),
        card_index("447b9412-2649-4548-8e9b-eed5e92ad92a"),
        card_index("4e1f4211-5865-492d-b850-0d36ac12e857"),
        card_index("50372c54-359c-4ee6-9e21-1e40ba53652d"),
        card_index("518d0773-40fa-4b02-a89e-5711c19617cd"),
        card_index("535007c5-03a8-495f-a132-37960f179f96"),
        card_index("552dcf9f-ff4a-4057-a93e-62f5527050c9"),
        card_index("56cb7592-ad58-42dc-b568-4b7754bba53d"),
        card_index("57d8d656-ccbd-4deb-abf1-a7fde4e7d691"),
        card_index("57d93683-8739-49ef-856a-5a70f92d8763"),
        card_index("57dfe4aa-1e9a-4891-851b-201edd1fc79c"),
        card_index("58d87cfe-7402-43c3-81ce-13de8cd04005"),
        card_index("58e754ad-88b4-47c5-b105-8b50e54143c8"),
        card_index("59b6bf51-d15d-4032-85ac-29b234b59fc9"),
        card_index("59fa8323-bba9-4c51-833d-2ab966599b62"),
        card_index("5a7ecb5c-9b22-4d2c-b851-db92c1d3c745"),
        card_index("5c1a0a7a-dea4-4b8e-b9c0-8c19942a01ef"),
        card_index("5c9d814b-4257-47bc-a1df-c3480bc038ba"),
        card_index("5fe2ed67-55a6-4816-910f-0e3c38ea06a6"),
        card_index("637b1a19-2925-422e-b867-f87537803071"),
        card_index("640fa591-ba79-4148-86a6-9de994a9e67c"),
        card_index("6774c0ce-103e-471b-8de0-f325805674ca"),
        card_index("678c76bf-fc31-47e4-8c19-712e15bba9e3"),
        card_index("67f59250-d46a-4637-9493-165d23278817"),
        card_index("68a99e50-0e98-480b-ad25-aa2c226d24ca"),
        card_index("6987d5d4-0ef6-4326-a590-678ac160fe56"),
        card_index("69b1b0ec-9db0-48d1-a7b5-71281aca16fe"),
        card_index("6adff11d-5126-4a4b-b182-e2700d406995"),
        card_index("6bc14f1e-1df8-4e72-b05a-a848faa3b080"),
        card_index("6ca0078f-d6b5-4643-b801-e7a98706f21c"),
        card_index("6eb74f6e-1c9d-4caf-a0d8-fa472d435346"),
        card_index("6edacc06-9523-4ccf-a4c2-2ac4af386b8a"),
        card_index("72df0b70-5c9f-44f3-96d5-81c95abc0065"),
        card_index("7338f964-b7d8-4491-b933-7d8b7d1b56ed"),
        card_index("7422f43d-0c1e-42cb-864d-1c63f2ec533a"),
        card_index("767c1f97-03de-474d-9112-e7521ba3b3f3"),
        card_index("782e8499-0714-4cee-869e-8ebd7dad0242"),
        card_index("786afd52-0eae-4453-b1bd-cffc21699849"),
        card_index("7a7f37a3-9e99-4d3d-b705-b074fb3e9b3e"),
        card_index("7b813e46-34f6-4918-97a7-a4b2a2464e86"),
        card_index("7f240b1f-fb7b-49a3-bcaf-6348956d2a38"),
        card_index("7fc904aa-377b-4671-8308-275205f56d81"),
        card_index("802b9333-c256-410a-a635-a22bfeda679f"),
        card_index("8049dbff-6b57-454a-a2e6-7e2ae39bb606"),
        card_index("82657b8b-de2d-46ab-ab56-e2790e326d95"),
        card_index("894979ba-e4c7-4914-b0d3-133cbaed38c2"),
        card_index("8cdda36f-5e82-4c2f-8eaa-75b9bbc93a02"),
        card_index("904b2caf-e083-4917-8841-95f48d90dc0c"),
        card_index("93c35f9e-3346-4269-b1af-df9f216bfd7a"),
        card_index("93cd12da-a7d5-4d01-a3bc-77e441cb0947"),
        card_index("954836f8-c225-450a-bb51-f2adb6923ef5"),
        card_index("994cf8bb-c900-448f-b055-ecc2ee6cf9e2"),
        card_index("9a642db9-c337-481c-b260-11e01f64e68e"),
        card_index("9b344428-ec35-45b9-9714-9ca58a338f5c"),
        card_index("9bc6d4ac-6fca-4574-9cd8-10a5528575b3"),
        card_index("9c724912-5afd-4813-affb-4dbda64fd4ee"),
        card_index("9d57cda6-9042-44bd-9785-38de9487ff85"),
        card_index("9f980829-90c1-43e2-a103-915f38317ed0"),
        card_index("a084b37a-bfb2-4dde-9faa-4c55ad894300"),
        card_index("a16f4b53-c8cf-45b0-89e7-507acd094a64"),
        card_index("a33982e4-d7d2-4319-bf04-534b8f253dfb"),
        card_index("a361fdd1-7f97-4aed-8e24-ab2a03387f73"),
        card_index("a7520962-46e0-4abd-bb19-569334913a05"),
        card_index("a93779c5-603e-4a91-aef8-a0cd8fd1b3e9"),
        card_index("a93f0735-9854-487d-b646-d55e2bee00c0"),
        card_index("aa17782b-d8e7-423b-9f03-01ccb600f69a"),
        card_index("adb5883d-44cc-455e-94dc-20e1c1f97d68"),
        card_index("ae2329a1-e37b-44aa-8766-077910fa1197"),
        card_index("af2a951f-7630-48fe-bb37-273b66901eb6"),
        card_index("b318d94e-314f-4df8-a178-b5ddd756e1c4"),
        card_index("b35c1b30-c1dc-489f-bd79-ad6566fb9389"),
        card_index("b365a7dd-fed1-40f8-9544-2b5bc1af761c"),
        card_index("b4de9edd-4a5b-4858-8454-38381df8b9a2"),
        card_index("b5868383-a0fb-40fc-b718-f1b5441c5ce3"),
        card_index("b5d5b9c9-accc-4dbe-aaf4-c806f3e1c61c"),
        card_index("b92e0156-ea0f-4b13-b3bc-7e84a31445fc"),
        card_index("b94591b8-2e99-4983-8f36-9b52207bfd2e"),
        card_index("b9be76a3-e14f-48dd-8824-17d5708ac7e0"),
        card_index("be37ef9d-955a-417e-bde8-bb9e0bdf3612"),
        card_index("c208b959-d0e4-4a9a-8255-2c7cc7596767"),
        card_index("c377c3e0-2b91-43c6-9f49-5dd6fbf94308"),
        card_index("c4a99095-38cc-4fd3-b3da-c72a6ccceb25"),
        card_index("cbe5c74b-1006-4c63-8b46-e06b61e6fe02"),
        card_index("cca29a9b-794f-4712-8e6c-36ce3da9cb8b"),
        card_index("cf02b774-2b3a-4459-bf59-e5d291691e77"),
        card_index("d014b02f-a8b8-4b97-96e0-f588792069f0"),
        card_index("d383902c-63ba-4714-bfeb-377829950b0d"),
        card_index("d38a33ae-59d1-4d08-8e14-5fa5a8c3762d"),
        card_index("d684553a-f225-49af-90eb-399f55ef65df"),
        card_index("d7ebd4a7-c542-469f-8a1c-d48b92fa540a"),
        card_index("d87ab916-5d49-4840-8d5b-805af800a83c"),
        card_index("d8c70f2f-985c-4982-97eb-99da3810e048"),
        card_index("d9410a82-1b7f-4f3a-a0fd-53099d7594ec"),
        card_index("d9b7881b-915a-42f1-bd2f-0aaa900a8981"),
        card_index("d9f43942-4296-417d-a180-5b4f64e4b12d"),
        card_index("df7580fb-4c37-4c09-8785-e320cb124621"),
        card_index("e80be353-8b19-42d2-9141-5353a8409c8e"),
        card_index("e9f410c1-d93d-4291-95fc-513832fb36e0"),
        card_index("efd70846-2226-43c6-80fe-6acdd15d4246"),
        card_index("f1ba6053-14e0-46b0-b760-b72e4fa0c5f5"),
        card_index("f394d2bf-89aa-44c1-b473-8c7a8874a9a3"),
        card_index("f8219a99-1620-40aa-a257-fc3dfb239d51"),
        card_index("f8848065-7075-41d8-82f6-67ace390625a"),
        card_index("fc1256e9-eb92-43b6-abfc-ce19d910a929"),
        card_index("feb244f8-bcb1-44cf-9940-2719221a7309"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            40_000 + n as u64,
            &[card],
            &[ids::barbary_apes(), ids::vampire_bats(), giant_spider()],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        let apes = obj(&e, P1, ids::barbary_apes());
        let bats = obj(&e, P1, ids::vampire_bats());
        let spider = obj(&e, P1, giant_spider());
        // Strip colors from the flying and reach probe blockers so protection from
        // color on a flying attacker does not interfere with testing flying/reach.
        let state = e.dev_state_mut(P1).unwrap();
        state.object_mut(bats).unwrap().base_mut().colors = ColorSet::EMPTY;
        state.object_mut(spider).unwrap().base_mut().colors = ColorSet::EMPTY;
        state.refresh_characteristics();

        assert!(has_kw(&e, attacker, KeywordSet::FLYING), "row {n}");
        let offer = attack(&mut e, &[attacker]);
        assert!(
            !may_block(&offer, apes, attacker),
            "row {n}: ground blocked flier"
        );
        assert!(
            may_block(&offer, bats, attacker),
            "row {n}: flier may block flier"
        );
        assert!(
            may_block(&offer, spider, attacker),
            "row {n}: reach may block flier"
        );
    }
}

#[test]
fn defender_fliers_have_flying_and_can_block_fliers() {
    let cards = [
        card_index("11f7f6e4-2c39-4478-8c10-4473dbf921a2"),
        card_index("14fa0771-cf2b-4563-98b2-ff4ee24bce21"),
        card_index("9cc092f9-e8f0-40ea-847d-813b70f889db"),
        card_index("a09cce6f-c957-49d7-af99-a7b2bce786e8"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            41_000 + n as u64,
            &[ids::vampire_bats()],
            &[card, ids::barbary_apes()],
            &[],
            &[],
        );
        let wall = obj(&e, P1, card);
        assert!(has_kw(&e, wall, KeywordSet::FLYING), "row {n}");
        let bats = obj(&e, P0, ids::vampire_bats());
        let offer = attack(&mut e, &[bats]);
        let apes = obj(&e, P1, ids::barbary_apes());
        assert!(
            may_block(&offer, wall, bats),
            "row {n}: flying wall blocks flier"
        );
        assert!(
            !may_block(&offer, apes, bats),
            "row {n}: ground ape cannot block flier"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn reach_creatures_can_block_attacking_fliers() {
    let cards = [
        card_index("0ae0c880-a508-471f-a90b-029c417f6b3b"),
        card_index("0ed811b7-6ead-4906-9e40-9096b735e271"),
        card_index("169429d8-aa98-46a8-8681-521c65dafbb4"),
        card_index("1f2d1de6-42e0-4f45-b43d-ce4980146469"),
        card_index("2061fe08-0c7c-4d79-a144-996c3012952c"),
        card_index("241ec1fc-cc50-468e-8ee3-f95a82a3e07b"),
        card_index("257f7c29-8857-429e-a421-9262982804e0"),
        card_index("2f047fcb-c198-44ac-aa1e-31447f6c6aba"),
        card_index("325b4288-fe1b-4e29-ae13-72924b4db3dd"),
        card_index("34ff281a-f294-4351-9665-863a5b2e882b"),
        card_index("369f086f-3497-496a-94a8-0faccd1d5931"),
        card_index("39daa265-8d02-4449-9c45-ae8ec2b13e5b"),
        card_index("419e9ab0-fe79-4a4f-8bcb-fe4e5373e292"),
        card_index("485e1d84-2867-4c66-a399-cdc2a22f6fc9"),
        card_index("4de340bf-eab6-4066-896d-7084a9c13c2d"),
        card_index("51ce0158-c9a5-4fa5-9704-46ab02f01b86"),
        card_index("533c4956-87f8-4bae-8924-166fc2e9ec1f"),
        card_index("61c3b999-4f5d-4018-a9ee-ef894fd1ef6c"),
        card_index("63c52071-e3a7-4236-9dcb-e5ea68240dc2"),
        card_index("6b941291-1802-4ced-9869-39fa43825550"),
        card_index("6fbaa853-48d4-4a3b-b22d-27415cc266d7"),
        card_index("73346137-3117-4dc3-b5a4-1aca1da54548"),
        card_index("76f22670-6806-4124-8bbb-822ed33d2bda"),
        card_index("77b69977-200e-44c2-bfde-228e6501b03a"),
        card_index("78bc8b7c-ca94-485c-b7bf-53c3724b7339"),
        card_index("913dbce2-6805-453f-a9c3-64f1cc658627"),
        card_index("91585b64-f10d-47c6-a563-384ceb944710"),
        card_index("930035c6-cde7-40ac-98ef-0840812037db"),
        card_index("939203ec-3de4-44d8-932e-63a32e5f2d47"),
        card_index("94399b1f-e66a-422e-ae41-6013fe741e8c"),
        card_index("95f20067-4471-4bfa-952d-dbbfbb2c2178"),
        card_index("a37c9b13-dc45-4b55-a63d-5314abde7aab"),
        card_index("ad6bdd24-5a32-4046-83ee-2fe51bf8bb7a"),
        card_index("b0747a4f-d041-4750-aef5-a5d101696f65"),
        card_index("b0c2938d-8370-4786-8ddd-126c61538bdd"),
        card_index("b52824f9-7897-46b7-a4e5-a0261861cb5b"),
        card_index("b5ba87f5-3589-4a8b-ad65-2ae2523e98e8"),
        card_index("b820c29e-2428-4074-a024-9502848484d6"),
        card_index("b8563788-debc-4f97-adf4-b186bf3960e2"),
        card_index("d481630c-83c4-47d2-a770-db331154d2b9"),
        card_index("df77cb98-e3c6-4927-99d5-c3380641f356"),
        card_index("e7e65cc5-9ceb-43b6-acde-ae492b71daf6"),
        card_index("e9b5d4f3-b98a-463a-b628-289c2df81204"),
        card_index("eb9a3b52-62ad-467f-b62a-0273ceee48e5"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            42_000 + n as u64,
            &[ids::vampire_bats()],
            &[card, ids::barbary_apes()],
            &[],
            &[],
        );
        let reacher = obj(&e, P1, card);
        assert!(has_kw(&e, reacher, KeywordSet::REACH), "row {n}");
        let bats = obj(&e, P0, ids::vampire_bats());
        let offer = attack(&mut e, &[bats]);
        let apes = obj(&e, P1, ids::barbary_apes());
        assert!(
            may_block(&offer, reacher, bats),
            "row {n}: reach blocks flier"
        );
        assert!(
            !may_block(&offer, apes, bats),
            "row {n}: ground cannot block flier"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn landwalkers_are_unblockable_when_defender_controls_matching_land() {
    let rows = [
        (
            card_index("01800a0c-ad93-4e45-a904-38253a143638"),
            mountain(),
        ),
        (card_index("01ab37a4-21d2-49f4-9b85-cb522a4d04ad"), forest()),
        (
            card_index("031cd561-d9b1-4a0e-bc38-bc09e80563c3"),
            mountain(),
        ),
        (card_index("048cc868-1657-448d-9f2c-97973e73f477"), forest()),
        (
            card_index("0534a514-2847-4dbc-8178-6a2886c75bf6"),
            mountain(),
        ),
        (card_index("0acb070b-6e81-4e0e-925d-14da9b931355"), island()),
        (card_index("19d5eb29-2d35-44e7-afa1-3ba8d812ed41"), forest()),
        (
            card_index("1f5ac983-a7a5-4b9d-a25c-727a7de6c465"),
            mountain(),
        ),
        (card_index("23875679-5c08-44ff-90b3-13a1a758e5f7"), forest()),
        (card_index("246f7446-036d-469c-9583-5699ebdcec6d"), swamp()),
        (
            card_index("25fb2a67-fd7f-4a06-bd59-118d82c86d6e"),
            mountain(),
        ),
        (
            card_index("27882623-b5ad-49f2-b5ab-f0bdb93268a1"),
            mountain(),
        ),
        (card_index("27c21086-8d19-4379-af37-f251b2ec2502"), forest()),
        (
            card_index("356e2df2-2b41-4d9d-8723-3e6a17a090e8"),
            mountain(),
        ),
        (card_index("3eff03f1-2c5f-4c59-b465-a8c4cd05e1ba"), swamp()),
        (
            card_index("4431ec11-9bd7-4750-8bc7-378beba06201"),
            mountain(),
        ),
        (card_index("47dd7ef4-7645-4878-ab91-261aa8fcf834"), forest()),
        (card_index("4c26c796-26ec-41f7-b9ca-58f4eb935c47"), forest()),
        (card_index("4d62a448-b6a5-43b1-a281-9e9361a5524a"), island()),
        (card_index("5041dbc4-d01d-4f0a-b5c5-6e1df0976340"), swamp()),
        (card_index("59483597-4d8c-4cdc-8bb5-226b1eef10f5"), island()),
        (card_index("59f57c25-ebb7-49df-8cdd-c6f069d1358d"), forest()),
        (card_index("5c4cb33d-2a79-4934-839f-d34e9ca284f8"), swamp()),
        (card_index("5ea4d1d7-219b-4842-99ea-291842baca0d"), swamp()),
        (card_index("60824fae-20ed-4122-82c9-e99a1b679c54"), swamp()),
        (card_index("61652a2d-dfe6-4d4f-8fad-9de1960208cb"), forest()),
        (
            card_index("6da00e2d-4287-48eb-8abe-d153318381fd"),
            mountain(),
        ),
        (card_index("7280b7c5-e4d0-4a20-8634-5e240e6b3ca0"), plains()),
        (card_index("79aba9a7-b957-4139-8d50-f1c91fa17793"), forest()),
        (card_index("7b2eb6f2-bd13-4eb6-b91a-c0fa8f3d96af"), swamp()),
        (card_index("8c4b1ab2-01d9-486a-a9db-ac7c72500d30"), swamp()),
        (card_index("9297e50f-f8a5-4793-b64d-a119404f94ce"), forest()),
        (
            card_index("934d42b2-5dca-4a29-82d3-6778796178d9"),
            mountain(),
        ),
        (card_index("96415c51-2d44-4a96-9caa-9c4fbde59fca"), swamp()),
        (card_index("9c9e71d9-a636-4574-8b7e-e6dd0f71c7c8"), swamp()),
        (card_index("a2c5a504-5c8f-4c1b-a8b1-4e045aee4673"), swamp()),
        (
            card_index("a56b3434-7478-4704-b04b-92f32a1bc462"),
            mountain(),
        ),
        (card_index("a6aa245c-b620-486c-864b-0ea4e9bc4048"), swamp()),
        (card_index("a8ba7a21-b2fe-42fd-87dd-6611c3105096"), forest()),
        (card_index("ae1f22a5-89cf-440c-b90d-fed194b2dee5"), swamp()),
        (card_index("b2f92310-3725-44c2-8953-8b280437217e"), forest()),
        (card_index("b3e9ae89-3507-4429-ab37-7175f4db4710"), forest()),
        (card_index("b97c4d9a-8ea0-40d0-839a-afcd785e807e"), swamp()),
        (card_index("bb1963f9-261c-4d3f-a401-90aaced77e7f"), forest()),
        (card_index("c6aa6091-5503-4c22-91af-f999112b0081"), swamp()),
        (card_index("c6aa6091-5503-4c22-91af-f999112b0081"), forest()),
        (card_index("d62e9510-dc41-493f-8289-6e11303884fc"), island()),
        (card_index("dbdd0963-b395-4cb8-ad9a-ed85a3e9f2e5"), island()),
        (
            card_index("dbf7b3ef-1ca4-4b0d-bec2-f5e60b150013"),
            mountain(),
        ),
        (card_index("e1e67a2f-61b6-49c5-8bcf-5d2b3f0df35d"), swamp()),
        (card_index("e5b11e43-fc7c-4cea-a44a-8af3f124e29e"), forest()),
        (card_index("e7dc3d8b-2c79-463d-b862-9f01ccea05fd"), swamp()),
        (card_index("eb0a0515-7fa4-4319-9fc3-7a31c608ce9b"), swamp()),
        (card_index("f01efb04-ad2b-46fb-852c-dfb7e523e1b7"), forest()),
        (card_index("f474c1f8-43bb-4e56-8131-27c71b3193d2"), swamp()),
        (card_index("f778bf66-e823-4b4c-8586-97e1d519efd8"), island()),
        (card_index("ff1fd718-ee97-425c-b8bf-97fea192c631"), swamp()),
    ];
    for (n, (walker, land)) in rows.into_iter().enumerate() {
        for (with_land, offset) in [(false, 0u64), (true, 1u64)] {
            let p1_board: Vec<CardIndex> = if with_land {
                vec![ids::vampire_bats(), land]
            } else {
                vec![ids::vampire_bats()]
            };
            let mut e = game(
                43_000 + (n as u64) * 2 + offset,
                &[walker],
                &p1_board,
                &[],
                &[],
            );
            let attacker = obj(&e, P0, walker);
            let offer = attack(&mut e, &[attacker]);
            let bats = obj(&e, P1, ids::vampire_bats());
            assert_eq!(
                may_block(&offer, bats, attacker),
                !with_land,
                "row {n} with_land={with_land}"
            );
        }
    }
}

#[test]
fn fear_creatures_are_blocked_only_by_black_or_artifact_creatures() {
    let cards = [
        card_index("2c526c61-6cdb-440c-b4c8-b6d1503cef73"),
        card_index("34a8a658-a5f6-406f-99d8-c32ea2e26202"),
        card_index("6d329bd2-8825-4de9-b3de-749625085015"),
        card_index("72e9a207-dad9-4c51-8040-e90a36bd35e6"),
        card_index("a3122c6a-20be-4bb3-ab2f-928143378ea6"),
        card_index("fa110e05-6759-4eeb-96eb-9b7a8e4a94c3"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            44_000 + n as u64,
            &[card],
            &[
                ids::barbary_apes(),
                ids::headless_horseman(),
                clockwork_beast(),
            ],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        assert!(has_kw(&e, attacker, KeywordSet::FEAR), "row {n}");
        let offer = attack(&mut e, &[attacker]);
        assert!(
            !may_block(&offer, obj(&e, P1, ids::barbary_apes()), attacker),
            "row {n}: green"
        );
        assert!(
            may_block(&offer, obj(&e, P1, ids::headless_horseman()), attacker),
            "row {n}: black"
        );
        assert!(
            may_block(&offer, obj(&e, P1, clockwork_beast()), attacker),
            "row {n}: artifact"
        );
    }
}

#[test]
fn shadow_creatures_are_blocked_only_by_shadow_creatures() {
    let cards = [
        card_index("5fed5377-07cf-4ad9-86fd-1efe1f182088"),
        card_index("6ccb3ac8-0509-431b-a196-d3cbf4fa80ae"),
        card_index("c1d0537b-954f-4d3b-b9fe-1dd7ffaa1197"),
        card_index("ca65b908-e6fe-4ca3-8ecd-247df957aa0f"),
        card_index("cb754749-a769-4c9c-b62b-4972dfc5069f"),
        card_index("f8c9cb9f-3a37-4004-9cde-f51ca5579e1a"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            45_000 + n as u64,
            &[card],
            &[ids::barbary_apes(), soltari_foot_soldier()],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        assert!(has_kw(&e, attacker, KeywordSet::SHADOW), "row {n}");
        let offer = attack(&mut e, &[attacker]);
        assert!(
            !may_block(&offer, obj(&e, P1, ids::barbary_apes()), attacker),
            "row {n}: non-shadow"
        );
        assert!(
            may_block(&offer, obj(&e, P1, soltari_foot_soldier()), attacker),
            "row {n}: shadow"
        );
    }
}

#[test]
fn menace_creatures_reject_single_blocker_and_accept_two_blockers() {
    let cards = [
        card_index("0f8e0b2c-f1a5-40ad-b8d8-7c06a012e909"),
        card_index("124cf15a-587a-4f1f-a943-f462072f2098"),
        card_index("38f66ef9-6afc-478b-a1d4-a9d42c31dfc9"),
        card_index("3c741801-bfa5-4ff6-bf06-8d5edc06388d"),
        card_index("419e9ab0-fe79-4a4f-8bcb-fe4e5373e292"),
        card_index("485e1d84-2867-4c66-a399-cdc2a22f6fc9"),
        card_index("488591d4-3dfb-4e1e-b0c4-1950ff92b848"),
        card_index("4c297085-5ec1-4d29-9a8e-1785c016ce09"),
        card_index("5fe2ed67-55a6-4816-910f-0e3c38ea06a6"),
        card_index("6f246109-8c8b-4909-9888-561bc46ad776"),
        card_index("7063eb2b-81a4-4599-b1db-edf861e39bfb"),
        card_index("7e280770-9a8f-4445-84d2-3b9cac659136"),
        card_index("84557d9d-ee22-46c9-8f4a-62a440fb302f"),
        card_index("880793a2-84b0-4ae1-9bb8-6e942e3dc723"),
        card_index("88b50194-bc39-47a6-968b-983553d89cf9"),
        card_index("92ce3e30-dde3-43bb-a3d6-ccd91ff18bc3"),
        card_index("9f2e03fb-45c0-442d-a716-a472373c72d0"),
        card_index("e2bc1600-eeec-443b-99e4-dd61497edd5b"),
        card_index("eb9b0655-3ca3-4d12-8665-54ef0c2c4de2"),
        card_index("fb9af562-7384-4575-93bb-34f1ada23735"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            46_000 + n as u64,
            &[card],
            &[ids::vampire_bats(), giant_spider()],
            &[],
            &[],
        );
        let attacker = obj(&e, P0, card);
        assert!(has_kw(&e, attacker, KeywordSet::MENACE), "row {n}");
        attack(&mut e, &[attacker]);
        let b1 = obj(&e, P1, ids::vampire_bats());
        let b2 = obj(&e, P1, giant_spider());
        let single = e.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(b1, attacker)],
            },
        );
        assert!(
            single.is_err(),
            "row {n}: single blocker must be rejected by menace"
        );
        block(&mut e, &[(b1, attacker), (b2, attacker)]);
    }
}

#[test]
fn defender_creatures_cannot_be_declared_as_attackers() {
    let cards = [
        card_index("11f7f6e4-2c39-4478-8c10-4473dbf921a2"),
        card_index("14fa0771-cf2b-4563-98b2-ff4ee24bce21"),
        card_index("263b3472-b437-4889-af5e-c7537abb6cb6"),
        card_index("35e684fd-69e3-4137-bf15-94f705191f5e"),
        card_index("40057917-4a88-4316-bc5b-efc0ada1ddff"),
        card_index("443f8dd1-333d-42c4-a286-302a9496209a"),
        card_index("51ce0158-c9a5-4fa5-9704-46ab02f01b86"),
        card_index("661b23fc-78cd-44f7-ba53-0cb89ccca41f"),
        card_index("674a269c-61ed-4fb5-ad74-422e581a5a91"),
        card_index("68df4c46-513a-4d75-9d4b-ac95d45be255"),
        card_index("9cc092f9-e8f0-40ea-847d-813b70f889db"),
        card_index("9e2f854b-17f7-4d52-927c-abdfef86c1f8"),
        card_index("a09cce6f-c957-49d7-af99-a7b2bce786e8"),
        card_index("a55202a9-bddb-40df-b93f-06678ae1e0ea"),
        card_index("ad6bdd24-5a32-4046-83ee-2fe51bf8bb7a"),
        card_index("b2ca8824-6bd8-4aa3-b156-618118eb98a4"),
        card_index("cb7e076c-a695-4c79-9f20-498370c81380"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(
            47_000 + n as u64,
            &[card, ids::barbary_apes()],
            &[],
            &[],
            &[],
        );
        let wall = obj(&e, P0, card);
        assert!(has_kw(&e, wall, KeywordSet::DEFENDER), "row {n}");
        let able = attackers_offered(&mut e);
        assert!(able.contains(&obj(&e, P0, ids::barbary_apes())), "row {n}");
        assert!(
            !able.contains(&wall),
            "row {n}: defender offered as attacker"
        );
    }
}

#[test]
fn mandatory_attackers_must_attack_each_combat_if_able() {
    let cards = [
        card_index("06a3fe24-7e1f-4f98-ba2f-dbb5d38f4b9d"),
        card_index("22134489-efce-4903-9f14-37a2248e8546"),
        card_index("3ef0fff2-6010-42de-a7ce-e4cac295a481"),
        card_index("47d4da1e-b1a2-4a23-9b96-c85d5d20e3ef"),
        card_index("57d93683-8739-49ef-856a-5a70f92d8763"),
        card_index("58e754ad-88b4-47c5-b105-8b50e54143c8"),
        card_index("5ba6a472-b2f3-4d8e-838c-269aaa5eb2e3"),
        card_index("69ba7864-0d09-4517-ad1a-7efaf437ded0"),
        card_index("7471e445-4655-4336-9769-3debd197b77c"),
        card_index("7f240b1f-fb7b-49a3-bcaf-6348956d2a38"),
        card_index("864c81e4-d612-4939-b4df-4b125cb8cdf2"),
        card_index("9e8a2d53-3145-421a-aad1-8f2b2f6eaec8"),
        card_index("aad16b50-a866-4871-843f-ec8813ad5dfd"),
        card_index("c1d0537b-954f-4d3b-b9fe-1dd7ffaa1197"),
        card_index("fafe2d90-e5fc-401a-a971-829192bec6c0"),
    ];
    for (n, card) in cards.into_iter().enumerate() {
        let mut e = game(48_000 + n as u64, &[card], &[], &[], &[]);
        let berserker = obj(&e, P0, card);
        attackers_offered(&mut e);
        let empty = e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] });
        assert!(
            empty.is_err(),
            "row {n}: skipping mandatory attack must be refused"
        );
        declare(&mut e, &[berserker]);
    }
}
