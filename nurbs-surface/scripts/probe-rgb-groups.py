"""QA candidate: change only Type 4 Form transparency blend spaces.

This is not the production build. Keeps isolated groups and opacity intact.
Requires PyMuPDF for the diagnostic edit; geometry, mesh bytes and page content
are unchanged. Edge/Adobe confirmation is required before adopting a fix.
"""
from pathlib import Path
import pymupdf as pdf

root = Path(__file__).resolve().parents[1]
doc = pdf.open(root / "main-type4.pdf")
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
assert changed, "No imported shading groups found"
dest = root / "main-type4-rgb-group.pdf"
doc.save(dest, deflate=True)
check = pdf.open(dest)
original = pdf.open(root / "main-type4.pdf")
for xref in range(1, original.xref_length()):
    if original.xref_is_stream(xref):
        assert original.xref_stream(xref) == check.xref_stream(xref)
assert len(check) == len(original)
print(f"{dest}: {dest.stat().st_size:,} bytes; {len(changed)} group spaces changed; all decoded streams unchanged")
