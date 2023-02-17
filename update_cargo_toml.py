#!/usr/bin/env python3
import sys
import toml

if len(sys.argv) != 2:
    print(sys.argv)
    sys.exit("must pass 1 positional argument, ex: python3 ./update_cargo_toml.py 1.0.1")


def update_cargo_toml_version(filename):
    toml_data = toml.load(filename)
    toml_data["package"]["version"] = sys.argv[1]

    with open(filename, 'w') as cargo:
        toml.dump(toml_data, cargo)


update_cargo_toml_version("momento-cli-opts/Cargo.toml")
update_cargo_toml_version("momento/Cargo.toml")
