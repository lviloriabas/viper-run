/* F-16C Block 50, procedural game cockpit. Metres in the exterior's body frame.
   Reference photographs and the limits of this model are in docs/F16-REFERENCES.md. */
(() => {
  "use strict";
  const C = window.__VIPER_CORE;
  const D = Math.PI / 180, TAU = Math.PI * 2;
  const add = (a,b) => a.map((v,i) => v+b[i]);
  const sub = (a,b) => a.map((v,i) => v-b[i]);
  const mul = (a,s) => a.map(v => v*s);
  const dot = (a,b) => a.reduce((s,v,i) => s+v*b[i],0);
  const cross = (a,b) => [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  const norm = a => mul(a,1/(Math.hypot(...a)||1));
  const mix = (a,b,t) => add(mul(a,1-t),mul(b,t));
  const clamp = C.clamp;
  const pad = (n,l=3) => String(Math.max(0,Math.round(n))).padStart(l,"0");
  const wrap = n => ((n%360)+360)%360;
  const ft = n => n*3.28084;
  const green = "#98ffa3", white = "#dddccf";
  const textures = new Map();
  let ctx, W, H, focal, eye, right, up, forward, queue, textureClock=0;
  let raster=null,modelOffset=0;
  // The eye is aft of the hips, consistent with the reclined F-16 seat.
  const EYE = [0,1.43,-2.05];

  function surface(name,w,h,draw) {
    if (!textures.has(name)) { const c=document.createElement("canvas");c.width=w;c.height=h;textures.set(name,c); }
    const c=textures.get(name), t=c.getContext("2d");
    if(c.lastDraw!==undefined&&textureClock-c.lastDraw<.075)return c;
    t.setTransform(1,0,0,1,0,0);t.clearRect(0,0,w,h);draw(t,w,h);
    c.lastDraw=textureClock;c.revision=(c.revision||0)+1;return c;
  }
  function text(t,s,x,y,size=15,color=white,align="left") {
    t.font=`${size}px Consolas, "DejaVu Sans Mono", monospace`;
    t.fillStyle=color;t.textAlign=align;t.textBaseline="middle";t.fillText(s,x,y);
  }
  function line(t,a,b,color=green,width=1) {
    t.strokeStyle=color;t.lineWidth=width;t.beginPath();t.moveTo(...a);t.lineTo(...b);t.stroke();
  }
  function dial(t,x,y,r,value,min,max,label,unit,red=Infinity) {
    t.fillStyle="#101619";t.beginPath();t.arc(x,y,r+4,0,TAU);t.fill();
    t.strokeStyle="#76817c";t.lineWidth=2;t.stroke();
    t.fillStyle="#020505";t.beginPath();t.arc(x,y,r,0,TAU);t.fill();
    for(let i=0;i<=10;i++) {
      const a=(-220+i*26)*D,c=Math.cos(a),s=Math.sin(a);
      line(t,[x+c*r*.74,y+s*r*.74],[x+c*r*.91,y+s*r*.91],i>=red?"#ff6654":white,2);
      if(i%2===0) text(t,String(Math.round(min+(max-min)*i/10)),x+c*r*.59,y+s*r*.59,r*.15,white,"center");
    }
    const a=(-220+clamp((value-min)/(max-min),0,1)*260)*D;
    line(t,[x-Math.cos(a)*r*.15,y-Math.sin(a)*r*.15],[x+Math.cos(a)*r*.75,y+Math.sin(a)*r*.75],"#f8f3d5",3);
    t.fillStyle="#9da79b";t.beginPath();t.arc(x,y,4,0,TAU);t.fill();
    text(t,label,x,y-r*.20,r*.18,white,"center");
    text(t,`${Math.round(value)} ${unit}`,x,y+r*.42,r*.17,green,"center");
  }
  function project(p) {
    const d=sub(p,eye),z=dot(d,forward);
    return {x:W*.5+dot(d,right)*focal/z,y:H*.5-dot(d,up)*focal/z,z};
  }
  function clipNear(points) {
    const out=[];
    for(let i=0;i<points.length;i++) {
      const a=points[i],b=points[(i+1)%points.length],za=dot(sub(a,eye),forward),zb=dot(sub(b,eye),forward);
      if(za>=.045)out.push(a);
      if((za>=.045)!==(zb>=.045))out.push(mix(a,b,(.045-za)/(zb-za)));
    }return out;
  }
  function polygon(points,color,texture=null,stroke=null) {
    if(modelOffset)points=points.map(p=>[p[0],p[1],p[2]+modelOffset]);
    const clipped=clipNear(points);if(clipped.length<3)return;
    const projected=clipped.map(project);
    if(projected.every(p=>p.x<0)||projected.every(p=>p.x>W)||projected.every(p=>p.y<0)||projected.every(p=>p.y>H))return;
    queue.push({points:clipped,texturePoints:points,projected,color,texture,stroke,depth:projected.reduce((s,p)=>s+p.z,0)/projected.length});
  }
  function box(c,size,color=[46,51,51]) {
    const v=[];for(let z of [-1,1])for(let y of [-1,1])for(let x of [-1,1])v.push([c[0]+x*size[0]/2,c[1]+y*size[1]/2,c[2]+z*size[2]/2]);
    const faces=[[0,1,3,2],[5,4,6,7],[4,0,2,6],[1,5,7,3],[2,3,7,6],[4,5,1,0]];
    const shades=[.8,.57,.7,.9,1.18,.5];
    faces.forEach((f,i)=>polygon(f.map(j=>v[j]),`rgb(${color.map(n=>Math.round(n*shades[i])).join(",")})`));
  }
  function capsule(a,b,r1,r2,color=[85,95,67],sides=12) {
    const axis=norm(sub(b,a)),r=norm(cross(axis,Math.abs(axis[1])>.9?[1,0,0]:[0,1,0])),u=cross(axis,r);
    const ring=(p,rad,i)=>add(p,add(mul(r,rad*Math.cos(i*TAU/sides)),mul(u,rad*Math.sin(i*TAU/sides))));
    for(let i=0;i<sides;i++) {
      const shade=.65+.35*Math.max(0,Math.sin((i+.5)*TAU/sides));
      polygon([ring(a,r1,i),ring(a,r1,i+1),ring(b,r2,i+1),ring(b,r2,i)],`rgb(${color.map(n=>Math.round(n*shade)).join(",")})`);
    }
    polygon(Array.from({length:sides},(_,i)=>ring(b,r2,i)),`rgb(${color.join(",")})`);
  }
  function ellipsoid(c,size,color) {
    const point=(i,j)=>[c[0]+size[0]*Math.sin(i*Math.PI/8)*Math.cos(j*TAU/12),c[1]+size[1]*Math.cos(i*Math.PI/8),c[2]+size[2]*Math.sin(i*Math.PI/8)*Math.sin(j*TAU/12)];
    for(let i=0;i<8;i++)for(let j=0;j<12;j++){
      const shade=.65+.35*Math.max(0,Math.cos((i+.5)*Math.PI/8));
      polygon([point(i,j),point(i,j+1),point(i+1,j+1),point(i+1,j)],`rgb(${color.map(n=>Math.round(n*shade)).join(",")})`);
    }
  }

  // A depth-buffered WebGL layer gives the near-field proper occlusion while
  // Canvas draws collimated HUD strokes at display resolution. Textures remain
  // procedural and require no downloads, libraries or external model files.
  function rasterize() {
    if(!raster){
      const canvas=document.createElement("canvas"),gl=canvas.getContext("webgl2",{alpha:true,antialias:true,premultipliedAlpha:false});
      if(!gl)throw Error("Cockpit requires WebGL 2");
      const shader=(type,source)=>{const s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(s));return s;};
      const program=gl.createProgram();
      gl.attachShader(program,shader(gl.VERTEX_SHADER,`#version 300 es
        precision highp float;
        in vec3 aPosition; in vec4 aColor; in vec2 aUV;
        uniform vec2 uScale; out vec4 vColor; out vec2 vUV;
        void main(){float z=aPosition.z;gl_Position=vec4(aPosition.xy*uScale,1.0001*z-.080004,z);vColor=aColor;vUV=aUV;}`));
      gl.attachShader(program,shader(gl.FRAGMENT_SHADER,`#version 300 es
        precision highp float; in vec4 vColor; in vec2 vUV; uniform sampler2D uTexture; uniform bool uTextured; out vec4 color;
        void main(){color=uTextured?texture(uTexture,vUV):vColor;}`));
      gl.linkProgram(program);if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
      const buffer=gl.createBuffer(),vao=gl.createVertexArray();gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
      for(const [name,count,offset] of [["aPosition",3,0],["aColor",4,12],["aUV",2,28]]){const l=gl.getAttribLocation(program,name);gl.enableVertexAttribArray(l);gl.vertexAttribPointer(l,count,gl.FLOAT,false,36,offset);}
      raster={canvas,gl,program,buffer,vao,scale:gl.getUniformLocation(program,"uScale"),textured:gl.getUniformLocation(program,"uTextured"),textures:new WeakMap()};
    }
    const {gl,canvas,program,buffer,vao}=raster,dpr=Math.min(window.devicePixelRatio||1,2);
    if(canvas.width!==Math.round(W*dpr)||canvas.height!==Math.round(H*dpr)){canvas.width=Math.round(W*dpr);canvas.height=Math.round(H*dpr);}
    gl.viewport(0,0,canvas.width,canvas.height);gl.clearColor(0,0,0,0);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT);
    gl.useProgram(program);gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.uniform2f(raster.scale,2*focal/W,2*focal/H);
    gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.disable(gl.CULL_FACE);gl.disable(gl.BLEND);gl.depthMask(true);
    const rgba=str=>{
      if(str[0]==="#")return [parseInt(str.slice(1,3),16)/255,parseInt(str.slice(3,5),16)/255,parseInt(str.slice(5,7),16)/255,1];
      const n=str.match(/[\d.]+/g).map(Number);return [n[0]/255,n[1]/255,n[2]/255,n[3]??1];
    };
    const vertex=(arr,p,color,uv=[0,0])=>{const d=sub(p,eye);arr.push(dot(d,right),dot(d,up),dot(d,forward),...color,...uv);};
    const opaque=[],transparent=[];
    for(const item of queue){const color=rgba(item.color),arr=color[3]<1?transparent:opaque;
      for(let i=1;i<item.points.length-1;i++)for(const p of [item.points[0],item.points[i],item.points[i+1]])vertex(arr,p,color);
    }
    const draw=arr=>{gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(arr),gl.DYNAMIC_DRAW);gl.drawArrays(gl.TRIANGLES,0,arr.length/9);};
    gl.uniform1i(raster.textured,0);draw(opaque);
    gl.uniform1i(raster.textured,1);
    for(const item of queue){
      if(!item.texture)continue;const img=item.texture;let tex=raster.textures.get(img);
      if(!tex){tex={id:gl.createTexture(),revision:-1};raster.textures.set(img,tex);gl.bindTexture(gl.TEXTURE_2D,tex.id);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);}
      gl.bindTexture(gl.TEXTURE_2D,tex.id);
      if(tex.revision!==img.revision){gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,img);tex.revision=img.revision;}
      const arr=[],corners=item.texturePoints,uv=[[0,0],[1,0],[1,1],[0,1]];
      for(const i of [0,1,2,0,2,3])vertex(arr,corners[i],[1,1,1,1],uv[i]);draw(arr);
    }
    gl.uniform1i(raster.textured,0);gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.depthMask(false);draw(transparent);gl.depthMask(true);
    ctx.drawImage(canvas,0,0,W,H);
  }
  // Four corners are top-left, top-right, bottom-right, bottom-left, in body coordinates.
  function panel(x,y,z,w,h,texture) {
    polygon([[x-w/2,y+h/2,z],[x+w/2,y+h/2,z],[x+w/2,y-h/2,z],[x-w/2,y-h/2,z]],"#1a2020",texture);
  }
  function mfdTexture(name,a,m,time) {
    return surface(name,384,384,(t,w,h)=>{
      t.fillStyle="#171e21";t.fillRect(0,0,w,h);t.strokeStyle="#626d6b";t.strokeRect(1,1,w-2,h-2);
      t.fillStyle="#020907";t.fillRect(37,37,310,310);
      for(let i=0;i<5;i++)for(let side=0;side<4;side++) {
        const p=60+i*65;t.fillStyle="#343c3e";
        if(side<2)t.fillRect(p-9,side?358:7,18,18);else t.fillRect(side===2?7:358,p-9,18,18);
      }
      text(t,name,192,53,15,green,"center");text(t,"OVRD",77,53,12);text(t,"CNTL",303,53,12,green,"center");
      text(t,"SWAP",78,331,12);text(t,"FCR",145,331,12);text(t,"HSD",212,331,12);text(t,"DTE",289,331,12);
      if(name==="HSD") {
        const center=[192,222],range=9260;
        for(const r of [55,110,145]){t.strokeStyle="#173e33";t.beginPath();t.arc(...center,r,0,TAU);t.stroke();}
        text(t,"5 NM",321,83,12,green,"right");
        const hdg=a.heading*D;
        const nav=p=>{const d=sub(p,a.pos);return [center[0]+(d[0]*Math.cos(hdg)+d[2]*Math.sin(hdg))*145/range,center[1]-(d[0]*Math.sin(hdg)-d[2]*Math.cos(hdg))*145/range];};
        t.save();t.beginPath();t.rect(40,70,304,240);t.clip();
        for(let i=0;i<m.gates.length;i++){
          const p=nav(m.gates[i].pos),next=m.gates[i+1];
          if(next)line(t,p,nav(next.pos),"#345e9a",2);
          t.strokeStyle=i===m.gateIndex?white:green;t.beginPath();t.arc(...p,i===m.gateIndex?7:4,0,TAU);t.stroke();text(t,String(i+1),p[0]+10,p[1],12);
        }t.restore();
        line(t,[180,230],[192,212],green,2);line(t,[192,212],[204,230],green,2);line(t,[180,230],[204,230],green,2);
        text(t,`${pad(a.heading)} HDG`,61,292,12);text(t,`STPT ${pad(Math.min(m.gateIndex+1,m.gates.length),2)}`,321,292,12,green,"right");
      } else if(name==="FCR") {
        text(t,"CRM  RWS  NORM",192,80,14,green,"center");text(t,"40",53,135,14);
        for(let i=1;i<4;i++)line(t,[72,100+i*49],[322,100+i*49],"#153b2b");
        line(t,[72,99],[72,294],green);line(t,[322,99],[322,294],green);
        const sx=78+(time*.24%1)*238;line(t,[sx,103],[sx,288],"#4d9960");
        text(t,"NO TRACK",192,195,16,green,"center");text(t,`${pad(a.airspeedKt)} C`,88,288,13);text(t,`${pad(ft(a.pos[1])/100,3)} ALT`,310,288,13,green,"right");
      } else {
        const rows=[["RPM",`${(a.rpm||0).toFixed(1)} %`],["FTIT",`${pad(a.ftit||0)} C`],["NOZZLE",`${pad(a.nozzle||0,2)} %`],["FUEL",`${pad(a.fuel*2.20462,4)} LB`],["FLOW",`${pad((a.fuelFlow||0)*7936.64,5)} PPH`],["G / AOA",`${a.gSmooth.toFixed(1)} / ${a.alphaDeg.toFixed(1)}`]];
        rows.forEach((r,i)=>{text(t,r[0],58,106+i*32,16);text(t,r[1],328,106+i*32,16,green,"right");});
      }
    });
  }
  function instruments(a,m,time) {
    const engine=surface("engine",220,640,(t)=>{
      t.fillStyle="#3a4242";t.fillRect(0,0,220,640);
      dial(t,110,80,65,a.rpm||0,0,110,"RPM","%",10);
      dial(t,110,238,65,a.ftit||0,200,1200,"FTIT","C",9);
      dial(t,110,396,65,a.nozzle||0,0,100,"NOZ","%",11);
      dial(t,110,554,65,a.oilPsi||0,0,100,"OIL","PSI",11);
    });
    panel(.56,.52,-4.13,.15,.53,engine);
    const ded=surface("ded",510,180,(t)=>{
      t.fillStyle="#020804";t.fillRect(0,0,510,180);
      const gate=m.gates[Math.min(m.gateIndex,m.gates.length-1)];const distance=Math.hypot(...sub(gate.pos,a.pos))/1852;
      const lines=[`UHF  305.00    STPT ${pad(Math.min(m.gateIndex+1,m.gates.length),2)}`,"VHF  127.50    * *",`M ${a.mach.toFixed(2)}       ${pad(a.heading)} DEG`,`DIST ${distance.toFixed(1)} NM   GPS OK`,`TIME ${pad(Math.floor(m.elapsed/60),2)}:${pad(m.elapsed%60,2)}    FUEL ${pad(a.fuel*2.20462,4)}`];
      lines.forEach((s,i)=>text(t,s,10,18+i*35,24,green));
    });panel(.325,1.075,-3.99,.28,.10,ded);
    const icp=surface("icp",320,330,(t)=>{
      t.fillStyle="#30383b";t.fillRect(0,0,320,330);
      const labels=[["COM1","COM2","IFF","LIST"],["1 T-ILS","2 ALOW","3","RCL"],["4 STPT","5 CRUS","6 TIME","ENTR"],["7 MARK","8 FIX","9 A-CAL","DCS"],["A-A","0 M-SEL","A-G","DRIFT"]];
      labels.forEach((row,y)=>row.forEach((s,x)=>{t.fillStyle="#12191b";t.fillRect(10+x*78,10+y*60,68,48);t.strokeStyle="#707b74";t.strokeRect(10+x*78,10+y*60,68,48);text(t,s,44+x*78,34+y*60,11,white,"center");}));
      text(t,"BRT     SYM    RET DEPR",160,319,12,white,"center");
    });panel(0,1.025,-3.99,.235,.24,icp);
    const lower=surface("flight",620,420,(t)=>{
      t.fillStyle="#333c3e";t.fillRect(0,0,620,420);
      // ADI sphere: horizon follows attitude, no artificial upright clamp.
      t.save();t.beginPath();t.arc(158,113,82,0,TAU);t.clip();t.translate(158,113);t.rotate(-a.bank*D);t.translate(0,a.pitch*2.2);
      t.fillStyle="#547a8b";t.fillRect(-200,-400,400,400);t.fillStyle="#835c36";t.fillRect(-200,0,400,400);line(t,[-200,0],[200,0],white,3);
      for(let p=-60;p<=60;p+=10)if(p){line(t,[-25,-p*2.2],[25,-p*2.2],white);text(t,String(Math.abs(p)),37,-p*2.2,12,white);}
      t.restore();line(t,[102,113],[144,113],"#ffc14e",4);line(t,[172,113],[214,113],"#ffc14e",4);line(t,[158,108],[158,122],"#ffc14e",3);
      t.strokeStyle="#8b9690";t.lineWidth=6;t.beginPath();t.arc(158,113,84,0,TAU);t.stroke();
      dial(t,450,113,82,ft(a.pos[1])%10000,0,10000,"ALT","FT");
      text(t,pad(ft(a.pos[1]),5),450,178,20,green,"center");
      t.fillStyle="#030606";t.beginPath();t.arc(307,310,90,0,TAU);t.fill();
      for(let b=0;b<360;b+=10){const r=(b-a.heading-90)*D;line(t,[307+Math.cos(r)*75,310+Math.sin(r)*75],[307+Math.cos(r)*86,310+Math.sin(r)*86],white,2);if(b%30===0)text(t,String(b/10),307+Math.cos(r)*62,310+Math.sin(r)*62,14,white,"center");}
      line(t,[307,265],[307,350],"#ffc15a",3);line(t,[285,310],[329,310],"#ffc15a",3);text(t,"HSI",177,310,18);
      text(t,`VVI ${Math.round(a.vs*196.85)}`,40,370,17);text(t,`AOA ${a.alphaDeg.toFixed(1)}`,475,310,17,green,"center");
    });panel(0,.525,-4.14,.43,.29,lower);
    const warn=surface("warn",320,180,(t)=>{
      t.fillStyle="#202729";t.fillRect(0,0,320,180);
      const labels=[["FLCS",a.alphaDeg>25],["FUEL LOW",a.fuel<450],["ENGINE",a.fuel<=0],["OIL",(a.oilPsi||60)<20],["ALT LOW",a.agl<100],["SPEED BRK",a.brake>.1]];
      labels.forEach(([s,on],i)=>{const x=(i%2)*160,y=Math.floor(i/2)*60;t.fillStyle=on?"#d49638":"#0c1414";t.fillRect(x+3,y+3,154,54);text(t,s,x+80,y+30,16,on?"#1c1405":"#6b7569","center");});
    });panel(.51,.875,-4.075,.22,.12,warn);
    const rwr=surface("rwr",320,320,(t)=>{
      t.fillStyle="#232c2e";t.fillRect(0,0,320,320);
      t.fillStyle="#020805";t.beginPath();t.arc(160,148,126,0,TAU);t.fill();
      for(const r of [42,85,120]){t.strokeStyle="#315340";t.lineWidth=2;t.beginPath();t.arc(160,148,r,0,TAU);t.stroke();}
      line(t,[42,148],[278,148],"#466d50");line(t,[160,28],[160,268],"#466d50");
      text(t,"RWR",160,300,17,white,"center");text(t,"N",160,43,18,green,"center");
      text(t,"NO EMITTER",160,150,16,green,"center");
    });panel(-.49,1.005,-4.075,.19,.19,rwr);
    const aux=surface("aux",320,250,(t)=>{
      t.fillStyle="#333c3d";t.fillRect(0,0,320,250);
      dial(t,160,98,76,a.airspeedKt,0,800,"KCAS","KT");
      text(t,`FUEL ${pad(a.fuel*2.20462,4)} LB`,160,205,22,green,"center");
      text(t,`G ${a.gSmooth.toFixed(1)}   ${pad(a.heading)} HDG`,160,236,17,white,"center");
    });panel(-.52,.625,-4.13,.18,.24,aux);
    // AOA indexer and master caution, placed beside the HUD rather than on it.
    const aoa=surface("indexer",80,240,t=>{
      t.fillStyle="#121b1c";t.fillRect(0,0,80,240);
      text(t,"V",40,40,48,a.alphaDeg>14?"#ff5f46":"#60382c","center");
      text(t,"O",40,120,40,a.alphaDeg>=11&&a.alphaDeg<=14?green:"#274534","center");
      text(t,"^",40,205,48,a.alphaDeg<11?"#e9ba53":"#5c4e2e","center");
    });panel(-.20,1.285,-4.12,.042,.13,aoa);
  }
  function consoles(a,hands) {
    for(const side of [-1,1]) {
      box([side*.50,.24,-2.94],[.25,.27,1.75],[44,50,51]);
      box([side*.65,.66,-2.93],[.075,.12,2.4],[31,39,39]);
      for(let j=0;j<5;j++){
        const z=-3.72+j*.29;
        box([side*.51,.391,z],[.23,.016,.25],[52,60,61]);
        for(let k=0;k<3;k++)capsule([side*.51+(k-1)*.055,.41,z],[side*.51+(k-1)*.055,.455,z-.018],.008,.006,[163,172,162],6);
      }
    }
    const t=hands.throttle,throttle=[-.48,.53,-3.08-t*.20];
    box([-.48,.412,-3.20],[.13,.018,.47],[16,20,21]);
    capsule([-.48,.43,throttle[2]],throttle,.027,.025,[68,77,78]);
    capsule(add(throttle,[-.075,0,0]),add(throttle,[.075,0,0]),.040,.040,[35,43,46]);
    box([-.425,.552,throttle[2]-.015],[.018,.015,.024],[165,174,163]);
    const stick=[.47+hands.roll*.008,.55,-3.04-hands.pitch*.009];
    box([.47,.42,-3.04],[.13,.03,.17],[17,23,24]);
    capsule([.47,.43,-3.04],stick,.025,.028,[39,47,48]);
    box(add(stick,[0,.04,0]),[.07,.10,.073],[39,45,46]);
    box(add(stick,[-.038,.063,-.009]),[.024,.018,.025],[155,162,150]);
    box(add(stick,[.012,.096,0]),[.025,.012,.025],[187,64,41]);
    // Reclined ACES II seat. The pilot's head is deliberately omitted at the eye.
    box([0,.13,-2.53],[.59,.13,.63],[36,45,40]);
    capsule([0,.20,-2.37],[0,.91,-2.03],.27,.245,[35,43,40]);
    for(const side of [-1,1])capsule([side*.29,.17,-2.43],[side*.29,.84,-2.12],.045,.045,[70,77,69]);
    const hip=[0,.27,-2.59],suit=[101,111,76],glove=[92,103,78];
    capsule(hip,[0,.86,-2.22],.235,.25,suit);
    box([0,.64,-2.39],[.28,.28,.08],[69,83,57]);
    for(const side of [-1,1]){
      const knee=[side*.21,.42,-3.42],ankle=[side*.19,.10,-3.95+side*hands.rudder*.045];
      capsule([side*.16,.27,-2.65],knee,.145,.118,suit);
      ellipsoid(knee,[.12,.12,.13],suit);
      capsule(knee,ankle,.112,.072,suit);
      capsule(ankle,add(ankle,[0,-.035,-.19]),.078,.068,[35,40,35]);
      box([side*.18,.13,ankle[2]-.16],[.15,.035,.18],[83,90,86]);
      // G-suit seams, straps and kneeboard.
      capsule([side*.22,.52,-3.12],[side*.23,.53,-3.37],.018,.016,[72,81,54],6);
    }
    const kneeboard=surface("kneeboard",180,220,t=>{
      t.fillStyle="#b7bca0";t.fillRect(0,0,180,220);text(t,"COLD RIVER",90,24,17,"#283c30","center");
      ["CHECK FUEL","10 STEERPOINTS","KCAS / AGL","F1 / F2 MFD","ALT + MOUSE","C CENTER VIEW"].forEach((s,i)=>text(t,s,10,58+i*25,12,"#283c30"));
    });
    polygon([[-.31,.575,-3.32],[-.10,.575,-3.32],[-.10,.540,-3.05],[-.31,.540,-3.05]],"#69745b",kneeboard);
    // Two shoulder-to-elbow-to-wrist chains remain attached to the controls.
    for(const [side,hand] of [[-1,throttle],[1,add(stick,[0,.04,0])]]){
      const shoulder=[side*.23,.81,-2.25],elbow=[side*.36,.43,-2.62],wrist=add(hand,[0,.015,.075]);
      capsule(shoulder,elbow,.095,.078,suit);capsule(elbow,wrist,.079,.050,suit);
      ellipsoid(elbow,[.079,.080,.080],suit);
      capsule(wrist,add(hand,[0,.026,.005]),.052,.054,glove);
      ellipsoid(add(hand,[0,.035,.006]),[.054,.039,.055],glove);
      for(let i=0;i<4;i++){
        const x=hand[0]+(i-1.5)*.022;
        capsule([x,hand[1]+.057,hand[2]+.030],[x,hand[1]+.056,hand[2]-.023],.012,.011,glove,6);
        capsule([x,hand[1]+.056,hand[2]-.023],[x,hand[1]+.008,hand[2]-.036],.011,.010,glove,6);
      }
      capsule(add(hand,[-side*.045,.025,.01]),add(hand,[-side*.032,.035,-.034]),.017,.014,glove,6);
      capsule(add(wrist,[0,.004,.025]),add(wrist,[0,.004,.047]),.056,.056,[47,55,41],8);
    }
    // Lap belts, buckle and yellow/black ejection handle between the thighs.
    for(const side of [-1,1])capsule([side*.25,.36,-2.67],[0,.42,-2.88],.033,.033,[162,164,135],6);
    box([0,.43,-2.88],[.075,.035,.07],[152,156,143]);
    for(let i=0;i<16;i++){
      const a=i*TAU/16,b=(i+1)*TAU/16;
      capsule([.075*Math.cos(a),.40+.07*Math.sin(a),-2.96],[.075*Math.cos(b),.40+.07*Math.sin(b),-2.96],.012,.012,i%2?[32,32,23]:[229,185,44],6);
    }
  }
  function hud(a,m,cam) {
    const glass=[[-.165,1.76,-3.35],[.165,1.76,-3.35],[.19,1.22,-3.25],[-.19,1.22,-3.25]];
    const clipped=clipNear(glass);if(clipped.length<3)return;
    const p=clipped.map(project);
    ctx.save();ctx.beginPath();ctx.moveTo(p[0].x,p[0].y);p.slice(1).forEach(v=>ctx.lineTo(v.x,v.y));ctx.closePath();ctx.clip();
    const baseFocal=H*.5/Math.tan(cam.fov/2),s=clamp(baseFocal/480,.55,1.7);
    const ax=C.axisSet(a.q),worldDir=d=>{
      const z=dot(d,cam.forward);if(z<=.01)return null;
      return {x:W*.5+dot(d,cam.right)*focal/z,y:H*.5-dot(d,cam.up)*focal/z};
    };
    const anchor=worldDir(ax.forward);if(!anchor){ctx.restore();return;}
    const at=(x,y)=>worldDir(norm(add(ax.forward,add(mul(ax.right,x/baseFocal),mul(ax.up,-y/baseFocal)))));
    const ht=(str,x,y,size=11,align="left")=>{const p=at(x*s,y*s);if(p)text(ctx,str,p.x,p.y,size*s,green,align);};
    const hl=(x,y,x2,y2)=>{const p=at(x*s,y*s),q=at(x2*s,y2*s);if(p&&q)line(ctx,[p.x,p.y],[q.x,q.y],green,1.05);};
    const half=baseFocal*.137/s;
    // NAV symbology: KCAS, MSL barometric altitude, Mach, G, alpha, steerpoint.
    ht(a.gSmooth.toFixed(1),-half+5,-92,11);
    ht(pad(a.airspeedKt),-half+5,0,14);ht("C",-half+8,18,9);
    ht(pad(ft(a.pos[1]),5),half-5,0,14,"right");ht("B",half-5,18,9,"right");
    ht(a.mach.toFixed(2),-half+5,42,10);ht(a.alphaDeg.toFixed(1),-half+5,56,10);
    ht((a.maxG||1).toFixed(1),-half+5,68,8);
    ht("NAV",-half+5,81,10);ht(`R ${pad(ft(a.agl),4)}`,half-5,42,10,"right");
    const gate=m.gates[Math.min(m.gateIndex,m.gates.length-1)],dist=Math.hypot(...sub(gate.pos,a.pos))/1852;
    ht(`${dist.toFixed(1)}>${Math.min(m.gateIndex+1,m.gates.length)}`,half-5,56,10,"right");
    ht(pad(a.heading),0,83,11,"center");
    for(let hd=-20;hd<=20;hd+=5){const n=wrap(Math.round(a.heading/5)*5+hd),diff=((n-a.heading+540)%360)-180;hl(diff*2.4,69,diff*2.4,73);if(n%10===0)ht(pad(n/10,2),diff*2.4,64,8,"center");}
    // Calibrated speed and barometric altitude scales flank the flight path.
    for(let i=-2;i<=2;i++){
      const speed=Math.round(a.airspeedKt/10)*10+i*10,dy=(a.airspeedKt-speed)*.7;
      hl(-half+2,dy,-half+6,dy);
      const alt=Math.round(ft(a.pos[1])/100)*100+i*100,ay=(ft(a.pos[1])-alt)*.07;
      hl(half-2,ay,half-6,ay);
    }
    hl(-9,-75,-3,-75);hl(3,-75,9,-75);hl(0,-79,0,-71);
    // Pitch ladder is an inertial direction, clipped by the physical combiner.
    const heading=a.heading*D;
    for(let pitch=-85;pitch<=85;pitch+=5){
      const dir=(az)=>{const e=pitch*D,h=heading+az*D;return [Math.sin(h)*Math.cos(e),Math.sin(e),-Math.cos(h)*Math.cos(e)];};
      const mid=worldDir(dir(0));if(!mid)continue;
      ctx.setLineDash(pitch<0?[4*s,3*s]:[]);
      for(const side of [-1,1]){const q=worldDir(dir(side*(pitch===0?8:2))),r=worldDir(dir(side*(pitch===0?2:5)));if(q&&r)line(ctx,[q.x,q.y],[r.x,r.y],green,1);}
      ctx.setLineDash([]);
      if(pitch&&Math.abs(pitch-a.pitch)<18){const q=worldDir(dir(6));if(q)text(ctx,String(Math.abs(pitch)),q.x,q.y,8*s,green);}
    }
    const fpm=worldDir(norm(a.vel));
    if(fpm){ctx.strokeStyle=green;ctx.lineWidth=1.2;ctx.beginPath();ctx.arc(fpm.x,fpm.y,5*s,0,TAU);ctx.stroke();line(ctx,[fpm.x-12*s,fpm.y],[fpm.x-5*s,fpm.y]);line(ctx,[fpm.x+5*s,fpm.y],[fpm.x+12*s,fpm.y]);line(ctx,[fpm.x,fpm.y-5*s],[fpm.x,fpm.y-10*s]);}
    const target=worldDir(norm(sub(gate.pos,cam.eye)));
    if(target){ctx.strokeStyle=green;ctx.beginPath();ctx.moveTo(target.x,target.y-6*s);ctx.lineTo(target.x+6*s,target.y);ctx.lineTo(target.x,target.y+6*s);ctx.lineTo(target.x-6*s,target.y);ctx.closePath();ctx.stroke();}
    const warning=a.fuel<=0?"ENGINE":a.agl<150&&a.vs<-10?"PULL UP":a.alphaDeg>25?"AOA":a.gSmooth>9.15?"G LIMIT":a.mach>2.05?"OVERSPEED":a.fuel<450?"BINGO":null;
    if(warning)ht(warning,0,27,11,"center");
    ctx.restore();
  }
  function render(context,width,height,game) {
    ctx=context;W=width;H=height;textureClock=game.time;const a=game.aircraft,m=game.mission,cam=game.camera;
    const inv=[-a.q[0],-a.q[1],-a.q[2],a.q[3]];
    eye=C.qRotate(inv,sub(cam.eye,a.pos));right=C.qRotate(inv,cam.right);up=C.qRotate(inv,cam.up);forward=C.qRotate(inv,cam.forward);
    focal=H*.5/Math.tan(cam.fov/2);queue=[];
    // Tub, coaming and forward instrument panel; no opaque bubble in the sight line.
    box([0,.03,-3.10],[1.28,.14,2.68],[30,37,38]);
    modelOffset=.85;
    polygon([[-.62,.25,-4.17],[.62,.25,-4.17],[.61,.91,-4.17],[.43,1.13,-4.17],[-.43,1.13,-4.17],[-.61,.91,-4.17]],"#343e40");
    box([-.49,.99,-4.15],[.23,.30,.12],[48,56,58]);
    box([.49,.99,-4.15],[.23,.30,.12],[48,56,58]);
    for(const side of [-1,1])box([side*.42,1.14,-4.35],[.36,.06,.36],[25,31,31]);
    box([0,1.15,-4.35],[.43,.05,.23],[25,31,31]);
    modelOffset=0;
    for(const side of [-1,1]){
      capsule([side*.65,.74,-4.43],[side*.65,.74,-1.63],.034,.034,[49,56,55]);
      capsule([side*.61,.75,-4.43],[side*.43,1.38,-4.89],.022,.018,[49,56,55]);
    }
    // Rear canopy bow is behind the pilot, leaving the bubble's forward view clear.
    for(let i=0;i<18;i++){
      const u=i*Math.PI/18,v=(i+1)*Math.PI/18;
      capsule([.64*Math.cos(u),.76+.98*Math.sin(u),-1.60],[.64*Math.cos(v),.76+.98*Math.sin(v),-1.60],.027,.027,[55,62,60],6);
    }
    const pages=game.mfdPages||["FCR","HSD"];
    modelOffset=.85;
    box([-.31,.755,-4.135],[.31,.31,.045],[25,30,31]);box([.31,.755,-4.135],[.31,.31,.045],[25,30,31]);
    panel(-.31,.755,-4.105,.30,.30,mfdTexture(pages[0],a,m,game.time));
    panel(.31,.755,-4.105,.30,.30,mfdTexture(pages[1],a,m,game.time));
    instruments(a,m,game.time);
    modelOffset=0;
    consoles(a,game.hands);
    // HUD combiner, metal edge and projector. Glass remains transparent.
    modelOffset=.85;
    box([0,1.18,-4.12],[.25,.075,.20],[27,32,31]);
    const glass=[[-.165,1.76,-4.20],[.165,1.76,-4.20],[.19,1.22,-4.10],[-.19,1.22,-4.10]];
    polygon(glass,"rgba(111,174,141,.035)");
    for(let i=0;i<4;i++)capsule(glass[i],glass[(i+1)%4],.004,.004,[116,139,125],5);
    for(const side of [-1,1])capsule([side*.185,1.20,-4.10],[side*.165,1.62,-4.17],.013,.011,[30,37,36],6);
    modelOffset=0;
    rasterize();hud(a,m,cam);
    text(ctx,"ALT + MOUSE  LOOK    C  CENTER    F1 / F2  MFD",20,H-20,10,"#afc8b2");
  }
  window.__VIPER_COCKPIT = {render,EYE,diagnostics:()=>({glError:raster?.gl.getError()??null,contextLost:raster?.gl.isContextLost()??true})};
})();
