"""End-to-end Typst/WASM benchmark. Run with Python from any directory.

Each sample starts a new Typst process; no persistent compilation cache.
Fixtures contain distinct inputs to avoid memoizing repeated curves.
Generated files and detailed results go under target/benchmark.
"""
import argparse
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "benchmark"
OUT.mkdir(parents=True, exist_ok=True)
REPEATS = 5
TIMEOUT = 30
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--case", help="Run only cases containing this substring")
args = parser.parse_args()


def run(command):
    started = time.perf_counter()
    p = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        stdout, stderr = p.communicate(timeout=TIMEOUT)
    except subprocess.TimeoutExpired:
        p.kill()
        p.communicate()
        return {"timeout": TIMEOUT}
    elapsed = time.perf_counter() - started
    peak = None
    if os.name == "nt":
        class Counters(ctypes.Structure):
            _fields_ = [("cb", ctypes.c_ulong), ("faults", ctypes.c_ulong)] + [
                (key, ctypes.c_size_t) for key in (
                    "peak_working", "working", "peak_paged", "paged",
                    "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile")]
        c = Counters()
        c.cb = ctypes.sizeof(c)
        fn = ctypes.windll.psapi.GetProcessMemoryInfo
        fn.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_ulong]
        if fn(int(p._handle), ctypes.byref(c), c.cb):
            peak = c.peak_working / (1024 * 1024)
    return {"seconds": elapsed, "peak_mib": peak, "exit": p.returncode,
            "stdout": stdout.decode("utf-8", errors="replace"),
            "stderr": stderr.decode("utf-8", errors="replace")}


def points(n, variant=0):
    return [[i / (n - 1) * 10,
             math.sin(i / (n - 1) * 4 * math.pi + variant * .013)
             * (1 + variant * .001), 0] for i in range(n)]


def curve(n, degree=3, rational=False, tolerance=.001, variant=0):
    return dict(control_points=points(n, variant),
                knots=[0] * (degree + 1) + list(range(1, n - degree))
                + [n - degree] * (degree + 1),
                weights=[1 + .6 * math.sin(i * 1.7) if rational else 1
                         for i in range(n)], tolerance=tolerance)


PREFIX = '''#import "@preview/cetz:0.5.2" as cetz
#import "../../package/lib.typ": nurbs, native-cubics, from-control-points, from-interpolation-points
#set page(width: 160mm, height: auto, margin: 8mm)
#set text(font: "Arial", size: 9pt)
'''


def fixture(name, inputs, draw=False, constructor=None, periodic=False, shared=False):
    data_path = OUT / (name + ".json")
    data_path.write_text(json.dumps(inputs), encoding="utf-8")
    code = PREFIX + f'#let inputs = json("{data_path.name}")\n#let curves = inputs.map(item => '
    if constructor:
        code += f'{constructor}(item, degree: 3, periodic: {str(periodic).lower()}))\n'
    else:
        code += 'item)\n'
    code += '''#let counts = curves.map(c => native-cubics(c).len())
#metadata((curves: curves.len(), segments: counts.sum())) <bench-result>
#text(str(counts.sum()))
'''
    if shared:
        code += '''#cetz.canvas(length: 10mm, {
  for c in curves { nurbs(c, stroke: blue + 0.7pt) }
})
'''
    elif draw:
        code += '''#for c in curves {
  cetz.canvas(length: 10mm, { nurbs(c, stroke: blue + 0.7pt) })
  parbreak()
}
'''
    path = OUT / (name + ".typ")
    path.write_text(code, encoding="utf-8")
    return path


cases = []
for name, content in [("empty", "Benchmark baseline"), ("imports", PREFIX + "Benchmark baseline")]:
    path = OUT / (name + ".typ")
    path.write_text(content, encoding="utf-8")
    cases.append((name, path))
cases.append(("article_12_pages", ROOT / "main.typ"))
for n in (8, 32, 128, 512, 1024):
    cases.append((f"poly3_n{n}_eval", fixture(f"poly3_n{n}_eval", [curve(n)])))
for count in (1, 10, 100):
    name = f"poly3_n32_draw{count}"
    cases.append((name, fixture(name, [curve(32, variant=i) for i in range(count)], draw=True)))
cases.append(("poly3_n1024_draw", fixture("poly3_n1024_draw", [curve(1024)], draw=True)))
cases.append(("poly3_n32_shared100", fixture("poly3_n32_shared100", [curve(32,variant=i) for i in range(100)], shared=True)))
for degree in (3, 5, 8):
    for tol in (.01, .0001, .000001):
        name = f"rational_p{degree}_n32_tol{tol}"
        cases.append((name, fixture(name, [curve(32, degree, True, tol)], draw=True)))
for periodic in (False, True):
    for n in (16, 64, 128, 256, 512):
        name = f"interpolate_n{n}_{'periodic' if periodic else 'open'}"
        cases.append((name, fixture(name, [points(n)], constructor="from-interpolation-points", periodic=periodic)))
for constructor in ("from-control-points", "from-interpolation-points"):
    name = constructor + "_n128_draw"
    cases.append((name, fixture(name, [points(128)], draw=True, constructor=constructor)))

results = dict(typst=subprocess.check_output(["typst", "--version"], text=True).strip(),
               wasm_sha256=hashlib.sha256((ROOT / "package/cetz_nurbs.wasm").read_bytes()).hexdigest(),
               repeats=REPEATS, timeout_seconds=TIMEOUT,
               clock="time.perf_counter wall time; new process per sample",
               memory="Windows process peak working set, MiB", cases=[])
result_path = OUT / ("results.filtered.json" if args.case else "results.json")
for name, path in cases:
    if args.case and args.case not in name:
        continue
    command = ["typst", "compile", "--root", str(ROOT), str(path), str(OUT / (name + ".pdf"))]
    warmup = run(command)
    row = dict(name=name, warmup=warmup)
    if warmup.get("exit") == 0:
        samples = [run(command) for _ in range(REPEATS)]
        row["samples"] = samples
        if all(s.get("exit") == 0 for s in samples):
            times = [s["seconds"] for s in samples]
            row.update(median_seconds=statistics.median(times), min_seconds=min(times),
                       max_seconds=max(times),
                       peak_mib=max(s["peak_mib"] or 0 for s in samples),
                       pdf_bytes=(OUT / (name + ".pdf")).stat().st_size)
        if path.parent == OUT and name not in ("empty", "imports"):
            q = run(["typst", "query", "--root", str(ROOT), str(path), "<bench-result>", "--field", "value"])
            row["counts"] = json.loads(q["stdout"]) if q.get("exit") == 0 else q
    results["cases"].append(row)
    result_path.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(json.dumps({k:v for k,v in row.items() if k not in ("samples", "warmup")}), flush=True)
print("Saved", result_path)
