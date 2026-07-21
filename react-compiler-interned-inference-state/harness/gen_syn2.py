#!/usr/bin/env python3
"""Synthetic component shape v2: control-flow + closure heavy, minimal JSX.

Each section adds branching (ternaries, ??, &&/||, if/else) that creates basic
blocks, closures that capture locals, and object spreads, so the aliasing state
and the block count grow together while the lowering-heavy JSX stays constant.

Usage: gen_syn2.py <out-dir> <N> [<N> ...]
"""
import os
import sys


def component(n):
    L = []
    L.append('import { useState, useCallback } from "react";')
    L.append("")
    L.append("export function BigForm(props) {")
    L.append("  const [state, setState] = useState(props.initial);")
    L.append("  const [errors, setErrors] = useState(null);")
    for i in range(n):
        L.append(f"  const raw{i} = props.fields ? props.fields.f{i} : undefined;")
        L.append(f"  const value{i} = raw{i} ?? state.defaults?.f{i} ?? props.fallback;")
        L.append(f"  const invalid{i} = props.required{i} && !value{i} ? `f{i} is required` : null;")
        L.append(f"  const onChange{i} = useCallback((next) => {{")
        L.append(f"    if (next !== value{i}) {{")
        L.append(f"      setState({{ ...state, f{i}: next }});")
        L.append(f"      if (invalid{i}) {{ setErrors({{ ...errors, f{i}: null }}); }}")
        L.append("    }")
        L.append(f"  }}, [state, errors, value{i}, invalid{i}]);")
        L.append(f"  let summary{i};")
        L.append(f"  if (invalid{i}) {{")
        L.append(f"    summary{i} = invalid{i};")
        L.append(f"  }} else if (value{i} && props.verbose) {{")
        L.append(f"    summary{i} = String(value{i});")
        L.append("  } else {")
        L.append(f"    summary{i} = props.placeholder || '';")
        L.append("  }")
    L.append("  const fields = [")
    for i in range(n):
        L.append(f"    {{ value: value{i}, summary: summary{i}, onChange: onChange{i} }},")
    L.append("  ];")
    L.append("  return <Form fields={fields} status={state.status} errors={errors} />;")
    L.append("}")
    return "\n".join(L) + "\n"


def main():
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    for arg in sys.argv[2:]:
        n = int(arg)
        path = os.path.join(out_dir, f"cf_{n:05d}.tsx")
        with open(path, "w") as f:
            f.write(component(n))
        print(path)


if __name__ == "__main__":
    main()
