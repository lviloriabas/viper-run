"""Geometry and lossless asset-pipeline checks."""
import gzip, hashlib, io, json, math, struct, tempfile, unittest
from pathlib import Path
from PIL import Image, ImageChops
import import_flightgear as fg

class ImportChecks(unittest.TestCase):
    def test_ac3d_object_matrices_and_nested_translation(self):
        data="""AC3Db
MATERIAL "skin" rgb 1 1 1 amb 0 0 0 emis 0 0 0 spec 0 0 0 shi 0 trans 0
OBJECT world
kids 1
OBJECT group
name "parent"
loc 10 20 30
rot 0 -1 0 1 0 0 0 0 1
kids 1
OBJECT poly
name "child"
loc 2 0 0
numvert 3
0 0 0
1 0 0
0 1 0
numsurf 1
SURF 0x30
mat 0
refs 3
0 0 0
1 1 0
2 0 1
kids 0
"""
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/"test.ac";p.write_text(data)
            o,vs=next(fg.flatten(fg.read_ac(p)))
            self.assertEqual(o.name,"child")
            self.assertEqual(vs,[(10.,22.,30.),(10.,23.,30.),(9.,22.,30.)])

    def test_concave_polygon_preserves_the_intake_notch(self):
        p=[(0.,0.,0.),(2.,0.,0.),(2.,1.,0.),(1.,1.,0.),(1.,2.,0.),(0.,2.,0.)]
        area=0.
        for ids in fg.triangulate(p):
            a,b,c=[p[i] for i in ids];v=fg.cross(fg.sub(b,a),fg.sub(c,a))
            area+=math.sqrt(fg.dot(v,v))/2
            centre=tuple((a[k]+b[k]+c[k])/3 for k in range(3))
            self.assertFalse(centre[0]>1 and centre[1]>1)
        self.assertAlmostEqual(area,3.)

    def test_hdd_cutout_preserves_linear_uv_coordinates(self):
        p=[(0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.),(2.,0.,0.,0.,0.,0.,0.,0.,0.,1.,0.),(2.,0.,2.,0.,0.,0.,0.,0.,0.,1.,1.)]
        for poly in fg.without_rectangle(p,.5,1.5,.5,1.5):
            for v in poly:
                self.assertAlmostEqual(v[9],v[0]/2)
                self.assertAlmostEqual(v[10],v[2]/2)

    def test_qoi_preserves_runs_index_diff_luma_and_full_rgb(self):
        pixels=[(0,0,0)]*70+[(64,80,32),(65,79,33),(80,90,44),(255,0,150),(64,80,32)]*50
        im=Image.new("RGB",(len(pixels),1));im.putdata(pixels)
        decoded=Image.open(io.BytesIO(fg.qoi(im))).convert("RGB")
        self.assertEqual(decoded.size,im.size)
        self.assertEqual(decoded.tobytes(),im.tobytes())

    def test_every_bundled_source_matches_its_original_hash(self):
        root=fg.ROOT/"assets/models"
        for entry in json.loads((root/"sources.json").read_text()):
            p=root/"source"/entry["path"]
            data=gzip.decompress(Path(str(p)+".gz").read_bytes()) if p.suffix in (".ac",".xml") else p.read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(),entry["sha256"],entry["path"])

    def test_embedded_cockpit_images_keep_all_original_pixels(self):
        root=fg.ROOT/"assets/models";data=(root/"su27.vrm").read_bytes();at=4
        def u32():
            nonlocal at
            n=struct.unpack_from("<I",data,at)[0];at+=4;return n
        for _ in range(u32()):
            n=u32();at+=n;n=u32();at+=4*n
        count=u32();entries=json.loads((root/"geometry.json").read_text())["su27"]["textures"]
        self.assertEqual(count,len(entries))
        for entry in entries:
            n=u32();length=u32();at+=n*4
            image=Image.open(io.BytesIO(data[at:at+length])).convert("RGB");at+=length
            source=Image.open(root/"source"/entry["source"]).convert("RGB")
            self.assertEqual(image.size,source.size)
            self.assertIsNone(ImageChops.difference(image,source).getbbox(),entry["source"])
        self.assertEqual(at,len(data))

if __name__=="__main__":unittest.main()
