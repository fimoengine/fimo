#include <catch2/catch_all.hpp>
#include <filesystem>

#include <fimo_std/context.h>
#include <fimo_std/module.h>
#include <fimo_std/tracing.h>

#include <fimo_python_module_loader/loader.h>

TEST_CASE("Load module", "[python_module_loader]") {
    FimoTracingCreationConfig config = {
            .type = FIMO_STRUCT_TYPE_TRACING_CREATION_CONFIG,
            .next = nullptr,
            .format_buffer_size = 0,
            .maximum_level = FIMO_TRACING_LEVEL_TRACE,
            .subscribers = const_cast<FimoTracingSubscriber *>(&FIMO_TRACING_DEFAULT_SUBSCRIBER),
            .subscriber_count = 1,
    };
    const FimoBaseStructIn *options[] = {reinterpret_cast<FimoBaseStructIn *>(&config), nullptr};

    FimoContext context;
    FimoResult error = fimo_context_init(options, &context);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    error = fimo_tracing_register_thread(context);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    std::filesystem::path modules_dir = FIMO_MODULES_DIR;
    std::filesystem::path module_home = modules_dir / "python_module_loader";

#if _WIN32
    std::filesystem::path module_path = module_home / "module.dll";
#elif __APPLE__
    std::filesystem::path module_path = module_home / "module.dylib";
#elif __linux__
    std::filesystem::path module_path = module_home / "module.so";
#else
#error "Unknown operating system"
#endif
    std::string module_path_string = module_path.make_preferred().string();

    FimoModuleLoadingSet *set;
    error = fimo_module_set_new(context, &set);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    error = fimo_module_set_append_modules(
            context, set, module_path_string.c_str(), [](auto, auto) { return true; }, nullptr);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    error = fimo_module_set_finish(context, set);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    const FimoModule *pseudo_module;
    error = fimo_module_pseudo_module_new(context, &pseudo_module);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_context_release(context);

    const FimoModuleInfo *info;
    error = fimo_module_find_by_name(pseudo_module->context, "python_module_loader", &info);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    error = fimo_module_acquire_dependency(pseudo_module, info);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    FIMO_MODULE_INFO_RELEASE(info);

    error = fimo_module_namespace_include(pseudo_module, FIPY_SYMBOL_NAMESPACE);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    const FimoModuleRawSymbol *run_string_symbol;
    error = fimo_module_load_symbol(pseudo_module, FIPY_SYMBOL_NAME_RUN_STRING, FIPY_SYMBOL_NAMESPACE,
                                    FIMO_VERSION(FIPY_SYMBOL_VERSION_MAJOR_RUN_STRING,
                                                 FIPY_SYMBOL_VERSION_MINOR_RUN_STRING,
                                                 FIPY_SYMBOL_VERSION_PATCH_RUN_STRING),
                                    &run_string_symbol);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    auto run_string = static_cast<const FipyRunString *>(FIMO_MODULE_SYMBOL_LOCK(run_string_symbol));
    error = fipy_run_string(run_string, R"(print("Hello Python!"))", nullptr);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    FIMO_MODULE_SYMBOL_RELEASE(run_string_symbol);

    fimo_context_release(context);
}
