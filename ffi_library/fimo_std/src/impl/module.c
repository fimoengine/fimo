#include <fimo_std/impl/module.h>

#include <stddef.h>

#ifdef _WIN32
__declspec(allocate("fi_mod$a")) const FimoModuleExport *fimo_impl_modules_section_start = NULL;
__declspec(allocate("fi_mod$z")) const FimoModuleExport *fimo_impl_modules_section_end = NULL;

#define FIMO_IMPL_MODULES_SECTION_START fimo_impl_modules_section_start
#define FIMO_IMPL_MODULES_SECTION_END fimo_impl_modules_section_end
#elif __APPLE__
// Allocate a dummy module to force the creation of the section symbols.
const FimoModuleExport *fimo_impl_modules_dummy_module
        __attribute__((retain, used, section(FIMO_IMPL_MODULE_SECTION))) = NULL;

extern const FimoModuleExport *fimo_impl_modules_section_start __asm("section$start$__DATA$__fimo_module");
extern const FimoModuleExport *fimo_impl_modules_section_end __asm("section$end$__DATA$__fimo_module");

#define FIMO_IMPL_MODULES_SECTION_START fimo_impl_modules_section_start
#define FIMO_IMPL_MODULES_SECTION_END fimo_impl_modules_section_end
#else
// Allocate a dummy module to force the creation of the section symbols.
const FimoModuleExport *fimo_impl_modules_dummy_module
        __attribute__((retain, used, section(FIMO_IMPL_MODULE_SECTION))) = NULL;

extern const FimoModuleExport *__start_fimo_module;
extern const FimoModuleExport *__stop_fimo_module;

#define FIMO_IMPL_MODULES_SECTION_START __start_fimo_module
#define FIMO_IMPL_MODULES_SECTION_END __stop_fimo_module
#endif

FIMO_IMPL_MODULE_EXPORT
void fimo_impl_module_export_iterator(const FimoImplModuleInspector inspector, void *data) {
    if (inspector == NULL) {
        return;
    }

    const FimoModuleExport **start = &FIMO_IMPL_MODULES_SECTION_START;
    const FimoModuleExport **end = &FIMO_IMPL_MODULES_SECTION_END;
    for (const FimoModuleExport **it = start; it != end; it++) {
        // Skip empty module declarations.
        if ((*it) == NULL) {
            continue;
        }

        // Pass the module to the inspection function.
        if (!inspector(*it, data)) {
            break;
        }
    }
}
