/* Modernized F-16 Viper, procedural game cockpit. Metres in the exterior's body frame.
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
  const BODY_OFFSET = -1.2;
  const EYE = [0,1.43,-2.05 + BODY_OFFSET];

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
    points=points.map(p=>[p[0],p[1],p[2]+modelOffset+BODY_OFFSET]);
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
  function capsule(a,b,r1,r2,color=[85,95,67],sides=24) {
    const axis=norm(sub(b,a)),r=norm(cross(axis,Math.abs(axis[1])>.9?[1,0,0]:[0,1,0])),u=cross(axis,r);
    const ring=(p,rad,i)=>add(p,add(mul(r,rad*Math.cos(i*TAU/sides)),mul(u,rad*Math.sin(i*TAU/sides))));
    for(let i=0;i<sides;i++) {
      const shade=.65+.35*Math.max(0,Math.sin((i+.5)*TAU/sides));
      polygon([ring(a,r1,i),ring(a,r1,i+1),ring(b,r2,i+1),ring(b,r2,i)],`rgb(${color.map(n=>Math.round(n*shade)).join(",")})`);
    }
    polygon(Array.from({length:sides},(_,i)=>ring(b,r2,i)),`rgb(${color.join(",")})`);
  }
  function ellipsoid(c,size,color,lean=0) {
    const point=(i,j)=>[c[0]+size[0]*Math.sin(i*Math.PI/16)*Math.cos(j*TAU/32),c[1]+size[1]*Math.cos(i*Math.PI/16),c[2]+size[2]*Math.sin(i*Math.PI/16)*Math.sin(j*TAU/32)+size[1]*Math.cos(i*Math.PI/16)*lean];
    for(let i=0;i<16;i++)for(let j=0;j<32;j++){
      const shade=.68+.32*Math.max(0,Math.cos((i+.5)*Math.PI/16));
      polygon([point(i,j),point(i,j+1),point(i+1,j+1),point(i+1,j)],`rgb(${color.map(n=>Math.round(n*shade)).join(",")})`);
    }
  }
  function roundedCase(x,y,z,w,h,r=.025,color=[45,49,51]) {
    const corners=[[x+w/2-r,y+h/2-r,0],[x-w/2+r,y+h/2-r,Math.PI/2],[x-w/2+r,y-h/2+r,Math.PI],[x+w/2-r,y-h/2+r,Math.PI*1.5]],p=[];
    for(const[cx,cy,a]of corners)for(let i=0;i<=7;i++){const t=a+i*Math.PI/14;p.push([cx+Math.cos(t)*r,cy+Math.sin(t)*r,z]);}
    polygon(p,`rgb(${color.join(',')})`);
    for(let i=0;i<p.length;i++){
      const a=p[i],b=p[(i+1)%p.length],back=q=>[q[0],q[1],q[2]-.028];
      polygon([a,b,back(b),back(a)],i<p.length/2?"#51575a":"#191e20");
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
  function hardware(t,w,h,label) {
    const g=t.createLinearGradient(0,0,w,h);g.addColorStop(0,"#33393d");g.addColorStop(.4,"#171d21");g.addColorStop(1,"#2b3236");t.fillStyle=g;t.fillRect(0,0,w,h);
    t.strokeStyle="#6a7276";t.lineWidth=2;t.strokeRect(2,2,w-4,h-4);t.strokeStyle="#070c10";t.strokeRect(7,7,w-14,h-14);
    for(let i=0;i<230;i++){const x=(i*73.31)%w,y=(i*47.73)%h;t.fillStyle=i%3?"#d9e1d508":"#00000020";t.fillRect(x,y,1+(i%3),1);}
    for(const[x,y]of[[12,12],[w-12,12],[12,h-12],[w-12,h-12]]){t.fillStyle="#101517";t.beginPath();t.arc(x,y,5,0,TAU);t.fill();line(t,[x-3,y],[x+3,y],"#77817b",1);}
    if(label)text(t,label,w/2,h-15,11,"#b7c0ba","center");
  }
  function mfdTexture(name,a,m,time) {
    return surface("display-"+name,384,512,(t,w,h)=>{
      hardware(t,w,h,"");t.fillStyle="#01070a";t.fillRect(35,35,314,442);
      for(let i=0;i<5;i++){
        for(const y of[8,486]){t.fillStyle="#4b5253";t.fillRect(50+i*65,y,22,17);line(t,[55+i*65,y+2],[66+i*65,y+2],"#8f9791");}
        for(const x of[8,359]){t.fillStyle="#434b4c";t.fillRect(x,75+i*82,17,22);line(t,[x+2,79+i*82],[x+2,91+i*82],"#8f9791");}
      }
      text(t,name,192,53,16,green,"center");text(t,"OVRD",58,54,11,green);text(t,"CNTL",328,54,11,green,"right");
      text(t,"SWAP",65,460,11,green);text(t,"FCR",148,460,11,green);text(t,"HSD",218,460,11,green);text(t,"DTE",311,460,11,green,"right");
      if(name==="HSD"){
        const center=[192,284],range=9260,heading=a.heading*D;
        for(const r of[52,103,145]){t.strokeStyle="#183d38";t.beginPath();t.arc(...center,r,0,TAU);t.stroke();}
        line(t,[192,120],[192,430],"#163631");line(t,[45,284],[339,284],"#163631");
        const nav=p=>{const d=sub(p,a.pos);return[192+(d[0]*Math.cos(heading)+d[2]*Math.sin(heading))*145/range,284-(d[0]*Math.sin(heading)-d[2]*Math.cos(heading))*145/range];};
        t.save();t.beginPath();t.rect(39,94,306,343);t.clip();
        for(let i=0;i<m.gates.length;i++){
          const p=nav(m.gates[i].pos),q=m.gates[i+1];if(q)line(t,p,nav(q.pos),"#498ca9",2);
          t.strokeStyle=i===m.gateIndex?"#f07ac9":"#7ac5b0";t.lineWidth=2;t.beginPath();t.arc(...p,i===m.gateIndex?7:4,0,TAU);t.stroke();text(t,String(i+1),p[0]+10,p[1],13,"#b6c8df");
        }t.restore();
        line(t,[178,294],[192,274],green,2);line(t,[192,274],[206,294],green,2);line(t,[178,294],[206,294],green,2);
        text(t,"5 NM",323,82,13,green,"right");text(t,pad(a.heading)+" HDG",54,417,13,green);text(t,"STPT "+pad(Math.min(m.gateIndex+1,m.gates.length),2),329,417,13,green,"right");
        text(t,"NAV   GPS   NORM",192,100,13,"#7cbcc2","center");
        text(t,"VS "+Math.round(a.vs*196.85),54,438,11,"#a2b8b6");text(t,"AOA "+a.alphaDeg.toFixed(1),329,438,11,"#a2b8b6","right");
      }else if(name==="FCR"){
        text(t,"CRM  RWS  NORM",192,83,14,green,"center");text(t,"40",52,137,14,green);
        for(let i=1;i<5;i++)line(t,[72,106+i*62],[322,106+i*62],"#153731");
        line(t,[72,106],[72,411],green);line(t,[322,106],[322,411],green);
        const sx=78+(time*.24%1)*238;line(t,[sx,110],[sx,405],"#397957");
        text(t,"NO TRACK",192,255,16,"#80b791","center");text(t,pad(a.airspeedKt)+" C",82,420,13,green);text(t,pad(ft(a.pos[1])/100)+" ALT",319,420,13,green,"right");
      }else{
        const rows=[["RPM",(a.rpm||0).toFixed(1)+" %"],["FTIT",pad(a.ftit||0)+" C"],["NOZZLE",pad(a.nozzle||0,2)+" %"],["FUEL",pad(a.fuel*2.20462,4)+" LB"],["FLOW",pad((a.fuelFlow||0)*7936.64,5)+" PPH"],["G / AOA",a.gSmooth.toFixed(1)+" / "+a.alphaDeg.toFixed(1)]];
        rows.forEach((r,i)=>{text(t,r[0],55,119+i*47,17,"#a2b8b6");text(t,r[1],329,119+i*47,17,green,"right");line(t,[54,140+i*47],[329,140+i*47],"#1c3431");});
        text(t,a.ab?"AFTERBURNER":"FLCS NORM",192,418,15,a.ab?"#e6bc7a":green,"center");
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
    panel(.493,.73,-4.13,.112,.41,engine);
    const ded=surface("ded",510,180,(t)=>{
      t.fillStyle="#020804";t.fillRect(0,0,510,180);
      const gate=m.gates[Math.min(m.gateIndex,m.gates.length-1)];const distance=Math.hypot(...sub(gate.pos,a.pos))/1852;
      const lines=[`UHF  305.00    STPT ${pad(Math.min(m.gateIndex+1,m.gates.length),2)}`,"VHF  127.50    * *",`M ${a.mach.toFixed(2)}       ${pad(a.heading)} DEG`,`DIST ${distance.toFixed(1)} NM   GPS OK`,`TIME ${pad(Math.floor(m.elapsed/60),2)}:${pad(m.elapsed%60,2)}    FUEL ${pad(a.fuel*2.20462,4)}`];
      lines.forEach((s,i)=>text(t,s,10,18+i*35,24,green));
    });panel(0,1.185,-3.99,.228,.060,ded);
    const icp=surface("icp",320,330,(t)=>{
      t.fillStyle="#30383b";t.fillRect(0,0,320,330);
      const labels=[["COM1","COM2","IFF","LIST"],["1 T-ILS","2 ALOW","3","RCL"],["4 STPT","5 CRUS","6 TIME","ENTR"],["7 MARK","8 FIX","9 A-CAL","DCS"],["A-A","0 M-SEL","A-G","DRIFT"]];
      labels.forEach((row,y)=>row.forEach((s,x)=>{t.fillStyle="#12191b";t.fillRect(10+x*78,10+y*60,68,48);t.strokeStyle="#707b74";t.strokeRect(10+x*78,10+y*60,68,48);text(t,s,44+x*78,34+y*60,11,white,"center");}));
      text(t,"BRT     SYM    RET DEPR",160,319,12,white,"center");
    });panel(0,1.055,-3.99,.228,.19,icp);
    const lower=surface("standby",480,220,(t)=>{
      hardware(t,480,220,"STANDBY");
      dial(t,116,105,77,ft(a.pos[1])%10000,0,10000,"ALT","FT");
      t.save();t.beginPath();t.roundRect(250,30,190,152,8);t.clip();t.translate(345,106);t.rotate(-a.bank*D);t.translate(0,a.pitch*2);
      t.fillStyle="#466d83";t.fillRect(-220,-400,440,400);t.fillStyle="#866044";t.fillRect(-220,0,440,400);line(t,[-220,0],[220,0],white,2);
      for(let n=-40;n<=40;n+=10)if(n)line(t,[-22,-n*2],[22,-n*2],white);
      t.restore();line(t,[291,107],[331,107],"#e8b95c",3);line(t,[359,107],[400,107],"#e8b95c",3);text(t,pad(ft(a.pos[1]),5),116,163,16,green,"center");
    });panel(0,.507,-4.105,.255,.117,lower);
    const warn=surface("warn",320,180,(t)=>{
      t.fillStyle="#202729";t.fillRect(0,0,320,180);
      const labels=[["FLCS",a.alphaDeg>25],["FUEL LOW",a.fuel<450],["ENGINE",a.fuel<=0],["OIL",(a.oilPsi||60)<20],["ALT LOW",a.agl<100],["SPEED BRK",a.brake>.1]];
      labels.forEach(([s,on],i)=>{const x=(i%2)*160,y=Math.floor(i/2)*60;t.fillStyle=on?"#d49638":"#0c1414";t.fillRect(x+3,y+3,154,54);text(t,s,x+80,y+30,16,on?"#1c1405":"#6b7569","center");});
    });panel(.486,.451,-4.075,.14,.10,warn);
    const rwr=surface("rwr",320,320,(t)=>{
      t.fillStyle="#232c2e";t.fillRect(0,0,320,320);
      t.fillStyle="#020805";t.beginPath();t.arc(160,148,126,0,TAU);t.fill();
      for(const r of [42,85,120]){t.strokeStyle="#315340";t.lineWidth=2;t.beginPath();t.arc(160,148,r,0,TAU);t.stroke();}
      line(t,[42,148],[278,148],"#466d50");line(t,[160,28],[160,268],"#466d50");
      text(t,"RWR",160,300,17,white,"center");text(t,"N",160,43,18,green,"center");
      text(t,"NO EMITTER",160,150,16,green,"center");
    });panel(-.49,.98,-4.075,.10,.10,rwr);
    const aux=surface("aux",320,250,(t)=>{
      t.fillStyle="#333c3d";t.fillRect(0,0,320,250);
      dial(t,160,98,76,a.airspeedKt,0,800,"KCAS","KT");
      text(t,`FUEL ${pad(a.fuel*2.20462,4)} LB`,160,205,22,green,"center");
      text(t,`G ${a.gSmooth.toFixed(1)}   ${pad(a.heading)} HDG`,160,236,17,white,"center");
    });panel(-.49,.746,-4.13,.105,.225,aux);
    // AOA indexer and master caution, placed beside the HUD rather than on it.
    const aoa=surface("indexer",80,240,t=>{
      t.fillStyle="#121b1c";t.fillRect(0,0,80,240);
      text(t,"V",40,40,48,a.alphaDeg>14?"#ff5f46":"#60382c","center");
      text(t,"O",40,120,40,a.alphaDeg>=11&&a.alphaDeg<=14?green:"#274534","center");
      text(t,"^",40,205,48,a.alphaDeg<11?"#e9ba53":"#5c4e2e","center");
    });panel(-.20,1.34,-4.12,.037,.115,aoa);
  }
  function consoles(a,hands) {
    for(const side of[-1,1]){
      box([side*.49,.23,-2.94],[.245,.27,1.75],[29,34,37]);
      capsule([side*.625,.42,-3.82],[side*.625,.42,-2.0],.016,.016,[69,74,73]);
      const names=side===-1?["FUEL","EPU / JFS","UHF / VHF","AUDIO","EXT LIGHTS"]:["POWER","FLCS","SENSOR PWR","AVIONICS","ECS"];
      for(let j=0;j<5;j++){
        const z=-3.72+j*.29;
        const image=surface("console"+side+"-"+j,250,280,(t,w,h)=>{
          hardware(t,w,h,names[j]);text(t,names[j],w/2,27,18,"#c7cfc5","center");
          for(let k=0;k<3;k++){
            const x=52+k*72;t.strokeStyle="#77837c";t.strokeRect(x-22,67,44,75);text(t,["ON","NORM","AUTO"][k],x,57,12,white,"center");text(t,"OFF",x,157,11,"#9daaa0","center");
          }
          if(j===2){t.fillStyle="#071410";t.fillRect(26,192,198,41);text(t,"305.000",125,213,28,green,"center");}
          else{line(t,[28,202],[220,202],"#6f7b72");text(t,"TEST    RESET",125,224,13,"#afb9b0","center");}
        });
        polygon([[side*.49-.115,.382,z-.125],[side*.49+.115,.382,z-.125],[side*.49+.115,.382,z+.125],[side*.49-.115,.382,z+.125]],"#1c2429",image);
        for(let k=0;k<2;k++){
          const x=side*.49+(k-.5)*.065,za=z-.03+k*.045;
          capsule([x,.389,za],[x,.42,za-.009],.008,.006,[168,175,165],12);
          ellipsoid([x,.425,za-.012],[.009,.009,.009],[133,145,137]);
        }
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
    ellipsoid(add(stick,[0,.04,0]),[.043,.065,.043],[39,45,46]);
    box(add(stick,[-.038,.063,-.009]),[.024,.018,.025],[155,162,150]);
    box(add(stick,[.012,.096,0]),[.025,.012,.025],[187,64,41]);
    // Reclined ACES II seat. The pilot's head is deliberately omitted at the eye.
    box([0,.13,-2.53],[.59,.13,.63],[36,45,40]);
    capsule([0,.20,-2.37],[0,.91,-2.03],.27,.245,[35,43,40]);
    for(const side of [-1,1])capsule([side*.29,.17,-2.43],[side*.29,.84,-2.12],.045,.045,[70,77,69]);
    const hip=[0,.27,-2.59],suit=[101,111,76],glove=[92,103,78];
    ellipsoid([0,.55,-2.44],[.235,.35,.17],suit,.52);
    for(const side of[-1,1])capsule([side*.17,.85,-2.31],[side*.13,.34,-2.71],.026,.026,[148,149,123],12);
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
      ["CHECK FUEL","10 STEERPOINTS","H AIRBRAKE","K CONFIG","ALT + MOUSE","C CENTER VIEW"].forEach((s,i)=>text(t,s,10,58+i*25,12,"#283c30"));
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
    const glass=[[-.125,1.60,-3.35],[.125,1.60,-3.35],[.140,1.28,-3.25],[-.140,1.28,-3.25]].map(p=>[p[0],p[1],p[2]+BODY_OFFSET]);
    const clipped=clipNear(glass);if(clipped.length<3)return;
    const p=clipped.map(project);
    ctx.save();ctx.beginPath();ctx.moveTo(p[0].x,p[0].y);p.slice(1).forEach(v=>ctx.lineTo(v.x,v.y));ctx.closePath();ctx.clip();
    const baseFocal=H*.5/Math.tan(cam.fov/2),s=clamp(baseFocal/750,.5,1.4);
    const ax=C.axisSet(a.q),worldDir=d=>{
      const z=dot(d,cam.forward);if(z<=.01)return null;
      return {x:W*.5+dot(d,cam.right)*focal/z,y:H*.5-dot(d,cam.up)*focal/z};
    };
    const anchor=worldDir(ax.forward);if(!anchor){ctx.restore();return;}
    const at=(x,y)=>worldDir(norm(add(ax.forward,add(mul(ax.right,x/baseFocal),mul(ax.up,-y/baseFocal)))));
    const ht=(str,x,y,size=11,align="left")=>{const p=at(x*s,y*s);if(p)text(ctx,str,p.x,p.y,size*s,green,align);};
    const hl=(x,y,x2,y2)=>{const p=at(x*s,y*s),q=at(x2*s,y2*s);if(p&&q)line(ctx,[p.x,p.y],[q.x,q.y],green,1.05);};
    const half=baseFocal*.087/s;
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
    box([0,.03,-3.10],[1.21,.10,2.68],[24,28,30]);
    modelOffset=.85;
    const outline=[[-.60,.73,-4.17],[-.155,.73,-4.17],[-.155,.40,-4.17],[.155,.40,-4.17],[.155,.73,-4.17],[.60,.73,-4.17],[.603,.88,-4.17],[.557,1.025,-4.17],[.369,1.253,-4.17],[-.369,1.253,-4.17],[-.557,1.025,-4.17],[-.603,.88,-4.17]];
    polygon([[-.60,.73,-4.17],[-.155,.73,-4.17],[-.155,1.253,-4.17],[-.369,1.253,-4.17],[-.557,1.025,-4.17],[-.603,.88,-4.17]],"#282e33");
    polygon([[.60,.73,-4.17],[.155,.73,-4.17],[.155,1.253,-4.17],[.369,1.253,-4.17],[.557,1.025,-4.17],[.603,.88,-4.17]],"#282e33");
    polygon([[-.155,.4,-4.17],[.155,.4,-4.17],[.155,1.253,-4.17],[-.155,1.253,-4.17]],"#282e33");
    for(let i=0;i<outline.length;i++){
      const p=outline[i],q=outline[(i+1)%outline.length];
      polygon([p,q,[q[0]*1.025,q[1],q[2]-.055],[p[0]*1.025,p[1],p[2]-.055]],"#131a1e");
      capsule(p,q,.012,.012,[83,89,91],16);
    }
    // The coaming follows the sloped panel shoulders, rather than a shelf.
    for(const side of[-1,1]){
      polygon([[side*.60,.56,-4.17],[side*.555,1.025,-4.17],[side*.37,1.25,-4.17],[side*.37,1.24,-4.48],[side*.56,.99,-4.48],[side*.61,.52,-4.48]],"#151b20");
      capsule([side*.37,1.25,-4.17],[side*.13,1.25,-4.17],.013,.013,[43,48,51],20);
    }
    roundedCase(0,1.083,-4.035,.262,.283,.025,[61,65,66]);
    const pages=game.mfdPages||["FCR","ENG"];
    for(const[side,page]of[[-1,pages[0]],[1,pages[1]]]){
      roundedCase(side*.305,1.025,-4.12,.288,.382,.022,[54,59,62]);
      panel(side*.305,1.025,-4.105,.267,.357,mfdTexture(page,a,m,game.time));
    }
    roundedCase(0,.780,-4.12,.293,.395,.026,[59,64,65]);
    panel(0,.780,-4.105,.269,.369,mfdTexture("HSD",a,m,game.time));
    roundedCase(0,.508,-4.12,.279,.135,.018,[52,58,61]);
    instruments(a,m,game.time);
    const switches=surface("left-master",190,280,(t,w,h)=>{
      hardware(t,w,h,"FLIGHT CONTROL");text(t,"MASTER ARM",95,32,13,white,"center");
      for(let i=0;i<3;i++){text(t,["SAFE","SIM","ARM"][i],128,72+i*27,12,"#b6c6b5");line(t,[48,72+i*27],[68,72+i*27],white,3);}
      text(t,"LASER ARM",95,187,13,white,"center");text(t,"ALT REL",95,231,13,"#96a5a0","center");
    });panel(-.49,.464,-4.105,.105,.155,switches);
    modelOffset=0;
    for(const side of[-1,1]){
      capsule([side*.62,.70,-4.15],[side*.62,.70,-1.63],.025,.025,[45,49,52]);
      capsule([side*.61,.72,-4.15],[side*.43,1.37,-4.86],.013,.013,[43,48,51]);
    }
    for(let i=0;i<40;i++){
      const u=i*Math.PI/40,v=(i+1)*Math.PI/40;
      capsule([.61*Math.cos(u),.72+.99*Math.sin(u),-1.60],[.61*Math.cos(v),.72+.99*Math.sin(v),-1.60],.020,.020,[50,54,57],12);
    }
    consoles(a,game.hands);
    modelOffset=.85;
    roundedCase(0,1.255,-4.10,.256,.062,.015,[35,41,44]);
    const glass=[[-.125,1.60,-4.20],[.125,1.60,-4.20],[.140,1.28,-4.10],[-.140,1.28,-4.10]];
    polygon(glass,"rgba(124,185,170,.03)");
    for(let i=0;i<4;i++)capsule(glass[i],glass[(i+1)%4],.003,.003,[139,158,150],12);
    for(const side of[-1,1])capsule([side*.139,1.28,-4.10],[side*.128,1.52,-4.175],.009,.008,[22,28,32],20);
    modelOffset=0;
    rasterize();hud(a,m,cam);
    text(ctx,"K  CONTROLS / CONFIG    ALT + MOUSE  LOOK    C  CENTER",20,H-20,10,"#bfd0bf");
  }
  window.__VIPER_COCKPIT = {render,EYE,diagnostics:()=>({glError:raster?.gl.getError()??null,contextLost:raster?.gl.isContextLost()??true})};
})();
