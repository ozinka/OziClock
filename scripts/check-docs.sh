#!/bin/sh

set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

failures=0

check_unique_ids() {
    label=$1
    definition_pattern=$2
    id_pattern=$3
    file=$4

    duplicates=$(grep -Eo "$definition_pattern" "$file" \
        | grep -Eo "$id_pattern" \
        | sort \
        | uniq -d \
        || true)
    if [ -n "$duplicates" ]; then
        printf '%s contains duplicate IDs:\n%s\n' "$label" "$duplicates" >&2
        failures=1
    fi
}

check_references() {
    label=$1
    definition_pattern=$2
    reference_pattern=$3
    definition_file=$4
    shift 4

    definitions=$(mktemp)
    references=$(mktemp)
    trap 'rm -f "$definitions" "$references"' EXIT HUP INT TERM

    grep -Eo "$definition_pattern" "$definition_file" \
        | grep -Eo "$reference_pattern" \
        | sort -u > "$definitions"
    grep -Ehro "$reference_pattern" "$@" | sort -u > "$references" || true
    unknown=$(comm -13 "$definitions" "$references")
    if [ -n "$unknown" ]; then
        printf '%s contains references to undefined IDs:\n%s\n' "$label" "$unknown" >&2
        failures=1
    fi

    rm -f "$definitions" "$references"
    trap - EXIT HUP INT TERM
}

requirement_id='(WIN|MODE|CLK|RUL|SET|ALM|UI|CAL|TMR|TSK|SW|REM|PLN|EVT|NTF|NFR)-[0-9]+[A-Z]?'
backlog_id='BL-[0-9]+'

requirement_definition="^- \*\*$requirement_id"
backlog_definition='^\| BL-[0-9]+ \|'

check_unique_ids "docs/REQUIREMENTS.md" "$requirement_definition" "$requirement_id" \
    docs/REQUIREMENTS.md
check_unique_ids "docs/BACKLOG.md queue and archive" "$backlog_definition" "$backlog_id" \
    docs/BACKLOG.md
check_references "Documentation" "$requirement_definition" "$requirement_id" \
    docs/REQUIREMENTS.md \
    docs/BACKLOG.md docs/decisions/*.md
check_references "Documentation" "$backlog_definition" "$backlog_id" docs/BACKLOG.md \
    docs/REQUIREMENTS.md docs/decisions/*.md

if [ "$failures" -ne 0 ]; then
    exit 1
fi

printf 'Documentation contracts are valid.\n'