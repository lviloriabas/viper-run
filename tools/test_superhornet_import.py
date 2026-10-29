"""Offline provenance and reconstruction checks for the E/G adaptation."""
from pathlib import Path
import gzip,hashlib,json,struct,tempfile,unittest,zlib
import import_superhornet as sh

ASSETS=Path(__file__).resolve().parents[1]/'assets/models'

class SuperHornetSources(unittest.TestCase):
    def test_pinned_sources_match_their_recorded_hashes_and_permission(self):
        root=ASSETS/'source/superhornet'
        for row in json.loads((ASSETS/'superhornet-sources.json').read_text()):
            p=root/row['path']
            data=p.read_bytes() if p.exists() else gzip.decompress(Path(str(p)+'.gz').read_bytes())
            self.assertEqual(hashlib.sha256(data).hexdigest(),row['sha256'],row['path'])
        text=gzip.decompress((root/'F-18F-yasim-set.xml.gz').read_bytes()).decode()
        self.assertIn('GPL2+',text)
        self.assertTrue((root/'Models/PjedvajAutorization.png').is_file())

    def test_packed_models_restore_editable_bytes_and_have_untextured_nozzles(self):
        for kind in ('f18e','ea18g'):
            raw=(ASSETS/(kind+'.vrm')).read_bytes();packed=(ASSETS/(kind+'.vrz')).read_bytes()
            self.assertEqual(packed[:4],b'VRZ1')
            self.assertEqual(struct.unpack_from('<I',packed,4)[0],len(raw))
            self.assertEqual(zlib.decompress(packed[8:]),raw)
            count=struct.unpack_from('<I',raw,4)[0];at=8;parts={}
            for _ in range(count):
                n=struct.unpack_from('<I',raw,at)[0];at+=4;name=raw[at:at+n].decode();at+=n
                n=struct.unpack_from('<I',raw,at)[0];at+=4
                parts[name]=struct.unpack_from('<'+'f'*n,raw,at);at+=n*4
            self.assertEqual(len(parts['brake']),0)
            objects=json.loads((ASSETS/'superhornet-geometry.json').read_text())[kind]['objects']
            nozzle=next(o for o in objects if o['name']=='F414Nozzles')
            v=parts[nozzle['part']];first=nozzle['vertex_start']*11
            self.assertGreater(nozzle['vertex_count'],100)
            for i in range(first,first+nozzle['vertex_count']*11,11):self.assertLess(v[i+9],0.0)

    def test_bundled_editable_sources_rebuild_both_variants_without_network(self):
        with tempfile.TemporaryDirectory(prefix='superhornet-') as temp:
            root=Path(temp)/'source';out=Path(temp)/'generated'
            for p in (ASSETS/'source/superhornet').rglob('*'):
                if not p.is_file():continue
                dest=root/p.relative_to(ASSETS/'source/superhornet')
                if p.suffix=='.gz':dest=dest.with_suffix('')
                dest.parent.mkdir(parents=True,exist_ok=True)
                dest.write_bytes(gzip.decompress(p.read_bytes()) if p.suffix=='.gz' else p.read_bytes())
            sh.generate(root,out)
            for name in ('f18e.vrm','ea18g.vrm','superhornet-skin.qoi','growler-skin.qoi',
                         'superhornet-geometry.json','superhornet-sources.json'):
                self.assertEqual((out/name).read_bytes(),(ASSETS/name).read_bytes(),name)

if __name__=='__main__':unittest.main()
