#ifndef FIMO_IMPL_MACROS_EXPORT_H
#define FIMO_IMPL_MACROS_EXPORT_H

/**
 * Marks the symbol as being exported.
 */
#ifdef FIMO_STD_BUILD_SHARED
#ifdef _WIN32
#ifdef FIMO_STD_EXPORT_SYMBOLS
#define FIMO_EXPORT __declspec(dllexport)
#else
#define FIMO_EXPORT __declspec(dllimport)
#endif
#else
#define FIMO_EXPORT __attribute__((visibility("default")))
#endif
#else
#define FIMO_EXPORT
#endif

#endif // FIMO_IMPL_MACROS_EXPORT_H
