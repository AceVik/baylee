"""CLI and compatibility imports for the supervised v4 trainer."""
from training4.driver import main
from training4.loss import compute_step_loss
from training4.validation import evaluate

if __name__ == "__main__":
    main()
