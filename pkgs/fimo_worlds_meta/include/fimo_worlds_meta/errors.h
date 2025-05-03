#ifndef FIMO_WORLDS_META_ERRORS_H
#define FIMO_WORLDS_META_ERRORS_H

#include <fimo_std/integers.h>

/// Errors defined by the package.
typedef enum FimoWorldsMeta_Error : FimoI32 {
    FIMO_WORLDS_META_ERROR_OK,
    FIMO_WORLDS_META_ERROR_OPERATION_FAILED,
} FimoWorldsMeta_Error;

#endif // FIMO_WORLDS_META_ERRORS_H
