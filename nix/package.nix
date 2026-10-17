{
  lib,
  rustPlatform,
  pkg-config,
  wrapGAppsHook4,
  installShellFiles,
  gtk4,
}:
let
  manifest = (lib.importTOML ../Cargo.toml).package;
in
rustPlatform.buildRustPackage {
  pname = manifest.name;
  inherit (manifest) version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../data
      ../spec
    ];
  };
  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [
    pkg-config
    wrapGAppsHook4
    installShellFiles
  ];
  buildInputs = [ gtk4 ];

  postInstall = ''
    install -Dm644 data/io.github.DisposableSoftware.JunkDrawer.desktop -t $out/share/applications
    install -Dm644 data/io.github.DisposableSoftware.JunkDrawer.svg -t $out/share/icons/hicolor/scalable/apps
    install -Dm644 data/config.example.ini -t $out/share/doc/junk-drawer
    installManPage data/junk.1
    # Not in share/dbus-1/services: Nautilus ships the same name, so the modules
    # link it there only when asked (INT-25).
    mkdir -p $out/share/junk-drawer
    substitute data/org.freedesktop.FileManager1.service \
      $out/share/junk-drawer/org.freedesktop.FileManager1.service \
      --replace-fail @bin@ $out/bin/junk
  '';

  meta = {
    description = "Minimal GTK4 file manager";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
    mainProgram = "junk";
  };
}
