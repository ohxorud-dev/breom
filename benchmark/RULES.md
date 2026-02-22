# Benchmarks

This directory contains micro-benchmarks across multiple languages.

Layout:

- `benchmark/run.go`: benchmark runner (Go implementation)
- `benchmark/run.sh`: thin wrapper for `go run ./benchmark/run.go`
- `<category>/<algorithm>/main.<ext>`: per-language implementations in one folder

Current categories:

- `sorting`
- `searching`
- `iterating`
- `file_io`

Supported language files:

- `main.brm` (Breom)
- `main.rs` (Rust)
- `main.go` (Go)
- `main.py` (Python)
- `main.js` (Node.js)
- `main.cpp` (C++)
- `Main.java` (Java)

Usage:

```bash
./benchmark/run.sh
./benchmark/run.sh --category sorting
./benchmark/run.sh --category sorting --algorithm bubble_sort
./benchmark/run.sh --category sorting --algorithm bubble_sort --lang rust
./benchmark/run.sh --category sorting --algorithm bubble_sort --lang rust,cpp
./benchmark/run.sh --category sorting --algorithm bubble_sort --exclude-lang python,node
./benchmark/run.sh --category sorting --algorithm bubble_sort --lang cpp
./benchmark/run.sh --category sorting --algorithm bubble_sort --lang java
./benchmark/run.sh --warmup 2 --repeat 5
./benchmark/run.sh --mode total
./benchmark/run.sh --mode exec
./benchmark/run.sh --metric median
./benchmark/run.sh --metric mean
./benchmark/run.sh --split-profiles
./benchmark/run.sh --split-profiles --lang rust
./benchmark/run.sh --split-profiles --lang cpp
./benchmark/run.sh --split-profiles --lang python
./benchmark/run.sh --split-profiles --lang rust-release,cpp-O3,python-cpython-O
./benchmark/run.sh --split-profiles --rust-profiles dev,release,release-lto --cpp-opt-levels O0,O2,Ofast,Os --python-profiles cpython,pypy,pypy-O

# direct Go runner (equivalent)
cd benchmark
go run ./run.go --category sorting --algorithm bubble_sort
```

The runner auto-discovers benchmarks and skips missing language runtimes/tools.
For Breom, the runner prefers `../target/release/breom` when present (falls back to debug).
`--lang` accepts a comma-separated list and matches both base languages (e.g. `rust,cpp`) and split-profile ids (e.g. `rust-release,cpp-O3`).
`--exclude-lang` accepts a comma-separated list of languages and excludes the language with all its profiles (e.g. `rust` excludes `rust-*`, `cpp` excludes `cpp-*`, `python` excludes `python-*`).

Measurement modes:

- `--mode exec` (default): builds compiled languages once per case, then measures execute-only repeats.
- `--mode total`: measures end-to-end runtime per repeat (build/compile + execute).

Summary metric:

- `--metric median` (default): uses median for ranking/ratio in Summary and Quick View.
- `--metric mean`: uses mean for ranking/ratio in Summary and Quick View.
- Per-case output always prints both `mean` and `median` regardless of selected metric.

Profile split mode:

- `--split-profiles`: expands Rust/C++/Python into build/runtime variants for side-by-side comparison.
  - Rust default variants: `rust-dev`, `rust-release`, `rust-release-native`, `rust-release-lto`, `rust-release-lto-native`, `rust-size`, `rust-size-z`
  - C++ default variants: `cpp-O0`, `cpp-O1`, `cpp-O2`, `cpp-O3`, `cpp-Ofast`, `cpp-Os`, `cpp-Oz`
  - Python default variants: `python-cpython`, `python-cpython-O`, `python-cpython-OO`, `python-pypy`, `python-pypy-O`, `python-pypy-OO`
- `--rust-profiles`: configure Rust variants in split mode (`dev`, `release`, `release-native`, `release-lto`, `release-lto-native`, `size`, `size-z`).
- `--cpp-opt-levels`: configure C++ variants in split mode (`O0`, `O1`, `O2`, `O3`, `Ofast`, `Os`, `Oz`).
- `--python-profiles`: configure Python runtime/flags in split mode (`cpython`, `cpython-O`, `cpython-OO`, `pypy`, `pypy-O`, `pypy-OO`).
- In split mode, `--lang rust`, `--lang cpp`, or `--lang python` runs all expanded variants for that language.

Output consistency checks:

- If one implementation prints different outputs across repeats, runner prints a `NOTICE` line for that case.
- If outputs differ between languages for the same `<category>/<algorithm>`, runner prints a `NOTICE` section after summary.

Algorithms by category:

- `sorting`: `bubble_sort`, `insertion_sort`, `selection_sort`, `merge_sort`, `quick_sort`, `heap_sort`, `shell_sort`, `counting_sort`, `comb_sort`
- `searching`: `linear_search`, `binary_search`, `jump_search`, `interpolation_search`, `exponential_search`, `ternary_search`, `fibonacci_search`, `lower_bound_search`, `sentinel_linear_search`
- `iterating`: `array_sum`, `nested_loop`, `prefix_sum`, `histogram_count`
- `file_io`: `line_count`, `sequential_read`, `line_length_sum`
