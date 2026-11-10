"""Copy original S14 English UI DC6 files into the Russian locale unchanged."""
import argparse,hashlib,json,subprocess
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
 data=dest.read_bytes()
 manifest.append({'file':name,'sha256':hashlib.sha256(data).hexdigest()})
(out.parent/(out.name+'-manifest.json')).write_text(json.dumps(manifest,indent=2))
print(f'Copied {len(manifest)} original S14 UI resources without changing their signatures.')
