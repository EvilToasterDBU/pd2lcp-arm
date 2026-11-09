"""Restore original S14 DC6 identifiers; HD text is drawn by D2GL, not these pixels."""
import argparse,hashlib,json,struct,subprocess
from pathlib import Path
ap=argparse.ArgumentParser();ap.add_argument('mpq');ap.add_argument('listfile');ap.add_argument('tool');ap.add_argument('output');a=ap.parse_args()
out=Path(a.output);out.mkdir(parents=True,exist_ok=True);manifest=[]
for path in Path(a.listfile).read_text().splitlines():
 prefix='data\\local\\ui\\eng\\'
 if not path.lower().startswith(prefix.lower()):continue
 name=path[len(prefix):]
 if '\\' in name or not name.lower().endswith('.dc6'):continue
 dest=out/name
 subprocess.run([a.tool,a.mpq,'extract',path,str(dest)],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 data=bytearray(dest.read_bytes())
 if name in {'SkillBar.dc6','SNext.dc6'}:
  assert struct.unpack_from('<II',data,16)==(1,1)
  frame=struct.unpack_from('<I',data,24)[0]
  old,new={'SkillBar.dc6':(138,249),'SNext.dc6':(171,250)}[name]
  assert struct.unpack_from('<I',data,frame+4)[0]==old
  struct.pack_into('<I',data,frame+4,new)
  dest.write_bytes(data)
 manifest.append({'file':name,'sha256':hashlib.sha256(data).hexdigest()})
(out.parent/(out.name+'-manifest.json')).write_text(json.dumps(manifest,indent=2))
print(f'Prepared {len(manifest)} original S14 UI resources; two signatures padded to avoid vanilla collisions.')
