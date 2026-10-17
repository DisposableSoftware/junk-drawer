self:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.junk-drawer;
  fileManager1 = lib.hiPrio (
    pkgs.runCommand "junk-drawer-filemanager1" { } ''
      install -Dm644 ${cfg.package}/share/junk-drawer/org.freedesktop.FileManager1.service \
        $out/share/dbus-1/services/org.freedesktop.FileManager1.service
    ''
  );
in
{
  options.programs.junk-drawer = {
    enable = lib.mkEnableOption "Junk Drawer, a minimal GTK4 file manager";
    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "junk-drawer.packages.\${system}.default";
    };
    defaultFileManager = lib.mkEnableOption "Junk Drawer as the default handler for inode/directory";
    fileManager1 = lib.mkOption {
      type = lib.types.bool;
      default = cfg.defaultFileManager;
      defaultText = lib.literalExpression "config.programs.junk-drawer.defaultFileManager";
      description = "Install Junk Drawer's D-Bus activation file for org.freedesktop.FileManager1 (\"Show in folder\"), taking priority over Nautilus's.";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ] ++ lib.optional cfg.fileManager1 fileManager1;
    # GIO needs gvfs for trash/mounting and udisks2 for removable drives.
    services.gvfs.enable = lib.mkDefault true;
    services.udisks2.enable = lib.mkDefault true;
    xdg.mime.defaultApplications = lib.mkIf cfg.defaultFileManager {
      "inode/directory" = "io.github.DisposableSoftware.JunkDrawer.desktop";
    };
  };
}
