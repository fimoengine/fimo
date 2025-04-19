#ifndef FIMO_IMPL_MACROS_MUST_USE_H
#define FIMO_IMPL_MACROS_MUST_USE_H

#include <fimo_std/impl/macros/msvc_sal.h>

/**
 * Adds a `must_use` attribute to a function return value.
 */
#if defined(__GNUC__) && (__GNUC__ >= 4)
#define FIMO_MUST_USE __attribute__((warn_unused_result))
#elif defined(_MSC_VER) && (_MSC_VER >= 1700)
#define FIMO_MUST_USE _Check_return_
#else
#define FIMO_MUST_USE
#endif

#endif // !FIMO_IMPL_MACROS_MUST_USE_H
