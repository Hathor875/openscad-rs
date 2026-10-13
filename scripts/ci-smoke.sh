#!/usr/bin/env bash
set -euo pipefail

# Smoke test for the headless CLI: every bundled example has to evaluate, and
# the models with geometry have to export to ASCII STL, binary STL and OBJ.

binary="${1:-./target/debug/openscad-rust}"
out="${2:-out}"

if [[ ! -x "${binary}" ]]; then
    echo "smoke: ${binary} not found or not executable; run 'cargo build' first" >&2
    exit 1
fi

examples_dir="$(cd "$(dirname "$0")/.." && pwd)/examples"

echo "smoke: evaluating $(find "${examples_dir}" -name '*.scad' | wc -l) examples with ${binary}"
while IFS= read -r model; do
    "${binary}" "${model}" >/dev/null
done < <(find "${examples_dir}" -name '*.scad' | sort)

mkdir -p "${out}"
for name in cyl if named test trans union; do
    model="${examples_dir}/${name}.scad"
    "${binary}" "${model}" -o "${out}/${name}.stl"
    "${binary}" "${model}" -o "${out}/${name}.bin.stl" --binary
    "${binary}" "${model}" -o "${out}/${name}.obj"

    [[ "$(head -c 5 "${out}/${name}.stl")" == "solid" ]]
    [[ "$(grep -c '^endsolid' "${out}/${name}.stl")" -gt 0 ]]

    binary_size=$(stat -c%s "${out}/${name}.bin.stl")
    if (( (binary_size - 84) % 50 != 0 )); then
        echo "smoke: ${name}: binary STL is ${binary_size} bytes, expected 84 + n*50" >&2
        exit 1
    fi

    [[ "$(head -n 1 "${out}/${name}.obj")" == "v "* ]]

    echo "smoke: ${name} OK"
done

echo "smoke: OK"