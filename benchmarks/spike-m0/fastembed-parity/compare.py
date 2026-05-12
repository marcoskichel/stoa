"""Compare two embedding JSON files (stdin: rust.json python.json).

Prints per-text cosine similarity between Rust and Python embeddings,
plus min / mean / max similarity across the corpus.
"""

import json
import math
import sys


def cosine(a: list[float], b: list[float]) -> float:
    dot = sum(x * y for x, y in zip(a, b, strict=True))
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(x * x for x in b))
    return dot / (na * nb)


def main() -> None:
    rust_path, py_path = sys.argv[1], sys.argv[2]
    with open(rust_path) as f:
        rust = json.load(f)
    with open(py_path) as f:
        py = json.load(f)

    assert len(rust) == len(py), f"len mismatch {len(rust)} vs {len(py)}"
    sims: list[float] = []
    for r, p in zip(rust, py, strict=True):
        assert r["text"] == p["text"], f"text mismatch {r['text']!r} vs {p['text']!r}"
        s = cosine(r["embedding"], p["embedding"])
        sims.append(s)
        print(f"{s:.6f}\t{r['text'][:60]}")

    print(f"\nn={len(sims)}")
    print(f"min={min(sims):.6f}")
    print(f"mean={sum(sims) / len(sims):.6f}")
    print(f"max={max(sims):.6f}")
    print(f"dim_rust={len(rust[0]['embedding'])}")
    print(f"dim_py={len(py[0]['embedding'])}")


if __name__ == "__main__":
    main()
