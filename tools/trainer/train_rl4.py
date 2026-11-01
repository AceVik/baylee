"""Compatibility entry point for the recurrent PPO trainer."""
from recurrent_ppo4.driver import main, compute_reverse_gae, action_logprob_and_kl

if __name__ == "__main__":
    main()
