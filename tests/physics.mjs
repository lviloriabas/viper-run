import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const context = {window: {}, console, Math, Float32Array, Uint16Array};
vm.runInNewContext(fs.readFileSync(new URL('../src/game.js',import.meta.url),'utf8'),context);
const C=context.window.__VIPER_CORE,dt=1/120;
let checks=0;
function check(name,fn){fn();checks++;console.log(`PASS ${name}`);}
function aircraft(alt=3000,speed=220){
  const a=C.makeAircraft(C.buildMission());a.pos=[0,alt,0];a.vel=[0,0,-speed];
  a.q=C.qMultiply(C.qFromForwardUp([0,0,-1]),C.qAxisAngle([1,0,0],2.35*Math.PI/180));
  return a;
}
function run(a,seconds,input={}){for(let i=0;i<seconds/dt;i++)C.updateAircraft(a,dt,input);return a;}
check('initial instruments have valid airborne telemetry before the first frame',()=>{
  const a=C.makeAircraft(C.buildMission());assert.ok(a.speed>200&&a.airspeedKt>300);assert.ok(a.mach>.5&&a.agl>100);assert.ok(a.rpm>80);
});
check('ISA sea level and tropopause',()=>{
  const sea=C.atmosphereAt(0),high=C.atmosphereAt(11000);
  assert.ok(Math.abs(sea.rho-1.225)<.001);assert.ok(Math.abs(sea.sound-340.294)<.1);
  assert.ok(Math.abs(high.pressure-22632)<10);assert.ok(Math.abs(high.sound-295.07)<.2);
});
check('compressible calibrated airspeed, including supersonic pitot',()=>{
  for(const speed of [50,200,340,500,680])assert.ok(Math.abs(C.calibratedAirspeed(speed,C.atmosphereAt(0))-speed)<.001);
  assert.ok(C.calibratedAirspeed(300,C.atmosphereAt(10000))<200);
  const a=run(aircraft(10000,300),dt);assert.ok(a.airspeedKt<a.tasKt);assert.ok(a.mach>1);
});
check('altitude reduces dynamic pressure, lift capacity, roll authority and thrust',()=>{
  const low=run(aircraft(3000),.5,{rollRight:true,afterburner:true,throttleUp:true});
  const high=run(aircraft(10000),.5,{rollRight:true,afterburner:true,throttleUp:true});
  assert.ok(high.qbar<low.qbar*.5);assert.ok(high.gAvail<low.gAvail);assert.ok(Math.abs(high.omega[2])<Math.abs(low.omega[2]));assert.ok(high.thrustN<low.thrustN);
});
check('airspeed changes attainable G and roll response',()=>{
  const slow=run(aircraft(3000,100),.25,{rollRight:true,pitchUp:true});
  const fast=run(aircraft(3000,250),.25,{rollRight:true,pitchUp:true});
  assert.ok(slow.gAvail<fast.gAvail);assert.ok(Math.abs(slow.omega[2])<Math.abs(fast.omega[2]));
});
check('fuel exhaustion removes thrust and fuel flow',()=>{
  const a=aircraft();a.fuel=0;run(a,1,{afterburner:true,throttleUp:true});
  assert.equal(a.thrustN,0);assert.equal(a.fuelFlow,0);assert.equal(a.ab,false);assert.equal(a.fuel,0);
});
check('afterburner requires military throttle and has light-up lag',()=>{
  const a=aircraft();a.throttle=.7;run(a,dt,{afterburner:true});assert.equal(a.ab,false);
  a.throttle=1;run(a,dt,{afterburner:true});assert.equal(a.ab,true);assert.ok(a.abLevel>0&&a.abLevel<.1);
});
check('inverted bank remains near 180 degrees',()=>{
  const a=aircraft();a.q=C.qMultiply(a.q,C.qAxisAngle([0,0,1],Math.PI));run(a,dt);
  assert.ok(Math.abs(a.bank)>170);
});
check('airbrake adds drag',()=>{
  const clean=run(aircraft(3000,250),2),braked=run(aircraft(3000,250),2,{airbrake:true});
  assert.ok(braked.dragN>clean.dragN);assert.ok(braked.speed<clean.speed);
});
check('engine and fuel instruments follow spool and consumption',()=>{
  const a=aircraft();const fuel=a.fuel;run(a,3,{throttleUp:true,afterburner:true});
  assert.ok(a.fuel<fuel);assert.ok(a.rpm>90);assert.ok(a.ftit>800);assert.ok(a.nozzle>80);assert.ok(a.oilPsi>50);
});
check('simulation stays finite across speed and altitude envelope',()=>{
  for(const alt of [1500,6000,12000,16000])for(const speed of [90,220,400,600]){
    const a=run(aircraft(alt,speed),12,{pitchAxis:.25,rollAxis:.07});
    for(const n of [...a.pos,...a.vel,...a.q,a.gLoad,a.alpha,a.airspeedKt])assert.ok(Number.isFinite(n));
    assert.ok(Math.abs(Math.hypot(...a.q)-1)<1e-6);assert.ok(a.fuel>=0);assert.ok(a.gAvail<=9);
  }
});
console.log(`${checks} physics checks passed`);
