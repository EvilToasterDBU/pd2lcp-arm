import json,re,sys
from pathlib import Path
root=Path(sys.argv[1]); p=root/'d2gl/src/modules/hd_text/variables.h'
s=p.read_text(encoding='utf-8-sig'); tr=json.loads(Path(__file__).with_name('translations.json').read_text(encoding='utf-8'))
english=s.split('inline std::map<uint32_t, std::vector<D2TextInfo>> g_options_text_others')[0]
mod=english.split('// cfgoptions.dc6\n',1)[1].rsplit('};',1)[0]
rows=re.findall(r'^\s*\{.*?L"([^"]+)".*?//.*$',mod,re.M)
assert len(rows)>70
missing=set(rows)-tr.keys()
assert not missing,missing
translated=re.sub(r'L"([^"]+)"',lambda m:'L"'+tr[m[1]]+'"',mod)
translated=translated.replace('{ 138, 26 }', '{ 249, 26 }').replace('{ 171, 26 }', '{ 250, 26 }')
a=s.index('{ LANG_RUS, {',s.index('g_options_text_others'))
b=s.index('\n\t} },\n};',a)
s=s[:b]+'\n\t\t// PD2 S14: use the native HD text renderer, retaining original DC6 signatures.\n'+translated+s[b:]
p.write_text(s,encoding='utf-8-sig')
print(f'Added {len(rows)} Russian HD menu entries')
