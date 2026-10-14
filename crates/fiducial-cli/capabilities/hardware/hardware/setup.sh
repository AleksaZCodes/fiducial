#!/usr/bin/env bash
# Everything `hardware/build.sh` needs, on this machine — the same steps CI runs.
#
#   hardware/setup.sh            install what is missing, then verify
#   hardware/setup.sh --check    verify only; exit 1 and say what is missing
#
# What it sets up, and why each piece:
#
#   KiCad      `kicad-cli` (fab outputs, STEP, schematic PDF) and the `pcbnew`
#              Python module route.py drives. pcbnew is built against the
#              system Python, so it comes from the system package manager.
#   Java 21    Freerouting, which route.py fetches once and pins by hash.
#   .venv      hardware/.venv — the system Python *with* its site-packages
#              (so pcbnew is importable) plus requirements.txt: build123d,
#              ortools, pypdf. A venv, because Ubuntu 23.04+ and Debian 12+
#              refuse `pip install` into the system Python (PEP 668).
#
# build.sh, and `fid derive` when it places parts, use hardware/.venv/bin/python
# whenever it exists. `FID_PYTHON` still overrides both.
#
# Installing system packages needs sudo (apt) or Homebrew; the script asks for
# nothing else. Platform-owned: `fid upgrade` keeps it current.
set -euo pipefail
cd "$(dirname "$0")/.."

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1
VENV=hardware/.venv

missing() {
  local m=()
  command -v kicad-cli >/dev/null || m+=("kicad-cli (KiCad)")
  if command -v java >/dev/null; then
    local v
    v=$(java -version 2>&1 | sed -n 's/.*version "\([0-9]*\).*/\1/p' | head -1)
    [ "${v:-0}" -ge 21 ] || m+=("java 21+ (found ${v:-unknown})")
  else
    m+=("java 21+")
  fi
  if [ -x "$VENV/bin/python" ]; then
    "$VENV/bin/python" -c "import pcbnew" 2>/dev/null || m+=("pcbnew in $VENV")
    "$VENV/bin/python" -c "import build123d, ortools, pypdf" 2>/dev/null ||
      m+=("requirements.txt in $VENV")
  else
    m+=("$VENV")
  fi
  printf '%s\n' "${m[@]}"
}

if [ "$CHECK_ONLY" = 1 ]; then
  gaps=$(missing)
  if [ -n "$gaps" ]; then
    echo "hardware/setup.sh --check: missing" >&2
    printf '%s\n' "$gaps" | sed 's/^/  /' >&2
    exit 1
  fi
  echo "hardware toolchain: ready"
  exit 0
fi

# ── System packages ──────────────────────────────────────────────────────────
base_python=python3
case "$(uname -s)" in
  Linux)
    if command -v apt-get >/dev/null; then
      need=()
      command -v kicad-cli >/dev/null || need+=(kicad)
      java -version 2>&1 | grep -qE 'version "(2[1-9]|[3-9][0-9])' || need+=(openjdk-21-jre-headless)
      python3 -c "import venv, ensurepip" 2>/dev/null || need+=(python3-venv)
      if [ ${#need[@]} -gt 0 ]; then
        echo "installing: ${need[*]} (apt, needs sudo)"
        sudo apt-get update -qq
        sudo apt-get install -y -qq --no-install-recommends "${need[@]}"
      fi
      # pcbnew is built for the distribution's Python; the venv must be too.
      base_python=/usr/bin/python3
    else
      echo "No apt-get here. Install KiCad 9, Java 21 and Python 3 with venv support" >&2
      echo "from your distribution, then re-run this script." >&2
      exit 1
    fi
    ;;
  Darwin)
    command -v brew >/dev/null || { echo "Install Homebrew first: https://brew.sh" >&2; exit 1; }
    command -v kicad-cli >/dev/null || brew install --cask kicad
    java -version 2>&1 | grep -qE 'version "(2[1-9]|[3-9][0-9])' || brew install openjdk@21
    # KiCad on macOS ships its own Python, and pcbnew lives in it.
    kpy=/Applications/KiCad/KiCad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3
    [ -x "$kpy" ] && base_python=$kpy
    ;;
  *)
    echo "Unsupported OS: $(uname -s). On Windows, run this inside WSL." >&2
    exit 1
    ;;
esac

# ── The Python environment ───────────────────────────────────────────────────
if [ ! -x "$VENV/bin/python" ]; then
  echo "creating $VENV from $base_python (with system site-packages, for pcbnew)"
  "$base_python" -m venv --system-site-packages "$VENV"
fi
"$VENV/bin/python" -m pip install -q --upgrade pip
"$VENV/bin/python" -m pip install -q -r hardware/requirements.txt

# ── Verify ───────────────────────────────────────────────────────────────────
gaps=$(missing)
if [ -n "$gaps" ]; then
  echo "hardware/setup.sh: still missing after setup:" >&2
  printf '%s\n' "$gaps" | sed 's/^/  /' >&2
  exit 1
fi
echo "hardware toolchain: ready ($(kicad-cli version 2>/dev/null | head -1), $("$VENV/bin/python" --version))"
