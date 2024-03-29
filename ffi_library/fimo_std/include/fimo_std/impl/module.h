#ifndef FIMO_IMPL_MODULE_H
#define FIMO_IMPL_MODULE_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
#define FIMO_IMPL_MODULE_EXPORT __declspec(dllexport)
#else
#define FIMO_IMPL_MODULE_EXPORT __attribute__((visibility("default")))
#endif

#ifdef _WIN32
// With the MSVC we have no way to get the start and end of
// a section, so we use three different sections. According
// to the documentation, the linker orders the entries with
// the same section prefix by the section name. Therefore,
// we can get the same result by allocating all entries in
// the middle section.
#pragma section("fi_mod$a", read)
#pragma section("fi_mod$u", read)
#pragma section("fi_mod$z", read)

/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_IMPL_MODULE_SECTION "fi_mod$u"
#elif __APPLE__
/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_IMPL_MODULE_SECTION "__DATA,__fimo_module"
#else
/**
 * Name of the section where the modules will be stored to.
 */
#define FIMO_IMPL_MODULE_SECTION "fimo_module"
#endif

typedef struct FimoModuleExport FimoModuleExport;

/**
 * Inspector function for the iterator of exported modules.
 *
 * @param arg0 export declaration
 * @param arg1 user defined data
 *
 * @return `true`, if the iteration should continue.
 */
typedef bool (*FimoImplModuleInspector)(const FimoModuleExport *arg0, void *arg1);

/**
 * Iterates over the modules exported by the current binary.
 *
 * @param inspector inspection function.
 * @param data user defined data to pass to the inspector.
 */
FIMO_IMPL_MODULE_EXPORT
void fimo_impl_module_export_iterator(FimoImplModuleInspector inspector, void *data);

#ifdef __cplusplus
}
#endif

#endif // FIMO_IMPL_MODULE_H
