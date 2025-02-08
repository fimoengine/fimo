#ifndef FIMO_MODULE_H
#define FIMO_MODULE_H

#include <assert.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std/async.h>
#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/path.h>
#include <fimo_std/version.h>

#include <fimo_std/impl/module.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Data type of a module parameter.
typedef enum FimoModuleParamType : FimoI32 {
    FIMO_MODULE_PARAM_TYPE_U8,
    FIMO_MODULE_PARAM_TYPE_U16,
    FIMO_MODULE_PARAM_TYPE_U32,
    FIMO_MODULE_PARAM_TYPE_U64,
    FIMO_MODULE_PARAM_TYPE_I8,
    FIMO_MODULE_PARAM_TYPE_I16,
    FIMO_MODULE_PARAM_TYPE_I32,
    FIMO_MODULE_PARAM_TYPE_I64,
} FimoModuleParamType;

/// Access group for a module parameter.
typedef enum FimoModuleParamAccessGroup : FimoI32 {
    FIMO_MODULE_PARAM_ACCESS_GROUP_PUBLIC,
    FIMO_MODULE_PARAM_ACCESS_GROUP_DEPENDENCY,
    FIMO_MODULE_PARAM_ACCESS_GROUP_PRIVATE,
} FimoModuleParamAccessGroup;

struct FimoModuleParam;

/// VTable of a parameter.
///
/// Adding fields to this struct is not a breaking change.
typedef struct FimoModuleParamVTable {
    /// Returns the value type of the parameter.
    FimoModuleParamType (*type)(const struct FimoModuleParam *param);
    /// Reads the value from the parameter.
    void (*read)(const struct FimoModuleParam *param, void *value);
    /// Writes the value into the parameter.
    void (*write)(const struct FimoModuleParam *param, const void *value);
} FimoModuleParamVTable;

/// A type-erased module parameter.
typedef struct FimoModuleParam {
    FimoModuleParamVTable vtable;
} FimoModuleParam;

/// VTable of a parameter data.
///
/// Adding fields to this struct is not a breaking change.
typedef struct FimoModuleParamDataVTable {
    /// Returns the value type of the parameter.
    FimoModuleParamType (*type)(void *data);
    /// Reads the value from the parameter.
    void (*read)(void *param, void *value);
    /// Writes the value into the parameter.
    void (*write)(void *param, const void *value);
} FimoModuleParamDataVTable;

/// A type-erased internal data type for a module parameter.
typedef struct FimoModuleParamData {
    void *data;
    const FimoModuleParamDataVTable *vtable;
} FimoModuleParamData;

struct FimoModuleInfo;

/// VTable of a FimoModuleInfo.
///
/// Adding fields to the vtable is not a breaking change.
typedef struct FimoModuleInfoVTable {
    /// Increases the reference count of the info instance.
    void (*acquire)(const struct FimoModuleInfo *info);
    /// Decreases the reference count of the info instance.
    void (*release)(const struct FimoModuleInfo *info);
    /// Signals that the module instance may be unloaded.
    ///
    /// The instance will be unloaded once it is no longer actively used by another instance.
    void (*mark_unloadable)(const struct FimoModuleInfo *info);
    /// Returns whether the owning instance is still loaded.
    bool (*is_loaded)(const struct FimoModuleInfo *info);
    /// Tries to increase the strong reference count of the module instance.
    ///
    /// Will prevent the instance from being unloaded. This may be used to pass data, like
    /// callbacks, between instances, without registering the dependency with the subsystem.
    bool (*try_acquire_module_strong)(const struct FimoModuleInfo *info);
    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `try_acquire_module_strong`, when the dependency is no longer
    /// required.
    void (*release_module_strong)(const struct FimoModuleInfo *info);
} FimoModuleInfoVTable;

/// Info of a loaded module.
typedef struct FimoModuleInfo {
    /// Pointer to a possible extension.
    ///
    /// Reserved for future use. Must be `NULL`.
    const FimoBaseStructIn *next;
    /// Module name.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Module description.
    const char *description;
    /// Module author.
    const char *author;
    /// Module license.
    const char *license;
    /// Path to the module directory.
    const char *module_path;
    /// VTable of the info.
    FimoModuleInfoVTable vtable;
} FimoModuleInfo;

typedef struct FimoModuleInstance FimoModuleInstance;

/// VTable of a FimoModuleInstance.
///
/// Adding fields to the vtable is not a breaking change.
typedef struct FimoModuleInstanceVTable {
    /// Increases the strong reference count of the module instance.
    ///
    /// Will prevent the instance from being unloaded. This may be used to pass data, like
    /// callbacks, between instances, without registering the dependency with the subsystem.
    void (*acquire)(struct FimoModuleInstance *ctx);
    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `acquire`, when the dependency is no longer required.
    void (*release)(struct FimoModuleInstance *ctx);
    /// Checks if a module includes a namespace.
    ///
    /// Checks if `module` specified that it includes the namespace `ns`. In that case, the module
    /// is allowed access to the symbols in the namespace. The result of the query is stored in
    /// `has_dependency`. Additionally, this function also queries whether the include is static,
    /// i.e., the include was specified by the module at load time. The include type is stored in
    /// `is_static`.
    FimoResult (*query_namespace)(const FimoModuleInstance* ctx, const char *ns,
                                  bool *has_dependency, bool *is_static);
    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols of its dependencies that are exposed
    /// in said namespace. A namespace can not be included multiple times.
    FimoResult (*add_namespace)(const FimoModuleInstance* ctx, const char *ns);
    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in said
    /// namespace. It is only possible to exclude namespaces that were manually added, whereas
    /// static namespace includes remain valid until the module is unloaded.
    FimoResult (*remove_namespace)(const FimoModuleInstance* ctx, const char *ns);
    /// Checks if a module depends on another module.
    ///
    /// Checks if `info` is a dependency of `module`. In that case `ctx` is allowed to access the
    /// symbols exported by `info`. The result of the query is stored in `has_dependency`.
    /// Additionally, this function also queries whether the dependency is static, i.e., the
    /// dependency was set by the module subsystem at load time. The dependency type is stored in
    /// `is_static`.
    FimoResult (*query_dependency)(const FimoModuleInstance* ctx, const FimoModuleInfo *info,
                                   bool *has_dependency, bool *is_static);
    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols and
    /// protected parameters of said dependency. Trying to acquire a dependency to a module that is
    /// already a dependency, or to a module that would result in a circular dependency will result
    /// in an error.
    FimoResult (*add_dependency)(const FimoModuleInstance* ctx, const FimoModuleInfo *info);
    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any
    /// references to resources originating from the former dependency, and allows for the
    /// unloading of the module. A module can only relinquish dependencies to modules that were
    /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
    FimoResult (*remove_dependency)(const FimoModuleInstance* ctx, const FimoModuleInfo *info);
    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, can be used until the module relinquishes the dependency to the
    /// module that exported the symbol. This function fails, if the module containing the symbol
    /// is not a dependency of the module.
    FimoResult (*load_symbol)(const FimoModuleInstance* ctx, const char *name,
                              const char *ns, FimoVersion version,
                              const void **symbol);
    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow reading with a dependency
    /// access.
    FimoResult (*read_parameter)(void *ctx, void *value,
                                 FimoModuleParamType type, const char *module,
                                 const char *param);
    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow writing with a dependency
    /// access.
    FimoResult (*write_parameter)(void *ctx, const void *value,
                                  FimoModuleParamType type, const char *module,
                                  const char *param);
} FimoModuleInstanceVTable;

/// Opaque type for a parameter table of a module.
///
/// The layout of a parameter table is equivalent to an array of `FimoModuleParam*`, where each
/// entry represents one parameter of the module parameter declaration list.
typedef void FimoModuleParamTable;

/// Opaque type for a resource path table of a module.
///
/// The import table is equivalent to an array of `const char*`, where each entry represents one
/// resource path. The resource paths are ordered in declaration order.
typedef void FimoModuleResourceTable;

/// Opaque type for a symbol import table of a module.
///
/// The import table is equivalent to an array of `const void*`, where each entry represents one
/// symbol of the module symbol import list. The symbols are ordered in declaration order.
typedef void FimoModuleSymbolImportTable;

/// Opaque type for a symbol export table of a module.
///
/// The export table is equivalent to an array of `const void*`, where each entry represents one
/// symbol of the module symbol export list, followed by the entries of the dynamic symbol export
/// list.
typedef void FimoModuleSymbolExportTable;

/// State of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance of
/// `FimoModuleInstance` is valid for as long as the owning module remains loaded. Modules must not
/// leak any resources outside it's own module, ensuring that they are destroyed upon module
/// unloading.
typedef struct FimoModuleInstance {
    /// VTable of the instance.
    const FimoModuleInstanceVTable* vtable;
    /// Module parameter table.
    const FimoModuleParamTable *parameters;
    /// Module resource table.
    const FimoModuleResourceTable *resources;
    /// Module symbol import table.
    const FimoModuleSymbolImportTable *imports;
    /// Module symbol export table.
    const FimoModuleSymbolExportTable *exports;
    /// Module info.
    const FimoModuleInfo *module_info;
    /// Context that loaded the module.
    FimoContext context;
    /// Private data of the module.
    void *module_data;
} FimoModuleInstance;

typedef struct FimoModuleExport FimoModuleExport;

typedef FIMO_ASYNC_ENQUEUED_FUTURE(FimoResult) FimoModuleLoadingSetCommitFuture;

/// Operation of the loading set filter function.
typedef enum FimoModuleLoadingSetFilterRequest : FimoI32 {
    /// Skip the specific module.
    FIMO_MODULE_LOADING_SET_FILTER_SKIP,
    /// Try loading the specific module.
    FIMO_MODULE_LOADING_SET_FILTER_LOAD,
} FimoModuleLoadingSetFilterRequest;

/// VTable of a loading set.
///
/// Adding fields to the VTable is not a breaking change.
typedef struct FimoModuleLoadingSetVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void *ctx);
    /// Decreases the reference count of the instance.
    void (*release)(void *ctx);
    /// Checks whether the set contains a specific module.
    bool (*query_module)(void *ctx, const char *name);
    /// Checks whether the set contains a specific symbol.
    bool (*query_symbol)(void *ctx, const char *name, const char *namespace, FimoVersion version);
    /// Adds a status callback to the set.
    ///
    /// Adds a callback to report a successful or failed loading of a module. The success callback
    /// wil be called if the set was able to load all requested modules, whereas the error callback
    /// will be called immediately after the failed loading of the module. Since the module set can
    /// be in a partially loaded state at the time of calling this function, the error path may be
    /// invoked immediately. The callbacks will be provided with a user-specified data pointer,
    /// which they are in charge of cleaning up. If an error occurs during the execution of the
    /// function, it will invoke the optional `on_abort` callback. If the requested module does not
    /// exist, the function will return an error.
    FimoResult (*add_callback)(void *ctx, const char *name,
                               void (*on_success)(const FimoModuleInfo *info, void *data),
                               void (*on_error)(const FimoModuleExport *exp, void *data),
                               void (*on_abort)(void *data),
                               void *data);
    /// Adds a module to the module set.
    ///
    /// Adds a module to the set, so that it may be loaded by a future call to `commit`. Trying to
    /// include an invalid module, a module with duplicate exports or duplicate name will result in
    /// an error. Unlike `add_modules_from_path`, this function allows for the loading of dynamic
    /// modules, i.e. modules that are created at runtime, like non-native modules, which may
    /// require a runtime to be executed in. The new module inherits a strong reference to the same
    /// binary as the caller's module.
    ///
    /// Note that the new module is not set up to automatically depend on the owner, but may
    /// prevent it from being unloaded while the set exists.
    FimoResult (*add_module)(void *ctx, const FimoModuleInstance *owner, const FimoModuleExport *exp);
    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load. If the path points to a file, the
    /// function will try to load the file as a binary, whereas, if it points to a directory, it
    /// will try to load a file named `module.fimo_module` contained in the directory. Each
    /// exported module is then passed to the filter, along with the provided data, which can then
    /// filter which modules to load. This function may skip invalid module exports. Trying to
    /// include a module with duplicate exports or duplicate name will result in an error. This
    /// function signals an error, if the binary does not contain the symbols necessary to query
    /// the exported modules, but does not return an error, if it does not export any modules. The
    /// necessary symbols are set up automatically, if the binary was linked with the fimo library.
    /// In case of an error, no modules are appended to the set.
    FimoResult (*add_modules_from_path)(void *ctx, FimoUTF8Path path,
                                        FimoModuleLoadingSetFilterRequest (*filter_fn)(
                                            const FimoModuleExport *exp, void *data),
                                        void (*filter_deinit)(void *data),
                                        void *filter_data);
    /// Adds modules to the set.
    ///
    /// Iterates over the exported modules of the current binary. Each exported module is then
    /// passed to the filter, along with the provided data, which can then filter which modules to
    /// load. This function may skip invalid module exports. Trying to include a module with
    /// duplicate exports or duplicate name will result in an error. This function signals an
    /// error, if the binary does not contain the symbols necessary to query the exported modules,
    /// but does not return an error, if it does not export any modules. The necessary symbols are
    /// set up automatically, if the binary was linked with the fimo library. In case of an error,
    /// no modules are appended to the set.
    FimoResult (*add_modules_from_local)(void *ctx,
                                        FimoModuleLoadingSetFilterRequest (*filter_fn)(
                                            const FimoModuleExport *exp, void *data),
                                        void (*filter_deinit)(void *data),
                                        void *filter_data,
                                        void (*iterator_fn)(
                                            bool (*filter_fn)(const FimoModuleExport *exp,
                                                void *data),
                                            void *data),
                                        const void *bin_ptr);
    /// Loads the modules contained in the set.
    ///
    /// If the returned future is successfull, the contained modules and their resources are made
    /// available to the remaining modules. Some conditions may hinder the loading of some module,
    /// like missing dependencies, duplicates, and other loading errors. In those cases, the
    /// modules will be skipped without erroring.
    ///
    /// It is possible to submit multiple concurrent commit requests, even from the same loading
    /// set. In that case, the requests will be handled atomically, in an unspecified order.
    FimoModuleLoadingSetCommitFuture (*commit)(void *ctx);
} FimoModuleLoadingSetVTable;

/// Type-erased set of modules to load by the subsystem.
typedef struct FimoModuleLoadingSet {
    void *data;
    const FimoModuleLoadingSetVTable *vtable;
} FimoModuleLoadingSet;

/// Tag of a debug info type.
typedef enum FimoModuleDebugInfoTypeTag : FimoI32 {
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_VOID,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_BOOL,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_INT,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_FLOAT,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_POINTER,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_ARRAY,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_STRUCT,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_ENUM,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_UNION,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_FN,
    FIMO_MODULE_DEBUG_INFO_TYPE_TAG_OPAQUE,
} FimoModuleDebugInfoTypeTag;

/// Recognized calling conventions.
typedef enum FimoModuleDebugInfoCallingConvention : FimoI32 {
    FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_X86_64_SYSV,
    FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_X86_64_WIN,
    FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS,
    FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_DARWIN,
    FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_WIN,
} FimoModuleDebugInfoCallingConvention;

/// VTable of a `FimoModuleDebugInfoSymbol`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoSymbolVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void* data);
    /// Decreases the reference count of the instance.
    void (*release)(void* data);
    /// Fetches the unique id of the symbol.
    FimoUSize (*get_symbol_id)(void* data);
    /// Fetches the unique id of the symbol type.
    bool (*get_type_id)(void* data, FimoUSize* id);
    /// Fetches the index of the symbol in the module import or export table.
    FimoUSize (*get_table_index)(void* data);
    /// Fetches the index in the respective `FimoModuleExport` array.
    FimoUSize (*get_declaration_index)(void* data);
    /// Checks whether the symbol is an import.
    bool (*is_import)(void* data);
    /// Checks whether the symbol is an export.
    bool (*is_export)(void* data);
    /// Checks whether the symbol is a static export.
    bool (*is_static_export)(void* data);
    /// Checks whether the symbol is a dynamic export.
    bool (*is_dynamic_export)(void* data);
} FimoModuleDebugInfoSymbolVTable;

/// Accessor for the debug info of a symbol.
typedef struct FimoModuleDebugInfoSymbol {
    void *data;
    const FimoModuleDebugInfoSymbolVTable *vtable;
} FimoModuleDebugInfoSymbol;

/// VTable of a `FimoModuleDebugInfoType`.
///
/// Adding fields to the structure **is** considered a breaking change.
typedef struct FimoModuleDebugInfoTypeVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void* data);
    /// Decreases the reference count of the instance.
    void (*release)(void* data);
    /// Fetches the tag of the type.
    FimoModuleDebugInfoTypeTag (*get_type_tag)(void* data);
    /// Fetches the name of the type.
    const char* (*get_name)(void* data);
    /// Reserved for future extensions.
    ///
    /// Must be `NULL`.
    const void* next;
} FimoModuleDebugInfoTypeVTable;

/// Accessor for the debug info of an opaque type.
typedef struct FimoModuleDebugInfoType {
    void *data;
    const FimoModuleDebugInfoTypeVTable *vtable;
} FimoModuleDebugInfoType;

/// VTable of a `FimoModuleDebugInfoVoidType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoVoidTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
} FimoModuleDebugInfoVoidTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoVoidType {
    void *data;
    const FimoModuleDebugInfoVoidTypeVTable *vtable;
} FimoModuleDebugInfoVoidType;

/// VTable of a `FimoModuleDebugInfoBoolType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoBoolTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
} FimoModuleDebugInfoBoolTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoBoolType {
    void *data;
    const FimoModuleDebugInfoBoolTypeVTable *vtable;
} FimoModuleDebugInfoBoolType;

/// VTable of a `FimoModuleDebugInfoIntType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoIntTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Fetches whether the integer type is unsigned.
    bool (*is_unsigned)(void* data);
    /// Fetches whether the integer type is signed.
    bool (*is_signed)(void* data);
    /// Fetches the width of the integer in bits.
    FimoU16 (*get_bitwidth)(void* data);
} FimoModuleDebugInfoIntTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoIntType {
    void *data;
    const FimoModuleDebugInfoIntTypeVTable *vtable;
} FimoModuleDebugInfoIntType;

/// VTable of a `FimoModuleDebugInfoFloatType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoBoolFloatVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Fetches the width of the float in bits.
    FimoU16 (*get_bitwidth)(void* data);
} FimoModuleDebugInfoBoolFloatVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoFloatType {
    void *data;
    const FimoModuleDebugInfoBoolFloatVTable *vtable;
} FimoModuleDebugInfoFloatType;

/// VTable of a `FimoModuleDebugInfoPointerType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoPointerTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Fetches the log of the alignment of the pointee.
    FimoU8 (*get_pointee_alignment)(void* data);
    /// Fetches whether the pointee is constant.
    bool (*is_const)(void* data);
    /// Fetches whether the pointee is volatile.
    bool (*is_volatile)(void* data);
    /// Fetches whether the pointer may not be null.
    bool (*is_nonzero)(void* data);
    /// Fetches the type of the pointee.
    FimoUSize (*get_child_id)(void* data);
} FimoModuleDebugInfoPointerTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoPointerType {
    void *data;
    const FimoModuleDebugInfoPointerTypeVTable *vtable;
} FimoModuleDebugInfoPointerType;

/// VTable of a `FimoModuleDebugInfoArrayType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoArrayTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Fetches the length of the array.
    FimoUSize (*get_length)(void* data);
    /// Fetches the type of the pointee.
    FimoUSize (*get_child_id)(void* data);
} FimoModuleDebugInfoArrayTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoArrayType {
    void *data;
    const FimoModuleDebugInfoArrayTypeVTable *vtable;
} FimoModuleDebugInfoArrayType;

/// VTable of a `FimoModuleDebugInfoStructType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoStructTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Checks whether the structure includes any padding bytes.
    bool (*is_packed_layout)(void* data);
    /// Fetches the number of fields in the structure.
    FimoUSize (*get_field_count)(void* data);
    /// Fetches the name of the field at the index.
    bool (*get_field_name)(void* data, FimoUSize index, const char **name);
    /// Fetches the type of the field at the index.
    bool (*get_field_type_id)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the byte offset to the field.
    bool (*get_field_offset)(void* data, FimoUSize index, FimoUSize *offset);
    /// Fetches the sub-byte offset to the field.
    bool (*get_field_bit_offset)(void* data, FimoUSize index, FimoU8 *offset);
    /// Fetches the log alignment of the field at the index.
    bool (*get_field_alignment)(void* data, FimoUSize index, FimoU8 *alignment);
} FimoModuleDebugInfoStructTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoStructType {
    void *data;
    const FimoModuleDebugInfoStructTypeVTable *vtable;
} FimoModuleDebugInfoStructType;

/// VTable of a `FimoModuleDebugInfoEnumType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoEnumTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Fetches the type of the tag.
    FimoUSize (*get_tag_id)(void* data);
} FimoModuleDebugInfoEnumTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoEnumType {
    void *data;
    const FimoModuleDebugInfoEnumTypeVTable *vtable;
} FimoModuleDebugInfoEnumType;

/// VTable of a `FimoModuleDebugInfoUnionType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoUnionTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void* data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void* data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void* data);
    /// Checks whether the union includes any padding bytes.
    bool (*is_packed_layout)(void* data);
    /// Fetches the number of fields in the union.
    FimoUSize (*get_field_count)(void* data);
    /// Fetches the name of the field at the index.
    bool (*get_field_name)(void* data, FimoUSize index, const char **name);
    /// Fetches the type of the field at the index.
    bool (*get_field_type_id)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the log alignment of the field at the index.
    bool (*get_field_alignment)(void* data, FimoUSize index, FimoU8 *alignment);
} FimoModuleDebugInfoUnionTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoUnionType {
    void *data;
    const FimoModuleDebugInfoUnionTypeVTable *vtable;
} FimoModuleDebugInfoUnionType;

/// VTable of a `FimoModuleDebugInfoFnType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoFnTypeVTable {
    /// Base VTable.
    FimoModuleDebugInfoTypeVTable base;
    /// Checks whether the calling convention of the function is the
    /// default for the C Abi of the target.
    bool (*is_default_calling_convention)(void* data);
    /// Fetches the calling convention of the function.
    bool (*get_calling_convention)(void* data, FimoModuleDebugInfoCallingConvention *cc);
    /// Fetches the alignment of the stack.
    bool (*get_stack_alignment)(void* data, FimoU8 *alignment);
    /// Checks whether the function supports a variable number of arguments.
    bool (*is_var_args)(void* data);
    /// Fetches the type id of the return value.
    FimoUSize (*get_return_type_id)(void* data);
    /// Fetches the number of parameters.
    FimoUSize (*get_parameter_count)(void* data);
    /// Fetches the type id of the parameter.
    bool (*get_parameter_type_id)(void* data, FimoUSize index, FimoUSize *id);
} FimoModuleDebugInfoFnTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModuleDebugInfoFnType {
    void *data;
    const FimoModuleDebugInfoFnTypeVTable *vtable;
} FimoModuleDebugInfoFnType;

/// VTable of a `FimoModuleDebugInfo`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModuleDebugInfoVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void* data);
    /// Decreases the reference count of the instance.
    void (*release)(void* data);
    /// Fetches the number of symbols.
    FimoUSize (*get_symbol_count)(void* data);
    /// Fetches the number of imported symbols.
    FimoUSize (*get_import_symbol_count)(void* data);
    /// Fetches the number of exported symbols.
    FimoUSize (*get_export_symbol_count)(void* data);
    /// Fetches the number of exported static symbols.
    FimoUSize (*get_static_export_symbol_count)(void* data);
    /// Fetches the number of exported dynamic symbols.
    FimoUSize (*get_dynamic_export_symbol_count)(void* data);
    /// Fetches the symbol id for the symbol at the index of the import table.
    bool (*get_symbol_id_by_import_index)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the export table.
    bool (*get_symbol_id_by_export_index)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the static export list.
    bool (*get_symbol_id_by_static_export_index)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the dynamic export list.
    bool (*get_symbol_id_by_dynamic_export_index)(void* data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol with the given id.
    bool (*get_symbol_by_id)(void* data, FimoUSize id, FimoModuleDebugInfoSymbol *symbol);
    /// Fetches the number of contained types.
    FimoUSize (*get_type_count)(void* data);
    /// Fetches the type with the given id.
    bool (*get_type_by_id)(void* data, FimoUSize id, FimoModuleDebugInfoType *type);
} FimoModuleDebugInfoVTable;

/// Accessor for the debug info of a module.
typedef struct FimoModuleDebugInfo {
    void *data;
    const FimoModuleDebugInfoVTable *vtable;
} FimoModuleDebugInfo;

/// Declaration of a module parameter.
typedef struct FimoModuleParamDecl {
    /// Type of the parameter.
    FimoModuleParamType type;
    /// Read access group.
    FimoModuleParamAccessGroup read_group;
    /// Write access group.
    FimoModuleParamAccessGroup write_group;
    /// Optional read function for the parameter.
    ///
    /// Calling into the context may cause a deadlock.
    void (*read)(FimoModuleParamData param, void *value);
    /// Optional write function for the parameter.
    ///
    /// Calling into the context may cause a deadlock.
    void (*write)(FimoModuleParamData param, const void *value);
    /// Name of the parameter.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Default value of the parameter.
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

/// Declaration of a module resource.
typedef struct FimoModuleResourceDecl {
    /// Resource path relative to the module directory.
    ///
    /// Must not be `NULL` or begin with a slash.
    const char *path;
} FimoModuleResourceDecl;

/// Declaration of a module namespace import.
typedef struct FimoModuleNamespaceImport {
    /// Imported namespace.
    ///
    /// Must not be `NULL`.
    const char *name;
} FimoModuleNamespaceImport;

/// Declaration of a module symbol import.
typedef struct FimoModuleSymbolImport {
    /// Symbol version.
    FimoVersion version;
    /// Symbol name.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Symbol namespace.
    ///
    /// Must not be `NULL`.
    const char *ns;
} FimoModuleSymbolImport;

/// Linkage of an symbol export.
typedef enum FimoModuleSymbolLinkage : FimoI32 {
    /// The symbol is visible to other instances and is unique.
    FIMO_MODULE_SYMBOL_LINKAGE_GLOBAL,
} FimoModuleSymbolLinkage;

/// Declaration of a static module symbol export.
typedef struct FimoModuleSymbolExport {
    /// Pointer to the symbol.
    const void *symbol;
    /// Symbol linkage.
    FimoModuleSymbolLinkage linkage;
    /// Symbol version.
    FimoVersion version;
    /// Symbol name.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Symbol namespace.
    ///
    /// Must not be `NULL`.
    const char *ns;
} FimoModuleSymbolExport;

/// Declaration of a dynamic module symbol export.
typedef struct FimoModuleDynamicSymbolExport {
    /// Constructor function for a dynamic symbol.
    ///
    /// The constructor is in charge of constructing an instance of a symbol. To that effect, it is
    /// provided  an instance to the module. The resulting symbol is written into the last argument.
    FimoResult (*constructor)(const FimoModuleInstance *module, void **symbol);
    /// Destructor function for a dynamic symbol.
    ///
    /// The destructor is safe to assume, that the symbol is no longer used by any other module.
    /// During its destruction, a symbol is not allowed to access the module subsystem.
    void (*destructor)(const FimoModuleInstance *module, void *symbol);
    /// Symbol linkage.
    FimoModuleSymbolLinkage linkage;
    /// Symbol version.
    FimoVersion version;
    /// Symbol name.
    ///
    /// Must not be `NULL`.
    const char *name;
    /// Symbol namespace.
    ///
    /// Must not be `NULL`.
    const char *ns;
} FimoModuleDynamicSymbolExport;

/// Valid keys of `FimoModuleExportModifier`.
typedef enum FimoModuleExportModifierKey : FimoI32 {
    /// Specifies that the module export has a destructor function that must be called. The value
    /// must be a pointer to a `FimoModuleExportModifierDestructor`.
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR,
    /// Specifies that the module should acquire a static dependency to another module. The value
    /// must be a strong reference to a `FimoModuleInfo`.
    FIMO_MODULE_EXPORT_MODIFIER_KEY_DEPENDENCY,
    /// Specifies that the module has its debug info embedded. The key may only be specified once
    /// per module. Adds an entry of the type `const FimoModuleDebugInfo*` to the modifiers table
    /// of the module.
    FIMO_MODULE_EXPORT_MODIFIER_DEBUG_INFO,
    /// A constructor and destructor for the state of a module.
    ///
    /// Can be specified to bind a state to an instance. The constructor will be called before the
    /// modules exports are initialized and returning an error will abort the loading of the
    /// instance. Inversely, the destructor function will be called after all exports have been
    /// deinitialized. May only be specified once. Adds an entry of the type
    /// `const FimoModuleExportModifierInstanceState*` to the modifiers table of the module.
    FIMO_MODULE_EXPORT_MODIFIER_INSTANCE_STATE,
    /// A listener for the start event of the instance.
    ///
    /// The event will be dispatched immediately after the instance has been loaded. An error will
    /// result in the destruction of the instance. May only be specified once. Adds an entry of the
    /// type `const FimoModuleExportModifierStartEvent*` to the modifiers table of the module.
    FIMO_MODULE_EXPORT_MODIFIER_START_EVENT,
    /// A listener for the stop event of the instance.
    ///
    /// The event will be dispatched immediately before any exports are deinitialized. May only be
    /// specified once. Adds an entry of the type `const FimoModuleExportModifierStartEvent*` to
    /// the modifiers table of the module.
    FIMO_MODULE_EXPORT_MODIFIER_STOP_EVENT,
} FimoModuleExportModifierKey;

/// A modifier declaration for a module export.
typedef struct FimoModuleExportModifier {
    FimoModuleExportModifierKey key;
    const void *value;
} FimoModuleExportModifier;

/// Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_DESTRUCTOR` modifier key.
typedef struct FimoModuleExportModifierDestructor {
    /// Type-erased data to pass to the destructor.
    void *data;
    /// Destructor function.
    void (*destructor)(void *data);
} FimoModuleExportModifierDestructor;

/// Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_DEBUG_INFO` modifier key.
typedef struct FimoModuleExportModifierDebugInfo {
    /// Type-erased data to pass to the constructor.
    void *data;
    /// Constructor function for the debug info.
    FimoResult (*construct)(void *data, FimoModuleDebugInfo *info);
} FimoModuleExportModifierDebugInfo;

/// Value for the `FIMO_MODULE_EXPORT_MODIFIER_KEY_INSTANCE_STATE` modifier key.
typedef struct FimoModuleExportModifierInstanceState {
    /// Constructor function for a module.
    ///
    /// The module constructor allows a module implementor to initialize some module specific data
    /// at module load time. Some use cases for module constructors are initialization of global
    /// module data, or fetching optional symbols. Returning an error aborts the loading of the
    /// module. Is called before the symbols of the modules are exported/initialized.
    FimoResult (*constructor)(const FimoModuleInstance *module, FimoModuleLoadingSet set, void **state);
    /// Destructor function for a module.
    ///
    /// During its destruction, a module is not allowed to access the module subsystem.
    void (*destructor)(const FimoModuleInstance *module, void *state);
} FimoModuleExportModifierInstanceState;

/// Value for the `FIMO_MODULE_EXPORT_MODIFIER_START_EVENT` modifier key.
typedef struct FimoModuleExportModifierStartEvent {
    /// Function to call once the module has been loaded.
    ///
    /// Implementors of a module can utilize this event to perform arbitrary an arbitrary action
    /// once the module has been loaded. If the call returns an error, the module will be unloaded.
    FimoResult (*on_event)(const FimoModuleInstance *module);
} FimoModuleExportModifierStartEvent;

/// Value for the `FIMO_MODULE_EXPORT_MODIFIER_STOP_EVENT` modifier key.
typedef struct FimoModuleExportModifierStopEvent {
    /// Optional function to call before the module is unloaded.
    ///
    /// May be used to finalize the module, before any symbols or state is unloaded.
    void (*on_event)(const FimoModuleInstance *module);
} FimoModuleExportModifierStopEvent;

/// Declaration of a module export.
struct FimoModuleExport {
    /// Pointer to a possible extension.
    ///
    /// Reserved for future use. Must be `NULL`.
    const FimoBaseStructIn *next;
    /// Version of the context compiled against.
    FimoVersion version;
    /// Module name.
    ///
    /// The module name must be unique to the module.
    /// Must not be `NULL`.
    const char *name;
    /// Module description.
    const char *description;
    /// Module author.
    const char *author;
    /// Module license.
    const char *license;
    /// List of parameters exposed by the module.
    ///
    /// A module is not allowed to expose duplicate parameters.
    const FimoModuleParamDecl *parameters;
    /// Number of parameters exposed by the module.
    FimoUSize parameters_count;
    /// List of resources declared by the module.
    const FimoModuleResourceDecl *resources;
    /// Number of resources declared by the module.
    FimoUSize resources_count;
    /// List of namespaces to import by the module.
    ///
    /// A module is only allowed to import and export symbols from/to an imported namespace. It is
    /// an error to specify a namespace, that does not exist, without exporting to that namespace.
    const FimoModuleNamespaceImport *namespace_imports;
    /// Number of namespaces to import by the module.
    FimoUSize namespace_imports_count;
    /// List of symbols to import by the module.
    ///
    /// Upon loading, the module is provided the listed symbols. If some symbols are not available,
    /// the loading fails.
    const FimoModuleSymbolImport *symbol_imports;
    /// Number of symbols to import by the module.
    FimoUSize symbol_imports_count;
    /// List of static symbols exported by the module.
    ///
    /// The named symbols will be made available to all other modules. Trying to export a duplicate
    /// symbol will result in an error upon loading of the module.
    const FimoModuleSymbolExport *symbol_exports;
    /// Number of static symbols exported by the module.
    FimoUSize symbol_exports_count;
    /// List of dynamic symbols exported by the module.
    ///
    /// A dynamic symbol is a symbol, whose creation is deferred until loading of the module. This
    /// is useful, in case the symbol depends on the module imports.
    const FimoModuleDynamicSymbolExport *dynamic_symbol_exports;
    /// Number of dynamic symbols exported by the module.
    FimoUSize dynamic_symbol_exports_count;
    /// List of modifier key-value pairs for the exported module.
    const FimoModuleExportModifier *modifiers;
    /// Number of modifiers for the module.
    FimoUSize modifiers_count;
};

/// A filter for selection modules to load by the module subsystem.
///
/// The filter function is passed the module export declaration and can then decide whether the
/// module should be loaded by the subsystem.
typedef bool (*FimoModuleLoadingFilter)(const FimoModuleExport *arg0, void *arg1);

/// A callback for successfully loading a module.
///
/// The callback function is called when the subsystem was successful in loading the requested
/// module, making it then possible to request symbols.
typedef void (*FimoModuleLoadingSuccessCallback)(const FimoModuleInfo *arg0, void *arg1);

/// A callback for a module loading error.
///
/// The callback function is called when the subsystem was not successful in loading the requested
/// module.
typedef void (*FimoModuleLoadingErrorCallback)(const FimoModuleExport *arg0, void *arg1);

/// VTable of the module subsystem.
///
/// Changing the VTable is a breaking change.
typedef struct FimoModuleVTableV0 {
    /// Constructs a new pseudo module.
    ///
    /// The functions of the module subsystem require that the caller owns a reference to their own
    /// module. This is a problem, as the constructor of the context won't be assigned a module
    /// instance during bootstrapping. As a workaround, we allow for the creation of pseudo
    /// modules, i.e., module handles without an associated module.
    FimoResult (*pseudo_module_new)(void *ctx, const FimoModuleInstance **module);
    /// Constructs a new empty set.
    ///
    /// Modules can only be loaded, if all of their dependencies can be resolved, which requires us
    /// to determine a suitable load order. A loading set is a utility to facilitate this process,
    /// by automatically computing a suitable load order for a batch of modules.
    FimoResult (*set_new)(void *ctx, FimoModuleLoadingSet *set);
    /// Searches for a module by it's name.
    ///
    /// Queries a module by its unique name. The returned `FimoModuleInfo` will have its reference
    /// count increased.
    FimoResult (*find_by_name)(void *ctx, const char *name,
                               const FimoModuleInfo **info);
    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol. The returned `FimoModuleInfo` will
    /// have its reference count increased.
    FimoResult (*find_by_symbol)(void *ctx, const char *name, const char *ns,
                                 FimoVersion version, const FimoModuleInfo **info);
    /// Checks for the presence of a namespace in the module subsystem.
    ///
    /// A namespace exists, if at least one loaded module exports one symbol in said namespace.
    FimoResult (*namespace_exists)(void *ctx, const char *ns, bool *exists);
    /// Unloads all unused instances.
    ///
    /// After calling this function, all unreferenced instances are unloaded.
    FimoResult (*prune_instances)(void *ctx);
    /// Queries the info of a module parameter.
    ///
    /// This function can be used to query the datatype, the read access, and the write access of a
    /// module parameter. This function fails, if the parameter can not be found.
    FimoResult (*query_parameter)(void *ctx, const char *module,
                                  const char *param, FimoModuleParamType *type,
                                  FimoModuleParamAccessGroup *read_group,
                                  FimoModuleParamAccessGroup *write_group);
    /// Reads a module parameter with public read access.
    ///
    /// Reads the value of a module parameter with public read access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow reading with a public access.
    /// The caller must ensure that `value` points to an instance of the same datatype as the
    /// parameter in question.
    FimoResult (*read_parameter)(void *ctx, void *value,
                                 FimoModuleParamType type, const char *module,
                                 const char *param);
    /// Sets a module parameter with public write access.
    ///
    /// Sets the value of a module parameter with public write access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow writing with a public access.
    /// The caller must ensure that `value` points to an instance of the same datatype as the
    /// parameter in question.
    FimoResult (*write_parameter)(void *ctx, const void *value,
                                  FimoModuleParamType type, const char *module,
                                  const char *param);
} FimoModuleVTableV0;

#ifdef __cplusplus
}
#endif

#endif // FIMO_MODULE_H
