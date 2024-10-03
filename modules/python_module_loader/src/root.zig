const std = @import("std");
const testing = std.testing;
const test_metadata = @import("test_metadata");

const fimo_std = @cImport({
    @cInclude("fimo_std/context.h");
    @cInclude("fimo_std/module.h");
    @cInclude("fimo_std/tracing.h");
});

const fimo_python_meta = @cImport({
    @cInclude("fimo_python_module_loader/loader.h");
});

test "run string" {
    const config = fimo_std.FimoTracingCreationConfig{
        .type = fimo_std.FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG,
        .next = null,
        .format_buffer_size = 0,
        .maximum_level = fimo_std.FIMO_TRACING_LEVEL_TRACE,
        .subscribers = @constCast(&fimo_std.FIMO_TRACING_DEFAULT_SUBSCRIBER),
        .subscriber_count = 1,
    };
    var options = [2][*c]const fimo_std.FimoBaseStructIn{ @ptrCast(&config), null };

    var context: fimo_std.FimoContext = undefined;
    var err = fimo_std.fimo_context_init(options[0..].ptr, &context);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
    defer fimo_std.fimo_context_release(context);

    err = fimo_std.fimo_tracing_register_thread(context);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
    defer _ = fimo_std.fimo_tracing_unregister_thread(context);

    const module_path = try std.fs.path.joinZ(testing.allocator, &.{
        test_metadata.modules_path.?,
        "python_module_loader",
        "module.fimo_module",
    });
    defer testing.allocator.free(module_path);

    var set: ?*fimo_std.FimoModuleLoadingSet = null;
    err = fimo_std.fimo_module_set_new(context, &set);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
    errdefer _ = fimo_std.fimo_module_set_dismiss(context, set);

    err = fimo_std.fimo_module_set_append_modules(
        context,
        set,
        module_path.ptr,
        struct {
            fn f(exp: [*c]const fimo_std.FimoModuleExport, data: ?*anyopaque) callconv(.C) bool {
                _ = exp;
                _ = data;
                return true;
            }
        }.f,
        null,
    );
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));

    err = fimo_std.fimo_module_set_finish(context, set);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));

    var pseudo_module: ?*const fimo_std.FimoModule = null;
    err = fimo_std.fimo_module_pseudo_module_new(context, &pseudo_module);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
    defer _ = fimo_std.fimo_module_pseudo_module_destroy(pseudo_module, &context);
    fimo_std.fimo_context_release(context);

    var info: ?*const fimo_std.FimoModuleInfo = null;
    err = fimo_std.fimo_module_find_by_name(pseudo_module.?.context, "python_module_loader", &info);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
    defer fimo_std.FIMO_MODULE_INFO_RELEASE(info);

    err = fimo_std.fimo_module_acquire_dependency(pseudo_module, info);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));

    err = fimo_std.fimo_module_namespace_include(pseudo_module, fimo_python_meta.FIPY_SYMBOL_NAMESPACE);
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));

    var run_string_symbol: ?*const anyopaque = null;
    err = fimo_std.fimo_module_load_symbol(
        pseudo_module,
        fimo_python_meta.FIPY_SYMBOL_NAME_RUN_STRING,
        fimo_python_meta.FIPY_SYMBOL_NAMESPACE,
        .{
            .major = fimo_python_meta.FIPY_SYMBOL_VERSION_MAJOR_RUN_STRING,
            .minor = fimo_python_meta.FIPY_SYMBOL_VERSION_MINOR_RUN_STRING,
            .patch = fimo_python_meta.FIPY_SYMBOL_VERSION_PATCH_RUN_STRING,
            .build = 0,
        },
        &run_string_symbol,
    );
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));

    const run_string: *const fimo_python_meta.FipyRunString = @alignCast(@ptrCast(run_string_symbol));
    err = @bitCast(fimo_python_meta.fipy_run_string(
        run_string,
        \\print("Hello Python!")
    ,
        null,
    ));
    try testing.expect(fimo_std.FIMO_RESULT_IS_OK(err));
}
