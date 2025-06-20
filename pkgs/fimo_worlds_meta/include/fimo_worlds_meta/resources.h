#ifndef FIMO_WORLDS_META_RESOURCES_H
#define FIMO_WORLDS_META_RESOURCES_H

#include <fimo_std/fimo.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FimoWorldsMeta_World FimoWorldsMeta_World;

/// A unique identifier of a registered resource.
typedef FimoUSize FimoWorldsMeta_ResourceId;

/// Descriptor of a new resource.
typedef struct FimoWorldsMeta_ResourceDescriptor {
    /// Reserved. Must be null.
    const void *next;
    /// Optional label of the resource.
    const char *label;
    /// Length in characters of the resource label.
    FimoUSize label_len;
    /// Size in bytes of the resource.
    FimoUSize size;
    /// Alignment in bytes of the resource. Must be a power-of-two.
    FimoUSize alignment;
} FimoWorldsMeta_ResourceDescriptor;

/// Registers a new resource to the universe.
///
/// Registered resources may be instantiated by any world that knows its id.
typedef FimoStatus (*FimoWorldsMeta_resource_register)(const FimoWorldsMeta_ResourceDescriptor *resource,
                                                       FimoWorldsMeta_ResourceId *id);

/// Unregister the resource from the universe.
///
/// Once unregistered, the identifier is invalidated and may be reused by another resouce.
/// The resource must not be used by any world when this method is called.
typedef void (*FimoWorldsMeta_resource_unregister)(FimoWorldsMeta_ResourceId id);

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_META_RESOURCES_H
