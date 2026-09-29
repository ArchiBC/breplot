"""Opaque sphere-only PDF Type 4 feasibility probe; not the production renderer.
Uses the exported mesh, 2 uniform lighting subdivisions, 16-bit coordinates/RGB.
No third-party dependencies. Run after build.ps1; output goes to target/performance.
"""
import json, math, struct, zlib
from pathlib import Path
root = Path(__file__).resolve().parents[1]
out = root / 'target/performance'
out.mkdir(exist_ok=True)
mesh = next(m for m in json.loads((root/'target/meshes.json').read_text()) if m['id']=='sphere')
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def unit(a): return [x/math.sqrt(dot(a,a)) for x in a]
def cross(a,b): return [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
view=unit([4,6,4]); light=unit([-3,4,7]); right=unit(cross([0,0,1],view)); up=cross(view,right)
proj=[[dot(v['point'],right),dot(v['point'],up),dot(v['point'],view)] for v in mesh['vertices']]
lo=[min(p[c] for p in proj) for c in range(2)]; hi=[max(p[c] for p in proj) for c in range(2)]
scale=min(600*.92/(hi[0]-lo[0]),480*.92/(hi[1]-lo[1]))
verts=[([(p[0]-(lo[0]+hi[0])/2)*scale+300,(p[1]-(lo[1]+hi[1])/2)*scale+240],v['normal'] or unit(v['point'])) for p,v in zip(proj,mesh['vertices'])]
h=unit([x+y for x,y in zip(view,light)])
def color(n):
 n=unit(n); d=.28+.72*max(0,dot(n,light)); s=.25*max(0,dot(n,h))**32 if dot(n,light)>0 and dot(n,view)>0 else 0
 return [(1-s)*d*c/255+s for c in [84,155,194]]
def mid(a,b): return [[(x+y)/2 for x,y in zip(u,v)] for u,v in zip(a,b)]
data=bytearray()
def triangle(t,depth):
 if depth:
  a,b,c=t; ab,bc,ca=mid(a,b),mid(b,c),mid(c,a)
  for child in [(a,ab,ca),(ab,b,bc),(ca,bc,c),(ab,bc,ca)]: triangle(child,depth-1)
 else:
  for p,n in t:
   values=[p[0]/600,p[1]/480,*color(n)]
   data.extend(struct.pack('>B5H',0,*[round(max(0,min(1,v))*65535) for v in values]))
for t in sorted(mesh['triangles'],key=lambda t:sum(proj[i][2] for i in t)): triangle([verts[i] for i in t],2)
def stream(data,attrs=''):
 data=zlib.compress(data)
 return f'<< /Length {len(data)} /Filter /FlateDecode {attrs} >>\nstream\n'.encode()+data+b'\nendstream'
objects=[b'<< /Type /Catalog /Pages 2 0 R >>',b'<< /Type /Pages /Kids [3 0 R] /Count 1 >>',b'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 480] /Resources << /Shading << /S 4 0 R >> >> /Contents 5 0 R >>',stream(data,'/ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 16 /BitsPerComponent 16 /BitsPerFlag 8 /Decode [0 600 0 480 0 1 0 1 0 1]'),stream(b'/S sh')]
pdf=bytearray(b'%PDF-1.7\n'); offsets=[0]
for i,obj in enumerate(objects,1): offsets.append(len(pdf));pdf.extend(f'{i} 0 obj\n'.encode()+obj+b'\nendobj\n')
xref=len(pdf);pdf.extend(f'xref\n0 {len(offsets)}\n0000000000 65535 f \n'.encode())
for offset in offsets[1:]:pdf.extend(f'{offset:010} 00000 n \n'.encode())
pdf.extend(f'trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF'.encode())
(out/'mesh-probe.pdf').write_bytes(pdf)
(out/'mesh-probe.typ').write_text('#set page(width:600pt,height:480pt,margin:0pt)\n#image("mesh-probe.pdf",width:100%)')
print(f'{len(data)//33} triangles, {len(pdf)} bytes; opaque sphere-only experiment')
