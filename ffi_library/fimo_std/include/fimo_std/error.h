#ifndef FIMO_ERROR_H
#define FIMO_ERROR_H

#include <stdbool.h>

#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

#define FIMO_IGNORE_(expr, var) \
    do {                        \
        FimoError var = (expr); \
        (void)var;              \
    } while (0)

/**
 * Ignores a `FimoError` result.
 *
 * Many functions are annotated with the `FIMO_MUST_USE`
 * attribute, which makes the compiler emit a warning,
 * in case the result is not used. Most compilers allow
 * suppressing the warning by using `(void)expr`, but
 * this does not work in GCC.
 *
 * @param expr expression to ignore
 */
#define FIMO_IGNORE(expr) FIMO_IGNORE_(expr, FIMO_VAR(_fimo_ignored))

/**
 * Posix error codes.
 */
typedef enum FimoError {
    FIMO_EOK = 0, /* Operation completed successfully */
    FIMO_E2BIG, /* Argument list too long */
    FIMO_EACCES, /* Permission denied */
    FIMO_EADDRINUSE, /* Address already in use */
    FIMO_EADDRNOTAVAIL, /* Address not available */
    FIMO_EAFNOSUPPORT, /* Address family not supported */
    FIMO_EAGAIN, /* Resource temporarily unavailable */
    FIMO_EALREADY, /* Connection already in progress */
    FIMO_EBADE, /* Invalid exchange */
    FIMO_EBADF, /* Bad file descriptor */
    FIMO_EBADFD, /* File descriptor in bad state */
    FIMO_EBADMSG, /* Bad message */
    FIMO_EBADR, /* Invalid request descriptor */
    FIMO_EBADRQC, /* Invalid request code */
    FIMO_EBADSLT, /* Invalid slot */
    FIMO_EBUSY, /* Device or resource busy */
    FIMO_ECANCELED, /* Operation canceled */
    FIMO_ECHILD, /* No child processes */
    FIMO_ECHRNG, /* Channel number out of range */
    FIMO_ECOMM, /* Communication error on send */
    FIMO_ECONNABORTED, /* Connection aborted */
    FIMO_ECONNREFUSED, /* Connection refused */
    FIMO_ECONNRESET, /* Connection reset */
    FIMO_EDEADLK, /* Resource deadlock avoided */
    FIMO_EDEADLOCK, /* File locking deadlock error (or Resource deadlock avoided) */
    FIMO_EDESTADDRREQ, /* Destination address required */
    FIMO_EDOM, /* Mathematics argument out of domain of function */
    FIMO_EDQUOT, /* Disk quota exceeded */
    FIMO_EEXIST, /* File exists */
    FIMO_EFAULT, /* Bad address */
    FIMO_EFBIG, /* File too large */
    FIMO_EHOSTDOWN, /* Host is down */
    FIMO_EHOSTUNREACH, /* Host is unreachable */
    FIMO_EHWPOISON, /* Memory page has hardware error */
    FIMO_EIDRM, /* Identifier removed */
    FIMO_EILSEQ, /* Invalid or incomplete multibyte or wide character */
    FIMO_EINPROGRESS, /* Operation in progress */
    FIMO_EINTR, /* Interrupted function call */
    FIMO_EINVAL, /* Invalid argument */
    FIMO_EIO, /* Input/output error */
    FIMO_EISCONN, /* Socket is connected */
    FIMO_EISDIR, /* Is a directory */
    FIMO_EISNAM, /* Is a named type file */
    FIMO_EKEYEXPIRED, /* Key has expired */
    FIMO_EKEYREJECTED, /* Key was rejected by service */
    FIMO_EKEYREVOKED, /* Key has been revoked */
    FIMO_EL2HLT, /* Level 2 halted */
    FIMO_EL2NSYNC, /* Level 2 not synchronized */
    FIMO_EL3HLT, /* Level 3 halted */
    FIMO_EL3RST, /* Level 3 reset */
    FIMO_ELIBACC, /* Cannot access a needed shared library */
    FIMO_ELIBBAD, /* Accessing a corrupted shared library */
    FIMO_ELIBMAX, /* Attempting to link in too many shared libraries */
    FIMO_ELIBSCN, /* .lib section in a.out corrupted */
    FIMO_ELIBEXEC, /* Cannot exec a shared library directly */
    FIMO_ELNRNG, /* Link number out of range */
    FIMO_ELOOP, /* Too many levels of symbolic links */
    FIMO_EMEDIUMTYPE, /* Wrong medium type */
    FIMO_EMFILE, /* Too many open files */
    FIMO_EMLINK, /* Too many links */
    FIMO_EMSGSIZE, /* Message too long */
    FIMO_EMULTIHOP, /* Multihop attempted */
    FIMO_ENAMETOOLONG, /* Filename too long */
    FIMO_ENETDOWN, /* Network is down */
    FIMO_ENETRESET, /* Connection aborted by network */
    FIMO_ENETUNREACH, /* Network unreachable */
    FIMO_ENFILE, /* Too many open files in system */
    FIMO_ENOANO, /* No anode */
    FIMO_ENOBUFS, /* No buffer space available */
    FIMO_ENODATA, /* The named attribute does not exist, or the process has no access to this attribute */
    FIMO_ENODEV, /* No such device */
    FIMO_ENOENT, /* No such file or directory */
    FIMO_ENOEXEC, /* Exec format error */
    FIMO_ENOKEY, /* Required key not available */
    FIMO_ENOLCK, /* No locks available */
    FIMO_ENOLINK, /* Link has been severed */
    FIMO_ENOMEDIUM, /* No medium found */
    FIMO_ENOMEM, /* Not enough space/cannot allocate memory */
    FIMO_ENOMSG, /* No message of the desired type */
    FIMO_ENONET, /* Machine is not on the network */
    FIMO_ENOPKG, /* Package not installed */
    FIMO_ENOPROTOOPT, /* Protocol not available */
    FIMO_ENOSPC, /* No space left on device */
    FIMO_ENOSR, /* No STREAM resources */
    FIMO_ENOSTR, /* Not a STREAM */
    FIMO_ENOSYS, /* Function not implemented */
    FIMO_ENOTBLK, /* Block device required */
    FIMO_ENOTCONN, /* The socket is not connected */
    FIMO_ENOTDIR, /* Not a directory */
    FIMO_ENOTEMPTY, /* Directory not empty */
    FIMO_ENOTRECOVERABLE, /* State not recoverable */
    FIMO_ENOTSOCK, /* Not a socket */
    FIMO_ENOTSUP, /* Operation not supported */
    FIMO_ENOTTY, /* Inappropriate I/O control operation */
    FIMO_ENOTUNIQ, /* Name not unique on network */
    FIMO_ENXIO, /* No such device or address */
    FIMO_EOPNOTSUPP, /* Operation not supported on socket */
    FIMO_EOVERFLOW, /* Value too large to be stored in data type */
    FIMO_EOWNERDEAD, /* Owner died */
    FIMO_EPERM, /* Operation not permitted */
    FIMO_EPFNOSUPPORT, /* Protocol family not supported */
    FIMO_EPIPE, /* Broken pipe */
    FIMO_EPROTO, /* Protocol error */
    FIMO_EPROTONOSUPPORT, /* Protocol not supported */
    FIMO_EPROTOTYPE, /* Protocol wrong type for socket */
    FIMO_ERANGE, /* Result too large */
    FIMO_EREMCHG, /* Remote address changed */
    FIMO_EREMOTE, /* Object is remote */
    FIMO_EREMOTEIO, /* Remote I/O error */
    FIMO_ERESTART, /* Interrupted system call should be restarted */
    FIMO_ERFKILL, /* Operation not possible due to RF-kill */
    FIMO_EROFS, /* Read-only filesystem */
    FIMO_ESHUTDOWN, /* Cannot send after transport endpoint shutdown */
    FIMO_ESPIPE, /* Invalid seek */
    FIMO_ESOCKTNOSUPPORT, /* Socket type not supported */
    FIMO_ESRCH, /* No such process */
    FIMO_ESTALE, /* Stale file handle */
    FIMO_ESTRPIPE, /* Streams pipe error */
    FIMO_ETIME, /* Timer expired */
    FIMO_ETIMEDOUT, /* Connection timed out */
    FIMO_ETOOMANYREFS, /* Too many references: cannot splice */
    FIMO_ETXTBSY, /* Text file busy */
    FIMO_EUCLEAN, /* Structure needs cleaning */
    FIMO_EUNATCH, /* Protocol driver not attached */
    FIMO_EUSERS, /* Too many users */
    FIMO_EWOULDBLOCK, /* Operation would block */
    FIMO_EXDEV, /* Invalid cross-device link */
    FIMO_EXFULL, /* Exchange full */
    FIMO_EUNKNOWN, /* Unknown error */
} FimoError;

/**
 * Upper range (inclusive) of the valid error codes.
 */
#define FIMO_MAX_ERROR FIMO_EUNKNOWN

/**
 * Checks if an error number is valid.
 *
 * @param errnum error number
 *
 * @return Error number is an error
 */
#define FIMO_IS_VALID_ERROR(errnum) (((errnum) >= FIMO_EOK) && ((errnum) <= FIMO_MAX_ERROR))

/**
 * Checks if an error number represents an error.
 *
 * @param errnum error number
 *
 * @return Error number is valid.
 */
#define FIMO_IS_ERROR(errnum) (((errnum) > FIMO_EOK) && FIMO_IS_VALID_ERROR(errnum))

#ifdef FIMO_MACRO_HELPER_FUNCTIONS
/**
 * Checks if an error number is valid.
 *
 * @param errnum error number
 *
 * @return Error number is valid.
 */
FIMO_MUST_USE bool fimo_is_valid_error(FimoError errnum);

/**
 * Checks if an error number represents an error.
 *
 * @param errnum error number
 *
 * @return Error number is an error
 */
FIMO_MUST_USE bool fimo_is_error(FimoError errnum);
#endif // FIMO_MACRO_HELPER_FUNCTIONS

/**
 * Get the name of the error.
 *
 * @param errnum the error number
 * @param err optional error value
 *
 * The success of this call are written into `err` if it is not `NULL`.
 * In case of an error this returns `"Unknown error number"`.
 *
 * @return The name of the error.
 *
 * @error `FIMO_EOK`: Operation was successful.
 * @error `FIMO_EINVAL`: The value of `errnum` is not a valid error number.
 */
FIMO_MUST_USE const char* fimo_strerrorname(FimoError errnum, FimoError* err);

/**
 * Get the description of the error.
 *
 * @param errnum the error number
 * @param err optional error value
 *
 * The success of this call are written into `err` if it is not `NULL`.
 * In case of an error this returns `"Unknown error number"`.
 *
 * @return The description of the error.
 *
 * @error `FIMO_EOK`: Operation was successful.
 * @error `FIMO_EINVAL`: The value of `errnum` is not a valid error number.
 */
FIMO_MUST_USE const char* fimo_strerrordesc(FimoError errnum, FimoError* err);

/**
 * Constructs an error code from an errno error code.
 *
 * @param errnum: errno error code
 *
 * Unknown errno codes translate to `FIMO_EUNKNOWN`.
 *
 * @return Status code.
 */
FIMO_MUST_USE FimoError fimo_error_from_errno(int errnum);

#ifdef __cplusplus
}
#endif // __cplusplus

#endif // FIMO_ERROR_H
