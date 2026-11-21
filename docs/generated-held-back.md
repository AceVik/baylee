# Generated cards held back from the pool

The full-corpus run (14.–15.10.2026, corpus `b6f135e649de`) took every script
the reader takes in full, except the cards below. `xtask batch` knows nothing
of this list and offers them again: leave them out until their cause is fixed,
then let them in and play them. Each cause is a reader, engine or fixture gap,
never a reason to edit a card by hand.

## Reader gaps

- Bloodhall Ooze, Graceful Antelope: optional "may" semantics the reader drops.
- Disembowel, Molder, Repeal: the target preview for X (its ceiling stays 6; the owner declined an exception list).
- Cogwork Spy, Deal Broker, Illusionary Informant, Myr Kinsmith: `validate`: the printing says "you may" and the code never asks.
- Rage-Scarred Berserker, Worst Fears: `validate`: "you control" with no `Filter::ControlledByYou`.
- Tempest Caller: `validate`: "an opponent controls" with no `Filter::ControlledByOpponent`.
- Hammer of Purphoros, Bident of Thassa, Bow of Nylea: `validate`: type line order "Enchantment Artifact" written as "Artifact Enchantment".
- Lotus Ring: its granted mana ability has no printed line, and it is a priced granted mana ability with no board test.
- Demonmail Hauberk, Murderer's Axe: mana sheet row with neither printed sentence nor cost.
- Orzhov Locket, Letter of Acceptance: a written row that cannot name its printed sentence.
- Found by the review of PR #410 (01.11.2026), whose full run finished them wrong. Hand-owned since (`codegen --adopt-stub`, then written and played as printed in the same review, so a full run no longer writes over them); each reader rule is still wrong and still finishes any other card that prints the sentence the same way:
  - Hazy Homunculus: "can't be blocked as long as defending player controls an untapped land" comes out as plain unblockable (the condition is dropped in `scriptgen/statics.rs::cant_block_by`).
  - Wall of Vipers: "Destroy this creature and target creature it's blocking" loses "destroy this creature" (two +0/+0 pump links accepted, `scriptgen/effects.rs::pump_effect`; the `Destroy` link emits only the target).
  - Opalescence: `AddType$ Creature` with mana-value P/T is read as `Modifier::AnimateNoncreatureArtifact`, which also adds the artifact type (`scriptgen/statics.rs::pt_modifiers`; right only for artifacts). The card now uses `Modifier::CreatureWithManaValuePT`, which that rule could emit for a non-artifact filter.
  - Mercenary Informer: "target nontoken Mercenary" loses "nontoken" (the script's valid-string carries none; nothing holds the printed word against `Filter::IsToken`).
- Sarkhan's Dragonfire (corpus gap, found in the review of the Dig reader, 19.11.2026): its card prints "the rest on the bottom of your library in a random order", and the reference script's dig line carries no random order, so the reader writes `LookAtTopFind { …, random: false }`, an order the player chooses. Leave it out until the script says so.
- Diviner's Wand: its dual-grant sentence (`Equipped creature has "..." and "{4}: Draw a card."`) leaves both `GrantActivated` and `GrantTriggeredDefinition` without printed lines in `generated_lines.rs` (`lines::tests::every_grant_is_found_on_its_own_face`).
- Undercity Uprising: valid spell whose team deathtouch sweep shares `Filter::YOUR_CREATURE` with its `Fight` target, tripping `lints::tests::targets::no_targeted_ability_sweeps_the_board_instead`.
- Obscuring Haze: `validate`: `"you control"` from `AltCondition::CommanderControlled` is not yet recognized by `check_text::filters_you` without `Filter::ControlledByYou`.
- Witch of the Moors: valid trigger whose opponent sacrifice (`Filter::CREATURE`) shares `HasType(CREATURE)` with its `GraveyardToHand` target, tripping `lints::tests::targets::no_targeted_ability_sweeps_the_board_instead`.
- Eladamri, Lord of Leaves Avatar: Vanguard card prints `{G}{G}` in its mana trigger while Scryfall carries an empty `color_identity`, tripping `lints::tests::faces::no_card_hides_a_color_it_prints`.
- Dune Mover: `validate`: optional `"you may"` is dropped (`SearchLibrary { ..., optional: false }`).
- Invasion of Vryn // Overloaded Mage-Ring: reference `CopySpellAbility` script omits `MayChooseTarget$ True`, so codegen writes `may_choose_new_targets: false` for `"You may choose new targets for the copy."` (`#326`).
- Romana II, The Fifth Doctor, The Sixth Doctor, The Third Doctor, The Thirteenth Doctor: `Time Lord` two-word subtype missing from `subtypes.rs` in `xtask validate`.
- Bolshack Dragon: `Armored` subtype missing from `subtypes.rs` in `xtask validate`.
- Omenpath to Naya: `Omenpath` subtype missing from `subtypes.rs` in `xtask validate`.
- Psychic Whorl: lowers `"Target opponent"` to `TargetSpec::AnyPlayer` instead of an opponent target, flagged by `xtask validate`.
- Wildfire Awakener, Neutralize the Guards: `check_text` rejects the generated token's player target and `PumpFilter.controlled_by` lowering respectively.
- Osgood, Operation Double: cast trigger (`"When you cast this spell, create a token that's a copy of it, except it isn't legendary."`) lowers to stack `Effect::CopySpellOrAbility` instead of creating a token copy on the battlefield when resolved (CR 111.10 / CR 111.13).
- Omega, Heartless Evolution: `check_text` wants `Filter::ControlledByOpponent` for "that opponent controls" beside `TargetSpec::ObjectOfEachOpponent` (`filters_theirs` does not read that spec yet).
- Wail of War: `check_text` rejects `controlled_by: Some(PlayerRel::Chosen)` paired with `TargetSpec::AnyOpponent` without `Filter::ControlledByOpponent`.
- Dion, Bahamut's Dominant, Jill, Shiva's Dominant: Chapter III (`"Exile ..., then return it to the battlefield (front face up)."`) lowers `ExileSelfReturnAsFace { owner_control: true }` despite omitting `"under its owner's control"`, tripping `entries::every_self_return_comes_back_under_the_control_its_sentence_prints`.
- The Eternity Elevator: non-creature Spacecraft (`Station 20+`) with no printed P/T box, tripping `faces::a_creature_is_exactly_a_face_with_a_body` because `pt_fault` unconditionally expects `power`/`toughness` on every `SPACECRAFT` face.
- Hog-Monkey Rampage: `"creature an opponent controls"` lowers to `Filter::Not(&Filter::ControlledByYou)` instead of `Filter::ControlledByOpponent` (`xtask validate`).
- Feast of Worms, Render Silent, Strength of the Coalition: valid targeted spells that reuse `&Filter::LAND`, `&Filter::Any`, or `&Filter::YOUR_CREATURE` on their follow-up sacrifice/prevention/kicked-sweep effect, tripping `lints::tests::targets::no_targeted_ability_sweeps_the_board_instead` (`target_reuse`).
- Startled Awake // Persistent Nightmare: its graveyard return ability prints `"Put this card from your graveyard onto the battlefield transformed"`, lacking `"return"` required by `lints::tests::entries::every_self_return_comes_back_under_the_control_its_sentence_prints`.
- Forked-Branch Garami: dual `Soulshift 4` lowers as two single-target `Dies` triggers instead of one `"up to two target"` trigger matching its combined reminder text (`xtask validate`).
- Blight Grenade, For the Common Good: valid spells whose `-3/-3` or token-indestructible sweep shares `&Filter::CREATURE` or `And(&[IsToken, ControlledByYou])` with their target, tripping `lints::tests::targets::no_targeted_ability_sweeps_the_board_instead` (`target_reuse`).
- Swashbuckler's Whip: its compound Equipment grant leaves `Modifier::GrantActivated` (`Discover 10`) without a printed line in `generated_lines.rs` (`lines::tests::every_grant_is_found_on_its_own_face`).
- Reroute: `scriptgen` emits nonexistent `TargetSpec::ActivatedAbility` instead of `TargetSpec::AbilityOnStack` for `"target activated ability with a single target"`.
- Redirect, Sideswipe: print `"You may choose/change ... target(s)"` and lower to bare `Effect::ChooseNewTargets` without `MayDo`, tripping `xtask validate`'s optional-clause check.
- Rikala, Homarid King: corpus script erroneously marks its mandatory upkeep tide-counter trigger optional (`OptionalDecider$ You`), wrapping the entire trigger in `Effect::MayDo`.
- Sword of War and Peace: `DealsCombatDamageToPlayer` lowers damage recipient as `TargetSpec::EventObject` (which is `None`) instead of `DamagedPlayer`.
- Furnace Scamp: `DealsCombatDamageToPlayer` lowers sacrifice-then-damage recipient as `TargetSpec::EventObject` instead of `DamagedPlayer`.
- Temporal Firestorm: `DealDamageEach` is emitted inside `ChooseCard.then` (`max: TimesKicked`) instead of after `ChooseCard`, dealing 0 damage unkicked and duplicated damage when kicked twice.
- Taborax, Hope's Demise: `MayDo` wraps an unconditional `draw(1)` outside `IfCondition(Cleric)`, letting the controller draw when a non-Cleric dies.
- Durkwood Tracker: `TargetPower` fight-back lowers `Durkwood Tracker` (`ThisObject` damage) as the source dealing damage to itself instead of the target creature dealing damage to `Durkwood Tracker`.

## Engine gaps

- Shallow Grave: its effect exists since Corpse Dance (`Effect::TopOfYourGraveyard`, 4 November 2026). Nothing but its card file and its own test is left, and the same effects do it.
- Comet Storm, Strength of the Tajuru, Zethi: a target count that grows with the kicks. `Amount::TimesKicked` reaches effects and entries, not `TargetReq`.

- Kolaghan's Command, Silumgar's Command: target in more than two modes.
- Behold the Beyond: a quantity-only search (CR 701.23d) that may not settle for fewer.
- Keldon Warcaller, Mind Unbound: lore counters; the AI guard wants a regression first (#166).
- Cruel Sadist, Geistflame Reservoir: `RemoveCounterSelfX` paired with a target asks for X before showing legal targets (CR 601.2b/c; flagged by `lints::tests::costs::no_cost_announces_a_number_on_an_ability_that_also_targets`).
- Ayula, Queen Among Bears: mode 1 of `ModalTriggered` uses `second_targets` (`Target Bear you control fights target creature you don't control`), which triggered stack selection does not yet read (`tests::no_modal_trigger_mode_prints_a_second_target`).
- Hourglass of the Lost: uses `RemoveCounterSelfX` on a stack-using activated ability (needs an engine test verifying announced `x_value` before release; flagged by `offer_tests::every_counter_x_cost_in_the_pool_is_on_a_mana_ability`).

## Fixture gaps (the card may be right; the pool-wide test cannot place it)

- Vapor Snare: upkeep "return a land" on an Aura; the fixture plays creatures only.
- Faerie Impostor, Glint Hawk, Quickling, Tragic Lesson: pay by returning a permanent; no case in `card_tests::lands`' table.
- Awakening Zone: its upkeep "may" stops the mana sweep before main.
- Abundant Growth, Gift of Paradise, Karametra's Favor, Nature's Embrace, New Horizons, Sheltered Aerie, Unbridled Growth, Urban Utopia, Weirding Wood: mana Auras the sweep's attach fixture cannot place.

## Identity clashes

- Pestbrood Sloth, Send in the Pest: a second `PEST_1_1_BLACK_GREEN` token (with an attack trigger) beside the pool's one.
- Drudge Spell: a second `SKELETON_1_1_BLACK` token (`b_1_1_skeleton_regenerate`, with `{B}: Regenerate this creature`) beside the pool's plain `SKELETON_1_1_BLACK` (`b_1_1_skeleton`).
- The 26 Alchemy rebalanced cards land on the paper card's file path: `A-Sepulcher Ghoul`, `A-Blood Artist`, `A-Canopy Tactician`, `A-Elderleaf Mentor`, `A-Skemfar Avenger`, `A-Rockslide Sorcerer`, `A-Umara Mystic`, `A-Dragonborn Looter`, `A-Eyes of the Beholder`, `A-Goggles of Night`, `A-Pseudodragon Familiar`, `A-Celebrity Fencer`, `A-Glittermonger`, `A-Jackhammer`, `A-Midnight Assassin`, `A-Most Wanted`, `A-Ominous Parcel`, `A-Riveteers Initiate`, `A-Social Climber`, `A-Speakeasy Server`, `A-Stimulus Package`, `A-Haywire Mite`, `A-The One Ring`, `A-Mightstone's Animation`, `A-Harald, King of Skemfar`, `A-Warm Welcome` (the last two read in full since the Dig reader, 19.11.2026).
