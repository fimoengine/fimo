#ifndef FIMO_PATH_H
#define FIMO_PATH_H

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * A growable filesystem path encoded as UTF-8.
 */
typedef struct FimoUTF8PathBuf {
    char *buffer;
    FimoUSize length;
    FimoUSize capacity;
} FimoUTF8PathBuf;

/**
 * An owned filesystem path encoded as UTF-8.
 *
 * The underlying string is not null-terminated.
 */
typedef struct FimoOwnedUTF8Path {
    char *path;
    FimoUSize length;
} FimoOwnedUTF8Path;

/**
 * A path encoded as UTF-8.
 *
 * The underlying string is not null-terminated.
 */
typedef struct FimoUTF8Path {
    const char *path;
    FimoUSize length;
} FimoUTF8Path;

/**
 * Character type for paths used by the native os apis.
 */
#if _WIN32
typedef wchar_t FimoOSPathChar;
#else
typedef char FimoOSPathChar;
#endif

/**
 * An owned path that may be passed to the native os apis.
 *
 * On Posix systems, the string encoding is unspecified.
 * On Windows systems, the strings are encoded as UTF-16.
 * The string is null-terminated.
 */
typedef struct FimoOwnedOSPath {
    FimoOSPathChar *path;
    FimoUSize length;
} FimoOwnedOSPath;

/**
 * An path that may be passed to the native os apis.
 *
 * On Posix systems, the string encoding is unspecified.
 * On Windows systems, the strings are encoded as UTF-16.
 * The string is null-terminated.
 */
typedef struct FimoOSPath {
    const FimoOSPathChar *path;
    FimoUSize length;
} FimoOSPath;

/**
 * A Windows path prefix.
 */
typedef struct FimoUTF8PathPrefix {
    enum {
        FIMO_UTF8_PATH_PREFIX_VERBATIM,
        FIMO_UTF8_PATH_PREFIX_VERBATIM_UNC,
        FIMO_UTF8_PATH_PREFIX_VERBATIM_DISK,
        FIMO_UTF8_PATH_PREFIX_DEVICE_NS,
        FIMO_UTF8_PATH_PREFIX_UNC,
        FIMO_UTF8_PATH_PREFIX_DISK,
    } type;
    union {
        // `\\?\prefix`
        FimoUTF8Path verbatim;
        // `\\?\UNC\hostname\share_name`
        struct {
            FimoUTF8Path hostname;
            FimoUTF8Path share_name;
        } verbatim_unc;
        // `\\?\C:`
        char verbatim_disk;
        // `\\.\NS`
        FimoUTF8Path device_ns;
        // `\\hostname\share_name`
        struct {
            FimoUTF8Path hostname;
            FimoUTF8Path share_name;
        } unc;
        // `C:`
        char disk;
    } data;
} FimoUTF8PathPrefix;

/**
 * Definition of all possible path components.
 */
typedef struct FimoUTF8PathComponent {
    enum {
        FIMO_UTF8_PATH_COMPONENT_PREFIX,
        FIMO_UTF8_PATH_COMPONENT_ROOT_DIR,
        FIMO_UTF8_PATH_COMPONENT_CUR_DIR,
        FIMO_UTF8_PATH_COMPONENT_PARENT_DIR,
        FIMO_UTF8_PATH_COMPONENT_NORMAL,
    } type;
    union {
        struct {
            FimoUTF8Path raw;
            FimoUTF8PathPrefix prefix;
        } prefix;
        FimoU8 root_dir;
        FimoU8 cur_dir;
        FimoU8 parent_dir;
        FimoUTF8Path normal;
    } data;
} FimoUTF8PathComponent;

/**
 * Internal state of a path component iterator.
 */
typedef enum FimoUTF8PathComponentIteratorState {
    FIMO_UTF8_PATH_COMPONENT_ITER_STATE_PREFIX,
    FIMO_UTF8_PATH_COMPONENT_ITER_STATE_START_DIR,
    FIMO_UTF8_PATH_COMPONENT_ITER_STATE_BODY,
    FIMO_UTF8_PATH_COMPONENT_ITER_STATE_DONE,
} FimoUTF8PathComponentIteratorState;

/**
 * Iterator over the components of a path.
 */
typedef struct FimoUTF8PathComponentIterator {
    FimoUTF8Path current;
    bool has_prefix;
    FimoUTF8PathPrefix prefix;
    bool has_root_separator;
    FimoUTF8PathComponentIteratorState front;
    FimoUTF8PathComponentIteratorState back;
} FimoUTF8PathComponentIterator;

/**
 * Creates a new empty path buffer.
 *
 * @return Empty buffer.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8PathBuf fimo_utf8_path_buf_new(void);

/**
 * Creates a new path buffer with at least the given capacity.
 *
 * @param capacity minimum capacity
 * @param buf resulting buffer
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_with_capacity(FimoUSize capacity, FimoUTF8PathBuf *buf);

/**
 * Deallocates the path buffer.
 *
 * @param buf buffer
 */
FIMO_EXPORT
void fimo_utf8_path_buf_free(FimoUTF8PathBuf *buf);

/**
 * Extracts the path.
 *
 * @param buf buffer
 *
 * @return Path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_buf_as_path(const FimoUTF8PathBuf *buf);

/**
 * Coerces the path buffer to an owned path.
 *
 * The path buffer may not be used after this call.
 *
 * @param buf path buffer to coerce
 * @param owned resulting path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_into_owned_path(FimoUTF8PathBuf *buf, FimoOwnedUTF8Path *owned);

/**
 * Extends the path buffer with a path.
 *
 * If `path` is absolute, it replaces the current path.
 *
 * On Windows:
 *
 * - if `path` has a root but no prefix (e.g., `\windows`), it replaces
 *   everything except for the prefix (if any) of `buf`.
 * - if `path` has a prefix but no root, it replaces `buf`.
 * - if `buf` has a verbatim prefix (e.g. `\\?\C:\windows`) and `path`
 *   is not empty, the new path is normalized: all references to `.`
 *   and `..` are removed`.
 *
 * @param buf buffer to extend
 * @param path path to append
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_push_path(FimoUTF8PathBuf *buf, FimoUTF8Path path);

/**
 * Extends the path buffer with a UTF-8 string.
 *
 * Is equivalent to `fimo_utf8_path_buf_push_path(fimo_utf8_path_new(path))`.
 *
 * @param buf buffer to extend
 * @param path path to append
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_buf_push_string(FimoUTF8PathBuf *buf, const char *path);

/**
 * Truncates the path buffer to its parent.
 *
 * Returns `false` and does nothing if there is no parent.
 * Otherwise, returns `true`.
 *
 * @param buf buffer to truncate
 *
 * @return `true`, if the buffer was truncated.
 */
FIMO_EXPORT
bool fimo_utf8_path_buf_pop(FimoUTF8PathBuf *buf);

/**
 * Constructs a new owned path by copying a UTF-8 string.
 *
 * @param path path to clone
 * @param owned resulting path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_string(const char *path, FimoOwnedUTF8Path *owned);

/**
 * Constructs a new owned path by copying the contents of another path.
 *
 * @param path path to clone
 * @param owned resulting path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_path(FimoUTF8Path path, FimoOwnedUTF8Path *owned);

/**
 * Constructs a new owned path from an os path.
 *
 * On Windows the path will re-encode the os path string from UTF-16
 * to UTF-8. No other conversions will be performed.
 *
 * @param path path to convert
 * @param owned resulting path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_utf8_path_from_os_path(FimoOSPath path, FimoOwnedUTF8Path *owned);

/**
 * Releases the memory associated with the path.
 *
 * The path may not be used after this call.
 *
 * @param path path to free
 */
FIMO_EXPORT
void fimo_owned_utf8_path_free(FimoOwnedUTF8Path path);

/**
 * Extracts the path from the owned path.
 *
 * @param path path to extract from
 *
 * @return Extracted path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_owned_utf8_path_as_path(FimoOwnedUTF8Path path);

/**
 * Coerces the owned path to a path buffer.
 *
 * The path may not be used after this call.
 *
 * @param path path to coerce
 *
 * @return Constructed path buffer.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8PathBuf fimo_owned_utf8_path_into_path_buf(FimoOwnedUTF8Path path);

/**
 * Creates a new path from a UTF-8 null-terminated string.
 *
 * @param path_str string to initialize the path with.
 * @param path resulting path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_utf8_path_new(const char *path_str, FimoUTF8Path *path);

/**
 * Returns whether the path is absolute, i.e., if it is independent of the current directory.
 *
 * @param path path
 *
 * @return Whether path is absolute.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_is_absolute(FimoUTF8Path path);

/**
 * Returns whether the path is relative, i.e., if it is dependent of the current directory.
 *
 * @param path path
 *
 * @return Whether path is relative.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_is_relative(FimoUTF8Path path);

/**
 * Returns if the path has a root.
 *
 * @param path path
 *
 * @return Whether path has a root.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_has_root(FimoUTF8Path path);

/**
 * Returns the path without its final component, if there is one.
 *
 * @param path path
 * @param parent resulting parent
 *
 * @return Whether the parent exists.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_parent(FimoUTF8Path path, FimoUTF8Path *parent);

/**
 * Returns the final component of the path, if there is one.
 *
 * @param path path
 * @param file_name resulting file name
 *
 * @return Whether the file name exists.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_file_name(FimoUTF8Path path, FimoUTF8Path *file_name);

/**
 * Constructs a new owned os from a UTF-8 path.
 *
 * @param path path to convert
 * @param os_path resulting os path
 *
 * @return Status.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoResult fimo_owned_os_path_from_path(FimoUTF8Path path, FimoOwnedOSPath *os_path);

/**
 * Frees the memory associated with the os path.
 *
 * @param path path to free
 */
FIMO_EXPORT
void fimo_owned_os_path_free(FimoOwnedOSPath path);

/**
 * Extracts the os path from the owned os path.
 *
 * @param path owned os path
 *
 * @return OS path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoOSPath fimo_owned_os_path_as_os_path(FimoOwnedOSPath path);

/**
 * Constructs a new os path from a null-terminated string.
 *
 * @path path string
 *
 * @return OS path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoOSPath fimo_os_path_new(const FimoOSPathChar *path);

/**
 * Constructs an iterator over the components of a path.
 *
 * @param path to iterate
 *
 * @return Iterator.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8PathComponentIterator fimo_utf8_path_component_iter_new(FimoUTF8Path path);

/**
 * Extracts a path corresponding to the portion of the path remaining for iteration.
 *
 * @param iter iterator
 *
 * @return Path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_component_iter_as_path(const FimoUTF8PathComponentIterator *iter);

/**
 * Performs an iteration step.
 *
 * Extracts the next component from the front of the iterator.
 *
 * @param iter iterator
 * @param component resulting component
 *
 * @return `true`, if the iterator was not empty, or `false` otherwise.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_component_iter_next(FimoUTF8PathComponentIterator *iter, FimoUTF8PathComponent *component);

/**
 * Performs an iteration step.
 *
 * Extracts the next component from the back of the iterator.
 *
 * @param iter iterator
 * @param component resulting component
 *
 * @return `true`, if the iterator was not empty, or `false` otherwise.
 */
FIMO_EXPORT
FIMO_MUST_USE
bool fimo_utf8_path_component_iter_next_back(FimoUTF8PathComponentIterator *iter, FimoUTF8PathComponent *component);

/**
 * Extracts the underlying path.
 *
 * @param component parsed component
 *
 * @return Component path.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoUTF8Path fimo_utf8_path_component_as_path(const FimoUTF8PathComponent *component);

#ifdef __cplusplus
}
#endif

#endif // FIMO_PATH_H
