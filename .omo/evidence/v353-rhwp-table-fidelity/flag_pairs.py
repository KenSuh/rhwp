# 같은 이름 HWP/HWPX 쌍에서 HWPX 표 속성과 HWP5 비트(TABLE attr·CTRL_HEADER attr·쪽나눔 방지)의 대응을 표별로 센다.
import sys, os, re, zipfile, subprocess, json
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict
probe, root, out_json = sys.argv[1], sys.argv[2], sys.argv[3]
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
res=subprocess.run([probe]+[p[0] for p in pairs],capture_output=True,text=True).stdout
hwp={l.split('\t',1)[0]:l.split('\t',1)[1] for l in res.splitlines()}
REL={'PAPER':0,'PAGE':1,'COLUMN':2,'PARA':3,'ABSOLUTE':4}
TF={'BOTH_SIDES':0,'LEFT_ONLY':1,'RIGHT_ONLY':2,'LARGEST_ONLY':3}
checks={
 'noAdjust~TABLEattr.bit3': lambda x,ra,ca,pp: (x['tbl'].get('noAdjust')=='1', bool(ra&0x08)),
 'flowWithText~CTRL.bit13': lambda x,ra,ca,pp: (x['pos'].get('flowWithText')=='1', bool(ca>>13&1)),
 'allowOverlap~CTRL.bit14': lambda x,ra,ca,pp: (x['pos'].get('allowOverlap')=='1', bool(ca>>14&1)),
 'affectLSpacing~CTRL.bit2': lambda x,ra,ca,pp: (x['pos'].get('affectLSpacing')=='1', bool(ca>>2&1)),
 'szProtect~CTRL.bit20': lambda x,ra,ca,pp: (x['sz'].get('protect')=='1', bool(ca>>20&1)),
 'lock~CTRL.bit30': lambda x,ra,ca,pp: (x['tbl'].get('lock')=='1', bool(ca>>30&1)),
 'textFlow~CTRL.bits24-25': lambda x,ra,ca,pp: (TF.get(x['tbl'].get('textFlow'),-1), ca>>24&3),
 'holdAnchorAndSO~preventPageBreak': lambda x,ra,ca,pp: (x['pos'].get('holdAnchorAndSO')=='1', pp!=0),
 'widthRelTo~CTRL.bits15-17': lambda x,ra,ca,pp: (REL.get(x['sz'].get('widthRelTo'),-1), ca>>15&7),
 'heightRelTo~CTRL.bits18-19': lambda x,ra,ca,pp: ({'PAPER':0,'PAGE':1,'ABSOLUTE':2}.get(x['sz'].get('heightRelTo'),-1), ca>>18&3),
 'repeatHeader~TABLEattr.bit2': lambda x,ra,ca,pp: (x['tbl'].get('repeatHeader')=='1', bool(ra&0x04)),
}
co=defaultdict(Counter); used=0
for h,x in pairs:
    v=hwp.get(h,'')
    if v.startswith('ERR') or not v: continue
    recs=[tuple(int(t) for t in r.split(':')) for r in v.split(',')]
    try:
        with zipfile.ZipFile(x) as z:
            names=sorted((n for n in z.namelist() if re.fullmatch(r'Contents/section\d+\.xml',n)),key=lambda n:int(re.search(r'(\d+)\.xml$',n).group(1)))
            tbls=[]
            for n in names:
                for el in ET.fromstring(z.read(n)).iter():
                    if local(el.tag)=='tbl':
                        sz=next((c for c in el if local(c.tag)=='sz'),None); pos=next((c for c in el if local(c.tag)=='pos'),None)
                        tbls.append({'tbl':dict(el.attrib),'sz':dict(sz.attrib) if sz is not None else {},'pos':dict(pos.attrib) if pos is not None else {}})
    except Exception: continue
    if len(tbls)!=len(recs) or not tbls: continue
    used+=1
    for t,(ra,ca,pp) in zip(tbls,recs):
        for name,f in checks.items():
            co[name][str(f(t,ra,ca,pp))]+=1
print('aligned_docs',used)
for name in checks:
    agree=sum(n for k,n in co[name].items() if (lambda a,b: a==b)(*eval(k)))
    total=sum(co[name].values())
    print(f'{name}: agree {agree}/{total}  detail {dict(co[name])}')
json.dump({'aligned_docs':used,'checks':{k:dict(v) for k,v in co.items()}},open(out_json,'w'),ensure_ascii=False,indent=1)
