"""Normalize Typst-imported mesh groups for Edge/Adobe.

Keeps isolation, opacity, geometry and every decoded stream unchanged.
The final document is rewritten in memory, without illustration intermediates.
Requires PyMuPDF; usage: python normalize-pdf-groups.py document.pdf
"""
from pathlib import Path
import pymupdf as pdf
import sys

root = Path(__file__).resolve().parents[1]
path = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "main-type4.pdf"
doc = pdf.open(path)
changed = []
for xref in range(1, doc.xref_length()):
    if doc.xref_get_key(xref, "Subtype")[1] != "/Form":
        continue
    if doc.xref_get_key(xref, "Group/S")[1] != "/Transparency":
        continue
    if doc.xref_get_key(xref, "Resources/Shading")[0] == "null":
        continue
    if doc.xref_get_key(xref, "Group/CS")[1] != "/DeviceRGB":
        doc.xref_set_key(xref, "Group/CS", "/DeviceRGB")
        changed.append(xref)
dest = path
result = doc.tobytes(deflate=True)
check = pdf.open(stream=result, filetype="pdf")
original = pdf.open(path)
for xref in range(1, original.xref_length()):
    if original.xref_is_stream(xref):
        assert original.xref_stream(xref) == check.xref_stream(xref)
assert len(check) == len(original)
original.close()
check.close()
doc.close()
dest.write_bytes(result)
print(f"{dest}: {dest.stat().st_size:,} bytes; {len(changed)} group spaces changed; all decoded streams unchanged")
