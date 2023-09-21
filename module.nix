{ config, lib, pkgs, inputs, ... }:

with lib;

let
  cfg = config.services.dns-agent;
  settingsFormat = pkgs.formats.toml { };
  configFile = settingsFormat.generate "config.toml" cfg.extraConfig;
in
{
  options = {
    services.dns-agent = {
      enable = mkEnableOption "dns-agent";

      package = mkOption {
        default = inputs.dns-agent.defaultPackage.${pkgs.system};
        defaultText = literalExpression "inputs.dns-agent.defaultPackage";
        type = types.package;
      };

      environmentFiles = mkOption {
        type = types.listOf types.path;
        default = [ ];
        example = [ "/run/keys/dns-agent.env" ];
        description = ''
          File to load as environment file. Environment variables from this file
          will be interpolated into the config file using envsubst with this
          syntax: <literal>$ENVIRONMENT</literal> or <literal>''${VARIABLE}</literal>.
          This is useful to avoid putting secrets into the nix store.
        '';
      };

      extraConfig = mkOption {
        default = { };
        description = "Extra configuration options";
        type = settingsFormat.type;
      };
    };
  };

  config = mkIf config.services.dns-agent.enable {
    systemd.services.dns-agent =
      let
        finalConfigFile =
          if config.services.dns-agent.environmentFiles == [ ]
          then configFile
          else "/var/run/dns-agent/config.toml";
      in
      {
        description = "dns-agent";
        serviceConfig = {
          EnvironmentFile = config.services.dns-agent.environmentFiles;
          Type = "simple";
          ExecStartPre = lib.optional (config.services.dns-agent.environmentFiles != [ ])
            (pkgs.writeShellScript "pre-start" ''
              umask 077
              ${pkgs.envsubst}/bin/envsubst -i "${configFile}" > /var/run/dns-agent/config.toml
            '');
          ExecStart = "${cfg.package}/bin/dns-agent -c ${finalConfigFile} -v";
          RuntimeDirectory = "dns-agent";
        };
      };
    systemd.timers.dns-agent = {
      wantedBy = [ "timers.target" ];
      partOf = [ "dns-agent.service" ];
      timerConfig = {
        OnCalendar = "0/1:00:00";
        Unit = "dns-agent.service";
      };
    };
  };
}
