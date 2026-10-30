"""Capture actual paint, opposite sides, materials and articulated seams."""
from pathlib import Path
import argparse,hashlib,json,os,subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('executable',type=Path);p.add_argument('--output',type=Path,required=True)
p.add_argument('--jets',default='f16,mig29,f18e,ea18g');p.add_argument('--views',default='')
p.add_argument('--geometry',type=Path,help='Also export exact game meshes and PNG copies of the embedded atlases for Blender')
a=p.parse_args();exe=a.executable.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=True)
config=out/'config';config.mkdir(exist_ok=True)
env={**os.environ,'DISPLAY':':0','LIBGL_ALWAYS_SOFTWARE':'1','APPDATA':str(config),'XDG_CONFIG_HOME':str(config)}
startup=None
if os.name=='nt':
    startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW;startup.wShowWindow=0
report={'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'captures':[]}
views=[('left',['--look','90,0','--zoom','.85']),('right',['--look','-90,0','--zoom','.85']),
       ('top',['--look','180,65','--zoom','1.4']),('bottom',['--look','180,-65','--zoom','1.4']),
       ('quarter',['--look','145,25','--zoom','.9']),('nozzles',['--look','0,5','--zoom','.6'])]
for name,sim in [('controls-mid','.25'),('controls-full','2')]:
    views.append((name,['--look','145,35','--zoom','.95','--mach','.25','--input','pull,right,yawright,flaps,brake','--sim',sim]))
def run(label,args,timeout=120):
    r=subprocess.run([str(x) for x in args],env=env,startupinfo=startup,capture_output=True,text=True,timeout=timeout)
    (out/(label+'.log')).write_text(r.stdout+r.stderr,encoding='utf-8',newline='\n')
    assert r.returncode==0,(label,r.stderr)
for jet in a.jets.split(','):
    for name,flags in views:
        if a.views and name not in a.views.split(','):continue
        shot=out/f'{jet}-{name}.png'
        run(jet+'-'+name,[exe,'--autostart','--chase','--no-performance','--jet',jet,'--size','1440x900','--shot',shot,*flags])
        assert shot.read_bytes()[:8]==b'\x89PNG\r\n\x1a\n'
        report['captures'].append({'jet':jet,'view':name,'file':shot.name,'options':flags})
        print('PASS',jet,name,flush=True)
(out/'captures.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8',newline='\n')
if a.geometry:
    from PIL import Image
    root=a.geometry.resolve();root.mkdir(parents=True,exist_ok=True)
    run('mesh-export',[exe,'--export-cockpit',root])
    repo=Path(__file__).resolve().parents[1]
    for jet,file in [('f16','f16'),('mig29','mig29'),('f18e','superhornet'),('ea18g','growler')]:
        Image.open(repo/'assets/models'/(file+'-skin.qoi')).save(root/(jet+'-skin.png'))
    proof={'sha256':report['sha256'],'exports':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in root.glob('*.vrm')}}
    (root/'mesh-provenance.json').write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8',newline='\n')
print('PASS',len(report['captures']),'native paint captures',flush=True)
