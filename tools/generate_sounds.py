from pathlib import Path
import math,wave,struct
r=Path(__file__).resolve().parents[1]
for name,notes in [('start',[(0,660),(.075,880)]),('stop',[(0,660),(.07,495)])]:
 rate=44100;length=.24;data=[]
 for n in range(round(length*rate)):
  t=n/rate;v=0
  for at,freq in notes:
   age=t-at
   if age>=0:
    env=min(1,age/.012)*math.exp(-age/0.055)*min(1,(length-t)/.025)
    v+=.22*env*(math.sin(math.tau*freq*age)+.08*math.sin(math.tau*2*freq*age))
  data.append(round(max(-1,min(1,v))*32767))
 with wave.open(str(r/'assets'/f'{name}.wav'),'wb') as w:w.setnchannels(1);w.setsampwidth(2);w.setframerate(rate);w.writeframes(struct.pack('<'+'h'*len(data),*data))
(r/'assets/sounds.txt').write_text('Silbenlot 0.6: original synthesized soft two-note chimes. No samples or F1 recordings. Mono PCM16 44100 Hz, 240 ms. Sine + 8% second harmonic, 12 ms attack, 55 ms exponential decay, 25 ms tail fade. Start 660/880 Hz, stop 660/495 Hz. Default independent playback volume 35%. Generator: tools/generate_sounds.py; WAV assets are self contained.\n')
