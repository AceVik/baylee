//! Machine-command fixture; real JSON validation is tested by baylee-server-json.
/// Compact responses supplied by the deploy/reboot machine fixtures.
pub const BODY: &str = r#"read -r input || exit 1
case "$1" in
open-games|health)
    field=running
    [ "$1" = open-games ] && field=$2
    pattern=\"$field\":'([0-9]+)'
    [[ $input =~ $pattern ]] || exit 1
    running=${BASH_REMATCH[1]}
    [[ $input =~ \"waiting\":([0-9]+) ]] || exit 1
    waiting=${BASH_REMATCH[1]}
    if [ "$1" = health ]; then
        admission=missing
        [[ $input =~ \"admission\":\"([^\"]+)\" ]] && admission=${BASH_REMATCH[1]}
        echo "$running $waiting $admission"
    else echo "$running $waiting"; fi
    ;;
info)
    [[ $input =~ \"commit\":\"([0-9a-f]{40})\" ]] || exit 1
    commit=${BASH_REMATCH[1]}
    dirty=unknown
    [[ $input =~ \"dirty\":false ]] && dirty=clean
    [[ $input =~ \"dirty\":true ]] && dirty=dirty
    echo "$commit $dirty"
    ;;
*) exit 1 ;;
esac"#;
