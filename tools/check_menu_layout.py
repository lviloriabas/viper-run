from pathlib import Path
import argparse,subprocess,json,time,hashlib
from concurrent.futures import ThreadPoolExecutor, as_completed

def cases(full=False):
    missions=['gates','dogfight','strike','airfield','free']
    jets=['f16','mig29','f18','su27']
    if full:
        for jet in jets:
            for mission in missions:
                for screen in ['menu','loadout']:
                    yield f'{screen}-{mission}-{jet}', ['--screen',screen,'--mission',mission,'--jet',jet]
    else:
        for mission in missions:
            for screen in ['menu','loadout']:
                yield f'{screen}-{mission}', ['--screen',screen,'--mission',mission]
    if full:
        for i in range(4):
            for jet in jets:
                yield f'threats-{i}-{jet}',['--screen','loadout','--mission','free','--jet',jet,'--layout-scene',f'threats-{i}']
            yield f'fuel-min-{i}',['--screen','loadout','--mission','free','--jet',jets[i],'--layout-scene','fuel-min']
        for mode in ['dogfight','teams','combat','strike','escort','defense']:
            yield 'network-'+mode,['--screen','network','--layout-scene','network-'+mode]
    if full:
        for lesson,steps in [('controls',9),('flight',4),('weapons',6),('takeoff',4),('landing',4)]:
            for stage in range(steps):
                yield f'training-{lesson}-{stage}',['--tutorial',lesson,'--layout-scene',f'training-{stage}']
    yield 'school',['--screen','school']
    yield 'school-passed',['--screen','school','--layout-scene','school-passed']
    for scene in ['', 'fields','browser','lobby','lobby-client','lobby-voice','lobby-embedded','lobby-host-help','create','create-external','nearby','help','browser-empty','server-error']:
        yield 'network-'+(scene or 'setup'),['--screen','network']+(['--layout-scene',scene] if scene else [])
    for scene in ['keys','mouse','classic-invert','rebind']:
        yield 'config-'+scene,['--screen','menu','--controls','--layout-scene',scene]
    yield 'paused-config',['--autostart','--controls']
    if full:
        for mission in missions:
            for screen in ['win','crash','timeout','shotdown']:
                yield f'{screen}-{mission}',['--screen',screen,'--mission',mission]
        yield 'lesson-passed',['--screen','win','--layout-scene','lesson-passed']
        yield 'lesson-failed',['--screen','crash','--layout-scene','lesson-failed']
    else:
        for screen in ['win','crash','timeout','shotdown']:
            yield screen,['--screen',screen]

def main():
    p=argparse.ArgumentParser();p.add_argument('--exe',type=Path,required=True);p.add_argument('--out',type=Path,required=True);p.add_argument('--sizes',default='640x360,1280x720,1920x1080');p.add_argument('--full',action='store_true');p.add_argument('--filter',default='');p.add_argument('--prompts',default='keyboard,controller');args=p.parse_args()
    exe=args.exe.resolve();out=args.out.resolve();out.mkdir(parents=True,exist_ok=True)
    (out/'layout-metadata.json').write_text(json.dumps({'exe':exe.name,'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'sizes':args.sizes,'prompts':args.prompts,'full':args.full,'filter':args.filter},indent=2),encoding='utf-8')
    results=[];start=time.time()
    jobs=[(size,pad,name,flags) for size in args.sizes.split(',') for pad in args.prompts.split(',') for name,flags in cases(args.full) if not args.filter or args.filter in name]
    def check(job):
        size,pad,name,flags=job
        label=f'{name}-{size}-{pad}';shot=out/(label+'.png')
        cmd=[str(exe),'--layout-check','--shot',str(shot),'--frames','1','--size',size,'--no-performance',*flags]+(['--pad'] if pad=='controller' else [])
        try:
            r=subprocess.run(cmd,cwd=exe.parent,capture_output=True,text=True,timeout=45)
            log=r.stdout+r.stderr;passed=r.returncode==0 and shot.is_file() and 'PASS menu layout:' in log
        except subprocess.TimeoutExpired as e:passed=False;log=str(e)
        (out/(label+'.txt')).write_text(log,encoding='utf-8')
        return {'name':label,'pass':passed,'issues':[line for line in log.splitlines() if line.startswith('LAYOUT ')]}
    with ThreadPoolExecutor(max_workers=3) as executor:
      for future in as_completed([executor.submit(check,job) for job in jobs]):
        result=future.result();results.append(result)
        if not result['pass']:print('FAIL',result['name'],*result['issues'][:6],sep='\n',flush=True)
        elif len(results)%20==0:print(f'{len(results)}/{len(jobs)} scenes checked ({time.time()-start:.0f}s)',flush=True)
    results.sort(key=lambda r:r['name'])
    (out/'layout-results.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
    bad=[x for x in results if not x['pass']]
    print(f'{len(results)-len(bad)}/{len(results)} menu layouts passed, {time.time()-start:.1f}s',flush=True)
    raise SystemExit(bool(bad))
if __name__=='__main__':main()
