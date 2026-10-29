#!/usr/bin/env python3
"""Exercise real game instances and the directory server, with no microphone capture."""
from pathlib import Path
import os, subprocess, time, socket, argparse, tempfile
ROOT=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description="Real-process multiplayer smoke tests; loopback only, microphone stays disabled.")
suffix="windows-x86_64.exe" if os.name=="nt" else "linux-x86_64"
parser.add_argument("--game",default=str(ROOT/"dist"/("viper-run-"+suffix)))
parser.add_argument("--server",default=str(ROOT/"dist"/("viper-server-"+suffix)))
parser.add_argument("--output",type=Path)
parser.add_argument("--modes",nargs="+",choices=["dogfight","teams","combat","strike","escort","defense"],default=["dogfight","teams","combat","strike","escort","defense"])
parser.add_argument("--transport",choices=["direct","tcp","embedded","both","all"],default="all")
args=parser.parse_args()
temporary=tempfile.TemporaryDirectory(prefix="viper-multiplayer-") if args.output is None else None
OUT=args.output or Path(temporary.name)
OUT.mkdir(parents=True,exist_ok=True)
EXE=str(Path(args.game).resolve())
SERVER=str(Path(args.server).resolve())
env={**os.environ,'DISPLAY':':0','LIBGL_ALWAYS_SOFTWARE':'1'}
def run_case(mode,relay=False,embedded=False):
    stem=mode+('-embedded' if embedded else '-tcp-relay' if relay else '-direct')
    inv=OUT/(stem+'.invite'); inv.unlink(missing_ok=True)
    hreport=OUT/(stem+'-host.txt');creport=OUT/(stem+'-client.txt')
    hreport.unlink(missing_ok=True);creport.unlink(missing_ok=True)
    hub=None;hubarg=[]
    if relay:
        probe=socket.socket(socket.AF_INET,socket.SOCK_STREAM);probe.bind(('127.0.0.1',0));port=probe.getsockname()[1];probe.close()
        hub=subprocess.Popen([SERVER,f'127.0.0.1:{port}'],cwd=ROOT,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,env=env)
        try:
            deadline=time.monotonic()+4
            while time.monotonic()<deadline:
                try:
                    with socket.create_connection(('127.0.0.1',port),timeout=.2): break
                except OSError:
                    assert hub.poll() is None, 'relay server failed to start'
                    time.sleep(.05)
            else: raise AssertionError('relay server did not listen on TCP')
        except BaseException:
            hub.terminate();hub.wait(timeout=3)
            raise
        hubarg=['--directory',f'127.0.0.1:{port}']
    common=['--size','640x360','--no-performance','--weapon','gun','--input','fire']
    with (OUT/(stem+'-host.log')).open('w') as hl,(OUT/(stem+'-client.log')).open('w') as cl:
        host=subprocess.Popen([EXE,'--host-game' if embedded else '--host','127.0.0.1:0','--callsign','Lead','--jet','f18','--online-mode',mode,'--launch-when-ready','--invite-file',str(inv),'--net-report',str(hreport),'--exit-after','13',*hubarg,*common],cwd=ROOT,env=env,stdout=hl,stderr=hl)
        client=None
        try:
            deadline=time.monotonic()+8
            while not inv.exists() and time.monotonic()<deadline and host.poll() is None: time.sleep(.1)
            assert inv.exists(),f'{stem}: invitation not written'
            invite=inv.read_text().strip()
            assert invite.startswith('relay+tcp:') if relay or embedded else not invite.startswith('relay'), invite
            client=subprocess.Popen([EXE,'--join',invite,'--callsign','Wing','--jet','su27','--ready','--net-report',str(creport),'--exit-after','9',*common],cwd=ROOT,env=env,stdout=cl,stderr=cl)
            assert client.wait(timeout=22)==0,f'{stem}: client exited abnormally'
            assert host.wait(timeout=12)==0,f'{stem}: host exited abnormally'
        finally:
            if client and client.poll() is None: client.terminate();client.wait(timeout=3)
            if host.poll() is None: host.terminate();host.wait(timeout=3)
            if hub: hub.terminate();hub.wait(timeout=3)
            inv.unlink(missing_ok=True)
    def read(path):
        assert path.exists(),f'{stem}: no report {path}'
        return dict(line.split('=',1) for line in path.read_text().splitlines() if '=' in line)
    h,c=read(hreport),read(creport)
    assert h['started']==c['started']=='true',(stem,h,c)
    assert int(h['commands'])>10 and int(c['frames'])>10,(stem,h,c)
    assert int(c['tick'])>60,(stem,h,c)
    assert c['jet']=='SU-27SK FLANKER-B',(stem,c)
    assert int(c['rounds'])<150,(stem,'remote gun inventory did not change',c)
    assert h['mic_capturing']==c['mic_capturing']=='false',(stem,'unexpected microphone capture')
    if embedded: assert h['embedded_server']=='true' and c['embedded_server']=='false',(stem,h,c)
    print('PASS',stem,'host commands',h['commands'],'client frames',c['frames'],'rounds',c['rounds'],'objective',c['objective'],flush=True)
for mode in args.modes:
    if args.transport in ('direct','both','all'): run_case(mode,relay=False)
    if args.transport in ('tcp','both','all'): run_case(mode,relay=True)
    if args.transport in ('embedded','all'): run_case(mode,embedded=True)
print('PASS all requested real-process multiplayer modes',flush=True)
