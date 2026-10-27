#!/usr/bin/env python3
"""Capture the model/cockpit regression matrix for visual inspection.

Usage: python tools/check_airframe_views.py PATH_TO_EXECUTABLE
Images are intentionally ignored by Git. Review the images as well as running
cargo test; a successful process alone does not prove correct visual geometry.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--output", type=Path, default=Path("target/model-review/final"))
    args = parser.parse_args()
    executable = args.executable.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    cases = []
    for jet, zoom in [("f18", "1.1"), ("su27", "1.3")]:
        for view, options in [
            ("forward", []),
            ("canopy", ["--look", "0,0"]),
            ("profile", ["--chase", "--look", "90,0", "--zoom", zoom]),
            ("head-on", ["--chase", "--look", "180,0", "--zoom", zoom]),
            ("panel", ["--look", "0,-32"]),
            ("systems", ["--look", "0,-32", "--mach", "0.25", "--input",
                         "gear,flaps,brake", "--sim", "3"]),
            ("left-console", ["--look", "-60,-40"]),
            ("right-console", ["--look", "60,-40"]),
            ("front", ["--chase", "--look", "145,20", "--zoom", zoom]),
            ("rear", ["--chase", "--look", "20,-15", "--zoom", zoom]),
            ("top", ["--chase", "--look", "180,65", "--zoom", str(float(zoom) * 1.55)]),
            ("rudder", ["--input", "yawright", "--sim", "0.4", "--chase",
                        "--look", "160,5", "--zoom", zoom]),
            ("gear", ["--mach", "0.25", "--input", "gear,flaps", "--sim", "8",
                      "--chase", "--look", "145,-10", "--zoom", zoom]),
            ("brake", ["--input", "brake", "--sim", "1", "--chase",
                       "--look", "145,20", "--zoom", zoom]),
            ("stores-front", ["--mission", "dogfight", "--chase",
                              "--look", "145,10", "--zoom", zoom]),
            ("stores-bottom", ["--mission", "dogfight", "--chase",
                               "--look", "170,-25", "--zoom", zoom]),
            ("stores-profile", ["--mission", "dogfight", "--chase",
                                "--look", "90,0", "--zoom", zoom]),
            ("rear-cockpit", ["--look", "160,-15"]),
        ]:
            cases.append((jet, view, options))
    cases.append(("f18", "weapons", ["--mission", "dogfight", "--weapon", "aim120", "--sim", "0.25"]))
    cases.append(("su27", "irst", ["--mission", "dogfight", "--weapon", "aim9", "--input", "irst", "--sim", "0.25"]))
    for jet in ["f16", "mig29", "f18", "su27"]:
        cases.append((jet, "detached-gear", [
            "--break-gear", "--sim", "0.5", "--chase", "--look", "100,-10",
            "--zoom", "1.3" if jet == "su27" else "1.1"]))
    startup = None
    if os.name == "nt":
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = 0
    report = []
    for jet, view, options in cases:
        output = (args.output / f"{jet}-{view}.png").resolve()
        command = [str(executable), "--autostart", "--no-performance", "--jet", jet,
                   "--size", "1440x900", "--shot", str(output), *options]
        completed = subprocess.run(command, timeout=90, capture_output=True,
                                   text=True, startupinfo=startup)
        if completed.returncode != 0:
            raise RuntimeError(f"{jet} {view}: {completed.stdout} {completed.stderr}")
        if not output.exists() or output.read_bytes()[:8] != b"\x89PNG\r\n\x1a\n":
            raise RuntimeError(f"Missing or invalid screenshot: {output}")
        report.append({"jet": jet, "view": view, "image": str(output)})
        print(f"Captured {jet} {view}", flush=True)
    (args.output / "captures.json").write_text(json.dumps(report, indent=2) + "\n")
    (args.output / "binary.sha256").write_text(hashlib.sha256(executable.read_bytes()).hexdigest() + "\n")
    print(f"{len(report)} screenshots ready for visual inspection.")

if __name__ == "__main__":
    main()
