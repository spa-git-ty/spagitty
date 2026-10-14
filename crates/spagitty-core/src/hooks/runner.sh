# SPDX-License-Identifier: GPL-3.0-or-later

# This is embedded in each installed hook. Git and its shell tools are the
# runtime: a terminal commit never needs the desktop application.
json() {
    printf '%sX' "$1" | awk '
    { if (NR > 1) s = s "\n"; s = s $0 }
    END {
        s = substr(s, 1, length(s)-1); printf "\""
        for (n=1; n<32; n++) code[sprintf("%c", n)] = n
        for (n=1; n<=length(s); n++) {
            c=substr(s,n,1)
            if (c == "\\") printf "\\\\"
            else if (c == "\"") printf "\\\""
            else if (c in code) printf "\\u%04x", code[c]
            else printf "%s", c
        }
        printf "\""
    }'
}
at() { date -u '+%Y-%m-%dT%H:%M:%SZ'; }
emit() { printf '%s\n' "$1" >> "$RECORD"; }

# POSIX ps on macOS and Linux supplies the fallback when GNU timeout is not
# installed. On Git for Windows the bundled timeout owns the process group.
terminate() {
    for child in $(ps -eo pid=,ppid= 2>/dev/null | awk -v parent="$1" '$2 == parent {print $1}'); do
        terminate "$child"
    done
    kill -TERM "$1" 2>/dev/null
}

# xargs -0 preserves spaces, quotes, newlines and leading dashes in paths.
files() {
    mode=$1; shift
    cd "$ROOT" || return 1
    for file do
        case "$file" in "$ROOT/"*) file=${file#"$ROOT/"} ;; esac
        case "$mode" in
        safe)
            if git --literal-pathspecs diff --quiet -- "$file"; then
                printf '%s\0' "$ROOT/$file" >> "$RUN_DIR/$BID-safe"
            else
                printf 'not restaged: has unstaged changes: %s\n' "$file"
                : > "$RUN_DIR/$BID-unsafe"
            fi ;;
        restage)
            before=$(git --literal-pathspecs ls-files -s -- "$file" | awk 'NR==1 {print $2}')
            permissions=$(git --literal-pathspecs ls-files -s -- "$file" | awk 'NR==1 {print $1}')
            git --literal-pathspecs add -- "$file" || return 1
            after=$(git --literal-pathspecs ls-files -s -- "$file" | awk 'NR==1 {print $2}')
            if [ "$before" != "$after" ]; then
                emit "{\"t\":\"restaged\",\"id\":$(json "$BID"),\"files\":[$(json "$file")],\"before\":$(json "$before"),\"after\":$(json "$after"),\"mode\":$(json "$permissions")}"
            fi ;;
        size)
            # Ask the index, not the working file: a partially staged file can differ.
            size=$(git cat-file -s ":$file") || return 1
            if [ "$size" -gt "$LIMIT" ]; then
                printf '%s exceeds the size limit (%s bytes)\n' "$file" "$size"
                : > "$RUN_DIR/$BID-large"
            fi ;;
        esac
    done
}
if [ "${1-}" = '--files' ]; then shift; files "$@"; exit $?; fi

begin() {
    STAGE=$1
    SELF=$(cd "$(dirname "$0")" && pwd)/$(basename "$0")
    ROOT=$(git rev-parse --show-toplevel) || exit 1
    cd "$ROOT" || exit 1
    case "$MESSAGE" in ''|/*|?:/*) ;; *) MESSAGE="$ROOT/$MESSAGE" ;; esac
    common=$(git rev-parse --git-common-dir) || exit 1
    COMMON=$(cd "$common" && pwd) || exit 1
    RUNS="$COMMON/spagitty/hooks/runs"
    mkdir -p "$RUNS" || exit 1
    RUN_ID="$(date '+%s')-$$-$STAGE"
    RUN_DIR="$RUNS/$RUN_ID"
    mkdir "$RUN_DIR" || exit 1
    RECORD="$RUNS/$RUN_ID.jsonl"
    SUBJECT=''
    if [ "$STAGE" = commit-msg ] && [ -f "$MESSAGE" ]; then
        SUBJECT=$(head -n 1 "$MESSAGE")
        # The preceding pre-commit record keeps the message once it is known.
        workgit=$(git rev-parse --git-dir)
        previous=$(cat "$workgit/spagitty-hook-precommit" 2>/dev/null)
        if [ "${MESSAGE##*/}" != try-message ] && [ -n "$previous" ] && [ -f "$previous" ]; then
            printf '{"t":"subject","subject":%s}\n' "$(json "$SUBJECT")" >> "$previous"
        fi
    fi
    export ROOT SELF RUN_DIR RECORD MESSAGE SUBJECT
    git diff --cached --name-only -z --diff-filter=ACMR > "$RUN_DIR/staged"
    if [ "$STAGE" = pre-push ]; then cat > "$RUN_DIR/push"; fi
    emit "{\"t\":\"run\",\"stage\":$(json "$STAGE"),\"subject\":$(json "$SUBJECT"),\"at\":$(json "$(at)"),\"blocks\":$BLOCKS}"
    if [ "$STAGE" = pre-commit ]; then
        workgit=$(git rev-parse --git-dir)
        printf '%s' "$RECORD" > "$workgit/spagitty-hook-precommit"
    fi
    result=0
    # Only managed record names are retained. Their companion directory contains
    # block logs; nothing in the working copy is removed.
    ls -1t "$RUNS/"*.jsonl 2>/dev/null | sed -n '51,$p' | while IFS= read -r old; do
        name=$(basename "$old" .jsonl)
        case "$name" in *[!a-zA-Z0-9_-]*|'') continue ;; esac
        tail -n 1 "$old" | grep -q '"t":"done"' || continue
        rm -f -- "$old"
        rm -rf -- "$RUNS/$name"
    done
}

step() {
    BID=$1; label=$2; warn=$3; seconds=$4; folder=$5; markdown=$6; wip=$7; block=$8
    export BID
    if [ -f "$RUN_DIR/stop" ]; then done_run 130; fi
    if [ "$markdown" = true ] && [ -s "$RUN_DIR/staged" ]; then
        git diff --cached --quiet -- . ':(exclude,glob)**/*.md' ':(exclude,glob)*.md'
        if [ $? -eq 0 ]; then emit "{\"t\":\"skip\",\"id\":$(json "$BID"),\"why\":\"only-markdown\"}"; return; fi
    fi
    if [ "$wip" = true ] && printf '%s' "$SUBJECT" | grep -Eiq '^wip([(: !]|$)'; then
        emit "{\"t\":\"skip\",\"id\":$(json "$BID"),\"why\":\"wip\"}"; return
    fi
    emit "{\"t\":\"start\",\"id\":$(json "$BID"),\"label\":$(json "$label"),\"at\":$(json "$(at)")}"
    started=$(date '+%s')
    (
        cd "$ROOT/$folder" || exit 1
        if timeout --version 2>/dev/null | grep -q 'GNU coreutils'; then
            exec timeout --kill-after=2 "$seconds" sh "$SELF" --block "$block"
        fi
        exec sh "$SELF" --block "$block"
    ) > "$RUN_DIR/$BID.log" 2>&1 &
    running=$!
    (
        elapsed=0
        while kill -0 "$running" 2>/dev/null; do
            sleep 1; elapsed=$((elapsed+1))
            if [ -f "$RUN_DIR/stop" ] || [ "$elapsed" -ge "$seconds" ]; then
                if [ -f "$RUN_DIR/stop" ]; then code=130; else code=124; fi
                printf '%s' "$code" > "$RUN_DIR/$BID-interrupted"
                terminate "$running"
                exit
            fi
        done
    ) & timer=$!
    wait "$running"; code=$?
    kill "$timer" 2>/dev/null; wait "$timer" 2>/dev/null
    if [ -f "$RUN_DIR/$BID-interrupted" ]; then
        code=$(cat "$RUN_DIR/$BID-interrupted")
    fi
    if [ "$code" = 124 ]; then printf 'Timed out after %s seconds.\n' "$seconds" >> "$RUN_DIR/$BID.log"; fi
    cat "$RUN_DIR/$BID.log"
    if [ "$code" = 77 ]; then
        emit "{\"t\":\"skip\",\"id\":$(json "$BID"),\"why\":\"no-match\"}"
        return
    fi
    ms=$((($(date '+%s')-started)*1000))
    emit "{\"t\":\"end\",\"id\":$(json "$BID"),\"exit\":$code,\"ms\":$ms,\"warn\":$warn}"
    if [ "$code" -ne 0 ] && { [ "$warn" = false ] || [ "$code" = 130 ]; }; then done_run "$code"; fi
}
done_run() {
    emit "{\"t\":\"done\",\"exit\":$1}"
    exit "$1"
}
