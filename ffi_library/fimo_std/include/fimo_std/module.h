#ifndef FIMO_MODULE_H
#define FIMO_MODULE_H

#include <assert.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/version.h>

#include <fimo_std/impl/module.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FimoModule FimoModule;

/**
 * Type-erased set of modules to load by the subsystem.
 */
typedef struct FimoModuleLoadingSet FimoModuleLoadingSet;

/**
 * Data type of a module parameter.
 */
typedef enum FimoModuleParamType {
    FIMO_MODULE_PARAM_TYPE_U8,
    FIMO_MODULE_PARAM_TYPE_U16,
    FIMO_MODULE_PARAM_TYPE_U32,
    FIMO_MODULE_PARAM_TYPE_U64,
    FIMO_MODULE_PARAM_TYPE_I8,
    FIMO_MODULE_PARAM_TYPE_I16,
    FIMO_MODULE_PARAM_TYPE_I32,
    FIMO_MODULE_PARAM_TYPE_I64,
} FimoModuleParamType;

/**
 * Access group for a module parameter.
 */
typedef enum FimoModuleParamAccessGroup {
    FIMO_MODULE_PARAM_ACCESS_GROUP_PUBLIC,
    FIMO_MODULE_PARAM_ACCESS_GROUP_DEPENDENCY,
    FIMO_MODULE_PARAM_ACCESS_GROUP_PRIVATE,
} FimoModuleParamAccessGroup;

/**
 * A type-erased module parameter.
 */
typedef struct FimoModuleParam FimoModuleParam;

/**
 * A type-erased internal data type for a module parameter.
 */
typedef struct FimoModuleParamData FimoModuleParamData;

/**
 * Declaration of a module parameter.
 */
typedef struct FimoModuleParamDecl {
    /**
     * Type of the parameter.
     */
    FimoModuleParamType type;
    /**
     * Read access group.
     */
    FimoModuleParamAccessGroup read_group;
    /**
     * Write access group.
     */
    FimoModuleParamAccessGroup write_group;
    /**
     * Setter for a module parameter.
     *
     * The setter can perform some validation before the parameter is set.
     * If the setter produces an error, the parameter won't be modified.
     *
     * @param module pointer to the module
     * @param value pointer to the new value
     * @param type type of the value
     * @param param data of the parameter
     *
     * @return Status code.
     */
    FimoResult (*setter)(const FimoModule *arg0, const void *value, FimoModuleParamType type,
                         FimoModuleParamData *param);
    /**
     * Getter for a module parameter.
     *
     * @param module pointer to the module
     * @param value buffer to store the value into
     * @param type buffer to store the type of the value into
     * @param param data of the parameter
     *
     * @return Status code.
     */
    FimoResult (*getter)(const FimoModule *module, void *value, FimoModuleParamType *type,
                         const FimoModuleParamData *param);
    /**
     * Name of the parameter.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Default value of the parameter.
     */
    union {
        FimoU8 u8;
        FimoU16 u16;
        FimoU32 u32;
        FimoU64 u64;
        FimoI8 i8;
        FimoI16 i16;
        FimoI32 i32;
        FimoI64 i64;
    } default_value;
} FimoModuleParamDecl;

/**
 * Declaration of a module resource.
 */
typedef struct FimoModuleResourceDecl {
    /**
     * Resource path relative to the module directory.
     *
     * Must not be `NULL` or begin with a slash.
     */
    const char *path;
} FimoModuleResourceDecl;

/**
 * Declaration of a module namespace import.
 */
typedef struct FimoModuleNamespaceImport {
    /**
     * Imported namespace.
     *
     * Must not be `NULL`.
     */
    const char *name;
} FimoModuleNamespaceImport;

/**
 * Declaration of a module symbol import.
 */
typedef struct FimoModuleSymbolImport {
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleSymbolImport;

/**
 * Declaration of a static module symbol export.
 */
typedef struct FimoModuleSymbolExport {
    /**
     * Pointer to the symbol.
     */
    const void *symbol;
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleSymbolExport;

/**
 * Declaration of a dynamic module symbol export.
 */
typedef struct FimoModuleDynamicSymbolExport {
    /**
     * Symbol constructor.
     *
     * Must not be `NULL`.
     */
    /**
     * Constructor function for a dynamic symbol.
     *
     * The constructor is in charge of constructing an instance of
     * a symbol. To that effect, it is provided  an instance to the
     * module. The resulting symbol is written into the last argument.
     *
     * @param module pointer to the module
     * @param symbol pointer to the resulting symbol
     *
     * @return Status code.
     */
    FimoResult (*constructor)(const FimoModule *module, void **symbol);
    /**
     * Destructor function for a dynamic symbol.
     *
     * The destructor is safe to assume, that the symbol is no longer
     * used by any other module. During its destruction, a symbol is
     * not allowed to access the module subsystem.
     *
     * @param symbol symbol to destroy
     */
    void (*destructor)(void *symbol);
    /**
     * Symbol version.
     */
    FimoVersion version;
    /**
     * Symbol name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Symbol namespace.
     *
     * Must not be `NULL`.
     */
    const char *ns;
} FimoModuleDynamicSymbolExport;

/**
 * Valid keys of `FimoModuleExportModifier`.
 */
typedef enum FimoModuleExportModifierKey {
    /**
     * Specifies that the module export has a destructor function
     * that must be called. The value must be a pointer to a
     * `FimoModuleExportModifierDestructor`.
     */
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR,
    /**
     * Specifies that the module should acquire a static dependency
     * to another module. The value must be a strong reference to
     * a `FimoModuleInfo`.
     */
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY,
    FIMO_MODULE_EXPORT_MODIFIER_KEY_LAST,
    FIMO_MODULE_EXPORT_MODIFIER_KEY_FORCE32 = 0x7FFFFFFF
} FimoModuleExportModifierKey;

/**
 * A modifier declaration for a module export.
 */
typedef struct FimoModuleExportModifier {
    FimoModuleExportModifierKey key;
    const void *value;
} FimoModuleExportModifier;

/**
 * Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR` modifier key.
 */
typedef struct FimoModuleExportModifierDestructor {
    /**
     * Type-erased data to pass to the destructor.
     */
    void *data;
    /**
     * Destructor function.
     */
    void (*destructor)(void *);
} FimoModuleExportModifierDestructor;

/**
 * Declaration of a module export.
 */
typedef struct FimoModuleExport {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_MODULE_EXPORT`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Version of the context compiled against.
     */
    FimoVersion version;
    /**
     * Module name.
     *
     * The module name must be unique to the module.
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Module description.
     */
    const char *description;
    /**
     * Module author.
     */
    const char *author;
    /**
     * Module license.
     */
    const char *license;
    /**
     * List of parameters exposed by the module.
     *
     * A module is not allowed to expose duplicate parameters.
     */
    const FimoModuleParamDecl *parameters;
    /**
     * Number of parameters exposed by the module.
     */
    FimoU32 parameters_count;
    /**
     * List of resources declared by the module.
     */
    const FimoModuleResourceDecl *resources;
    /**
     * Number of resources declared by the module.
     */
    FimoU32 resources_count;
    /**
     * List of namespaces to import by the module.
     *
     * A module is only allowed to import and export symbols
     * from/to an imported namespace. It is an error to specify
     * a namespace, that does not exist, without exporting to
     * that namespace.
     */
    const FimoModuleNamespaceImport *namespace_imports;
    /**
     * Number of namespaces to import by the module.
     */
    FimoU32 namespace_imports_count;
    /**
     * List of symbols to import by the module.
     *
     * Upon loading, the module is provided the listed symbols.
     * If some symbols are not available, the loading fails.
     */
    const FimoModuleSymbolImport *symbol_imports;
    /**
     * Number of symbols to import by the module.
     */
    FimoU32 symbol_imports_count;
    /**
     * List of static symbols exported by the module.
     *
     * The named symbols will be made available to all other
     * modules. Trying to export a duplicate symbol will
     * result in an error upon loading of the module.
     */
    const FimoModuleSymbolExport *symbol_exports;
    /**
     * Number of static symbols exported by the module.
     */
    FimoU32 symbol_exports_count;
    /**
     * List of dynamic symbols exported by the module.
     *
     * A dynamic symbol is a symbol, whose creation is deferred
     * until loading of the module. This is useful, in case
     * the symbol depends on the module imports.
     */
    const FimoModuleDynamicSymbolExport *dynamic_symbol_exports;
    /**
     * Number of dynamic symbols exported by the module.
     */
    FimoU32 dynamic_symbol_exports_count;
    /**
     * List of modifier key-value pairs for the exported module.
     */
    const FimoModuleExportModifier *modifiers;
    /**
     * Number of modifiers for the module.
     */
    FimoU32 modifiers_count;
    /**
     * Optional constructor function for a module.
     *
     * The module constructor allows a module implementor to initialize
     * some module specific data at module load time. Some use cases for
     * module constructors are initialization of global module data, or
     * fetching optional symbols. Returning an error aborts the loading
     * of the module. Is called before the symbols of the modules are
     * exported/initialized.
     *
     * @param module pointer to the partially initialized module
     * @param set module set that contained the module
     * @param state pointer to the resulting module state
     *
     * @return Status code.
     */
    FimoResult (*constructor)(const FimoModule *module, FimoModuleLoadingSet *set, void **state);

    /**
     * Optional destructor function for a module.
     *
     * During its destruction, a module is not allowed to access the
     * module subsystem. Must be specified, if the module specifies a
     * constructor, and must be `NULL` otherwise.
     *
     * @param arg0 pointer to the module
     * @param arg1 module state to destroy
     */
    void (*destructor)(const FimoModule *module, void *state);
} FimoModuleExport;

/**
 * Opaque type for a parameter table of a module.
 *
 * The layout of a parameter table is equivalent to an array of
 * `FimoModuleParam*`, where each entry represents one parameter
 * of the module parameter declaration list.
 */
typedef void FimoModuleParamTable;

/**
 * Opaque type for a resource path table of a module.
 *
 * The import table is equivalent to an array of `const char*`,
 * where each entry represents one resource path. The resource
 * paths are ordered in declaration order.
 */
typedef void FimoModuleResourceTable;

/**
 * Opaque type for a symbol import table of a module.
 *
 * The import table is equivalent to an array of `const void*`,
 * where each entry represents one symbol of the module symbol
 * import list. The symbols are ordered in declaration order.
 */
typedef void FimoModuleSymbolImportTable;

/**
 * Opaque type for a symbol export table of a module.
 *
 * The export table is equivalent to an array of `const void*`,
 * where each entry represents one symbol of the module symbol
 * export list, followed by the entries of the dynamic symbol
 * export list.
 */
typedef void FimoModuleSymbolExportTable;

/**
 * Info of a loaded module.
 */
typedef struct FimoModuleInfo {
    /**
     * Type of the struct.
     *
     * Must be `FIMO_STRUCT_TYPE_MODULE_INFO`.
     */
    FimoStructType type;
    /**
     * Pointer to a possible extension.
     *
     * Reserved for future use. Must be `NULL`.
     */
    const FimoBaseStructIn *next;
    /**
     * Module name.
     *
     * Must not be `NULL`.
     */
    const char *name;
    /**
     * Module description.
     */
    const char *description;
    /**
     * Module author.
     */
    const char *author;
    /**
     * Module license.
     */
    const char *license;
    /**
     * Path to the module directory.
     */
    const char *module_path;
    /**
     * Increases the reference count of the info instance.
     */
    void (*acquire)(const struct FimoModuleInfo *info);
    /**
     * Decreases the reference count of the info instance.
     */
    void (*release)(const struct FimoModuleInfo *info);
    /**
     * Returns whether the owning module is still loaded.
     */
    bool (*is_loaded)(const struct FimoModuleInfo *info);
    /**
     * Increases the strong reference count of the module instance.
     *
     * Will prevent the module from being unloaded. This may be used to pass
     * data, like callbacks, between modules, without registering the dependency
     * with the subsystem.
     */
    FimoResult (*acquire_module_strong)(const struct FimoModuleInfo *info);
    /**
     * Decreases the strong reference count of the module instance.
     *
     * Should only be called after `acquire_module_strong`, when the dependency
     * is no longer required.
     */
    void (*release_module_strong)(const struct FimoModuleInfo *info);
} FimoModuleInfo;

/**
 * State of a loaded module.
 *
 * A module is self-contained, and may not be passed to other modules.
 * An instance of `FimoModule` is valid for as long as the owning module
 * remains loaded. Modules must not leak any resources outside it's own
 * module, ensuring that they are destroyed upon module unloading.
 */
typedef struct FimoModule {
    /**
     * Module parameter table.
     */
    const FimoModuleParamTable *parameters;
    /**
     * Module resource table.
     */
    const FimoModuleResourceTable *resources;
    /**
     * Module symbol import table.
     */
    const FimoModuleSymbolImportTable *imports;
    /**
     * Module symbol export table.
     */
    const FimoModuleSymbolExportTable *exports;
    /**
     * Module info.
     */
    const FimoModuleInfo *module_info;
    /**
     * Context that loaded the module.
     */
    FimoContext context;
    /**
     * Private data of the module.
     */
    void *module_data;
} FimoModule;

/**
 * A filter for selection modules to load by the module subsystem.
 *
 * The filter function is passed the module export declaration
 * and can then decide whether the module should be loaded by
 * the subsystem.
 *
 * @param arg0 module export to inspect
 * @param arg1 filter data
 *
 * @return `true`, if the module should be loaded.
 */
typedef bool (*FimoModuleLoadingFilter)(const FimoModuleExport *arg0, void *arg1);

/**
 * A callback for successfully loading a module.
 *
 * The callback function is called when the subsystem was successful
 * in loading the requested module, making it then possible to
 * request symbols.
 *
 * @param arg0 loaded module
 * @param arg1 callback data
 */
typedef void (*FimoModuleLoadingSuccessCallback)(const FimoModuleInfo *arg0, void *arg1);

/**
 * A callback for a module loading error.
 *
 * The callback function is called when the subsystem was not
 * successful in loading the requested module.
 *
 * @param arg0 module that caused the error
 * @param arg1 callback data
 */
typedef void (*FimoModuleLoadingErrorCallback)(const FimoModuleExport *arg0, void *arg1);

/**
 * VTable of the module subsystem.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoModuleVTableV0 {
    /**
     * Constructs a new pseudo module.
     *
     * The functions of the module subsystem require that the caller owns
     * a reference to their own module. This is a problem, as the constructor
     * of the context won't be assigned a module instance during bootstrapping.
     * As a workaround, we allow for the creation of pseudo modules, i.e.,
     * module handles without an associated module.
     *
     * @param ctx the context
     * @param module resulting pseudo module
     *
     * @return Status code.
     */
    FimoResult (*pseudo_module_new)(void *ctx, const FimoModule **module);
    /**
     * Destroys an existing pseudo module.
     *
     * By destroying the pseudo module, the caller ensures that they
     * relinquished all access to handles derived by the module subsystem.
     *
     * @param ctx the context
     * @param module pseudo module to destroy
     * @param out_ctx extracted context from the module
     *
     * @return Status code.
     */
    FimoResult (*pseudo_module_destroy)(void *ctx, const FimoModule *module, FimoContext *out_ctx);
    /**
     * Constructs a new empty module set.
     *
     * The loading of a module fails, if at least one dependency can
     * not be satisfied, which requires the caller to manually find a
     * suitable loading order. To facilitate the loading, we load
     * multiple modules together, and automatically determine an
     * appropriate load order for all modules inside the module set.
     *
     * @param ctx the context
     * @param set new module set
     *
     * @return Status code.
     */
    FimoResult (*set_new)(void *ctx, FimoModuleLoadingSet **set);
    /**
     * Checks whether a module set contains a module.
     *
     * @param ctx the context
     * @param set module set to query
     * @param name name of the module
     * @param contained query result
     *
     * @return Status code.
     */
    FimoResult (*set_has_module)(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                 bool *contained);
    /**
     * Checks whether a module set contains a symbol.
     *
     * @param ctx the context
     * @param set module set to query
     * @param name symbol name
     * @param ns namespace name
     * @param version symbol version
     * @param contained query result
     *
     * @return Status code.
     */
    FimoResult (*set_has_symbol)(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                 const char *ns, FimoVersion version, bool *contained);
    /**
     * Adds a status callback to the module set.
     *
     * Adds a set of callbacks to report a successful or failed loading of
     * a module. The `on_success` callback wil be called if the set was
     * able to load all requested modules, whereas the `on_error` callback
     * will be called immediately after the failed loading of the module. Since
     * the module set can be in a partially loaded state at the time of calling
     * this function, the `on_error` callback may be invoked immediately. The
     * callbacks will be provided with a user-specified data pointer, which they
     * are in charge of cleaning up. If the requested module `module` does
     * not exist, this function will return an error.
     *
     * @param ctx the context
     * @param set set of modules
     * @param module module to query
     * @param on_success success callback
     * @param on_error error callback
     * @param user_data callback user data
     *
     * @return Status code.
     */
    FimoResult (*set_append_callback)(void *ctx, FimoModuleLoadingSet *set,
                                      const char *module,
                                      FimoModuleLoadingSuccessCallback on_success,
                                      FimoModuleLoadingErrorCallback on_error,
                                      void *user_data);
    /**
     * Adds a freestanding module to the module set.
     *
     * Adds a freestanding module to the set, so that it may be loaded
     * by the set. Trying to include an invalid module, a module with
     * duplicate exports or duplicate name will result in an error.
     * This function allows for the loading of dynamic modules, i.e.
     * modules that are created at runtime, like non-native modules,
     * which may require a runtime to be executed in. The new module
     * inherits a strong reference to the same binary as the caller's module.
     * Note that the new module is not setup to automatically depend
     * on `module`, but may prevent it from being unloaded while
     * the set exists.
     *
     * @param ctx the context
     * @param owner owner of the export
     * @param set set of modules
     * @param module_export module to append to the set
     *
     * @return Status code.
     */
    FimoResult (*set_append_freestanding_module)(void *ctx, const FimoModule *owner,
                                                 FimoModuleLoadingSet *set,
                                                 const FimoModuleExport *module_export);
    /**
     * Adds modules to the module set.
     *
     * Opens up a module binary to select which modules to load.
     * The binary path `module_path` must be encoded as `UTF-8`,
     * and point to the binary that contains the modules.  If the
     * path is `NULL`, it iterates over the exported modules of the
     * current binary. Each exported module is then passed to the
     * `filter`, along with the provided `filter_data`, which can
     * then filter which modules to load. This function may skip
     * invalid module exports. Trying to include a module with duplicate
     * exports or duplicate name will result in an error. This function
     * signals an error, if the binary does not contain the symbols
     * necessary to query the exported modules, but does not return
     * an error, if it does not export any modules. The necessary
     * symbols are setup automatically, if the binary was linked with
     * the fimo library. In case of an error, no modules are appended
     * to the set.
     *
     * @param ctx the context
     * @param set set of modules
     * @param module_path path to the binary to inspect
     * @param filter filter function
     * @param filter_data custom data to pass to the filter function
     * @param iterator export iterator function
     * @param bin_handle pointer to a symbol contained in the callers binary
     *
     * @return Status code.
     */
    FimoResult (*set_append_modules)(void *ctx, FimoModuleLoadingSet *set,
                                     const char *module_path,
                                     FimoModuleLoadingFilter filter, void * filter_data,
                                     void (*iterator)(bool (*)(const FimoModuleExport *, void *), void *),
                                     const void *bin_handle);
    /**
     * Destroys the module set without loading any modules.
     *
     * It is not possible to dismiss a module set that is currently
     * being loaded.
     *
     * @param ctx the context
     * @param set the module set to destroy
     *
     * @return Status code.
     */
    FimoResult (*set_dismiss)(void *ctx, FimoModuleLoadingSet *set);
    /**
     * Destroys the module set and loads the modules contained in it.
     *
     * After successfully calling this function, the modules contained
     * in the set are loaded, and their symbols are available to all
     * other modules. If the construction of one module results in an
     * error, or if a dependency can not be satisfied, this function
     * rolls back the loading of all modules contained in the set
     * and returns an error. It is not possible to load a module set,
     * while another set is being loaded.
     *
     * @param ctx the context
     * @param set a set of modules to load
     *
     * @return Status code.
     */
    FimoResult (*set_finish)(void *ctx, FimoModuleLoadingSet *set);
    /**
     * Searches for a module by it's name.
     *
     * Queries a module by its unique name. The returned `FimoModuleInfo`
     * will have its reference count increased.
     *
     * @param ctx context
     * @param name module name
     * @param info resulting module info.
     *
     * @return Status code.
     */
    FimoResult (*find_by_name)(void *ctx, const char *name, const FimoModuleInfo **info);
    /**
     * Searches for a module by a symbol it exports.
     *
     * Queries the module that exported the specified symbol. The returned
     * `FimoModuleInfo` will have its reference count increased.
     *
     * @param ctx context
     * @param name symbol name
     * @param ns symbol namespace
     * @param version symbol version
     * @param info resulting module info
     *
     * @return Status code.
     */
    FimoResult (*find_by_symbol)(void *ctx, const char *name, const char *ns, FimoVersion version,
                                 const FimoModuleInfo **info);
    /**
     * Checks for the presence of a namespace in the module subsystem.
     *
     * A namespace exists, if at least one loaded module exports
     * one symbol in said namespace.
     *
     * @param ctx context
     * @param ns namespace to query
     * @param exists query result
     *
     * @return Status code.
     */
    FimoResult (*namespace_exists)(void *ctx, const char *ns, bool *exists);
    /**
     * Includes a namespace by the module.
     *
     * Once included, the module gains access to the symbols
     * of its dependencies that are exposed in said namespace.
     * A namespace can not be included multiple times.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param ns namespace to include
     *
     * @return Status code.
     */
    FimoResult (*namespace_include)(void *ctx, const FimoModule *caller, const char *ns);
    /**
     * Removes a namespace include from the module.
     *
     * Once excluded, the caller guarantees to relinquish
     * access to the symbols contained in said namespace.
     * It is only possible to exclude namespaces that were
     * manually added, whereas static namespace includes
     * remain valid until the module is unloaded.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param ns namespace to exclude.
     *
     * @return Status code.
     */
    FimoResult (*namespace_exclude)(void *ctx, const FimoModule *caller, const char *ns);
    /**
     * Checks if a module includes a namespace.
     *
     * Checks if `module` specified that it includes the
     * namespace `ns`. In that case, the module is allowed access
     * to the symbols in the namespace. The result of the query
     * is stored in `is_included`. Additionally, this function also
     * queries whether the include is static, i.e., the include was
     * specified by the module at load time. The include type is
     * stored in `is_static`.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param ns namespace to query
     * @param is_included result of the query
     * @param is_static resulting include type
     *
     * @return Status code.
     */
    FimoResult (*namespace_included)(void *ctx, const FimoModule *caller, const char *ns,
                                     bool *is_included, bool *is_static);
    /**
     * Acquires another module as a dependency.
     *
     * After acquiring a module as a dependency, the module
     * is allowed access to the symbols and protected parameters
     * of said dependency. Trying to acquire a dependency to a
     * module that is already a dependency, or to a module that
     * would result in a circular dependency will result in an
     * error.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param info module to acquire as a dependency
     *
     * @return Status code.
     */
    FimoResult (*acquire_dependency)(void *ctx, const FimoModule *caller,
                                     const FimoModuleInfo *info);
    /**
     * Removes a module as a dependency.
     *
     * By removing a module as a dependency, the caller
     * ensures that it does not own any references to resources
     * originating from the former dependency, and allows for
     * the unloading of the module. A module can only relinquish
     * dependencies to modules that were acquired dynamically,
     * as static dependencies remain valid until the module is
     * unloaded.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param info dependency to remove
     *
     * @return Status code.
     */
    FimoResult (*relinquish_dependency)(void *ctx, const FimoModule *caller,
                                        const FimoModuleInfo *info);
    /**
     * Checks if a module depends on another module.
     *
     * Checks if `other` is a dependency of `module`. In that
     * case `module` is allowed to access the symbols exported
     * by `other`. The result of the query is stored in
     * `has_dependency`. Additionally, this function also
     * queries whether the dependency is static, i.e., the
     * dependency was set by the module subsystem at load time.
     * The dependency type is stored in `is_static`.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param info other module to check as a dependency
     * @param has_dependency result of the query
     * @param is_static resulting dependency type
     *
     * @return Status code.
     */
    FimoResult (*has_dependency)(void *ctx, const FimoModule *caller, const FimoModuleInfo *info,
                                 bool *has_dependency, bool *is_static);

    /**
     * Loads a symbol from the module subsystem.
     *
     * The caller can query the subsystem for a symbol of a loaded
     * module. This is useful for loading optional symbols, or
     * for loading symbols after the creation of a module. The
     * symbol, if it exists, is written into `symbol`, and can
     * be used until the module relinquishes the dependency to
     * the module that exported the symbol. This function fails,
     * if the module containing the symbol is not a dependency
     * of the module.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param name symbol name
     * @param ns symbol namespace
     * @param version symbol version
     * @param symbol resulting symbol
     *
     * @return Status code.
     */
    FimoResult (*load_symbol)(void *ctx, const FimoModule *caller, const char *name,
                              const char *ns, FimoVersion version, const void **symbol);
    /**
     * Unloads a module.
     *
     * If successful, this function unloads the module `module`.
     * To succeed, the module no other module may depend on the module.
     * This function automatically unloads cleans up unreferenced modules,
     * except if they are a pseudo module.
     *
     * Setting `module` to `NULL` only runs the cleanup of all loose modules.
     *
     * @param ctx the context
     * @param info module to unload
     *
     * @return Status code.
     */
    FimoResult (*unload)(void *ctx, const FimoModuleInfo *info);
    /**
     * Queries the info of a module parameter.
     *
     * This function can be used to query the datatype, the read access,
     * and the write access of a module parameter. This function fails,
     * if the parameter can not be found.
     *
     * @param ctx context
     * @param module name of the module containing the parameter
     * @param param parameter to query
     * @param type queried parameter datatype
     * @param read_group queried parameter read group
     * @param write_group queried parameter write group
     *
     * @return Status code.
     */
    FimoResult (*param_query)(void *ctx, const char *module, const char *param,
                              FimoModuleParamType *type, FimoModuleParamAccessGroup *read_group,
                              FimoModuleParamAccessGroup *write_group);
    /**
     * Sets a module parameter with public write access.
     *
     * Sets the value of a module parameter with public write access.
     * The operation fails, if the parameter does not exist, or if
     * the parameter does not allow writing with a public access.
     * The caller must ensure that `value` points to an instance of
     * the same datatype as the parameter in question.
     *
     * @param ctx context
     * @param value pointer to the value to store
     * @param type type of the value
     * @param module name of the module containing the parameter
     * @param param name of the parameter
     *
     * @return Status code.
     */
    FimoResult (*param_set_public)(void *ctx, const void *value, FimoModuleParamType type,
                                   const char *module, const char *param);
    /**
     * Reads a module parameter with public read access.
     *
     * Reads the value of a module parameter with public read access.
     * The operation fails, if the parameter does not exist, or if
     * the parameter does not allow reading with a public access.
     * The caller must ensure that `value` points to an instance of
     * the same datatype as the parameter in question.
     *
     * @param ctx context
     * @param value pointer where to store the value
     * @param type buffer where to store the type of the parameter
     * @param module name of the module containing the parameter
     * @param param name of the parameter
     *
     * @return Status code.
     */
    FimoResult (*param_get_public)(void *ctx, void *value, FimoModuleParamType *type,
                                   const char *module, const char *param);
    /**
     * Sets a module parameter with dependency write access.
     *
     * Sets the value of a module parameter with dependency write
     * access. The operation fails, if the parameter does not exist,
     * or if the parameter does not allow writing with a dependency
     * access. The caller must ensure that `value` points to an
     * instance of the same datatype as the parameter in question.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value pointer to the value to store
     * @param type type of the value
     * @param module name of the module containing the parameter
     * @param param name of the parameter
     *
     * @return Status code.
     */
    FimoResult (*param_set_dependency)(void *ctx, const FimoModule *caller, const void *value,
                                       FimoModuleParamType type, const char *module,
                                       const char *param);
    /**
     * Reads a module parameter with dependency read access.
     *
     * Reads the value of a module parameter with dependency read
     * access. The operation fails, if the parameter does not exist,
     * or if the parameter does not allow reading with a dependency
     * access. The caller must ensure that `value` points to an
     * instance of the same datatype as the parameter in question.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value pointer where to store the value
     * @param type buffer where to store the type of the parameter
     * @param module name of the module containing the parameter
     * @param param name of the parameter
     *
     * @return Status code.
     */
    FimoResult (*param_get_dependency)(void *ctx, const FimoModule *caller, void *value,
                                       FimoModuleParamType *type, const char *module,
                                       const char *param);
    /**
     * Setter for a module parameter.
     *
     * If the setter produces an error, the parameter won't be modified.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value value to write
     * @param type type of the value
     * @param param parameter to write
     *
     * @return Status code.
     */
    FimoResult (*param_set_private)(void *ctx, const FimoModule *caller, const void *value,
                                    FimoModuleParamType type, FimoModuleParam *param);
    /**
     * Getter for a module parameter.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value buffer where to store the parameter
     * @param type buffer where to store the type of the parameter
     * @param param parameter to load
     *
     * @return Status code.
     */
    FimoResult (*param_get_private)(void *ctx, const FimoModule *caller, void *value,
                                    FimoModuleParamType *type, const FimoModuleParam *param);
    /**
     * Internal setter for a module parameter.
     *
     * If the setter produces an error, the parameter won't be modified.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value value to write
     * @param type type of the value
     * @param param parameter to write
     *
     * @return Status code.
     */
    FimoResult (*param_set_inner)(void *ctx, const FimoModule *caller, const void *value,
                                  FimoModuleParamType type, FimoModuleParamData *param);
    /**
     * Internal getter for a module parameter.
     *
     * @param ctx context
     * @param caller module of the caller
     * @param value buffer where to store the parameter
     * @param type buffer where to store the type of the parameter
     * @param param parameter to load
     *
     * @return Status code.
     */
    FimoResult (*param_get_inner)(void *ctx, const FimoModule *caller, void *value,
                                  FimoModuleParamType *type, const FimoModuleParamData *param);
} FimoModuleVTableV0;

#ifdef __cplusplus
}
#endif

#endif // FIMO_MODULE_H
