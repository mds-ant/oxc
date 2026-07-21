#!/usr/bin/env python3
"""Synthetic component shape v3: accumulator with a growing alias set.

`config` is conditionally rebuilt in every section, so the SSA phi joining
the branches accumulates every historical config value: the variable's
value set (and the merge work over it) grows linearly with the section
count. Combined with the growing state size and block count, this is the
shape where per-entry deep copies of sets/reasons become super-quadratic.

Usage: gen_syn3.py <out-dir> <N> [<N> ...]
"""
import os
import sys


def component(n):
    L = []
    L.append('import { useState, useCallback } from "react";')
    L.append("")
    L.append("export function BigConfig(props) {")
    L.append("  const [state, setState] = useState(props.initial);")
    L.append("  let config = { base: props.base, status: state.status };")
    for i in range(n):
        L.append(f"  const value{i} = props.values ? props.values.v{i} : state.fallback;")
        L.append(f"  const handler{i} = useCallback(() => {{")
        L.append(f"    setState({{ ...state, last: value{i}, config }});")
        L.append(f"  }}, [state, value{i}, config]);")
        L.append(f"  if (props.flag{i}) {{")
        L.append(f"    config = {{ ...config, k{i}: value{i}, on{i}: handler{i} }};")
        L.append(f"  }} else if (props.alt{i}) {{")
        L.append(f"    config = props.override{i} ?? config;")
        L.append("  }")
    L.append("  return <Panel config={config} status={state.status} />;")
    L.append("}")
    return "\n".join(L) + "\n"


def main():
    out_dir = sys.argv[1]
    os.makedirs(out_dir, exist_ok=True)
    for arg in sys.argv[2:]:
        n = int(arg)
        path = os.path.join(out_dir, f"acc_{n:05d}.tsx")
        with open(path, "w") as f:
            f.write(component(n))
        print(path)


if __name__ == "__main__":
    main()
