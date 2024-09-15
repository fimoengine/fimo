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
        ffi_path = pathlib.Path.joinpath(cwd, "ffi")

        # guess the target triple
        match platform.machine():
            case "x86_64" | "amd64" | "AMD64":
                target_machine = "x86_64"
            case "aarch64" | "arm64":
                target_machine = "aarch64"
            case _:
                raise RuntimeError("Unsupported architecture")

        match platform.system():
            case "Windows":
                target_os = "windows"
                target_abi = "msvc"
            case "Linux":
                target_os = "linux"
                target_abi = "gnu"
            case "Darwin":
                target_os = "macos"
                target_abi = ""
            case _:
                raise RuntimeError("Unsupported platform")

        if len(target_abi) == 0:
            target_triple = f"{target_machine}-{target_os}"
        else:
            target_triple = f"{target_machine}-{target_os}-{target_abi}"

        # configure zig build
        zig_cache_dir = pathlib.Path(self.build_temp).joinpath(".zig-cache")
        zig_out_dir = pathlib.Path(self.build_temp).joinpath("zig-out")

        zig_args = [
            "build",
            f"-Dtarget={target_triple}",
            f"-Dcpu={target_machine}",
            "--prefix",
            str(zig_out_dir.absolute()),
            "--cache-dir",
            str(zig_cache_dir.absolute()),
        ]
        if not self.debug:
            zig_args.append("--release=safe")

        if not self.dry_run:
            os.chdir(str(ffi_path))
            self.spawn(["zig"] + zig_args)
            os.chdir(str(cwd))

        if platform.system() == "Windows":
            fimo_lib_dir = "bin"
        else:
            fimo_lib_dir = "lib"

        fimo_std_build_dir = pathlib.Path(zig_out_dir).joinpath(fimo_lib_dir)
        fimo_std_install_dir = pathlib.Path(self.build_lib).joinpath("fimo_std/ffi")

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
        self.copy_file(fimo_std_build_file.__str__(), fimo_std_install_file.__str__())


setup(
    name="fimo_std",
    version="0.0.1",
    description="Bindings to the fimo_std library",
    license="MIT OR Apache-2.0",
    where=["src"],
    install_requires=[],
    cmdclass={"build": build},
    include_package_data=True,
)
