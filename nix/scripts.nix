# Mission-control scripts for shared-kernel-rs.
#
# Mirrors the pattern from nammayatri/Backend/nix/scripts.nix, adapted for
# a Rust workspace. We use https://github.com/Platonic-Systems/mission-control
# (provided transitively by the `common` flake input). Inside `nix develop`
# these are invokable as `, <name>`.
{ inputs, ... }:
{
  perSystem = { config, pkgs, system, lib, ... }:
    {
      mission-control.scripts = {
        run-generator = {
          category = "Generator";
          description = "Run namma-dsl-rs to (re)generate storage code from spec/Storage/*.yaml.";
          exec = ''
            # --skip-update is a wrapper-script flag (controls freshness
            # check); strip it before forwarding to the generator binary.
            skip_update=false
            generator_args=()
            for arg in "$@"; do
              if [[ "$arg" == "--skip-update" ]]; then
                skip_update=true
              else
                generator_args+=("$arg")
              fi
            done
            current_commit_hash=$( ${pkgs.jq}/bin/jq -r '.nodes."namma-dsl-rs".locked.rev' "''${FLAKE_ROOT}/flake.lock" || true)
            # Skip update check if using local path (current_commit_hash will be null)
            if [[ "$current_commit_hash" == "null" || -z "$current_commit_hash" ]];
            then
              echo -e "\033[32mUsing local namma-dsl-rs path, skipping update check"
            else
              latest_commit_hash=$(${pkgs.curl}/bin/curl -s "https://api.github.com/repos/nammayatri/namma-dsl-rs/commits/main" | ${pkgs.jq}/bin/jq -r '.sha' || true)
              if [[ -z $latest_commit_hash ]];
              then
                echo -e "\033[33mNot able to get status of namma-dsl-rs"
              else
                if [[ "$current_commit_hash" != "$latest_commit_hash" ]]; then
                    echo -e "\033[33mnamma-dsl-rs is not up to date !!\nCurrent commit hash: $current_commit_hash\nLatest commit hash: $latest_commit_hash"
                    if [[ $skip_update == false ]]; then
                        echo -e "\033[33mUpdating namma-dsl-rs to latest commit";
                        nix flake lock --update-input namma-dsl-rs;
                        echo -e "\033[32mnamma-dsl-rs updated to latest commit\nPlease run nix develop again to use the updated version"
                        echo -e "\033[00m";
                        exit 0
                    fi
                else
                    echo -e "\033[32mnamma-dsl-rs is up to date";
                fi
              fi
            fi
            echo -e "\033[00m";
            set -x
            cd "''${FLAKE_ROOT}"
            ${inputs.namma-dsl-rs.packages.${system}.default}/bin/namma-dsl-rs "''${generator_args[@]}"
          '';
        };

        fmt = {
          category = "Format";
          description = "Run cargo fmt on the whole workspace.";
          exec = ''
            cd "''${FLAKE_ROOT}"
            cargo fmt --all "$@"
          '';
        };

        lint = {
          category = "Lint";
          description = "Run cargo clippy across all targets.";
          exec = ''
            cd "''${FLAKE_ROOT}"
            cargo clippy --all-targets "$@"
          '';
        };

        hpack = {
          category = "Compat";
          description = "No-op stub. Cargo handles project files; kept for muscle-memory parity with Haskell repos.";
          exec = ''
            echo "shared-kernel-rs is a Cargo workspace; there is no hpack step."
          '';
        };
      };
    };
}
