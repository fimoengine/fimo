#ifndef FIMO_INTERNAL_MODULE_H
#define FIMO_INTERNAL_MODULE_H

#include <fimo_std/array_list.h>
#include <fimo_std/graph.h>
#include <fimo_std/module.h>

#include <hashmap/hashmap.h>

#if __APPLE__
#include <tinycthread/tinycthread.h>
#else
#include <threads.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FimoInternalModuleContext {
    mtx_t mutex;
    struct hashmap *symbols;
    struct hashmap *modules;
    struct hashmap *namespaces;
    FimoGraph *dependency_graph;
    bool is_loading;
} FimoInternalModuleContext;

///////////////////////////////////////////////////////////////////////
//// Trampoline functions
///////////////////////////////////////////////////////////////////////

FimoError fimo_internal_trampoline_module_pseudo_module_new(void *ctx, const FimoModule **module);
FimoError fimo_internal_trampoline_module_pseudo_module_destroy(void *ctx, const FimoModule *module,
                                                                FimoContext *module_context);
FimoError fimo_internal_trampoline_module_set_new(void *ctx, FimoModuleLoadingSet **set);
FimoError fimo_internal_trampoline_module_set_has_module(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                                         bool *has_module);
FimoError fimo_internal_trampoline_module_set_has_symbol(void *ctx, FimoModuleLoadingSet *set, const char *name,
                                                         const char *ns, FimoVersion version, bool *has_symbol);
FimoError fimo_internal_trampoline_module_set_append_callback(void *ctx, FimoModuleLoadingSet *set,
                                                              const char *module_name,
                                                              FimoModuleLoadingSuccessCallback on_success,
                                                              FimoModuleLoadingErrorCallback on_error, void *user_data);
FimoError fimo_internal_trampoline_module_set_append_freestanding_module(void *ctx, const FimoModule *module,
                                                                         FimoModuleLoadingSet *set,
                                                                         const FimoModuleExport *export);
FimoError fimo_internal_trampoline_module_set_append_modules(
        void *ctx, FimoModuleLoadingSet *set, const char *module_path, FimoModuleLoadingFilter filter,
        void *filter_data, void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *),
        const void *binary_handle);
FimoError fimo_internal_trampoline_module_set_dismiss(void *ctx, FimoModuleLoadingSet *set);
FimoError fimo_internal_trampoline_module_set_finish(void *ctx, FimoModuleLoadingSet *set);
FimoError fimo_internal_trampoline_module_find_by_name(void *ctx, const char *name, const FimoModuleInfo **module);
FimoError fimo_internal_trampoline_module_find_by_symbol(void *ctx, const char *name, const char *ns,
                                                         FimoVersion version, const FimoModuleInfo **module);
FimoError fimo_internal_trampoline_module_namespace_exists(void *ctx, const char *ns, bool *exists);
FimoError fimo_internal_trampoline_module_namespace_include(void *ctx, const FimoModule *module, const char *ns);
FimoError fimo_internal_trampoline_module_namespace_exclude(void *ctx, const FimoModule *module, const char *ns);
FimoError fimo_internal_trampoline_module_namespace_included(void *ctx, const FimoModule *module, const char *ns,
                                                             bool *is_included, bool *is_static);
FimoError fimo_internal_trampoline_module_acquire_dependency(void *ctx, const FimoModule *module,
                                                             const FimoModuleInfo *dependency);
FimoError fimo_internal_trampoline_module_relinquish_dependency(void *ctx, const FimoModule *module,
                                                                const FimoModuleInfo *dependency);
FimoError fimo_internal_trampoline_module_has_dependency(void *ctx, const FimoModule *module,
                                                         const FimoModuleInfo *other, bool *has_dependency,
                                                         bool *is_static);
FimoError fimo_internal_trampoline_module_param_query(void *ctx, const char *module_name, const char *param,
                                                      FimoModuleParamType *type, FimoModuleParamAccess *read,
                                                      FimoModuleParamAccess *write);
FimoError fimo_internal_trampoline_module_param_set_public(void *ctx, const void *value, FimoModuleParamType type,
                                                           const char *module_name, const char *param);
FimoError fimo_internal_trampoline_module_param_get_public(void *ctx, void *value, FimoModuleParamType *type,
                                                           const char *module_name, const char *param);
FimoError fimo_internal_trampoline_module_param_set_dependency(void *ctx, const FimoModule *module, const void *value,
                                                               FimoModuleParamType type, const char *module_name,
                                                               const char *param);
FimoError fimo_internal_trampoline_module_param_get_dependency(void *ctx, const FimoModule *module, void *value,
                                                               FimoModuleParamType *type, const char *module_name,
                                                               const char *param);
FimoError fimo_internal_trampoline_module_load_symbol(void *ctx, const FimoModule *module, const char *name,
                                                      const char *ns, FimoVersion version,
                                                      const FimoModuleRawSymbol **symbol);
FimoError fimo_internal_trampoline_module_unload(void *ctx, const FimoModuleInfo *module);
FimoError fimo_internal_trampoline_module_param_set_private(void *ctx, const FimoModule *module, const void *value,
                                                            FimoModuleParamType type, FimoModuleParam *param);
FimoError fimo_internal_trampoline_module_param_get_private(void *ctx, const FimoModule *module, void *value,
                                                            FimoModuleParamType *type, const FimoModuleParam *param);
FimoError fimo_internal_trampoline_module_param_set_inner(void *ctx, const FimoModule *module, const void *value,
                                                          FimoModuleParamType type, FimoModuleParamData *param);
FimoError fimo_internal_trampoline_module_get_inner(void *ctx, const FimoModule *module, void *value,
                                                    FimoModuleParamType *type, const FimoModuleParamData *param);

///////////////////////////////////////////////////////////////////////
//// Module Subsystem API
///////////////////////////////////////////////////////////////////////

/**
 * Initializes the module subsystem.
 *
 * @param ctx the context.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_init(FimoInternalModuleContext *ctx);

/**
 * Destroys the module subsystem.
 *
 * @param ctx the context.
 */
void fimo_internal_module_destroy(FimoInternalModuleContext *ctx);

/**
 * Constructs a new pseudo module.
 *
 * The functions of the module backend require that the caller owns
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
FIMO_MUST_USE
FimoError fimo_internal_module_pseudo_module_new(FimoInternalModuleContext *ctx, const FimoModule **module);

/**
 * Destroys an existing pseudo module.
 *
 * By destroying the pseudo module, the caller ensures that they
 * relinquished all access to handles derived by the module backend.
 *
 * @param ctx the context
 * @param module pseudo module to destroy
 * @param module_context context extracted from the module
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_pseudo_module_destroy(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                     FimoContext *module_context);

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
FIMO_MUST_USE
FimoError fimo_internal_module_set_new(FimoInternalModuleContext *ctx, FimoModuleLoadingSet **set);

/**
 * Checks whether a module set contains a module.
 *
 * @param ctx the context
 * @param set module set to query
 * @param name name of the module
 * @param has_module query result
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_set_has_module(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                              const char *name, bool *has_module);

/**
 * Checks whether a module set contains a symbol.
 *
 * @param ctx the context
 * @param set module set to query
 * @param name symbol name
 * @param ns namespace name
 * @param version symbol version
 * @param has_symbol query result
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_set_has_symbol(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                              const char *name, const char *ns, FimoVersion version, bool *has_symbol);

/**
 * Adds a status callback to the module set.
 *
 * Adds a set of callbacks to report a successful or failed loading of
 * a module. The `on_success` callback wil be called if the set was able to load
 * the requested module, whereas the `on_error` callback will be called immediately
 * after the failed loading of the module. Since the module set can be in a partially
 * loaded state at the time of calling this function, one of the callbacks may be
 * invoked immediately. The callbacks will be provided with a user-specified data
 * pointer, which they are in charge of to clean up. If the requested module
 * `module_name` does not exist, this function will return an error.
 *
 * @param ctx the context
 * @param set set of modules
 * @param module_name module to query
 * @param on_success success callback
 * @param on_error error callback
 * @param user_data callback user data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_set_append_callback(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                                   const char *module_name, FimoModuleLoadingSuccessCallback on_success,
                                                   FimoModuleLoadingErrorCallback on_error, void *user_data);

/**
 * Adds a freestanding module to the module set.
 *
 * Adds a freestanding module to the set, so that it may be loaded
 * by a future call to `fimo_module_set_finish`. Trying to include
 * an invalid module, a module with duplicate exports or duplicate
 * name will result in an error. Unlike `fimo_module_set_append_modules`,
 * this function allows for the loading of dynamic modules, i.e.
 * modules that are created at runtime, like non-native modules,
 * which may require a runtime to be executed in. To ensure that
 * the binary of the module calling this function is not unloaded
 * while the new module is instantiated, the new module inherits
 * a strong reference to the same binary as the callers module.
 * Note that the new module is not setup to automatically depend
 * on `module`, but may prevent it from being unloaded while
 * the set exists.
 *
 * @param module the ctx
 * @param module owner of the export
 * @param set set of modules
 * @param export module to append to the set
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_set_append_freestanding_module(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                              FimoModuleLoadingSet *set,
                                                              const FimoModuleExport *export);

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
 * @param export_iterator iterator over all exports of a binary
 * @param binary_handle handle to a resource contained in the module binary
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError
fimo_internal_module_set_append_modules(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set,
                                        const char *module_path, FimoModuleLoadingFilter filter, void *filter_data,
                                        void (*export_iterator)(bool (*)(const FimoModuleExport *, void *), void *),
                                        const void *binary_handle);

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
FIMO_MUST_USE
FimoError fimo_internal_module_set_dismiss(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set);

/**
 * Destroys the module set and loads the modules contained in it.
 *
 * After successfully calling this function, the modules contained
 * in the set are loaded, and their symbols are available to all
 * other modules. This function does not return an error, if it was
 * not able to construct a module. It is not possible to load a module
 * set, while another set is being loaded.
 *
 * @param ctx the context
 * @param set a set of modules to load
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_set_finish(FimoInternalModuleContext *ctx, FimoModuleLoadingSet *set);

/**
 * Searches for a module by it's name.
 *
 * Queries a module by its unique name.
 *
 * @param ctx context
 * @param name module name
 * @param module resulting module.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_find_by_name(FimoInternalModuleContext *ctx, const char *name,
                                            const FimoModuleInfo **module);

/**
 * Searches for a module by a symbol it exports.
 *
 * Queries the module that exported the specified symbol.
 *
 * @param ctx context
 * @param name symbol name
 * @param ns symbol namespace
 * @param version symbol version
 * @param module resulting module
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_find_by_symbol(FimoInternalModuleContext *ctx, const char *name, const char *ns,
                                              FimoVersion version, const FimoModuleInfo **module);

/**
 * Checks for the presence of a namespace in the module backend.
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
FIMO_MUST_USE
FimoError fimo_internal_module_namespace_exists(FimoInternalModuleContext *ctx, const char *ns, bool *exists);

/**
 * Includes a namespace by the module.
 *
 * Once included, the module gains access to the symbols
 * of its dependencies that are exposed in said namespace.
 * A namespace can not be included multiple times.
 *
 * @param ctx context
 * @param module module of the caller
 * @param ns namespace to include
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_namespace_include(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                 const char *ns);

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
 * @param module module of the caller
 * @param ns namespace to exclude.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_namespace_exclude(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                 const char *ns);

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
 * @param module module of the caller
 * @param ns namespace to query
 * @param is_included result of the query
 * @param is_static resulting include type
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_namespace_included(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                  const char *ns, bool *is_included, bool *is_static);

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
 * @param module module of the caller
 * @param dependency module to acquire as a dependency
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_acquire_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                  const FimoModuleInfo *dependency);

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
 * @param module module of the caller
 * @param dependency dependency to remove
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_relinquish_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                     const FimoModuleInfo *dependency);

/**
 * Checks if a module depends on another module.
 *
 * Checks if `other` is a dependency of `module`. In that
 * case `module` is allowed to access the symbols exported
 * by `other`. The result of the query is stored in
 * `has_dependency`. Additionally, this function also
 * queries whether the dependency is static, i.e., the
 * dependency was set by the module backend at load time.
 * The dependency type is stored in `is_static`.
 *
 * @param ctx context
 * @param module module of the caller
 * @param other other module to check as a dependency
 * @param has_dependency result of the query
 * @param is_static resulting dependency type
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_has_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                              const FimoModuleInfo *other, bool *has_dependency, bool *is_static);

/**
 * Loads a symbol from the module backend.
 *
 * The caller can query the backend for a symbol of a loaded
 * module. This is useful for loading optional symbols, or
 * for loading symbols after the creation of a module. The
 * symbol, if it exists, is written into `symbol`, and can
 * be used until the module relinquishes the dependency to
 * the module that exported the symbol. This function fails,
 * if the module containing the symbol is not a dependency
 * of the module.
 *
 * @param ctx context
 * @param module module that requires the symbol
 * @param name symbol name
 * @param ns symbol namespace
 * @param version symbol version
 * @param symbol resulting symbol
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_load_symbol(FimoInternalModuleContext *ctx, const FimoModule *module, const char *name,
                                           const char *ns, FimoVersion version, const FimoModuleRawSymbol **symbol);

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
 * @param module module to unload
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_unload(FimoInternalModuleContext *ctx, const FimoModuleInfo *module);

/**
 * Queries the info of a module parameter.
 *
 * This function can be used to query the datatype, the read access,
 * and the write access of a module parameter. This function fails,
 * if the parameter can not be found.
 *
 * @param ctx context
 * @param module_name name of the module containing the parameter
 * @param param parameter to query
 * @param type queried parameter datatype
 * @param read queried parameter read access
 * @param write queried parameter write access
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_query(FimoInternalModuleContext *ctx, const char *module_name, const char *param,
                                           FimoModuleParamType *type, FimoModuleParamAccess *read,
                                           FimoModuleParamAccess *write);

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
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_internal_module_param_set_public(FimoInternalModuleContext *ctx, const void *value,
                                                              FimoModuleParamType type, const char *module_name,
                                                              const char *param);

/**
 * Reads a module parameter with public read access.
 *
 * Reads the value of a module parameter with public read access.
 * The operation fails, if the parameter does not exist, or if
 * the parameter does not allow reading with a public access.
 * The caller must ensure that `value` points to an instance of
 * the same datatype as the parameter in question.
 *
 * @param ctx the context
 * @param value pointer where to store the value
 * @param type buffer where to store the type of the parameter
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_get_public(FimoInternalModuleContext *ctx, void *value, FimoModuleParamType *type,
                                                const char *module_name, const char *param);

/**
 * Sets a module parameter with dependency write access.
 *
 * Sets the value of a module parameter with dependency write
 * access. The operation fails, if the parameter does not exist,
 * or if the parameter does not allow writing with a dependency
 * access. The caller must ensure that `value` points to an
 * instance of the same datatype as the parameter in question.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value pointer to the value to store
 * @param type type of the value
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_set_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                    const void *value, FimoModuleParamType type,
                                                    const char *module_name, const char *param);

/**
 * Reads a module parameter with dependency read access.
 *
 * Reads the value of a module parameter with dependency read
 * access. The operation fails, if the parameter does not exist,
 * or if the parameter does not allow reading with a dependency
 * access. The caller must ensure that `value` points to an
 * instance of the same datatype as the parameter in question.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value pointer where to store the value
 * @param type buffer where to store the type of the parameter
 * @param module_name name of the module containing the parameter
 * @param param name of the parameter
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_get_dependency(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                    void *value, FimoModuleParamType *type, const char *module_name,
                                                    const char *param);

/**
 * Setter for a module parameter.
 *
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value value to write
 * @param type type of the value
 * @param param parameter to write
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_set_private(FimoInternalModuleContext *ctx, const FimoModule *module,
                                                 const void *value, FimoModuleParamType type, FimoModuleParam *param);

/**
 * Getter for a module parameter.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value buffer where to store the parameter
 * @param type buffer where to store the type of the parameter
 * @param param parameter to load
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_get_private(FimoInternalModuleContext *ctx, const FimoModule *module, void *value,
                                                 FimoModuleParamType *type, const FimoModuleParam *param);

/**
 * Internal setter for a module parameter.
 *
 * If the setter produces an error, the parameter won't be modified.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value value to write
 * @param type type of the value
 * @param param parameter to write
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_set_inner(FimoInternalModuleContext *ctx, const FimoModule *module,
                                               const void *value, FimoModuleParamType type, FimoModuleParamData *param);

/**
 * Internal getter for a module parameter.
 *
 * @param ctx the context
 * @param module module of the caller
 * @param value buffer where to store the parameter
 * @param type buffer where to store the type of the parameter
 * @param param parameter to load
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_internal_module_param_get_inner(FimoInternalModuleContext *ctx, const FimoModule *module, void *value,
                                               FimoModuleParamType *type, const FimoModuleParamData *param);

#ifdef __cplusplus
}
#endif

#endif // FIMO_INTERNAL_MODULE_H
