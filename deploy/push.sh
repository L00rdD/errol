#!/bin/sh
# Builds Errol on this machine and sends it to the Pi: no toolchain, no git
# pull and no compiling there. Needs Docker able to run linux/arm64 natively
# (a Mac with Apple silicon, colima or Docker Desktop) and ssh to the Pi.
#   sh deploy/push.sh [friday|cyberpunk|hogwarts]
# The Pi must have been set up once with install.sh (service, GL libraries).
# ERROL_HOST picks the Pi (default dpi.local).
set -e
cd "$(dirname "$0")/.."
host=${ERROL_HOST:-dpi.local}
docker build -q --platform linux/arm64 -t errol-build deploy >/dev/null
# the registry and the target dir stay in volumes: the next build is incremental
docker volume create errol-cargo >/dev/null
docker volume create errol-target >/dev/null
mkdir -p target/pi
docker run --rm --platform linux/arm64 \
  -v "$PWD:/src:ro" -v errol-cargo:/usr/local/cargo/registry \
  -v errol-target:/target -v "$PWD/target/pi:/out" -w /src \
  -e CARGO_TARGET_DIR=/target errol-build sh -c \
  "cargo build --release -p errol-app && cp /target/release/errol /out/errol"
scp -q target/pi/errol "$host:/tmp/errol"
ssh "$host" sh -s "$1" <<'REMOTE'
set -e
sudo install -m 755 /tmp/errol /usr/local/bin/errol
rm /tmp/errol
if [ -n "$1" ]; then
  sudo touch /etc/default/errol
  sudo sed -i '/^ERROL_LOOK=/d' /etc/default/errol
  echo "ERROL_LOOK=$1" | sudo tee -a /etc/default/errol >/dev/null
fi
sudo systemctl restart errol.service
echo "errol: $(systemctl is-active errol.service)"
REMOTE
