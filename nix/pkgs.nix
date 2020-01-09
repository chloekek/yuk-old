let
    tarball = fetchTarball {
        url = "https://github.com/NixOS/nixpkgs/archive/7d90e34e9f15fc668eba35f7609f99b6e73b14cc.tar.gz";
        sha256 = "1jsvjqd3yp30y12wvkb6k42mpk8gfgnr8y9j995fpasjg1jymy9f";
    };
    config = {
    };
in
    {}: import tarball {inherit config;}
