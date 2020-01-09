{
    pkgs ? import ./nix/pkgs.nix {}
}:
[
    pkgs.cargo
    pkgs.gcc
]
