"""Native acceptance views for Hornet bows, E/G models and store clearance."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('executable',type=Path)
p.add_argument('--output',type=Path,required=True)
p.add_argument('--jets',default='f18,f18e,ea18g')
p.add_argument('--views',default='')
a=p.parse_args();exe=a.executable.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
config=out/'config';config.mkdir(exist_ok=True)
env={**os.environ,'DISPLAY':':0','LIBGL_ALWAYS_SOFTWARE':'1','APPDATA':str(config),'XDG_CONFIG_HOME':str(config)}
startup=None
if os.name=='nt':
    startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW;startup.wShowWindow=0
views=[('forward',['--look','0,0']),('crown',['--look','0,18']),
       ('compass',['--look','35,2']),('panel',['--look','0,-32']),
       ('profile',['--chase','--look','90,0','--zoom','1.2']),
       ('front',['--chase','--look','145,20','--zoom','1.2']),
       ('top',['--chase','--look','180,65','--zoom','1.7']),
       ('nozzles',['--chase','--look','0,0','--zoom','0.85']),
       ('stores-bottom',['--mission','dogfight','--chase','--look','170,-25','--zoom','1.2']),
       ('gear',['--chase','--look','145,-10','--zoom','1.2','--input','gear,flaps','--sim','8','--mach','.25']),
       ('brake',['--chase','--look','145,20','--zoom','1.2','--input','brake','--sim','1']),
       ('structural',['--chase','--look','145,35','--zoom','1.0','--stress-preview']),
       ('weapon',['--chase','--look','145,35','--zoom','1.0','--damage-preview'])]
report=[]
for jet in a.jets.split(','):
    for name,flags in views:
        if a.views and name not in a.views.split(','):continue
        shot=out/f'{jet}-{name}.png'
        r=subprocess.run([str(exe),'--autostart','--no-performance','--jet',jet,'--size','1440x900','--shot',str(shot),*flags],env=env,capture_output=True,text=True,timeout=90,startupinfo=startup)
        (out/f'{jet}-{name}.log').write_text(r.stdout+r.stderr)
        assert r.returncode==0 and shot.is_file() and shot.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n',(jet,name,r.stderr)
        report.append({'jet':jet,'view':name,'file':shot.name,'options':flags})
        print('PASS capture',jet,name,flush=True)
(out/'captures.json').write_text(json.dumps({'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'captures':report},indent=2)+'\n')
print('PASS',len(report),'native review captures',flush=True)
