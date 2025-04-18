#!/usr/bin/env python
import re
import os
import argparse
from pathlib import Path

def update_rust_version(path, major, minor, patch, pre, build):
    version = f"{major}.{minor}.{patch}"
    if pre is not None:
        version = f"{version}-{pre}"
    if build is not None:
        version = f"{version}+{build}"

    with open(path, 'r') as f:
        content = f.read()
        content_new = re.sub('^version = ".*"', f'version = "{version}"', content, flags = re.M)

    with open(path, 'w') as f:
        f.write(content_new)

def update_zig_version(path, major, minor, patch, pre, build):
    version = f"{major}.{minor}.{patch}"
    if pre is not None:
        version = f"{version}-{pre}"
    if build is not None:
        version = f"{version}+{build}"

    with open(path, 'r') as f:
        content = f.read()
        content_new = re.sub('\.version = ".*"', f'.version = "{version}"', content, flags = re.M)

    with open(path, 'w') as f:
        f.write(content_new)

def fimo_version(path, major, minor, patch, pre, build):
    version = f'.major = {major}, .minor = {minor}, .patch = {patch}'
    if pre is not None:
        version = f'{version}, .pre = "{pre}"'
    if build is not None:
        version = f'{version}, .build = "{build}"'

    with open(path, 'r') as f:
        content = f.read()
        pattern = "^pub const fimo_version: std.SemanticVersion = .{.*};"
        replacement = f"pub const fimo_version: std.SemanticVersion = .{{ {version} }};"
        content_new = re.sub(pattern, replacement, content, flags = re.M)

    with open(path, 'w') as f:
        f.write(content_new)

def update_version_recursive(path, major, minor, patch, pre, build):
    for child in path.iterdir():
        if child.name == '.zig-cache':
            continue
        if child.is_dir():
            update_version_recursive(child, major, minor, patch, pre, build)
        if child.name == 'build.zig.zon':
            update_zig_version(child, major, minor, patch, pre, build)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
                        prog='update-version',
                        description='Updates the version of the workspace')
    parser.add_argument('version')
    args = parser.parse_args()

    version = args.version
    regex = r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
    m = re.match(regex, version)

    major = m.group(1)
    minor = m.group(2)
    patch = m.group(3)
    pre = m.group(4)
    build = m.group(5)

    root_path = Path(os.path.realpath(__file__)).parent.parent

    update_rust_version(root_path.joinpath("Cargo.toml"), major, minor, patch, pre, build)
    update_zig_version(root_path.joinpath("build.zig.zon"), major, minor, patch, pre, build)
    fimo_version(root_path.joinpath("tools/build-internals/build.zig"), major, minor, patch, pre, build)
    update_version_recursive(root_path.joinpath("pkgs"), major, minor, patch, pre, build)
    update_version_recursive(root_path.joinpath("modules"), major, minor, patch, pre, build)
    update_version_recursive(root_path.joinpath("rust"), major, minor, patch, pre, build)
    update_version_recursive(root_path.joinpath("src"), major, minor, patch, pre, build)
