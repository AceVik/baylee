# Lobby, earned insignia and the Oak Tower

Owner request, 13 November 2026: rebase both repositories, replace the challenge
presentation with a composed lobby, individual escalating animated insignia,
then fully implement a manually played tower with deck improvements between
stages. At least five genuine improvement passes for each part. Test only changed
areas; retain evidence and review the final implementation in the existing PRs.

## Presentation acceptance

- The Lobby is the signed-in home, independent of selecting a gateway. Daily and
  Weekly have equal visual weight, aligned baselines and one clear action each.
- Long rules belong in task details. Connection diagnostics belong in the
  gateway popover; the home has no large gateway information panel.
- Five insignia have distinct original motifs, not a recoloured generic medal.
  Bronze, silver, gold, platinum and obsidian add silhouette, relief, inlays,
  framing and controlled motion. Locked insignia remain recognisable.
- Motion respects reduced motion and graphics preferences. Shader time is the
  shared GPU clock, asset counts are bounded, idle UI trees do not rebuild.
- Keyboard focus, confirmation, loading, failure, archive, resume and privacy
  controls remain complete. German/English, desktop minimum and wide layouts
  must be inspected in the real client.

## UI iteration ledger

1. Composition: lobby entry, balanced task cards, compact multiplayer entry.
   Implemented the signed-in home, paired cards and a five-column progress
   gallery. Removed the gateway diagnostics card and compacted connection access.
   Seven focused challenge UI tests pass, including home → connections → home
   before gateway admission and the existing keyboard/consent/account isolation.
2. Identity: five bespoke insignia, five escalating material tiers.
   Added procedural sun, crescent, compass, true-strike and oak motifs, with
   engraved silver bezels, gold laurels, platinum jewels and an obsidian crown.
   Naga validation and all thirty motif/tier cache identities pass. Motion reads
   the shared GPU clock and follows both reduced-motion and effects settings.
3. Interaction: composed details, scoreboards, progress and contextual actions.
   Task details and ranking now form two balanced columns on a wide screen.
   Long scoring/reward rules expand explicitly; privacy consent stays visible.
   Six challenge state tests and seven native UI tests pass. The first real
   1440×900 render confirmed equal task widths and aligned actions; all five
   earned bronze motifs render through the native Metal pipeline. The test
   client was closed and its owned PID and harness port verified gone.
4. Robustness: responsive layout, localisation, keyboard and state transitions.
5. Finish: rendered refinement, motion/counter-test, performance and final QA.

Each entry is completed only after an actual change and focused verification;
follow-up rounds are added if the rendered result still needs work.

## Tower direction and invariants

An original Oak Tower expedition: the player fights each duel, chooses a bounded
deck upgrade between floors, reaches distinct milestones and bosses, and keeps
permanent cosmetic progress. The same centrally published expedition is shared
across gates. Ranking authority, run ownership and rewards live in Central;
clients cannot report wins, mint upgrades or submit authoritative scores.

Use existing deterministic engine, certified card pools, deck construction,
authenticated execution, durable result delivery and ordinary game controls.
No copied Oaken Tower art, text or code. No new third-party dependency or custom
cryptography is needed. Failure, retry, resume, seasonal closure and concurrent
upgrade choices must have explicit states before implementation.

## Tower iteration ledger (after UI passes)

1. Rules and authority: bounded run model, progression and security invariants.
2. Play: certified encounters, real engine launch and durable floor outcomes.
3. Choices: meaningful deck improvements, milestones and earned rewards.
4. Presentation: tower map, encounters, upgrade selection and rankings.
5. Adversarial/play refinement: replay, duplicate results, resume and difficulty.

## Evidence

Both worktrees were clean before rebasing. Central already contained current
`origin/main`; Baylee's 24 feature commits rebased without conflicts. Existing
local demo services had exited before this work began.
