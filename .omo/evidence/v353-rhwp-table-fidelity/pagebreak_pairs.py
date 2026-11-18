# 같은 이름의 HWP/HWPX 표본 쌍에서 표별 HWPX pageBreak 문자열과 HWP5 TABLE bit 0-1 을 대조한다.
import sys, os, re, zipfile, subprocess, json
import xml.etree.ElementTree as ET
from collections import Counter
probe, root = sys.argv[1], sys.argv[2]
def local(t): return t.rsplit('}',1)[-1]
pairs=[]
for d,_,fs in os.walk(root):
    stems={}
    for f in fs:
        m=re.match(r'(.*)\.(hwpx|hwp)$', f, re.I)
        if m: stems.setdefault(m.group(1),{})[m.group(2).lower()]=os.path.join(d,f)
    for s,v in stems.items():
        if 'hwp' in v and 'hwpx' in v: pairs.append((v['hwp'],v['hwpx']))
pairs.sort()
out=subprocess.run([probe]+[p[0] for p in pairs],capture_output=True,text=True).stdout
hwp={}
for line in out.splitlines():
    path,val=line.split('\t',1); hwp[path]=val
co=Counter(); used=0; skipped=[]; docs=[]
for h,x in pairs:
    v=hwp.get(h,'')
    if v.startswith('ERR'): skipped.append((os.path.basename(h),'hwp-err')); continue
    bits=[b for b in v.split(',') if b!='']
    try:
        with zipfile.ZipFile(x) as z:
            names=sorted((n for n in z.namelist() if re.fullmatch(r'Contents/section\d+\.xml',n)),key=lambda n:int(re.search(r'(\d+)\.xml$',n).group(1)))
            pb=[]
            for n in names:
                for el in ET.fromstring(z.read(n)).iter():
                    if local(el.tag)=='tbl': pb.append(el.get('pageBreak'))
    except Exception as e:
        skipped.append((os.path.basename(x),'hwpx-err')); continue
    if len(pb)!=len(bits) or not pb:
        skipped.append((os.path.basename(x),f'count {len(pb)}!={len(bits)}')); continue
    used+=1
    c=Counter(zip(pb,bits)); co.update(c); docs.append((os.path.relpath(x,root),{f"{a}|{b}":n for (a,b),n in c.items()}))
print('pairs',len(pairs),'aligned_docs',used,'skipped',len(skipped))
print('co-occurrence (HWPX pageBreak, HWP5 bits0-1):')
for k,v in sorted(co.items()): print('  ',k,v)
print('docs with TABLE:',[d for d in docs if any(k.startswith("TABLE|") for k in d[1])][:10])
json.dump({'pairs':len(pairs),'aligned_docs':used,'cooccurrence':{f'{a}|{b}':n for (a,b),n in co.items()},'docs':docs,'skipped':skipped},open(sys.argv[3],'w'),ensure_ascii=False,indent=1)
