import ctypes as c
from typing import Self
import pytest

from ..context import Context
from .. import tracing
from .. import module
from .. import version
from .. import error
from .. import ffi


class IntSymbol(ffi.FFITransferable[c.c_void_p]):
    def __init__(self, ptr: c.c_void_p):
        if not isinstance(ptr, c.c_void_p):
            raise TypeError()
        if not ptr:
            raise ValueError()

        self._ptr = c.cast(ptr, c.POINTER(c.c_int))

    @classmethod
    def transfer_from_ffi(cls, ptr: c.c_void_p) -> Self:
        return cls(ptr)

    def transfer_to_ffi(self) -> c.c_void_p:
        return c.cast(self._ptr, c.c_void_p)

    @property
    def value(self) -> int:
        return self._ptr.contents.value


a_export_0 = module.symbol(
    sym_type=IntSymbol,
    name="a_export_0",
    namespace=None,
    version=version.Version(0, 1, 0),
)

a_export_1 = module.symbol(
    sym_type=IntSymbol,
    name="a_export_1",
    namespace=None,
    version=version.Version(0, 1, 0),
)

b_export_0 = module.symbol(
    sym_type=IntSymbol,
    name="b_export_0",
    namespace="b",
    version=version.Version(0, 1, 0),
)

b_export_1 = module.symbol(
    sym_type=IntSymbol,
    name="b_export_1",
    namespace="b",
    version=version.Version(0, 1, 0),
)

module.export_module(
    name="a",
    description="Test module a",
    exports={
        "a0": module.module_static_symbol_export(
            symbol=a_export_0, obj=c.POINTER(c.c_int)(c.c_int(5))
        ),
        "a1": module.module_static_symbol_export(
            symbol=a_export_1, obj=c.POINTER(c.c_int)(c.c_int(10))
        ),
    },
)

module.export_module(
    name="b",
    description="Test module b",
    author="Fimo",
    exports={
        "b0": module.module_static_symbol_export(
            symbol=b_export_0, obj=c.POINTER(c.c_int)(c.c_int(-2))
        ),
        "b1": module.module_static_symbol_export(
            symbol=b_export_1, obj=c.POINTER(c.c_int)(c.c_int(77))
        ),
    },
)


def c_factory(mod: module.ModuleBase, _module_set: module.LoadingSetView, cls):
    parameters = mod.parameters()
    assert hasattr(parameters, "pub_pub")
    assert hasattr(parameters, "pub_dep")
    assert hasattr(parameters, "pub_pri")
    assert hasattr(parameters, "dep_pub")
    assert hasattr(parameters, "dep_dep")
    assert hasattr(parameters, "dep_pri")
    assert hasattr(parameters, "pri_pub")
    assert hasattr(parameters, "pri_dep")
    assert hasattr(parameters, "pri_pri")

    pub_pub = parameters.pub_pub.read(mod)
    pub_dep = parameters.pub_dep.read(mod)
    pub_pri = parameters.pub_pri.read(mod)
    dep_pub = parameters.dep_pub.read(mod)
    dep_dep = parameters.dep_dep.read(mod)
    dep_pri = parameters.dep_pri.read(mod)
    pri_pub = parameters.pri_pub.read(mod)
    pri_dep = parameters.pri_dep.read(mod)
    pri_pri = parameters.pri_pri.read(mod)
    assert isinstance(pub_pub, module.ParameterValue)
    assert isinstance(pub_dep, module.ParameterValue)
    assert isinstance(pub_pri, module.ParameterValue)
    assert isinstance(dep_pub, module.ParameterValue)
    assert isinstance(dep_dep, module.ParameterValue)
    assert isinstance(dep_pri, module.ParameterValue)
    assert isinstance(pri_pub, module.ParameterValue)
    assert isinstance(pri_dep, module.ParameterValue)
    assert isinstance(pri_pri, module.ParameterValue)
    assert pub_pub.type == module.ParameterType.U32 and pub_pub.value == 0
    assert pub_dep.type == module.ParameterType.U32 and pub_dep.value == 1
    assert pub_pri.type == module.ParameterType.U32 and pub_pri.value == 2
    assert dep_pub.type == module.ParameterType.U32 and dep_pub.value == 3
    assert dep_dep.type == module.ParameterType.U32 and dep_dep.value == 4
    assert dep_pri.type == module.ParameterType.U32 and dep_pri.value == 5
    assert pri_pub.type == module.ParameterType.U32 and pri_pub.value == 6
    assert pri_dep.type == module.ParameterType.U32 and pri_dep.value == 7
    assert pri_pri.type == module.ParameterType.U32 and pri_pri.value == 8

    parameters.pub_pub.write(mod, module.ParameterValue(0, module.ParameterType.U32))
    parameters.pub_dep.write(mod, module.ParameterValue(1, module.ParameterType.U32))
    parameters.pub_pri.write(mod, module.ParameterValue(2, module.ParameterType.U32))
    parameters.dep_pub.write(mod, module.ParameterValue(3, module.ParameterType.U32))
    parameters.dep_dep.write(mod, module.ParameterValue(4, module.ParameterType.U32))
    parameters.dep_pri.write(mod, module.ParameterValue(5, module.ParameterType.U32))
    parameters.pri_pub.write(mod, module.ParameterValue(6, module.ParameterType.U32))
    parameters.pri_dep.write(mod, module.ParameterValue(7, module.ParameterType.U32))
    parameters.pri_pri.write(mod, module.ParameterValue(8, module.ParameterType.U32))

    resources = mod.resources()
    mod.context().tracing().emit_info("empty: {}", resources.empty)
    mod.context().tracing().emit_info("a: {}", resources.a)
    mod.context().tracing().emit_info("b: {}", resources.b)
    mod.context().tracing().emit_info("img: {}", resources.img)

    imports = mod.imports()
    a_0 = imports.a_0
    a_1 = imports.a_1
    b_0 = imports.b_0
    b_1 = imports.b_1
    assert isinstance(a_0, module.Symbol)
    assert isinstance(a_1, module.Symbol)
    assert isinstance(b_0, module.Symbol)
    assert isinstance(b_1, module.Symbol)
    with a_0 as x:
        assert x.value == 5
    with a_1 as x:
        assert x.value == 10
    with b_0 as x:
        assert x.value == -2
    with b_1 as x:
        assert x.value == 77

    mod.context().tracing().emit_info("{!r}", mod.module_info())

    return cls()


module.export_module(
    name="c",
    description="Test module c",
    author="Fimo",
    license="None",
    parameters={
        "pub_pub": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Public,
            write=module.ParameterAccess.Public,
            default=0,
        ),
        "pub_dep": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Public,
            write=module.ParameterAccess.Dependency,
            default=1,
        ),
        "pub_pri": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Public,
            write=module.ParameterAccess.Private,
            default=2,
        ),
        "dep_pub": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Dependency,
            write=module.ParameterAccess.Public,
            default=3,
        ),
        "dep_dep": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Dependency,
            write=module.ParameterAccess.Dependency,
            default=4,
        ),
        "dep_pri": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Dependency,
            write=module.ParameterAccess.Private,
            default=5,
        ),
        "pri_pub": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Private,
            write=module.ParameterAccess.Public,
            default=6,
        ),
        "pri_dep": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Private,
            write=module.ParameterAccess.Dependency,
            default=7,
        ),
        "pri_pri": module.module_parameter(
            type=module.ParameterType.U32,
            read=module.ParameterAccess.Private,
            write=module.ParameterAccess.Private,
            default=8,
        ),
    },
    resources={
        "empty": module.module_resource(path=""),
        "a": module.module_resource(path="a.bin"),
        "b": module.module_resource(path="b.txt"),
        "img": module.module_resource(path="c/d.img"),
    },
    namespaces=[module.module_namespace_import(namespace="b")],
    imports={
        "a_0": module.module_symbol_import(symbol=a_export_0),
        "a_1": module.module_symbol_import(symbol=a_export_1),
        "b_0": module.module_symbol_import(symbol=b_export_0),
        "b_1": module.module_symbol_import(symbol=b_export_1),
    },
    factory=c_factory,
)


def test_modules():
    tracing_config = (
        tracing.CreationConfig()
        .with_max_level(tracing.Level.Trace)
        .with_subscriber(tracing.DefaultSubscriber)
    )
    context = Context.new_context([tracing_config])

    _t = tracing.ThreadAccess(context)

    def loading_set_filter(export: module.ModuleExport) -> bool:
        context.tracing().emit_info("{}", export)
        return True

    loading_set = module.LoadingSet.new(context)
    loading_set.append_modules(context, None, loading_set_filter)
    loading_set.finish()

    mod = module.PseudoModule(context)
    a = module.ModuleInfo.find_by_name(context, "a")
    b = module.ModuleInfo.find_by_name(context, "b")
    c = module.ModuleInfo.find_by_name(context, "c")
    assert mod.module_info().is_loaded()
    assert a.is_loaded()
    assert b.is_loaded()
    assert c.is_loaded()

    mod.acquire_dependency(a)
    mod.acquire_dependency(b)
    mod.acquire_dependency(c)

    a_0 = mod.load_symbol(a_export_0)
    with a_0 as x:
        assert x.value == 5

    with pytest.raises(error.Error):
        mod.load_symbol(b_export_0)
    mod.include_namespace(b_export_0.symbol_namespace())
    mod.load_symbol(b_export_0)

    mod.destroy()
    assert not a.is_loaded()
    assert not b.is_loaded()
    assert not c.is_loaded()

    _t.unregister()
