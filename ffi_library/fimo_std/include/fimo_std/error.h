#ifndef FIMO_ERROR_H
#define FIMO_ERROR_H

#include <assert.h>
#include <stdalign.h>
#include <stdbool.h>

#include <fimo_std/utils.h>

#ifdef _WIN32
#define WIN32_LEAN_AND_MEAN
#include <Windows.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Upper range (inclusive) of the valid error codes.
 */
#define FIMO_ERROR_CODE_MAX FIMO_ERROR_CODE_XFULL

/**
 * Ignores a `FimoResult` result.
 *
 * @param EXPR expression to ignore
 */
#define FIMO_RESULT_IGNORE(EXPR) fimo_result_release((EXPR))

/**
 * Checks whether a `FimoResult` contains an error.
 *
 * @param EXPR expression to check
 */
#define FIMO_RESULT_IS_ERROR(EXPR) fimo_result_is_error((EXPR))

/**
 * Checks whether a `FimoResult` does not contain an error.
 *
 * @param EXPR expression to check
 */
#define FIMO_RESULT_IS_OK(EXPR) fimo_result_is_ok((EXPR))

/**
 * Constructs a `FimoResult` from a static string.
 *
 * @param CODE error code
 */
#define FIMO_RESULT_FROM_STRING(ERROR) fimo_result_from_static_string(ERROR)

/**
 * Constructs a `FimoResult` from a dynamic string.
 *
 * @param CODE error code
 */
#define FIMO_RESULT_FROM_DYNAMIC_STRING(ERROR) fimo_result_from_dynamic_string(ERROR)

/**
 * Constructs a `FimoResult` from a `FimoErrorCode`.
 *
 * @param CODE error code
 */
#define FIMO_RESULT_FROM_ERROR_CODE(CODE) fimo_result_from_error_code(CODE)

/**
 * Constructs a `FimoResult` from a `FimoSystemErrorCode`.
 *
 * @param CODE error code
 */
#define FIMO_RESULT_FROM_SYSTEM_ERROR_CODE(CODE) fimo_result_from_system_error_code(CODE)

#define FIMO_EOK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_OK)
#define FIMO_E2BIG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_2BIG)
#define FIMO_EACCES FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ACCES)
#define FIMO_EADDRINUSE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ADDRINUSE)
#define FIMO_EADDRNOTAVAIL FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ADDRNOTAVAIL)
#define FIMO_EAFNOSUPPORT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_AFNOSUPPORT)
#define FIMO_EAGAIN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_AGAIN)
#define FIMO_EALREADY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ALREADY)
#define FIMO_EBADE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADE)
#define FIMO_EBADF FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADF)
#define FIMO_EBADFD FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADFD)
#define FIMO_EBADMSG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADMSG)
#define FIMO_EBADR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADR)
#define FIMO_EBADRQC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADRQC)
#define FIMO_EBADSLT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BADSLT)
#define FIMO_EBUSY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_BUSY)
#define FIMO_ECANCELED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CANCELED)
#define FIMO_ECHILD FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CHILD)
#define FIMO_ECHRNG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CHRNG)
#define FIMO_ECOMM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_COMM)
#define FIMO_ECONNABORTED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CONNABORTED)
#define FIMO_ECONNREFUSED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CONNREFUSED)
#define FIMO_ECONNRESET FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_CONNRESET)
#define FIMO_EDEADLK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_DEADLK)
#define FIMO_EDEADLOCK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_DEADLOCK)
#define FIMO_EDESTADDRREQ FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_DESTADDRREQ)
#define FIMO_EDOM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_DOM)
#define FIMO_EDQUOT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_DQUOT)
#define FIMO_EEXIST FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_EXIST)
#define FIMO_EFAULT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_FAULT)
#define FIMO_EFBIG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_FBIG)
#define FIMO_EHOSTDOWN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_HOSTDOWN)
#define FIMO_EHOSTUNREACH FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_HOSTUNREACH)
#define FIMO_EHWPOISON FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_HWPOISON)
#define FIMO_EIDRM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_IDRM)
#define FIMO_EILSEQ FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ILSEQ)
#define FIMO_EINPROGRESS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_INPROGRESS)
#define FIMO_EINTR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_INTR)
#define FIMO_EINVAL FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_INVAL)
#define FIMO_EIO FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_IO)
#define FIMO_EISCONN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ISCONN)
#define FIMO_EISDIR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ISDIR)
#define FIMO_EISNAM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ISNAM)
#define FIMO_EKEYEXPIRED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_KEYEXPIRED)
#define FIMO_EKEYREJECTED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_KEYREJECTED)
#define FIMO_EKEYREVOKED FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_KEYREVOKED)
#define FIMO_EL2HLT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_L2HLT)
#define FIMO_EL2NSYNC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_L2NSYNC)
#define FIMO_EL3HLT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_L3HLT)
#define FIMO_EL3RST FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_L3RST)
#define FIMO_ELIBACC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LIBACC)
#define FIMO_ELIBBAD FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LIBBAD)
#define FIMO_ELIBMAX FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LIBMAX)
#define FIMO_ELIBSCN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LIBSCN)
#define FIMO_ELIBEXEC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LIBEXEC)
#define FIMO_ELNRNG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LNRNG)
#define FIMO_ELOOP FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_LOOP)
#define FIMO_EMEDIUMTYPE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_MEDIUMTYPE)
#define FIMO_EMFILE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_MFILE)
#define FIMO_EMLINK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_MLINK)
#define FIMO_EMSGSIZE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_MSGSIZE)
#define FIMO_EMULTIHOP FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_MULTIHOP)
#define FIMO_ENAMETOOLONG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NAMETOOLONG)
#define FIMO_ENETDOWN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NETDOWN)
#define FIMO_ENETRESET FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NETRESET)
#define FIMO_ENETUNREACH FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NETUNREACH)
#define FIMO_ENFILE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NFILE)
#define FIMO_ENOANO FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOANO)
#define FIMO_ENOBUFS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOBUFS)
#define FIMO_ENODATA FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NODATA)
#define FIMO_ENODEV FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NODEV)
#define FIMO_ENOENT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOENT)
#define FIMO_ENOEXEC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOEXEC)
#define FIMO_ENOKEY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOKEY)
#define FIMO_ENOLCK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOLCK)
#define FIMO_ENOLINK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOLINK)
#define FIMO_ENOMEDIUM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOMEDIUM)
#define FIMO_ENOMEM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOMEM)
#define FIMO_ENOMSG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOMSG)
#define FIMO_ENONET FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NONET)
#define FIMO_ENOPKG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOPKG)
#define FIMO_ENOPROTOOPT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOPROTOOPT)
#define FIMO_ENOSPC FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOSPC)
#define FIMO_ENOSR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOSR)
#define FIMO_ENOSTR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOSTR)
#define FIMO_ENOSYS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOSYS)
#define FIMO_ENOTBLK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTBLK)
#define FIMO_ENOTCONN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTCONN)
#define FIMO_ENOTDIR FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTDIR)
#define FIMO_ENOTEMPTY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTEMPTY)
#define FIMO_ENOTRECOVERABLE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTRECOVERABLE)
#define FIMO_ENOTSOCK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTSOCK)
#define FIMO_ENOTSUP FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTSUP)
#define FIMO_ENOTTY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTTY)
#define FIMO_ENOTUNIQ FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NOTUNIQ)
#define FIMO_ENXIO FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_NXIO)
#define FIMO_EOPNOTSUPP FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_OPNOTSUPP)
#define FIMO_EOVERFLOW FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_OVERFLOW)
#define FIMO_EOWNERDEAD FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_OWNERDEAD)
#define FIMO_EPERM FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PERM)
#define FIMO_EPFNOSUPPORT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PFNOSUPPORT)
#define FIMO_EPIPE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PIPE)
#define FIMO_EPROTO FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PROTO)
#define FIMO_EPROTONOSUPPORT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PROTONOSUPPORT)
#define FIMO_EPROTOTYPE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_PROTOTYPE)
#define FIMO_ERANGE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_RANGE)
#define FIMO_EREMCHG FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_REMCHG)
#define FIMO_EREMOTE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_REMOTE)
#define FIMO_EREMOTEIO FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_REMOTEIO)
#define FIMO_ERESTART FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_RESTART)
#define FIMO_ERFKILL FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_RFKILL)
#define FIMO_EROFS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_ROFS)
#define FIMO_ESHUTDOWN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_SHUTDOWN)
#define FIMO_ESPIPE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_SPIPE)
#define FIMO_ESOCKTNOSUPPORT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_SOCKTNOSUPPORT)
#define FIMO_ESRCH FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_SRCH)
#define FIMO_ESTALE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_STALE)
#define FIMO_ESTRPIPE FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_STRPIPE)
#define FIMO_ETIME FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_TIME)
#define FIMO_ETIMEDOUT FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_TIMEDOUT)
#define FIMO_ETOOMANYREFS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_TOOMANYREFS)
#define FIMO_ETXTBSY FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_TXTBSY)
#define FIMO_EUCLEAN FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_UCLEAN)
#define FIMO_EUNATCH FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_UNATCH)
#define FIMO_EUSERS FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_USERS)
#define FIMO_EWOULDBLOCK FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_WOULDBLOCK)
#define FIMO_EXDEV FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_XDEV)
#define FIMO_EXFULL FIMO_RESULT_FROM_ERROR_CODE(FIMO_ERROR_CODE_XFULL)

/**
 * Posix error codes.
 */
typedef enum FimoErrorCode {
    FIMO_ERROR_CODE_OK = 0, /* Operation completed successfully */
    FIMO_ERROR_CODE_2BIG, /* Argument list too long */
    FIMO_ERROR_CODE_ACCES, /* Permission denied */
    FIMO_ERROR_CODE_ADDRINUSE, /* Address already in use */
    FIMO_ERROR_CODE_ADDRNOTAVAIL, /* Address not available */
    FIMO_ERROR_CODE_AFNOSUPPORT, /* Address family not supported */
    FIMO_ERROR_CODE_AGAIN, /* Resource temporarily unavailable */
    FIMO_ERROR_CODE_ALREADY, /* Connection already in progress */
    FIMO_ERROR_CODE_BADE, /* Invalid exchange */
    FIMO_ERROR_CODE_BADF, /* Bad file descriptor */
    FIMO_ERROR_CODE_BADFD, /* File descriptor in bad state */
    FIMO_ERROR_CODE_BADMSG, /* Bad message */
    FIMO_ERROR_CODE_BADR, /* Invalid request descriptor */
    FIMO_ERROR_CODE_BADRQC, /* Invalid request code */
    FIMO_ERROR_CODE_BADSLT, /* Invalid slot */
    FIMO_ERROR_CODE_BUSY, /* Device or resource busy */
    FIMO_ERROR_CODE_CANCELED, /* Operation canceled */
    FIMO_ERROR_CODE_CHILD, /* No child processes */
    FIMO_ERROR_CODE_CHRNG, /* Channel number out of range */
    FIMO_ERROR_CODE_COMM, /* Communication error on send */
    FIMO_ERROR_CODE_CONNABORTED, /* Connection aborted */
    FIMO_ERROR_CODE_CONNREFUSED, /* Connection refused */
    FIMO_ERROR_CODE_CONNRESET, /* Connection reset */
    FIMO_ERROR_CODE_DEADLK, /* Resource deadlock avoided */
    FIMO_ERROR_CODE_DEADLOCK, /* File locking deadlock error (or Resource deadlock avoided) */
    FIMO_ERROR_CODE_DESTADDRREQ, /* Destination address required */
    FIMO_ERROR_CODE_DOM, /* Mathematics argument out of domain of function */
    FIMO_ERROR_CODE_DQUOT, /* Disk quota exceeded */
    FIMO_ERROR_CODE_EXIST, /* File exists */
    FIMO_ERROR_CODE_FAULT, /* Bad address */
    FIMO_ERROR_CODE_FBIG, /* File too large */
    FIMO_ERROR_CODE_HOSTDOWN, /* Host is down */
    FIMO_ERROR_CODE_HOSTUNREACH, /* Host is unreachable */
    FIMO_ERROR_CODE_HWPOISON, /* Memory page has hardware error */
    FIMO_ERROR_CODE_IDRM, /* Identifier removed */
    FIMO_ERROR_CODE_ILSEQ, /* Invalid or incomplete multibyte or wide character */
    FIMO_ERROR_CODE_INPROGRESS, /* Operation in progress */
    FIMO_ERROR_CODE_INTR, /* Interrupted function call */
    FIMO_ERROR_CODE_INVAL, /* Invalid argument */
    FIMO_ERROR_CODE_IO, /* Input/output error */
    FIMO_ERROR_CODE_ISCONN, /* Socket is connected */
    FIMO_ERROR_CODE_ISDIR, /* Is a directory */
    FIMO_ERROR_CODE_ISNAM, /* Is a named type file */
    FIMO_ERROR_CODE_KEYEXPIRED, /* Key has expired */
    FIMO_ERROR_CODE_KEYREJECTED, /* Key was rejected by service */
    FIMO_ERROR_CODE_KEYREVOKED, /* Key has been revoked */
    FIMO_ERROR_CODE_L2HLT, /* Level 2 halted */
    FIMO_ERROR_CODE_L2NSYNC, /* Level 2 not synchronized */
    FIMO_ERROR_CODE_L3HLT, /* Level 3 halted */
    FIMO_ERROR_CODE_L3RST, /* Level 3 reset */
    FIMO_ERROR_CODE_LIBACC, /* Cannot access a needed shared library */
    FIMO_ERROR_CODE_LIBBAD, /* Accessing a corrupted shared library */
    FIMO_ERROR_CODE_LIBMAX, /* Attempting to link in too many shared libraries */
    FIMO_ERROR_CODE_LIBSCN, /* .lib section in a.out corrupted */
    FIMO_ERROR_CODE_LIBEXEC, /* Cannot exec a shared library directly */
    FIMO_ERROR_CODE_LNRNG, /* Link number out of range */
    FIMO_ERROR_CODE_LOOP, /* Too many levels of symbolic links */
    FIMO_ERROR_CODE_MEDIUMTYPE, /* Wrong medium type */
    FIMO_ERROR_CODE_MFILE, /* Too many open files */
    FIMO_ERROR_CODE_MLINK, /* Too many links */
    FIMO_ERROR_CODE_MSGSIZE, /* Message too long */
    FIMO_ERROR_CODE_MULTIHOP, /* Multihop attempted */
    FIMO_ERROR_CODE_NAMETOOLONG, /* Filename too long */
    FIMO_ERROR_CODE_NETDOWN, /* Network is down */
    FIMO_ERROR_CODE_NETRESET, /* Connection aborted by network */
    FIMO_ERROR_CODE_NETUNREACH, /* Network unreachable */
    FIMO_ERROR_CODE_NFILE, /* Too many open files in system */
    FIMO_ERROR_CODE_NOANO, /* No anode */
    FIMO_ERROR_CODE_NOBUFS, /* No buffer space available */
    FIMO_ERROR_CODE_NODATA, /* The named attribute does not exist, or the process has no access to this attribute */
    FIMO_ERROR_CODE_NODEV, /* No such device */
    FIMO_ERROR_CODE_NOENT, /* No such file or directory */
    FIMO_ERROR_CODE_NOEXEC, /* Exec format error */
    FIMO_ERROR_CODE_NOKEY, /* Required key not available */
    FIMO_ERROR_CODE_NOLCK, /* No locks available */
    FIMO_ERROR_CODE_NOLINK, /* Link has been severed */
    FIMO_ERROR_CODE_NOMEDIUM, /* No medium found */
    FIMO_ERROR_CODE_NOMEM, /* Not enough space/cannot allocate memory */
    FIMO_ERROR_CODE_NOMSG, /* No message of the desired type */
    FIMO_ERROR_CODE_NONET, /* Machine is not on the network */
    FIMO_ERROR_CODE_NOPKG, /* Package not installed */
    FIMO_ERROR_CODE_NOPROTOOPT, /* Protocol not available */
    FIMO_ERROR_CODE_NOSPC, /* No space left on device */
    FIMO_ERROR_CODE_NOSR, /* No STREAM resources */
    FIMO_ERROR_CODE_NOSTR, /* Not a STREAM */
    FIMO_ERROR_CODE_NOSYS, /* Function not implemented */
    FIMO_ERROR_CODE_NOTBLK, /* Block device required */
    FIMO_ERROR_CODE_NOTCONN, /* The socket is not connected */
    FIMO_ERROR_CODE_NOTDIR, /* Not a directory */
    FIMO_ERROR_CODE_NOTEMPTY, /* Directory not empty */
    FIMO_ERROR_CODE_NOTRECOVERABLE, /* State not recoverable */
    FIMO_ERROR_CODE_NOTSOCK, /* Not a socket */
    FIMO_ERROR_CODE_NOTSUP, /* Operation not supported */
    FIMO_ERROR_CODE_NOTTY, /* Inappropriate I/O control operation */
    FIMO_ERROR_CODE_NOTUNIQ, /* Name not unique on network */
    FIMO_ERROR_CODE_NXIO, /* No such device or address */
    FIMO_ERROR_CODE_OPNOTSUPP, /* Operation not supported on socket */
    FIMO_ERROR_CODE_OVERFLOW, /* Value too large to be stored in data type */
    FIMO_ERROR_CODE_OWNERDEAD, /* Owner died */
    FIMO_ERROR_CODE_PERM, /* Operation not permitted */
    FIMO_ERROR_CODE_PFNOSUPPORT, /* Protocol family not supported */
    FIMO_ERROR_CODE_PIPE, /* Broken pipe */
    FIMO_ERROR_CODE_PROTO, /* Protocol error */
    FIMO_ERROR_CODE_PROTONOSUPPORT, /* Protocol not supported */
    FIMO_ERROR_CODE_PROTOTYPE, /* Protocol wrong type for socket */
    FIMO_ERROR_CODE_RANGE, /* Result too large */
    FIMO_ERROR_CODE_REMCHG, /* Remote address changed */
    FIMO_ERROR_CODE_REMOTE, /* Object is remote */
    FIMO_ERROR_CODE_REMOTEIO, /* Remote I/O error */
    FIMO_ERROR_CODE_RESTART, /* Interrupted system call should be restarted */
    FIMO_ERROR_CODE_RFKILL, /* Operation not possible due to RF-kill */
    FIMO_ERROR_CODE_ROFS, /* Read-only filesystem */
    FIMO_ERROR_CODE_SHUTDOWN, /* Cannot send after transport endpoint shutdown */
    FIMO_ERROR_CODE_SPIPE, /* Invalid seek */
    FIMO_ERROR_CODE_SOCKTNOSUPPORT, /* Socket type not supported */
    FIMO_ERROR_CODE_SRCH, /* No such process */
    FIMO_ERROR_CODE_STALE, /* Stale file handle */
    FIMO_ERROR_CODE_STRPIPE, /* Streams pipe error */
    FIMO_ERROR_CODE_TIME, /* Timer expired */
    FIMO_ERROR_CODE_TIMEDOUT, /* Connection timed out */
    FIMO_ERROR_CODE_TOOMANYREFS, /* Too many references: cannot splice */
    FIMO_ERROR_CODE_TXTBSY, /* Text file busy */
    FIMO_ERROR_CODE_UCLEAN, /* Structure needs cleaning */
    FIMO_ERROR_CODE_UNATCH, /* Protocol driver not attached */
    FIMO_ERROR_CODE_USERS, /* Too many users */
    FIMO_ERROR_CODE_WOULDBLOCK, /* Operation would block */
    FIMO_ERROR_CODE_XDEV, /* Invalid cross-device link */
    FIMO_ERROR_CODE_XFULL, /* Exchange full */
} FimoErrorCode;

/**
 * A system error code.
 */
#ifdef _WIN32
typedef DWORD FimoSystemErrorCode;
#else
typedef int FimoSystemErrorCode;
#endif

static_assert(sizeof(FimoSystemErrorCode) <= sizeof(void *), "FimoSystemErrorCode size too large");
static_assert(alignof(FimoSystemErrorCode) <= alignof(void *), "FimoSystemErrorCode alignment too large");

/**
 * An owned string returned from a `FimoResult`.
 */
typedef struct FimoResultString {
    const char *str;
    void (*release)(const char *str);
} FimoResultString;

/**
 * Core VTable of a `FimoResult`.
 *
 * Changing the VTable is a breaking change.
 */
typedef struct FimoResultVTableV0 {
    void (*release)(void *);
    FimoResultString (*error_name)(void *);
    FimoResultString (*error_description)(void *);
} FimoResultVTableV0;

/**
 * VTable of a `FimoResult`.
 */
typedef struct FimoResultVTable {
    FimoResultVTableV0 v0;
} FimoResultVTable;

/**
 * Status of an operation.
 */
typedef struct FimoResult {
    void *data;
    const FimoResultVTable *vtable;
} FimoResult;

/**
 * VTable for a `FimoResult` containing a static string.
 */
FIMO_EXPORT
extern const FimoResultVTable FIMO_IMPL_RESULT_STATIC_STRING_VTABLE;

/**
 * VTable for a `FimoResult` containing a dynamic string.
 */
FIMO_EXPORT
extern const FimoResultVTable FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE;

/**
 * VTable for a `FimoResult` containing a `FimoErrorCode`.
 */
FIMO_EXPORT
extern const FimoResultVTable FIMO_IMPL_RESULT_ERROR_CODE_VTABLE;

/**
 * VTable for a `FimoResult` containing a `FimoSystemErrorCode`.
 */
FIMO_EXPORT
extern const FimoResultVTable FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE;

/**
 * A result indicating that no error occurred.
 */
FIMO_EXPORT
extern const FimoResult FIMO_IMPL_RESULT_OK;

/**
 * A result indicating the failed construction of a `FimoResult`.
 */
FIMO_EXPORT
extern const FimoResult FIMO_IMPL_RESULT_INVALID_ERROR;

/**
 * Name of the `FIMO_IMPL_RESULT_OK` result.
 */
FIMO_EXPORT
extern const FimoResultString FIMO_IMPL_RESULT_OK_NAME;

/**
 * Description of the `FIMO_IMPL_RESULT_OK` result.
 */
FIMO_EXPORT
extern const FimoResultString FIMO_IMPL_RESULT_OK_DESCRIPTION;

/**
 * Get the name of the error code.
 *
 * In case of an unknown error this returns `"FIMO_ERROR_CODE_UNKNOWN"`.
 *
 * @param errnum the error code
 *
 * @return The name of the error.
 */
FIMO_EXPORT
FIMO_MUST_USE
const char *fimo_error_code_name(FimoErrorCode errnum);

/**
 * Get the description of the error code.
 *
 * In case of an unknown error this returns `"unknown error code"`.
 *
 * @param errnum the error code
 *
 * @return The description of the error.
 */
FIMO_EXPORT
FIMO_MUST_USE
const char *fimo_error_code_description(FimoErrorCode errnum);

/**
 * Constructs an error code from an errno error code.
 *
 * Unknown errno codes translate to an invalid error code.
 *
 * @param errnum: errno error code
 *
 * @return Status code.
 */
FIMO_EXPORT
FIMO_MUST_USE
FimoErrorCode fimo_error_code_from_errno(int errnum);

/**
 * Releases a `FimoResultString`.
 *
 * @param str string to release
 */
static FIMO_INLINE_ALWAYS void fimo_result_string_release(FimoResultString str) {
    if (str.release) {
        str.release(str.str);
    }
}

#ifdef __cplusplus
#define FIMO_IMPL_RESULT_INITIALIZER
#define FIMO_IMPL_RESULT_STRING_INITIALIZER
#else
#define FIMO_IMPL_RESULT_INITIALIZER (FimoResult)
#define FIMO_IMPL_RESULT_STRING_INITIALIZER (FimoResultString)
#endif

/**
 * Constructs a `FimoResult` from a static string.
 *
 * @param error error string
 *
 * @return Result instance.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResult fimo_result_from_static_string(const char *error) {
    if (!error) {
        return FIMO_IMPL_RESULT_INVALID_ERROR;
    }
    return FIMO_IMPL_RESULT_INITIALIZER{.data = (void *)error, .vtable = &FIMO_IMPL_RESULT_STATIC_STRING_VTABLE};
}

/**
 * Constructs a `FimoResult` from a dynamic string.
 *
 * The string must be allocated in a way that it can be freed with `fimo_free`.
 *
 * @param error error string
 *
 * @return Result instance.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResult fimo_result_from_dynamic_string(const char *error) {
    if (!error) {
        return FIMO_IMPL_RESULT_INVALID_ERROR;
    }
    return FIMO_IMPL_RESULT_INITIALIZER{.data = (void *)error, .vtable = &FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE};
}

/**
 * Constructs a `FimoResult` from a `FimoErrorCode`.
 *
 * @param code error code
 *
 * @return Result instance.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResult fimo_result_from_error_code(FimoErrorCode code) {
    if (code == FIMO_ERROR_CODE_OK) {
        return FIMO_IMPL_RESULT_OK;
    }
    if (code > FIMO_ERROR_CODE_MAX) {
        return FIMO_IMPL_RESULT_INVALID_ERROR;
    }
    return FIMO_IMPL_RESULT_INITIALIZER{.data = (void *)(intptr_t)code, .vtable = &FIMO_IMPL_RESULT_ERROR_CODE_VTABLE};
}

/**
 * Constructs a `FimoResult` from a `FimoSystemErrorCode`.
 *
 * @param code error code
 *
 * @return Result instace.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResult fimo_result_from_system_error_code(FimoSystemErrorCode code) {
    return FIMO_IMPL_RESULT_INITIALIZER{.data = (void *)(intptr_t)code,
                                        .vtable = &FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE};
}

/**
 * Checks whether the `FimoResult` signifies an error.
 *
 * @param result result
 *
 * @return Whether the result is an error.
 */
static FIMO_INLINE_ALWAYS bool fimo_result_is_error(FimoResult result) { return result.vtable != NULL; }

/**
 * Checks whether the `FimoResult` does not signify an error.
 *
 * @param result result
 *
 * @return Whether the result is not an error.
 */
static FIMO_INLINE_ALWAYS bool fimo_result_is_ok(FimoResult result) { return result.vtable == NULL; }

/**
 * Releases the `FimoResult`.
 *
 * The value may not be used again after releasing it.
 *
 * @param result result to release
 */
static FIMO_INLINE_ALWAYS void fimo_result_release(FimoResult result) {
    if (fimo_result_is_error(result) && result.vtable->v0.release) {
        result.vtable->v0.release(result.data);
    }
}

/**
 * Get the error name contained in the `FimoResult`.
 *
 * In case `result` does not contain an error this returns `"FIMO_IMPL_RESULT_OK_NAME"`.
 *
 * @param result the result
 *
 * @return Error name.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResultString fimo_result_error_name(FimoResult result) {
    if (fimo_result_is_ok(result)) {
        return FIMO_IMPL_RESULT_OK_NAME;
    }
    return result.vtable->v0.error_name(result.data);
}

/**
 * Get the error description contained in the `FimoResult`.
 *
 * In case `result` does not contain an error this returns `FIMO_IMPL_RESULT_OK_DESCRIPTION`.
 *
 * @param result the result
 *
 * @return Error description.
 */
FIMO_MUST_USE
static FIMO_INLINE_ALWAYS FimoResultString fimo_result_error_description(FimoResult result) {
    if (fimo_result_is_ok(result)) {
        return FIMO_IMPL_RESULT_OK_DESCRIPTION;
    }
    return result.vtable->v0.error_description(result.data);
}

#undef FIMO_IMPL_RESULT_INITIALIZER
#undef FIMO_IMPL_RESULT_STRING_INITIALIZER

#ifdef __cplusplus
}
#endif

#endif // FIMO_ERROR_H
