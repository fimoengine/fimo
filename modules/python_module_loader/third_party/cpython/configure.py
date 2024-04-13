import os
import sys
import shutil
import argparse
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
build_release = args.release

print('Configuring CPython.')
print(f'\tMode: {"Release" if build_release else "Debug"}')
print(f'\tCPython directory: {source_dir}')
print(f'\tBuild directory: {binary_dir}')
print()

print('\tRemoving old build files.')
old_build_files = os.listdir(binary_dir)
if len(old_build_files) == 0:
    print('\t\tNo old build files present.')
for old in old_build_files:
    old = os.path.join(binary_dir, old)
    print(f'\t\tRemoving: {old}')
    if os.path.isfile(old) or os.path.islink(old):
        os.remove(old)
    elif os.path.isdir(old):
        shutil.rmtree(old)
    else:
        raise ValueError(f'file {old} is not a file or dir.')
print()

print('\tGenerating configuration.')
if os.name == 'nt':
    print('\t\tFetching external packages.')
    for line in execute([os.path.join(source_dir, 'PCbuild/get_externals.bat')]):
        print(f'\t\t\t{line}', end='')
else:
    configure_args = [
        './configure',
        '--enable-shared',
        '--without-static-libpython'
    ]
    if build_release:
        configure_args.append('--enable-optimizations')
        configure_args.append('--with-lto')
    else:
        configure_args.append('--with-pydebug')
    if sys.platform.startswith('darwin'):
        configure_args.append('--enable-universalsdk')
        configure_args.append('--with-universal-archs=universal2')
    print(f'\t\tRunning: {" ".join(configure_args)}')
    for line in execute(configure_args):
        print(f'\t\t\t{line}', end='')
print()

print('\tCopying include directory.')
include_dir = os.path.join(binary_dir, 'include/cpython')
shutil.copytree(
    os.path.join(source_dir, 'Include'),
    include_dir,
    ignore=lambda path, names: log_copy('\t\tCopying file: ', path, names))
if not os.name == 'nt':
    pyconfig_path = os.path.join(source_dir, 'pyconfig.h')
    print(f'\t\tCopying file: {pyconfig_path}')
    shutil.copy(pyconfig_path, os.path.join(include_dir, 'pyconfig.h'))
print()

print('\tCreating build dir.')
os.mkdir(os.path.join(binary_dir, 'build'))
