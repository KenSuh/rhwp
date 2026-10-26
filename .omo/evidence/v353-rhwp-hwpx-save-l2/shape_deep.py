# 도형 요소별 직속 자식 속성까지 원본과 비교한다(측정 전용).
import sys, zipfile, re, collections
import xml.etree.ElementTree as ET
def local(t): return t.split("}",1)[1] if "}" in t else t
def elems(fn, tags):
    z=zipfile.ZipFile(fn); out=[]
    for n in sorted(x for x in z.namelist() if re.match(r"Contents/section\d+\.xml$",x)):
        r=ET.fromstring(z.read(n))
        def walk(el, parent_tag):
            for c in el:
                if local(c.tag) in tags: out.append((local(c.tag), parent_tag, c))
                walk(c, local(c.tag))
        walk(r, "")
    return out
def amap(e): return {local(k):v for k,v in e.attrib.items()}
tags={"rect","container","line","ellipse","pic","tbl"}
A=elems(sys.argv[1],tags); B=elems(sys.argv[2],tags)
print("count", len(A), len(B))
diff=collections.Counter(); samples={}
for (ta,pa,ea),(tb,pb,eb) in zip(A,B):
    assert ta==tb, (ta,tb)
    ingroup = "child" if pa=="container" else "top"
    key0=(ta,ingroup)
    for k in set(amap(ea))|set(amap(eb)):
        if amap(ea).get(k)!=amap(eb).get(k):
            diff[(key0,"@"+k)]+=1; samples.setdefault((key0,"@"+k),(amap(ea).get(k),amap(eb).get(k)))
    ca={}; cb={}
    for c in ea:
        if local(c.tag) in tags: continue
        ca.setdefault(local(c.tag),[]).append(c)
    for c in eb:
        if local(c.tag) in tags: continue
        cb.setdefault(local(c.tag),[]).append(c)
    for name in set(ca)|set(cb):
        if name in ("subList","tr","drawText","caption"): continue
        la=ca.get(name,[]); lb=cb.get(name,[])
        if len(la)!=len(lb): diff[(key0,name+"#count")]+=1; samples.setdefault((key0,name+"#count"),(len(la),len(lb))); continue
        for x,y in zip(la,lb):
            for k in set(amap(x))|set(amap(y)):
                if amap(x).get(k)!=amap(y).get(k):
                    diff[(key0,name+"@"+k)]+=1; samples.setdefault((key0,name+"@"+k),(amap(x).get(k),amap(y).get(k)))
for (k,c) in sorted(diff.items(), key=lambda kv:(kv[0][0],-kv[1])):
    print(f"{c:4d} {k[0][0]:9s} {k[0][1]:5s} {k[1]:28s} e.g. {samples[k]}")
