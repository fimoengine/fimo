#ifndef FIMO_MODULES_H
#define FIMO_MODULES_H

#include <assert.h>
#include <stdalign.h>
#include <stdatomic.h>
#include <stdbool.h>

#include <fimo_std/context.h>
#include <fimo_std/error.h>
#include <fimo_std/path.h>
#include <fimo_std/tasks.h>
#include <fimo_std/version.h>

#include <fimo_std/impl/module.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Data type of a module parameter.
typedef enum FimoModulesParamType : FimoI32 {
    FIMO_MODULES_PARAM_TYPE_U8,
    FIMO_MODULES_PARAM_TYPE_U16,
    FIMO_MODULES_PARAM_TYPE_U32,
    FIMO_MODULES_PARAM_TYPE_U64,
    FIMO_MODULES_PARAM_TYPE_I8,
    FIMO_MODULES_PARAM_TYPE_I16,
    FIMO_MODULES_PARAM_TYPE_I32,
    FIMO_MODULES_PARAM_TYPE_I64,
} FimoModulesParamType;

/// Access group for a module parameter.
typedef enum FimoModulesParamAccessGroup : FimoI32 {
    FIMO_MODULES_PARAM_ACCESS_GROUP_PUBLIC,
    FIMO_MODULES_PARAM_ACCESS_GROUP_DEPENDENCY,
    FIMO_MODULES_PARAM_ACCESS_GROUP_PRIVATE,
} FimoModulesParamAccessGroup;

struct FimoModulesParam;

/// VTable of a parameter.
///
/// Adding fields to this struct is not a breaking change.
typedef struct FimoModulesParamVTable {
    /// Returns the value type of the parameter.
    FimoModulesParamType (*type)(const struct FimoModulesParam *param);
    /// Reads the value from the parameter.
    void (*read)(const struct FimoModulesParam *param, void *value);
    /// Writes the value into the parameter.
    void (*write)(const struct FimoModulesParam *param, const void *value);
} FimoModulesParamVTable;

/// A type-erased module parameter.
typedef struct FimoModulesParam {
    FimoModulesParamVTable vtable;
} FimoModulesParam;

/// VTable of a parameter data.
///
/// Adding fields to this struct is not a breaking change.
typedef struct FimoModulesParamDataVTable {
    /// Returns the value type of the parameter.
    FimoModulesParamType (*type)(void *data);
    /// Reads the value from the parameter.
    void (*read)(void *param, void *value);
    /// Writes the value into the parameter.
    void (*write)(void *param, const void *value);
} FimoModulesParamDataVTable;

/// A type-erased internal data type for a module parameter.
typedef struct FimoModulesParamData {
    void *data;
    const FimoModulesParamDataVTable *vtable;
} FimoModulesParamData;

struct FimoModulesInfo;

/// VTable of a FimoModulesInfo.
///
/// Adding fields to the vtable is not a breaking change.
typedef struct FimoModulesInfoVTable {
    /// Increases the reference count of the info instance.
    void (*acquire)(const struct FimoModulesInfo *info);
    /// Decreases the reference count of the info instance.
    void (*release)(const struct FimoModulesInfo *info);
    /// Signals that the module instance may be unloaded.
    ///
    /// The instance will be unloaded once it is no longer actively used by another instance.
    void (*mark_unloadable)(const struct FimoModulesInfo *info);
    /// Returns whether the owning instance is still loaded.
    bool (*is_loaded)(const struct FimoModulesInfo *info);
    /// Tries to increase the strong reference count of the module instance.
    ///
    /// Will prevent the instance from being unloaded. This may be used to pass data, like
    /// callbacks, between instances, without registering the dependency with the subsystem.
    bool (*try_acquire_module_strong)(const struct FimoModulesInfo *info);
    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `try_acquire_module_strong`, when the dependency is no longer
    /// required.
    void (*release_module_strong)(const struct FimoModulesInfo *info);
} FimoModulesInfoVTable;

/// Info of a loaded module.
typedef struct FimoModulesInfo {
    /// Pointer to a possible extension.
    ///
    /// Reserved for future use. Must be `NULL`.
    const void *next;
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
    FimoModulesInfoVTable vtable;
} FimoModulesInfo;

typedef struct FimoModulesInstance FimoModulesInstance;

/// VTable of a FimoModulesInstance.
///
/// Adding fields to the vtable is not a breaking change.
typedef struct FimoModulesInstanceVTable {
    /// Increases the strong reference count of the module instance.
    ///
    /// Will prevent the instance from being unloaded. This may be used to pass data, like
    /// callbacks, between instances, without registering the dependency with the subsystem.
    void (*acquire)(struct FimoModulesInstance *ctx);
    /// Decreases the strong reference count of the module instance.
    ///
    /// Should only be called after `acquire`, when the dependency is no longer required.
    void (*release)(struct FimoModulesInstance *ctx);
    /// Checks if a module includes a namespace.
    ///
    /// Checks if `module` specified that it includes the namespace `ns`. In that case, the module
    /// is allowed access to the symbols in the namespace. The result of the query is stored in
    /// `has_dependency`. Additionally, this function also queries whether the include is static,
    /// i.e., the include was specified by the module at load time. The include type is stored in
    /// `is_static`.
    FimoResult (*query_namespace)(const FimoModulesInstance *ctx, const char *ns, bool *has_dependency,
                                  bool *is_static);
    /// Includes a namespace by the module.
    ///
    /// Once included, the module gains access to the symbols of its dependencies that are exposed
    /// in said namespace. A namespace can not be included multiple times.
    FimoResult (*add_namespace)(const FimoModulesInstance *ctx, const char *ns);
    /// Removes a namespace include from the module.
    ///
    /// Once excluded, the caller guarantees to relinquish access to the symbols contained in said
    /// namespace. It is only possible to exclude namespaces that were manually added, whereas
    /// static namespace includes remain valid until the module is unloaded.
    FimoResult (*remove_namespace)(const FimoModulesInstance *ctx, const char *ns);
    /// Checks if a module depends on another module.
    ///
    /// Checks if `info` is a dependency of `module`. In that case `ctx` is allowed to access the
    /// symbols exported by `info`. The result of the query is stored in `has_dependency`.
    /// Additionally, this function also queries whether the dependency is static, i.e., the
    /// dependency was set by the module subsystem at load time. The dependency type is stored in
    /// `is_static`.
    FimoResult (*query_dependency)(const FimoModulesInstance *ctx, const FimoModulesInfo *info, bool *has_dependency,
                                   bool *is_static);
    /// Acquires another module as a dependency.
    ///
    /// After acquiring a module as a dependency, the module is allowed access to the symbols and
    /// protected parameters of said dependency. Trying to acquire a dependency to a module that is
    /// already a dependency, or to a module that would result in a circular dependency will result
    /// in an error.
    FimoResult (*add_dependency)(const FimoModulesInstance *ctx, const FimoModulesInfo *info);
    /// Removes a module as a dependency.
    ///
    /// By removing a module as a dependency, the caller ensures that it does not own any
    /// references to resources originating from the former dependency, and allows for the
    /// unloading of the module. A module can only relinquish dependencies to modules that were
    /// acquired dynamically, as static dependencies remain valid until the module is unloaded.
    FimoResult (*remove_dependency)(const FimoModulesInstance *ctx, const FimoModulesInfo *info);
    /// Loads a symbol from the module subsystem.
    ///
    /// The caller can query the subsystem for a symbol of a loaded module. This is useful for
    /// loading optional symbols, or for loading symbols after the creation of a module. The
    /// symbol, if it exists, can be used until the module relinquishes the dependency to the
    /// module that exported the symbol. This function fails, if the module containing the symbol
    /// is not a dependency of the module.
    FimoResult (*load_symbol)(const FimoModulesInstance *ctx, const char *name, const char *ns, FimoVersion version,
                              const void **symbol);
    /// Reads a module parameter with dependency read access.
    ///
    /// Reads the value of a module parameter with dependency read access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow reading with a dependency
    /// access.
    FimoResult (*read_parameter)(void *ctx, void *value, FimoModulesParamType type, const char *module,
                                 const char *param);
    /// Sets a module parameter with dependency write access.
    ///
    /// Sets the value of a module parameter with dependency write access. The operation fails, if
    /// the parameter does not exist, or if the parameter does not allow writing with a dependency
    /// access.
    FimoResult (*write_parameter)(void *ctx, const void *value, FimoModulesParamType type, const char *module,
                                  const char *param);
} FimoModulesInstanceVTable;

/// Opaque type for a parameter table of a module.
///
/// The layout of a parameter table is equivalent to an array of `FimoModulesParam*`, where each
/// entry represents one parameter of the module parameter declaration list.
typedef void FimoModulesParamTable;

/// Opaque type for a resource path table of a module.
///
/// The import table is equivalent to an array of `FimoUTF8Path`, where each entry represents one
/// resource path. Additionally, each path is null-terminated. The resource paths are ordered in
/// declaration order.
typedef void FimoModulesResourceTable;

/// Opaque type for a symbol import table of a module.
///
/// The import table is equivalent to an array of `const void*`, where each entry represents one
/// symbol of the module symbol import list. The symbols are ordered in declaration order.
typedef void FimoModulesSymbolImportTable;

/// Opaque type for a symbol export table of a module.
///
/// The export table is equivalent to an array of `const void*`, where each entry represents one
/// symbol of the module symbol export list, followed by the entries of the dynamic symbol export
/// list.
typedef void FimoModulesSymbolExportTable;

/// State of a loaded module.
///
/// A module is self-contained, and may not be passed to other modules. An instance of
/// `FimoModulesInstance` is valid for as long as the owning module remains loaded. Modules must not
/// leak any resources outside it's own module, ensuring that they are destroyed upon module
/// unloading.
typedef struct FimoModulesInstance {
    /// VTable of the instance.
    const FimoModulesInstanceVTable *vtable;
    /// Module parameter table.
    const FimoModulesParamTable *parameters;
    /// Module resource table.
    const FimoModulesResourceTable *resources;
    /// Module symbol import table.
    const FimoModulesSymbolImportTable *imports;
    /// Module symbol export table.
    const FimoModulesSymbolExportTable *exports;
    /// Module info.
    const FimoModulesInfo *module_info;
    /// Context that loaded the module.
    const FimoContextHandle *handle;
    /// Private data of the module.
    void *module_data;
} FimoModulesInstance;

typedef struct FimoModulesExport FimoModulesExport;

typedef FIMO_TASKS_ENQUEUED_FUTURE(FimoResult) FimoModulesLoadingSetCommitFuture;

/// Operation of the loading set filter function.
typedef enum FimoModulesLoadingSetFilterRequest : FimoI32 {
    /// Skip the specific module.
    FIMO_MODULES_LOADING_SET_FILTER_SKIP,
    /// Try loading the specific module.
    FIMO_MODULES_LOADING_SET_FILTER_LOAD,
} FimoModulesLoadingSetFilterRequest;

/// VTable of a loading set.
///
/// Adding fields to the VTable is not a breaking change.
typedef struct FimoModulesLoadingSetVTable {
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
    FimoResult (*add_callback)(void *ctx, const char *name, void (*on_success)(const FimoModulesInfo *info, void *data),
                               void (*on_error)(const FimoModulesExport *exp, void *data), void (*on_abort)(void *data),
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
    FimoResult (*add_module)(void *ctx, const FimoModulesInstance *owner, const FimoModulesExport *exp);
    /// Adds modules to the set.
    ///
    /// Opens up a module binary to select which modules to load. If the path points to a file, the
    /// function will try to load the file as a binary, whereas, if it points to a directory, it
    /// will try to load a file named `module.FIMO_MODULES` contained in the directory. Each
    /// exported module is then passed to the filter, along with the provided data, which can then
    /// filter which modules to load. This function may skip invalid module exports. Trying to
    /// include a module with duplicate exports or duplicate name will result in an error. This
    /// function signals an error, if the binary does not contain the symbols necessary to query
    /// the exported modules, but does not return an error, if it does not export any modules. The
    /// necessary symbols are set up automatically, if the binary was linked with the fimo library.
    /// In case of an error, no modules are appended to the set.
    FimoResult (*add_modules_from_path)(void *ctx, FimoUTF8Path path,
                                        FimoModulesLoadingSetFilterRequest (*filter_fn)(const FimoModulesExport *exp,
                                                                                        void *data),
                                        void (*filter_deinit)(void *data), void *filter_data);
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
    FimoResult (*add_modules_from_local)(
            void *ctx, FimoModulesLoadingSetFilterRequest (*filter_fn)(const FimoModulesExport *exp, void *data),
            void (*filter_deinit)(void *data), void *filter_data,
            void (*iterator_fn)(bool (*filter_fn)(const FimoModulesExport *exp, void *data), void *data),
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
    FimoModulesLoadingSetCommitFuture (*commit)(void *ctx);
} FimoModulesLoadingSetVTable;

/// Type-erased set of modules to load by the subsystem.
typedef struct FimoModulesLoadingSet {
    void *data;
    const FimoModulesLoadingSetVTable *vtable;
} FimoModulesLoadingSet;

/// Tag of a debug info type.
typedef enum FimoModulesDebugInfoTypeTag : FimoI32 {
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_VOID,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_BOOL,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_INT,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_FLOAT,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_POINTER,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_ARRAY,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_STRUCT,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_ENUM,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_UNION,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_FN,
    FIMO_MODULES_DEBUG_INFO_TYPE_TAG_OPAQUE,
} FimoModulesDebugInfoTypeTag;

/// Recognized calling conventions.
typedef enum FimoModulesDebugInfoCallingConvention : FimoI32 {
    FIMO_MODULES_DEBUG_INFO_CALLING_CONVENTION_X86_64_SYSV,
    FIMO_MODULES_DEBUG_INFO_CALLING_CONVENTION_X86_64_WIN,
    FIMO_MODULES_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS,
    FIMO_MODULES_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_DARWIN,
    FIMO_MODULES_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_WIN,
} FimoModulesDebugInfoCallingConvention;

/// VTable of a `FimoModulesDebugInfoSymbol`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoSymbolVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void *data);
    /// Decreases the reference count of the instance.
    void (*release)(void *data);
    /// Fetches the unique id of the symbol.
    FimoUSize (*get_symbol_id)(void *data);
    /// Fetches the unique id of the symbol type.
    bool (*get_type_id)(void *data, FimoUSize *id);
    /// Fetches the index of the symbol in the module import or export table.
    FimoUSize (*get_table_index)(void *data);
    /// Fetches the index in the respective `FimoModulesExport` array.
    FimoUSize (*get_declaration_index)(void *data);
    /// Checks whether the symbol is an import.
    bool (*is_import)(void *data);
    /// Checks whether the symbol is an export.
    bool (*is_export)(void *data);
    /// Checks whether the symbol is a static export.
    bool (*is_static_export)(void *data);
    /// Checks whether the symbol is a dynamic export.
    bool (*is_dynamic_export)(void *data);
} FimoModulesDebugInfoSymbolVTable;

/// Accessor for the debug info of a symbol.
typedef struct FimoModulesDebugInfoSymbol {
    void *data;
    const FimoModulesDebugInfoSymbolVTable *vtable;
} FimoModulesDebugInfoSymbol;

/// VTable of a `FimoModulesDebugInfoType`.
///
/// Adding fields to the structure **is** considered a breaking change.
typedef struct FimoModulesDebugInfoTypeVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void *data);
    /// Decreases the reference count of the instance.
    void (*release)(void *data);
    /// Fetches the tag of the type.
    FimoModulesDebugInfoTypeTag (*get_type_tag)(void *data);
    /// Fetches the name of the type.
    const char *(*get_name)(void *data);
    /// Reserved for future extensions.
    ///
    /// Must be `NULL`.
    const void *next;
} FimoModulesDebugInfoTypeVTable;

/// Accessor for the debug info of an opaque type.
typedef struct FimoModulesDebugInfoType {
    void *data;
    const FimoModulesDebugInfoTypeVTable *vtable;
} FimoModulesDebugInfoType;

/// VTable of a `FimoModulesDebugInfoVoidType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoVoidTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
} FimoModulesDebugInfoVoidTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoVoidType {
    void *data;
    const FimoModulesDebugInfoVoidTypeVTable *vtable;
} FimoModulesDebugInfoVoidType;

/// VTable of a `FimoModulesDebugInfoBoolType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoBoolTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
} FimoModulesDebugInfoBoolTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoBoolType {
    void *data;
    const FimoModulesDebugInfoBoolTypeVTable *vtable;
} FimoModulesDebugInfoBoolType;

/// VTable of a `FimoModulesDebugInfoIntType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoIntTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Fetches whether the integer type is unsigned.
    bool (*is_unsigned)(void *data);
    /// Fetches whether the integer type is signed.
    bool (*is_signed)(void *data);
    /// Fetches the width of the integer in bits.
    FimoU16 (*get_bitwidth)(void *data);
} FimoModulesDebugInfoIntTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoIntType {
    void *data;
    const FimoModulesDebugInfoIntTypeVTable *vtable;
} FimoModulesDebugInfoIntType;

/// VTable of a `FimoModulesDebugInfoFloatType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoBoolFloatVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Fetches the width of the float in bits.
    FimoU16 (*get_bitwidth)(void *data);
} FimoModulesDebugInfoBoolFloatVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoFloatType {
    void *data;
    const FimoModulesDebugInfoBoolFloatVTable *vtable;
} FimoModulesDebugInfoFloatType;

/// VTable of a `FimoModulesDebugInfoPointerType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoPointerTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Fetches the log of the alignment of the pointee.
    FimoU8 (*get_pointee_alignment)(void *data);
    /// Fetches whether the pointee is constant.
    bool (*is_const)(void *data);
    /// Fetches whether the pointee is volatile.
    bool (*is_volatile)(void *data);
    /// Fetches whether the pointer may not be null.
    bool (*is_nonzero)(void *data);
    /// Fetches the type of the pointee.
    FimoUSize (*get_child_id)(void *data);
} FimoModulesDebugInfoPointerTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoPointerType {
    void *data;
    const FimoModulesDebugInfoPointerTypeVTable *vtable;
} FimoModulesDebugInfoPointerType;

/// VTable of a `FimoModulesDebugInfoArrayType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoArrayTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Fetches the length of the array.
    FimoUSize (*get_length)(void *data);
    /// Fetches the type of the pointee.
    FimoUSize (*get_child_id)(void *data);
} FimoModulesDebugInfoArrayTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoArrayType {
    void *data;
    const FimoModulesDebugInfoArrayTypeVTable *vtable;
} FimoModulesDebugInfoArrayType;

/// VTable of a `FimoModulesDebugInfoStructType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoStructTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Checks whether the structure includes any padding bytes.
    bool (*is_packed_layout)(void *data);
    /// Fetches the number of fields in the structure.
    FimoUSize (*get_field_count)(void *data);
    /// Fetches the name of the field at the index.
    bool (*get_field_name)(void *data, FimoUSize index, const char **name);
    /// Fetches the type of the field at the index.
    bool (*get_field_type_id)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the byte offset to the field.
    bool (*get_field_offset)(void *data, FimoUSize index, FimoUSize *offset);
    /// Fetches the sub-byte offset to the field.
    bool (*get_field_bit_offset)(void *data, FimoUSize index, FimoU8 *offset);
    /// Fetches the log alignment of the field at the index.
    bool (*get_field_alignment)(void *data, FimoUSize index, FimoU8 *alignment);
} FimoModulesDebugInfoStructTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoStructType {
    void *data;
    const FimoModulesDebugInfoStructTypeVTable *vtable;
} FimoModulesDebugInfoStructType;

/// VTable of a `FimoModulesDebugInfoEnumType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoEnumTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Fetches the type of the tag.
    FimoUSize (*get_tag_id)(void *data);
} FimoModulesDebugInfoEnumTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoEnumType {
    void *data;
    const FimoModulesDebugInfoEnumTypeVTable *vtable;
} FimoModulesDebugInfoEnumType;

/// VTable of a `FimoModulesDebugInfoUnionType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoUnionTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Fetches the size of the type in full bytes.
    FimoUSize (*get_size)(void *data);
    /// Fetches the sub-byte size of the type.
    FimoU8 (*get_bit_size)(void *data);
    /// Fetches the log of the type alignment.
    FimoU8 (*get_alignment)(void *data);
    /// Checks whether the union includes any padding bytes.
    bool (*is_packed_layout)(void *data);
    /// Fetches the number of fields in the union.
    FimoUSize (*get_field_count)(void *data);
    /// Fetches the name of the field at the index.
    bool (*get_field_name)(void *data, FimoUSize index, const char **name);
    /// Fetches the type of the field at the index.
    bool (*get_field_type_id)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the log alignment of the field at the index.
    bool (*get_field_alignment)(void *data, FimoUSize index, FimoU8 *alignment);
} FimoModulesDebugInfoUnionTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoUnionType {
    void *data;
    const FimoModulesDebugInfoUnionTypeVTable *vtable;
} FimoModulesDebugInfoUnionType;

/// VTable of a `FimoModulesDebugInfoFnType`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoFnTypeVTable {
    /// Base VTable.
    FimoModulesDebugInfoTypeVTable base;
    /// Checks whether the calling convention of the function is the
    /// default for the C Abi of the target.
    bool (*is_default_calling_convention)(void *data);
    /// Fetches the calling convention of the function.
    bool (*get_calling_convention)(void *data, FimoModulesDebugInfoCallingConvention *cc);
    /// Fetches the alignment of the stack.
    bool (*get_stack_alignment)(void *data, FimoU8 *alignment);
    /// Checks whether the function supports a variable number of arguments.
    bool (*is_var_args)(void *data);
    /// Fetches the type id of the return value.
    FimoUSize (*get_return_type_id)(void *data);
    /// Fetches the number of parameters.
    FimoUSize (*get_parameter_count)(void *data);
    /// Fetches the type id of the parameter.
    bool (*get_parameter_type_id)(void *data, FimoUSize index, FimoUSize *id);
} FimoModulesDebugInfoFnTypeVTable;

/// Accessor for the debug info of a `void` type.
typedef struct FimoModulesDebugInfoFnType {
    void *data;
    const FimoModulesDebugInfoFnTypeVTable *vtable;
} FimoModulesDebugInfoFnType;

/// VTable of a `FimoModulesDebugInfo`.
///
/// Adding fields to the structure is not considered a breaking change.
typedef struct FimoModulesDebugInfoVTable {
    /// Increases the reference count of the instance.
    void (*acquire)(void *data);
    /// Decreases the reference count of the instance.
    void (*release)(void *data);
    /// Fetches the number of symbols.
    FimoUSize (*get_symbol_count)(void *data);
    /// Fetches the number of imported symbols.
    FimoUSize (*get_import_symbol_count)(void *data);
    /// Fetches the number of exported symbols.
    FimoUSize (*get_export_symbol_count)(void *data);
    /// Fetches the number of exported static symbols.
    FimoUSize (*get_static_export_symbol_count)(void *data);
    /// Fetches the number of exported dynamic symbols.
    FimoUSize (*get_dynamic_export_symbol_count)(void *data);
    /// Fetches the symbol id for the symbol at the index of the import table.
    bool (*get_symbol_id_by_import_index)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the export table.
    bool (*get_symbol_id_by_export_index)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the static export list.
    bool (*get_symbol_id_by_static_export_index)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol id for the symbol at the index of the dynamic export list.
    bool (*get_symbol_id_by_dynamic_export_index)(void *data, FimoUSize index, FimoUSize *id);
    /// Fetches the symbol with the given id.
    bool (*get_symbol_by_id)(void *data, FimoUSize id, FimoModulesDebugInfoSymbol *symbol);
    /// Fetches the number of contained types.
    FimoUSize (*get_type_count)(void *data);
    /// Fetches the type with the given id.
    bool (*get_type_by_id)(void *data, FimoUSize id, FimoModulesDebugInfoType *type);
} FimoModulesDebugInfoVTable;

/// Accessor for the debug info of a module.
typedef struct FimoModulesDebugInfo {
    void *data;
    const FimoModulesDebugInfoVTable *vtable;
} FimoModulesDebugInfo;

/// Declaration of a module parameter.
typedef struct FimoModulesParamDecl {
    /// Type of the parameter.
    FimoModulesParamType type;
    /// Read access group.
    FimoModulesParamAccessGroup read_group;
    /// Write access group.
    FimoModulesParamAccessGroup write_group;
    /// Optional read function for the parameter.
    ///
    /// Calling into the context may cause a deadlock.
    void (*read)(FimoModulesParamData param, void *value);
    /// Optional write function for the parameter.
    ///
    /// Calling into the context may cause a deadlock.
    void (*write)(FimoModulesParamData param, const void *value);
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
} FimoModulesParamDecl;

/// Declaration of a module resource.
typedef struct FimoModulesResourceDecl {
    /// Resource path relative to the module directory.
    ///
    /// Must not be `NULL` or begin with a slash.
    const char *path;
} FimoModulesResourceDecl;

/// Declaration of a module namespace import.
typedef struct FimoModulesNamespaceImport {
    /// Imported namespace.
    ///
    /// Must not be `NULL`.
    const char *name;
} FimoModulesNamespaceImport;

/// Declaration of a module symbol import.
typedef struct FimoModulesSymbolImport {
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
} FimoModulesSymbolImport;

/// Linkage of an symbol export.
typedef enum FimoModulesSymbolLinkage : FimoI32 {
    /// The symbol is visible to other instances and is unique.
    FIMO_MODULES_SYMBOL_LINKAGE_GLOBAL,
} FimoModulesSymbolLinkage;

/// Declaration of a static module symbol export.
typedef struct FimoModulesSymbolExport {
    /// Pointer to the symbol.
    const void *symbol;
    /// Symbol linkage.
    FimoModulesSymbolLinkage linkage;
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
} FimoModulesSymbolExport;

typedef FIMO_TASKS_FALLIBLE(void *) FimoModulesDynamicSymbolExportFutureResult;
typedef FIMO_TASKS_ENQUEUED_FUTURE(FimoModulesDynamicSymbolExportFutureResult) FimoModulesDynamicSymbolExportFuture;

/// Declaration of a dynamic module symbol export.
typedef struct FimoModulesDynamicSymbolExport {
    /// Constructor function for a dynamic symbol.
    ///
    /// The constructor is in charge of constructing an instance of a symbol. To that effect, it is
    /// provided  an instance to the module.
    FimoModulesDynamicSymbolExportFuture (*constructor)(const FimoModulesInstance *module);
    /// Destructor function for a dynamic symbol.
    ///
    /// The destructor is safe to assume, that the symbol is no longer used by any other module.
    /// During its destruction, a symbol is not allowed to access the module subsystem.
    void (*destructor)(const FimoModulesInstance *module, void *symbol);
    /// Symbol linkage.
    FimoModulesSymbolLinkage linkage;
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
} FimoModulesDynamicSymbolExport;

/// Valid keys of `FimoModulesExportModifier`.
typedef enum FimoModulesExportModifierKey : FimoI32 {
    /// Specifies that the module export has a destructor function that must be called. The value
    /// must be a pointer to a `FimoModulesExportModifierDestructor`.
    FIMO_MODULES_EXPORT_MODIFIER_KEY_DESTRUCTOR,
    /// Specifies that the module should acquire a static dependency to another module. The value
    /// must be a strong reference to a `FimoModulesInfo`.
    FIMO_MODULES_EXPORT_MODIFIER_KEY_DEPENDENCY,
    /// Specifies that the module has its debug info embedded. The key may only be specified once
    /// per module. Adds an entry of the type `const FimoModulesDebugInfo*` to the modifiers table
    /// of the module.
    FIMO_MODULES_EXPORT_MODIFIER_DEBUG_INFO,
    /// A constructor and destructor for the state of a module.
    ///
    /// Can be specified to bind a state to an instance. The constructor will be called before the
    /// modules exports are initialized and returning an error will abort the loading of the
    /// instance. Inversely, the destructor function will be called after all exports have been
    /// deinitialized. May only be specified once. Adds an entry of the type
    /// `const FimoModulesExportModifierInstanceState*` to the modifiers table of the module.
    FIMO_MODULES_EXPORT_MODIFIER_INSTANCE_STATE,
    /// A listener for the start event of the instance.
    ///
    /// The event will be dispatched immediately after the instance has been loaded. An error will
    /// result in the destruction of the instance. May only be specified once. Adds an entry of the
    /// type `const FimoModulesExportModifierStartEvent*` to the modifiers table of the module.
    FIMO_MODULES_EXPORT_MODIFIER_START_EVENT,
    /// A listener for the stop event of the instance.
    ///
    /// The event will be dispatched immediately before any exports are deinitialized. May only be
    /// specified once. Adds an entry of the type `const FimoModulesExportModifierStartEvent*` to
    /// the modifiers table of the module.
    FIMO_MODULES_EXPORT_MODIFIER_STOP_EVENT,
} FimoModulesExportModifierKey;

/// A modifier declaration for a module export.
typedef struct FimoModulesExportModifier {
    FimoModulesExportModifierKey key;
    const void *value;
} FimoModulesExportModifier;

/// Value for the `FIMO_MODULES_EXPORT_MODIFIER_KEY_DESTRUCTOR` modifier key.
typedef struct FimoModulesExportModifierDestructor {
    /// Type-erased data to pass to the destructor.
    void *data;
    /// Destructor function.
    void (*destructor)(void *data);
} FimoModulesExportModifierDestructor;

/// Value for the `FIMO_MODULES_EXPORT_MODIFIER_KEY_DEBUG_INFO` modifier key.
typedef struct FimoModulesExportModifierDebugInfo {
    /// Type-erased data to pass to the constructor.
    void *data;
    /// Constructor function for the debug info.
    FimoResult (*construct)(void *data, FimoModulesDebugInfo *info);
} FimoModulesExportModifierDebugInfo;

typedef FIMO_TASKS_FALLIBLE(void *) FimoModulesExportModifierInstanceStateFutureResult;
typedef FIMO_TASKS_ENQUEUED_FUTURE(FimoModulesExportModifierInstanceStateFutureResult)
        FimoModulesExportModifierInstanceStateFuture;

/// Value for the `FIMO_MODULES_EXPORT_MODIFIER_KEY_INSTANCE_STATE` modifier key.
typedef struct FimoModulesExportModifierInstanceState {
    /// Constructor function for a module.
    ///
    /// The module constructor allows a module implementor to initialize some module specific data
    /// at module load time. Some use cases for module constructors are initialization of global
    /// module data, or fetching optional symbols. Returning an error aborts the loading of the
    /// module. Is called before the symbols of the modules are exported/initialized.
    FimoModulesExportModifierInstanceStateFuture (*constructor)(const FimoModulesInstance *module,
                                                                FimoModulesLoadingSet set);
    /// Destructor function for a module.
    ///
    /// During its destruction, a module is not allowed to access the module subsystem.
    void (*destructor)(const FimoModulesInstance *module, void *state);
} FimoModulesExportModifierInstanceState;

typedef FIMO_TASKS_ENQUEUED_FUTURE(FimoResult) FimoModulesExportModifierStartEventFuture;

/// Value for the `FIMO_MODULES_EXPORT_MODIFIER_START_EVENT` modifier key.
typedef struct FimoModulesExportModifierStartEvent {
    /// Function to call once the module has been loaded.
    ///
    /// Implementors of a module can utilize this event to perform arbitrary an arbitrary action
    /// once the module has been loaded. If the call returns an error, the module will be unloaded.
    FimoModulesExportModifierStartEventFuture (*on_event)(const FimoModulesInstance *module);
} FimoModulesExportModifierStartEvent;

/// Value for the `FIMO_MODULES_EXPORT_MODIFIER_STOP_EVENT` modifier key.
typedef struct FimoModulesExportModifierStopEvent {
    /// Optional function to call before the module is unloaded.
    ///
    /// May be used to finalize the module, before any symbols or state is unloaded.
    void (*on_event)(const FimoModulesInstance *module);
} FimoModulesExportModifierStopEvent;

/// Declaration of a module export.
struct FimoModulesExport {
    /// Pointer to a possible extension.
    ///
    /// Reserved for future use. Must be `NULL`.
    const void *next;
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
    const FimoModulesParamDecl *parameters;
    /// Number of parameters exposed by the module.
    FimoUSize parameters_count;
    /// List of resources declared by the module.
    const FimoModulesResourceDecl *resources;
    /// Number of resources declared by the module.
    FimoUSize resources_count;
    /// List of namespaces to import by the module.
    ///
    /// A module is only allowed to import and export symbols from/to an imported namespace. It is
    /// an error to specify a namespace, that does not exist, without exporting to that namespace.
    const FimoModulesNamespaceImport *namespace_imports;
    /// Number of namespaces to import by the module.
    FimoUSize namespace_imports_count;
    /// List of symbols to import by the module.
    ///
    /// Upon loading, the module is provided the listed symbols. If some symbols are not available,
    /// the loading fails.
    const FimoModulesSymbolImport *symbol_imports;
    /// Number of symbols to import by the module.
    FimoUSize symbol_imports_count;
    /// List of static symbols exported by the module.
    ///
    /// The named symbols will be made available to all other modules. Trying to export a duplicate
    /// symbol will result in an error upon loading of the module.
    const FimoModulesSymbolExport *symbol_exports;
    /// Number of static symbols exported by the module.
    FimoUSize symbol_exports_count;
    /// List of dynamic symbols exported by the module.
    ///
    /// A dynamic symbol is a symbol, whose creation is deferred until loading of the module. This
    /// is useful, in case the symbol depends on the module imports.
    const FimoModulesDynamicSymbolExport *dynamic_symbol_exports;
    /// Number of dynamic symbols exported by the module.
    FimoUSize dynamic_symbol_exports_count;
    /// List of modifier key-value pairs for the exported module.
    const FimoModulesExportModifier *modifiers;
    /// Number of modifiers for the module.
    FimoUSize modifiers_count;
};

/// Profile of the module subsystem.
///
/// Each profile enables a set of default features.
typedef enum FimoModulesProfile : FimoI32 {
    FIMO_MODULES_PROFILE_RELEASE,
    FIMO_MODULES_PROFILE_DEV,
} FimoModulesProfile;

/// Optional features recognized by the module subsystem.
///
/// Some features may be mutually exclusive.
typedef enum FimoModulesFeatureTag : FimoU16 {
    // remove once a feature has been declared
    FIMO_MODULES_FEATURE_TAG_,
} FimoModulesFeatureTag;

/// Request flag for an optional feature.
typedef enum FimoModulesFeatureRequestFlag : FimoU16 {
    FIMO_MODULES_FEATURE_REQUEST_FLAG_REQUIRED,
    FIMO_MODULES_FEATURE_REQUEST_FLAG_ON,
    FIMO_MODULES_FEATURE_REQUEST_FLAG_OFF,
} FimoModulesFeatureRequestFlag;

/// Request for an optional feature.
typedef struct FimoModulesFeatureRequest {
    FimoModulesFeatureTag tag;
    FimoModulesFeatureRequestFlag flag;
} FimoModulesFeatureRequest;

/// Status flag of an optional feature.
typedef enum FimoModulesFeatureStatusFlag : FimoU16 {
    FIMO_MODULES_FEATURE_STATUS_FLAG_ON,
    FIMO_MODULES_FEATURE_STATUS_FLAG_OFF,
} FimoModulesFeatureStatusFlag;

/// Status of an optional feature.
typedef struct FimoModulesFeatureStatus {
    FimoModulesFeatureTag tag;
    FimoModulesFeatureStatusFlag flag;
} FimoModulesFeatureStatus;

/// Configuration for the module subsystem.
typedef struct FimoModulesConfig {
    /// Type of the struct.
    ///
    /// Must be `FIMO_CONFIG_ID_MODULES`.
    FimoConfigId id;
    /// Feature profile of the subsytem.
    FimoModulesProfile profile;
    /// Array of optional feature requests.
    const FimoModulesFeatureRequest *features;
    /// Number of optional feature requests.
    FimoUSize feature_count;
} FimoModulesConfig;

/// A filter for selection modules to load by the module subsystem.
///
/// The filter function is passed the module export declaration and can then decide whether the
/// module should be loaded by the subsystem.
typedef bool (*FimoModulesLoadingFilter)(const FimoModulesExport *arg0, void *arg1);

/// A callback for successfully loading a module.
///
/// The callback function is called when the subsystem was successful in loading the requested
/// module, making it then possible to request symbols.
typedef void (*FimoModulesLoadingSuccessCallback)(const FimoModulesInfo *arg0, void *arg1);

/// A callback for a module loading error.
///
/// The callback function is called when the subsystem was not successful in loading the requested
/// module.
typedef void (*FimoModulesLoadingErrorCallback)(const FimoModulesExport *arg0, void *arg1);

/// VTable of the module subsystem.
///
/// Changing the VTable is a breaking change.
typedef struct FimoModulesVTable {
    /// Returns the active profile of the module subsystem.
    FimoModulesProfile (*profile)();
    /// Returns the status of all features known to the subsystem.
    ///
    /// The start of the array or `NULL` is written into `features`. The return value is the array
    /// length.
    FimoUSize (*features)(const FimoModulesFeatureRequest **features);
    /// Constructs a new pseudo module.
    ///
    /// The functions of the module subsystem require that the caller owns a reference to their own
    /// module. This is a problem, as the constructor of the context won't be assigned a module
    /// instance during bootstrapping. As a workaround, we allow for the creation of pseudo
    /// modules, i.e., module handles without an associated module.
    FimoResult (*pseudo_module_new)(const FimoModulesInstance **module);
    /// Constructs a new empty set.
    ///
    /// Modules can only be loaded, if all of their dependencies can be resolved, which requires us
    /// to determine a suitable load order. A loading set is a utility to facilitate this process,
    /// by automatically computing a suitable load order for a batch of modules.
    FimoResult (*set_new)(FimoModulesLoadingSet *set);
    /// Searches for a module by it's name.
    ///
    /// Queries a module by its unique name. The returned `FimoModulesInfo` will have its reference
    /// count increased.
    FimoResult (*find_by_name)(const char *name, const FimoModulesInfo **info);
    /// Searches for a module by a symbol it exports.
    ///
    /// Queries the module that exported the specified symbol. The returned `FimoModulesInfo` will
    /// have its reference count increased.
    FimoResult (*find_by_symbol)(const char *name, const char *ns, FimoVersion version, const FimoModulesInfo **info);
    /// Checks for the presence of a namespace in the module subsystem.
    ///
    /// A namespace exists, if at least one loaded module exports one symbol in said namespace.
    FimoResult (*namespace_exists)(const char *ns, bool *exists);
    /// Marks all instances as unloadable.
    ///
    /// Tries to unload all instances that are not referenced by any other modules. If the instance is
    /// still referenced, this will mark the instance as unloadable and enqueue it for unloading.
    FimoResult (*prune_instances)();
    /// Queries the info of a module parameter.
    ///
    /// This function can be used to query the datatype, the read access, and the write access of a
    /// module parameter. This function fails, if the parameter can not be found.
    FimoResult (*query_parameter)(const char *module, const char *param, FimoModulesParamType *type,
                                  FimoModulesParamAccessGroup *read_group, FimoModulesParamAccessGroup *write_group);
    /// Reads a module parameter with public read access.
    ///
    /// Reads the value of a module parameter with public read access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow reading with a public access.
    /// The caller must ensure that `value` points to an instance of the same datatype as the
    /// parameter in question.
    FimoResult (*read_parameter)(void *value, FimoModulesParamType type, const char *module, const char *param);
    /// Sets a module parameter with public write access.
    ///
    /// Sets the value of a module parameter with public write access. The operation fails, if the
    /// parameter does not exist, or if the parameter does not allow writing with a public access.
    /// The caller must ensure that `value` points to an instance of the same datatype as the
    /// parameter in question.
    FimoResult (*write_parameter)(const void *value, FimoModulesParamType type, const char *module, const char *param);
} FimoModulesVTable;

#ifdef __cplusplus
}
#endif

#endif // FIMO_MODULES_H
