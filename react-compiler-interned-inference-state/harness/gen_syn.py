#!/usr/bin/env python3
"""Generate synthetic single-component TSX files of growing size.

Each of the N sections adds a state hook, a memoized object literal, a
callback closure capturing local values, a conditional (two branches merging
into one variable), and JSX referencing all of it, so the aliasing state
(values + variables) and the number of basic blocks both grow with N.

Usage: gen_syn.py <out-dir> <N> [<N> ...]
"""
import os
import sys


def component(n):
    lines = []
    lines.append('import { useState, useMemo, useCallback } from "react";')
    lines.append("")
    lines.append("export function BigComponent(props) {")
    lines.append("  const [status, setStatus] = useState(props.initialStatus);")
    for i in range(n):
        lines.append(f"  const [value{i}, setValue{i}] = useState(props.seed{i});")
        lines.append(
            f"  const model{i} = useMemo(() => ({{ value: value{i}, key: props.key{i}, status }}), [value{i}, props.key{i}, status]);"
        )
        lines.append(f"  const onChange{i} = useCallback(() => {{")
        lines.append(f"    setValue{i}(model{i}.value);")
        lines.append("    setStatus('changed');")
        lines.append("  }, [model" + str(i) + "]);")
        lines.append(f"  let cell{i} = null;")
        lines.append(f"  if (props.flag{i}) {{")
        lines.append(
            f"    cell{i} = <Row id={{{i}}} model={{model{i}}} onChange={{onChange{i}}} status={{status}} />;"
        )
        lines.append("  } else {")
        lines.append(f"    cell{i} = <span className=\"empty\">{{value{i}}}</span>;")
        lines.append("  }")
    lines.append("  return (")
    lines.append("    <div className=\"big\">")
    for i in range(n):
        lines.append(f"      {{cell{i}}}")
    lines.append("    </div>")
    lines.append("  );")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    for arg in sys.argv[2:]:
        n = int(arg)
        path = os.path.join(out_dir, f"syn_{n:05d}.tsx")
        with open(path, "w") as f:
            f.write(component(n))
        print(path)


if __name__ == "__main__":
    main()
