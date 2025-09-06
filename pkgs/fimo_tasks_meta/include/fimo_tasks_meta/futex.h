/// Portable user-space implementation of the linux futex API.

#ifndef FIMO_TASKS_META_FUTEX_H
#define FIMO_TASKS_META_FUTEX_H

#include <fimo_std.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Maximum number of keys allowed for the `waitv` operation.
#define FIMO_TASKS_META_FUTEX_MAX_WAITV_KEY_COUNT 128

/// Possible status codes of the futex symbols.
typedef enum FimoTasksMeta_FutexStatus : FSTD_I32 {
    /// Operation completed successfully.
    FIMO_TASKS_META_FUTEX_STATUS_OK = 0,
    /// Futex value does not match the expected value.
    FIMO_TASKS_META_FUTEX_STATUS_INVALID = 1,
    /// Operation timed out.
    FIMO_TASKS_META_FUTEX_STATUS_TIMEOUT = 2,
    /// Unexpected number of keys.
    FIMO_TASKS_META_FUTEX_STATUS_KEY_ERROR = 3,
} FimoTasksMeta_FutexStatus;

/// Information required for a wait operation.
typedef struct FimoTasksMeta_FutexKeyExpect {
    const void *key;
    FSTD_USize key_size;
    FSTD_U64 expect;
    FSTD_USize token;
} FimoTasksMeta_FutexKeyExpect;

/// Filter for a filter operation.
///
/// Encodes the following operation:
///
/// ```
/// token &= token_mask;
/// TokenType token_value = token_op(token);
/// TokenType cmp_value = cmp_arg_op(cmp_arg);
/// bool cmp = cmp_op(token_value, cmp_value);
/// return cmp;
/// ```
typedef struct FimoTasksMeta_FutexFilter {
    FSTD_USize op;
    FSTD_USize token_mask;
    FSTD_USize cmp_arg;
} FimoTasksMeta_FutexFilter;

typedef enum FimoTasksMeta_FutexFilterTokenOp : FSTD_USize {
    /// TokenType token_value = token
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_OP_NOOP = 0,
    /// TokenType token_value = *(const TokenType*)token
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_OP_DEREF = 1,
} FimoTasksMeta_FutexFilterTokenOp;

typedef enum FimoTasksMeta_FutexFilterTokenType : FSTD_USize {
    /// typedef FSTD_U8 TokenType;
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U8 = 0,
    /// typedef FSTD_U16 TokenType;
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U16 = 1,
    /// typedef FSTD_U32 TokenType;
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U32 = 2,
    /// typedef FSTD_U64 TokenType;
    FIMO_TASKS_META_FUTEX_FILTER_TOKEN_TYPE_U64 = 3,
} FimoTasksMeta_FutexFilterTokenType;

typedef enum FimoTasksMeta_FutexFilterCmpOp : FSTD_USize {
    /// bool cmp = token_value == cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_EQ = 0,
    /// bool cmp = token_value != cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_NE = 1,
    /// bool cmp = token_value < cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_LT = 2,
    /// bool cmp = token_value <= cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_LE = 3,
    /// bool cmp = token_value > cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_GT = 4,
    /// bool cmp = token_value >= cmp_value
    FIMO_TASKS_META_FUTEX_FILTER_CMP_OP_GE = 5,
} FimoTasksMeta_FutexFilterCmpOp;

typedef enum FimoTasksMeta_FutexFilterCmpArgOp : FSTD_USize {
    /// TokenType cmp_value = cmp_arg
    FIMO_TASKS_META_FUTEX_FILTER_CMP_ARG_OP_NOOP = 0,
    /// TokenType cmp_value = *(const TokenType*)cmp_arg
    FIMO_TASKS_META_FUTEX_FILTER_CMP_ARG_OP_DEREF = 1,
} FimoTasksMeta_FutexFilterCmpArgOp;

/// Initializes a new operation of a filter.
static FSTD_USize FimoTasksMeta_futex_filter_op_init(FimoTasksMeta_FutexFilterTokenOp token_op,
                                                     FimoTasksMeta_FutexFilterTokenType token_type,
                                                     FimoTasksMeta_FutexFilterCmpOp cmp_op,
                                                     FimoTasksMeta_FutexFilterCmpArgOp cmp_arg_op) {
    FSTD_USize op = 0;
    op |= token_op & 0b1;
    op |= (token_type & 0b11) << 1;
    op |= (cmp_op & 0b111) << 3;
    op |= (cmp_arg_op & 0b1) << 6;
    return op;
}


/// Initializes a filter.
static FimoTasksMeta_FutexFilter FimoTasksMeta_futex_filter_init(FSTD_USize op, FSTD_USize token_mask,
                                                                 FSTD_USize cmp_arg) {
    return (FimoTasksMeta_FutexFilter){
            .op = op,
            .token_mask = token_mask,
            .cmp_arg = cmp_arg,
    };
}

/// Constructs a filter that accept all tokens.
///
/// Builds the operation: `return (FSTD_U8)(token & 0) == 0`
#define FIMO_TASKS_META_FUTEX_FILTER_ALL FimoTasksMeta_futex_filter_init(0, 0, 0)

/// Result of the requeue operation.
typedef struct FimoTasksMeta_FutexRequeueResult {
    FSTD_USize wake_count;
    FSTD_USize requeue_count;
} FimoTasksMeta_FutexRequeueResult;

/// Puts the caller to sleep if the value pointed to by `key` equals `expect`.
///
/// If the value does not match, the function returns imediately with
/// `FIMO_TASKS_META_FUTEX_STATUS_INVALID`. The `key_size` parameter specifies the size of the
/// value in bytes and must be either of `1`, `2`, `4` or `8`, in which case `key` is treated as a
/// pointer to `u8`, `u16`, `u32`, or `u64` respectively, and `expect` is truncated. The `token` is
/// a user definable integer to store additional metadata about the waiter, which can be utilized
/// to controll some wake operations.
///
/// If `timeout` is set, and it is reached before a wake operation wakes the task, the task will be
/// resumed, and the function returns `FIMO_TASKS_META_FUTEX_STATUS_TIMEOUT`.
typedef FimoTasksMeta_FutexStatus (*FimoTasksMeta_futex_wait)(const void *key, FSTD_USize key_size, FSTD_U64 expect,
                                                              FSTD_USize token, const FSTD_Instant *timeout);

/// Puts the caller to sleep if all keys match their expected values.
///
/// Is a generalization of `wait` for multiple keys. At least `1` key, and at most
/// `max_waitv_key_count` may be passed to this function. Otherwise it returns
/// `FIMO_TASKS_META_FUTEX_STATUS_KEY_ERROR`. On wakeup, the index of the woken up key is stored
/// into `wake_index`.
typedef FimoTasksMeta_FutexStatus (*FimoTasksMeta_futex_waitv)(const FimoTasksMeta_FutexKeyExpect *keys,
                                                               FSTD_USize key_count, const FSTD_Instant *timeout,
                                                               FSTD_USize *wake_index);

/// Wakes at most `max_waiters` waiting on `key`.
///
/// Uses the token provided by the waiter and the `filter` to determine whether to ignore it from
/// being woken up. Returns the number of woken waiters.
typedef FSTD_USize (*FimoTasksMeta_futex_wake)(const void *key, FSTD_USize max_waiters,
                                               FimoTasksMeta_FutexFilter filter);

/// Requeues waiters from `key_from` to `key_to`.
///
/// Checks if the value behind `key_from` equals `expect`, in which case up to a maximum of
/// `max_wakes` waiters are woken up from `key_from` and a maximum of `max_requeues` waiters
/// are requeued from the `key_from` queue to the `key_to` queue. If the value does not match
/// the function returns `FIMO_TASKS_META_FUTEX_STATUS_INVALID`. Uses the token provided by the
/// waiter and the `filter` to determine whether to ignore it from being woken up.
typedef FimoTasksMeta_FutexStatus (*FimoTasksMeta_futex_requeue)(const void *key_from, const void *key_to,
                                                                 FSTD_USize key_size, FSTD_U64 expect,
                                                                 FSTD_USize max_wakes, FSTD_USize max_requeues,
                                                                 FimoTasksMeta_FutexFilter filter,
                                                                 FimoTasksMeta_FutexRequeueResult *result);

#ifdef __cplusplus
}
#endif

#endif // FIMO_TASKS_META_FUTEX_H
