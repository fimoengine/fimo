#include <fimo_std/error.h>

#include <errno.h>
#include <stddef.h>

#ifdef FIMO_MACRO_HELPER_FUNCTIONS
FIMO_MUST_USE bool fimo_is_valid_error(FimoError errnum)
{
    return FIMO_IS_VALID_ERROR(errnum);
}

FIMO_MUST_USE bool fimo_is_error(FimoError errnum)
{
    return FIMO_IS_ERROR(errnum);
}
#endif // FIMO_MACRO_HELPER_FUNCTIONS

FIMO_MUST_USE const char* fimo_strerrorname(FimoError errnum, FimoError* err)
{
    if (err) {
        *err = FIMO_EOK;
    }

    switch (errnum) {
    case FIMO_EOK:
        return FIMO_STRINGIFY(FIMO_EOK);
    case FIMO_E2BIG:
        return FIMO_STRINGIFY(FIMO_E2BIG);
    case FIMO_EACCES:
        return FIMO_STRINGIFY(FIMO_EACCES);
    case FIMO_EADDRINUSE:
        return FIMO_STRINGIFY(FIMO_EADDRINUSE);
    case FIMO_EADDRNOTAVAIL:
        return FIMO_STRINGIFY(FIMO_EADDRNOTAVAIL);
    case FIMO_EAFNOSUPPORT:
        return FIMO_STRINGIFY(FIMO_EAFNOSUPPORT);
    case FIMO_EAGAIN:
        return FIMO_STRINGIFY(FIMO_EAGAIN);
    case FIMO_EALREADY:
        return FIMO_STRINGIFY(FIMO_EALREADY);
    case FIMO_EBADE:
        return FIMO_STRINGIFY(FIMO_EBADE);
    case FIMO_EBADF:
        return FIMO_STRINGIFY(FIMO_EBADF);
    case FIMO_EBADFD:
        return FIMO_STRINGIFY(FIMO_EBADFD);
    case FIMO_EBADMSG:
        return FIMO_STRINGIFY(FIMO_EBADMSG);
    case FIMO_EBADR:
        return FIMO_STRINGIFY(FIMO_EBADR);
    case FIMO_EBADRQC:
        return FIMO_STRINGIFY(FIMO_EBADRQC);
    case FIMO_EBADSLT:
        return FIMO_STRINGIFY(FIMO_EBADSLT);
    case FIMO_EBUSY:
        return FIMO_STRINGIFY(FIMO_EBUSY);
    case FIMO_ECANCELED:
        return FIMO_STRINGIFY(FIMO_ECANCELED);
    case FIMO_ECHILD:
        return FIMO_STRINGIFY(FIMO_ECHILD);
    case FIMO_ECHRNG:
        return FIMO_STRINGIFY(FIMO_ECHRNG);
    case FIMO_ECOMM:
        return FIMO_STRINGIFY(FIMO_ECOMM);
    case FIMO_ECONNABORTED:
        return FIMO_STRINGIFY(FIMO_ECONNABORTED);
    case FIMO_ECONNREFUSED:
        return FIMO_STRINGIFY(FIMO_ECONNREFUSED);
    case FIMO_ECONNRESET:
        return FIMO_STRINGIFY(FIMO_ECONNRESET);
    case FIMO_EDEADLK:
        return FIMO_STRINGIFY(FIMO_EDEADLK);
    case FIMO_EDEADLOCK:
        return FIMO_STRINGIFY(FIMO_EDEADLOCK);
    case FIMO_EDESTADDRREQ:
        return FIMO_STRINGIFY(FIMO_EDESTADDRREQ);
    case FIMO_EDOM:
        return FIMO_STRINGIFY(FIMO_EDOM);
    case FIMO_EDQUOT:
        return FIMO_STRINGIFY(FIMO_EDQUOT);
    case FIMO_EEXIST:
        return FIMO_STRINGIFY(FIMO_EEXIST);
    case FIMO_EFAULT:
        return FIMO_STRINGIFY(FIMO_EFAULT);
    case FIMO_EFBIG:
        return FIMO_STRINGIFY(FIMO_EFBIG);
    case FIMO_EHOSTDOWN:
        return FIMO_STRINGIFY(FIMO_EHOSTDOWN);
    case FIMO_EHOSTUNREACH:
        return FIMO_STRINGIFY(FIMO_EHOSTUNREACH);
    case FIMO_EHWPOISON:
        return FIMO_STRINGIFY(FIMO_EHWPOISON);
    case FIMO_EIDRM:
        return FIMO_STRINGIFY(FIMO_EIDRM);
    case FIMO_EILSEQ:
        return FIMO_STRINGIFY(FIMO_EILSEQ);
    case FIMO_EINPROGRESS:
        return FIMO_STRINGIFY(FIMO_EINPROGRESS);
    case FIMO_EINTR:
        return FIMO_STRINGIFY(FIMO_EINTR);
    case FIMO_EINVAL:
        return FIMO_STRINGIFY(FIMO_EINVAL);
    case FIMO_EIO:
        return FIMO_STRINGIFY(FIMO_EIO);
    case FIMO_EISCONN:
        return FIMO_STRINGIFY(FIMO_EISCONN);
    case FIMO_EISDIR:
        return FIMO_STRINGIFY(FIMO_EISDIR);
    case FIMO_EISNAM:
        return FIMO_STRINGIFY(FIMO_EISNAM);
    case FIMO_EKEYEXPIRED:
        return FIMO_STRINGIFY(FIMO_EKEYEXPIRED);
    case FIMO_EKEYREJECTED:
        return FIMO_STRINGIFY(FIMO_EKEYREJECTED);
    case FIMO_EKEYREVOKED:
        return FIMO_STRINGIFY(FIMO_EKEYREVOKED);
    case FIMO_EL2HLT:
        return FIMO_STRINGIFY(FIMO_EL2HLT);
    case FIMO_EL2NSYNC:
        return FIMO_STRINGIFY(FIMO_EL2NSYNC);
    case FIMO_EL3HLT:
        return FIMO_STRINGIFY(FIMO_EL3HLT);
    case FIMO_EL3RST:
        return FIMO_STRINGIFY(FIMO_EL3RST);
    case FIMO_ELIBACC:
        return FIMO_STRINGIFY(FIMO_ELIBACC);
    case FIMO_ELIBBAD:
        return FIMO_STRINGIFY(FIMO_ELIBBAD);
    case FIMO_ELIBMAX:
        return FIMO_STRINGIFY(FIMO_ELIBMAX);
    case FIMO_ELIBSCN:
        return FIMO_STRINGIFY(FIMO_ELIBSCN);
    case FIMO_ELIBEXEC:
        return FIMO_STRINGIFY(FIMO_ELIBEXEC);
    case FIMO_ELNRNG:
        return FIMO_STRINGIFY(FIMO_ELNRNG);
    case FIMO_ELOOP:
        return FIMO_STRINGIFY(FIMO_ELOOP);
    case FIMO_EMEDIUMTYPE:
        return FIMO_STRINGIFY(FIMO_EMEDIUMTYPE);
    case FIMO_EMFILE:
        return FIMO_STRINGIFY(FIMO_EMFILE);
    case FIMO_EMLINK:
        return FIMO_STRINGIFY(FIMO_EMLINK);
    case FIMO_EMSGSIZE:
        return FIMO_STRINGIFY(FIMO_EMSGSIZE);
    case FIMO_EMULTIHOP:
        return FIMO_STRINGIFY(FIMO_EMULTIHOP);
    case FIMO_ENAMETOOLONG:
        return FIMO_STRINGIFY(FIMO_ENAMETOOLONG);
    case FIMO_ENETDOWN:
        return FIMO_STRINGIFY(FIMO_ENETDOWN);
    case FIMO_ENETRESET:
        return FIMO_STRINGIFY(FIMO_ENETRESET);
    case FIMO_ENETUNREACH:
        return FIMO_STRINGIFY(FIMO_ENETUNREACH);
    case FIMO_ENFILE:
        return FIMO_STRINGIFY(FIMO_ENFILE);
    case FIMO_ENOANO:
        return FIMO_STRINGIFY(FIMO_ENOANO);
    case FIMO_ENOBUFS:
        return FIMO_STRINGIFY(FIMO_ENOBUFS);
    case FIMO_ENODATA:
        return FIMO_STRINGIFY(FIMO_ENODATA);
    case FIMO_ENODEV:
        return FIMO_STRINGIFY(FIMO_ENODEV);
    case FIMO_ENOENT:
        return FIMO_STRINGIFY(FIMO_ENOENT);
    case FIMO_ENOEXEC:
        return FIMO_STRINGIFY(FIMO_ENOEXEC);
    case FIMO_ENOKEY:
        return FIMO_STRINGIFY(FIMO_ENOKEY);
    case FIMO_ENOLCK:
        return FIMO_STRINGIFY(FIMO_ENOLCK);
    case FIMO_ENOLINK:
        return FIMO_STRINGIFY(FIMO_ENOLINK);
    case FIMO_ENOMEDIUM:
        return FIMO_STRINGIFY(FIMO_ENOMEDIUM);
    case FIMO_ENOMEM:
        return FIMO_STRINGIFY(FIMO_ENOMEM);
    case FIMO_ENOMSG:
        return FIMO_STRINGIFY(FIMO_ENOMSG);
    case FIMO_ENONET:
        return FIMO_STRINGIFY(FIMO_ENONET);
    case FIMO_ENOPKG:
        return FIMO_STRINGIFY(FIMO_ENOPKG);
    case FIMO_ENOPROTOOPT:
        return FIMO_STRINGIFY(FIMO_ENOPROTOOPT);
    case FIMO_ENOSPC:
        return FIMO_STRINGIFY(FIMO_ENOSPC);
    case FIMO_ENOSR:
        return FIMO_STRINGIFY(FIMO_ENOSR);
    case FIMO_ENOSTR:
        return FIMO_STRINGIFY(FIMO_ENOSTR);
    case FIMO_ENOSYS:
        return FIMO_STRINGIFY(FIMO_ENOSYS);
    case FIMO_ENOTBLK:
        return FIMO_STRINGIFY(FIMO_ENOTBLK);
    case FIMO_ENOTCONN:
        return FIMO_STRINGIFY(FIMO_ENOTCONN);
    case FIMO_ENOTDIR:
        return FIMO_STRINGIFY(FIMO_ENOTDIR);
    case FIMO_ENOTEMPTY:
        return FIMO_STRINGIFY(FIMO_ENOTEMPTY);
    case FIMO_ENOTRECOVERABLE:
        return FIMO_STRINGIFY(FIMO_ENOTRECOVERABLE);
    case FIMO_ENOTSOCK:
        return FIMO_STRINGIFY(FIMO_ENOTSOCK);
    case FIMO_ENOTSUP:
        return FIMO_STRINGIFY(FIMO_ENOTSUP);
    case FIMO_ENOTTY:
        return FIMO_STRINGIFY(FIMO_ENOTTY);
    case FIMO_ENOTUNIQ:
        return FIMO_STRINGIFY(FIMO_ENOTUNIQ);
    case FIMO_ENXIO:
        return FIMO_STRINGIFY(FIMO_ENXIO);
    case FIMO_EOPNOTSUPP:
        return FIMO_STRINGIFY(FIMO_EOPNOTSUPP);
    case FIMO_EOVERFLOW:
        return FIMO_STRINGIFY(FIMO_EOVERFLOW);
    case FIMO_EOWNERDEAD:
        return FIMO_STRINGIFY(FIMO_EOWNERDEAD);
    case FIMO_EPERM:
        return FIMO_STRINGIFY(FIMO_EPERM);
    case FIMO_EPFNOSUPPORT:
        return FIMO_STRINGIFY(FIMO_EPFNOSUPPORT);
    case FIMO_EPIPE:
        return FIMO_STRINGIFY(FIMO_EPIPE);
    case FIMO_EPROTO:
        return FIMO_STRINGIFY(FIMO_EPROTO);
    case FIMO_EPROTONOSUPPORT:
        return FIMO_STRINGIFY(FIMO_EPROTONOSUPPORT);
    case FIMO_EPROTOTYPE:
        return FIMO_STRINGIFY(FIMO_EPROTOTYPE);
    case FIMO_ERANGE:
        return FIMO_STRINGIFY(FIMO_ERANGE);
    case FIMO_EREMCHG:
        return FIMO_STRINGIFY(FIMO_EREMCHG);
    case FIMO_EREMOTE:
        return FIMO_STRINGIFY(FIMO_EREMOTE);
    case FIMO_EREMOTEIO:
        return FIMO_STRINGIFY(FIMO_EREMOTEIO);
    case FIMO_ERESTART:
        return FIMO_STRINGIFY(FIMO_ERESTART);
    case FIMO_ERFKILL:
        return FIMO_STRINGIFY(FIMO_ERFKILL);
    case FIMO_EROFS:
        return FIMO_STRINGIFY(FIMO_EROFS);
    case FIMO_ESHUTDOWN:
        return FIMO_STRINGIFY(FIMO_ESHUTDOWN);
    case FIMO_ESPIPE:
        return FIMO_STRINGIFY(FIMO_ESPIPE);
    case FIMO_ESOCKTNOSUPPORT:
        return FIMO_STRINGIFY(FIMO_ESOCKTNOSUPPORT);
    case FIMO_ESRCH:
        return FIMO_STRINGIFY(FIMO_ESRCH);
    case FIMO_ESTALE:
        return FIMO_STRINGIFY(FIMO_ESTALE);
    case FIMO_ESTRPIPE:
        return FIMO_STRINGIFY(FIMO_ESTRPIPE);
    case FIMO_ETIME:
        return FIMO_STRINGIFY(FIMO_ETIME);
    case FIMO_ETIMEDOUT:
        return FIMO_STRINGIFY(FIMO_ETIMEDOUT);
    case FIMO_ETOOMANYREFS:
        return FIMO_STRINGIFY(FIMO_ETOOMANYREFS);
    case FIMO_ETXTBSY:
        return FIMO_STRINGIFY(FIMO_ETXTBSY);
    case FIMO_EUCLEAN:
        return FIMO_STRINGIFY(FIMO_EUCLEAN);
    case FIMO_EUNATCH:
        return FIMO_STRINGIFY(FIMO_EUNATCH);
    case FIMO_EUSERS:
        return FIMO_STRINGIFY(FIMO_EUSERS);
    case FIMO_EWOULDBLOCK:
        return FIMO_STRINGIFY(FIMO_EWOULDBLOCK);
    case FIMO_EXDEV:
        return FIMO_STRINGIFY(FIMO_EXDEV);
    case FIMO_EXFULL:
        return FIMO_STRINGIFY(FIMO_EXFULL);
    case FIMO_EUNKNOWN:
        return FIMO_STRINGIFY(FIMO_EUNKNOWN);
    }

    if (err) {
        *err = FIMO_EINVAL;
    }
    return "Unknown error number";
}

FIMO_MUST_USE const char* fimo_strerrordesc(FimoError errnum, FimoError* err)
{
    if (err) {
        *err = FIMO_EOK;
    }

    switch (errnum) {
    case FIMO_EOK:
        return "Operation completed successfully";
    case FIMO_E2BIG:
        return "Argument list too long";
    case FIMO_EACCES:
        return "Permission denied";
    case FIMO_EADDRINUSE:
        return "Address already in use";
    case FIMO_EADDRNOTAVAIL:
        return "Address not available";
    case FIMO_EAFNOSUPPORT:
        return "Address family not supported";
    case FIMO_EAGAIN:
        return "Resource temporarily unavailable";
    case FIMO_EALREADY:
        return "Connection already in progress";
    case FIMO_EBADE:
        return "Invalid exchange";
    case FIMO_EBADF:
        return "Bad file descriptor";
    case FIMO_EBADFD:
        return "File descriptor in bad state";
    case FIMO_EBADMSG:
        return "Bad message";
    case FIMO_EBADR:
        return "Invalid request descriptor";
    case FIMO_EBADRQC:
        return "Invalid request code";
    case FIMO_EBADSLT:
        return "Invalid slot";
    case FIMO_EBUSY:
        return "Device or resource busy";
    case FIMO_ECANCELED:
        return "Operation canceled";
    case FIMO_ECHILD:
        return "No child processes";
    case FIMO_ECHRNG:
        return "Channel number out of range";
    case FIMO_ECOMM:
        return "Communication error on send";
    case FIMO_ECONNABORTED:
        return "Connection aborted";
    case FIMO_ECONNREFUSED:
        return "Connection refused";
    case FIMO_ECONNRESET:
        return "Connection reset";
#if defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
    case FIMO_EDEADLK:
    case FIMO_EDEADLOCK:
        return "Resource deadlock avoided";
#else
    case FIMO_EDEADLK:
        return "Resource deadlock avoided";
    case FIMO_EDEADLOCK:
        return "File locking deadlock error";
#endif // defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
    case FIMO_EDESTADDRREQ:
        return "Destination address required";
    case FIMO_EDOM:
        return "Mathematics argument out of domain of function";
    case FIMO_EDQUOT:
        return "Disk quota exceeded";
    case FIMO_EEXIST:
        return "File exists";
    case FIMO_EFAULT:
        return "Bad address";
    case FIMO_EFBIG:
        return "File too large";
    case FIMO_EHOSTDOWN:
        return "Host is down";
    case FIMO_EHOSTUNREACH:
        return "Host is unreachable";
    case FIMO_EHWPOISON:
        return "Memory page has hardware error";
    case FIMO_EIDRM:
        return "Identifier removed";
    case FIMO_EILSEQ:
        return "Invalid or incomplete multibyte or wide character";
    case FIMO_EINPROGRESS:
        return "Operation in progress";
    case FIMO_EINTR:
        return "Interrupted function call";
    case FIMO_EINVAL:
        return "Invalid argument";
    case FIMO_EIO:
        return "Input/output error";
    case FIMO_EISCONN:
        return "Socket is connected";
    case FIMO_EISDIR:
        return "Is a directory";
    case FIMO_EISNAM:
        return "Is a named type file";
    case FIMO_EKEYEXPIRED:
        return "Key has expired";
    case FIMO_EKEYREJECTED:
        return "Key was rejected by service";
    case FIMO_EKEYREVOKED:
        return "Key has been revoked";
    case FIMO_EL2HLT:
        return "Level 2 halted";
    case FIMO_EL2NSYNC:
        return "Level 2 not synchronized";
    case FIMO_EL3HLT:
        return "Level 3 halted";
    case FIMO_EL3RST:
        return "Level 3 reset";
    case FIMO_ELIBACC:
        return "Cannot access a needed shared library";
    case FIMO_ELIBBAD:
        return "Accessing a corrupted shared library";
    case FIMO_ELIBMAX:
        return "Attempting to link in too many shared libraries";
    case FIMO_ELIBSCN:
        return ".lib section in a.out corrupted";
    case FIMO_ELIBEXEC:
        return "Cannot exec a shared library directly";
    case FIMO_ELNRNG:
        return "Link number out of range";
    case FIMO_ELOOP:
        return "Too many levels of symbolic links";
    case FIMO_EMEDIUMTYPE:
        return "Wrong medium type";
    case FIMO_EMFILE:
        return "Too many open files";
    case FIMO_EMLINK:
        return "Too many links";
    case FIMO_EMSGSIZE:
        return "Message too long";
    case FIMO_EMULTIHOP:
        return "Multihop attempted";
    case FIMO_ENAMETOOLONG:
        return "Filename too long";
    case FIMO_ENETDOWN:
        return "Network is down";
    case FIMO_ENETRESET:
        return "Connection aborted by network";
    case FIMO_ENETUNREACH:
        return "Network unreachable";
    case FIMO_ENFILE:
        return "Too many open files in system";
    case FIMO_ENOANO:
        return "No anode";
    case FIMO_ENOBUFS:
        return "No buffer space available";
    case FIMO_ENODATA:
        return "The named attribute does not exist, or the process has no access to this attribute";
    case FIMO_ENODEV:
        return "No such device";
    case FIMO_ENOENT:
        return "No such file or directory";
    case FIMO_ENOEXEC:
        return "Exec format error";
    case FIMO_ENOKEY:
        return "Required key not available";
    case FIMO_ENOLCK:
        return "No locks available";
    case FIMO_ENOLINK:
        return "Link has been severed";
    case FIMO_ENOMEDIUM:
        return "No medium found";
    case FIMO_ENOMEM:
        return "Not enough space/cannot allocate memory";
    case FIMO_ENOMSG:
        return "No message of the desired type";
    case FIMO_ENONET:
        return "Machine is not on the network";
    case FIMO_ENOPKG:
        return "Package not installed";
    case FIMO_ENOPROTOOPT:
        return "Protocol not available";
    case FIMO_ENOSPC:
        return "No space left on device";
    case FIMO_ENOSR:
        return "No STREAM resources";
    case FIMO_ENOSTR:
        return "Not a STREAM";
    case FIMO_ENOSYS:
        return "Function not implemented";
    case FIMO_ENOTBLK:
        return "Block device required";
    case FIMO_ENOTCONN:
        return "The socket is not connected";
    case FIMO_ENOTDIR:
        return "Not a directory";
    case FIMO_ENOTEMPTY:
        return "Directory not empty";
    case FIMO_ENOTRECOVERABLE:
        return "State not recoverable";
    case FIMO_ENOTSOCK:
        return "Not a socket";
    case FIMO_ENOTSUP:
        return "Operation not supported";
    case FIMO_ENOTTY:
        return "Inappropriate I/O control operation";
    case FIMO_ENOTUNIQ:
        return "Name not unique on network";
    case FIMO_ENXIO:
        return "No such device or address";
    case FIMO_EOPNOTSUPP:
        return "Operation not supported on socket";
    case FIMO_EOVERFLOW:
        return "Value too large to be stored in data type";
    case FIMO_EOWNERDEAD:
        return "Owner died";
    case FIMO_EPERM:
        return "Operation not permitted";
    case FIMO_EPFNOSUPPORT:
        return "Protocol family not supported";
    case FIMO_EPIPE:
        return "Broken pipe";
    case FIMO_EPROTO:
        return "Protocol error";
    case FIMO_EPROTONOSUPPORT:
        return "Protocol not supported";
    case FIMO_EPROTOTYPE:
        return "Protocol wrong type for socket";
    case FIMO_ERANGE:
        return "Result too large";
    case FIMO_EREMCHG:
        return "Remote address changed";
    case FIMO_EREMOTE:
        return "Object is remote";
    case FIMO_EREMOTEIO:
        return "Remote I/O error";
    case FIMO_ERESTART:
        return "Interrupted system call should be restarted";
    case FIMO_ERFKILL:
        return "Operation not possible due to RF-kill";
    case FIMO_EROFS:
        return "Read-only filesystem";
    case FIMO_ESHUTDOWN:
        return "Cannot send after transport endpoint shutdown";
    case FIMO_ESPIPE:
        return "Invalid seek";
    case FIMO_ESOCKTNOSUPPORT:
        return "Socket type not supported";
    case FIMO_ESRCH:
        return "No such process";
    case FIMO_ESTALE:
        return "Stale file handle";
    case FIMO_ESTRPIPE:
        return "Streams pipe error";
    case FIMO_ETIME:
        return "Timer expired";
    case FIMO_ETIMEDOUT:
        return "Connection timed out";
    case FIMO_ETOOMANYREFS:
        return "Too many references: cannot splice";
    case FIMO_ETXTBSY:
        return "Text file busy";
    case FIMO_EUCLEAN:
        return "Structure needs cleaning";
    case FIMO_EUNATCH:
        return "Protocol driver not attached";
    case FIMO_EUSERS:
        return "Too many users";
    case FIMO_EWOULDBLOCK:
        return "Operation would block";
    case FIMO_EXDEV:
        return "Invalid cross-device link";
    case FIMO_EXFULL:
        return "Exchange full";
    case FIMO_EUNKNOWN:
        return "Unknown error";
    }

    if (err) {
        *err = FIMO_EINVAL;
    }
    return "Unknown error number";
}

FIMO_MUST_USE FimoError fimo_error_from_errno(int errnum)
{
    switch (errnum) {
    case 0:
        return FIMO_EOK;

#ifdef E2BIG
    case E2BIG:
        return FIMO_E2BIG;
#endif // E2BIG

#ifdef EACCES
    case EACCES:
        return FIMO_EACCES;
#endif // EACCES

#ifdef EADDRINUSE
    case EADDRINUSE:
        return FIMO_EADDRINUSE;
#endif // EADDRINUSE

#ifdef EADDRNOTAVAIL
    case EADDRNOTAVAIL:
        return FIMO_EADDRNOTAVAIL;
#endif // EADDRNOTAVAIL

#ifdef EAFNOSUPPORT
    case EAFNOSUPPORT:
        return FIMO_EAFNOSUPPORT;
#endif // EAFNOSUPPORT

// EAGAIN and EWOULDBLOCK are allowed to be the same value
#if defined(EAGAIN) && defined(EWOULDBLOCK) && EAGAIN == EWOULDBLOCK
    case EAGAIN:
        return FIMO_EAGAIN;
#else
#ifdef EAGAIN
    case EAGAIN:
        return FIMO_EAGAIN;
#endif // EAGAIN

#ifdef EWOULDBLOCK
    case EWOULDBLOCK:
        return FIMO_EWOULDBLOCK;
#endif // EWOULDBLOCK
#endif // defined(EAGAIN) && defined(EWOULDBLOCK) && EAGAIN == EWOULDBLOCK

#ifdef EALREADY
    case EALREADY:
        return FIMO_EALREADY;
#endif // EALREADY

#ifdef EBADE
    case EBADE:
        return FIMO_EBADE;
#endif // EBADE

#ifdef EBADF
    case EBADF:
        return FIMO_EBADF;
#endif // EBADF

#ifdef EBADFD
    case EBADFD:
        return FIMO_EBADFD;
#endif // EBADFD

#ifdef EBADMSG
    case EBADMSG:
        return FIMO_EBADMSG;
#endif // EBADMSG

#ifdef EBADR
    case EBADR:
        return FIMO_EBADR;
#endif // EBADR

#ifdef EBADRQC
    case EBADRQC:
        return FIMO_EBADRQC;
#endif // EBADRQC

#ifdef EBADSLT
    case EBADSLT:
        return FIMO_EBADSLT;
#endif // EBADSLT

#ifdef EBUSY
    case EBUSY:
        return FIMO_EBUSY;
#endif // EBUSY

#ifdef ECANCELED
    case ECANCELED:
        return FIMO_ECANCELED;
#endif // ECANCELED

#ifdef ECHILD
    case ECHILD:
        return FIMO_ECHILD;
#endif // ECHILD

#ifdef ECHRNG
    case ECHRNG:
        return FIMO_ECHRNG;
#endif // ECHRNG

#ifdef ECOMM
    case ECOMM:
        return FIMO_ECOMM;
#endif // ECOMM

#ifdef ECONNABORTED
    case ECONNABORTED:
        return FIMO_ECONNABORTED;
#endif // ECONNABORTED

#ifdef ECONNREFUSED
    case ECONNREFUSED:
        return FIMO_ECONNREFUSED;
#endif // ECONNREFUSED

#ifdef ECONNRESET
    case ECONNRESET:
        return FIMO_ECONNRESET;
#endif // ECONNRESET

// EDEADLOCK is usually a synonym for EDEADLK
#if defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
    case EDEADLK:
        return FIMO_EDEADLK;
#else
#ifdef EDEADLK
    case EDEADLK:
        return FIMO_EDEADLK;
#endif // EDEADLK
#ifdef EDEADLOCK
    case EDEADLOCK:
        return FIMO_EDEADLOCK;
#endif // EDEADLOCK
#endif // defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK

#ifdef EDESTADDRREQ
    case EDESTADDRREQ:
        return FIMO_EDESTADDRREQ;
#endif // EDESTADDRREQ

#ifdef EDOM
    case EDOM:
        return FIMO_EDOM;
#endif // EDOM

#ifdef EDQUOT
    case EDQUOT:
        return FIMO_EDQUOT;
#endif // EDQUOT

#ifdef EEXIST
    case EEXIST:
        return FIMO_EEXIST;
#endif // EEXIST

#ifdef EFAULT
    case EFAULT:
        return FIMO_EFAULT;
#endif // EFAULT

#ifdef EFBIG
    case EFBIG:
        return FIMO_EFBIG;
#endif // EFBIG

#ifdef EHOSTDOWN
    case EHOSTDOWN:
        return FIMO_EHOSTDOWN;
#endif // EHOSTDOWN

#ifdef EHOSTUNREACH
    case EHOSTUNREACH:
        return FIMO_EHOSTUNREACH;
#endif // EHOSTUNREACH

#ifdef EHWPOISON
    case EHWPOISON:
        return FIMO_EHWPOISON;
#endif // EHWPOISON

#ifdef EIDRM
    case EIDRM:
        return FIMO_EIDRM;
#endif // EIDRM

#ifdef EILSEQ
    case EILSEQ:
        return FIMO_EILSEQ;
#endif // EILSEQ

#ifdef EINPROGRESS
    case EINPROGRESS:
        return FIMO_EINPROGRESS;
#endif // EINPROGRESS

#ifdef EINTR
    case EINTR:
        return FIMO_EINTR;
#endif // EINTR

#ifdef EINVAL
    case EINVAL:
        return FIMO_EINVAL;
#endif // EINVAL

#ifdef EIO
    case EIO:
        return FIMO_EIO;
#endif // EIO

#ifdef EISCONN
    case EISCONN:
        return FIMO_EISCONN;
#endif // EISCONN

#ifdef EISDIR
    case EISDIR:
        return FIMO_EISDIR;
#endif // EISDIR

#ifdef EISNAM
    case EISNAM:
        return FIMO_EISNAM;
#endif // EISNAM

#ifdef EKEYEXPIRED
    case EKEYEXPIRED:
        return FIMO_EKEYEXPIRED;
#endif // EKEYEXPIRED

#ifdef EKEYREJECTED
    case EKEYREJECTED:
        return FIMO_EKEYREJECTED;
#endif // EKEYREJECTED

#ifdef EKEYREVOKED
    case EKEYREVOKED:
        return FIMO_EKEYREVOKED;
#endif // EKEYREVOKED

#ifdef EL2HLT
    case EL2HLT:
        return FIMO_EL2HLT;
#endif // EL2HLT

#ifdef EL2NSYNC
    case EL2NSYNC:
        return FIMO_EL2NSYNC;
#endif // EL2NSYNC

#ifdef EL3HLT
    case EL3HLT:
        return FIMO_EL3HLT;
#endif // EL3HLT

#ifdef EL3RST
    case EL3RST:
        return FIMO_EL3RST;
#endif // EL3RST

#ifdef ELIBACC
    case ELIBACC:
        return FIMO_ELIBACC;
#endif // ELIBACC

#ifdef ELIBBAD
    case ELIBBAD:
        return FIMO_ELIBBAD;
#endif // ELIBBAD

#ifdef ELIBMAX
    case ELIBMAX:
        return FIMO_ELIBMAX;
#endif // ELIBMAX

#ifdef ELIBSCN
    case ELIBSCN:
        return FIMO_ELIBSCN;
#endif // ELIBSCN

#ifdef ELIBEXEC
    case ELIBEXEC:
        return FIMO_ELIBEXEC;
#endif // ELIBEXEC

#ifdef ELNRNG
    case ELNRNG:
        return FIMO_ELNRNG;
#endif // ELNRNG

#ifdef ELOOP
    case ELOOP:
        return FIMO_ELOOP;
#endif // ELOOP

#ifdef EMEDIUMTYPE
    case EMEDIUMTYPE:
        return FIMO_EMEDIUMTYPE;
#endif // EMEDIUMTYPE

#ifdef EMFILE
    case EMFILE:
        return FIMO_EMFILE;
#endif // EMFILE

#ifdef EMLINK
    case EMLINK:
        return FIMO_EMLINK;
#endif // EMLINK

#ifdef EMSGSIZE
    case EMSGSIZE:
        return FIMO_EMSGSIZE;
#endif // EMSGSIZE

#ifdef EMULTIHOP
    case EMULTIHOP:
        return FIMO_EMULTIHOP;
#endif // EMULTIHOP

#ifdef ENAMETOOLONG
    case ENAMETOOLONG:
        return FIMO_ENAMETOOLONG;
#endif // ENAMETOOLONG

#ifdef ENETDOWN
    case ENETDOWN:
        return FIMO_ENETDOWN;
#endif // ENETDOWN

#ifdef ENETRESET
    case ENETRESET:
        return FIMO_ENETRESET;
#endif // ENETRESET

#ifdef ENETUNREACH
    case ENETUNREACH:
        return FIMO_ENETUNREACH;
#endif // ENETUNREACH

#ifdef ENFILE
    case ENFILE:
        return FIMO_ENFILE;
#endif // ENFILE

#ifdef ENOANO
    case ENOANO:
        return FIMO_ENOANO;
#endif // ENOANO

#ifdef ENOBUFS
    case ENOBUFS:
        return FIMO_ENOBUFS;
#endif // ENOBUFS

#ifdef ENODATA
    case ENODATA:
        return FIMO_ENODATA;
#endif // ENODATA

#ifdef ENODEV
    case ENODEV:
        return FIMO_ENODEV;
#endif // ENODEV

#ifdef ENOENT
    case ENOENT:
        return FIMO_ENOENT;
#endif // ENOENT

#ifdef ENOEXEC
    case ENOEXEC:
        return FIMO_ENOEXEC;
#endif // ENOEXEC

#ifdef ENOKEY
    case ENOKEY:
        return FIMO_ENOKEY;
#endif // ENOKEY

#ifdef ENOLCK
    case ENOLCK:
        return FIMO_ENOLCK;
#endif // ENOLCK

#ifdef ENOLINK
    case ENOLINK:
        return FIMO_ENOLINK;
#endif // ENOLINK

#ifdef ENOMEDIUM
    case ENOMEDIUM:
        return FIMO_ENOMEDIUM;
#endif // ENOMEDIUM

#ifdef ENOMEM
    case ENOMEM:
        return FIMO_ENOMEM;
#endif // ENOMEM

#ifdef ENOMSG
    case ENOMSG:
        return FIMO_ENOMSG;
#endif // ENOMSG

#ifdef ENONET
    case ENONET:
        return FIMO_ENONET;
#endif // ENONET

#ifdef ENOPKG
    case ENOPKG:
        return FIMO_ENOPKG;
#endif // ENOPKG

#ifdef ENOPROTOOPT
    case ENOPROTOOPT:
        return FIMO_ENOPROTOOPT;
#endif // ENOPROTOOPT

#ifdef ENOSPC
    case ENOSPC:
        return FIMO_ENOSPC;
#endif // ENOSPC

#ifdef ENOSR
    case ENOSR:
        return FIMO_ENOSR;
#endif // ENOSR

#ifdef ENOSTR
    case ENOSTR:
        return FIMO_ENOSTR;
#endif // ENOSTR

#ifdef ENOSYS
    case ENOSYS:
        return FIMO_ENOSYS;
#endif // ENOSYS

#ifdef ENOTBLK
    case ENOTBLK:
        return FIMO_ENOTBLK;
#endif // ENOTBLK

#ifdef ENOTCONN
    case ENOTCONN:
        return FIMO_ENOTCONN;
#endif // ENOTCONN

#ifdef ENOTDIR
    case ENOTDIR:
        return FIMO_ENOTDIR;
#endif // ENOTDIR

#ifdef ENOTEMPTY
    case ENOTEMPTY:
        return FIMO_ENOTEMPTY;
#endif // ENOTEMPTY

#ifdef ENOTRECOVERABLE
    case ENOTRECOVERABLE:
        return FIMO_ENOTRECOVERABLE;
#endif // ENOTRECOVERABLE

#ifdef ENOTSOCK
    case ENOTSOCK:
        return FIMO_ENOTSOCK;
#endif // ENOTSOCK

// EOPNOTSUPP and ENOTSUP have the same value on linux.
#if defined(ENOTSUP) && defined(EOPNOTSUPP) && ENOTSUP == EOPNOTSUPP
    case ENOTSUP:
        return FIMO_ENOTSUP;
#else
#ifdef ENOTSUP
    case ENOTSUP:
        return FIMO_ENOTSUP;
#endif // ENOTSUP

#ifdef EOPNOTSUPP
    case EOPNOTSUPP:
        return FIMO_EOPNOTSUPP;
#endif // EOPNOTSUPP
#endif // defined(ENOTSUP) && defined(EOPNOTSUPP) && ENOTSUP == EOPNOTSUPP

#ifdef ENOTTY
    case ENOTTY:
        return FIMO_ENOTTY;
#endif // ENOTTY

#ifdef ENOTUNIQ
    case ENOTUNIQ:
        return FIMO_ENOTUNIQ;
#endif // ENOTUNIQ

#ifdef ENXIO
    case ENXIO:
        return FIMO_ENXIO;
#endif // ENXIO

#ifdef EOVERFLOW
    case EOVERFLOW:
        return FIMO_EOVERFLOW;
#endif // EOVERFLOW

#ifdef EOWNERDEAD
    case EOWNERDEAD:
        return FIMO_EOWNERDEAD;
#endif // EOWNERDEAD

#ifdef EPERM
    case EPERM:
        return FIMO_EPERM;
#endif // EPERM

#ifdef EPFNOSUPPORT
    case EPFNOSUPPORT:
        return FIMO_EPFNOSUPPORT;
#endif // EPFNOSUPPORT

#ifdef EPIPE
    case EPIPE:
        return FIMO_EPIPE;
#endif // EPIPE

#ifdef EPROTO
    case EPROTO:
        return FIMO_EPROTO;
#endif // EPROTO

#ifdef EPROTONOSUPPORT
    case EPROTONOSUPPORT:
        return FIMO_EPROTONOSUPPORT;
#endif // EPROTONOSUPPORT

#ifdef EPROTOTYPE
    case EPROTOTYPE:
        return FIMO_EPROTOTYPE;
#endif // EPROTOTYPE

#ifdef ERANGE
    case ERANGE:
        return FIMO_ERANGE;
#endif // ERANGE

#ifdef EREMCHG
    case EREMCHG:
        return FIMO_EREMCHG;
#endif // EREMCHG

#ifdef EREMOTE
    case EREMOTE:
        return FIMO_EREMOTE;
#endif // EREMOTE

#ifdef EREMOTEIO
    case EREMOTEIO:
        return FIMO_EREMOTEIO;
#endif // EREMOTEIO

#ifdef ERESTART
    case ERESTART:
        return FIMO_ERESTART;
#endif // ERESTART

#ifdef ERFKILL
    case ERFKILL:
        return FIMO_ERFKILL;
#endif // ERFKILL

#ifdef EROFS
    case EROFS:
        return FIMO_EROFS;
#endif // EROFS

#ifdef ESHUTDOWN
    case ESHUTDOWN:
        return FIMO_ESHUTDOWN;
#endif // ESHUTDOWN

#ifdef ESPIPE
    case ESPIPE:
        return FIMO_ESPIPE;
#endif // ESPIPE

#ifdef ESOCKTNOSUPPORT
    case ESOCKTNOSUPPORT:
        return FIMO_ESOCKTNOSUPPORT;
#endif // ESOCKTNOSUPPORT

#ifdef ESRCH
    case ESRCH:
        return FIMO_ESRCH;
#endif // ESRCH

#ifdef ESTALE
    case ESTALE:
        return FIMO_ESTALE;
#endif // ESTALE

#ifdef ESTRPIPE
    case ESTRPIPE:
        return FIMO_ESTRPIPE;
#endif // ESTRPIPE

#ifdef ETIME
    case ETIME:
        return FIMO_ETIME;
#endif // ETIME

#ifdef ETIMEDOUT
    case ETIMEDOUT:
        return FIMO_ETIMEDOUT;
#endif // ETIMEDOUT

#ifdef ETOOMANYREFS
    case ETOOMANYREFS:
        return FIMO_ETOOMANYREFS;
#endif // ETOOMANYREFS

#ifdef ETXTBSY
    case ETXTBSY:
        return FIMO_ETXTBSY;
#endif // ETXTBSY

#ifdef EUCLEAN
    case EUCLEAN:
        return FIMO_EUCLEAN;
#endif // EUCLEAN

#ifdef EUNATCH
    case EUNATCH:
        return FIMO_EUNATCH;
#endif // EUNATCH

#ifdef EUSERS
    case EUSERS:
        return FIMO_EUSERS;
#endif // EUSERS

#ifdef EXDEV
    case EXDEV:
        return FIMO_EXDEV;
#endif // EXDEV

#ifdef EXFULL
    case EXFULL:
        return FIMO_EXFULL;
#endif // EXFULL

    default:
        return FIMO_EUNKNOWN;
    }
}
