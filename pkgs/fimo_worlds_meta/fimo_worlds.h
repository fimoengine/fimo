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
typedef struct FWRLD_SysArgs {
    /// World owning the system.
    FWRLD_World world;
    /// Scheduler owning the system.
    FWRLD_Scheduler sched;
    /// Arena whose lifetime matches the execution of all systems
    /// in the current scheduler.
    FSTD_Arena *arena;
    /// Slice of requested resources.
    FSTD_SliceConst(void *) resources;
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

typedef FSTD_I32 FWRLD_SysArgTag;
enum {
    FWRLD_SysArgTag_ReadResource = (FWRLD_SysArgTag)0,
    FWRLD_SysArgTag_WriteResource = (FWRLD_SysArgTag)1,
    FWRLD__SysArgTag_ = FSTD_I32_MAX,
};

typedef struct {
    FWRLD_SysArgTag tag;
    union {
        FWRLD_Res *resource;
    };
} FWRLD_SysArg;

typedef struct FWRLD_SysCondDesc {
    /// List of required arguments.
    FSTD_SliceConst(FWRLD_SysArg) args;
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
            /// List of required arguments.
            FSTD_SliceConst(FWRLD_SysArg) args;
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
    namespace sym {
        constexpr static auto Namespace = FWRLD_SYM_NS;
        constexpr static fstd::Version SymVersion = {FSTD_CTX_VERSION};
        constexpr static auto WorldInit = FWRLD_Sym_WorldInit__Cxx;
        constexpr static auto WorldDeinit = FWRLD_Sym_WorldDeinit__Cxx;
        constexpr static auto WorldAddRes = FWRLD_Sym_WorldAddRes__Cxx;
        constexpr static auto WorldAddScheduler = FWRLD_Sym_WorldAddScheduler__Cxx;
        constexpr static auto ResourceDeinit = FWRLD_Sym_ResourceDeinit__Cxx;
        constexpr static auto ResourceLockRead = FWRLD_Sym_ResourceLockRead__Cxx;
        constexpr static auto ResourceUnlockRead = FWRLD_Sym_ResourceUnlockRead__Cxx;
        constexpr static auto ResourceLockWrite = FWRLD_Sym_ResourceLockWrite__Cxx;
        constexpr static auto ResourceUnlockWrite = FWRLD_Sym_ResourceUnlockWrite__Cxx;
        constexpr static auto SchedulerDeinit = FWRLD_Sym_SchedulerDeinit__Cxx;
        constexpr static auto SchedulerAddSys = FWRLD_Sym_SchedulerAddSys__Cxx;
        constexpr static auto SchedulerRun = FWRLD_Sym_SchedulerRun__Cxx;
        constexpr static auto SchedulerSchedule = FWRLD_Sym_SchedulerSchedule__Cxx;
        constexpr static auto SchedulerFlush = FWRLD_Sym_SchedulerFlush__Cxx;
        constexpr static auto SysDeinit = FWRLD_Sym_SysDeinit__Cxx;

        constexpr static auto AllSymbols = fstd::modules::SymbolImportList{
                WorldInit,        WorldDeinit,        WorldAddRes,       WorldAddScheduler,   ResourceDeinit,
                ResourceLockRead, ResourceUnlockRead, ResourceLockWrite, ResourceUnlockWrite, SchedulerDeinit,
                SchedulerAddSys,  SchedulerRun,       SchedulerSchedule, SchedulerFlush,      SysDeinit,
        };
    } // namespace sym

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

        constexpr World() noexcept = default;
        constexpr explicit World(const FWRLD_World &other) noexcept : handle(other) {};
        constexpr World(const World &other) noexcept = default;
        constexpr World(World &&other) noexcept = default;
        constexpr World &operator=(const World &other) noexcept = default;
        constexpr World &operator=(World &&other) noexcept = default;

        /// Initializes a new empty world.
        static auto init(const WorldDesc &desc) noexcept -> std::expected<World, fstd::Status> {
            World world{};
            FWRLD_WorldDesc d{.label = desc.label};
            auto status = fwrld_world_init(&world.handle, &d);
            if (status != FSTD_Status_Ok)
                return std::unexpected(static_cast<fstd::Status>(status));
            return world;
        }

        /// Deinitializes an empty world.
        auto deinit() const noexcept -> void {
            if (this->handle)
                fwrld_world_deinit(this->handle);
        }

        /// Adds a new resource to the world.
        template<typename T>
        [[nodiscard]]
        auto add_res(const ResDesc<T> &desc) const noexcept -> Res<T>;

        /// Adds an empty scheduler to the world.
        [[nodiscard]]
        auto add_scheduler(const SchedulerDesc &desc) const noexcept -> Scheduler;
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

        constexpr Res() noexcept = default;
        constexpr explicit Res(const FWRLD_Res &other) noexcept : handle(other) {};
        constexpr Res(const Res &other) noexcept = default;
        constexpr Res(Res &&other) noexcept = default;
        constexpr Res &operator=(const Res &other) noexcept = default;
        constexpr Res &operator=(Res &&other) noexcept = default;
        constexpr operator Res<void>() noexcept { return Res{this->handle}; }

        /// Invalidates the resource.
        ///
        /// The resource may not be in use.
        auto deinit() const noexcept -> void;

        /// Acquires the resource with read access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        [[nodiscard("the resource must be unlocked")]]
        auto lock_read() const noexcept -> T *;

        /// Unlocks a resource acquired with read access.
        auto unlock_read() const noexcept -> void;

        /// Acquires the resource with write access.
        ///
        /// __NOTE__: This may inhibit the scheduling of systems, as it is a valid implementation
        /// strategy to acquire all necessary resources before executing any system.
        [[nodiscard("the resource must be unlocked")]]
        auto lock_write() const noexcept -> T *;

        /// Unlocks a resource acquired with write access.
        auto unlock_write() const noexcept -> void;
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

        constexpr Scheduler() noexcept = default;
        constexpr explicit Scheduler(const FWRLD_Scheduler &other) noexcept : handle(other) {};
        constexpr Scheduler(const Scheduler &other) noexcept = default;
        constexpr Scheduler(Scheduler &&other) noexcept = default;
        constexpr Scheduler &operator=(const Scheduler &other) noexcept = default;
        constexpr Scheduler &operator=(Scheduler &&other) noexcept = default;

        /// Deinitializes an empty world.
        auto deinit() const noexcept -> void {
            if (this->handle)
                fwrld_scheduler_deinit(this->handle);
        }

        /// Adds a system(-set) to the scheduler.
        [[nodiscard]]
        auto add_sys(const SysDesc &desc) const noexcept -> std::expected<Sys, fstd::Status>;

        /// Starts a new run of the systems, blocking the current thread until it completes.
        auto run(fstd::Arena &arena) const noexcept -> void { fwrld_scheduler_run(this->handle, &arena); }

        /// Schedules the systems of the scheduler to be run asynchronously.
        ///
        /// Multiple concurrent schedule operations are serialized. The arena must remain valid until `completion` is
        /// signaled. The systems will start running after `start` is signaled. If no executor is associated with the
        /// scheduler, this operation will block the calling thread until all systems are run.
        auto schedule(fstd::Arena &arena, FTSK_Fence *FSTD_MAYBE_NULL start,
                      FTSK_Fence *FSTD_MAYBE_NULL completion) const noexcept -> void {
            fwrld_scheduler_schedule(this->handle, &arena, start, completion);
        }

        /// Blocks the calling thread until all scheduled operations are completed.
        auto flush() const noexcept -> void { fwrld_scheduler_flush(this->handle); }
    };

    namespace detail {
        struct Arg {
            // NOLINTNEXTLINE(performance-enum-size)
            enum class Tag : fstd::i32 {
                ReadResource = FWRLD_SysArgTag_ReadResource,
                WriteResource = FWRLD_SysArgTag_WriteResource,
            };
            union {
                Res<void> resource;
            };
        };
    } // namespace detail

    template<typename T>
    struct ReadResouceArg {
        detail::Arg::Tag tag;
        Res<T> resource;

        constexpr ReadResouceArg(const Res<T> &other) noexcept :
            tag(detail::Arg::Tag::ReadResource), resource(other) {};
        constexpr ReadResouceArg(const ReadResouceArg &other) noexcept = default;
        constexpr ReadResouceArg(ReadResouceArg &&other) noexcept = default;
        constexpr ReadResouceArg &operator=(const ReadResouceArg &other) noexcept = default;
        constexpr ReadResouceArg &operator=(ReadResouceArg &&other) noexcept = default;
    };

    template<typename T>
    struct WriteResouceArg {
        detail::Arg::Tag tag;
        Res<T> resource;

        constexpr WriteResouceArg(const Res<T> &other) noexcept :
            tag(detail::Arg::Tag::WriteResource), resource(other) {};
        constexpr WriteResouceArg(const WriteResouceArg &other) noexcept = default;
        constexpr WriteResouceArg(WriteResouceArg &&other) noexcept = default;
        constexpr WriteResouceArg &operator=(const WriteResouceArg &other) noexcept = default;
        constexpr WriteResouceArg &operator=(WriteResouceArg &&other) noexcept = default;
    };

    template<typename T>
    struct Read {
        T *ptr;

        constexpr explicit Read(T *other) noexcept : ptr(other) {};
        constexpr Read(const Read &other) noexcept = default;
        constexpr Read(Read &&other) noexcept = default;
        constexpr Read &operator=(const Read &other) noexcept = default;
        constexpr Read &operator=(Read &&other) noexcept = default;
        constexpr T &operator*() const noexcept { return *ptr; }
        constexpr T *operator->() const noexcept { return ptr; }
    };

    template<typename T>
    struct Write {
        T *ptr;

        constexpr explicit Write(T *other) noexcept : ptr(other) {};
        constexpr Write(const Write &other) noexcept = default;
        constexpr Write(Write &&other) noexcept = default;
        constexpr Write &operator=(const Write &other) noexcept = default;
        constexpr Write &operator=(Write &&other) noexcept = default;
        constexpr T &operator*() const noexcept { return *ptr; }
        constexpr T *operator->() const noexcept { return ptr; }
    };

    namespace detail {
        template<typename T>
        struct IsConstValue : fstd::ConstexprValue<false> {};
        template<auto V>
        struct IsConstValue<fstd::ConstexprValue<V>> : fstd::ConstexprValue<true> {};

        template<typename T, typename... Ts>
        struct IsContained;
        template<typename T, typename Head, typename... Rest>
        struct IsContained<T, Head, Rest...>
            : fstd::ConstexprValue<std::is_same_v<T, Head> or IsContained<T, Rest...>::Value> {};
        template<typename T>
        struct IsContained<T> : fstd::ConstexprValue<false> {};

        template<typename... Ts>
        struct IsUnique;
        template<typename Head, typename... Rest>
        struct IsUnique<Head, Rest...>
            : fstd::ConstexprValue<!IsContained<Head, Rest...>::Value and IsUnique<Rest...>::Value> {};
        template<>
        struct IsUnique<> : fstd::ConstexprValue<true> {};

        template<typename T>
        struct IsArg : fstd::ConstexprValue<false> {};
        template<>
        struct IsArg<World> : fstd::ConstexprValue<true> {
            constexpr static bool IsExplicit = false;
        };
        template<>
        struct IsArg<Scheduler> : fstd::ConstexprValue<true> {
            constexpr static bool IsExplicit = false;
        };
        template<>
        struct IsArg<fstd::Arena &> : fstd::ConstexprValue<true> {
            constexpr static bool IsExplicit = false;
        };
        template<typename T>
        struct IsArg<Read<T>> : fstd::ConstexprValue<true> {
            using Arg = ReadResouceArg<T>;
            constexpr static bool IsExplicit = true;
        };
        template<typename T>
        struct IsArg<Write<T>> : fstd::ConstexprValue<true> {
            using Arg = WriteResouceArg<T>;
            constexpr static bool IsExplicit = true;
        };

        template<typename T>
        struct IsArgList;
        template<typename Head, typename... Rest>
        struct IsArgList<fstd::Tuple<Head, Rest...>>
            : fstd::ConstexprValue<IsArg<Head>::Value and IsUnique<Head, Rest...>::Value and
                                   IsArgList<fstd::Tuple<Rest...>>::Value> {};
        template<>
        struct IsArgList<fstd::Tuple<>> : fstd::ConstexprValue<true> {};

        template<typename T>
        struct FunctionArgs;
        template<typename Ret, typename... Args>
        struct FunctionArgs<Ret (*)(Args...)> {
            using StaticType = Ret (*)(Args...);
            using ReturnType = Ret;
            using ArgsType = fstd::Tuple<Args...>;
        };
        template<typename Ret, typename T, typename... Args>
        struct FunctionArgs<Ret (T::*)(Args...)> {
            using StaticType = Ret (*)(Args...);
            using ReturnType = Ret;
            using ArgsType = fstd::Tuple<Args...>;
        };
        template<typename Ret, typename T, typename... Args>
        struct FunctionArgs<Ret (T::*)(Args...) const> {
            using StaticType = Ret (*)(Args...);
            using ReturnType = Ret;
            using ArgsType = fstd::Tuple<Args...>;
        };

        template<typename T>
        struct CallableInfo;
        template<typename Ret, typename... Args_>
        struct CallableInfo<Ret (*)(Args_...)> {
            using Type = Ret (*)(Args_...);
            using StaticType = FunctionArgs<Type>::StaticType;
            using Return = FunctionArgs<Type>::ReturnType;
            using Args = FunctionArgs<Type>::ArgsType;
        };
        template<typename T>
            requires(!std::is_function_v<T>)
        struct CallableInfo<T> {
            using Type = decltype(&T::operator());
            using StaticType = FunctionArgs<Type>::StaticType;
            using Return = FunctionArgs<Type>::ReturnType;
            using Args = FunctionArgs<Type>::ArgsType;
        };

        template<typename T>
        struct ArgTuple {
            using Type = fstd::Tuple<>;
        };
        template<typename T>
            requires IsArg<T>::IsExplicit
        struct ArgTuple<T> {
            using Type = fstd::Tuple<typename IsArg<T>::Arg>;
        };

        template<typename T>
        struct CollectArgs;

        template<typename... Args>
        struct CollectArgs<fstd::Tuple<Args...>> {
            using Type = decltype(fstd::concatTuple(std::declval<typename ArgTuple<Args>::Type>()...));
        };

        template<typename T>
            requires IsArgList<typename FunctionArgs<T>::ArgsType>::Value
        struct ArgsTuple {
            using Type = CollectArgs<typename FunctionArgs<T>::ArgsType>::Type;
            template<fstd::usize... I, typename... Ts>
            constexpr static Type makeArgs(fstd::Tuple<Ts...> &&tuple, std::index_sequence<I...>) noexcept {
                return fstd::makeTuple(std::forward<Ts>(fstd::get<I>(std::forward<decltype(tuple)>(tuple)))...);
            }
            template<typename... Ts>
            constexpr static Type makeArgs(fstd::Tuple<Ts...> &&tuple) noexcept {
                return makeArgs(std::forward<decltype(tuple)>(tuple), std::make_index_sequence<sizeof...(Ts)>{});
            }
        };

        static_assert(std::is_same_v<ArgsTuple<void (*)()>::Type, fstd::Tuple<>>);
        static_assert(std::is_same_v<ArgsTuple<void (*)(World)>::Type, fstd::Tuple<>>);
        static_assert(std::is_same_v<ArgsTuple<void (*)(Scheduler)>::Type, fstd::Tuple<>>);
        static_assert(std::is_same_v<ArgsTuple<void (*)(fstd::Arena &)>::Type, fstd::Tuple<>>);
        static_assert(std::is_same_v<ArgsTuple<void (*)(Read<int>)>::Type, fstd::Tuple<ReadResouceArg<int>>>);
        static_assert(std::is_same_v<ArgsTuple<void (*)(Write<int>)>::Type, fstd::Tuple<WriteResouceArg<int>>>);
        static_assert(std::is_same_v<
                      ArgsTuple<void (*)(World, Scheduler, fstd::Arena &, Read<int>, Read<float>, Write<int>)>::Type,
                      fstd::Tuple<ReadResouceArg<int>, ReadResouceArg<float>, WriteResouceArg<int>>>);

        template<typename T, fstd::usize ResIdx>
            requires IsArg<T>::Value
        struct UnpackArgHelper;
        template<fstd::usize ResIdx>
        struct UnpackArgHelper<World, ResIdx> {
            constexpr static fstd::usize NextResIdx = ResIdx;
            constexpr static World get(const FWRLD_SysArgs *args) noexcept { return World{args->world}; }
        };
        template<fstd::usize ResIdx>
        struct UnpackArgHelper<Scheduler, ResIdx> {
            constexpr static fstd::usize NextResIdx = ResIdx;
            constexpr static Scheduler get(const FWRLD_SysArgs *args) noexcept { return Scheduler{args->sched}; }
        };
        template<fstd::usize ResIdx>
        struct UnpackArgHelper<fstd::Arena &, ResIdx> {
            constexpr static fstd::usize NextResIdx = ResIdx;
            constexpr static fstd::Arena &get(const FWRLD_SysArgs *args) noexcept {
                return *static_cast<fstd::Arena *>(args->arena);
            }
        };
        template<typename T, fstd::usize ResIdx>
        struct UnpackArgHelper<Read<T>, ResIdx> {
            constexpr static fstd::usize NextResIdx = ResIdx + 1;
            constexpr static Read<T> get(const FWRLD_SysArgs *args) noexcept {
                return Read<T>{static_cast<T *>(const_cast<void *>(args->resources.ptr[ResIdx]))};
            }
        };
        template<typename T, fstd::usize ResIdx>
        struct UnpackArgHelper<Write<T>, ResIdx> {
            constexpr static fstd::usize NextResIdx = ResIdx + 1;
            constexpr static Write<T> get(const FWRLD_SysArgs *args) noexcept {
                return Write<T>{static_cast<T *>(const_cast<void *>(args->resources.ptr[ResIdx]))};
            }
        };

        template<typename Tuple, fstd::usize ResIdx, typename Args>
            requires IsArgList<Args>::Value
        struct UnpackArgsCollector;
        template<typename Tuple, fstd::usize ResIdx, typename Head, typename... Rest>
        struct UnpackArgsCollector<Tuple, ResIdx, fstd::Tuple<Head, Rest...>> {
            using Helper = UnpackArgHelper<Head, ResIdx>;
            using ConcatTuple = decltype(fstd::concatTuple(std::declval<Tuple>(), std::declval<fstd::Tuple<Helper>>()));
            using Type = typename UnpackArgsCollector<ConcatTuple, Helper::NextResIdx, fstd::Tuple<Rest...>>::Type;
        };
        template<typename Tuple, fstd::usize ResIdx>
        struct UnpackArgsCollector<Tuple, ResIdx, fstd::Tuple<>> {
            using Type = Tuple;
        };

        template<typename Tuple>
        struct UnpackArgsImpl;
        template<typename... Ts>
        struct UnpackArgsImpl<fstd::Tuple<Ts...>> {
            constexpr static std::tuple<decltype(Ts::get(nullptr))...> get(const FWRLD_SysArgs *args) noexcept {
                return std::tuple<decltype(Ts::get(nullptr))...>(Ts::get(args)...);
            }
        };

        template<typename F>
        struct UnpackArgsHelper {
            using Collector = UnpackArgsCollector<fstd::Tuple<>, 0, typename CallableInfo<F>::Args>;
            using Impl = UnpackArgsImpl<typename Collector::Type>;
            using Return = decltype(Impl::get(nullptr));
            constexpr static Return get(const FWRLD_SysArgs *args) noexcept { return Impl::get(args); }
        };
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)()>::Return, std::tuple<>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(World)>::Return, std::tuple<World>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(Scheduler)>::Return, std::tuple<Scheduler>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(fstd::Arena &)>::Return, std::tuple<fstd::Arena &>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(Read<int>)>::Return, std::tuple<Read<int>>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(Write<int>)>::Return, std::tuple<Write<int>>>);
        static_assert(std::is_same_v<UnpackArgsHelper<void (*)(World, Scheduler, fstd::Arena &, Read<int>, Read<float>,
                                                               Write<int>)>::Return,
                                     std::tuple<World, Scheduler, fstd::Arena &, Read<int>, Read<float>, Write<int>>>);

        template<typename F>
        constexpr static typename UnpackArgsHelper<F>::Return makeArgsTuple(const FWRLD_SysArgs *args) {
            return UnpackArgsHelper<F>::get(args);
        }

        struct Condition {
            fstd::Slice<const Arg> args;
            void *context;
            FWRLD_SysCond condition;

            constexpr Condition(fstd::Slice<const Arg> args, void *context, FWRLD_SysCond condition) noexcept :
                args(args), context(context), condition(condition) {}
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr Condition(const ArgsTuple<typename CallableInfo<F>::Type>::Type &args, F f) noexcept {
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                static_assert(std::is_convertible_v<F, typename CallableInfo<F>::StaticType>);
                using StaticType = typename CallableInfo<F>::StaticType;
                StaticType cf = f;
                this->context = const_cast<void *>(reinterpret_cast<const void *>(f));
                this->condition = [](void *ctx, const FWRLD_SysArgs *args) -> bool {
                    StaticType f = reinterpret_cast<StaticType>(const_cast<const void *>(ctx));
                    return std::apply(f, makeArgsTuple<F>(args));
                };
            };
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr Condition(const ArgsTuple<typename CallableInfo<F>::Type>::Type &args, F &f) noexcept {
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                if constexpr (std::is_convertible_v<F, typename CallableInfo<F>::StaticType>) {
                    using StaticType = typename CallableInfo<F>::StaticType;
                    StaticType cf = f;
                    this->context = const_cast<void *>(reinterpret_cast<const void *>(f));
                    this->condition = [](void *ctx, const FWRLD_SysArgs *args) -> bool {
                        StaticType f = reinterpret_cast<StaticType>(const_cast<const void *>(ctx));
                        return std::apply(f, makeArgsTuple<F>(args));
                    };
                }
                else {
                    this->context = const_cast<std::remove_cvref_t<F> *>(&f);
                    this->condition = [](void *ctx, const FWRLD_SysArgs *args) -> bool {
                        F &f = *static_cast<F *>(ctx);
                        return std::apply(f, makeArgsTuple<F>(args));
                    };
                }
            };
            template<typename F, F f>
            constexpr Condition(const ArgsTuple<typename CallableInfo<F>::Type>::Type &args,
                                fstd::ConstexprValue<f>) noexcept {
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                this->context = nullptr;
                this->condition = [](void *, const FWRLD_SysArgs *args) -> bool {
                    return std::apply(f, makeArgsTuple<F>(args));
                };
            };
            constexpr Condition(const Condition &) noexcept = default;
            constexpr Condition(Condition &&) noexcept = default;
            constexpr Condition &operator=(const Condition &) noexcept = default;
            constexpr Condition &operator=(Condition &&) noexcept = default;
        };

        struct System {
            fstd::StrConst label;
            fstd::Slice<const Arg> args;
            void *context;
            FWRLD_SysRun run;

            constexpr System(fstd::Slice<const Arg> args, void *context, FWRLD_SysRun run) noexcept :
                System({}, args, context, run) {}
            constexpr System(fstd::StrConst label, fstd::Slice<const Arg> args, void *context,
                             FWRLD_SysRun run) noexcept : label(label), args(args), context(context), run(run) {}
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr System(const ArgsTuple<typename CallableInfo<F>::Type>::Type &args, F f) noexcept :
                System({}, args, f) {}
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr System(fstd::StrConst label, const ArgsTuple<typename CallableInfo<F>::Type>::Type &args,
                             F f) noexcept : label(label) {
                static_assert(std::is_same_v<void, typename CallableInfo<F>::Return> or
                              std::is_same_v<FTSK_Fence *, typename CallableInfo<F>::Return>);
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                static_assert(std::is_convertible_v<F, typename CallableInfo<F>::StaticType>);
                using StaticType = typename CallableInfo<F>::StaticType;
                StaticType cf = f;
                this->context = const_cast<void *>(reinterpret_cast<const void *>(cf));
                this->run = [](void *ctx, const FWRLD_SysArgs *args) -> FTSK_Fence * {
                    StaticType f = reinterpret_cast<StaticType>(ctx);
                    if constexpr (std::is_same_v<void, typename CallableInfo<F>::Return>) {
                        std::apply(f, makeArgsTuple<F>(args));
                        return nullptr;
                    }
                    else {
                        return std::apply(f, makeArgsTuple<F>(args));
                    }
                };
            }
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr System(const ArgsTuple<typename CallableInfo<F>::Type>::Type &args, F &f) noexcept :
                System({}, args, f) {}
            template<typename F>
                requires(!IsConstValue<F>::Value)
            constexpr System(fstd::StrConst label, const ArgsTuple<typename CallableInfo<F>::Type>::Type &args,
                             F &f) noexcept : label(label) {
                static_assert(std::is_same_v<void, typename CallableInfo<F>::Return> or
                              std::is_same_v<FTSK_Fence *, typename CallableInfo<F>::Return>);
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                if constexpr (std::is_convertible_v<F, typename CallableInfo<F>::StaticType>) {
                    using StaticType = typename CallableInfo<F>::StaticType;
                    StaticType cf = f;
                    this->context = const_cast<void *>(reinterpret_cast<const void *>(cf));
                    this->run = [](void *ctx, const FWRLD_SysArgs *args) -> FTSK_Fence * {
                        StaticType f = reinterpret_cast<StaticType>(const_cast<const void *>(ctx));
                        if constexpr (std::is_same_v<void, typename CallableInfo<F>::Return>) {
                            std::apply(f, makeArgsTuple<F>(args));
                            return nullptr;
                        }
                        else {
                            return std::apply(f, makeArgsTuple<F>(args));
                        }
                    };
                }
                else {
                    this->context = const_cast<std::remove_cvref_t<F> *>(&f);
                    this->run = [](void *ctx, const FWRLD_SysArgs *args) -> FTSK_Fence * {
                        F &f = *static_cast<F *>(ctx);
                        if constexpr (std::is_same_v<void, typename CallableInfo<F>::Return>) {
                            std::apply(f, makeArgsTuple<F>(args));
                            return nullptr;
                        }
                        else {
                            return std::apply(f, makeArgsTuple<F>(args));
                        }
                    };
                }
            }
            template<auto f>
            constexpr System(const ArgsTuple<typename CallableInfo<decltype(f)>::Type>::Type &args,
                             fstd::ConstexprValue<f> v) noexcept : System({}, args, v) {}
            template<auto f>
            constexpr System(fstd::StrConst label,
                             const ArgsTuple<typename CallableInfo<decltype(f)>::Type>::Type &args,
                             fstd::ConstexprValue<f>) noexcept : label(label) {
                static_assert(std::is_same_v<void, typename CallableInfo<decltype(f)>::Return> or
                              std::is_same_v<FTSK_Fence *, typename CallableInfo<decltype(f)>::Return>);
                this->args = {reinterpret_cast<const Arg *>(&args),
                              fstd::TupleSizeV<std::remove_cvref_t<decltype(args)>>};
                this->context = nullptr;
                this->run = [](void *, const FWRLD_SysArgs *args) -> FTSK_Fence * {
                    if constexpr (std::is_same_v<void, typename CallableInfo<decltype(f)>::Return>) {
                        std::apply(f, makeArgsTuple<decltype(f)>(args));
                        return nullptr;
                    }
                    else {
                        return std::apply(f, makeArgsTuple<decltype(f)>(args));
                    }
                };
            }
            constexpr System(const System &) noexcept = default;
            constexpr System(System &&) noexcept = default;
            constexpr System &operator=(const System &) noexcept = default;
            constexpr System &operator=(System &&) noexcept = default;
        };

        struct SubSet {
            bool serialize;
            fstd::Slice<const SysDesc> sub_desc;

            constexpr SubSet(fstd::Slice<const SysDesc> sub_desc) noexcept : serialize(false), sub_desc(sub_desc) {}
            constexpr SubSet(bool serialize, fstd::Slice<const SysDesc> sub_desc) noexcept :
                serialize(serialize), sub_desc(sub_desc) {}
            constexpr SubSet(const SubSet &other) noexcept = default;
            constexpr SubSet(SubSet &&other) noexcept = default;

            constexpr SubSet &operator=(const SubSet &other) noexcept = default;
            constexpr SubSet &operator=(SubSet &&other) noexcept = default;
        };
    } // namespace detail

    /// Description of a system(-set).
    struct SysDesc {
        // NOLINTNEXTLINE(performance-enum-size)
        enum class Tag : fstd::i32 { Sys = FWRLD_SysDescTag_Sys, Set = FWRLD_SysDescTag_Set };
        template<typename F>
        using ArgsTuple = detail::ArgsTuple<F>::Type;
        using Condition = detail::Condition;
        using System = detail::System;
        using SubSet = detail::SubSet;

        Tag tag;
        fstd::Slice<const Sys> before;
        fstd::Slice<const Sys> after;
        fstd::Slice<const Condition> conditions;
        union {
            System sys;
            SubSet set;
        };

        constexpr SysDesc(System sys) noexcept : SysDesc({}, {}, {}, sys) {}
        constexpr SysDesc(fstd::Slice<const Condition> conditions, System sys) noexcept :
            SysDesc({}, {}, conditions, sys) {}
        constexpr SysDesc(fstd::Slice<const Sys> before, fstd::Slice<const Sys> after, System sys) noexcept :
            SysDesc(before, after, {}, sys) {}
        constexpr SysDesc(fstd::Slice<const Sys> before, fstd::Slice<const Sys> after,
                          fstd::Slice<const Condition> conditions, System sys) noexcept :
            tag(Tag::Sys), before(before), after(after), conditions(conditions), sys(sys) {}
        constexpr SysDesc(SubSet set) noexcept : SysDesc({}, {}, {}, set) {}
        constexpr SysDesc(fstd::Slice<const Condition> conditions, SubSet set) noexcept :
            SysDesc({}, {}, conditions, set) {}
        constexpr SysDesc(fstd::Slice<const Sys> before, fstd::Slice<const Sys> after, SubSet set) noexcept :
            SysDesc(before, after, {}, set) {}
        constexpr SysDesc(fstd::Slice<const Sys> before, fstd::Slice<const Sys> after,
                          fstd::Slice<const Condition> conditions, SubSet set) noexcept :
            tag(Tag::Sys), before(before), after(after), conditions(conditions), set(set) {}
        constexpr SysDesc(const SysDesc &other) noexcept = default;
        constexpr SysDesc(SysDesc &&other) noexcept = default;
        constexpr SysDesc &operator=(const SysDesc &other) noexcept = default;
        constexpr SysDesc &operator=(SysDesc &&other) noexcept = default;
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
        auto deinit(FTSK_Fence *FSTD_MAYBE_NULL fence = nullptr) const noexcept -> void {
            if (this->handle)
                fwrld_sys_deinit(this->handle, fence);
            else if (fence)
                ftsk_fence_signal(fence);
        }
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
        return Res<T>{fwrld_world_add_res(this->handle, &d)};
    }

    /// Adds an empty scheduler to the world.
    [[nodiscard]]
    inline auto World::add_scheduler(const SchedulerDesc &desc) const noexcept -> Scheduler {
        FWRLD_SchedulerDesc d{.label = desc.label, .executor = desc.executor};
        return Scheduler{fwrld_world_add_scheduler(this->handle, &d)};
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
