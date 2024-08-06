#include <fimo_std/error.h>

#include <errno.h>
#include <stddef.h>
#include <string.h>

#include "fimo_std/memory.h"

FIMO_EXPORT
FIMO_MUST_USE
const char *fimo_error_code_name(const FimoErrorCode errnum) {
    switch (errnum) {
        case FIMO_ERROR_CODE_OK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_OK);
        case FIMO_ERROR_CODE_2BIG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_2BIG);
        case FIMO_ERROR_CODE_ACCES:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ACCES);
        case FIMO_ERROR_CODE_ADDRINUSE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ADDRINUSE);
        case FIMO_ERROR_CODE_ADDRNOTAVAIL:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ADDRNOTAVAIL);
        case FIMO_ERROR_CODE_AFNOSUPPORT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_AFNOSUPPORT);
        case FIMO_ERROR_CODE_AGAIN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_AGAIN);
        case FIMO_ERROR_CODE_ALREADY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ALREADY);
        case FIMO_ERROR_CODE_BADE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADE);
        case FIMO_ERROR_CODE_BADF:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADF);
        case FIMO_ERROR_CODE_BADFD:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADFD);
        case FIMO_ERROR_CODE_BADMSG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADMSG);
        case FIMO_ERROR_CODE_BADR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADR);
        case FIMO_ERROR_CODE_BADRQC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADRQC);
        case FIMO_ERROR_CODE_BADSLT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BADSLT);
        case FIMO_ERROR_CODE_BUSY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_BUSY);
        case FIMO_ERROR_CODE_CANCELED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CANCELED);
        case FIMO_ERROR_CODE_CHILD:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CHILD);
        case FIMO_ERROR_CODE_CHRNG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CHRNG);
        case FIMO_ERROR_CODE_COMM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_COMM);
        case FIMO_ERROR_CODE_CONNABORTED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CONNABORTED);
        case FIMO_ERROR_CODE_CONNREFUSED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CONNREFUSED);
        case FIMO_ERROR_CODE_CONNRESET:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_CONNRESET);
        case FIMO_ERROR_CODE_DEADLK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_DEADLK);
        case FIMO_ERROR_CODE_DEADLOCK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_DEADLOCK);
        case FIMO_ERROR_CODE_DESTADDRREQ:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_DESTADDRREQ);
        case FIMO_ERROR_CODE_DOM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_DOM);
        case FIMO_ERROR_CODE_DQUOT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_DQUOT);
        case FIMO_ERROR_CODE_EXIST:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_EXIST);
        case FIMO_ERROR_CODE_FAULT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_FAULT);
        case FIMO_ERROR_CODE_FBIG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_FBIG);
        case FIMO_ERROR_CODE_HOSTDOWN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_HOSTDOWN);
        case FIMO_ERROR_CODE_HOSTUNREACH:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_HOSTUNREACH);
        case FIMO_ERROR_CODE_HWPOISON:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_HWPOISON);
        case FIMO_ERROR_CODE_IDRM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_IDRM);
        case FIMO_ERROR_CODE_ILSEQ:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ILSEQ);
        case FIMO_ERROR_CODE_INPROGRESS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_INPROGRESS);
        case FIMO_ERROR_CODE_INTR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_INTR);
        case FIMO_ERROR_CODE_INVAL:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_INVAL);
        case FIMO_ERROR_CODE_IO:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_IO);
        case FIMO_ERROR_CODE_ISCONN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ISCONN);
        case FIMO_ERROR_CODE_ISDIR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ISDIR);
        case FIMO_ERROR_CODE_ISNAM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ISNAM);
        case FIMO_ERROR_CODE_KEYEXPIRED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_KEYEXPIRED);
        case FIMO_ERROR_CODE_KEYREJECTED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_KEYREJECTED);
        case FIMO_ERROR_CODE_KEYREVOKED:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_KEYREVOKED);
        case FIMO_ERROR_CODE_L2HLT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_L2HLT);
        case FIMO_ERROR_CODE_L2NSYNC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_L2NSYNC);
        case FIMO_ERROR_CODE_L3HLT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_L3HLT);
        case FIMO_ERROR_CODE_L3RST:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_L3RST);
        case FIMO_ERROR_CODE_LIBACC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LIBACC);
        case FIMO_ERROR_CODE_LIBBAD:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LIBBAD);
        case FIMO_ERROR_CODE_LIBMAX:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LIBMAX);
        case FIMO_ERROR_CODE_LIBSCN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LIBSCN);
        case FIMO_ERROR_CODE_LIBEXEC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LIBEXEC);
        case FIMO_ERROR_CODE_LNRNG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LNRNG);
        case FIMO_ERROR_CODE_LOOP:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_LOOP);
        case FIMO_ERROR_CODE_MEDIUMTYPE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_MEDIUMTYPE);
        case FIMO_ERROR_CODE_MFILE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_MFILE);
        case FIMO_ERROR_CODE_MLINK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_MLINK);
        case FIMO_ERROR_CODE_MSGSIZE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_MSGSIZE);
        case FIMO_ERROR_CODE_MULTIHOP:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_MULTIHOP);
        case FIMO_ERROR_CODE_NAMETOOLONG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NAMETOOLONG);
        case FIMO_ERROR_CODE_NETDOWN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NETDOWN);
        case FIMO_ERROR_CODE_NETRESET:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NETRESET);
        case FIMO_ERROR_CODE_NETUNREACH:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NETUNREACH);
        case FIMO_ERROR_CODE_NFILE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NFILE);
        case FIMO_ERROR_CODE_NOANO:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOANO);
        case FIMO_ERROR_CODE_NOBUFS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOBUFS);
        case FIMO_ERROR_CODE_NODATA:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NODATA);
        case FIMO_ERROR_CODE_NODEV:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NODEV);
        case FIMO_ERROR_CODE_NOENT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOENT);
        case FIMO_ERROR_CODE_NOEXEC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOEXEC);
        case FIMO_ERROR_CODE_NOKEY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOKEY);
        case FIMO_ERROR_CODE_NOLCK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOLCK);
        case FIMO_ERROR_CODE_NOLINK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOLINK);
        case FIMO_ERROR_CODE_NOMEDIUM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOMEDIUM);
        case FIMO_ERROR_CODE_NOMEM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOMEM);
        case FIMO_ERROR_CODE_NOMSG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOMSG);
        case FIMO_ERROR_CODE_NONET:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NONET);
        case FIMO_ERROR_CODE_NOPKG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOPKG);
        case FIMO_ERROR_CODE_NOPROTOOPT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOPROTOOPT);
        case FIMO_ERROR_CODE_NOSPC:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOSPC);
        case FIMO_ERROR_CODE_NOSR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOSR);
        case FIMO_ERROR_CODE_NOSTR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOSTR);
        case FIMO_ERROR_CODE_NOSYS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOSYS);
        case FIMO_ERROR_CODE_NOTBLK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTBLK);
        case FIMO_ERROR_CODE_NOTCONN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTCONN);
        case FIMO_ERROR_CODE_NOTDIR:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTDIR);
        case FIMO_ERROR_CODE_NOTEMPTY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTEMPTY);
        case FIMO_ERROR_CODE_NOTRECOVERABLE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTRECOVERABLE);
        case FIMO_ERROR_CODE_NOTSOCK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTSOCK);
        case FIMO_ERROR_CODE_NOTSUP:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTSUP);
        case FIMO_ERROR_CODE_NOTTY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTTY);
        case FIMO_ERROR_CODE_NOTUNIQ:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NOTUNIQ);
        case FIMO_ERROR_CODE_NXIO:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_NXIO);
        case FIMO_ERROR_CODE_OPNOTSUPP:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_OPNOTSUPP);
        case FIMO_ERROR_CODE_OVERFLOW:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_OVERFLOW);
        case FIMO_ERROR_CODE_OWNERDEAD:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_OWNERDEAD);
        case FIMO_ERROR_CODE_PERM:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PERM);
        case FIMO_ERROR_CODE_PFNOSUPPORT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PFNOSUPPORT);
        case FIMO_ERROR_CODE_PIPE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PIPE);
        case FIMO_ERROR_CODE_PROTO:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PROTO);
        case FIMO_ERROR_CODE_PROTONOSUPPORT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PROTONOSUPPORT);
        case FIMO_ERROR_CODE_PROTOTYPE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_PROTOTYPE);
        case FIMO_ERROR_CODE_RANGE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_RANGE);
        case FIMO_ERROR_CODE_REMCHG:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_REMCHG);
        case FIMO_ERROR_CODE_REMOTE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_REMOTE);
        case FIMO_ERROR_CODE_REMOTEIO:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_REMOTEIO);
        case FIMO_ERROR_CODE_RESTART:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_RESTART);
        case FIMO_ERROR_CODE_RFKILL:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_RFKILL);
        case FIMO_ERROR_CODE_ROFS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_ROFS);
        case FIMO_ERROR_CODE_SHUTDOWN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_SHUTDOWN);
        case FIMO_ERROR_CODE_SPIPE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_SPIPE);
        case FIMO_ERROR_CODE_SOCKTNOSUPPORT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_SOCKTNOSUPPORT);
        case FIMO_ERROR_CODE_SRCH:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_SRCH);
        case FIMO_ERROR_CODE_STALE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_STALE);
        case FIMO_ERROR_CODE_STRPIPE:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_STRPIPE);
        case FIMO_ERROR_CODE_TIME:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_TIME);
        case FIMO_ERROR_CODE_TIMEDOUT:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_TIMEDOUT);
        case FIMO_ERROR_CODE_TOOMANYREFS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_TOOMANYREFS);
        case FIMO_ERROR_CODE_TXTBSY:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_TXTBSY);
        case FIMO_ERROR_CODE_UCLEAN:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_UCLEAN);
        case FIMO_ERROR_CODE_UNATCH:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_UNATCH);
        case FIMO_ERROR_CODE_USERS:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_USERS);
        case FIMO_ERROR_CODE_WOULDBLOCK:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_WOULDBLOCK);
        case FIMO_ERROR_CODE_XDEV:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_XDEV);
        case FIMO_ERROR_CODE_XFULL:
            return FIMO_STRINGIFY(FIMO_ERROR_CODE_XFULL);
    }

    return "FIMO_ERROR_CODE_UNKNOWN";
}

FIMO_EXPORT
FIMO_MUST_USE
const char *fimo_error_code_description(const FimoErrorCode errnum) {
    switch (errnum) {
        case FIMO_ERROR_CODE_OK:
            return "operation completed successfully";
        case FIMO_ERROR_CODE_2BIG:
            return "argument list too long";
        case FIMO_ERROR_CODE_ACCES:
            return "permission denied";
        case FIMO_ERROR_CODE_ADDRINUSE:
            return "address already in use";
        case FIMO_ERROR_CODE_ADDRNOTAVAIL:
            return "address not available";
        case FIMO_ERROR_CODE_AFNOSUPPORT:
            return "address family not supported";
        case FIMO_ERROR_CODE_AGAIN:
            return "resource temporarily unavailable";
        case FIMO_ERROR_CODE_ALREADY:
            return "connection already in progress";
        case FIMO_ERROR_CODE_BADE:
            return "invalid exchange";
        case FIMO_ERROR_CODE_BADF:
            return "bad file descriptor";
        case FIMO_ERROR_CODE_BADFD:
            return "file descriptor in bad state";
        case FIMO_ERROR_CODE_BADMSG:
            return "bad message";
        case FIMO_ERROR_CODE_BADR:
            return "invalid request descriptor";
        case FIMO_ERROR_CODE_BADRQC:
            return "invalid request code";
        case FIMO_ERROR_CODE_BADSLT:
            return "invalid slot";
        case FIMO_ERROR_CODE_BUSY:
            return "device or resource busy";
        case FIMO_ERROR_CODE_CANCELED:
            return "operation canceled";
        case FIMO_ERROR_CODE_CHILD:
            return "no child processes";
        case FIMO_ERROR_CODE_CHRNG:
            return "channel number out of range";
        case FIMO_ERROR_CODE_COMM:
            return "communication error on send";
        case FIMO_ERROR_CODE_CONNABORTED:
            return "connection aborted";
        case FIMO_ERROR_CODE_CONNREFUSED:
            return "connection refused";
        case FIMO_ERROR_CODE_CONNRESET:
            return "connection reset";
#if defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
        case FIMO_ERROR_CODE_DEADLK:
        case FIMO_ERROR_CODE_DEADLOCK:
            return "resource deadlock avoided";
#else
        case FIMO_ERROR_CODE_DEADLK:
            return "resource deadlock avoided";
        case FIMO_ERROR_CODE_DEADLOCK:
            return "file locking deadlock error";
#endif // defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
        case FIMO_ERROR_CODE_DESTADDRREQ:
            return "destination address required";
        case FIMO_ERROR_CODE_DOM:
            return "mathematics argument out of domain of function";
        case FIMO_ERROR_CODE_DQUOT:
            return "disk quota exceeded";
        case FIMO_ERROR_CODE_EXIST:
            return "file exists";
        case FIMO_ERROR_CODE_FAULT:
            return "bad address";
        case FIMO_ERROR_CODE_FBIG:
            return "file too large";
        case FIMO_ERROR_CODE_HOSTDOWN:
            return "host is down";
        case FIMO_ERROR_CODE_HOSTUNREACH:
            return "host is unreachable";
        case FIMO_ERROR_CODE_HWPOISON:
            return "memory page has hardware error";
        case FIMO_ERROR_CODE_IDRM:
            return "identifier removed";
        case FIMO_ERROR_CODE_ILSEQ:
            return "invalid or incomplete multibyte or wide character";
        case FIMO_ERROR_CODE_INPROGRESS:
            return "operation in progress";
        case FIMO_ERROR_CODE_INTR:
            return "interrupted function call";
        case FIMO_ERROR_CODE_INVAL:
            return "invalid argument";
        case FIMO_ERROR_CODE_IO:
            return "input/output error";
        case FIMO_ERROR_CODE_ISCONN:
            return "socket is connected";
        case FIMO_ERROR_CODE_ISDIR:
            return "is a directory";
        case FIMO_ERROR_CODE_ISNAM:
            return "is a named type file";
        case FIMO_ERROR_CODE_KEYEXPIRED:
            return "key has expired";
        case FIMO_ERROR_CODE_KEYREJECTED:
            return "key was rejected by service";
        case FIMO_ERROR_CODE_KEYREVOKED:
            return "key has been revoked";
        case FIMO_ERROR_CODE_L2HLT:
            return "level 2 halted";
        case FIMO_ERROR_CODE_L2NSYNC:
            return "level 2 not synchronized";
        case FIMO_ERROR_CODE_L3HLT:
            return "level 3 halted";
        case FIMO_ERROR_CODE_L3RST:
            return "level 3 reset";
        case FIMO_ERROR_CODE_LIBACC:
            return "cannot access a needed shared library";
        case FIMO_ERROR_CODE_LIBBAD:
            return "accessing a corrupted shared library";
        case FIMO_ERROR_CODE_LIBMAX:
            return "attempting to link in too many shared libraries";
        case FIMO_ERROR_CODE_LIBSCN:
            return ".lib section in a.out corrupted";
        case FIMO_ERROR_CODE_LIBEXEC:
            return "cannot exec a shared library directly";
        case FIMO_ERROR_CODE_LNRNG:
            return "link number out of range";
        case FIMO_ERROR_CODE_LOOP:
            return "too many levels of symbolic links";
        case FIMO_ERROR_CODE_MEDIUMTYPE:
            return "wrong medium type";
        case FIMO_ERROR_CODE_MFILE:
            return "too many open files";
        case FIMO_ERROR_CODE_MLINK:
            return "too many links";
        case FIMO_ERROR_CODE_MSGSIZE:
            return "message too long";
        case FIMO_ERROR_CODE_MULTIHOP:
            return "multihop attempted";
        case FIMO_ERROR_CODE_NAMETOOLONG:
            return "filename too long";
        case FIMO_ERROR_CODE_NETDOWN:
            return "network is down";
        case FIMO_ERROR_CODE_NETRESET:
            return "connection aborted by network";
        case FIMO_ERROR_CODE_NETUNREACH:
            return "network unreachable";
        case FIMO_ERROR_CODE_NFILE:
            return "too many open files in system";
        case FIMO_ERROR_CODE_NOANO:
            return "no anode";
        case FIMO_ERROR_CODE_NOBUFS:
            return "no buffer space available";
        case FIMO_ERROR_CODE_NODATA:
            return "the named attribute does not exist, or the process has no access to this attribute";
        case FIMO_ERROR_CODE_NODEV:
            return "no such device";
        case FIMO_ERROR_CODE_NOENT:
            return "no such file or directory";
        case FIMO_ERROR_CODE_NOEXEC:
            return "exec format error";
        case FIMO_ERROR_CODE_NOKEY:
            return "required key not available";
        case FIMO_ERROR_CODE_NOLCK:
            return "no locks available";
        case FIMO_ERROR_CODE_NOLINK:
            return "link has been severed";
        case FIMO_ERROR_CODE_NOMEDIUM:
            return "no medium found";
        case FIMO_ERROR_CODE_NOMEM:
            return "not enough space/cannot allocate memory";
        case FIMO_ERROR_CODE_NOMSG:
            return "no message of the desired type";
        case FIMO_ERROR_CODE_NONET:
            return "machine is not on the network";
        case FIMO_ERROR_CODE_NOPKG:
            return "package not installed";
        case FIMO_ERROR_CODE_NOPROTOOPT:
            return "protocol not available";
        case FIMO_ERROR_CODE_NOSPC:
            return "no space left on device";
        case FIMO_ERROR_CODE_NOSR:
            return "no STREAM resources";
        case FIMO_ERROR_CODE_NOSTR:
            return "not a STREAM";
        case FIMO_ERROR_CODE_NOSYS:
            return "function not implemented";
        case FIMO_ERROR_CODE_NOTBLK:
            return "block device required";
        case FIMO_ERROR_CODE_NOTCONN:
            return "the socket is not connected";
        case FIMO_ERROR_CODE_NOTDIR:
            return "not a directory";
        case FIMO_ERROR_CODE_NOTEMPTY:
            return "directory not empty";
        case FIMO_ERROR_CODE_NOTRECOVERABLE:
            return "state not recoverable";
        case FIMO_ERROR_CODE_NOTSOCK:
            return "not a socket";
        case FIMO_ERROR_CODE_NOTSUP:
            return "operation not supported";
        case FIMO_ERROR_CODE_NOTTY:
            return "inappropriate I/O control operation";
        case FIMO_ERROR_CODE_NOTUNIQ:
            return "name not unique on network";
        case FIMO_ERROR_CODE_NXIO:
            return "no such device or address";
        case FIMO_ERROR_CODE_OPNOTSUPP:
            return "operation not supported on socket";
        case FIMO_ERROR_CODE_OVERFLOW:
            return "value too large to be stored in data type";
        case FIMO_ERROR_CODE_OWNERDEAD:
            return "owner died";
        case FIMO_ERROR_CODE_PERM:
            return "operation not permitted";
        case FIMO_ERROR_CODE_PFNOSUPPORT:
            return "protocol family not supported";
        case FIMO_ERROR_CODE_PIPE:
            return "broken pipe";
        case FIMO_ERROR_CODE_PROTO:
            return "protocol error";
        case FIMO_ERROR_CODE_PROTONOSUPPORT:
            return "protocol not supported";
        case FIMO_ERROR_CODE_PROTOTYPE:
            return "protocol wrong type for socket";
        case FIMO_ERROR_CODE_RANGE:
            return "result too large";
        case FIMO_ERROR_CODE_REMCHG:
            return "remote address changed";
        case FIMO_ERROR_CODE_REMOTE:
            return "object is remote";
        case FIMO_ERROR_CODE_REMOTEIO:
            return "remote I/O error";
        case FIMO_ERROR_CODE_RESTART:
            return "interrupted system call should be restarted";
        case FIMO_ERROR_CODE_RFKILL:
            return "operation not possible due to RF-kill";
        case FIMO_ERROR_CODE_ROFS:
            return "read-only filesystem";
        case FIMO_ERROR_CODE_SHUTDOWN:
            return "cannot send after transport endpoint shutdown";
        case FIMO_ERROR_CODE_SPIPE:
            return "invalid seek";
        case FIMO_ERROR_CODE_SOCKTNOSUPPORT:
            return "socket type not supported";
        case FIMO_ERROR_CODE_SRCH:
            return "no such process";
        case FIMO_ERROR_CODE_STALE:
            return "stale file handle";
        case FIMO_ERROR_CODE_STRPIPE:
            return "streams pipe error";
        case FIMO_ERROR_CODE_TIME:
            return "timer expired";
        case FIMO_ERROR_CODE_TIMEDOUT:
            return "connection timed out";
        case FIMO_ERROR_CODE_TOOMANYREFS:
            return "too many references: cannot splice";
        case FIMO_ERROR_CODE_TXTBSY:
            return "text file busy";
        case FIMO_ERROR_CODE_UCLEAN:
            return "structure needs cleaning";
        case FIMO_ERROR_CODE_UNATCH:
            return "protocol driver not attached";
        case FIMO_ERROR_CODE_USERS:
            return "too many users";
        case FIMO_ERROR_CODE_WOULDBLOCK:
            return "operation would block";
        case FIMO_ERROR_CODE_XDEV:
            return "invalid cross-device link";
        case FIMO_ERROR_CODE_XFULL:
            return "exchange full";
    }

    return "unknown error code";
}

FIMO_EXPORT
FIMO_MUST_USE
FimoErrorCode fimo_error_code_from_errno(const int errnum) {
    switch (errnum) {
        case 0:
            return FIMO_ERROR_CODE_OK;

#ifdef E2BIG
        case E2BIG:
            return FIMO_ERROR_CODE_2BIG;
#endif // E2BIG

#ifdef EACCES
        case EACCES:
            return FIMO_ERROR_CODE_ACCES;
#endif // EACCES

#ifdef EADDRINUSE
        case EADDRINUSE:
            return FIMO_ERROR_CODE_ADDRINUSE;
#endif // EADDRINUSE

#ifdef EADDRNOTAVAIL
        case EADDRNOTAVAIL:
            return FIMO_ERROR_CODE_ADDRNOTAVAIL;
#endif // EADDRNOTAVAIL

#ifdef EAFNOSUPPORT
        case EAFNOSUPPORT:
            return FIMO_ERROR_CODE_AFNOSUPPORT;
#endif // EAFNOSUPPORT

// EAGAIN and EWOULDBLOCK are allowed to be the same value
#if defined(EAGAIN) && defined(EWOULDBLOCK) && EAGAIN == EWOULDBLOCK
        case EAGAIN:
            return FIMO_ERROR_CODE_AGAIN;
#else
#ifdef EAGAIN
        case EAGAIN:
            return FIMO_ERROR_CODE_AGAIN;
#endif // EAGAIN

#ifdef EWOULDBLOCK
        case EWOULDBLOCK:
            return FIMO_ERROR_CODE_WOULDBLOCK;
#endif // EWOULDBLOCK
#endif // defined(EAGAIN) && defined(EWOULDBLOCK) && EAGAIN == EWOULDBLOCK

#ifdef EALREADY
        case EALREADY:
            return FIMO_ERROR_CODE_ALREADY;
#endif // EALREADY

#ifdef EBADE
        case EBADE:
            return FIMO_ERROR_CODE_BADE;
#endif // EBADE

#ifdef EBADF
        case EBADF:
            return FIMO_ERROR_CODE_BADF;
#endif // EBADF

#ifdef EBADFD
        case EBADFD:
            return FIMO_ERROR_CODE_BADFD;
#endif // EBADFD

#ifdef EBADMSG
        case EBADMSG:
            return FIMO_ERROR_CODE_BADMSG;
#endif // EBADMSG

#ifdef EBADR
        case EBADR:
            return FIMO_ERROR_CODE_BADR;
#endif // EBADR

#ifdef EBADRQC
        case EBADRQC:
            return FIMO_ERROR_CODE_BADRQC;
#endif // EBADRQC

#ifdef EBADSLT
        case EBADSLT:
            return FIMO_ERROR_CODE_BADSLT;
#endif // EBADSLT

#ifdef EBUSY
        case EBUSY:
            return FIMO_ERROR_CODE_BUSY;
#endif // EBUSY

#ifdef ECANCELED
        case ECANCELED:
            return FIMO_ERROR_CODE_CANCELED;
#endif // ECANCELED

#ifdef ECHILD
        case ECHILD:
            return FIMO_ERROR_CODE_CHILD;
#endif // ECHILD

#ifdef ECHRNG
        case ECHRNG:
            return FIMO_ERROR_CODE_CHRNG;
#endif // ECHRNG

#ifdef ECOMM
        case ECOMM:
            return FIMO_ERROR_CODE_COMM;
#endif // ECOMM

#ifdef ECONNABORTED
        case ECONNABORTED:
            return FIMO_ERROR_CODE_CONNABORTED;
#endif // ECONNABORTED

#ifdef ECONNREFUSED
        case ECONNREFUSED:
            return FIMO_ERROR_CODE_CONNREFUSED;
#endif // ECONNREFUSED

#ifdef ECONNRESET
        case ECONNRESET:
            return FIMO_ERROR_CODE_CONNRESET;
#endif // ECONNRESET

// EDEADLOCK is usually a synonym for EDEADLK
#if defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK
        case EDEADLK:
            return FIMO_ERROR_CODE_DEADLK;
#else
#ifdef EDEADLK
        case EDEADLK:
            return FIMO_ERROR_CODE_DEADLK;
#endif // EDEADLK
#ifdef EDEADLOCK
        case EDEADLOCK:
            return FIMO_ERROR_CODE_DEADLOCK;
#endif // EDEADLOCK
#endif // defined(EDEADLK) && defined(EDEADLOCK) && EDEADLK == EDEADLOCK

#ifdef EDESTADDRREQ
        case EDESTADDRREQ:
            return FIMO_ERROR_CODE_DESTADDRREQ;
#endif // EDESTADDRREQ

#ifdef EDOM
        case EDOM:
            return FIMO_ERROR_CODE_DOM;
#endif // EDOM

#ifdef EDQUOT
        case EDQUOT:
            return FIMO_ERROR_CODE_DQUOT;
#endif // EDQUOT

#ifdef EEXIST
        case EEXIST:
            return FIMO_ERROR_CODE_EXIST;
#endif // EEXIST

#ifdef EFAULT
        case EFAULT:
            return FIMO_ERROR_CODE_FAULT;
#endif // EFAULT

#ifdef EFBIG
        case EFBIG:
            return FIMO_ERROR_CODE_FBIG;
#endif // EFBIG

#ifdef EHOSTDOWN
        case EHOSTDOWN:
            return FIMO_ERROR_CODE_HOSTDOWN;
#endif // EHOSTDOWN

#ifdef EHOSTUNREACH
        case EHOSTUNREACH:
            return FIMO_ERROR_CODE_HOSTUNREACH;
#endif // EHOSTUNREACH

#ifdef EHWPOISON
        case EHWPOISON:
            return FIMO_ERROR_CODE_HWPOISON;
#endif // EHWPOISON

#ifdef EIDRM
        case EIDRM:
            return FIMO_ERROR_CODE_IDRM;
#endif // EIDRM

#ifdef EILSEQ
        case EILSEQ:
            return FIMO_ERROR_CODE_ILSEQ;
#endif // EILSEQ

#ifdef EINPROGRESS
        case EINPROGRESS:
            return FIMO_ERROR_CODE_INPROGRESS;
#endif // EINPROGRESS

#ifdef EINTR
        case EINTR:
            return FIMO_ERROR_CODE_INTR;
#endif // EINTR

#ifdef EINVAL
        case EINVAL:
            return FIMO_ERROR_CODE_INVAL;
#endif // EINVAL

#ifdef EIO
        case EIO:
            return FIMO_ERROR_CODE_IO;
#endif // EIO

#ifdef EISCONN
        case EISCONN:
            return FIMO_ERROR_CODE_ISCONN;
#endif // EISCONN

#ifdef EISDIR
        case EISDIR:
            return FIMO_ERROR_CODE_ISDIR;
#endif // EISDIR

#ifdef EISNAM
        case EISNAM:
            return FIMO_ERROR_CODE_ISNAM;
#endif // EISNAM

#ifdef EKEYEXPIRED
        case EKEYEXPIRED:
            return FIMO_ERROR_CODE_KEYEXPIRED;
#endif // EKEYEXPIRED

#ifdef EKEYREJECTED
        case EKEYREJECTED:
            return FIMO_ERROR_CODE_KEYREJECTED;
#endif // EKEYREJECTED

#ifdef EKEYREVOKED
        case EKEYREVOKED:
            return FIMO_ERROR_CODE_KEYREVOKED;
#endif // EKEYREVOKED

#ifdef EL2HLT
        case EL2HLT:
            return FIMO_ERROR_CODE_L2HLT;
#endif // EL2HLT

#ifdef EL2NSYNC
        case EL2NSYNC:
            return FIMO_ERROR_CODE_L2NSYNC;
#endif // EL2NSYNC

#ifdef EL3HLT
        case EL3HLT:
            return FIMO_ERROR_CODE_L3HLT;
#endif // EL3HLT

#ifdef EL3RST
        case EL3RST:
            return FIMO_ERROR_CODE_L3RST;
#endif // EL3RST

#ifdef ELIBACC
        case ELIBACC:
            return FIMO_ERROR_CODE_LIBACC;
#endif // ELIBACC

#ifdef ELIBBAD
        case ELIBBAD:
            return FIMO_ERROR_CODE_LIBBAD;
#endif // ELIBBAD

#ifdef ELIBMAX
        case ELIBMAX:
            return FIMO_ERROR_CODE_LIBMAX;
#endif // ELIBMAX

#ifdef ELIBSCN
        case ELIBSCN:
            return FIMO_ERROR_CODE_LIBSCN;
#endif // ELIBSCN

#ifdef ELIBEXEC
        case ELIBEXEC:
            return FIMO_ERROR_CODE_LIBEXEC;
#endif // ELIBEXEC

#ifdef ELNRNG
        case ELNRNG:
            return FIMO_ERROR_CODE_LNRNG;
#endif // ELNRNG

#ifdef ELOOP
        case ELOOP:
            return FIMO_ERROR_CODE_LOOP;
#endif // ELOOP

#ifdef EMEDIUMTYPE
        case EMEDIUMTYPE:
            return FIMO_ERROR_CODE_MEDIUMTYPE;
#endif // EMEDIUMTYPE

#ifdef EMFILE
        case EMFILE:
            return FIMO_ERROR_CODE_MFILE;
#endif // EMFILE

#ifdef EMLINK
        case EMLINK:
            return FIMO_ERROR_CODE_MLINK;
#endif // EMLINK

#ifdef EMSGSIZE
        case EMSGSIZE:
            return FIMO_ERROR_CODE_MSGSIZE;
#endif // EMSGSIZE

#ifdef EMULTIHOP
        case EMULTIHOP:
            return FIMO_ERROR_CODE_MULTIHOP;
#endif // EMULTIHOP

#ifdef ENAMETOOLONG
        case ENAMETOOLONG:
            return FIMO_ERROR_CODE_NAMETOOLONG;
#endif // ENAMETOOLONG

#ifdef ENETDOWN
        case ENETDOWN:
            return FIMO_ERROR_CODE_NETDOWN;
#endif // ENETDOWN

#ifdef ENETRESET
        case ENETRESET:
            return FIMO_ERROR_CODE_NETRESET;
#endif // ENETRESET

#ifdef ENETUNREACH
        case ENETUNREACH:
            return FIMO_ERROR_CODE_NETUNREACH;
#endif // ENETUNREACH

#ifdef ENFILE
        case ENFILE:
            return FIMO_ERROR_CODE_NFILE;
#endif // ENFILE

#ifdef ENOANO
        case ENOANO:
            return FIMO_ERROR_CODE_NOANO;
#endif // ENOANO

#ifdef ENOBUFS
        case ENOBUFS:
            return FIMO_ERROR_CODE_NOBUFS;
#endif // ENOBUFS

#ifdef ENODATA
        case ENODATA:
            return FIMO_ERROR_CODE_NODATA;
#endif // ENODATA

#ifdef ENODEV
        case ENODEV:
            return FIMO_ERROR_CODE_NODEV;
#endif // ENODEV

#ifdef ENOENT
        case ENOENT:
            return FIMO_ERROR_CODE_NOENT;
#endif // ENOENT

#ifdef ENOEXEC
        case ENOEXEC:
            return FIMO_ERROR_CODE_NOEXEC;
#endif // ENOEXEC

#ifdef ENOKEY
        case ENOKEY:
            return FIMO_ERROR_CODE_NOKEY;
#endif // ENOKEY

#ifdef ENOLCK
        case ENOLCK:
            return FIMO_ERROR_CODE_NOLCK;
#endif // ENOLCK

#ifdef ENOLINK
        case ENOLINK:
            return FIMO_ERROR_CODE_NOLINK;
#endif // ENOLINK

#ifdef ENOMEDIUM
        case ENOMEDIUM:
            return FIMO_ERROR_CODE_NOMEDIUM;
#endif // ENOMEDIUM

#ifdef ENOMEM
        case ENOMEM:
            return FIMO_ERROR_CODE_NOMEM;
#endif // ENOMEM

#ifdef ENOMSG
        case ENOMSG:
            return FIMO_ERROR_CODE_NOMSG;
#endif // ENOMSG

#ifdef ENONET
        case ENONET:
            return FIMO_ERROR_CODE_NONET;
#endif // ENONET

#ifdef ENOPKG
        case ENOPKG:
            return FIMO_ERROR_CODE_NOPKG;
#endif // ENOPKG

#ifdef ENOPROTOOPT
        case ENOPROTOOPT:
            return FIMO_ERROR_CODE_NOPROTOOPT;
#endif // ENOPROTOOPT

#ifdef ENOSPC
        case ENOSPC:
            return FIMO_ERROR_CODE_NOSPC;
#endif // ENOSPC

#ifdef ENOSR
        case ENOSR:
            return FIMO_ERROR_CODE_NOSR;
#endif // ENOSR

#ifdef ENOSTR
        case ENOSTR:
            return FIMO_ERROR_CODE_NOSTR;
#endif // ENOSTR

#ifdef ENOSYS
        case ENOSYS:
            return FIMO_ERROR_CODE_NOSYS;
#endif // ENOSYS

#ifdef ENOTBLK
        case ENOTBLK:
            return FIMO_ERROR_CODE_NOTBLK;
#endif // ENOTBLK

#ifdef ENOTCONN
        case ENOTCONN:
            return FIMO_ERROR_CODE_NOTCONN;
#endif // ENOTCONN

#ifdef ENOTDIR
        case ENOTDIR:
            return FIMO_ERROR_CODE_NOTDIR;
#endif // ENOTDIR

#ifdef ENOTEMPTY
        case ENOTEMPTY:
            return FIMO_ERROR_CODE_NOTEMPTY;
#endif // ENOTEMPTY

#ifdef ENOTRECOVERABLE
        case ENOTRECOVERABLE:
            return FIMO_ERROR_CODE_NOTRECOVERABLE;
#endif // ENOTRECOVERABLE

#ifdef ENOTSOCK
        case ENOTSOCK:
            return FIMO_ERROR_CODE_NOTSOCK;
#endif // ENOTSOCK

// EOPNOTSUPP and ENOTSUP have the same value on linux.
#if defined(ENOTSUP) && defined(EOPNOTSUPP) && ENOTSUP == EOPNOTSUPP
        case ENOTSUP:
            return FIMO_ERROR_CODE_NOTSUP;
#else
#ifdef ENOTSUP
        case ENOTSUP:
            return FIMO_ERROR_CODE_NOTSUP;
#endif // ENOTSUP

#ifdef EOPNOTSUPP
        case EOPNOTSUPP:
            return FIMO_ERROR_CODE_OPNOTSUPP;
#endif // EOPNOTSUPP
#endif // defined(ENOTSUP) && defined(EOPNOTSUPP) && ENOTSUP == EOPNOTSUPP

#ifdef ENOTTY
        case ENOTTY:
            return FIMO_ERROR_CODE_NOTTY;
#endif // ENOTTY

#ifdef ENOTUNIQ
        case ENOTUNIQ:
            return FIMO_ERROR_CODE_NOTUNIQ;
#endif // ENOTUNIQ

#ifdef ENXIO
        case ENXIO:
            return FIMO_ERROR_CODE_NXIO;
#endif // ENXIO

#ifdef EOVERFLOW
        case EOVERFLOW:
            return FIMO_ERROR_CODE_OVERFLOW;
#endif // EOVERFLOW

#ifdef EOWNERDEAD
        case EOWNERDEAD:
            return FIMO_ERROR_CODE_OWNERDEAD;
#endif // EOWNERDEAD

#ifdef EPERM
        case EPERM:
            return FIMO_ERROR_CODE_PERM;
#endif // EPERM

#ifdef EPFNOSUPPORT
        case EPFNOSUPPORT:
            return FIMO_ERROR_CODE_PFNOSUPPORT;
#endif // EPFNOSUPPORT

#ifdef EPIPE
        case EPIPE:
            return FIMO_ERROR_CODE_PIPE;
#endif // EPIPE

#ifdef EPROTO
        case EPROTO:
            return FIMO_ERROR_CODE_PROTO;
#endif // EPROTO

#ifdef EPROTONOSUPPORT
        case EPROTONOSUPPORT:
            return FIMO_ERROR_CODE_PROTONOSUPPORT;
#endif // EPROTONOSUPPORT

#ifdef EPROTOTYPE
        case EPROTOTYPE:
            return FIMO_ERROR_CODE_PROTOTYPE;
#endif // EPROTOTYPE

#ifdef ERANGE
        case ERANGE:
            return FIMO_ERROR_CODE_RANGE;
#endif // ERANGE

#ifdef EREMCHG
        case EREMCHG:
            return FIMO_ERROR_CODE_REMCHG;
#endif // EREMCHG

#ifdef EREMOTE
        case EREMOTE:
            return FIMO_ERROR_CODE_REMOTE;
#endif // EREMOTE

#ifdef EREMOTEIO
        case EREMOTEIO:
            return FIMO_ERROR_CODE_REMOTEIO;
#endif // EREMOTEIO

#ifdef ERESTART
        case ERESTART:
            return FIMO_ERROR_CODE_RESTART;
#endif // ERESTART

#ifdef ERFKILL
        case ERFKILL:
            return FIMO_ERROR_CODE_RFKILL;
#endif // ERFKILL

#ifdef EROFS
        case EROFS:
            return FIMO_ERROR_CODE_ROFS;
#endif // EROFS

#ifdef ESHUTDOWN
        case ESHUTDOWN:
            return FIMO_ERROR_CODE_SHUTDOWN;
#endif // ESHUTDOWN

#ifdef ESPIPE
        case ESPIPE:
            return FIMO_ERROR_CODE_SPIPE;
#endif // ESPIPE

#ifdef ESOCKTNOSUPPORT
        case ESOCKTNOSUPPORT:
            return FIMO_ERROR_CODE_SOCKTNOSUPPORT;
#endif // ESOCKTNOSUPPORT

#ifdef ESRCH
        case ESRCH:
            return FIMO_ERROR_CODE_SRCH;
#endif // ESRCH

#ifdef ESTALE
        case ESTALE:
            return FIMO_ERROR_CODE_STALE;
#endif // ESTALE

#ifdef ESTRPIPE
        case ESTRPIPE:
            return FIMO_ERROR_CODE_STRPIPE;
#endif // ESTRPIPE

#ifdef ETIME
        case ETIME:
            return FIMO_ERROR_CODE_TIME;
#endif // ETIME

#ifdef ETIMEDOUT
        case ETIMEDOUT:
            return FIMO_ERROR_CODE_TIMEDOUT;
#endif // ETIMEDOUT

#ifdef ETOOMANYREFS
        case ETOOMANYREFS:
            return FIMO_ERROR_CODE_TOOMANYREFS;
#endif // ETOOMANYREFS

#ifdef ETXTBSY
        case ETXTBSY:
            return FIMO_ERROR_CODE_TXTBSY;
#endif // ETXTBSY

#ifdef EUCLEAN
        case EUCLEAN:
            return FIMO_ERROR_CODE_UCLEAN;
#endif // EUCLEAN

#ifdef EUNATCH
        case EUNATCH:
            return FIMO_ERROR_CODE_UNATCH;
#endif // EUNATCH

#ifdef EUSERS
        case EUSERS:
            return FIMO_ERROR_CODE_USERS;
#endif // EUSERS

#ifdef EXDEV
        case EXDEV:
            return FIMO_ERROR_CODE_XDEV;
#endif // EXDEV

#ifdef EXFULL
        case EXFULL:
            return FIMO_ERROR_CODE_XFULL;
#endif // EXFULL

        default:
            return FIMO_ERROR_CODE_MAX + 1;
    }
}

static FimoResultString error_name_string_static_(void *data) {
    return (FimoResultString){.str = data, .release = NULL};
}

static FimoResultString error_description_string_static_(void *data) {
    return (FimoResultString){.str = data, .release = NULL};
}

const FimoResultVTable FIMO_IMPL_RESULT_STATIC_STRING_VTABLE = {
        .v0 =
                {
                        .release = NULL,
                        .error_name = error_name_string_static_,
                        .error_description = error_description_string_static_,
                },
};

static void free_string_dynamic_(void *data) { fimo_free(data); }

static FimoResultString error_name_string_dynamic_(void *data) {
    const char *str = data;
    FimoUSize str_len = strlen(str);

    FimoResult error = FIMO_EOK;
    char *cpy = fimo_calloc(str_len + 1, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return fimo_result_error_name(error);
    }
    memcpy(cpy, str, str_len);
    return (FimoResultString){.str = data, .release = NULL};
}

static FimoResultString error_description_string_dynamic_(void *data) {
    const char *str = data;
    FimoUSize str_len = strlen(str);

    FimoResult error = FIMO_EOK;
    char *cpy = fimo_calloc(str_len + 1, &error);
    if (FIMO_RESULT_IS_ERROR(error)) {
        return fimo_result_error_description(error);
    }
    memcpy(cpy, str, str_len);
    return (FimoResultString){.str = data, .release = NULL};
}

const FimoResultVTable FIMO_IMPL_RESULT_DYNAMIC_STRING_VTABLE = {
        .v0 =
                {
                        .release = free_string_dynamic_,
                        .error_name = error_name_string_dynamic_,
                        .error_description = error_description_string_dynamic_,
                },
};

static FimoResultString error_name_error_code_(void *data) {
    FimoErrorCode error = (int)(intptr_t)data;
    return (FimoResultString){.str = fimo_error_code_name(error), .release = NULL};
}

static FimoResultString error_description_error_code_(void *data) {
    FimoErrorCode error = (int)(intptr_t)data;
    return (FimoResultString){.str = fimo_error_code_description(error), .release = NULL};
}

const FimoResultVTable FIMO_IMPL_RESULT_ERROR_CODE_VTABLE = {
        .v0 =
                {
                        .release = NULL,
                        .error_name = error_name_error_code_,
                        .error_description = error_description_error_code_,
                },
};

#ifdef _WIN32
static void free_string_system_(const char *str) { LocalFree((char *)str); }
#endif

static FimoResultString error_name_system_(void *data) {
    FimoSystemErrorCode error = (FimoSystemErrorCode)(intptr_t)data;
#ifdef _WIN32
    LPSTR error_name_template = "SystemError(%1!l!)";
    DWORD_PTR error_name_args[] = {(DWORD_PTR)error};

    LPSTR error_name = NULL;
    if (!FormatMessageA(FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_FROM_STRING | FORMAT_MESSAGE_ARGUMENT_ARRAY,
                        error_name_template, 0, 0, (LPTSTR)&error_name, 0, (va_list *)error_name_args)) {
        return (FimoResultString){.str = "SystemError(unknown)", .release = NULL};
    }
    return (FimoResultString){.str = error_name, .release = free_string_system_};
#else
    FimoErrorCode code = fimo_error_code_from_errno(error);
    return (FimoResultString){.str = fimo_error_code_name(code), .release = NULL};
#endif
}

static FimoResultString error_description_system_(void *data) {
    FimoSystemErrorCode error = (FimoSystemErrorCode)(intptr_t)data;
#ifdef _WIN32
    LPSTR error_description = NULL;
    if (!FormatMessageA(FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_ALLOCATE_BUFFER | FORMAT_MESSAGE_IGNORE_INSERTS,
                        NULL, error, MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT), (LPTSTR)error_description, 0, NULL)) {
        return (FimoResultString){.str = "unknown error", .release = NULL};
    }
    return (FimoResultString){.str = error_description, .release = free_string_system_};
#else
    FimoErrorCode code = fimo_error_code_from_errno(error);
    return (FimoResultString){.str = fimo_error_code_description(code), .release = NULL};
#endif
}

const FimoResultVTable FIMO_IMPL_RESULT_SYSTEM_ERROR_CODE_VTABLE = {
        .v0 =
                {
                        .release = NULL,
                        .error_name = error_name_system_,
                        .error_description = error_description_system_,
                },
};

const FimoResult FIMO_IMPL_RESULT_OK = {.data = NULL, .vtable = NULL};
const FimoResult FIMO_IMPL_RESULT_INVALID_ERROR = {.data = "invalid error",
                                                   .vtable = &FIMO_IMPL_RESULT_STATIC_STRING_VTABLE};
const FimoResultString FIMO_IMPL_RESULT_OK_NAME = {.str = "ok", .release = NULL};
const FimoResultString FIMO_IMPL_RESULT_OK_DESCRIPTION = {.str = "ok", .release = NULL};
