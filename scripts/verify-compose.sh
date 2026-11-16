#!/usr/bin/env bash
#
# The derived Compose file runs, and the broker it configures enforces the ACL
# the topology implies.
#
# Everything else about `deploy = "compose"` is checked as text: that the file is
# derived, gated and general. This runs it. It scaffolds the building use case (an
# MQTT broker and one device, `controller`), derives, validates the file with
# `docker compose config`, starts the broker with TLS and a password file, and
# asserts what each user can and cannot do:
#
#   - a device publishes its own `state` and is delivered;
#   - it cannot publish another device's subtree, nor a `command` to itself;
#   - the server's command reaches the device;
#   - a wrong password is refused, and plaintext on the TLS port is refused.
#
# Not covered: the product's own server image (there is none to build here) and
# Caddy in front of it.
#
# Usage (needs a Docker daemon):
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-compose.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }
docker info >/dev/null 2>&1 || { echo "no Docker daemon" >&2; exit 1; }

work="$(mktemp -d)"
compose() { docker compose -f "$work/building/deploy/compose.yaml" "$@"; }
cleanup() { compose down -v >/dev/null 2>&1 || true; rm -rf "$work"; }
trap cleanup EXIT

echo "── compose ────────────────────────────────────────────────────────────"
cd "$work"
"$fid" new building >/dev/null
cd building
"$fid" add capability topology >/dev/null
"$fid" add capability deploy-compose >/dev/null
printf '\n[adapters]\ndeploy = "compose"\n' >> fiducial.toml
# The topology declares a broker, so derive owes its config and names it here.
sed -i 's|^  "deploy/Caddyfile",|  "deploy/Caddyfile",\n  "deploy/mosquitto/mosquitto.conf",\n  "deploy/mosquitto/acl",|' pipelines/deploy-compose.toml
"$fid" derive >/dev/null
compose config -q

# What a deployment supplies: certificates and the password file.
cd deploy
mkdir certs
openssl req -x509 -newkey rsa:2048 -nodes -keyout certs/server.key -out certs/server.crt \
  -days 1 -subj "/CN=mosquitto" -addext "subjectAltName=DNS:mosquitto,DNS:localhost" 2>/dev/null
cp certs/server.crt certs/ca.crt
chmod 644 certs/*
docker run --rm -v "$PWD/mosquitto:/m" eclipse-mosquitto:2 sh -c \
  'mosquitto_passwd -b -c /m/passwd server s3rv && mosquitto_passwd -b /m/passwd controller c0nt && chmod 644 /m/passwd' >/dev/null
compose up -d --no-deps mosquitto >/dev/null
for _ in $(seq 1 30); do
  compose exec -T mosquitto mosquitto_pub -h 127.0.0.1 -u server -P s3rv -t building/server/online -m up 2>/dev/null && break
  sleep 1
done

x() { compose exec -T mosquitto "$@"; }
x sh -c 'timeout 8 mosquitto_sub -h 127.0.0.1 -u server -P s3rv -t "building/+/state" -v > /tmp/server.out 2>&1' &
x sh -c 'timeout 8 mosquitto_sub -h 127.0.0.1 -u controller -P c0nt -t "building/controller/command" -v > /tmp/device.out 2>&1' &
sleep 2
pub() { x mosquitto_pub -h 127.0.0.1 -u "$1" -P "$2" -t "$3" -m "$4"; }
pub controller c0nt building/controller/state own-state
pub controller c0nt building/other/state spoofed-state
pub controller c0nt building/controller/command forged-command
pub server s3rv building/controller/command real-command
wait

fail=0
expect() { # description, file, pattern, "yes"|"no"
  local seen=no
  x grep -q "$3" "$2" && seen=yes
  if [ "$seen" = "$4" ]; then echo "  ✓ $1"; else echo "  ✗ $1" >&2; fail=1; fi
}
expect "a device's own state is delivered"            /tmp/server.out "own-state"      yes
expect "another device's state cannot be written"     /tmp/server.out "spoofed-state"  no
expect "a device cannot forge its own command"        /tmp/device.out "forged-command" no
expect "the server's command reaches the device"      /tmp/device.out "real-command"   yes

if x mosquitto_pub -h 127.0.0.1 -u controller -P wrong -t building/controller/state -m x >/dev/null 2>&1; then
  echo "  ✗ a wrong password was accepted" >&2; fail=1
else echo "  ✓ a wrong password is refused"; fi
if x mosquitto_pub -h localhost -p 8883 --cafile /mosquitto/certs/ca.crt -u controller -P c0nt \
     -t building/controller/state -m tls >/dev/null 2>&1; then echo "  ✓ TLS with the CA is accepted"
else echo "  ✗ TLS with the CA was refused" >&2; fail=1; fi
if x timeout 5 mosquitto_pub -h localhost -p 8883 -u controller -P c0nt -t building/controller/state -m plain >/dev/null 2>&1; then
  echo "  ✗ plaintext was accepted on the TLS port" >&2; fail=1
else echo "  ✓ plaintext on the TLS port is refused"; fi

[ "$fail" = 0 ] || { echo "compose: FAILED" >&2; exit 1; }
echo "compose: ok"
