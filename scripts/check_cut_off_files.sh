#!/usr/bin/env bash
# Fails unless the folder has cut-off files, no temp file, and every file is well formed:
# lines `ID BALANCE` with normalized numbers, strictly increasing ids, a final newline.
# Usage: check_cut_off_files.sh <cut_off_dir>
set -euo pipefail

CUT_OFF_DIR="${1:?usage: check_cut_off_files.sh <cut_off_dir>}"
# `ID BALANCE` with the id and the balance in normal form: no leading or trailing zeros.
LINE_PATTERN='^[1-9][0-9]* (0|-?[1-9][0-9]*(\.[0-9]*[1-9])?|-?0\.[0-9]*[1-9])$'
GREP_FOUND=0
GREP_NOT_FOUND=1

fail() {
  echo "FAIL: $1" >&2
  exit 1
}

# Succeeds when the file has a line that does not match LINE_PATTERN; a grep error stops the script.
has_malformed_line() {
  local status=0
  grep --invert-match --extended-regexp --quiet "$LINE_PATTERN" "$1" || status=$?
  case "$status" in
    "$GREP_FOUND") return 0 ;;
    "$GREP_NOT_FOUND") return 1 ;;
    *) fail "grep could not read $1" ;;
  esac
}

ends_with_newline_or_is_empty() {
  [[ ! -s "$1" || -z "$(tail -c 1 "$1")" ]]
}

ids_strictly_increase() {
  awk '$1 <= previous { exit 1 } { previous = $1 }' "$1"
}

shopt -s nullglob dotglob
temp_entries=("$CUT_OFF_DIR"/.tmp-*)
cut_off_files=("$CUT_OFF_DIR"/*.DAT)
total_lines=0

[[ ${#temp_entries[@]} -eq 0 ]] || fail "temp file left in $CUT_OFF_DIR: ${temp_entries[0]}"
[[ ${#cut_off_files[@]} -gt 0 ]] || fail "no cut-off file in $CUT_OFF_DIR"

for cut_off_file in "${cut_off_files[@]}"; do
  ends_with_newline_or_is_empty "$cut_off_file" || fail "$cut_off_file has no final newline"
  if has_malformed_line "$cut_off_file"; then
    fail "$cut_off_file has a line that is not a normalized 'ID BALANCE'"
  fi
  ids_strictly_increase "$cut_off_file" || fail "$cut_off_file has ids that do not increase"
  total_lines=$((total_lines + $(wc -l < "$cut_off_file")))
done

# An empty cut-off is valid, but a check that saw no line at all proves nothing.
[[ "$total_lines" -gt 0 ]] || fail "the cut-off files have no 'ID BALANCE' line"

echo "ok: ${#cut_off_files[@]} cut-off file(s) in $CUT_OFF_DIR, $total_lines line(s), all well formed"
