#!/usr/bin/env python3
"""Build a Russian PD2 S14 patchstring.tbl from extracted Diablo II tables.

Usage: build_translation.py S14-English-patchstring.tbl S9-Russian-patchstring.tbl [output.tbl]
The committed strings.tsv is the translation source of truth.
"""
import csv
import re
import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parent))
from tbl_format import read_tbl, write_tbl

ROOT=Path(__file__).resolve().parent
if len(sys.argv)<3:
    raise SystemExit('usage: build_translation.py S14-English-patchstring.tbl S9-Russian-patchstring.tbl [output.tbl]')
source=Path(sys.argv[1]); s9_path=Path(sys.argv[2]); out=Path(sys.argv[3]) if len(sys.argv)>3 else ROOT/'patchstring.tbl'

with (ROOT/'strings.tsv').open(encoding='utf-8',newline='') as f:
    sheet=list(csv.DictReader(f,delimiter='\t'))
translations={row['key']:row['russian'].replace('\\n','\n') for row in sheet}
sources={row['key']:row['source'] for row in sheet}
english=dict(read_tbl(source))
s9=dict(read_tbl(s9_path))
missing=[key for key in english if key not in s9]
if len(missing)!=900:
    print(f'Note: this S9/S14 pair has {len(missing)} S14-only patch strings (expected 900).')
absent=[key for key in english if key not in translations]
if absent:
    raise SystemExit(f'translation sheet lacks {len(absent)} keys; first: {absent[:10]}')

def signature(s):
    return (sorted(re.findall(r'%[-+0-9.#]*[diuoxXfFeEgGcsp]',s)), sorted(re.findall(r'ÿc.',s)), s.count('\n'))
def align_newlines(english, russian):
    target=english.count('\n'); lines=russian.split('\n')
    while len(lines)-1 < target:
        choices=[(len(x),i,[m.start() for m in re.finditer(' ',x) if m.start()>0]) for i,x in enumerate(lines)]
        choices=[x for x in choices if x[2]]
        if not choices: break
        _,i,spaces=max(choices); pos=min(spaces,key=lambda p:abs(p-len(lines[i])//2))
        lines[i:i+1]=[lines[i][:pos],lines[i][pos+1:]]
    while len(lines)-1 > target:
        i=min(range(len(lines)-1),key=lambda j:len(lines[j])+len(lines[j+1]))
        lines[i:i+2]=[lines[i]+' '+lines[i+1]]
    return '\n'.join(lines)
rows=[]
for key,english_value in english.items():
    russian=align_newlines(english_value,translations[key])
    if signature(english_value)!=signature(russian):
        raise SystemExit(f'format mismatch for {key}: {signature(english_value)} != {signature(russian)}')
    rows.append((key,russian))
# The MPQ has repeated keys; keep the last record, matching Diablo II lookup behavior.
unique={}
for key,value in rows: unique[key]=value
write_tbl(out,unique.items())
# Keep the editable TSV normalized to the exact current S14 key set.
with (ROOT/'strings.tsv').open('w',encoding='utf-8',newline='') as f:
    writer=csv.writer(f,delimiter='\t',lineterminator='\n')
    writer.writerow(['key','english','russian','source'])
    for key,value in unique.items():
        origin=('S9 revised' if key in s9 and sources.get(key)=='S9 revised' else 'S9 reused') if key in s9 else 'S14 translated'
        writer.writerow([key,english[key].replace('\n','\\n'),value.replace('\n','\\n'),origin])
print(f'Wrote {out}: {len(unique)} unique strings, {len(missing)} S14-only strings; preserved format markers.')
