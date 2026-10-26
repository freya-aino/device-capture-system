{ pkgs, lib, config, inputs, ... }:

{
  # System libraries needed by the sys-crates in this workspace:
  #   - zmq-sys / networking      -> zeromq
  #   - alsa / devices            -> alsa-lib
  #   - v4l2-sys-mit / devices    -> libv4l, plus libclang for its bindgen build step
  packages = [
    pkgs.pkg-config
    pkgs.alsa-lib
    pkgs.zeromq
    pkgs.libv4l
  ];

  languages.rust.enable = true;

  # v4l2-sys-mit runs bindgen at build time, which needs a libclang matching the
  # toolchain in this shell. Pointing it elsewhere (e.g. a libclang found on the
  # host system outside this shell's Nix closure) can fail with a glibc symbol
  # mismatch, since it won't necessarily share this environment's glibc.
  env.LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";

  enterShell = ''
    echo "device-capture-system dev shell: $(rustc --version)"
  '';

  enterTest = ''
    cargo check --workspace --all-targets
    cargo test --workspace
  '';
}
