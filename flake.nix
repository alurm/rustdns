{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs { inherit system; };
  in {
    packages.default = pkgs.rustPlatform.buildRustPackage {
      name = "dns";
      src = ./.;
      cargoHash = "sha256-2v2TsuVICUXvwaUVrgXlSLRCSREmyW2QUTTHBaFTo5g=";
    };
    devShells.default = pkgs.mkShell {
      nativeBuildInputs = builtins.attrValues { inherit (pkgs) rustc cargo lldb dig; };
    };
  });
}
