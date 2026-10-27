#!/bin/sh
# Builds Errol on the Raspberry Pi, in a Rust container (no toolchain on the
# host), and runs it as a service on the HDMI display, as the current user.
#   sh deploy/install.sh [friday|cyberpunk|hogwarts]
# Re-run after a `git pull` to rebuild; with a look, it also switches look.
# The first build takes about 10 minutes on a Pi 4, later ones a few. Once
# installed, deploy/push.sh from a Mac is faster: it builds there and only
# sends the binary.
#
# Optional, separate: EDID=1 sh deploy/install.sh also installs an EDID for the
# screen (make_edid.py) in /lib/firmware/edid. It does not touch the boot
# configuration: it prints the parameter to add to /boot/firmware/cmdline.txt,
# which takes effect at the next reboot.
set -e
cd "$(dirname "$0")/.."
# the build image keeps Rust and SDL2's headers: rebuilt only when the Dockerfile changes
docker build -q -t errol-build deploy >/dev/null
docker volume create errol-cargo >/dev/null
docker run --rm -v "$PWD:/src" -v errol-cargo:/usr/local/cargo/registry -w /src errol-build sh -c \
  "cargo build --release -p errol-app && chown -R $(id -u):$(id -g) target"
# the GL libraries too, marked as wanted: they often came with something else
# (a browser) and an apt autoremove would take them, leaving "EGL not initialized"
libs="libsdl2-2.0-0 libgles2 libegl1 libegl-mesa0 libgbm1"
# shellcheck disable=SC2086
if ! dpkg -s $libs >/dev/null 2>&1; then
  sudo apt-get install -y -qq $libs >/dev/null
fi
# shellcheck disable=SC2086
sudo apt-mark manual $libs >/dev/null
sudo install -m 755 target/release/errol /usr/local/bin/errol
if [ -n "$1" ]; then
  # keep the other settings, replace the look
  sudo touch /etc/default/errol
  sudo sed -i '/^ERROL_LOOK=/d' /etc/default/errol
  echo "ERROL_LOOK=$1" | sudo tee -a /etc/default/errol >/dev/null
fi
sed "s/@USER@/$(id -un)/" deploy/errol.service | sudo tee /etc/systemd/system/errol.service >/dev/null
sudo systemctl daemon-reload
sudo systemctl enable errol.service
sudo systemctl restart errol.service
echo "errol: $(systemctl is-active errol.service)"

if [ "${EDID:-0}" = 1 ]; then
  python3 deploy/make_edid.py > /tmp/errol-800x480.bin
  sudo install -D -m 644 /tmp/errol-800x480.bin /lib/firmware/edid/errol-800x480.bin
  echo "EDID installed in /lib/firmware/edid/errol-800x480.bin. To use it, add to the"
  echo "single line of /boot/firmware/cmdline.txt (then reboot):"
  echo "  drm.edid_firmware=HDMI-A-1:edid/errol-800x480.bin"
fi
