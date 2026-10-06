# Card Audit — Issues Found

Full audit of implemented cards against their Scryfall ruletext, October 2026.
Cards are tested in the running client via the dev-control harness; data fields
are checked against `data/scryfall-cache/*.json`. Nothing here is fixed yet —
this file only records what does not work as printed.

Per-card progress: `scratchpad/card-audit.tsv` (status: pending / ok-static /
ok-game / issues).

## Format

- **Card Name** (`crates/baylee-cards/src/cards/…`) — what is wrong, and what
  the ruletext says instead. CR citations where they settle it.

## Checks completed

- **Data-field diff (all 2954 cards vs Scryfall cache)** — clean. Cost, types,
  supertypes, subtypes, P/T and color identity all match; `*` P/T is modeled as
  base 0/0 + `Modifier::CharacteristicPT` by convention (docs/card-dsl.md).
  One card has no Scryfall cache entry: Emeritus of Woe (fan-set card).
- **Firing sweep (L4, all 2530 implemented cards, 2026-10-05)** — oracle text
  and rulings for the cards touched below were re-fetched from the Scryfall
  API (`api.scryfall.com`). Every implemented card's abilities fire in some
  engine test except the **17 recorder blind spots** in the next section;
  those cards' tests exist, assert the right behaviour, and pass. The three
  actionable gaps were closed:
  - **Oasis** — new test
    `oasis_prevents_the_next_point_of_damage_to_a_creature_and_no_more`
    (`crates/baylee-engine/src/engine/card_tests/lands.rs`). A shielded Hill
    Giant takes a Lightning Bolt for 2 and lives; the unshielded mirror takes
    all 3 and dies, so the survival cannot pass on a Bolt that dealt nothing
    (CR 615.1). Mutant `348:0` killed.
  - **Rivendell** — the existing scry test stopped at the `Pending::Arrange`
    prompt, so the `{1}{U}, {T}: Scry 2` ability never finished resolving and
    the firing log never saw it. The test now answers the scry and asserts
    where the two looked-at cards land (CR 701.18a). Mutant `26089:1` killed.
  - **Scrap Trawler** — `scrap_trawler_tests.rs` resolved its cards through a
    printed-name scan of `baylee_cards::generated::ALL`, a lookup the
    verification ladder does not read (`baylee_train::working` reads oracle-id
    literals and ledger constants). The helper now names each card by oracle
    id through `card_index(…)`, as the rest of the suite does. Mutant
    `16563:0` killed.

## Phyrexian mana on spells (owner report)

Done (2026-10-05, `c41/engine-phyrexian-recorder`): a spell's Phyrexian
symbol can be paid with 2 life (offer and cast wizard; activations already
could). Tests: `card_tests::instants::mental_misstep_*`. Open: the house AI
does not yet seek a Phyrexian cast it lacks the colour for.

## Verification-hook findings (L4 recorder)

Done (2026-10-05, `c41/engine-phyrexian-recorder`): the three recorder bugs
are fixed in `ability_log.rs` — a self-exiling spell logs from what it read as
it began to resolve; static notes are keyed without the timestamp an attach
changes; `MayChooseNotToUntap` logs where the untap step reads it. Test:
`verification_tests::the_recorder_logs_a_self_exiling_spell_an_attached_grant_and_an_untap_static`.

L4's mechanics half ("no untested mechanic") additionally needs `--coverage`
(a `cargo llvm-cov` export), which is not installed on this machine; `xtask
verify` currently reports "no door logs Echo/Prepared/Suspend/Ward/Toxic and
the rule tests do not run it" for those cards, which cannot be graded without
it.

## Arabian Nights (2026-10-05) — engine gaps

The owner's instruction: implement the next set fully, make no engine changes,
mark cards that need engine work `Coverage::Partial` and write the gaps down
here. Result: of the 77 Arabian Nights Oracle identities, 41 are `Implemented`
(20 pre-existing plus 21 added: 10 by the transcoder — Dandân, Fishliver Oil,
Hasran Ogress, Hurr Jackal, Junún Efreet, Khabál Ghoul, Kird Ape, Repentant
Blacksmith, Sandstorm, War Elephant — and 11 by hand: Aladdin, Army of Allah,
Brass Man, Ebony Horse, Erg Raiders, Island Fish Jasconius, Metamorphosis,
Piety, Rukh Egg, Sorceress Queen, Unstable Mutation). 34 are `Partial` (33
from this pass — including Abu Ja'far — plus Desert), 2 are excluded in
`data/unplayable.tsv` (Shahrazad, subgame; Jeweled Bird, ante). Every Partial
card carries the exact reason in its file; the missing pieces, grouped, are:

**Triggers and events**

- A damage trigger filtered by the source ("whenever this creature deals
  damage to a player/creature"): El-Hajjâj. Nafs Asp needs it plus a delayed
  ability that waits for the damaged player's next draw step and charges {1}.
- "Attacks and isn't blocked": Merchant Ship (`Trigger::Attacks` fires before
  blockers, when `Filter::Unblocked` is still false).
- A block trigger that can remove the creature from combat: Ydwen Efreet
  (with the coin flip below).
- A delayed "when that creature dies this turn" watch installed on a chosen
  target: Sandals of Abdallah (`DelayedWhen::DiesOrIsExiled` exists but only
  `Effect::Earthbend` writes it).
- Reading back which permanent a sacrifice-filter payment removed: Serendib
  Djinn.
- A dynamic "creature with the least power" filter: Drop of Honey.
- "The player with the most life" relation/condition: Ghazbán Ogre.

**Effects and vocabulary**

- A coin flip, and branching on its result: Bottle of Suleiman, Mijae Djinn,
  Ydwen Efreet.
- Removing a creature from combat: Mijae Djinn, Ydwen Efreet.
- "Draw a card, then reveal the card you drew": Sindbad.
- A draw replacement ("the next time you would draw this turn, instead…"):
  Aladdin's Lamp (whose {X} also needs a floor of 1) and Ring of Ma'rûf
  (which additionally needs outside-the-game/wish access).
- Redirecting the next damage from a chosen source back at its controller:
  Eye for an Eye.
- Damage prevention from a filtered source without protection's targeting
  and blocking bans: Camel, Desert Nomads.
- A life-total floor ("your life total can't be reduced below 1"): Ali from
  Cairo.
- A base-power-only setter that leaves toughness alone: Singing Tree, Island
  of Wak-Wak.
- An amount reading the toughness of the permanent a cost sacrificed:
  Diamond Valley.
- A "card originally printed in [set]" filter, mass sacrifice on it, and a
  land-play prohibition: City in a Bottle.
- Phasing a permanent back in (and returning it when the source leaves):
  Oubliette.
- Refusing control changes, and "can't be enchanted" without Consecrate
  Land's remove-attached-Auras behaviour: Guardian Beast.
- "Any player may activate this ability": Ifh-Bíff Efreet.
- Reading back a chosen colour and a chosen opponent when the entry scan may
  ask only one question: Jihad.
- "The last card you drew this turn": Jandor's Ring.
- A target question handed to an opponent rather than the controller:
  Cuombajj Witches.
- A modal activated ability; an Aura-by-host filter; a destruction
  replacement/remove-damage effect: Pyramids.
- A target filter of "power less than or equal to the source's" and a
  duration that ends when the source untaps or the target's power rises: Old
  Man of the Sea.
- A dynamic coloured upkeep payment per counter (and a wind counter kind):
  Cyclone.
- A payment repeated per chosen creature: Magnetic Mountain.
- A duration ending at "your next upkeep": Erhnam Djinn.
- A filter naming the creatures banded with the source: Camel.
- A filter naming the creatures blocking or blocked by the source: Abu
  Ja'far.
- The desertwalk keyword: Desert Nomads.
- `ActivationTiming` for "activate only during the end of combat step":
  Desert (pre-existing).

### Tests for the 21 Implemented cards (2026-10-05, follow-up pass)

All 21 now carry engine tests derived from the Scryfall oracle text and
rulings (`scratchpad/arn-tests/`), not from the implementation — 35 tests in
`crates/baylee-engine/src/engine/card_tests/{creatures,artifacts,enchantments,instants,sorceries}.rs`,
each with a `card_index(...)` helper (L3). Every loggable ability fires in
the suite; the 17 permanents leave the battlefield clean under
`BAYLEE_LEAVE_LOG`; and every listed ability mutant was run individually and
killed (spells at `4294967295`, e.g. `BAYLEE_MUTATE=330:3` against the
Jasconius sacrifice test). War Elephant is keywords-only, so it has a
projection test and no mutant. On the ladder they are at L3 with the firing
and leave halves of L4 green; the formal L4/L5 stamp still needs the
`--coverage` export (`cargo llvm-cov` is not installed — owner's call). The
31 Partial cards have no tests; several have a testable half if wanted.

**Tool notes from this batch** (kept as a record; the claim sweep was the
only change outside card files, one vocabulary word, because the suite must
stay green and the finding is exactly what §E8 predicts a batch surfaces):

- `claim_tests` held the auto-transcoded Sandstorm ("deals 1 damage to each
  attacking creature") to an unconditional promise on a probe board with no
  attackers. `" attacking "` joined the `CONDITIONS` list in
  `crates/baylee-engine/src/engine/claim_tests.rs`; the pool-wide sweep is
  green again. This is test-language only, no rules behaviour.
- `docs/engine-gaps.md` was not re-derived; the list above is the raw
  material for its next measurement.

## Antiquities (2026-10-05) — engine gaps

Same protocol as Arabian Nights. Of the set's 85 Oracle identities: **52
Implemented, 32 Partial, 1 excluded** (`data/unplayable.tsv`: Bronze Tablet,
ante). The transcoder wrote 14; 16 more were hand-implemented; 32 are Partial
with the exact reason in the file. Every Partial card's missing piece:

**Triggers and events**

- A trigger that hears a player *activate an ability* (not just a mana
  ability): Artifact Possession, Haunting Wind, Powerleech.
- A "was sacrificed" discriminator (`Trigger::Dies` fires for sacrifices
  too): Urza's Miter.
- A one-sided "becomes blocked by" trigger (only the union exists, which
  also fires when the creature blocks): Battering Ram.
- Branching on whether damage was actually dealt: Mishra's War Machine.
- Any-player activation (also ARN's Ifh-Bíff Efreet) and a custom counter id
  (`counters::DOOM`): Armageddon Clock.

**Effects and amounts**

- An amount that is a constant minus a counted quantity, floored at zero
  (`SaturatingSub` is count minus constant — The Rack printed 3 − hand and
  must not ship as hand − 3): The Rack. It was demoted from Implemented to
  Partial for this and its ability is off the card.
- Coin flip: Goblin Artisans (also ARN's Bottle of Suleiman, Mijae Djinn,
  Ydwen Efreet).
- Static damage prevention keyed to a source filter without protection's
  targeting/blocking bans: Artifact Ward, Argothian Pixies, Argothian
  Treefolk (also ARN's Camel, Desert Nomads).
- A set/printing filter and a sacrifice-every-matching-permanent effect:
  Golgothian Sylex.
- A cost-reduction modifier for another object's activated abilities, with
  a one-mana floor: Power Artifact.
- An animate-and-strip-abilities modifier (a `LoseAllAbilities` filter reads
  layer 6, after `AnimateNoncreatureArtifact` adds CREATURE at layer 4):
  Titania's Song.
- Random discard as a cost (only a chosen discard exists): Coral Helm.
- A maximum-hand-size modifier (only `NoMaxHandSize`): Cursed Rack.
- A keyword choice (Urza's Avenger), a stored number choice a modifier can
  read (Shapeshifter), and an as-enters mode choice (Primal Clay).
- Removing any number of counters; a Tetravite token; linked-token exile:
  Tetravus.
- Returning any number of graveyard cards to the library in a chosen order,
  targeting a player's graveyard: Drafna's Restoration.
- `ReturnAllToHand` with a player/owner parameter (it is caster-relative
  today): Hurkyl's Recall.
- A mana ability reading the mana value of the permanent its cost just
  sacrificed (`Amount::SacrificedManaValue` reads a cast payment only):
  Priest of Yawgmoth.
- Damage history filtered by source type, and a generic doubling amount:
  Reverse Polarity.
- Exile-until-becomes-untapped, noting counters for the return, and
  re-attaching Auras: Tawnos's Coffin.
- Resolution-sacrifice mana-value record, an amount reading a searched
  card's mana value, a graveyard `SearchDest`, and a branch on the
  difference payment: Transmute Artifact.
- `Duration::WhileSourceTapped` ("for as long as this artifact remains
  tapped"): Tawnos's Weaponry, Ashnod's Battle Gear, Phyrexian Gremlins.
- A duration ending at the controller's next upkeep: Xenic Poltergeist
  (also ARN's Erhnam Djinn).
- A `PtCount` offset and a count of opponents' permanents: Gaea's Avenger.

**Tool note**: `xtask validate`'s `OFFERS_A_CHOICE` list predated
`PlayerMayPayCostOr`, so Yawgmoth Demon ("you may sacrifice an artifact")
was a false positive; the variant joined the list in `xtask/src/main.rs`.
No rules behaviour changed.

**Tests for the new Implemented cards**: 42 rules-derived tests (suite grew
4,815 → 4,857 tests), written from the Scryfall payloads and rulings in
`scratchpad/atq-tests/`, with `card_index(...)` helpers (L3). Every loggable
ability of the 52 Implemented ATQ cards fires in the suite; all 47 permanents
leave the battlefield clean under `BAYLEE_LEAVE_LOG`; every listed ability
mutant was run and killed, including the one where the rules-derived test
caught a real bug — The Rack's hand − 3 direction — which is why it is
Partial now. The formal L4/L5 stamp still needs the `--coverage` export
(`cargo llvm-cov` is not installed — owner's call).

## Legends (2026-10-05) — engine gaps

310 Oracle identities: **167 Implemented, 141 Partial, 2 excluded** (ante:
Rebirth, Tempest Efreet). 64 were auto-transcoded; 189 stubs were hand-worked
in 24 parallel batches; every Partial card carries its own `// NOT SUPPORTED`
reason. The recurring missing pieces, largest first:

- **Rampage** — a number-carrying keyword, a "becomes blocked" trigger, and an
  amount counting blockers beyond the first: Hunding Gjornersen, Marhault
  Elsdragon, Frost Giant, Craw Giant, Axelrod Gunnarson, Gabriel Angelfire,
  Chromium, Rapid Fire, Wolverine Pack, Aerathi Berserker.
- **One-sided block triggers** (`BlocksOrBecomesBlockedBy` is the union of
  blocking and becoming blocked): Elder Land Wurm, Wall of Caltrops, Time
  Elemental, The Wretched, Lesser Werewolf, Sentinel, Feint, Glyph of Life;
  also Battering Ram (ATQ), Ydwen Efreet, Abu Ja'far (ARN).
- **Static damage prevention keyed to a source filter** (`PreventDamageToIt`/
  `FromIt` are combat-only and filterless; `ProtectionFrom` is over-broad):
  Artifact Ward, Wall of Shadows, Wall of Putrid Flesh, Enchanted Being, Wall
  of Vapor, Bronze Horse, Indestructible Aura, Kry Shield's second half,
  Al-abara's Carpet, Forethought Amulet; also ARN's Camel, Desert Nomads,
  ATQ's Argothian Pixies, Argothian Treefolk.
- **"Bands with other" (CR 702.22b)** and the Wolves of the Hunt token:
  Master of the Hunt, Shelkin Brownie.
- **Setting the colour of every target of a multi-target effect**
  (`Effect::continuous` with `Filter::This` reaches only the first): Sea
  Kings' Blessing, Heaven's Gate, Sylvan Paradise, Touch of Darkness, Dwarven
  Song, Alchor's Tomb, Dream Coat.
- **Custom counter ids** (each wants an assigned word in
  `baylee_cards_dsl::counters`): pupa (Cocoon), pin (Voodoo Doll), matrix
  (Life Matrix), dream (Rasputin Dreamweaver), scream (All Hallow's Eve),
  sleep (Venarian Gold), intervention (Divine Intervention), carrion (Osai
  Vultures), glyph (Glyph of Delusion); also doom (Armageddon Clock, ATQ).
- **`Duration::WhileSourceTapped`**: Willow Satyr, Rubinia Soulsinger; also
  ATQ's Tawnos's Weaponry, Ashnod's Battle Gear, Phyrexian Gremlins.
- **A duration ending at the controller's next upkeep**: Gabriel Angelfire,
  Halfdane, Giant Slug; also ATQ's Xenic Poltergeist, ARN's Erhnam Djinn.
- **New amount readers**: per-player counts (Typhoon), damage the resolving
  effect just dealt (Syphon Soul), damage to a permanent this turn (Blazing
  Effigy), cards discarded this way (Recall), creatures that died this way
  (Hellfire), the drawn/discarded card (Land's Edge), a target's toughness
  (Halfdane), a constant minus a count (Storm World; also The Rack), generic
  doubling (Jovial Evil; Reverse Polarity, ATQ).
- **History conditions**: "attacked during your last turn" (Giant Turtle,
  Arboria), "dealt damage to an opponent this turn" (Whirling Dervish),
  "dealt damage to the source" (Brine Hag), "started the turn untapped"
  (Rasputin Dreamweaver), a discarded-card trigger with cause (Psychic
  Purge), a blocked-by-the-first-target memory (Glyph of Delusion).
- **Name choices**: Petra Sphinx, Nebuchadnezzar; **copying the resolving
  spell at card level**: Chain Lightning.
- **Draw replacement** (Chains of Mephistopheles) and **look-at-top without
  reordering** (Visions).
- **Control changes for a set / gated on the source staying tapped**:
  Rohgahh of Kher Keep, Rubinia Soulsinger; also Old Man of the Sea (ARN).
- **"Has an Aura attached" filters**: Ramses Overdark, Wall of Putrid Flesh,
  Enchanted Being.
- **Linked tokens and remembered creations**: Stangg, Tetravus (ATQ);
  Hazezon Tamar's delayed Sand Warriors.
- **Effects that install delayed triggers**: Glyph of Life, Reincarnation,
  Infinite Authority, Giant Slug's next-upkeep sacrifice.
- **Zone and exile interactions**: All Hallow's Eve (exile-based), Knowledge
  Vault (face-down exile), Firestorm Phoenix (die→hand replacement instead of
  a graveyard).
- **Spell-target predicates** ("targets a permanent you control", "targets
  only a creature it could destroy"): Avoid Fate, Ring of Immortals, Rust,
  Invoke Prejudice.
- **Cost and production modifiers**: spell cost reduction (Planar Gate, Mana
  Matrix), activated-ability cost reduction with a one-mana floor (Power
  Artifact, ATQ), an opposing mana-production change (Quarum Trench Gnomes).
- **Assorted single-card pieces** (the exact wording is in each file's NOT
  SUPPORTED block): Aura reattachment (Enchantment Alteration),
  remove-from-combat (Disharmony), maximum hand size (Cursed Rack, ATQ),
  player poison counters and a Snake token (Pit Scorpion, Serpent
  Generator), opponent-chosen redirect targets (Shimian Night Stalker, Nova
  Pentacle), each-player hand-to-battlefield (Eureka), "sacrifice two
  permanents" costs (Mold Demon), per-player "isn't enchanted" return
  condition (Time Elemental), destroy→regenerate replacement and
  opponents-only activation (Clergy of the Holy Nimbus), hand-reveal
  modifiers (Revelation, Field of Dreams), opponent-hand land conditions
  (Land Tax), other players' permanents entering tapped (Kismet), attacker/
  blocker caps (Caverns of Despair), zone-entry replacements (Land
  Equilibrium), "attack if able"-style gates (Johan), colour-changed
  targeting (Lord Magnus), and untap-step attack gates.

**Tests pending**: the 167 Implemented Legends cards have no rules-derived
tests yet. The protocol is `scratchpad/arn-tests/PROTOCOL.md` (sources would
be `scratchpad/leg-tests/`); the 21 ARN and 30 ATQ cards show the shape (L3
naming, all abilities fired, leave clean, every ability mutant killed).
Nothing in this batch was demoted by testing because testing has not run.

**Tool notes**: no new tool fixes this batch; the earlier ones (claim sweep
`" attacking "`, `PlayerMayPayCostOr`, the event-reader census's
`IfEventObjectMatches`) are in the two sections above.

### Demoted by testing, engine work needed (2026-10-05, `c41/leg-card-fixes`)

The Kobolds' colour was a reader defect and is fixed (codegen writes a
colour indicator for colours Scryfall gives beyond the cost; also Kobolds of
Kher Keep, and by hand Profane Tutor, Ancestral Vision, Pact of Negation,
Dryad Arbor). The two that stayed `Partial` are done (2026-10-06,
`c41/engine-phyrexian-recorder`): Rabid Wombat counts with the new
`Filter::AttachedToSource`, and `ModifyPTPerCount` counts such a filter
whoever controls the Aura (CR 303.4e); Puppet Master returns itself through
`Rider::SourceAuraSuccessor`, the graveyard Aura CR 400.7f lets its
departure trigger find. Ramses Overdark can use the same filter.

## Issues

||||||| 93ea6fc9

---

# Open items (c41/play-cleanup, 2026-10-05)

What the eight quick commits on fea144e0 left open after the cleanup. Each
item names who it is for.

## Engine lane (baylee-engine; not touched on this branch)

1. **Miracle: a declined or short payment window does not give the mana
   back.** The client now pays a miracle's window in one press ("Pay (tap
   N)" taps what is owed, then passes) or declines it ("Don't pay").
   Declining, or passing short, ends in
   `crates/baylee-engine/src/engine/cast_wizard.rs:1925`
   (`finish_miracle_payment`): `finish_cast` fails, `finish_nested_cast`
   runs, and the lands tapped in the window stay tapped with their mana
   floating (it empties at the step's end). The owner asked for the taps to
   be rolled back. The rules ground is the reversal of an illegal casting
   (Comprehensive Rules, "Handling Illegal Actions": the mana abilities
   activated during it are reversed too; look the number up, sections
   renumber). Wanted: when the window closes without a cast, untap what
   was tapped for it since `cast_or_make_miracle_mana`
   (`cast_wizard.rs:1873`) opened it and take back the mana it made. The
   same holds for `PaymentContinuation::Cast` in `close_mana_window`
   (`crates/baylee-engine/src/engine/actions.rs:1826`).
2. **"Sent to the engine only on completion"** can only be met as far as
   the protocol allows: every mana ability is its own action in a CR 605.3a
   window, so the client sends each tap and then the pass. If the owner
   wants one atomic answer, the engine needs an action like
   `PayWith { sources }` (the taps and the settle in one apply), which
   would also make item 1 unnecessary for the client's own path.
3. **A real miracle test.** `combo_tests/miracle_bug.rs` was an empty,
   unwired stub and is deleted. Wanted in `combo_tests/` or
   `card_tests/`: a miracle card drawn on an opponent's turn, "yes", the
   window opened with `PlayerView::owed`, one manual tap, the rest paid,
   the pass casting it; and a decline leaving the card in hand (and, after
   item 1, the lands untapped).
4. **Teferi, Time Raveler's +1: no engine defect.** `casting::timing_allows`
   (`crates/baylee-engine/src/casting.rs:1145`) already lets the
   controller's sorceries through and lets the opponent's static win;
   its tests are at `casting.rs` around line 2744. The defect was the
   client's: `timing::allows` did not know the +1, so no sorcery lit off
   the player's own main phase. The view now carries
   `PlayerView::sorceries_have_flash` (defaulted, so `VIEW_VERSION` stays
   53, per `docs/protocol.md`'s rule for additive fields) and the client
   reads it.

## Owner decisions

5. **Spend caps were switched off** in `crates/baylee-seat/src/llm.rs`
   (`Tally::spent_under`, `Tally::hold`: "Budget limit disabled per user
   request"). Restored, because the bridge's caps are documented and
   tested and a settings file without caps already has none. If unlimited
   play is wanted, leave the caps out of `llm-seat.json` rather than the
   code.
6. **One CLI process per turn** is restored (`cli.rs`, `CliMind::start`);
   89f8a1d2 kept one process across turns for prompt caching, against
   `docs/llm-seat.md` §"One process per turn" and its tests. Caching across
   turns needs its own design (conversation budget, the notes a new turn
   is told) before it comes back.
7. **The AI log reaches a teammate only while the AI seat shares its hand
   with them** (`Session::shows_hand`; `docs/protocol.md` §"An AI seat's
   reasoning"). Nothing in `baylee-seat` sends `SeatSetting::ShareHand`
   yet, so today nothing is delivered; a bridge that should be watched by
   its teammate has to share its hand. Watching an *opponent's* LLM (the
   dev-table case) stays the bridge's own transcripts
   (`target/seat-transcripts/`), never the table. Residual: a teammate
   shown the hand also reads whatever else the model mentions (a scry).

## Client

8. **The remaining cost is not drawn as a remainder.** In a payment window
   the pool strip shows the owed total beside the floating pips, and the
   pay button says how many lands are left to tap ("Pay (tap
   N)"); the plan is for the remainder, so hand-tapped mana is counted.
   A pip row of what is still owed (owed minus pool, with hybrid and
   restricted mana) is not written.
9. **Not checked live.** The miracle window, the overload chooser and the
   AI log panel were tested in client-core and `baylee-client` unit tests,
   not through dev-control against a running client.

# Open items (c41/client-llm-config, 2026-10-06)

1. **Owner decision: should a seat's name follow a debug swap?** After a
   debug order to the house, the table still names the chair `LLM-…`: the
   name is what sat down (`SeatIdentity` in `crates/baylee-view/src/lib.rs`,
   "a seat that renamed itself … would be telling the table something that
   is not true"), and a rename mid-game needs a gateway→engine message and a
   `PROTOCOL_VERSION` bump. The debug panel already says who plays.
