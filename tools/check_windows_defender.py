#!/usr/bin/env python3
"""Scan the Windows distribution with Defender without changing protection settings."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]

def main():
    if sys.platform != "win32":
        raise SystemExit("Run this check with native Windows Python and Microsoft Defender.")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--files", nargs="+", type=Path)
    parser.add_argument("--output", type=Path, default=ROOT / "target/multiplayer-review/defender-scan.json")
    args = parser.parse_args()
    files = args.files or [ROOT / "dist/viper-run-windows-x86_64.exe", ROOT / "dist/viper-server-windows-x86_64.exe"]
    platform = Path(os.environ["ProgramData"]) / "Microsoft/Windows Defender/Platform"
    versions = [p for p in platform.iterdir() if p.is_dir() and re.fullmatch(r"\d+(?:\.\d+)+-\d+", p.name)]
    versions.sort(key=lambda p: tuple(map(int, p.name.split("-")[0].split("."))), reverse=True)
    scanner = versions[0] / "MpCmdRun.exe" if versions else Path(os.environ["ProgramFiles"]) / "Windows Defender/MpCmdRun.exe"
    if not scanner.is_file():
        raise SystemExit("Microsoft Defender scanner is unavailable.")
    results = []
    for file in files:
        file = file.resolve(strict=True)
        digest = hashlib.sha256(file.read_bytes()).hexdigest()
        # Diagnostic only: no remediation, exclusions, or security policy changes.
        result = subprocess.run([str(scanner), "-Scan", "-ScanType", "3", "-File", str(file), "-DisableRemediation"],
                                capture_output=True, text=True, errors="replace", timeout=300)
        output = (result.stdout + result.stderr).strip()
        print(output, flush=True)
        results.append({"file": str(file), "sha256": digest, "scanner": str(scanner),
                        "utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                        "exitCode": result.returncode, "passed": result.returncode == 0, "output": output})
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(results, indent=2), encoding="utf-8")
    if not all(r["passed"] for r in results):
        raise SystemExit(f"Defender reported a detection or scan failure. Review {args.output}.")
    print(f"PASS Defender scans. Report: {args.output}", flush=True)

if __name__ == "__main__":
    main()
