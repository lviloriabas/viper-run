// Game-model calibration, not a claim of validated F-16 performance tables.
import fs from 'node:fs';import vm from 'node:vm';import assert from 'node:assert/strict';
const sandbox={window:{},Math,console,Float32Array,Uint16Array};
vm.runInNewContext(fs.readFileSync(new URL('../src/game.js',import.meta.url),'utf8'),sandbox);
const C=sandbox.window.__VIPER_CORE,S=C.AIRCRAFT_SPEC;
function equilibrium(altitude,mach){
 const atm=C.atmosphereAt(altitude),speed=mach*atm.sound,q=.5*atm.rho*speed*speed;
 const a=C.makeAircraft(C.buildMission());a.pos=[0,altitude,0];a.vel=[0,0,-speed];
 const clMax=S.clMax*Math.max(.82,1-.28*C.smoothstep(.85,1.25,mach));
 const alpha=a.mass*9.80665/(q*S.wingArea*clMax)*S.alphaStall;
 a.q=C.qMultiply(C.qFromForwardUp([0,0,-1]),C.qAxisAngle([1,0,0],alpha));
 a.throttle=1;a.engineSpool=1;a.abLevel=1;C.updateAircraft(a,0,{afterburner:true});
 return {mach,thrust:a.thrustN,drag:a.dragN,excess:a.thrustN*Math.cos(alpha)-a.dragN,g:a.gLoad};
}
for(const feet of [10000,20000,30000,40000]){
 let low=1.1,high=2.6;for(let i=0;i<35;i++){const mid=(low+high)/2;if(equilibrium(feet*.3048,mid).excess>0)low=mid;else high=mid;}
 const result=equilibrium(feet*.3048,(low+high)/2);
 if(feet===40000)assert.ok(Math.abs(result.mach-2.05)<.1,'40,000-ft full-AB equilibrium must remain close to the public reference');
 console.log(JSON.stringify({altitudeFt:feet,levelFullABMach:result.mach.toFixed(3),g:result.g.toFixed(2)}));
}
console.log('Published Block 50 short-duration reference: Mach 2.05 at 40,000 ft. Thermal, intake and stores limits are not simulated.');
