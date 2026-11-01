"""Opt-in loss CLI and training-only readout detachment; no model state keys."""
from .loss_config import SupervisedLossConfig


def add_options(parser):
    parser.add_argument('--loss-version',type=int,choices=(4,5),default=4)
    parser.add_argument('--loss-normalization',choices=('legacy_timestep','unroll_per_head'),default='legacy_timestep')
    parser.add_argument('--outcome-weighting-off',action='store_true')
    parser.add_argument('--detach-value-input','--detach-value',action='store_true')
    parser.add_argument('--detach-aux-input','--detach-aux',action='store_true')


def configuration(args, parser):
    try:
        config=SupervisedLossConfig(version=args.loss_version,outcome_weighting=not args.outcome_weighting_off,
            normalization=args.loss_normalization,detach_value_input=args.detach_value_input,detach_aux_input=args.detach_aux_input)
        config.validate_execution(reference_execution=args.reference_execution, init_state=args.init_state)
    except ValueError as error:
        parser.error(str(error))
    return config


def auxiliary_readout(net, features, config):
    return net.aux_head(features.detach() if config.detach_aux_input else features)
