# BayleeAI thinking: deliberation over the answers

Design note, 10 November 2026, written before the build. Status of each part
is in §7; nothing here changes a default until it is measured (§6).

The owner wants a net that thinks, corrects itself and spends more thought on
hard decisions. Today's thinking steps K do nothing measurable (K = 0, 1, 4, 8
within noise of K = 2 in play). This note says why, what replaces them, how it
is trained and what it costs.

## 1. Why today's thinking is inert

Shown from the code (`tools/trainer/model4/player.py`, `layers.py`,
`training4/`) and from one CPU measurement on r1.

1. **The steps never see an answer.** `ThinkingStepBlock` updates nine summary
   slots `z` (one global, eight seats) by cross-attention into the board
   tokens (`player.py`, the `for k in range(max_k)` loop). It has no input
   from the candidates, from the logits of an earlier step, or from the value.
   There is no proposal in the loop, so there is nothing to reconsider.
2. **The answers barely see the steps.** A candidate is scored from
   `[c_emb, c_ent, c_seat, g_ref]` (`candidate_hidden`). `c_ent` comes from
   `e_ref = board[:, RECURRENT_SLOTS:]`, the trunk's entity tokens, which no
   thinking step touches. Only `g_ref` and `c_seat` come from `z`, and `g_ref`
   is the same vector for every candidate of a decision. Thinking can shift a
   whole menu, hardly one answer against another. (r1 is candidate revision 1;
   revision 3's decoder reads `z_out` as nine of its keys, still one way.)
3. **The step repeats what the trunk did.** The nine slots already attended to
   every board token in each of the trunk's 12 layers. One more cross-attention
   from the same slots to the same tokens is a thirteenth layer for nine
   tokens, with shared weights at every step.
4. **Training never rewarded a step.** The loss reads the logits after the last
   step only (`compute_loss_v5`, `prepared[t][1]`); `DepthSchedule` is fixed at
   `positive_k = (2,)`. No step is supervised on its own, nothing compares step
   k with step k - 1, and no depth other than 2 was ever trained (the
   `legacy_frame_cycle` curriculum trains one depth per frame, again with no
   order between depths). The net was never asked to be better at K = 4 than at
   K = 2, so it is not.
5. **Measured** (`tools/trainer/thinking_probe5.py`, r1-4eb, gen4 validation
   shard, 272 decisions with two or more answers, same memory and board at
   every depth): against K = 2 the top answer differs in 2.2 % of decisions at
   K = 0, 1.1 % at K = 1, 1.1 % at K = 4, 1.5 % at K = 8. The scratchpad itself
   moves a lot (K = 0 against K = 2: 129 % of its norm), the largest logit
   change is 0.08-0.59, and the choice stays. Three to six answers in 272
   differ: that is the whole lever the K sweep measured, which is why K = 1, 4,
   8 came out at +0.4, +1.3 / -1.2, +2.1 / -1.1 pp, all across zero.

So: the steps cannot see their proposal or its consequences (1, 2), have
nothing new to compute (3), and were never rewarded (4). All three of the
suspected causes hold.

## 2. What thinking should be for this net

Four parts, in the order they are worth building. The first two are built now.

**A. Deliberation over the answers (built).** A small block with shared
weights that runs S times after the heads have produced their first logits:

- Tokens: one decision token and the M = 16 best candidates by the net's own
  first logits (a fixed shape: the mean menu has 4.9 answers; a card-naming
  menu of thousands keeps its first logits outside the best 16).
- Each step reads its **previous proposal** per candidate: probability as a
  single choice and as a subset member, distance to the best logit, whether it
  is the top answer, how its probability moved in the last step, and the
  **predicted consequence** q of taking it (below). The decision token reads
  the top probability, the margin, the entropy and its own last confidence.
- Self-attention across the candidates (answers are compared with one
  another, which revisions 1-3 never do), cross-attention from each candidate
  into the thought-through slots and every entity (it looks at the board again
  with the proposal in hand), feed-forward.
- Outputs per step: a **correction** to each candidate's logit (logits after
  step s = logits after s - 1 plus the correction), a consequence estimate q
  per candidate, and a **confidence**: the probability that the current top
  answer is the right one.
- The correction's output layer starts at zero, so a net with the block
  attached plays exactly as its source until trained (the same rule as
  `cand_decoder` and the relational block).

What it sees of consequences and legality: every candidate is legal by
construction (the engine's menu); the candidate features carry what the
encoder knows about each answer (Encoder 10's target side; Encoder 12 is
planned to carry the look's reads as columns and edges, and the block reads
them without change). q is the learned consequence: the win chance if this
answer is taken, trained from outcomes now and from search values later.

**B. Adaptive depth (built).** The confidence is the halting signal. Because a
step is cheap against the trunk (§4), a seat does not save much by stopping
early inside the block; the signal is used where thought is expensive:

- seat: play the shallow graph; when its confidence is below a threshold, ask
  the deep graph (`--think-adaptive LO,HI,TAU`); the share of escalated
  decisions is the thinking budget and is reported;
- later, search: only decisions the net itself calls unsure are searched
  (the v6 proposal's "Bedenkzeit nach Bedarf", today a hand rule over 21 % of
  decisions).

**C. Self-correction by training (built), §3.**

**D. Internal lookahead over the opponent's answers (not built; design).**
From public information only: a learned opponent reader (the v6 proposal's
Gegner-Leser) gives a distribution over the opponent's likely answers to each
of my candidates; the block gets one more token set (my candidate x their
likely answer) and q becomes an expectation over them. Its teacher is the
engine search with fair worlds (hidden cards dealt from a public prior, never
the opponent's list). It needs the reader and the fair world model first; the
q head and the token interface built now are where it attaches. Blocker pairs
join the block in the same step (they are 1-2 pp today).

Not kept: more scratchpad steps. The legacy scratchpad stays as trained (K = 2
for r1) because the trunk's heads read it; the runtime depth K now counts
deliberation steps.

## 3. Training so that extra steps matter

Three terms beside the ordinary policy loss on the final logits
(`training4/think_loss.py`):

1. **Deep supervision**: the policy loss at every step s = 1..S, not only the
   last, with the same targets and weights.
2. **Improvement**: `relu(loss_s - stopgrad(loss_{s-1}) + margin)`: a later
   step is penalised when it is worse than the one before. With (1) this makes
   depth monotone by construction of the objective, not by hope.
3. **Confidence**: binary cross-entropy of each step's confidence against
   "this step's decoded answer equals the target". That is what makes the
   halting signal a calibrated statement about the net's own answer.
4. **Consequence** (small weight): q of the target answer against the game's
   outcome; with search data, q of every searched answer against its searched
   value.

The proposal a step reads is detached: a step is trained to correct what it is
given, not to shape what the previous step hands over.

**Stage A (priority into the policy by DAgger).** The decided stage A trains
the candidate head or a small adapter on its inputs with the trunk frozen, on
the rule layer's answer R at states the student visits. The deliberation block
is that adapter: `--train-thinking-only` freezes everything else, the trunk's
logits stay r1's, and the block learns the corrections from r1's answer N to R.
This fits the data exactly: the label matters where N != R, which is where the
first proposal is wrong and a correction is needed; where N = R the zero
correction is already right (and the KL-to-r1 term of stage A keeps it there).
The confidence target becomes "my answer equals R", so low confidence marks
the decisions where the net still disagrees with the layer. Selection stays
top-1 at N != R, reported per step, so the measurement shows directly whether
step s beats step s - 1 (§6). The stage A trainer on `c42/v6t-learner` reads
its outputs through `deferred_readouts`; the per-step trace is delivered there
(`net.thinking_frames`), so its distillation loss can add these terms without
a second forward.

**Later, search.** Search gives values for several answers of one decision:
q is regressed on them at every step and the final logits on the search
policy. The confidence then gates which decisions are searched at all, and the
search's changed answers are exactly the N != teacher cases the block learns to
correct, the same shape as stage A.

**Later, RL.** PPO's ratio is taken on the final logits; the deep terms stay as
auxiliary losses with the taken action as target weighted by the advantage's
sign (a step that moves probability toward a better-than-expected action is
improving), and the confidence target becomes "the taken top answer had a
non-negative advantage". Depth is sampled per decision during collection so
every depth has data; the recorded behaviour probability is the played depth's.

## 4. Cost

Estimate before the build, to be replaced by the measurement in §7. The trunk
runs 12 layers of width 640 over 9 + E tokens (E ~ 26 at a decision, up to
several hundred); one deliberation step runs one layer of width 256 over 17
tokens with cross-attention into 9 + E keys whose projections are computed
once per decision. Per step that is on the order of 1 % of the trunk. On the
CPU the K = 2 seat takes ~100-140 ms per decision; S = 4 steps should add a
few milliseconds.

- **CPU seat**: S = 4 by default is affordable; S = 8 as the deep graph. The
  adaptive seat pays a second graph call (trunk included) on escalated
  decisions only; at a 20 % escalation rate that is +20 % mean latency. If
  measurement shows the steps are as cheap as estimated, the seat simply runs
  the deep graph always and escalation is kept for search.
- **XDNA NPU (acenb)**: no execution provider for ONNX Runtime is installed
  there (campaign notes, 18:45: DNNL, XNNPACK and CPU only; installing Ryzen AI
  or VitisAI is the owner's decision), so nothing can be measured on the NPU
  today. What the design does for it: the block is fixed-shape (17 tokens,
  fixed step count per graph, no loop or branch operator, top-16 by `TopK`
  with a constant k); the only dynamic axis it touches is the key count 9 + E,
  which the trunk already has. A split export (trunk graph, think graph) with
  E padded to a fixed size is the NPU form and is a follow-up once an
  execution provider exists.

## 5. Rules kept

No heuristic at play time: the block is a learned sub-network; the top-16 are
chosen by the net's own logits; the escalation threshold compares the net's own
confidence. Hidden cards are never an input: the block reads only tensors the
net already has. House AI only as opponent. No v3/v5 games.

## 6. Measurements that show it helps (plan)

Each is paired, written before it runs, and decides one thing.

1. **Offline, per step** (`thinking_probe5.py --steps`, CPU or GPU): top-1 at
   N != R on held-out tables after each step s = 0..S (0 = r1's own answer).
   Pass: accuracy rises from step 0 to step S and step S beats a one-step block
   of the same size (so depth, not parameters, earns it). Also the confidence's
   ECE and the share of decisions each threshold would escalate.
2. **Ablations, same data, three seeds**: (a) no proposal feedback (the block
   sees candidates and board but not its own last logits), (b) no deep
   supervision and no improvement term (last-step loss only), (c) S = 1. The
   mechanism is "thinking" only if the full block beats (a) and (b) at equal
   steps.
3. **Play, raw, 1v1 against `sharp`**, test and fresh 512, on the stage A
   student: K = 1, 2, 4, 8 deliberation steps paired against K = 0-equivalent
   (the same student with the block's correction off) and against each other.
   Pass: lower bound above zero for the trained depth against no deliberation
   on both splits, and K = 4 not below K = 1. This is the measurement the old
   K sweep failed.
4. **Adaptive**: LO = 1, HI = 4 at thresholds that escalate ~10, 20, 40 % of
   decisions, paired against always-4: non-inferior (lower bound above -1.5 pp)
   at the lowest escalation rate that holds; mean and p90 latency per decision
   on the CPU for each.
5. **Stage A gate unchanged**: at least +8 pp over raw r1 on test and fresh and
   opponent-turn casts well above 0.2 %; the block is compared with stage A's
   plain head/adapter arm on the same DAgger data (paired, three seeds pooled).
   If the plain adapter is as good, the block has not earned its place and is
   dropped from stage A.

GPU runs go through `train_ops pipeline` in the campaign lead's queue.

## 7. Status

Filled in after the build (what exists, tests, measured cost).
