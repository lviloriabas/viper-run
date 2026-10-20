import fs from 'node:fs';import vm from 'node:vm';import assert from 'node:assert/strict';
const sandbox={window:{},Math};vm.runInNewContext(fs.readFileSync(new URL('../src/airframe.js',import.meta.url),'utf8'),sandbox);
const model=sandbox.window.__VIPER_AIRFRAME.buildGeometry();
let vertices=0;
for(const name of['exterior','canopy','crew','flame']){
 const mesh=model[name];assert.ok(mesh.length>0&&mesh.length%27===0);
 for(let i=0;i<mesh.length;i+=9){
   assert.ok(mesh.slice(i,i+9).every(Number.isFinite),name+' contains non-finite data');
   assert.ok(Math.abs(Math.hypot(...mesh.slice(i+3,i+6))-1)<.001,name+' has invalid surface normals');vertices++;
 }
 console.log('PASS '+name+' geometry and normals');
}
const bounds={min:[Infinity,Infinity,Infinity],max:[-Infinity,-Infinity,-Infinity]};
for(let i=0;i<model.exterior.length;i+=9)for(let j=0;j<3;j++){
 bounds.min[j]=Math.min(bounds.min[j],model.exterior[i+j]);bounds.max[j]=Math.max(bounds.max[j],model.exterior[i+j]);
}
assert.ok(Math.abs(bounds.max[2]-bounds.min[2]-15.06)<.03);
assert.ok(Math.abs(bounds.max[0]-bounds.min[0]-9.96)<.03);
assert.ok(model.stats.triangleCount>15000);
console.log('PASS measured 15.06 m length and 9.96 m span');
console.log(vertices+' verified vertices');
