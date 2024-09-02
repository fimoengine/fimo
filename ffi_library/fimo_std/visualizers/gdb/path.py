import gdb.printing


class FimoUTF8PathBufPrinter(gdb.ValuePrinter):
    """Print a FimoUTF8PathBuf struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        return [
            ("path", self.__val["buffer"]["elements"].cast(gdb.lookup_type("char*"))),
            ("length", self.__val["buffer"]["size"]),
            ("capacity", self.__val["buffer"]["capacity"]),
        ]

    def to_string(self):
        if int(self.__val["buffer"]["elements"]) == 0:
            return ""

        try:
            length = int(self.__val["buffer"]["size"])
            path = (
                self.__val["buffer"]["elements"]
                .cast(gdb.lookup_type("char*"))
                .string(encoding="utf-8", length=length)
            )
            return f'"{path}"'
        except:
            return "invalid path"

    def display_hint(self):
        return "string"


class FimoOwnedUTF8PathPrinter(gdb.ValuePrinter):
    """Print a FimoOwnedUTF8Path struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        return [
            ("path", self.__val["path"]),
            ("length", self.__val["length"]),
        ]

    def to_string(self):
        if int(self.__val["path"]) == 0:
            return "invalid path"

        try:
            length = int(self.__val["length"])
            path = self.__val["path"].string(encoding="utf-8", length=length)
            return f'"{path}"'
        except:
            return "invalid path"

    def display_hint(self):
        return "string"


class FimoUTF8PathPrinter(gdb.ValuePrinter):
    """Print a FimoUTF8Path struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        return [
            ("path", self.__val["path"]),
            ("length", self.__val["length"]),
        ]

    def to_string(self):
        if int(self.__val["path"]) == 0:
            return "invalid path"

        try:
            length = int(self.__val["length"])
            path = self.__val["path"].string(encoding="utf-8", length=length)
            return f'"{path}"'
        except:
            return "invalid path"

    def display_hint(self):
        return "string"


class FimoOwnedOSPathPrinter(gdb.ValuePrinter):
    """Print a FimoOSPath struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        return [
            ("path", self.__val["path"]),
            ("length", self.__val["length"]),
        ]

    def to_string(self):
        if int(self.__val["path"]) == 0:
            return "invalid path"

        try:
            length = int(self.__val["length"])
            path = self.__val["path"].string(encoding="utf-8", length=length)
            return f'"{path}"'
        except:
            return "invalid path"

    def display_hint(self):
        return "string"


class FimoOSPathPrinter(gdb.ValuePrinter):
    """Print a FimoOSPath struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        return [
            ("path", self.__val["path"]),
            ("length", self.__val["length"]),
        ]

    def to_string(self):
        if int(self.__val["path"]) == 0:
            return "invalid path"

        try:
            length = int(self.__val["length"])
            path = self.__val["path"].string(encoding="utf-8", length=length)
            return f'"{path}"'
        except:
            return "invalid path"

    def display_hint(self):
        return "string"


class FimoUTF8PathPrefixPrinter(gdb.ValuePrinter):
    """Print a FimoUTF8PathPrefix struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        variant = int(self.__val["type"])
        if variant == 0:
            return [("variant", self.__val["data"]["verbatim"])]
        elif variant == 1:
            return [("variant", self.__val["data"]["verbatim_unc"])]
        elif variant == 2:
            return [("variant", self.__val["data"]["verbatim_disk"])]
        elif variant == 3:
            return [("variant", self.__val["data"]["device_ns"])]
        elif variant == 4:
            return [("variant", self.__val["data"]["unc"])]
        elif variant == 5:
            return [("variant", self.__val["data"]["disk"])]
        else:
            return []

    def to_string(self):
        variant = int(self.__val["type"])
        if variant == 0:
            return self.__val["data"]["verbatim"]
        elif variant == 1:
            return self.__val["data"]["verbatim_unc"]
        elif variant == 2:
            return self.__val["data"]["verbatim_disk"]
        elif variant == 3:
            return self.__val["data"]["device_ns"]
        elif variant == 4:
            return self.__val["data"]["unc"]
        elif variant == 5:
            return self.__val["data"]["disk"]
        else:
            return "invalid prefix type"

    def display_hint(self):
        return "string"


class FimoUTF8PathComponentPrinter(gdb.ValuePrinter):
    """Print a FimoUTF8PathComponent struct"""

    def __init__(self, val):
        self.__val = val

    def children(self):
        variant = int(self.__val["type"])
        if variant == 0:
            return [("variant", self.__val["data"]["prefix"])]
        elif variant == 1:
            return [("variant", self.__val["data"]["root_dir"])]
        elif variant == 2:
            return [("variant", self.__val["data"]["cur_dir"])]
        elif variant == 3:
            return [("variant", self.__val["data"]["parent_dir"])]
        elif variant == 4:
            return [("variant", self.__val["data"]["normal"])]
        else:
            return []

    def to_string(self):
        variant = int(self.__val["type"])
        if variant == 0:
            return self.__val["data"]["prefix"]
        elif variant == 1:
            return "/"
        elif variant == 2:
            return "."
        elif variant == 3:
            return ".."
        elif variant == 4:
            return self.__val["data"]["normal"]
        else:
            return "invalid component type"

    def display_hint(self):
        return "string"


def build_pretty_printer():
    pp = gdb.printing.RegexpCollectionPrettyPrinter("fimo_std")
    pp.add_printer(
        "FimoUTF8PathBuf Printer", "^FimoUTF8PathBuf", FimoUTF8PathBufPrinter
    )
    pp.add_printer(
        "FimoOwnedUTF8Path Printer", "^FimoOwnedUTF8Path", FimoOwnedUTF8PathPrinter
    )
    pp.add_printer("FimoUTF8Path Printer", "^FimoUTF8Path$", FimoUTF8PathPrinter)
    pp.add_printer(
        "FimoOwnedOSPath Printer", "^FimoOwnedOSPath", FimoOwnedOSPathPrinter
    )
    pp.add_printer("FimoOSPath Printer", "^FimoOSPath", FimoOSPathPrinter)
    pp.add_printer(
        "FimoUTF8PathPrefix Printer", "^FimoUTF8PathPrefix", FimoUTF8PathPrefixPrinter
    )
    pp.add_printer(
        "FimoUTF8PathComponent Printer",
        "^FimoUTF8PathComponent",
        FimoUTF8PathComponentPrinter,
    )
    return pp


gdb.printing.register_pretty_printer(gdb.current_objfile(), build_pretty_printer())
