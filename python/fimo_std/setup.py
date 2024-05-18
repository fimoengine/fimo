import os
import pathlib
import platform

from setuptools import setup
from setuptools.command.build import build as _build


class build(_build):
    def run(self):
        super().run()
        self.build_ffi()

    def build_ffi(self):
        cwd = pathlib.Path().absolute()
        ffi_path = pathlib.Path.joinpath(cwd, 'ffi')

        # these dirs will be created in build_py, so if you don't have
        # any python sources to bundle, the dirs will be missing
        build_temp = pathlib.Path(self.build_temp).joinpath('cmake/fimo_std')
        build_temp.mkdir(parents=True, exist_ok=True)
        output_dir = pathlib.Path(self.build_temp).joinpath('cmake/install')

        # example of cmake args
        config = 'Debug' if self.debug else 'Release'
        cmake_args = [
            '-DCMAKE_INSTALL_PREFIX=' + str(output_dir.absolute()),
            '-DCMAKE_BUILD_TYPE=' + config,
            '-DFIMO_INSTALL_BINDINGS:BOOL=ON'
        ]

        # example of build args
        build_args = [
            '--target', 'install',
            '--config', config,
        ]

        os.chdir(str(build_temp))
        self.spawn(['cmake', str(ffi_path)] + cmake_args)
        if not self.dry_run:
            self.spawn(['cmake', '--build', '.'] + build_args)
        # Troubleshooting: if fail on line above then delete all possible
        # temporary CMake files including "CMakeCache.txt" in top level dir.
        os.chdir(str(cwd))

        if platform.system() == 'Windows':
            fimo_lib_dir = 'bin'
        else:
            fimo_lib_dir = 'lib'

        fimo_std_build_dir = pathlib.Path(output_dir).joinpath(fimo_lib_dir)
        fimo_std_install_dir = pathlib.Path(
            self.build_lib).joinpath('fimo_std/ffi')

        if platform.system() == "Linux":
            fimo_lib_name = "libfimo_std_shared.so"
        elif platform.system() == "Darwin":
            fimo_lib_name = "libfimo_std_shared.dylib"
        elif platform.system() == "Windows":
            fimo_lib_name = "fimo_std_shared.dll"
        else:
            raise RuntimeError("Unsupported platform")

        fimo_std_build_file = fimo_std_build_dir.joinpath(fimo_lib_name)
        fimo_std_install_file = fimo_std_install_dir.joinpath(fimo_lib_name)
        self.copy_file(fimo_std_build_file.__str__(),
                       fimo_std_install_file.__str__())


setup(
    name='fimo_std',
    version='0.0.1',
    description='Bindings to the fimo_std library',
    license='MIT OR Apache-2.0',
    where=['src'],
    install_requires=[],
    cmdclass={'build': build}
)
