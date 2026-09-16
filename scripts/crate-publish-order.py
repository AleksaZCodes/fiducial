#!/usr/bin/env python3
"""Print workspace crates in an order `cargo publish` will accept.

`cargo publish` requires every path dependency to already exist on crates.io at
the declared version, so a crate cannot be published before the siblings it
depends on. This emits a topological order of the workspace members, dependencies
first.

The order is *derived* from the dependency graph rather than written down. The
release workflow previously carried `for crate in fiducial`, publishing one of
fifteen; a hand-maintained list is how that stays wrong when a sixteenth crate
is added.

Reads `cargo metadata --no-deps --format-version 1` JSON on stdin, or runs it
itself when stdin is a TTY. Prints one `name version` pair per line.

The version is emitted here rather than looked up again with `jq` in the
workflow: this script has already parsed the metadata, and a second parser is a
second thing that can disagree with the first.
"""

import json
import subprocess
import sys


def publish_order(metadata: dict) -> list[tuple[str, str]]:
    """Workspace members as `(name, version)`, dependencies before dependents."""
    names = {p["name"] for p in metadata["packages"]}
    # Only edges *within* the workspace matter; a crates.io dependency is
    # already published by definition.
    deps = {
        p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in names}
        for p in metadata["packages"]
    }

    versions = {p["name"]: p["version"] for p in metadata["packages"]}
    seen: set[str] = set()
    order: list[tuple[str, str]] = []

    def visit(name: str, path: tuple[str, ...] = ()) -> None:
        if name in path:
            cycle = " → ".join((*path[path.index(name) :], name))
            raise SystemExit(f"dependency cycle, which cargo cannot publish: {cycle}")
        if name in seen:
            return
        seen.add(name)
        for dep in sorted(deps[name]):
            visit(dep, (*path, name))
        order.append((name, versions[name]))

    # Sorted for determinism: the same workspace must always give the same
    # order, or a failed release is hard to reproduce.
    for name in sorted(names):
        visit(name)
    return order


def main() -> None:
    if sys.stdin.isatty():
        raw = subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    else:
        raw = sys.stdin.read()
    for name, version in publish_order(json.loads(raw)):
        print(f"{name} {version}")


if __name__ == "__main__":
    main()
