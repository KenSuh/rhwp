# 레인 2 측정: 문단 원자 순서열 비교 + 도형·필드·header 잔여 결함(측정 전용).
import sys, zipfile, re, collections, json
import xml.etree.ElementTree as ET
NS_HP="http://www.hancom.co.kr/hwpml/2011/paragraph"
def local(t): return t.split("}",1)[1] if "}" in t else t
def prefix_ns(t): return t.split("}",1)[0][1:] if "}" in t else ""
def sec_roots(fn):
    z=zipfile.ZipFile(fn)
    names=sorted((n for n in z.namelist() if re.match(r"Contents/section\d+\.xml$",n)), key=lambda s:int(re.findall(r"\d+",s)[0]))
    return [ET.fromstring(z.read(n)) for n in names], z
def text_of_t(t):
    out=[]
    if t.text: out.append(t.text)
    for g in t:
        n=local(g.tag)
        if n=="tab": out.append("\t")
        elif n in ("lineBreak","columnBreak"): out.append("\n")
        elif n=="nbSpace": out.append(" ")
        elif n=="fwSpace": out.append(" ")
        if g.tail: out.append(g.tail)
    return "".join(out)
def atoms_of_p(p):
    atoms=[]
    def push_text(s):
        if not s: return
        if atoms and atoms[-1][0]=="T": atoms[-1]=("T",atoms[-1][1]+s)
        else: atoms.append(("T",s))
    for run in p:
        if local(run.tag)!="run": continue
        for ch in run:
            n=local(ch.tag)
            if n=="t": push_text(text_of_t(ch))
            elif n=="ctrl":
                for g in ch: atoms.append(("C",local(g.tag)))
            elif n=="secPr": atoms.append(("S","secPr"))
            else: atoms.append(("O",n))
    return atoms
def paras(root):
    out=[]
    def walk(el, depth):
        for c in el:
            if local(c.tag)=="p":
                out.append((depth, atoms_of_p(c)))
                walk(c, depth+1)
            else:
                walk(c, depth)
    walk(root,0)
    return out
def all_paras(fn):
    roots,_=sec_roots(fn); res=[]
    for si,r in enumerate(roots):
        for d,a in paras(r): res.append((si,d,a))
    return res
def kinds(a): return [(k, (v if k!="T" else "T")) for k,v in a]
def obj_text_order(a):
    # 객체·컨트롤과 텍스트의 상대 순서만(텍스트 내용 무시)
    return [k if k=="T" else f"{k}:{v}" for k,v in a]
def compare(ref_fn, out_fn, label):
    R=all_paras(ref_fn); O=all_paras(out_fn)
    res={"label":label,"ref_paras":len(R),"out_paras":len(O)}
    if len(R)!=len(O):
        res["error"]="paragraph count differs"; return res
    order_diff=[]; text_diff=0; obj_ws=0; ex=[]
    for i,((s1,d1,a1),(s2,d2,a2)) in enumerate(zip(R,O)):
        if obj_text_order(a1)!=obj_text_order(a2):
            order_diff.append(i)
            # 인라인 표 앞에 공백 run: 원본은 tbl 먼저, 출력은 공백 T 먼저
            if a2 and a2[0][0]=="T" and a2[0][1].strip()=="" and any(k=="O" and v=="tbl" for k,v in a2[1:2]):
                obj_ws+=1
            if len(ex)<8: ex.append({"i":i,"depth":d1,"ref":[f"{k}:{v[:12]!r}" for k,v in a1][:8],"out":[f"{k}:{v[:12]!r}" for k,v in a2][:8]})
        elif a1!=a2:
            text_diff+=1
    res.update({"order_diff_paras":len(order_diff),"ws_before_inline_tbl":obj_ws,"text_only_diff_paras":text_diff,"examples":ex})
    cats=collections.Counter()
    for i in order_diff:
        a1=obj_text_order(R[i][2]); a2=obj_text_order(O[i][2])
        cats[(" ".join(a1[:6])," -> "," ".join(a2[:6]))]+=1
    res["order_diff_patterns"]=[f"{c}x {k[0]} {k[1]} {k[2]}" for k,c in cats.most_common(15)]
    return res
def elem_children(fn, tagname):
    roots,_=sec_roots(fn); out=[]
    for r in roots:
        for el in r.iter():
            if local(el.tag)==tagname:
                out.append(([local(c.tag) for c in el], dict(el.attrib)))
    return out
def shape_report(ref_fn, out_fn):
    rep={}
    for tag in ["rect","container","line","ellipse","pic","tbl"]:
        A=elem_children(ref_fn,tag); B=elem_children(out_fn,tag)
        d={"ref":len(A),"out":len(B)}
        if A and len(A)==len(B):
            child_diff=collections.Counter(); attr_diff=collections.Counter()
            for (ca,aa),(cb,ab) in zip(A,B):
                if ca!=cb:
                    miss=[c for c in ca if c not in cb]; extra=[c for c in cb if c not in ca]
                    if miss or extra: child_diff[("missing",tuple(sorted(set(miss))),"extra",tuple(sorted(set(extra))))]+=1
                    else: child_diff[("order",tuple(ca),"vs",tuple(cb))]+=1
                for k in set(aa)|set(ab):
                    if aa.get(k)!=ab.get(k): attr_diff[k]+=1
            d["child_diffs"]={str(k):v for k,v in child_diff.items()}
            d["attr_diffs"]=dict(attr_diff)
        rep[tag]=d
    return rep
def field_report(fn):
    roots,z=sec_roots(fn)
    raw=sum(len(re.findall(r"[\x03\x04]", z.read(n).decode("utf-8"))) for n in z.namelist() if re.match(r"Contents/section\d+\.xml$",n))
    fb=sum(1 for r in roots for el in r.iter() if local(el.tag)=="fieldBegin")
    fe=sum(1 for r in roots for el in r.iter() if local(el.tag)=="fieldEnd")
    return {"raw_ctrl_chars":raw,"fieldBegin":fb,"fieldEnd":fe}
def header_report(fn):
    z=zipfile.ZipFile(fn); x=z.read("Contents/header.xml").decode("utf-8")
    r={}
    for tag in ["intent","left","right","prev","next"]:
        r[f"hh:{tag}"]=len(re.findall(rf"<hh:{tag}\b",x)); r[f"hc:{tag}"]=len(re.findall(rf"<hc:{tag}\b",x))
    r["paraPr"]=len(re.findall(r"<hh:paraPr\b",x)); r["switch"]=len(re.findall(r"<hp:switch\b",x))
    r["margin_in_switch_case"]=len(re.findall(r"<hp:case[^>]*>\s*<hh:margin",x))
    r["margin_total"]=len(re.findall(r"<hh:margin\b",x))
    r["breakNonLatinWord"]=collections.Counter(re.findall(r'breakNonLatinWord="([A-Z_]+)"',x))
    return r
if __name__=="__main__":
    ref=sys.argv[1]; outs=sys.argv[2:]
    rep={"ref":ref,"compare":[],"shapes":{},"fields":{},"header":{}}
    rep["fields"]["ref"]=field_report(ref); rep["header"]["ref"]=header_report(ref)
    for o in outs:
        lab=o.split("/")[-1]
        rep["compare"].append(compare(ref,o,lab))
        rep["shapes"][lab]=shape_report(ref,o)
        rep["fields"][lab]=field_report(o)
        rep["header"][lab]=header_report(o)
    print(json.dumps(rep,ensure_ascii=False,indent=1,default=str))
