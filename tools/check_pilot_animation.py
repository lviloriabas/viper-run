#!/usr/bin/env python3
"""Capture control poses and every ejection phase on the native game binary."""
import argparse,hashlib,json,os,subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument("executable",type=Path);p.add_argument("--output",type=Path,default=Path("target/model-review/pilot"))
a=p.parse_args();exe=a.executable.resolve();a.output.mkdir(parents=True,exist_ok=True)
startup=None
if os.name=="nt":
    startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW;startup.wShowWindow=0
cases=[
("controls-left",["--input","pull,left,yawleft,throttle","--sim",".4","--look","0,-62"]),
("controls-right",["--input","push,right,yawright","--sim",".4","--look","0,-62"]),
("reach",["--input","eject","--sim",".18","--look","0,-62"]),
("pull",["--input","eject","--sim",".43","--look","0,-62"]),
("canopy",["--input","eject","--sim",".65","--look","0,-62"]),
("canopy-external",["--input","eject","--sim",".65","--chase","--look","30,-10","--zoom",".35"]),
("seat",["--input","eject","--sim","1.15","--chase","--look","30,-10","--zoom",".35"]),
("seat-first-person",["--input","eject","--sim","1.15","--look","0,-50"]),
("separating",["--input","eject","--sim","2.2","--chase","--look","30,-10","--zoom",".45"]),
("chute-grip",["--input","eject","--sim","5","--look","65,55"]),
("chute-body",["--input","eject","--sim","5","--look","0,-50"]),
("landed",["--input","eject","--sim","18","--chase","--look","65,-20","--zoom",".4"])]
report=[]
for jet in ["f16","mig29","f18","su27"]:
    for name,base_opts in cases:
        opts=list(base_opts)
        if name=="separating":
            opts[opts.index("--sim")+1]="2.65" if jet in ("mig29","su27") else "2.0"
        shot=(a.output/(jet+"-"+name+".png")).resolve()
        result=subprocess.run([str(exe),"--autostart","--no-performance","--jet",jet,
            "--mission","airfield","--size","1440x900","--shot",str(shot),*opts],
            timeout=90,capture_output=True,text=True,startupinfo=startup)
        if result.returncode or not shot.exists() or shot.read_bytes()[:8]!=b"\x89PNG\r\n\x1a\n":raise RuntimeError(f"{jet} {name}: {result.stdout} {result.stderr}")
        report.append({"jet":jet,"view":name,"options":opts,"image":shot.name})
(a.output/"captures.json").write_text(json.dumps(report,indent=2))
(a.output/"binary.sha256").write_text(hashlib.sha256(exe.read_bytes()).hexdigest()+"\n")
print(f"Captured {len(report)} pilot views")
