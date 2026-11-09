#!/usr/bin/env python3
"""Rebuild an indexed vanilla Diablo II string.tbl from legacy-strings.tsv.

Usage: build_legacy_translation.py string.tbl|expansionstring.tbl English.tbl [output.tbl]
"""
import csv
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tbl_format import read_tbl, write_tbl_rows

ROOT = Path(__file__).resolve().parent
if len(sys.argv) < 3:
    raise SystemExit('usage: build_legacy_translation.py TABLE English.tbl [output.tbl]')

table_name = sys.argv[1]
english_path = Path(sys.argv[2])
output_path = Path(sys.argv[3]) if len(sys.argv) > 3 else Path(table_name)
english = read_tbl(english_path)

with (ROOT / 'legacy-strings.tsv').open(encoding='utf-8', newline='') as f:
    sheet = [r for r in csv.DictReader(f, delimiter='\t') if r['table'] == table_name]
if len(sheet) != len(english):
    raise SystemExit(f'{table_name}: source has {len(sheet)} rows; English table has {len(english)}')

def signature(s: str):
    return (
        re.findall(r'%(?!%)[-+0-9.#]*(?:hh|h|ll|l|I64)?[diuoxXfFeEgGcspS]', s),
        re.findall(r'ÿc.', s),
    )

rows = []
for index, ((key, english_value), row) in enumerate(zip(english, sheet)):
    russian = row['russian'].replace('\\n', '\n')
    if int(row['index']) != index or row['key'] != key or row['english'].replace('\\n', '\n') != english_value:
        raise SystemExit(f'{table_name}: English source mismatch at row {index} ({key})')
    if signature(english_value) != signature(russian):
        raise SystemExit(f'{table_name}: format mismatch at row {index} ({key})')
    rows.append((key, russian))

write_tbl_rows(output_path, rows, allow_duplicate_keys=True)
if read_tbl(output_path) != rows:
    raise SystemExit(f'{table_name}: output did not preserve row indices')
print(f'Wrote {output_path}: {len(rows)} indexed rows preserved.')
