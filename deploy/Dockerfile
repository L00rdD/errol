# The image Errol is built in: Rust and SDL2's headers on Debian trixie, the
# Pi's own system, so the binary links against the Pi's libraries. Built once
# and kept; push.sh (on a Mac) and install.sh (on the Pi) both use it.
FROM rust:slim-trixie
RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends libsdl2-dev pkg-config >/dev/null \
 && rm -rf /var/lib/apt/lists/*
