#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

#include <fimo_std/module.h>

static const int a_export_0 = 5;
static const int a_export_1 = 10;
static FimoModuleSymbolExport a_exports[] = {
        FIMO_MODULE_EXPORT_SYMBOL_VAR("a_export_0", a_export_0, 0, 1, 0),
        FIMO_MODULE_EXPORT_SYMBOL_VAR("a_export_1", a_export_1, 0, 1, 0),
};
FIMO_MODULE_EXPORT_MODULE("a", NULL, NULL, NULL, FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(a_exports), )

static const int b_export_0 = -2;
static const int b_export_1 = 77;
static FimoModuleSymbolExport b_exports[] = {
        FIMO_MODULE_EXPORT_SYMBOL_VAR("b_export_0", b_export_0, 0, 1, 0),
        FIMO_MODULE_EXPORT_SYMBOL_VAR("b_export_1", b_export_1, 0, 1, 0),
};
FIMO_MODULE_EXPORT_MODULE("b", NULL, NULL, NULL, FIMO_MODULE_EXPORT_MODULE_SYMBOL_EXPORTS(b_exports), )

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
static FimoError c_constructor(const FimoModule *module, FimoModuleLoadingSet *set, void *reserved, void **data) {
    assert_true(module != NULL);
    assert_true(set != NULL);
    assert_true(reserved == NULL);
    assert_true(data != NULL);

    assert_true(module->parameters != NULL);
    assert_true(module->resources != NULL);
    assert_true(module->imports != NULL);
    assert_true(module->exports == NULL);
    assert_true(module->module_info != NULL);
    assert_true(module->module_data == NULL);

    const struct CParamTable *params = module->parameters;
    FimoU32 value;
    FimoModuleParamType type;
    FimoError error = fimo_module_param_get_private(module, &value, &type, params->pub_pub);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 0 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pub_pub);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pub_dep);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 1 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pub_dep);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pub_pri);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 2 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pub_pri);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_pub);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 3 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->dep_pub);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_dep);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 4 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->dep_dep);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->dep_pri);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 5 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->dep_pri);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_pub);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 6 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pri_pub);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_dep);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 7 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pri_dep);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_private(module, &value, &type, params->pri_pri);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 8 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_private(module, &value, type, params->pri_pri);
    assert_false(FIMO_IS_ERROR(error));

    const struct CResourceTable *resources = module->resources;
    (void)resources;

    const struct CImportTable *imports = module->imports;
    const int *a_0 = FIMO_MODULE_SYMBOL_LOCK(imports->a_0);
    const int *a_1 = FIMO_MODULE_SYMBOL_LOCK(imports->a_1);
    const int *b_0 = FIMO_MODULE_SYMBOL_LOCK(imports->b_0);
    const int *b_1 = FIMO_MODULE_SYMBOL_LOCK(imports->b_1);
    assert_true(*a_0 == a_export_0);
    assert_true(*a_1 == a_export_1);
    assert_true(*b_0 == b_export_0);
    assert_true(*b_1 == b_export_1);
    FIMO_MODULE_SYMBOL_RELEASE(imports->a_0);
    FIMO_MODULE_SYMBOL_RELEASE(imports->a_1);
    FIMO_MODULE_SYMBOL_RELEASE(imports->b_0);
    FIMO_MODULE_SYMBOL_RELEASE(imports->b_1);

    *data = NULL;
    return FIMO_EOK;
}
static void c_destructor(const FimoModule *module, void *reserved, void *data) {
    (void)module;
    (void)reserved;
    (void)data;
}
FIMO_MODULE_EXPORT_MODULE("c", NULL, NULL, NULL, FIMO_MODULE_EXPORT_MODULE_PARAMS(c_params),
                          FIMO_MODULE_EXPORT_MODULE_RESOURCES(c_resources),
                          FIMO_MODULE_EXPORT_MODULE_SYMBOL_IMPORTS(c_imports),
                          FIMO_MODULE_EXPORT_MODULE_CONSTRUCTOR(c_constructor, c_destructor))

static bool modules_filter(const FimoModuleExport *arg0, void *arg1) {
    (void)arg0;
    (void)arg1;
    return true;
}

static void load_modules(void **state) {
    (void)state; /* unused */
    FimoContext context;
    FimoError error = fimo_context_init(NULL, &context);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_lock(context);
    assert_false(FIMO_IS_ERROR(error));

    FimoModuleLoadingSet *set;
    error = fimo_module_set_new(context, &set);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_set_append_modules(context, set, NULL, modules_filter, NULL);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_set_finish(context, set);
    assert_false(FIMO_IS_ERROR(error));

    const FimoModule *pseudo_module;
    error = fimo_module_pseudo_module_new(context, &pseudo_module);
    assert_false(FIMO_IS_ERROR(error));
    fimo_context_release(context);

    FimoU32 value;
    FimoModuleParamType type;
    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "pub_pub");
    assert_false(FIMO_IS_ERROR(error));
    assert_true(value == 0 && type == FIMO_MODULE_PARAM_TYPE_U32);
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_pub");
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "dep_pub");
    assert_true(FIMO_IS_ERROR(error));
    error = fimo_module_param_get_public(pseudo_module->context, &value, &type, "c", "pri_pub");
    assert_true(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_dep");
    assert_true(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_public(pseudo_module->context, &value, type, "c", "pub_pri");
    assert_true(FIMO_IS_ERROR(error));

    const FimoModuleInfo *a_info;
    error = fimo_module_find_by_name(pseudo_module->context, "a", &a_info);
    assert_false(FIMO_IS_ERROR(error));
    const FimoModuleInfo *c_info;
    error = fimo_module_find_by_name(pseudo_module->context, "c", &c_info);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_acquire_dependency(pseudo_module, a_info);
    assert_false(FIMO_IS_ERROR(error));
    error = fimo_module_acquire_dependency(pseudo_module, c_info);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_param_get_dependency(pseudo_module, &value, &type, "c", "dep_pub");
    assert_false(FIMO_IS_ERROR(error));
    error = fimo_module_param_set_dependency(pseudo_module, &value, type, "c", "pub_dep");
    assert_false(FIMO_IS_ERROR(error));

    const FimoModuleRawSymbol *a_export_0_symbol;
    error = fimo_module_load_symbol(pseudo_module, "a_export_0", "", (FimoVersion)FIMO_VERSION(0, 1, 0),
                                    &a_export_0_symbol);
    assert_false(FIMO_IS_ERROR(error));
    const int *a_export_0_symbol_ptr = FIMO_MODULE_SYMBOL_LOCK(a_export_0_symbol);
    assert_true(*a_export_0_symbol_ptr == a_export_0);
    FIMO_MODULE_SYMBOL_RELEASE(a_export_0_symbol);

    error = fimo_module_pseudo_module_destroy(pseudo_module, &context);
    assert_false(FIMO_IS_ERROR(error));

    error = fimo_module_unlock(context);
    assert_false(FIMO_IS_ERROR(error));

    fimo_context_release(context);
}

int main(void) {
    const struct CMUnitTest tests[] = {
            cmocka_unit_test(load_modules),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}