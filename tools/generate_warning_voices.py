#!/usr/bin/env python3
"""Regenerate original, embedded VMS speech. Requires espeak-ng on PATH."""
from pathlib import Path
import os
import subprocess
import tempfile
import wave
import struct

os.chdir(Path(__file__).resolve().parents[1])
work = tempfile.TemporaryDirectory(prefix='viper-voices-')
out=Path('assets/voices');out.mkdir(parents=True,exist_ok=True)
clips={'f16-pullup':('en-us+f3','Pull up. Pull up.'),'f16-altitude':('en-us+f3','Altitude. Altitude.'),'f16-warning':('en-us+f3','Warning. Warning.'),'f16-caution':('en-us+f3','Caution. Caution.'),'mig-altitude':('ru+f3','Опасная высота.'),'mig-aoa':('ru+f3','Предельный угол атаки.'),'mig-speed':('ru+f3','Предельная скорость.'),'mig-g':('ru+f3','Предельная перегрузка.'),'mig-gear':('ru+f3','Выпусти шасси.'),'mig-fire':('ru+f3','Пожар двигателя.')}
for name,(voice,text) in clips.items():
    temp=Path(work.name)/(name+'.wav')
    subprocess.run(['espeak-ng','-v',voice,'-s','170','-p','55','-a','140','-w',str(temp),text],check=True)
    with wave.open(str(temp)) as w: rate=w.getframerate();raw=w.readframes(w.getnframes())
    data=list(struct.unpack('<'+'h'*(len(raw)//2),raw));peak=max(map(abs,data))
    # Trim dead air and normalise the clip; no copied simulator recordings.
    a=next((i for i,x in enumerate(data) if abs(x)>120),0);b=len(data)-next((i for i,x in enumerate(reversed(data)) if abs(x)>120),0)
    data=data[max(0,a-rate//40):min(len(data),b+rate//12)]
    vals=[max(-32767,min(32767,round(x*24000/max(1,peak)))) for x in data]
    with wave.open(str(out/(name+'.wav')),'wb') as w:
        w.setnchannels(1);w.setsampwidth(2);w.setframerate(rate);w.writeframes(struct.pack('<'+'h'*len(vals),*vals))
    print(name,round(len(vals)/rate,2),flush=True)
