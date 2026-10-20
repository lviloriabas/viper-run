/* Smooth, dimensioned F-16 exterior. Body axes: right, up, aft. Metres. */
(() => {
  "use strict";
  const TAU=Math.PI*2;
  const add=(a,b)=>a.map((n,i)=>n+b[i]),sub=(a,b)=>a.map((n,i)=>n-b[i]),mul=(a,s)=>a.map(n=>n*s);
  const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
  const norm=a=>mul(a,1/(Math.hypot(...a)||1)),mix=(a,b,t)=>add(mul(a,1-t),mul(b,t));
  const clamp=(n,a,b)=>Math.max(a,Math.min(b,n));
  function buildGeometry(){
    const exterior=[],canopy=[],flame=[],crew=[];
    const gray=[.45,.49,.51],light=[.55,.59,.60],dark=[.23,.27,.29],seam=[.35,.39,.41];
    const push=(out,p,n,c)=>out.push(...p,...n,...c);
    const tri=(out,a,b,c,color,normal=null)=>{const n=normal||norm(cross(sub(b,a),sub(c,a)));for(const p of[a,b,c])push(out,p,n,color);};
    const quad=(out,a,b,c,d,color,n)=>{tri(out,a,b,c,color,n);tri(out,a,c,d,color,n);};
    const cylinder=(a,b,r,color,out=exterior,sides=12)=>{
      const axis=norm(sub(b,a)),u=norm(cross(axis,Math.abs(axis[1])>.9?[1,0,0]:[0,1,0])),v=cross(axis,u);
      const p=(c,i)=>add(c,add(mul(u,r*Math.cos(i*TAU/sides)),mul(v,r*Math.sin(i*TAU/sides))));
      for(let i=0;i<sides;i++)quad(out,p(a,i),p(a,i+1),p(b,i+1),p(b,i),color);
    };
    // Catmull-Rom profiles give a continuous fuselage instead of visible rings.
    const body=[[-7.55,.008,.008,.14],[-7.25,.10,.11,.14],[-6.65,.26,.27,.13],[-5.9,.45,.40,.10],[-5.15,.56,.48,.06],[-4.2,.63,.53,.04],[-3.1,.71,.59,.01],[-1.5,.83,.66,-.015],[.6,.85,.69,-.025],[2.5,.79,.67,-.025],[4.4,.69,.59,0],[5.75,.57,.50,.015],[6.6,.49,.43,.015]];
    const profile=(z)=>{
      let k=0;while(k<body.length-2&&z>body[k+1][0])k++;
      const t=clamp((z-body[k][0])/(body[k+1][0]-body[k][0]),0,1);
      const b=body[k],c=body[k+1],a=body[Math.max(0,k-1)],d=body[Math.min(body.length-1,k+2)];
      return [z,...[1,2,3].map(i=>.5*((2*b[i])+(-a[i]+c[i])*t+(2*a[i]-5*b[i]+4*c[i]-d[i])*t*t+(-a[i]+3*b[i]-3*c[i]+d[i])*t*t*t))];
    };
    const bodyPoint=(z,angle,extra=0)=>{const p=profile(z);return [Math.cos(angle)*(p[1]+extra),p[3]+Math.sin(angle)*(p[2]+extra),z];};
    const bodyNormal=(z,angle)=>{
      const dz=sub(bodyPoint(Math.min(6.6,z+.005),angle),bodyPoint(Math.max(-7.55,z-.005),angle));
      return norm(cross(sub(bodyPoint(z,angle+.003),bodyPoint(z,angle-.003)),dz));
    };
    for(let i=0;i<104;i++)for(let j=0;j<48;j++){
      const z=-7.55+i*14.15/104,z2=-7.55+(i+1)*14.15/104,a=j*TAU/48,b=(j+1)*TAU/48;
      const points=[[z,a],[z,b],[z2,b],[z2,a]],color=z<-5.7?[.34,.38,.40]:z<-.8?light:gray;
      for(const k of[0,1,2,0,2,3]){const [v,t]=points[k];push(exterior,bodyPoint(v,t),bodyNormal(v,t),color);}
    }
    // Maintenance rings and dorsal service panels follow the curved skin.
    for(const z of[-5.72,-4.7,-2.1,.9,3.5,5.75])for(let i=0;i<48;i++)cylinder(bodyPoint(z,i*TAU/48,.006),bodyPoint(z,(i+1)*TAU/48,.006),.005,seam);
    for(const angle of[.35,Math.PI-.35,Math.PI*1.45,Math.PI*1.55])for(let z=-4.8;z<5.8;z+=.2)cylinder(bodyPoint(z,angle,.006),bodyPoint(z+.2,angle,.006),.004,seam,exterior,5);
    // A broad blended forebody / leading-edge root extension is a defining F-16 feature.
    for(const sign of[-1,1]){
      const p=[[.52,.39,-4.85],[.83,.35,-3.45],[1.38,.20,-1.75],[1.45,.10,.25],[.78,.23,1.6]].map(v=>[v[0]*sign,v[1],v[2]]);
      for(let i=1;i<p.length-1;i++)tri(exterior,p[0],p[i],p[i+1],light,[0,1,0]);
      for(let i=0;i<p.length-1;i++)cylinder(p[i],p[i+1],.014,gray);
    }
    // D-shaped intake, rounded lip and recessed dark duct. No rectangular box.
    const inlet=(z,scale,i)=>{
      const a=i*TAU/48,c=Math.cos(a),s=Math.sin(a);
      return [.62*scale*c,-.61+scale*(s>0?.24*s:.61*s),z];
    };
    for(let j=0;j<48;j++){
      const a=inlet(-3.55,1,j),b=inlet(-3.55,1,j+1),c=inlet(-3.34,.94,j+1),d=inlet(-3.34,.94,j);
      quad(exterior,a,b,c,d,light);
      quad(exterior,d,c,inlet(-2.55,.74,j+1),inlet(-2.55,.74,j),[.07,.095,.10]);
      tri(exterior,[0,-.61,-2.52],inlet(-2.55,.74,j),inlet(-2.55,.74,j+1),[.012,.019,.022]);
      cylinder(a,b,.018,gray);
      const aft=[a[0]*.92,a[1]*.68,-.1],aft2=[b[0]*.92,b[1]*.68,-.1];quad(exterior,a,aft,aft2,b,gray);
    }
    // Upper canopy, smooth tinted glass, a thin rubber sill and an internal pilot.
    const canopyPoint=(u,v)=>{
      const z=-5.12+u*3.02,width=.48*Math.pow(Math.sin(Math.PI*u),.65),height=.94*Math.pow(Math.sin(Math.PI*u),.85);
      return [width*Math.cos(v*Math.PI),.60+height*Math.sin(v*Math.PI),z];
    };
    for(let i=0;i<48;i++)for(let j=0;j<32;j++){
      const uv=[[i/48,j/32],[(i+1)/48,j/32],[(i+1)/48,(j+1)/32],[i/48,(j+1)/32]];
      for(const k of[0,1,2,0,2,3]){
        const [u,v]=uv[k],p=canopyPoint(u,v);
        let n=norm(cross(sub(canopyPoint(Math.min(.99999,u+.0005),v),canopyPoint(Math.max(.00001,u-.0005),v)),sub(canopyPoint(u,Math.min(1,v+.0005)),canopyPoint(u,Math.max(0,v-.0005)))));
        if(Math.hypot(...n)<.1)n=[0,1,0];if(n[1]<0)n=mul(n,-1);push(canopy,p,n,[.17,.30,.32]);
      }
    }
    for(const v of[0,1])for(let i=0;i<48;i++)cylinder(canopyPoint(i/48,v),canopyPoint((i+1)/48,v),.018,[.12,.16,.17]);
    const ball=(center,size,color,lat=12,lon=24,out=exterior)=>{
      const p=(i,j)=>[center[0]+size[0]*Math.sin(i*Math.PI/lat)*Math.cos(j*TAU/lon),center[1]+size[1]*Math.cos(i*Math.PI/lat),center[2]+size[2]*Math.sin(i*Math.PI/lat)*Math.sin(j*TAU/lon)];
      for(let i=0;i<lat;i++)for(let j=0;j<lon;j++){
        const pts=[p(i,j),p(i+1,j),p(i+1,j+1),p(i,j+1)];
        for(const k of[0,1,2,0,2,3])push(out,pts[k],norm(pts[k].map((n,l)=>(n-center[l])/(size[l]*size[l]))),color);
      }
    };
    ball([0,.97,-3.38],[.24,.30,.18],[.20,.25,.16],12,24,crew);
    ball([0,1.31,-3.30],[.135,.14,.14],[.57,.60,.56],12,24,crew);
    ball([0,1.32,-3.428],[.113,.061,.024],[.035,.065,.072],8,20,crew);
    // Blended spine joins the bubble to the base of the swept fin.
    for(let z=-2.1;z<4.4;z+=.10){
      const top=profile(z)[2]+profile(z)[3]+.12;
      quad(exterior,[-.22,top-.06,z],[.22,top-.06,z],[.20,top-.04,z+.1],[-.20,top-.04,z+.1],gray,[0,1,0]);
    }
    // Thin smooth airfoil surfaces with 40-degree leading-edge sweep.
    const wingPoint=(sign,u,v,upper)=>{
      const x=(.72+3.99*u)*sign,le=-1.90+3.35*u,te=3.14-.16*u,chord=te-le;
      const naca=5*(.052-.025*u)*(.2969*Math.sqrt(Math.max(v,0))-.126*v-.3516*v*v+.2843*v*v*v-.1036*v*v*v*v);
      return [x,.075-.025*u+(upper?1:-1)*chord*naca,le+chord*v];
    };
    for(const sign of[-1,1])for(const upper of[true,false])for(let i=0;i<24;i++)for(let j=0;j<24;j++){
      const uv=[[i/24,j/24],[(i+1)/24,j/24],[(i+1)/24,(j+1)/24],[i/24,(j+1)/24]];
      for(const k of[0,1,2,0,2,3]){
        const [u,v]=uv[k],p=wingPoint(sign,u,v,upper);
        let n=norm(cross(sub(wingPoint(sign,Math.min(1,u+.001),v,upper),wingPoint(sign,Math.max(0,u-.001),v,upper)),sub(wingPoint(sign,u,Math.min(1,v+.001),upper),wingPoint(sign,u,Math.max(0,v-.001),upper))));
        if((upper&&n[1]<0)||(!upper&&n[1]>0))n=mul(n,-1);
        push(exterior,p,n,upper?gray:[.36,.40,.42]);
      }
    }
    for(const sign of[-1,1]){
      for(const chord of[.15,.74])for(let i=0;i<24;i++){
        const p=wingPoint(sign,i/24,chord,true),q=wingPoint(sign,(i+1)/24,chord,true);p[1]+=.008;q[1]+=.008;cylinder(p,q,.006,seam,exterior,5);
      }
      for(const u of[.12,.5,.8])for(let j=0;j<20;j++){
        const p=wingPoint(sign,u,j/20,true),q=wingPoint(sign,u,(j+1)/20,true);p[1]+=.007;q[1]+=.007;cylinder(p,q,.004,seam,exterior,5);
      }
      // Wing walk strips near the root.
      for(const u of[.035,.14])for(let v=.2;v<.74;v+=.03){const p=wingPoint(sign,u,v,true),q=wingPoint(sign,u,v+.03,true);p[1]+=.01;q[1]+=.01;cylinder(p,q,.010,[.18,.22,.23],exterior,5);}
    }
    const thinFin=(points,thickness,color,axis=0)=>{
      const a=points.map(p=>{const q=[...p];q[axis]-=thickness/2;return q;}),b=points.map(p=>{const q=[...p];q[axis]+=thickness/2;return q;});
      for(let i=1;i<a.length-1;i++){tri(exterior,a[0],a[i],a[i+1],color);tri(exterior,b[0],b[i+1],b[i],color);}
      for(let i=0;i<a.length;i++){let j=(i+1)%a.length;quad(exterior,a[i],b[i],b[j],a[j],light);}
    };
    for(const sign of[-1,1]){
      thinFin([[.47,.04,4.55],[2.83,-.10,6.08],[2.75,-.10,7.22],[.47,.04,6.85]].map(p=>[p[0]*sign,p[1],p[2]]),.055,gray,1);
      // Ventral strakes, swept and canted.
      thinFin([[.73,-.42,4.8],[.83,-.48,6.7],[1.01,-1.02,6.3],[.91,-1.10,5.9]].map(p=>[p[0]*sign,p[1],p[2]]),.032,dark);
      cylinder([sign*4.76,.04,1.35],[sign*4.76,.04,3.15],.038,dark);
      ball([sign*4.72,.12,2.76],[.045,.034,.08],sign===1?[.04,.92,.25]:[.95,.06,.035],6,12);
    }
    thinFin([[0,.62,3.72],[0,3.32,5.20],[0,3.20,6.26],[0,.64,6.92]],.095,gray);
    // Rudder hinge, fin cap and a restrained tail marking.
    cylinder([-.052,.85,6.50],[-.052,3.19,6.02],.009,seam);
    for(const sign of[-1,1]){
      quad(exterior,[sign*.051,2.93,5.02],[sign*.051,3.27,5.20],[sign*.051,3.15,6.25],[sign*.051,2.81,6.35],[.20,.28,.34]);
      cylinder([sign*.057,1.70,5.40],[sign*.057,2.35,5.58],.024,[.72,.75,.72]);
      cylinder([sign*.057,2.35,5.58],[sign*.057,1.70,5.95],.024,[.72,.75,.72]);
    }
    // Circular, petalled nozzle with a dark interior and hot core.
    for(let j=0;j<32;j++){
      const a=j*TAU/32,b=(j+1)*TAU/32,p=(z,r,t)=>[Math.cos(t)*r,Math.sin(t)*r*.92+.015,z];
      const metal=j%2?[.34,.35,.34]:[.25,.27,.27];
      quad(exterior,p(6.50,.49,a),p(6.50,.49,b),p(7.48,.43,b),p(7.48,.43,a),metal);
      quad(exterior,p(7.48,.43,a),p(7.48,.43,b),p(7.21,.34,b),p(7.21,.34,a),[.08,.085,.08]);
      tri(exterior,[0,.015,7.17],p(7.21,.34,a),p(7.21,.34,b),[.035,.025,.018]);
      const f=(z,r,t)=>[Math.cos(t)*r,Math.sin(t)*r*.92+.015,z];
      quad(flame,f(7.49,.35,a),f(7.49,.35,b),f(10.7,.02,b),f(10.7,.02,a),[1,.38,.08]);
    }
    // Pitot, small dorsal aerials and underside pylons finish the silhouette.
    cylinder([0,.14,-7.54],[0,.14,-8.0],.009,[.58,.62,.62],exterior,8);
    for(const z of[-1.75,.6])thinFin([[0,.68,z],[0,1.02,z+.14],[0,.73,z+.45]],.018,dark);
    for(const sign of[-1,1])for(const x of[1.65,2.95])thinFin([[x*sign,-.02,.4],[x*sign,-.40,1.0],[x*sign,-.38,2.1],[x*sign,-.03,2.65]],.10,gray);
    const sx=9.96/9.596,sz=15.06/15.48;
    for(const array of[exterior,canopy,flame,crew])for(let i=0;i<array.length;i+=9){
      array[i]*=sx;array[i+2]*=sz;
      const n=norm([array[i+3]/sx,array[i+4],array[i+5]/sz]);array[i+3]=n[0];array[i+4]=n[1];array[i+5]=n[2];
    }
    return {exterior,canopy,flame,crew,stats:{overallLength:15.06,wingspan:9.96,triangleCount:exterior.length/27}};
  }
  window.__VIPER_AIRFRAME={buildGeometry};
})();
