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
  ini = pkgs.formats.ini { listToValue = lib.concatMapStringsSep ";" toString; };
in
{
  options.programs.junk-drawer = {
    enable = lib.mkEnableOption "Junk Drawer, a minimal GTK4 file manager";
    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "junk-drawer.packages.\${system}.default";
    };
    settings = lib.mkOption {
      inherit (ini) type;
      default = { };
      example = {
        appearance = {
          flavor = "mocha";
          accent = "mauve";
          opacity = 0.85;
        };
        view.default = "details";
        keybindings.go-up = [
          "<Alt>Up"
          "BackSpace"
        ];
      };
      description = "Written to $XDG_CONFIG_HOME/junk-drawer/config.ini.";
    };
    style = lib.mkOption {
      type = lib.types.lines;
      default = "";
      description = "Extra GTK CSS, written to $XDG_CONFIG_HOME/junk-drawer/style.css. Catppuccin names (@base, @mauve, @accent…) are in scope.";
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
    home.packages = [ cfg.package ] ++ lib.optional cfg.fileManager1 fileManager1;
    xdg.configFile."junk-drawer/config.ini" = lib.mkIf (cfg.settings != { }) {
      source = ini.generate "junk-drawer-config.ini" cfg.settings;
    };
    xdg.configFile."junk-drawer/style.css" = lib.mkIf (cfg.style != "") { text = cfg.style; };
    xdg.mimeApps.defaultApplications = lib.mkIf cfg.defaultFileManager {
      "inode/directory" = [ "io.github.DisposableSoftware.JunkDrawer.desktop" ];
    };
  };
}
