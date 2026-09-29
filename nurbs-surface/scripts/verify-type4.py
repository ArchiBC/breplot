"""Audit final Type 4 PDF with independent zlib/MuPDF decoders. No files written.
QA-only dependency: PyMuPDF. Usage: python verify-type4.py [document.pdf]
"""
import json,sys,zlib
from pathlib import Path
import pymupdf
path=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parents[1]/'main-type4.pdf'
doc=pymupdf.open(path);flags={0:0,1:0,2:0};triangles=streams=compressed=0
for i in range(1,doc.xref_length()):
 obj=doc.xref_object(i)
 if doc.xref_is_stream(i) and '/FlateDecode' in obj:
  assert zlib.decompress(doc.xref_stream_raw(i))==doc.xref_stream(i)
  compressed+=1
 if '/ShadingType 4' not in obj: continue
 assert '/BitsPerCoordinate 32' in obj and '/BitsPerComponent 16' in obj and '/BitsPerFlag 8' in obj
 data=doc.xref_stream(i);assert len(data)%15==0
 records=[data[j:j+15] for j in range(0,len(data),15)];j=0;previous=None
 while j<len(records):
  f=records[j][0];assert f in flags;flags[f]+=1
  if f==0:assert j+2<len(records);previous=records[j:j+3];j+=3
  else:assert previous is not None;previous=[previous[1 if f==1 else 0],previous[2],records[j]];j+=1
  triangles+=1
 streams+=1
print(json.dumps(dict(bytes=path.stat().st_size,pages=len(doc),streams=streams,triangles=triangles,flags=flags,verified_flate_streams=compressed,bitmap_images=sum(len(p.get_images()) for p in doc)),indent=2))
