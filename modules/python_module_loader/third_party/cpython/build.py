import os
import shutil
import argparse
import platform
import sysconfig
import subprocess


def dir_path(string):
    if os.path.isdir(string):
        return string
    else:
        raise NotADirectoryError(string)


def execute(cmd):
    cmd = ' '.join(cmd)
    popen = subprocess.Popen(cmd,
                             shell=True,
                             stdout=subprocess.PIPE,
                             text=True,
                             cwd=os.getcwd())
    for proc_line in iter(popen.stdout.readline, ""):
        yield proc_line
    popen.stdout.close()
    return_code = popen.wait()
    if return_code:
        raise subprocess.CalledProcessError(return_code, cmd)


def log_copy(prefix, path, names):
    print(f'{prefix}{path}')
    return []


parser = argparse.ArgumentParser()
parser.add_argument('--binary_dir',
                    type=dir_path,
                    required=True,
                    help='directory that will contain the build artifacts')
parser.add_argument('--release',
                    action='store_const',
                    const=True,
                    default=False,
                    help='make a release configuration')
args = parser.parse_args()
source_dir = os.getcwd()
binary_dir = args.binary_dir
build_dir = os.path.join(binary_dir, 'build')
build_release = args.release

print('Building CPython.')
print(f'\tMode: {"Release" if build_release else "Debug"}')
print(f'\tCPython directory: {source_dir}')
print(f'\tBuild directory: {binary_dir}')
print()

for old in os.listdir(build_dir):
    old = os.path.join(build_dir, old)
    if os.path.isfile(old) or os.path.islink(old):
        os.remove(old)
    elif os.path.isdir(old):
        shutil.rmtree(old)
    else:
        raise ValueError(f'file {old} is not a file or dir.')

print("\tStarting build.")
if os.name == 'nt':
    build_commands = [os.path.join(source_dir, 'PCbuild/build.bat'), '-e']
    if build_release:
        build_commands.append('-c')
        build_commands.append('Release')
    else:
        build_commands.append('-c')
        build_commands.append('Debug')

    machine = platform.machine().lower()
    if machine == 'x86_64' or machine == 'amd64':
        build_commands.append('-p')
        build_commands.append('x64')
        cpython_build_dir = os.path.join(source_dir, 'PCbuild/amd64')
    elif machine == 'arm64':
        build_commands.append('-p')
        build_commands.append('ARM64')
        cpython_build_dir = os.path.join(source_dir, 'PCbuild/arm64')
    else:
        raise ValueError(f'unknown machine type: {machine}')
    for line in execute(build_commands):
        print(f'\t\t{line}', end='')
else:
    cpython_build_dir = source_dir
    for line in execute(['make', '-j', str(os.cpu_count())]):
        print(f'\t\t{line}', end='')
print()

print('\tCopying files.')
if os.name == 'nt':
    include_dir = os.path.join(binary_dir, 'include/cpython')
    lib_dir = os.path.join(build_dir, 'Lib')
    dll_dir = os.path.join(build_dir, 'DLLs')
    os.mkdir(dll_dir)

    shutil.copytree(
        os.path.join(source_dir, 'Lib'),
        lib_dir,
        ignore=lambda path, names: log_copy('\t\tCopying file: ', path, names))

    for file in os.listdir(cpython_build_dir):
        source = os.path.join(cpython_build_dir, file)
        if not os.path.isfile(source):
            continue
        _, extension = os.path.splitext(file)
        if extension not in ['.dll', '.lib', '.pyd', '.h']:
            continue

        print(f'\t\tCopying file: {source}')
        if (extension == '.dll' or extension == '.lib') and 'python' in file:
            target = os.path.join(build_dir, file)
        elif extension == '.h':
            target = os.path.join(include_dir, file)
        else:
            target = os.path.join(dll_dir, file)
        shutil.copy(source, target)
else:
    src_build_dir = os.path.join(source_dir, 'build')
    src_modules_dir = os.path.join(source_dir, 'Modules')

    lib_dir = os.path.join(build_dir, 'Lib')
    so_dir = os.path.join(lib_dir, 'lib-dynload')
    so_suffix = sysconfig.get_config_var('SHLIB_SUFFIX')

    shutil.copytree(
        os.path.join(source_dir, 'Lib'),
        lib_dir,
        ignore=lambda path, names: log_copy('\t\tCopying file: ', path, names))
    os.mkdir(so_dir)

    for file in os.listdir(source_dir):
        source = os.path.join(source_dir, file)
        if so_suffix in file:
            print(f'\t\tCopying file: {source}')
            shutil.copy(source, os.path.join(build_dir, file))

    for dir_name in os.listdir(src_build_dir):
        dir_path = os.path.join(src_build_dir, dir_name)
        if not os.path.isdir(dir_path) or not dir_name.startswith('lib.'):
            continue
        for file in os.listdir(dir_path):
            source = os.path.join(dir_path, file)
            if not os.path.isfile(source):
                continue
            print(f'\t\tCopying file: {source}')
            shutil.copy(source, os.path.join(so_dir, file), follow_symlinks=True)
