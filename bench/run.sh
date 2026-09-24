#!/usr/bin/env bash
# Both lexers, the same bytes, the same method -- and the two tables side by
# side at the end.
#
# Meadow is built with `--release --runtime aot` (LLVM at -O2) against Rust's
# `--release`, because the comparison is otherwise between an optimised build
# and an unoptimised one.
set -euo pipefail
cd "$(dirname "$0")"

echo "==> meadow (release, aot)"
meadow run --release --runtime aot | tee /tmp/logos-bench-meadow.txt
echo
echo "==> rust logos (release)"
( cd rust && cargo build --release --quiet && ./target/release/logos-bench ) \
  | tee /tmp/logos-bench-rust.txt

python3 - <<'PY'
import re

def rows(path):
    out = {}
    for line in open(path):
        m = re.match(r"^(\S.*?)\s{2,}(\d+) us\s+(\d+) KB/s\s*$", line.rstrip())
        if m:
            out[m.group(1).strip()] = (int(m.group(2)), int(m.group(3)))
    return out

mw = rows("/tmp/logos-bench-meadow.txt")
rs = rows("/tmp/logos-bench-rust.txt")

def mb(kb):
    return f"{kb/1000:,.1f} MB/s"

print()
print("=" * 64)
print(f"{'lexer':<26}{'best':>12}{'throughput':>16}")
print("-" * 64)
for name, (us, kb) in list(mw.items()) + list(rs.items()):
    label = name if name.startswith("logos") else f"meadow: {name}"
    print(f"{label:<26}{us:>9,} us{mb(kb):>16}")
print("=" * 64)

if "before" in mw and "after" in mw:
    print(f"the optimisations:  {mw['before'][0] / mw['after'][0]:.1f}x faster than the engine they replaced")
if "fold, kinds only" in mw and "logos" in rs:
    print(f"against rust logos: {mw['fold, kinds only'][0] / rs['logos'][0]:.0f}x slower (token kinds only, iterated)")
if "fold" in mw and "logos, owned text" in rs:
    print(f"                    {mw['fold'][0] / rs['logos, owned text'][0]:.0f}x slower against logos "
          f"building an owned String per token, which is what a")
    print( "                    meadow token carrying its text does -- the nearest like-for-like")
PY
