from pathlib import Path
import pymupdf as pdf
from PIL import ImageCms
source=Path('nurbs-surface/main-type4.pdf')
out=pdf.open();page=out.new_page(width=900,height=640)
clip=pdf.Rect(175,78,423,321)
variants=[('A Original',False,False,False),('B Explicit sRGB',True,False,False),('C AntiAlias off',False,True,False),('D Coordinates 16-bit',False,False,True),('E sRGB + coordinates 16',True,False,True)]
for index,(label,srgb,noaa,coord16) in enumerate(variants):
 doc=pdf.open(source)
 if srgb:
  icc=doc.get_new_xref();doc.update_object(icc,'<< /N 3 /Alternate /DeviceRGB >>');doc.update_stream(icc,ImageCms.ImageCmsProfile(ImageCms.createProfile('sRGB')).tobytes())
 for i in range(1,doc.xref_length()):
  obj=doc.xref_object(i)
  if '/ShadingType 4' not in obj:continue
  if srgb:doc.xref_set_key(i,'ColorSpace',f'[/ICCBased {icc} 0 R]')
  if noaa:doc.xref_set_key(i,'AntiAlias','false')
  if coord16:
   data=doc.xref_stream(i);buf=bytearray()
   for j in range(0,len(data),15):
    r=data[j:j+15];buf.append(r[0])
    for k in [1,5]:buf.extend(round(int.from_bytes(r[k:k+4],'big')/65537).to_bytes(2,'big'))
    buf.extend(r[9:])
   doc.xref_set_key(i,'BitsPerCoordinate','16');doc.update_stream(i,bytes(buf))
 x=(index%3)*300;y=(index//3)*320
 page.insert_text((x+16,y+22),label,fontsize=13)
 page.show_pdf_page(pdf.Rect(x+10,y+35,x+290,y+310),doc,12,clip=clip)
 # keep source objects alive until copied; show_pdf_page copies resources now
 if index==1:doc.save('nurbs-surface/main-type4-srgb.pdf',deflate=True)
ref=pdf.open(source);pix=ref[12].get_pixmap(matrix=pdf.Matrix(3,3),clip=clip,alpha=False)
page.insert_text((616,342),'F Reference raster (comparison only)',fontsize=12)
page.insert_image(pdf.Rect(610,355,890,630),pixmap=pix)
out.save('nurbs-surface/edge-color-diagnostic.pdf',garbage=4,deflate=True)
print('Created one-page A-F diagnostic; source renderer unchanged.')
