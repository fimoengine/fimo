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
            FSTD_SliceConst(struct FWRLD_SysDesc) sub_desc;
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

#include <tuple>

namespace fworlds {
    /// Descriptor for a resource.
    struct WorldDesc {
        /// Optional label of the world.
        fstd::Slice<const char> label;
    };

    template<typename T>
    struct ResDesc;
    template<typename T>
    struct Res;

    struct SchedulerDesc;
    struct Scheduler;

    /// A collection of resources and systems.
    struct World {
        FWRLD_World handle;

        /// Initializes a new empty world.
        static inline auto init(const WorldDesc &desc) noexcept -> std::expected<World, fstd::Status> {
            World world{};
            FWRLD_WorldDesc d{.label = desc.label};
            auto status = fwrld_world_init(&world.handle, &d);
            if (status != FSTD_Status_Ok)
                return std::unexpected(static_cast<fstd::Status>(status));
            return world;
        }

        /// Deinitializes an empty world.
        inline auto deinit() const noexcept -> void {
            if (this->handle)
                fwrld_world_deinit(this->handle);
        }

        /// Adds a new resource to the world.
        template<typename T>
        [[nodiscard]]
        inline auto add_res(const ResDesc<T> &desc) const noexcept -> Res<T>;

        /// Adds an empty scheduler to the world.
        [[nodiscard]]
        inline auto add_scheduler(const SchedulerDesc &desc) const noexcept -> Scheduler;
    };

    /// Descriptor for a resource.
    template<typename T>
    struct ResDesc {
        /// Optional label of the resource.
        fstd::Slice<const char> label;
        /// Pointer to the resource.
        T *value;
    };

    /// A handle to a resource in a world.
    ///
    /// The handle uniquely identifies the resource in the world.
    template<typename T>
    struct Res {
        FWRLD_Res handle;

        inline constexpr operator Res<void>() noexcept { return {this->handle}; }

        /// Invalidates the resource.
        ///
        /// The resource may not be in use.
        inline auto deinit() const noexcept -> void;

        /// Acquires the resource with read access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        [[nodiscard("the resource must be unlocked")]]
        inline auto lock_read() const noexcept -> T *;

        /// Unlocks a resource acquired with read access.
        inline auto unlock_read() const noexcept -> void;

        /// Acquires the resource with write access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        [[nodiscard("the resource must be unlocked")]]
        inline auto lock_write() const noexcept -> T *;

        /// Unlocks a resource acquired with write access.
        inline auto unlock_write() const noexcept -> void;
    };

    /// Descriptor for a resource.
    struct SchedulerDesc {
        /// Optional label of the scheduler.
        fstd::Slice<const char> label;
        /// Optional executor for the scheduler.
        ///
        /// If no scheduler is provided, the scheduler will be created in
        /// single threaded mode. And all systems will be run in the thread
        /// that starts the schedule operation.
        FTSK_Executor *FSTD_MAYBE_NULL executor;
    };

    struct SysDesc;
    struct Sys;

    /// Handle to a system scheduler within a world.
    struct Scheduler {
        FWRLD_Scheduler handle;

        /// Deinitializes an empty world.
        inline auto deinit() const noexcept -> void {
            if (this->handle)
                fwrld_scheduler_deinit(this->handle);
        }

        /// Adds a system(-set) to the scheduler.
        [[nodiscard]]
        inline auto add_sys(const SysDesc &desc) const noexcept -> std::expected<Sys, fstd::Status>;

        /// Starts a new run of the systems, blocking the current thread until it completes.
        inline auto run(fstd::Arena &arena) const noexcept -> void { fwrld_scheduler_run(this->handle, &arena); }

        /// Schedules the systems of the scheduler to be run asynchronously.
        ///
        /// Multiple concurrent schedule operations are serialized. The arena must remain valid until `completion` is
        /// signaled. The systems will start running after `start` is signaled. If no executor is associated with the
        /// scheduler, this operation will block the calling thread until all systems are run.
        inline auto schedule(fstd::Arena &arena, FTSK_Fence *FSTD_MAYBE_NULL start,
                             FTSK_Fence *FSTD_MAYBE_NULL completion) const noexcept -> void {
            fwrld_scheduler_schedule(this->handle, &arena, start, completion);
        }

        /// Blocks the calling thread until all scheduled operations are completed.
        inline auto flush() const noexcept -> void { fwrld_scheduler_flush(this->handle); }
    };

    /// A handle to a registered system(-set).
    ///
    /// The handle uniquely identifies the system(-set) in the scheduler.
    struct Sys {
        FWRLD_Sys handle;

        /// Removes the system from the scheduler.
        ///
        /// The handle is invalidated after calling this function.
        /// The operation signals the fence on completion.
        /// If no fence is provided, this function blocks until completion.
        inline auto deinit(FTSK_Fence *FSTD_MAYBE_NULL fence = nullptr) const noexcept -> void {
            if (this->handle)
                fwrld_sys_deinit(this->handle, fence);
            else if (fence)
                ftsk_fence_signal(fence);
        }
    };

    /// Arguments passed to a system function.
    ///
    /// Should not be copied, as it may be extended in the future.
    template<typename Read, typename Write>
    struct SysArgs {
        World world;
        Scheduler sched;
        fstd::Arena &arena;
        Read read;
        Write write;
    };

    template<typename... Ts>
    struct ResList {
        std::array<Res<void>, sizeof...(Ts)> list;

        ResList(Res<Ts>... args) noexcept : list{args.handle...} {}
        inline operator fstd::Slice<const Res<void>>() const noexcept { return this->list; }
    };

    template<typename... Ts>
    struct ResArgs {
        fstd::Slice<const void *> list;

        template<fstd::usize Index>
        inline auto get() const noexcept -> std::tuple_element_t<Index, std::tuple<Ts...>> & {
            using T = std::tuple_element_t<Index, std::tuple<Ts...>>;
            return *static_cast<T *>((const_cast<void *>(this->list[Index])));
        }
    };

    struct CondDesc {
        fstd::Slice<const Res<void>> read;
        fstd::Slice<const Res<void>> write;
        void *FSTD_MAYBE_NULL data;
        FWRLD_SysCond condition;


        template<typename... Read, typename... Write, typename F>
        static inline constexpr auto init(fstd::Slice<const char> label, const ResList<Read...> &read,
                                          const ResList<Write...> &write, F) noexcept -> CondDesc {
            return {
                    .label = label,
                    .read = read,
                    .write = write,
                    .data = nullptr,
                    .system = +[](void *, const FWRLD_SysArgs *args) -> bool {
                        const SysArgs<ResArgs<Read...>, ResArgs<Write...>> args2 = {
                                .world = {args->world},
                                .sched = {args->sched},
                                .arena = *static_cast<fstd::Arena *>(args->arena),
                                .read = {.list = {args->read.ptr, args->read.len}},
                                .write = {.list = {args->write.ptr, args->write.len}},
                        };
                        return std::invoke_r<bool>(F{}, args2);
                    },
            };
        }
    };

    struct SysDescSys {
        fstd::Slice<const char> label;
        fstd::Slice<const Res<void>> read;
        fstd::Slice<const Res<void>> write;
        void *FSTD_MAYBE_NULL data;
        FWRLD_SysRun system;

        template<typename... Read, typename... Write, typename F>
        static inline constexpr auto init(fstd::Slice<const char> label, const ResList<Read...> &read,
                                          const ResList<Write...> &write, F) noexcept -> SysDescSys {
            return {
                    .label = label,
                    .read = read,
                    .write = write,
                    .data = nullptr,
                    .system = +[](void *, const FWRLD_SysArgs *args) -> FTSK_Fence *FSTD_MAYBE_NULL {
                        const SysArgs<ResArgs<Read...>, ResArgs<Write...>> args2 = {
                                .world = {args->world},
                                .sched = {args->sched},
                                .arena = *static_cast<fstd::Arena *>(args->arena),
                                .read = {.list = {args->read.ptr, args->read.len}},
                                .write = {.list = {args->write.ptr, args->write.len}},
                        };
                        using Ret = decltype((F{})(args2));
                        if constexpr (std::is_same_v<Ret, void>) {
                            std::invoke_r<void>(F{}, args2);
                            return nullptr;
                        }
                        else {
                            return std::invoke_r<FTSK_Fence * FSTD_MAYBE_NULL>(F{}, args2);
                        }
                    },
            };
        }
    };

    struct SysDescSet {
        bool serialize;
        fstd::Slice<const SysDesc> sub_desc;
    };

    /// Description of a system(-set).
    struct SysDesc {
        // NOLINTNEXTLINE(performance-enum-size)
        enum class Tag : fstd::i32 { Sys = FWRLD_SysDescTag_Sys, Set = FWRLD_SysDescTag_Set };

        Tag tag;
        fstd::Slice<const Sys> before;
        fstd::Slice<const Sys> after;
        fstd::Slice<const CondDesc> conditions;
        union {
            SysDescSys sys;
            SysDescSet set;
        };
    };

    template<typename T>
    inline auto Res<T>::deinit() const noexcept -> void {
        if (this->handle)
            fwrld_resource_deinit(this->handle);
    }

    template<typename T>
    [[nodiscard("the resource must be unlocked")]]
    inline auto Res<T>::lock_read() const noexcept -> T * {
        return static_cast<T *>(fwrld_resource_lock_read(this->handle));
    }

    template<typename T>
    inline auto Res<T>::unlock_read() const noexcept -> void {
        fwrld_resource_unlock_read(this->handle);
    }

    template<typename T>
    [[nodiscard("the resource must be unlocked")]]
    inline auto Res<T>::lock_write() const noexcept -> T * {
        return static_cast<T *>(fwrld_resource_lock_write(this->handle));
    }

    template<typename T>
    inline auto Res<T>::unlock_write() const noexcept -> void {
        fwrld_resource_unlock_write(this->handle);
    }

    template<typename T>
    [[nodiscard]]
    inline auto World::add_res(const ResDesc<T> &desc) const noexcept -> Res<T> {
        FWRLD_ResDesc d{.label = desc.label, .value = desc.value};
        return {fwrld_world_add_res(this->handle, &d)};
    }

    /// Adds an empty scheduler to the world.
    [[nodiscard]]
    inline auto World::add_scheduler(const SchedulerDesc &desc) const noexcept -> Scheduler {
        FWRLD_SchedulerDesc d{.label = desc.label, .executor = desc.executor};
        return {fwrld_world_add_scheduler(this->handle, &d)};
    }


    [[nodiscard]]
    inline auto Scheduler::add_sys(const SysDesc &desc) const noexcept -> std::expected<Sys, fstd::Status> {
        Sys sys{};
        const FWRLD_SysDesc *d = reinterpret_cast<const FWRLD_SysDesc *>(&desc);
        auto status = fwrld_scheduler_add_sys(this->handle, d, &sys.handle);
        if (status != FSTD_Status_Ok)
            return std::unexpected(static_cast<fstd::Status>(status));
        return sys;
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
