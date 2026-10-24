#!/usr/bin/env bash
# Fails unless the folder has cut-off files and every line of them is `ID BALANCE`.
# Usage: check_cut_off_files.sh <cut_off_dir>
set -euo pipefail

CUT_OFF_DIR="${1:?usage: check_cut_off_files.sh <cut_off_dir>}"
# `ID BALANCE` with the id and the balance in normal form: no leading or trailing zeros.
LINE_PATTERN='^[1-9][0-9]* (0|-?[1-9][0-9]*(\.[0-9]*[1-9])?|-?0\.[0-9]*[1-9])$'

shopt -s nullglob
cut_off_files=("$CUT_OFF_DIR"/*.DAT)

if [[ ${#cut_off_files[@]} -eq 0 ]]; then
  echo "FAIL: no cut-off file in $CUT_OFF_DIR" >&2
  exit 1
fi

if grep --invert-match --extended-regexp --quiet "$LINE_PATTERN" "${cut_off_files[@]}"; then
  echo "FAIL: a cut-off line is not 'ID BALANCE'" >&2
  exit 1
fi

# An empty cut-off is valid, but a check that saw no line at all proves nothing.
if ! grep --extended-regexp --quiet "$LINE_PATTERN" "${cut_off_files[@]}"; then
  echo "FAIL: the cut-off files have no 'ID BALANCE' line" >&2
  exit 1
fi

echo "ok: ${#cut_off_files[@]} cut-off file(s) in $CUT_OFF_DIR, every line is 'ID BALANCE'"
