#!/usr/bin/env python3
"""Build the indexed Russian PD2 S14 patchstring.tbl.

Usage: build_translation.py S14-English-patchstring.tbl [output.tbl]
The TSV is row-indexed because Diablo II tables can contain duplicate keys.
Preserving every source row and its index is essential for correct lookups.
"""
import csv
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from tbl_format import read_tbl, write_tbl_rows

ROOT = Path(__file__).resolve().parent
if len(sys.argv) < 2:
    raise SystemExit('usage: build_translation.py S14-English-patchstring.tbl [output.tbl]')

source = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 'patchstring.tbl'
english = read_tbl(source)

with (ROOT / 'strings.tsv').open(encoding='utf-8', newline='') as f:
    sheet = list(csv.DictReader(f, delimiter='\t'))

expected_fields = {'index', 'key', 'english', 'russian', 'source'}
if not sheet or not expected_fields.issubset(sheet[0]):
    raise SystemExit('strings.tsv must contain index, key, english, russian, and source columns')
if len(sheet) != len(english):
    raise SystemExit(f'row count mismatch: strings.tsv has {len(sheet)} rows; S14 has {len(english)}')

def signature(s: str):
    return (
        re.findall(r'%(?!%)[-+0-9.#]*(?:hh|h|ll|l|I64)?[diuoxXfFeEgGcspS]', s),
        re.findall(r'ÿc.', s),
        s.count('\n'),
    )

rows = []
for index, ((key, english_value), row) in enumerate(zip(english, sheet)):
    source_english = row['english'].replace('\\n', '\n')
    russian = row['russian'].replace('\\n', '\n')
    if int(row['index']) != index or row['key'] != key or source_english != english_value:
        raise SystemExit(f'S14 source mismatch at row {index}: {key!r}')
    if signature(english_value) != signature(russian):
        raise SystemExit(f'format mismatch at row {index} ({key}): {signature(english_value)} != {signature(russian)}')
    rows.append((key, russian))

write_tbl_rows(out, rows, allow_duplicate_keys=True)
round_trip = read_tbl(out)
if round_trip != rows:
    raise SystemExit('output table did not preserve the full S14 row order')

print(f'Wrote {out}: {len(rows)} indexed rows, {len(set(key for key, _ in rows))} unique keys; preserved duplicate keys and row order.')
