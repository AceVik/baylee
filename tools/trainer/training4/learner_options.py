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
    parser.add_argument("--window-state-cache", default="",
                        help="1c: reuse/write the initial H pass of this exact plan and --init checkpoint")
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
    if args.phase_profile and (args.phase_profile_warmup < 0 or args.phase_profile_steps < 1):
        parser.error("Phase profile needs nonnegative warmup and positive steps")
