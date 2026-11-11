#!/bin/sh
d=$(cd "$(dirname "$0")" && pwd)
if [ -x "$d/.venv/Scripts/python.exe" ]; then py="$d/.venv/Scripts/python.exe"; else py="$d/.venv/bin/python"; fi
exec "$py" "$d/detect.py" --stdin
