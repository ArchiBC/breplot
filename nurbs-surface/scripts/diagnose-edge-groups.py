"""Isolate the imported PDF transparency group; does not change production output.

All four panels reuse the exact same quantized mesh colors and vector lines.
The fourth replaces Gouraud interpolation with each microtriangle's mean color:
it is a diagnostic control, not a new smooth-surface implementation.
Run from any directory. Requires PyMuPDF only (QA dependency).
"""
from pathlib import Path
import re
import pymupdf as pdf

ROOT = Path(__file__).resolve().parents[1]
source = pdf.open(ROOT / "main-type4.pdf")
form = source[12].get_xobjects()[0][0]
resources = source.xref_get_key(form, "Resources")[1]
mesh_id = int(re.search(r"/S1\s+(\d+) 0 R", resources)[1])
assert source.xref_get_key(mesh_id, "BitsPerCoordinate")[1] == "32"
assert source.xref_get_key(mesh_id, "BitsPerComponent")[1] == "16"
raw = source.xref_stream(mesh_id)
assert len(raw) % 15 == 0
paths = []
previous = None
pending = []
for offset in range(0, len(raw), 15):
    row = raw[offset:offset + 15]
    vertex = (int.from_bytes(row[1:5], "big") / 0xffffffff * 600,
              int.from_bytes(row[5:9], "big") / 0xffffffff * 480,
              *(int.from_bytes(row[k:k+2], "big") / 65535 for k in (9, 11, 13)))
    flag = row[0]
    if pending or flag == 0:
        pending.append(vertex)
        if len(pending) < 3:
            continue
        tri = pending
        pending = []
    else:
        assert previous is not None and flag in (1, 2)
        tri = [previous[1 if flag == 1 else 0], previous[2], vertex]
    previous = tri
    color = [sum(v[k] for v in tri) / 3 for k in (2, 3, 4)]
    paths.append(" ".join(f"{v:.8f}" for v in color) + " rg " +
                 " ".join(f"{v[0]:.6f} {v[1]:.6f} {'m' if i == 0 else 'l'}"
                          for i, v in enumerate(tri)) + " h f\n")
assert not pending

out = pdf.open()
page = out.new_page(width=900, height=760)
labels = ["A  Original ICC group", "B  DeviceRGB group",
          "C  No transparency group", "D  Flat vector paths (same RGB)"]
for i, label in enumerate(labels):
    # Copy the source graph, then expose the figure directly as a page.
    # This avoids retaining any ancestor from Typst or show_pdf_page's source.
    doc = pdf.open(stream=source.tobytes(), filetype="pdf")
    doc.select([12])
    p = doc[0]
    p.set_mediabox(pdf.Rect(0, 0, 600, 480))
    doc.xref_set_key(p.xref, "Resources", resources)
    doc.xref_set_key(p.xref, "Group", "null")
    if i < 2:
        cs = source.xref_get_key(form, "Group/CS")[1] if i == 0 else "/DeviceRGB"
        doc.xref_set_key(p.xref, "Group", f"<< /S /Transparency /I true /CS {cs} >>")
    content = source.xref_stream(form)
    if i == 3:
        assert content.count(b"/S1 sh") == 1
        content = content.replace(b"/S1 sh", "".join(paths).encode("ascii"))
    # A constant RGB fill is a local color-management control in every panel.
    content += b"\nq 0.32941176 0.60784314 0.76078431 rg 75 2 450 8 re f Q\n"
    stream = doc.get_new_xref()
    doc.update_object(stream, "<< >>")
    doc.update_stream(stream, content)
    p.set_contents(stream)
    x, y = (i % 2) * 450, (i // 2) * 380
    page.insert_text((x + 20, y + 24), label, fontsize=14)
    imported_form = page.show_pdf_page(pdf.Rect(x + 10, y + 40, x + 440, y + 370), doc, 0)
    # show_pdf_page does NOT copy the source page's /Group. Attach it to the
    # returned full-page Form explicitly; otherwise A/B silently become C.
    if i < 2:
        if i == 0:
            # Reuse the actual profile bytes, with references in this output PDF.
            original_cs = source.xref_get_key(form, "Group/CS")[1]
            cs_array = source.xref_object(int(original_cs.split()[0]))
            original_icc = int(re.search(r"/ICCBased\s+(\d+) 0 R", cs_array)[1])
            icc = out.get_new_xref()
            out.update_object(icc, "<< /N 3 >>")
            out.update_stream(icc, source.xref_stream(original_icc))
            cs = f"[/ICCBased {icc} 0 R]"
        else:
            cs = "/DeviceRGB"
        out.xref_set_key(imported_form, "Group", f"<< /S /Transparency /I true /CS {cs} >>")
        assert out.xref_get_key(imported_form, "Group/S")[1] == "/Transparency"
    else:
        assert out.xref_get_key(imported_form, "Group")[0] == "null"

destination = ROOT / "edge-group-diagnostic-v2.pdf"
out.save(destination, garbage=4, deflate=True)
print(f"{destination}: {destination.stat().st_size:,} bytes; {len(paths)} triangles")
