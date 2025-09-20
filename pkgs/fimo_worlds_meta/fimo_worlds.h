/// fimo-worlds

#ifndef FIMO_WORLDS_HEADER
#define FIMO_WORLDS_HEADER

#include <fimo_std.h>
#include <fimo_tasks.h>

#ifdef __cplusplus
extern "C" {
#endif

/// A collection of resources and systems.
typedef struct FWRLD__World *FWRLD_World;

/// Descriptor for a world.
typedef struct {
    /// Optional label of the world.
    FSTD_StrConst label;
} FWRLD_WorldDesc;

/// A handle to a resource in a world.
///
/// The handle uniquely identifies the resource in the world.
typedef struct FWRLD__Res *FWRLD_Res;

/// Descriptor for a resource.
typedef struct {
    /// Optional label of the resource.
    FSTD_StrConst label;
    /// Pointer to the resource.
    void *value;
} FWRLD_ResDesc;

/// Handle to a system scheduler within a world.
typedef struct FWRLD__Scheduler *FWRLD_Scheduler;

/// Descriptor for a system scheduler.
typedef struct {
    /// Optional label of the scheduler.
    FSTD_StrConst label;
    /// Optional executor for the scheduler.
    ///
    /// If no scheduler is provided, the scheduler will be created in
    /// single threaded mode. And all systems will be run in the thread
    /// that starts the schedule operation.
    FTSK_Executor *FSTD_MAYBE_NULL executor;
} FWRLD_SchedulerDesc;

/// A handle to a registered system(-set).
///
/// The handle uniquely identifies the system(-set) in the scheduler.
typedef struct FWRLD__Sys *FWRLD_Sys;

/// Arguments passed to a system function.
///
/// Should not be copied, as it may be extended in the future.
typedef struct {
    /// World owning the system.
    FWRLD_World world;
    /// Scheduler owning the system.
    FWRLD_Scheduler sched;
    /// Arena whose lifetime matches the execution of all systems
    /// in the current scheduler.
    FSTD_Arena *arena;
    /// Slice of requested resources with read permission.
    FSTD_SliceConst(void *) read;
    /// Slice of requested resources with write permission.
    FSTD_SliceConst(void *) write;
} FWRLD_SysArgs;

/// Function that will be called when a scheduler runs a system.
///
/// The system may spawn additional tasks which outlive the system.
/// In that case, the system may return a fence to indicate when the sub-tasks are done.
/// The scheduler will then wait upon the completion of all sub-tasks before completing it's run.
typedef FTSK_Fence *FSTD_MAYBE_NULL (*FWRLD_SysRun)(void *FSTD_MAYBE_NULL data, const FWRLD_SysArgs *args);

/// A condition function that is executed to determine whether the execute a system(-set).
///
/// If the function returns `false`, the scheduler will skip the execution of the system(-set).
typedef bool (*FWRLD_SysCond)(void *data, const FWRLD_SysArgs *args);

typedef FSTD_I32 FWRLD_SysDescTag;
enum {
    /// The description contains a single system.
    FWRLD_SysDescTag_Sys = (FWRLD_SysDescTag)0,
    /// The description is recursively defined by multiple system (sets).
    FWRLD_SysDescTag_Set = (FWRLD_SysDescTag)1,
    FWRLD__SysDescTag_ = FSTD_I32_MAX,
};

typedef struct {
    /// List of required resrources with read access.
    FSTD_SliceConst(FWRLD_Res) read;
    /// List of required resrources with write access.
    FSTD_SliceConst(FWRLD_Res) write;
    /// Data to pass to the condition function.
    void *FSTD_MAYBE_NULL data;
    /// Condition function to invoke.
    FWRLD_SysCond condition;
} FWRLD_SysCondDesc;

/// Description of a system(-set).
typedef struct FWRLD_SysDesc {
    /// Active tag in the union.
    FWRLD_SysDescTag tag;
    /// Systems to run before the systems of the current set.
    ///
    /// The current set will see the effects of all listed systems.
    FSTD_SliceConst(FWRLD_Sys) before;
    /// Systems to run after the current set.
    ///
    /// These systems will see the effects of the current set.
    FSTD_SliceConst(FWRLD_Sys) after;
    /// Conditions to evaluate whether to run or skip the current set.
    ///
    /// All conditions must return `true` to execute the set.
    /// Each condition will only be evaluated once.
    FSTD_SliceConst(FWRLD_SysCondDesc) conditions;
    union {
        struct {
            /// Optional label of the system.
            FSTD_StrConst label;
            /// List of required resrources with read access.
            FSTD_SliceConst(FWRLD_Res) read;
            /// List of required resrources with write access.
            FSTD_SliceConst(FWRLD_Res) write;
            /// Data to pass to the system function.
            void *FSTD_MAYBE_NULL data;
            /// System function to invoke.
            FWRLD_SysRun system;
        } sys;
        struct {
            /// Whether to serialize the execution of the systems in the set.
            bool serialize;
            /// List of sub-systems in the set.
            FSTD_SliceConst(struct FWRLD_SysDesc *const) sub_desc;
        } set;
    };
} FWRLD_SysDesc;

/// Initializes a new empty world.
FSTD_CHECK_USE fstd_func FSTD_Status fwrld_world_init(FWRLD_World *world, const FWRLD_WorldDesc *desc);

/// Deinitializes an empty world.
fstd_func void fwrld_world_deinit(FWRLD_World world);

/// Adds a new resource to the world.
FSTD_CHECK_USE fstd_func FWRLD_Res fwrld_world_add_res(FWRLD_World world, const FWRLD_ResDesc *desc);

/// Invalidates the resource.
///
/// The resource may not be in use.
fstd_func void fwrld_resource_deinit(FWRLD_Res res);

/// Acquires the resource with read access.
///
/// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
/// strategy to acquire all necessary resources before executing any system.
fstd_func void *fwrld_resource_lock_read(FWRLD_Res res);

/// Unlocks a resource acquired with read access.
fstd_func void fwrld_resource_unlock_read(FWRLD_Res res);

/// Acquires the resource with write access.
///
/// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
/// strategy to acquire all necessary resources before executing any system.
fstd_func void *fwrld_resource_lock_write(FWRLD_Res res);

/// Unlocks a resource acquired with write access.
fstd_func void fwrld_resource_unlock_write(FWRLD_Res res);

/// Adds an empty scheduler to the world.
FSTD_CHECK_USE fstd_func FWRLD_Scheduler fwrld_world_add_scheduler(FWRLD_World world, const FWRLD_SchedulerDesc *desc);

/// Invalidates the scheduler.
///
/// The scheduler may not be running.
fstd_func void fwrld_scheduler_deinit(FWRLD_Scheduler sched);

/// Adds a system(-set) to the scheduler.
FSTD_CHECK_USE fstd_func FSTD_Status fwrld_scheduler_add_sys(FWRLD_Scheduler sched, const FWRLD_SysDesc *desc,
                                                             FWRLD_Sys *sys);

/// Removes the system from the scheduler.
///
/// The handle is invalidated after calling this function.
/// The operation signals the fence on completion.
/// If no fence is provided, this function blocks until completion.
fstd_func void fwrld_sys_deinit(FWRLD_Sys sys, FTSK_Fence *FSTD_MAYBE_NULL fence);

/// Starts a new run of the systems, blocking the current thread until it completes.
fstd_func void fwrld_scheduler_run(FWRLD_Scheduler sched, FSTD_Arena *arena);

/// Schedules the systems of the scheduler to be run asynchronously.
///
/// Multiple concurrent schedule operations are serialized. The arena must remain valid until `completion` is signaled.
/// The systems will start running after `start` is signaled. If no executor is associated with the scheduler, this
/// operation will block the calling thread until all systems are run.
fstd_func void fwrld_scheduler_schedule(FWRLD_Scheduler sched, FSTD_Arena *arena, FTSK_Fence *FSTD_MAYBE_NULL start,
                                        FTSK_Fence *FSTD_MAYBE_NULL completion);

/// Blocks the calling thread until all scheduled operations are completed.
fstd_func void fwrld_scheduler_flush(FWRLD_Scheduler sched);

#define FWRLD_SYM_NS "fimo-worlds"
#define FWRLD__SYM_ID(name) FSTD_MODULE_SYMBOL_NS(name, FWRLD_SYM_NS, FSTD_CTX_VERSION)
#define FWRLD_SYM_ALL                                                                                                  \
    FWRLD_Sym_WorldInit, FWRLD_Sym_WorldDeinit, FWRLD_Sym_WorldAddRes, FWRLD_Sym_WorldAddScheduler,                    \
            FWRLD_Sym_ResourceDeinit, FWRLD_Sym_ResourceLockRead, FWRLD_Sym_ResourceUnlockRead,                        \
            FWRLD_Sym_ResourceLockWrite, FWRLD_Sym_ResourceUnlockWrite, FWRLD_Sym_SchedulerDeinit,                     \
            FWRLD_Sym_SchedulerAddSys, FWRLD_Sym_SchedulerRun, FWRLD_Sym_SchedulerSchedule, FWRLD_Sym_SchedulerFlush,  \
            FWRLD_Sym_SysDeinit

FSTD_SYM_FN(FWRLD_Sym_WorldInit, FWRLD__SYM_ID("world_init"), FSTD_Status, FWRLD_World *world,
            const FWRLD_WorldDesc *desc)
FSTD_SYM_FN(FWRLD_Sym_WorldDeinit, FWRLD__SYM_ID("world_deinit"), void, FWRLD_World world)
FSTD_SYM_FN(FWRLD_Sym_WorldAddRes, FWRLD__SYM_ID("world_add_res"), FWRLD_Res, FWRLD_World world,
            const FWRLD_ResDesc *desc)
FSTD_SYM_FN(FWRLD_Sym_WorldAddScheduler, FWRLD__SYM_ID("world_add_scheduler"), FWRLD_Scheduler, FWRLD_World world,
            const FWRLD_SchedulerDesc *desc)
FSTD_SYM_FN(FWRLD_Sym_ResourceDeinit, FWRLD__SYM_ID("resource_deinit"), void, FWRLD_Res res)
FSTD_SYM_FN(FWRLD_Sym_ResourceLockRead, FWRLD__SYM_ID("resource_lock_read"), void *, FWRLD_Res res)
FSTD_SYM_FN(FWRLD_Sym_ResourceUnlockRead, FWRLD__SYM_ID("resource_unlock_read"), void, FWRLD_Res res)
FSTD_SYM_FN(FWRLD_Sym_ResourceLockWrite, FWRLD__SYM_ID("resource_lock_write"), void *, FWRLD_Res res)
FSTD_SYM_FN(FWRLD_Sym_ResourceUnlockWrite, FWRLD__SYM_ID("resource_unlock_write"), void, FWRLD_Res res)
FSTD_SYM_FN(FWRLD_Sym_SchedulerDeinit, FWRLD__SYM_ID("scheduler_deinit"), void, FWRLD_Scheduler sched)
FSTD_SYM_FN(FWRLD_Sym_SchedulerAddSys, FWRLD__SYM_ID("scheduler_add_sys"), FSTD_Status, FWRLD_Scheduler sched,
            const FWRLD_SysDesc *desc, FWRLD_Sys *sys)
FSTD_SYM_FN(FWRLD_Sym_SchedulerRun, FWRLD__SYM_ID("scheduler_run"), void, FWRLD_Scheduler sched, FSTD_Arena *arena)
FSTD_SYM_FN(FWRLD_Sym_SchedulerSchedule, FWRLD__SYM_ID("scheduler_schedule"), void, FWRLD_Scheduler sched,
            FSTD_Arena *arena, FTSK_Fence *FSTD_MAYBE_NULL start, FTSK_Fence *FSTD_MAYBE_NULL completion)
FSTD_SYM_FN(FWRLD_Sym_SchedulerFlush, FWRLD__SYM_ID("scheduler_flush"), void, FWRLD_Scheduler sched)
FSTD_SYM_FN(FWRLD_Sym_SysDeinit, FWRLD__SYM_ID("sys_deinit"), void, FWRLD_Sys sys, FTSK_Fence *FSTD_MAYBE_NULL fence)

#ifdef __cplusplus
}
#endif

#ifdef __cplusplus

namespace fworlds {
    /// A handle to a resource in a world.
    ///
    /// The handle uniquely identifies the resource in the world.
    template<typename T>
    struct Res;

    /// Descriptor for a resource.
    template<typename T>
    struct ResDesc {
        /// Optional label of the resource.
        FSTD_StrConst label;
        /// Pointer to the resource.
        T *value;
    };

    /// Adds a new resource to the world.
    template<typename T>
    [[nodiscard]]
    fstd_util auto world_add_res(FWRLD_World world, const ResDesc<T> &desc) noexcept -> Res<T> * {
        FWRLD_Res res = fwrld_world_add_res(world, reinterpret_cast<const FWRLD_ResDesc *>(&desc));
        return reinterpret_cast<Res<T> *>(res);
    }

    /// Invalidates the resource.
    ///
    /// The resource may not be in use.
    template<typename T>
    fstd_util auto resource_deinit(Res<T> *res) noexcept -> void {
        return fwrld_resource_deinit(reinterpret_cast<FWRLD_Res>(res));
    }

    /// Acquires the resource with read access.
    ///
    /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
    /// strategy to acquire all necessary resources before executing any system.
    template<typename T>
    [[nodiscard("the resource must be unlocked")]]
    fstd_util auto resource_lock_read(Res<T> *res) noexcept -> T * {
        return reinterpret_cast<T *>(fwrld_resource_lock_read(reinterpret_cast<FWRLD_Res>(res)));
    }

    /// Unlocks a resource acquired with read access.
    template<typename T>
    fstd_util auto resource_unlock_read(Res<T> *res) noexcept -> void {
        return fwrld_resource_unlock_read(reinterpret_cast<FWRLD_Res>(res));
    }

    /// Acquires the resource with write access.
    ///
    /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
    /// strategy to acquire all necessary resources before executing any system.
    template<typename T>
    [[nodiscard("the resource must be unlocked")]]
    fstd_util auto resource_lock_write(Res<T> *res) noexcept -> T * {
        return reinterpret_cast<T *>(fwrld_resource_lock_write(reinterpret_cast<FWRLD_Res>(res)));
    }

    /// Unlocks a resource acquired with write access.
    template<typename T>
    fstd_util auto resource_unlock_write(Res<T> *res) noexcept -> void {
        return fwrld_resource_unlock_write(reinterpret_cast<FWRLD_Res>(res));
    }
} // namespace fworlds

#endif

#ifdef FIMO_WORLDS_IMPLEMENTATION

#ifdef __cplusplus
extern "C" {
#endif

FSTD_SYM_FN_IMP(FWRLD_Sym_WorldInit, FSTD_Status, FWRLD_World *world, const FWRLD_WorldDesc *desc)
FSTD_SYM_FN_IMP(FWRLD_Sym_WorldDeinit, void, FWRLD_World world)
FSTD_SYM_FN_IMP(FWRLD_Sym_WorldAddRes, FWRLD_Res, FWRLD_World world, const FWRLD_ResDesc *desc)
FSTD_SYM_FN_IMP(FWRLD_Sym_WorldAddScheduler, FWRLD_Scheduler, FWRLD_World world, const FWRLD_SchedulerDesc *desc)
FSTD_SYM_FN_IMP(FWRLD_Sym_ResourceDeinit, void, FWRLD_Res res)
FSTD_SYM_FN_IMP(FWRLD_Sym_ResourceLockRead, void *, FWRLD_Res res)
FSTD_SYM_FN_IMP(FWRLD_Sym_ResourceUnlockRead, void, FWRLD_Res res)
FSTD_SYM_FN_IMP(FWRLD_Sym_ResourceLockWrite, void *, FWRLD_Res res)
FSTD_SYM_FN_IMP(FWRLD_Sym_ResourceUnlockWrite, void, FWRLD_Res res)
FSTD_SYM_FN_IMP(FWRLD_Sym_SchedulerDeinit, void, FWRLD_Scheduler sched)
FSTD_SYM_FN_IMP(FWRLD_Sym_SchedulerAddSys, FSTD_Status, FWRLD_Scheduler sched, const FWRLD_SysDesc *desc,
                FWRLD_Sys *sys)
FSTD_SYM_FN_IMP(FWRLD_Sym_SchedulerRun, void, FWRLD_Scheduler sched, FSTD_Arena *arena)
FSTD_SYM_FN_IMP(FWRLD_Sym_SchedulerSchedule, void, FWRLD_Scheduler sched, FSTD_Arena *arena,
                FTSK_Fence *FSTD_MAYBE_NULL start, FTSK_Fence *FSTD_MAYBE_NULL completion)
FSTD_SYM_FN_IMP(FWRLD_Sym_SchedulerFlush, void, FWRLD_Scheduler sched)
FSTD_SYM_FN_IMP(FWRLD_Sym_SysDeinit, void, FWRLD_Sys sys, FTSK_Fence *FSTD_MAYBE_NULL fence)

FSTD_CHECK_USE fstd_func FSTD_Status fwrld_world_init(FWRLD_World *world, const FWRLD_WorldDesc *desc) {
    return FWRLD_Sym_WorldInit__get()(world, desc);
}

fstd_func_impl void fwrld_world_deinit(FWRLD_World world) { return FWRLD_Sym_WorldDeinit__get()(world); }

FSTD_CHECK_USE fstd_func_impl FWRLD_Res fwrld_world_add_res(FWRLD_World world, const FWRLD_ResDesc *desc) {
    return FWRLD_Sym_WorldAddRes__get()(world, desc);
}

fstd_func_impl void fwrld_resource_deinit(FWRLD_Res res) { return FWRLD_Sym_ResourceDeinit__get()(res); }

fstd_func_impl void *fwrld_resource_lock_read(FWRLD_Res res) { return FWRLD_Sym_ResourceLockRead__get()(res); }

fstd_func_impl void fwrld_resource_unlock_read(FWRLD_Res res) { return FWRLD_Sym_ResourceUnlockRead__get()(res); }

fstd_func_impl void *fwrld_resource_lock_write(FWRLD_Res res) { return FWRLD_Sym_ResourceLockWrite__get()(res); }

fstd_func_impl void fwrld_resource_unlock_write(FWRLD_Res res) { return FWRLD_Sym_ResourceUnlockWrite__get()(res); }

FSTD_CHECK_USE fstd_func_impl FWRLD_Scheduler fwrld_world_add_scheduler(FWRLD_World world,
                                                                        const FWRLD_SchedulerDesc *desc) {
    return FWRLD_Sym_WorldAddScheduler__get()(world, desc);
}

fstd_func_impl void fwrld_scheduler_deinit(FWRLD_Scheduler sched) { return FWRLD_Sym_SchedulerDeinit__get()(sched); }

FSTD_CHECK_USE fstd_func_impl FSTD_Status fwrld_scheduler_add_sys(FWRLD_Scheduler sched, const FWRLD_SysDesc *desc,
                                                                  FWRLD_Sys *sys) {
    return FWRLD_Sym_SchedulerAddSys__get()(sched, desc, sys);
}

fstd_func_impl void fwrld_sys_deinit(FWRLD_Sys sys, FTSK_Fence *FSTD_MAYBE_NULL fence) {
    return FWRLD_Sym_SysDeinit__get()(sys, fence);
}

fstd_func_impl void fwrld_scheduler_run(FWRLD_Scheduler sched, FSTD_Arena *arena) {
    return FWRLD_Sym_SchedulerRun__get()(sched, arena);
}

fstd_func_impl void fwrld_scheduler_schedule(FWRLD_Scheduler sched, FSTD_Arena *arena,
                                             FTSK_Fence *FSTD_MAYBE_NULL start,
                                             FTSK_Fence *FSTD_MAYBE_NULL completion) {
    return FWRLD_Sym_SchedulerSchedule__get()(sched, arena, start, completion);
}

fstd_func_impl void fwrld_scheduler_flush(FWRLD_Scheduler sched) { return FWRLD_Sym_SchedulerFlush__get()(sched); }

#ifdef __cplusplus
}
#endif

#endif // FIMO_WORLDS_IMPLEMENTATION

#endif // FIMO_WORLDS_HEADER
