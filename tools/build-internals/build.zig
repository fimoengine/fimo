const std = @import("std");
const Allocator = std.mem.Allocator;
const Target = std.Target;
const Build = std.Build;
const LazyPath = Build.LazyPath;
const ArrayListUnmanaged = std.ArrayListUnmanaged;
const AutoArrayHashMapUnmanaged = std.AutoArrayHashMapUnmanaged;
const StringArrayHashMapUnmanaged = std.StringArrayHashMapUnmanaged;

pub fn build(b: *Build) void {
    _ = b.addModule("build-internals", .{
        .root_source_file = b.path("build.zig"),
    });
}

pub const fimo_version: std.SemanticVersion = .{ .major = 0, .minor = 2, .patch = 0, .pre = "dev" };

pub const FimoBuild = struct {
    build: *Build,
    build_dep: *Build.Dependency,
    graph: *Graph,

    pub const Graph = struct {
        allocator: Allocator,
        target: Build.ResolvedTarget,
        optimize: std.builtin.OptimizeMode,
        pkgs: StringArrayHashMapUnmanaged(*Package) = .empty,
        modules: StringArrayHashMapUnmanaged(*Module) = .empty,
        dependencies: AutoArrayHashMapUnmanaged(usize, void) = .empty,

        pub fn addPackage(self: *Graph, pkg: *Package) *Package {
            if (self.pkgs.contains(pkg.name))
                std.debug.panic("package `{s}` already defined", .{pkg.name});
            self.pkgs.put(self.allocator, pkg.name, pkg) catch @panic("oom");
            return pkg;
        }

        pub fn getPackage(self: *Graph, name: []const u8) ?*Package {
            return self.pkgs.get(name);
        }

        pub fn addModule(self: *Graph, mod: *Module) *Module {
            if (self.modules.contains(mod.name))
                std.debug.panic("module `{s}` already defined", .{mod.name});
            self.modules.put(self.allocator, mod.name, mod) catch @panic("oom");
            return mod;
        }

        pub fn getModule(self: *Graph, name: []const u8) ?*Module {
            return self.modules.get(name);
        }

        fn markDependencyLoaded(self: *Graph, dep: *Build.Dependency) void {
            self.dependencies.put(self.allocator, @intFromPtr(dep), {}) catch @panic("oom");
        }

        fn dependencyLoaded(self: *Graph, dep: *Build.Dependency) bool {
            return self.dependencies.contains(@intFromPtr(dep));
        }
    };

    pub const CreatePackageOptions = struct {
        name: []const u8,
        root_module: *Build.Module,
        headers: ?LazyPath = null,
    };

    pub const Package = struct {
        owner: *FimoBuild,
        name: []const u8,
        root_module: *Build.Module,
        headers: ?LazyPath,
        tests: ArrayListUnmanaged(*Test) = .empty,

        pub fn create(owner: *FimoBuild, options: CreatePackageOptions) *Package {
            const self = owner.build.allocator.create(Package) catch @panic("oom");
            self.* = .{
                .owner = owner,
                .name = owner.build.dupe(options.name),
                .root_module = options.root_module,
                .headers = options.headers,
            };
            return self;
        }

        pub fn addTest(self: *Package, options: CreateTestOptions) *Test {
            const t = Test.create(self.owner, options);
            self.tests.append(self.owner.build.allocator, t) catch @panic("oom");
            return t;
        }

        pub fn addInstallHeaders(self: *Package) ?*Build.Step.InstallDir {
            return self.addInstallHeadersEx(self.owner.build);
        }

        pub fn addInstallHeadersEx(self: *Package, b: *Build) ?*Build.Step.InstallDir {
            if (self.headers == null) return null;
            return b.addInstallDirectory(.{
                .source_dir = self.headers.?,
                .install_dir = .header,
                .install_subdir = ".",
            });
        }
    };

    pub const CreateModuleOptions = struct {
        name: []const u8,
        root_module: ?*Build.Module = null,
        headers: ?LazyPath = null,
        module_deps: []const []const u8 = &.{},
    };

    pub const Module = struct {
        owner: *FimoBuild,
        name: []const u8,
        root_module: ?*Build.Module,
        headers: ?LazyPath,
        module_deps: []const []const u8,
        tests: ArrayListUnmanaged(*Test) = .empty,

        link_module: ?*Build.Module = null,
        static_lib: ?*Build.Step.Compile = null,
        dynamic_lib: ?*Build.Step.Compile = null,

        pub fn create(owner: *FimoBuild, options: CreateModuleOptions) *Module {
            const allocator = owner.build.allocator;
            const self = allocator.create(Module) catch @panic("oom");
            self.* = .{
                .owner = owner,
                .name = owner.build.dupe(options.name),
                .root_module = options.root_module,
                .headers = options.headers,
                .module_deps = blk: {
                    const deps = allocator.alloc([]const u8, options.module_deps.len) catch @panic("oom");
                    for (deps, options.module_deps) |*dst, src| {
                        dst.* = owner.build.dupe(src);
                    }
                    break :blk deps;
                },
            };
            return self;
        }

        pub fn getLinkModule(self: *Module) *Build.Module {
            if (self.link_module) |x| return x;

            const owner = self.owner;
            const b = owner.build;

            const wf = b.addWriteFiles();

            var bytes = ArrayListUnmanaged(u8).empty;
            bytes.appendSlice(b.allocator,
                \\ const std = @import("std");
                \\ fn refAllExports(comptime T: type) void {
                \\    inline for (comptime std.meta.declarations(T)) |decl| {
                \\        _ = &@field(T, decl.name);
                \\    }
                \\ }
                \\ pub fn forceExportModules() void {
                \\    @setEvalBranchQuota(2000000);
                \\    inline for (comptime std.meta.declarations(@This())) |decl| {
                \\        if (@TypeOf(@field(@This(), decl.name)) == type) {
                \\            switch (@typeInfo(@field(@This(), decl.name))) {
                \\                .@"struct", .@"enum", .@"union", .@"opaque" => {
                \\                    refAllExports(@field(@This(), decl.name));
                \\                    if (@hasDecl(@field(@This(), decl.name), "forceExportModules")) {
                \\                        @field(@This(), decl.name).forceExportModules();
                \\                    }
                \\                    if (@hasDecl(@field(@This(), decl.name), "fimo_export")) {
                \\                        refAllExports(@field(@This(), decl.name).fimo_export);
                \\                    }
                \\                },
                \\                else => {},
                \\            }
                \\        }
                \\    }
                \\ }
                \\ comptime { forceExportModules(); }
                \\
            ) catch @panic("oom");
            if (self.root_module != null) {
                bytes.appendSlice(
                    b.allocator,
                    "pub const module = @import(\"module\");\n",
                ) catch @panic("oom");
            }
            for (self.module_deps) |dep| {
                bytes.appendSlice(b.allocator, b.fmt(
                    "pub const {s} = @import(\"{s}\");\n",
                    .{ dep, dep },
                )) catch @panic("oom");
            }

            const root_path = wf.add("root.zig", bytes.items);
            const root_module = b.createModule(.{
                .root_source_file = root_path,
                .target = owner.graph.target,
                .optimize = owner.graph.optimize,
            });
            if (self.root_module) |root| root_module.addImport("module", root);

            for (self.module_deps) |dep| {
                if (std.mem.eql(u8, dep, self.name))
                    std.debug.panic("module `{s}` depends on itself", .{self.name});
                const module = owner.getModule(dep);
                const link_mod = module.getLinkModule();
                root_module.addImport(dep, link_mod);
            }

            self.link_module = root_module;
            return root_module;
        }

        pub fn getStaticLib(self: *Module) *Build.Step.Compile {
            if (self.static_lib) |x| return x;

            const owner = self.owner;
            const b = owner.build;

            const root_module = self.getLinkModule();
            const artifact = b.addLibrary(.{
                .linkage = .static,
                .name = self.name,
                .root_module = root_module,
            });
            artifact.bundle_compiler_rt = true;

            self.static_lib = artifact;
            return artifact;
        }

        pub fn getDynamicLib(self: *Module) *Build.Step.Compile {
            if (self.dynamic_lib) |x| return x;

            const owner = self.owner;
            const b = owner.build;

            const root_module = self.getLinkModule();
            const artifact = b.addLibrary(.{
                .linkage = .dynamic,
                .name = self.name,
                .root_module = root_module,
            });

            self.dynamic_lib = artifact;
            return artifact;
        }

        pub fn addTest(self: *Module, options: CreateTestOptions) *Test {
            const t = Test.create(self.owner, options);
            self.tests.append(self.owner.build.allocator, t) catch @panic("oom");
            return t;
        }
    };

    pub const TestStep = union(enum) {
        module: *Build.Module,
        executable: *Build.Step.Compile,
    };

    pub const CreateTestOptions = struct {
        name: []const u8 = "test",
        step: TestStep,
        configure: ?*const fn (pkg: *Test) void = null,
    };

    pub const Test = struct {
        owner: *FimoBuild,
        name: []const u8,
        step: TestStep,
        configure: ?*const fn (pkg: *Test) void = null,
        artifact: ?*Build.Step.Compile = null,
        run_artifact: ?*Build.Step.Run = null,

        pub fn create(owner: *FimoBuild, options: CreateTestOptions) *Test {
            const allocator = owner.build.allocator;
            const self = allocator.create(Test) catch @panic("oom");
            self.* = .{
                .owner = owner,
                .name = owner.build.dupe(options.name),
                .step = options.step,
                .configure = options.configure,
            };
            return self;
        }

        pub fn getArtifact(self: *Test) *Build.Step.Compile {
            if (self.artifact) |x| return x;
            if (self.configure) |f| f(self);

            const owner = self.owner;
            const b = owner.build;
            const artifact = switch (self.step) {
                .module => |root_module| b.addTest(.{
                    .name = self.name,
                    .root_module = root_module,
                    .test_runner = .{
                        .path = owner.build_dep.path("test_runner.zig"),
                        .mode = .simple,
                    },
                }),
                .executable => |exe| exe,
            };

            self.artifact = artifact;
            return artifact;
        }

        pub fn getRunArtifact(self: *Test) *Build.Step.Run {
            if (self.run_artifact) |x| return x;

            const owner = self.owner;
            const b = owner.build;
            const artifact = self.getArtifact();

            const run_artifact = b.addRunArtifact(artifact);
            self.run_artifact = run_artifact;
            return run_artifact;
        }
    };

    pub const CreateOptions = struct {
        build: *Build,
        build_dep: *Build.Dependency,
        target: Build.ResolvedTarget,
        optimize: std.builtin.OptimizeMode,
    };

    pub fn createRoot(options: CreateOptions) *FimoBuild {
        const allocator = options.build.allocator;
        const graph = allocator.create(Graph) catch @panic("oom");
        graph.* = .{
            .allocator = allocator,
            .target = options.target,
            .optimize = options.optimize,
        };

        const self = allocator.create(FimoBuild) catch @panic("oom");
        self.* = .{
            .build = options.build,
            .build_dep = options.build_dep,
            .graph = graph,
        };
        return self;
    }

    pub fn lazyImport(
        self: *FimoBuild,
        comptime asking_build_zig: type,
        comptime dep_name: []const u8,
    ) ?struct { type, *Build.Dependency } {
        const b = self.build;
        const build_zig = b.lazyImport(asking_build_zig, dep_name) orelse return null;
        const dep = b.dependencyFromBuildZig(build_zig, .{});

        if (!self.graph.dependencyLoaded(dep)) {
            const allocator = self.build.allocator;
            const child = allocator.create(FimoBuild) catch @panic("oom");
            child.* = .{
                .build = dep.builder,
                .build_dep = self.build_dep,
                .graph = self.graph,
            };
            build_zig.configure(child);
            self.graph.markDependencyLoaded(dep);
        }

        return .{ build_zig, dep };
    }

    pub fn lazyDependency(self: *FimoBuild, name: []const u8) ?*Build.Dependency {
        const b = self.build;
        const dep = b.lazyDependency(name, .{}) orelse return null;
        if (self.graph.dependencyLoaded(dep)) return dep;

        const build_runner = @import("root");
        const deps = build_runner.dependencies;
        const pkg_hash = findPkgHashOrFatal(b, name);

        inline for (@typeInfo(deps.packages).@"struct".decls) |decl| {
            if (std.mem.eql(u8, decl.name, pkg_hash)) {
                const allocator = self.build.allocator;
                const child = allocator.create(FimoBuild) catch @panic("oom");
                child.* = .{
                    .build = dep.builder,
                    .build_dep = self.build_dep,
                    .graph = self.graph,
                };

                const pkg = @field(deps.packages, decl.name);
                if (@hasDecl(pkg, "build_zig") and @hasDecl(pkg.build_zig, "configure"))
                    pkg.build_zig.configure(child);
                self.graph.markDependencyLoaded(dep);
                return dep;
            }
        }

        unreachable; // Bad @dependencies source
    }

    fn findPkgHashOrFatal(b: *Build, name: []const u8) []const u8 {
        for (b.available_deps) |dep| {
            if (std.mem.eql(u8, dep[0], name)) return dep[1];
        }

        const full_path = b.pathFromRoot("build.zig.zon");
        std.debug.panic("no dependency named '{s}' in '{s}'. All packages used in build.zig must be declared in this file", .{ name, full_path });
    }

    pub fn addPackage(self: *FimoBuild, options: CreatePackageOptions) *Package {
        const pkg = Package.create(self, options);
        return self.graph.addPackage(pkg);
    }

    pub fn getPackage(self: *FimoBuild, name: []const u8) *Package {
        return self.graph.getPackage(name) orelse std.debug.panic("package `{s}` not found", .{name});
    }

    pub fn getOptionalPackage(self: *FimoBuild, name: []const u8) ?*Package {
        return self.graph.getPackage(name);
    }

    pub fn addModule(self: *FimoBuild, options: CreateModuleOptions) *Module {
        const mod = Module.create(self, options);
        return self.graph.addModule(mod);
    }

    pub fn createModule(self: *FimoBuild, options: CreateModuleOptions) *Module {
        return Module.create(self, options);
    }

    pub fn getModule(self: *FimoBuild, name: []const u8) *Module {
        return self.graph.getModule(name) orelse std.debug.panic("module `{s}` not found", .{name});
    }

    pub fn getOptionalModule(self: *FimoBuild, name: []const u8) ?*Module {
        return self.graph.getModule(name);
    }
};
