"""Provenance, UV orientation and offline paint reconstruction."""
from pathlib import Path
import gzip,hashlib,json,tempfile,unittest
import numpy as np
from PIL import Image
import import_liveries as paint
import import_superhornet as hornet

ASSETS=Path(__file__).resolve().parents[1]/'assets/models'

class PaintSources(unittest.TestCase):
    def test_original_inputs_and_paint_are_pinned(self):
        rows=json.loads((ASSETS/'livery-sources.json').read_text())
        for jet,row in rows.items():
            self.assertEqual(len(row['revision']),40)
            self.assertGreater(min(row['covered_pixels']),100000)
            self.assertEqual(hashlib.sha256((ASSETS/(jet+'-skin.qoi')).read_bytes()).hexdigest(),row['sha256'])
            for entry in row['inputs']:
                p=ASSETS/'source/liveries'/jet/entry['path']
                data=p.read_bytes() if p.exists() else gzip.decompress(Path(str(p)+'.gz').read_bytes())
                self.assertEqual(hashlib.sha256(data).hexdigest(),entry['sha256'])

    def test_ac3d_bottom_origin_is_converted_and_alpha_does_not_paint(self):
        rgb=np.zeros((2,2,3),dtype=np.uint8);z=np.full((2,2),-np.inf)
        image=np.array([[[255,0,0,255],[0,255,0,255]],[[0,0,255,255],[255,255,255,255]]],dtype=np.uint8)
        points=[(0,0,1),(1,0,1),(0,1,1)];uv=[(0,1),(1,1),(0,0)]
        paint.raster_triangle(rgb,z,points,uv,image)
        self.assertEqual(rgb[0,0].tolist(),[255,0,0]);self.assertEqual(rgb[1,0].tolist(),[0,0,255])
        self.assertEqual(rgb[0,1].tolist(),[0,255,0]);self.assertTrue(np.isneginf(z[1,1]))
        image[0,0,3]=0;rgb[:]=0;z[:]=-np.inf
        paint.raster_triangle(rgb,z,points,uv,image)
        self.assertTrue(np.isneginf(z[0,0]));self.assertTrue(np.isfinite(z[1,0]))
        u,v=hornet.atlas_uv(.5,1.,(0,0,2048,2048),(2048,2560))
        self.assertAlmostEqual(u,.5);self.assertLess(v,.001)
        self.assertGreater(hornet.atlas_uv(.5,0.,(0,2048,512,512),(2048,2560))[1],.99)

    def test_depth_and_edge_padding_keep_near_skin_paint(self):
        rgb=np.zeros((4,4,3),dtype=np.uint8);z=np.full((4,4),-np.inf)
        uv=[(0,1),(1,1),(0,0)]
        for depth,colour in [(2,[30,40,50,255]),(1,[255,0,0,255])]:
            paint.raster_triangle(rgb,z,[(0,0,depth),(1,0,depth),(0,1,depth)],uv,np.array([[colour]],dtype=np.uint8))
        self.assertEqual(rgb[0,0].tolist(),[30,40,50])
        valid=np.isfinite(z);paint.pad(rgb,valid,8)
        self.assertTrue(valid.all());self.assertTrue((rgb==[30,40,50]).all())

    def test_blank_canvas_is_removed_but_isolated_white_markings_survive(self):
        with tempfile.TemporaryDirectory(prefix='viper-canvas-') as temp:
            image=Image.new('RGB',(7,7),'white')
            for y in range(1,6):
                for x in range(1,6):image.putpixel((x,y),(120,125,130))
            image.putpixel((3,3),(255,255,255));path=Path(temp)/'source.png';image.save(path)
            px=paint.source_image(path,'f16')
            self.assertEqual(int(px[0,0,3]),0);self.assertEqual(int(px[3,3,3]),255)

    def test_bundled_sources_rebuild_both_atlases_offline(self):
        with tempfile.TemporaryDirectory(prefix='viper-paint-') as temp:
            source=Path(temp)/'source';out=Path(temp)/'out'
            for p in (ASSETS/'source/liveries').rglob('*'):
                if not p.is_file():continue
                dest=source/p.relative_to(ASSETS/'source/liveries')
                if p.suffix=='.gz':dest=dest.with_suffix('')
                dest.parent.mkdir(parents=True,exist_ok=True)
                dest.write_bytes(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
            paint.generate(source,out)
            for name in ['f16-skin.qoi','mig29-skin.qoi','livery-sources.json']:
                self.assertEqual((out/name).read_bytes(),(ASSETS/name).read_bytes(),name)

if __name__=='__main__':unittest.main()
