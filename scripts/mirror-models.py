#!/usr/bin/env python3
"""Mirror the assistant's language models onto the hkfree.org server.

The app downloads its model only from our server (/models/), never from
Hugging Face or GitHub. This copies, for each model ID, the files WebLLM
needs, in the layout it asks for:

    DEST/<model-id>/resolve/main/<file>   weights, tokenizer, config
    DEST/libs/<model-lib>.wasm            the compiled model library

Which files and which library version come from the vendored WebLLM bundle,
so the mirror always matches the code that will load it. Already complete
files are skipped; every weight shard is checked against the model's own
index (ndarray-cache.json). Run it with `python3 -I`; it only downloads data.

    python3 -I scripts/mirror-models.py --dest ~/hkdiag/models \\
        --webllm web/vendor/web-llm/index.js \\
        Qwen2.5-0.5B-Instruct-q4f16_1-MLC Qwen2.5-1.5B-Instruct-q4f16_1-MLC
"""

import argparse
import hashlib
import json
import os
import re
import sys
import urllib.request

SAFE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
UA = {"User-Agent": "hkdiag-mirror (hkfree.org)"}


def record(bundle, model_id):
    """(Hugging Face repo URL, library URL) from the bundle's prebuilt model list."""
    text = open(bundle, encoding="utf-8").read()
    prefix = re.search(r'modelLibURLPrefix\s*=\s*"([^"]+)"', text).group(1)
    version = re.search(r'modelVersion\s*=\s*"([^"]+)"', text).group(1)
    m = re.search(
        r'model:\s*"(https://huggingface\.co/[^"]+)",\s*model_id:\s*"' + re.escape(model_id)
        + r'",\s*model_lib:\s*modelLibURLPrefix\s*\+\s*modelVersion\s*\+\s*"([^"]+)"',
        text,
    )
    if not m:
        sys.exit(f"{model_id}: not in the vendored WebLLM model list")
    return m.group(1), prefix + version + m.group(2)


def fetch(url, dest, size=None):
    """Downloads url to dest (via dest.part), unless dest already has `size` bytes."""
    if size is not None and os.path.isfile(dest) and os.path.getsize(dest) == size:
        return False
    part = dest + ".part"
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=120) as r, open(part, "wb") as f:
        while chunk := r.read(1 << 20):
            f.write(chunk)
    if size is not None and os.path.getsize(part) != size:
        os.remove(part)
        sys.exit(f"{url}: expected {size} bytes, got a different size")
    os.replace(part, dest)
    return True


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            h.update(chunk)
    return h.hexdigest()


def mirror(dest, bundle, model_id):
    if not SAFE.match(model_id):
        sys.exit(f"{model_id}: not a plain model ID")
    repo, lib_url = record(bundle, model_id)
    api = repo.replace("https://huggingface.co/", "https://huggingface.co/api/models/")
    with urllib.request.urlopen(urllib.request.Request(api, headers=UA), timeout=60) as r:
        info = json.load(r)
    # Repository housekeeping (.gitattributes) isn't part of the model.
    files = [s["rfilename"] for s in info.get("siblings", []) if not s["rfilename"].startswith(".")]
    bad = [f for f in files if not SAFE.match(f)]
    if bad:
        sys.exit(f"{model_id}: unexpected file names {bad[:3]}")
    target = os.path.join(dest, model_id, "resolve", "main")
    os.makedirs(target, exist_ok=True)
    # The index first: it says how big every weight shard must be.
    fetch(f"{repo}/resolve/main/ndarray-cache.json", os.path.join(target, "ndarray-cache.json"))
    index = json.load(open(os.path.join(target, "ndarray-cache.json")))
    sizes = {r["dataPath"]: r["nbytes"] for r in index["records"]}
    for name in files:
        if name == "ndarray-cache.json":
            continue
        new = fetch(f"{repo}/resolve/main/{name}", os.path.join(target, name), sizes.get(name))
        print(f"  {'got ' if new else 'have'} {model_id}/{name}")
    missing = [p for p in sizes if not os.path.isfile(os.path.join(target, p))]
    if missing:
        sys.exit(f"{model_id}: shards missing from the repository: {missing[:3]}")
    lib_name = lib_url.rsplit("/", 1)[1]
    if not SAFE.match(lib_name):
        sys.exit(f"{lib_url}: not a plain library name")
    os.makedirs(os.path.join(dest, "libs"), exist_ok=True)
    lib = os.path.join(dest, "libs", lib_name)
    if not os.path.isfile(lib):
        fetch(lib_url, lib)
    print(f"  lib  {lib_name}")
    return [os.path.join(target, n) for n in os.listdir(target) if not n.endswith(".part")] + [lib]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--dest", required=True)
    ap.add_argument("--webllm", required=True, help="the vendored web/vendor/web-llm/index.js")
    ap.add_argument("models", nargs="+")
    a = ap.parse_args()
    dest = os.path.expanduser(a.dest)
    paths = []
    for m in a.models:
        print(m)
        paths += mirror(dest, a.webllm, m)
    with open(os.path.join(dest, "SHA256SUMS.txt"), "w") as f:
        for p in sorted(set(paths)):
            f.write(f"{sha256(p)}  {os.path.relpath(p, dest)}\n")
    total = sum(os.path.getsize(p) for p in set(paths))
    print(f"done: {len(set(paths))} files, {total / 1e6:.0f} MB in {dest}")


if __name__ == "__main__":
    main()
