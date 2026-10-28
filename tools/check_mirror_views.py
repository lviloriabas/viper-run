#!/usr/bin/env python3
"""Native game captures for mirror placement, optical sampling and depth."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument("executable",type=Path);p.add_argument("--output",type=Path,required=True)
a=p.parse_args();exe=a.executable.resolve();a.output.mkdir(parents=True,exist_ok=True)
startup=None
if os.name=="nt":
    startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW;startup.wShowWindow=0
views=[("forward",["--look","0,0"]),("crown",["--look","0,18"]),
       ("left",["--look","-30,10"]),("right",["--look","30,10"]),
       ("panel",["--look","0,-25"]),
       ("hud-lead",["--look","-16,-7"]),
       ("bank",["--look","0,8","--input","right","--sim",".7"]),
       ("disabled",["--look","0,0","--no-mirrors"])]
cases=[(jet,name,opts) for jet in ["f18","su27"] for name,opts in views]
cases += [("mig29",name,opts) for name,opts in views[:4]]
cases += [("f16","forward",views[0][1])]
report=[]
for jet,name,opts in cases:
    path=(a.output/(jet+"-"+name+".png")).resolve()
    r=subprocess.run([str(exe),"--autostart","--no-performance","--jet",jet,
        "--size","1440x900","--shot",str(path),*opts],capture_output=True,text=True,timeout=90,startupinfo=startup)
    if r.returncode or not path.exists() or path.read_bytes()[:8]!=b"\x89PNG\r\n\x1a\n":
        raise RuntimeError(f"{jet} {name}: {r.stdout} {r.stderr}")
    report.append({"jet":jet,"view":name,"image":path.name,"options":opts})
(a.output/"captures.json").write_text(json.dumps(report,indent=2))
(a.output/"binary.sha256").write_text(hashlib.sha256(exe.read_bytes()).hexdigest()+"\n")
print(f"{len(report)} mirror captures ready for review")
