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
- Rage-Scarred Berserker: `validate`: "you control" with no `Filter::ControlledByYou`.
- Tempest Caller: `validate`: "an opponent controls" with no `Filter::ControlledByOpponent`.
- Hammer of Purphoros: `validate`: type line order "Enchantment Artifact" written as "Artifact Enchantment".
- Lotus Ring: its granted mana ability has no printed line, and it is a priced granted mana ability with no board test.
- Demonmail Hauberk, Murderer's Axe: mana sheet row with neither printed sentence nor cost.
- Orzhov Locket, Letter of Acceptance: a written row that cannot name its printed sentence.
- Found by the review of PR #410 (01.11.2026), whose full run finished them; returned to their stubs, since `--hold-back` refuses a finished machine-owned card and no corpus was at hand to fix the reader against:
  - Hazy Homunculus: "can't be blocked as long as defending player controls an untapped land" came out as plain unblockable (the condition dropped in `scriptgen/statics.rs::cant_block_by`).
  - Wall of Vipers: "Destroy this creature and target creature it's blocking" lost "destroy this creature" (two +0/+0 pump links accepted, `scriptgen/effects.rs::pump_effect`; the `Destroy` link emits only the target).
  - Opalescence: `AddType$ Creature` with mana-value P/T read as `Modifier::AnimateNoncreatureArtifact`, which also adds the artifact type (`scriptgen/statics.rs::pt_modifiers`; right only for artifacts).
  - Mercenary Informer: "target nontoken Mercenary" lost "nontoken" (the script's valid-string carries none; nothing holds the printed word against `Filter::IsToken`).

## Engine gaps

- Kolaghan's Command, Silumgar's Command: target in more than two modes.
- Behold the Beyond: a quantity-only search (CR 701.23d) that may not settle for fewer.
- Keldon Warcaller, Mind Unbound: lore counters; the AI guard wants a regression first (#166).

## Fixture gaps (the card may be right; the pool-wide test cannot place it)

- Vapor Snare: upkeep "return a land" on an Aura; the fixture plays creatures only.
- Faerie Impostor, Glint Hawk, Quickling, Tragic Lesson: pay by returning a permanent; no case in `card_tests::lands`' table.
- Awakening Zone: its upkeep "may" stops the mana sweep before main.
- Abundant Growth, Gift of Paradise, Karametra's Favor, Nature's Embrace, New Horizons, Sheltered Aerie, Unbridled Growth, Urban Utopia, Weirding Wood: mana Auras the sweep's attach fixture cannot place.

## Identity clashes

- Pestbrood Sloth, Send in the Pest: a second `PEST_1_1_BLACK_GREEN` token (with an attack trigger) beside the pool's one.
- Drudge Spell: a second `SKELETON_1_1_BLACK` token (`b_1_1_skeleton_regenerate`, with `{B}: Regenerate this creature`) beside the pool's plain `SKELETON_1_1_BLACK` (`b_1_1_skeleton`).
- The 24 Alchemy rebalanced cards land on the paper card's file path: `A-Sepulcher Ghoul`, `A-Blood Artist`, `A-Canopy Tactician`, `A-Elderleaf Mentor`, `A-Skemfar Avenger`, `A-Rockslide Sorcerer`, `A-Umara Mystic`, `A-Dragonborn Looter`, `A-Eyes of the Beholder`, `A-Goggles of Night`, `A-Pseudodragon Familiar`, `A-Celebrity Fencer`, `A-Glittermonger`, `A-Jackhammer`, `A-Midnight Assassin`, `A-Most Wanted`, `A-Ominous Parcel`, `A-Riveteers Initiate`, `A-Social Climber`, `A-Speakeasy Server`, `A-Stimulus Package`, `A-Haywire Mite`, `A-The One Ring`, `A-Mightstone's Animation`.
