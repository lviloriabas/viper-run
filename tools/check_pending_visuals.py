"""Capture pending-task changes using the native game, with isolated settings."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--game',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    game=args.game.resolve();out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    config=out/'config';config.mkdir(exist_ok=True)
    env={**os.environ,'APPDATA':str(config),'XDG_CONFIG_HOME':str(config),
         'DISPLAY':':0','LIBGL_ALWAYS_SOFTWARE':'1'}
    startup=None
    if os.name=='nt':
        startup=subprocess.STARTUPINFO();startup.dwFlags|=subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow=0
    cases=[]
    for jet in ['f16','mig29','f18','su27','f18e','ea18g']:
        cases.extend([
            (jet,'panel',['--look','0,-28']),
            (jet,'outside-canopy',['--chase','--look','115,32','--zoom','.65']),
            (jet,'damage',['--damage-preview','--chase','--look','145,20','--zoom','1.1']),
            (jet,'drogue',['--input','eject','--sim','1.7','--chase','--look','110,5']),
        ])
    for label,flags in [('slats-rest',[]),('slats-down',['--input','gear,flaps','--mach','.3','--sim','6']),
                        ('brake',['--input','brake','--sim','1'])]:
        cases.append(('su27',label,['--chase','--look','145,35','--zoom','1.3',*flags]))
    report=[]
    for jet,label,flags in cases:
        shot=out/f'{jet}-{label}.png'
        cmd=[str(game),'--autostart','--no-performance','--jet',jet,'--size','1280x800',
             '--shot',str(shot),*flags]
        result=subprocess.run(cmd,env=env,cwd=game.parent,startupinfo=startup,capture_output=True,text=True,timeout=90)
        (out/f'{jet}-{label}.log').write_text(result.stdout+result.stderr,encoding='utf-8')
        if result.returncode or not shot.is_file():raise RuntimeError(f'capture failed: {jet} {label}')
        report.append({'jet':jet,'view':label,'image':shot.name})
        print(f'PASS capture {jet} {label}',flush=True)
    (out/'captures.json').write_text(json.dumps({'sha256':hashlib.sha256(game.read_bytes()).hexdigest(),
                                               'captures':report},indent=2),encoding='utf-8')

if __name__=='__main__':main()
