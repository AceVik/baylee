"""Opt-in learner throughput levers; every default leaves the step unchanged."""


def add_learner_options(parser):
    parser.add_argument("--phase-profile", default="", help="Write per-phase step timings to this JSON path")
    parser.add_argument("--phase-profile-warmup", type=int, default=50, help="Steps excluded before the profile window")
    parser.add_argument("--phase-profile-steps", type=int, default=300, help="Steps inside the profile window")
    parser.add_argument("--phase-profile-sync", action="store_true",
                        help="Wait for the GPU at every phase boundary (phase completion time, slower)")
    parser.add_argument("--passive-grad-window", type=int, default=-1,
                        help="1b: only frames within W frames before a decision encode boards and update memory "
                             "with gradient; others run without (R2D2 burn-in). -1 keeps every frame differentiable")
    parser.add_argument("--window-burn-in", type=int, default=-1,
                        help="1c: train only windows [decision-B, decision] from stored H (-1 = off)")
    parser.add_argument("--window-length", type=int, default=32, help="1c: frames per packed lane")
    parser.add_argument("--window-state-lanes", type=int, default=32, help="1c: lanes of the no-grad H pass")
    parser.add_argument("--window-state-refresh", type=int, default=0,
                        help="1c: recompute stored H every N optimizer updates (0 = once per epoch)")
    parser.add_argument("--window-entity-buckets", type=int, default=0,
                        help="1c: pack lanes within entity-count buckets of this width (0 = no bucketing)")
    parser.add_argument("--window-epoch-fraction", type=float, default=1.0,
                        help="1c: stop each epoch after this fraction of its window batches")
    parser.add_argument("--window-state-workers", type=int, default=4, help="1c: loader workers of the H pass")
    parser.add_argument("--window-state-prefetch", type=int, default=2,
                        help="1c: batches each H-pass worker prepares ahead; every one waits in /dev/shm")
    parser.add_argument("--window-state-cache", default="",
                        help="1c: reuse/write the initial H pass of this exact plan and --init checkpoint "
                             "(a JSON key file beside its .states store file and .entities.npy)")
    parser.add_argument("--window-state-seats", type=int, default=8,
                        help="1c: store memory slots 0..N only (global and N seats); a frame with a seat beyond N "
                             "fails. 2 for duels. Default 8 keeps every slot")
    parser.add_argument("--window-state-bf16", action="store_true",
                        help="1c: store H as bf16 (half the bytes; rounds H, so the stored-H parity is approximate)")
    parser.add_argument("--memory-limit-gib", type=float, default=0.,
                        help="Host memory limit for the budget check, e.g. the train_ops stage's memory_gib; it can "
                             "only tighten the cgroup's memory.max (0 = that, else MemTotal)")
    parser.add_argument("--memory-budget-fraction", type=float, default=0.5,
                        help="Refuse a window run whose estimated host memory exceeds this fraction of the limit")
    parser.add_argument("--shm-limit-gib", type=float, default=0.,
                        help="Shared memory this user may hold in /dev/shm, where loader workers hand over "
                             "batches; it can only tighten the tmpfs quota (0 = that quota, else the tmpfs size)")
    parser.add_argument("--shm-budget-fraction", type=float, default=0.5,
                        help="Refuse a window run whose estimated /dev/shm use exceeds this fraction of that limit")
    parser.add_argument("--report-peak-rss", action="store_true",
                        help="Print peak host memory after the window-state pass and at the end")
    parser.add_argument("--train-policy-head-only", action="store_true",
                        help="Stage A: freeze everything but the candidate (priority) head")
    parser.add_argument("--adapter-on-candidate-head", type=int, default=0, metavar="RANK",
                        help="Stage A: freeze everything; train a rank-RANK adapter on the candidate head's input "
                             "layer, merged into its weights whenever a checkpoint is saved")
    parser.add_argument("--trainable-trunk-from-layer", type=int, default=-1, metavar="L",
                        help="Stage B: train trunk layers >= L, the thinking block and the heads; -1 = off")
    parser.add_argument("--freeze-value-head", action="store_true", help="Freeze seat_value_head and aux_head")
    parser.add_argument("--distill-bundle", action="store_true",
                        help="Distil bundle_distillation_v1 labels R: CE to R where N != R at --distill-kinds, "
                             "KL to the frozen --init policy everywhere else")
    parser.add_argument("--distill-kinds", default="0",
                        help="Comma-separated pending kinds distilled to R (0 = priority, 11 = mulligan)")
    parser.add_argument("--distill-kl-weight", type=float, default=1.0, help="Weight of every KL-to-r1 term")
    parser.add_argument("--distill-scalar-tolerance", type=float, default=0.05,
                        help="A scalar agrees with R within this normalized distance")
    parser.add_argument("--distill-subtypes", default="",
                        help="Comma-separated subtypes of --distill-kinds that get CE to R (e.g. "
                             "cast,pass,instant_their_turn,spend); the kinds' other subtypes stay under the KL. "
                             "Empty = every subtype")
    parser.add_argument("--distill-group-weights", default="",
                        help="Comma-separated group=weight over the step's fired rule groups; a CE row's weight "
                             "is the product of its fired groups' weights (others 1). Empty = off")
    parser.add_argument("--distill-out-of-scope", choices=("kl", "ce-n"), default="kl",
                        help="The distilled kinds' out-of-scope single choices: kl = KL to r1 (default); ce-n = "
                             "cross-entropy to r1's top-1 (N), its own term ce_n_out_of_scope, the KL there reported only")
    parser.add_argument("--distill-out-of-scope-weight", type=float, default=1.0,
                        help="Weight of the ce-n term")
    parser.add_argument("--skip-final-validation", action="store_true",
                        help="Throughput probes only: no held-out pass after a --max-steps/time-budget stop")
    parser.add_argument("--vectorized-rule-closure", action="store_true",
                        help="Build the active rule closure's index arrays with numpy instead of a per-node loop")


def check_learner_options(args, parser):
    if args.passive_grad_window < -1 or args.window_burn_in < -1:
        parser.error("Window sizes are -1 (off) or nonnegative")
    if args.passive_grad_window >= 0 and args.window_burn_in >= 0:
        parser.error("--passive-grad-window and --window-burn-in are separate learners")
    for name in ("passive_grad_window", "window_burn_in"):
        if getattr(args, name) >= 0 and (args.decision_chunk <= 1 or args.reference_execution):
            parser.error(f"--{name.replace('_', '-')} requires the deferred path (--decision-chunk > 1)")
    if args.window_burn_in >= 0:
        if args.resume or args.init_state or args.bc_replay_data:
            parser.error("--window-burn-in supports fresh --init runs only")
        if args.window_length < 1 or args.window_state_lanes < 1 or args.window_state_refresh < 0:
            parser.error("Window lanes must be positive and the refresh nonnegative")
        if not 0 < args.window_epoch_fraction <= 1:
            parser.error("--window-epoch-fraction must be in (0, 1]")
        if not 1 <= args.window_state_seats <= 8:
            parser.error("--window-state-seats must be between 1 and 8")
    if args.memory_limit_gib < 0 or not 0 < args.memory_budget_fraction <= 1:
        parser.error("--memory-limit-gib must be nonnegative and --memory-budget-fraction in (0, 1]")
    if args.shm_limit_gib < 0 or not 0 < args.shm_budget_fraction <= 1 or args.window_state_prefetch < 1:
        parser.error("--shm-limit-gib must be nonnegative, --shm-budget-fraction in (0, 1] "
                     "and --window-state-prefetch positive")
    if args.phase_profile and (args.phase_profile_warmup < 0 or args.phase_profile_steps < 1):
        parser.error("Phase profile needs nonnegative warmup and positive steps")
    stages = [args.train_policy_head_only, args.adapter_on_candidate_head > 0, args.trainable_trunk_from_layer >= 0]
    if args.adapter_on_candidate_head < 0 or args.trainable_trunk_from_layer < -1:
        parser.error("Adapter rank must be nonnegative and the trunk layer -1 or nonnegative")
    if sum(stages) > 1:
        parser.error("Choose one of --train-policy-head-only, --adapter-on-candidate-head, --trainable-trunk-from-layer")
    upper = args.layers - args.trainable_trunk_from_layer
    if (args.trainable_trunk_from_layer >= 0 and args.compile_regions and args.activation_checkpointing
            and args.saved_activation_layers < upper):
        # Frozen and trainable layers share one compiled code object; its cache
        # holds a graph per requires_grad and shape case, and on CUDA a backward
        # recompute can run another graph than the forward did.
        parser.error(f"--trainable-trunk-from-layer {args.trainable_trunk_from_layer} with --compile-regions would "
                     "recompute compiled trunk layers in backward, which can run another graph than their forward "
                     f"(a torch.utils.checkpoint metadata mismatch); keep their activations with "
                     f"--saved-activation-layers {upper}")
    partial = any(stages) or args.freeze_value_head
    if partial and (args.freeze_trunk or args.train_candidate_column is not None or args.train_rules_only):
        parser.error("Partial training replaces --freeze-trunk, --train-candidate-column and --train-rules-only")
    if (partial or args.distill_bundle) and (not args.init or args.resume or args.init_state or args.bc_replay_data):
        parser.error("Partial training and --distill-bundle start fresh from --init only")
    try:
        args.distill_kinds = tuple(sorted({int(k) for k in args.distill_kinds.split(",") if k.strip()}))
    except ValueError:
        parser.error("--distill-kinds is a comma-separated list of pending kinds")
    args.distill_subtypes = tuple(sorted({s.strip() for s in args.distill_subtypes.split(",") if s.strip()}))
    weights = {}
    for item in (i.strip() for i in args.distill_group_weights.split(",") if i.strip()):
        name, _, number = item.partition("=")
        try:
            weights[name.strip()] = float(number)
        except ValueError:
            parser.error("--distill-group-weights is a comma-separated list of group=weight")
        if not name.strip() or not weights[name.strip()] >= 0:
            parser.error("--distill-group-weights needs named groups with nonnegative weights")
    args.distill_group_weights = weights
    if (args.distill_subtypes or weights) and not args.distill_bundle:
        parser.error("--distill-subtypes and --distill-group-weights require --distill-bundle")
    if args.distill_out_of_scope != "kl" and not args.distill_bundle:
        parser.error("--distill-out-of-scope requires --distill-bundle")
    if args.distill_out_of_scope_weight != 1.0 and args.distill_out_of_scope != "ce-n":
        parser.error("--distill-out-of-scope-weight weights the ce-n term only")
    if not args.distill_out_of_scope_weight >= 0:
        parser.error("--distill-out-of-scope-weight must be nonnegative")
    if args.distill_bundle:
        if not args.distill_kinds or args.distill_kl_weight < 0 or args.distill_scalar_tolerance < 0:
            parser.error("--distill-bundle needs kinds and a nonnegative KL weight and scalar tolerance")
        if args.decision_chunk <= 1 or args.reference_execution:
            parser.error("--distill-bundle requires the deferred path (--decision-chunk > 1)")
        if args.checkpoint_selection != "distill_n_ne_r":
            parser.error("--distill-bundle selects checkpoints with --checkpoint-selection distill_n_ne_r")
    elif args.checkpoint_selection == "distill_n_ne_r":
        parser.error("--checkpoint-selection distill_n_ne_r requires --distill-bundle")
