#!/bin/sh
# Stdin scorer for `--preset strong`: e5-small RAID detector P(AI) on the last stdout line.
d=$(cd "$(dirname "$0")" && pwd)
if [ -x "$d/.venv/Scripts/python.exe" ]; then py="$d/.venv/Scripts/python.exe"; else py="$d/.venv/bin/python"; fi
exec "$py" "$d/holdout.py" --model e5raid --stdin
