#include <catch2/catch_all.hpp>

#include <fimo_std/module.h>
#include <fimo_std/tracing.h>

static const int a_export_0 = 5;
static const int a_export_1 = 10;
static FimoModuleSymbolExport a_exports[] = {
        FIMO_MODULE_EXPORT_SYMBOL_VAR("a_export_0", a_export_0, 0, 1, 0),
        FIMO_MODULE_EXPORT_SYMBOL_VAR("a_export_1", a_export_1, 0, 1, 0),
};
FIMO_MODULE_EXPORT_MODULE("a", nullptr, nullptr, nullptr, FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(a_exports), )

static const int b_export_0 = -2;
static const int b_export_1 = 77;
static FimoModuleSymbolExport b_exports[] = {
        FIMO_MODULE_EXPORT_SYMBOL_VAR("b_export_0", b_export_0, 0, 1, 0),
        FIMO_MODULE_EXPORT_SYMBOL_VAR("b_export_1", b_export_1, 0, 1, 0),
};
FIMO_MODULE_EXPORT_MODULE("b", nullptr, nullptr, nullptr, FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(b_exports), )

static FimoModuleParamDecl c_params[] = {
        FIMO_MODULE_PARAM_U32("pub_pub", 0, FIMO_MODULE_PARAM_ACCESS_PUBLIC, FIMO_MODULE_PARAM_ACCESS_PUBLIC),
        FIMO_MODULE_PARAM_U32("pub_dep", 1, FIMO_MODULE_PARAM_ACCESS_PUBLIC, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY),
        FIMO_MODULE_PARAM_U32("pub_pri", 2, FIMO_MODULE_PARAM_ACCESS_PUBLIC, FIMO_MODULE_PARAM_ACCESS_PRIVATE),
        FIMO_MODULE_PARAM_U32("dep_pub", 3, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY, FIMO_MODULE_PARAM_ACCESS_PUBLIC),
        FIMO_MODULE_PARAM_U32("dep_dep", 4, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY),
        FIMO_MODULE_PARAM_U32("dep_pri", 5, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY, FIMO_MODULE_PARAM_ACCESS_PRIVATE),
        FIMO_MODULE_PARAM_U32("pri_pub", 6, FIMO_MODULE_PARAM_ACCESS_PRIVATE, FIMO_MODULE_PARAM_ACCESS_PUBLIC),
        FIMO_MODULE_PARAM_U32("pri_dep", 7, FIMO_MODULE_PARAM_ACCESS_PRIVATE, FIMO_MODULE_PARAM_ACCESS_DEPENDENCY),
        FIMO_MODULE_PARAM_U32("pri_pri", 8, FIMO_MODULE_PARAM_ACCESS_PRIVATE, FIMO_MODULE_PARAM_ACCESS_PRIVATE),
};
FIMO_MODULE_PARAM_TABLE(
        struct CParamTable, 9, struct CParamTable {
            FIMO_MODULE_PARAM_TABLE_PARAM(pub_pub);
            FIMO_MODULE_PARAM_TABLE_PARAM(pub_dep);
            FIMO_MODULE_PARAM_TABLE_PARAM(pub_pri);
            FIMO_MODULE_PARAM_TABLE_PARAM(dep_pub);
            FIMO_MODULE_PARAM_TABLE_PARAM(dep_dep);
            FIMO_MODULE_PARAM_TABLE_PARAM(dep_pri);
            FIMO_MODULE_PARAM_TABLE_PARAM(pri_pub);
            FIMO_MODULE_PARAM_TABLE_PARAM(pri_dep);
            FIMO_MODULE_PARAM_TABLE_PARAM(pri_pri);
        })
static FimoModuleResourceDecl c_resources[] = {
        FIMO_MODULE_RESOURCE(""),
        FIMO_MODULE_RESOURCE("a.bin"),
        FIMO_MODULE_RESOURCE("b.txt"),
        FIMO_MODULE_RESOURCE("c/d.img"),
};
FIMO_MODULE_RESOURCE_TABLE(
        struct CResourceTable, 4, struct CResourceTable {
            FIMO_MODULE_RESOURCE_TABLE_PARAM(empty);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(a);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(b);
            FIMO_MODULE_RESOURCE_TABLE_PARAM(img);
        })
static FimoModuleSymbolImport c_imports[] = {
        FIMO_MODULE_IMPORT_SYMBOL("a_export_0", 0, 1, 0),
        FIMO_MODULE_IMPORT_SYMBOL("a_export_1", 0, 1, 0),
        FIMO_MODULE_IMPORT_SYMBOL("b_export_0", 0, 1, 0),
        FIMO_MODULE_IMPORT_SYMBOL("b_export_1", 0, 1, 0),
};
FIMO_MODULE_SYMBOL_TABLE(
        struct CImportTable, 4, struct CImportTable {
            FIMO_MODULE_SYMBOL_TABLE_VAR(a_0, int);
            FIMO_MODULE_SYMBOL_TABLE_VAR(a_1, int);
            FIMO_MODULE_SYMBOL_TABLE_VAR(b_0, int);
            FIMO_MODULE_SYMBOL_TABLE_VAR(b_1, int);
        })
static FimoError c_constructor(const FimoModule *module, FimoModuleLoadingSet *set, void **data) {
    REQUIRE(module != nullptr);
    REQUIRE(set != nullptr);
    REQUIRE(data != nullptr);

    REQUIRE(module->parameters != nullptr);
    REQUIRE(module->resources != nullptr);
    REQUIRE(module->imports != nullptr);
    REQUIRE(module->exports == nullptr);
    REQUIRE(module->module_info != nullptr);
    REQUIRE(module->module_data == nullptr);

    const CParamTable *params = static_cast<const CParamTable *>(module->parameters);
    FimoU32 value;
    FimoModuleParamType type;
    FimoError error = fimo_module_param_get_private(module, &value, &type, params->pub_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 0 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pub_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pub_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 1 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pub_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pub_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 2 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pub_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 3 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->dep_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 4 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->dep_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 5 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->dep_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 6 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pri_pub);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 7 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pri_dep);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 8 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_private(module, &value, type, params->pri_pri);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    const CResourceTable *resources = static_cast<const CResourceTable *>(module->resources);
    (void)resources;

    const CImportTable *imports = static_cast<const CImportTable *>(module->imports);
    const int *a_0 = FIMO_MODULE_SYMBOL_LOCK(imports->a_0);
    const int *a_1 = FIMO_MODULE_SYMBOL_LOCK(imports->a_1);
    const int *b_0 = FIMO_MODULE_SYMBOL_LOCK(imports->b_0);
    const int *b_1 = FIMO_MODULE_SYMBOL_LOCK(imports->b_1);
    REQUIRE(*a_0 == a_export_0);
    REQUIRE(*a_1 == a_export_1);
    REQUIRE(*b_0 == b_export_0);
    REQUIRE(*b_1 == b_export_1);
    FIMO_MODULE_SYMBOL_RELEASE(imports->a_0);
    FIMO_MODULE_SYMBOL_RELEASE(imports->a_1);
    FIMO_MODULE_SYMBOL_RELEASE(imports->b_0);
    FIMO_MODULE_SYMBOL_RELEASE(imports->b_1);

    *data = nullptr;
    return FIMO_EOK;
}
static void c_destructor(const FimoModule *module, void *data) {
    (void)module;
    (void)data;
}
FIMO_MODULE_EXPORT_MODULE("c", nullptr, nullptr, nullptr, FIMO_MODULE_EXPORT_MODULE_PARAMS(c_params),
                          FIMO_MODULE_EXPORT_MODULE_RESOURCES(c_resources),
                          FIMO_MODULE_EXPORT_MODULE_SYMBOL_IMPORTS(c_imports),
                          FIMO_MODULE_EXPORT_MODULE_CONSTRUCTOR(c_constructor, c_destructor))

static bool modules_filter(const FimoModuleExport *arg0, void *arg1) {
    (void)arg0;
    (void)arg1;
    return true;
}

TEST_CASE("Load modules", "[modules]") {
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
    FimoError error = fimo_context_init(options, &context);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_tracing_register_thread(context);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    FimoModuleLoadingSet *set;
    error = fimo_module_set_new(context, &set);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_set_append_modules(context, set, nullptr, modules_filter, nullptr);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_set_finish(context, set);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    const FimoModule *pseudo_module;
    error = fimo_module_pseudo_module_new(context, &pseudo_module);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    fimo_context_release(context);

    FimoU32 value;
    FimoModuleParamType type;
    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "pub_pub");
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    REQUIRE((value == 0 && type == FIMO_MODULE_PARAM_TYPE_U32));
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_pub");
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "dep_pub");
    REQUIRE(FIMO_IS_ERROR(error));
    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "pri_pub");
    REQUIRE(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_dep");
    REQUIRE(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_pri");
    REQUIRE(FIMO_IS_ERROR(error));

    const FimoModuleInfo *a_info;
    error = fimo_module_find_by_name(pseudo_module->context, "a", &a_info);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    const FimoModuleInfo *c_info;
    error = fimo_module_find_by_name(pseudo_module->context, "c", &c_info);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_acquire_dependency(pseudo_module, a_info);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    error = fimo_module_acquire_dependency(pseudo_module, c_info);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_dependency(pseudo_module, &value, &type, "c", "dep_pub");
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_dependency(pseudo_module, &value, type, "c", "pub_dep");
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    const FimoModuleRawSymbol *a_export_0_symbol;
    error = fimo_module_load_symbol(pseudo_module, "a_export_0", "", FIMO_VERSION(0, 1, 0), &a_export_0_symbol);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));
    const int *a_export_0_symbol_ptr = static_cast<const int *>(FIMO_MODULE_SYMBOL_LOCK(a_export_0_symbol));
    REQUIRE(*a_export_0_symbol_ptr == a_export_0);
    FIMO_MODULE_SYMBOL_RELEASE(a_export_0_symbol);

    error = fimo_module_pseudo_module_destroy(pseudo_module, &context);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    FIMO_MODULE_INFO_RELEASE(a_info);
    FIMO_MODULE_INFO_RELEASE(c_info);

    fimo_context_release(context);
}
