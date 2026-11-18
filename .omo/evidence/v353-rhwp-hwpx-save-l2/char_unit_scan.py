# 한컴 저장 HWPX 의 paraPr switch 에서 case unit=CHAR 와 default 값의 관계를 센다(측정 전용).
import sys, zipfile, re, os
pat_pp = re.compile(r'<hh:paraPr id="(\d+)".*?</hh:paraPr>', re.S)
pat_case = re.compile(r'<hp:case[^>]*HwpUnitChar[^>]*>(.*?)</hp:case>\s*<hp:default>(.*?)</hp:default>', re.S)
pat_m = re.compile(r'<hc:(intent|left|right|prev|next) value="(-?\d+)" unit="(\w+)"/>')
def scan(fn):
    try:
        z = zipfile.ZipFile(fn); h = z.read("Contents/header.xml").decode("utf-8")
    except Exception as e:
        return None
    ver = ""
    try:
        v = z.read("version.xml").decode("utf-8", "replace")
        m = re.search(r'appVersion="([^"]*)"', v); ver = (m.group(1) if m else "") + " " + " ".join(re.findall(r'os="(\d+)"', v))
    except Exception:
        pass
    char = []; odd_default = 0; rel_ok = 0; rel_bad = []
    for pp in pat_pp.finditer(h):
        sw = pat_case.search(pp.group(0))
        if not sw: continue
        case = {k: (int(v), u) for k, v, u in pat_m.findall(sw.group(1))}
        default = {k: int(v) for k, v, _ in pat_m.findall(sw.group(2))}
        for k, (cv, cu) in case.items():
            dv = default.get(k)
            if dv is None: continue
            if dv % 2: odd_default += 1
            if cu == "CHAR":
                ok = dv == cv * 2 + 1
                rel_ok += ok
                if not ok: rel_bad.append((pp.group(1), k, cv, cu, dv))
                char.append((pp.group(1), k, cv, cu, dv))
    return ver, char, odd_default, rel_ok, rel_bad
roots = sys.argv[1:]
files = []
for r in roots:
    for dp, dn, fn in os.walk(r):
        files += [os.path.join(dp, f) for f in fn if f.lower().endswith(".hwpx")]
tot = 0
for f in sorted(files):
    res = scan(f)
    if not res: continue
    ver, char, odd_default, rel_ok, rel_bad = res
    if char or odd_default:
        tot += 1
        print(f"{len(char):4d} CHAR | odd default {odd_default:4d} | CHAR→default=case*2+1 ok {rel_ok} bad {len(rel_bad)} | app {ver} | {f}")
        if char: print("      e.g.", char[:3])
        if rel_bad: print("      BAD", rel_bad[:3])
print("files scanned", len(files), "with CHAR/odd", tot)
