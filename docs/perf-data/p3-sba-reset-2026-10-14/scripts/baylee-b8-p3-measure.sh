set -euo pipefail
source /tmp/baylee-b8-p3-common.sh
quiet_machine
mkdir "$OUT/catalogue"
clean env CRITERION_HOME="$OUT/catalogue/A" "$OUT/A/basics" --bench --list \
    > "$OUT/catalogue/A.txt" 2> "$OUT/catalogue/A.log"
clean env CRITERION_HOME="$OUT/catalogue/B" "$OUT/B/basics" --bench --list \
    > "$OUT/catalogue/B.txt" 2> "$OUT/catalogue/B.log"
cmp "$OUT/catalogue/A.txt" "$OUT/catalogue/B.txt"
rg 'combat/attack_to_blocks_(100|900)' "$OUT/catalogue/A.txt"

FOCUS='^combat/attack_to_blocks_(100|900)$'
measure A focus-1-A1 30 1 3 "$FOCUS"
measure B focus-1-B  30 1 3 "$FOCUS"
measure A focus-1-A2 30 1 3 "$FOCUS"
measure A focus-2-A1 30 1 3 "$FOCUS"
measure B focus-2-B  30 1 3 "$FOCUS"
measure A focus-2-A2 30 1 3 "$FOCUS"

# Survey, not a second high-confidence performance claim for every row.
measure A survey-A1 10 0.5 1
measure B survey-B  10 0.5 1
measure A survey-A2 10 0.5 1
