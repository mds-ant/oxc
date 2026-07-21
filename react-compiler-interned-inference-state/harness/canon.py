#!/usr/bin/env python3
"""Canonicalize an oxlint --format json run into an order-independent diagnostic set.

Usage: canon.py <stdout.json> <exitcode-file> <out.canon>

Each diagnostic becomes one line: filename, code, severity, message, help,
and its label spans (offset:length:label), all tab-joined; lines are sorted.
The header records the parse status, exit code, diagnostic count, and any
summary fields present in the JSON, so two runs are equal iff their .canon
files are byte-identical.
"""
import json
import sys


def main():
    raw_path, exit_path, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
    exit_code = open(exit_path).read().strip()
    raw = open(raw_path, encoding="utf-8", errors="replace").read()
    header = [f"exit={exit_code}"]
    lines = []
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as err:
        header.append(f"json_parse_error={err.msg} bytes={len(raw)}")
        data = None
    if data is not None:
        diags = data.get("diagnostics", [])
        header.append(f"diagnostics={len(diags)}")
        for key in sorted(k for k in data.keys() if k != "diagnostics"):
            value = data[key]
            if key in ("start_time", "duration", "threads_count"):
                continue
            header.append(f"{key}={value}")
        for d in diags:
            spans = sorted(
                f"{l.get('span', {}).get('offset')}:{l.get('span', {}).get('length')}:{l.get('label')}"
                for l in d.get("labels", [])
            )
            fields = [
                str(d.get("filename")),
                str(d.get("code")),
                str(d.get("severity")),
                str(d.get("message")),
                str(d.get("help")),
                ";".join(spans),
            ]
            lines.append("\t".join(fields))
    lines.sort()
    with open(out_path, "w", encoding="utf-8") as out:
        out.write("# " + " ".join(header) + "\n")
        for line in lines:
            out.write(line + "\n")
    print(" ".join(header))


if __name__ == "__main__":
    main()
