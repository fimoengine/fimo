#!/usr/bin/env python

import os
import click
import shutil
import subprocess

fimo_dir = os.path.dirname(os.path.abspath(__file__))
manifest_path = os.path.join(fimo_dir, "Cargo.toml")
target_dir = os.path.join(fimo_dir, "fimo")
out_dir = os.path.join(target_dir, "bin")

home_dir = os.path.expanduser("~")
install_dir = os.path.join(home_dir, ".fimo")
install_bin_dir = os.path.join(install_dir, "bin")


@click.group()
def cli():
    pass


@click.command()
def bootstrap():
    # build binaries of the engine
    subprocess.run(["cargo", "build", "--bins", "--release", "--manifest-path",
                   manifest_path, "--out-dir", out_dir, "-Z", "unstable-options"], check=True)


@click.command()
@click.pass_context
def install(ctx):
    ctx.invoke(bootstrap)

    if not os.path.exists(install_dir):
        os.mkdir(install_dir)

    if os.path.exists(install_bin_dir):
        shutil.rmtree(install_bin_dir)

    shutil.copytree(out_dir, install_bin_dir)


@click.command()
def clean():
    shutil.rmtree(target_dir)


cli.add_command(bootstrap)
cli.add_command(install)
cli.add_command(clean)

if __name__ == '__main__':
    cli()
