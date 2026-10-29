"""Tessellate a STEP reference for Blender; never packages CAD in the game.
Requires cadquery-ocp-novtk in a separate development environment.
"""
import argparse,hashlib,json
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepMesh import BRepMesh_IncrementalMesh
from OCP.StlAPI import StlAPI_Writer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument("source",type=Path);ap.add_argument("output",type=Path)
    args=ap.parse_args();args.output.parent.mkdir(parents=True,exist_ok=True)
    reader=STEPControl_Reader()
    if reader.ReadFile(str(args.source))!=IFSelect_RetDone:raise ValueError("STEP read failed")
    reader.TransferRoots();shape=reader.OneShape()
    from OCP.BRep import BRep_Tool
    from OCP.TopoDS import TopoDS
    from OCP.TopLoc import TopLoc_Location
    from OCP.TopAbs import TopAbs_FACE,TopAbs_REVERSED
    # Some reference STEP faces have inconsistent tolerances. Export every
    # successful triangulation and record missing faces instead of inventing it.
    try:BRepMesh_IncrementalMesh(shape,0.7,False,0.25,True).Perform()
    except Exception as error:print("Assembly mesher:",type(error).__name__)
    explorer=TopExp_Explorer(shape,TopAbs_FACE);base=1;missing=0;faces=0;solids=[]
    lo=[float("inf")]*3;hi=[float("-inf")]*3
    with args.output.open("w") as out:
        while explorer.More():
            face=TopoDS.Face_s(explorer.Current());loc=TopLoc_Location()
            tri=BRep_Tool.Triangulation_s(face,loc)
            if tri is None:
                try:BRepMesh_IncrementalMesh(face,0.7,False,0.25).Perform()
                except Exception:pass
                tri=BRep_Tool.Triangulation_s(face,loc)
            if tri is None:missing+=1;explorer.Next();continue
            for i in range(1,tri.NbNodes()+1):
                point=tri.Node(i).Transformed(loc.Transformation())
                xyz=(point.X(),point.Y(),point.Z())
                out.write("v "+" ".join(str(v) for v in xyz)+"\n")
                lo=[min(a,b) for a,b in zip(lo,xyz)];hi=[max(a,b) for a,b in zip(hi,xyz)]
            for i in range(1,tri.NbTriangles()+1):
                ids=tri.Triangle(i).Get()
                if face.Orientation()==TopAbs_REVERSED:ids=tuple(reversed(ids))
                out.write("f "+" ".join(str(base+j-1) for j in ids)+"\n")
            base+=tri.NbNodes();faces+=1;explorer.Next()
    bounds=lo+hi
    report={"source":str(args.source),"sha256":hashlib.sha256(args.source.read_bytes()).hexdigest(),
            "units":"STEP reader default millimetres","bounds":bounds,"meshed_faces":faces,"missing_faces":missing,
            "tessellation_mm":0.7,"angular_deflection_rad":0.25}
    args.output.with_suffix(".json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps({k:v for k,v in report.items() if k!="solid_bounds"}))
    print("Vertices",base-1,"missing CAD faces",missing)
if __name__=="__main__":main()
