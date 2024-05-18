#!/usr/bin/env python3

import argparse
import pathlib
import shutil
import subprocess
import tempfile
import venv


class EnvBuilder(venv.EnvBuilder):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.context = None

    def post_setup(self, context):
        self.context = context


def execute_subprocess(args) -> None:
    with subprocess.Popen(args, stdout=subprocess.PIPE, text=True) as p:
        for line in p.stdout:
            print(f'\t{line}', end='')

    if p.returncode != 0:
        raise subprocess.CalledProcessError(p.returncode, p.args)


def build(src_dir, out_dir, **kwargs):
    if src_dir is None:
        src_dir = pathlib.Path.cwd()
    else:
        src_dir = pathlib.Path(src_dir)

    if out_dir is None:
        out_dir = pathlib.Path.cwd()
    else:
        out_dir = pathlib.Path(out_dir)

    with tempfile.TemporaryDirectory() as target_dir_path:
        print(f" *** Created temporary directory '{target_dir_path}'.")
        print(f" *** Creating virtual environment...")
        venv_builder = EnvBuilder(with_pip=True)
        venv_builder.create(str(target_dir_path))
        venv_context = venv_builder.context
        #
        requirements = [
            'build',
        ]
        print(f" *** Installing {requirements}...")
        pip_install_command = [
            venv_context.env_exe,
            '-m',
            'pip',
            'install',
            *requirements,
        ]
        execute_subprocess(pip_install_command)
        #
        print(" *** Building package...")
        dist_dir = pathlib.Path(target_dir_path).joinpath('dist')
        build_command = [
            venv_context.env_exe,
            '-m',
            'build',
            '--outdir',
            str(dist_dir),
            str(src_dir)
        ]
        execute_subprocess(build_command)
        #
        out_dir.mkdir(parents=True, exist_ok=True)
        for wheel_src in dist_dir.glob('*.whl'):
            wheel_dst = out_dir.joinpath(wheel_src.relative_to(dist_dir))
            print(f" *** Copying wheel '{str(wheel_dst)}'.")
            shutil.copy(wheel_src, wheel_dst)


def test(package, dist_dir, dependencies, **kwargs):
    if dist_dir is None:
        dist_dir = pathlib.Path.cwd()
    else:
        dist_dir = pathlib.Path(dist_dir)

    if dependencies is None:
        dependencies = []

    with tempfile.TemporaryDirectory() as target_dir_path:
        print(f" *** Created temporary directory '{target_dir_path}'.")
        print(f" *** Creating virtual environment...")
        venv_builder = EnvBuilder(with_pip=True)
        venv_builder.create(str(target_dir_path))
        venv_context = venv_builder.context
        #
        print(f" *** Installing package {package}...")
        pip_package_install_command = [
            venv_context.env_exe,
            '-m',
            'pip',
            'install',
            '--find-links',
            dist_dir,
            package,
        ]
        execute_subprocess(pip_package_install_command)
        #
        requirements = [
            'pytest',
            *dependencies
        ]
        print(f" *** Installing {requirements}...")
        pip_install_command = [
            venv_context.env_exe,
            '-m',
            'pip',
            'install',
            *requirements,
        ]
        execute_subprocess(pip_install_command)
        #
        print(f" *** Running tests for {package}...")
        pip_test_command = [
            venv_context.env_exe,
            '-m',
            'pytest',
            '-s',
            '--pyargs',
            package
        ]
        execute_subprocess(pip_test_command)


global_parser = argparse.ArgumentParser()
subparsers = global_parser.add_subparsers(
    title='operations', help='operations on python packages'
)

build_parser = subparsers.add_parser('build', help='builds a package wheel')
build_parser.add_argument('--src-dir',
                          type=str,
                          required=False,
                          help='Source directory (defaults to current directory).')
build_parser.add_argument('--out-dir',
                          type=str,
                          required=False,
                          help='Copy final artifacts to this directory (defaults to current directory).')
build_parser.set_defaults(func=build)

test_parser = subparsers.add_parser('test', help='tests a package')
test_parser.add_argument('--package',
                         type=str,
                         required=True,
                         help='Runs the tests in a package.')
test_parser.add_argument('--dist-dir',
                         type=str,
                         required=False,
                         help='Binary distribution directory (defaults to current directory).')
test_parser.add_argument('-d',
                         '--dependencies',
                         action='append',
                         required=False,
                         help='Additional test dependencies.')
test_parser.set_defaults(func=test)

if __name__ == '__main__':
    parsed_args = global_parser.parse_args()
    parsed_args.func(**vars(parsed_args))
