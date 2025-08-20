const std = @import("std");

const build_internals = @import("tools/build-internals");

pub fn configure(builder: *build_internals.FimoBuild) void {
    const b = builder.build;
    const target = builder.graph.target;
    const optimize = builder.graph.optimize;
    const glfw_dep = b.dependency("glfw", .{});
    const fimo_std_pkg = builder.getPackage("fimo_std");

    const headers = b.addTranslateC(.{
        .target = target,
        .optimize = optimize,
        .root_source_file = b.path("headers.h"),
    });
    headers.defineCMacro("IMGUI_DISABLE_OBSOLETE_FUNCTIONS", "1");
    headers.defineCMacro("CIMGUI_DEFINE_ENUMS_AND_STRUCTS", "1");
    headers.defineCMacro("CIMGUI_USE_GLFW", "1");
    headers.defineCMacro("CIMGUI_USE_OPENGL3", "1");
    headers.addIncludePath(b.path("glad/include"));
    headers.addIncludePath(b.path("cimgui"));
    headers.addIncludePath(glfw_dep.path("include"));

    const profiler = builder.addExecutable(.{
        .name = "profiler",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/root.zig"),
            .target = target,
            .optimize = optimize,
            .link_libc = true,
            .link_libcpp = true,
        }),
    });
    profiler.root_module.addImport("fimo_std", fimo_std_pkg.root_module);
    profiler.root_module.addImport("headers", headers.createModule());
    profiler.root_module.addIncludePath(b.path("glad/include"));
    profiler.root_module.addIncludePath(glfw_dep.path("include"));
    profiler.root_module.addIncludePath(b.path("cimgui"));
    profiler.root_module.addIncludePath(b.path("cimgui/imgui"));
    profiler.root_module.addIncludePath(b.path("cimgui/imgui/backends"));
    profiler.root_module.addCSourceFiles(.{
        .root = b.path("glad"),
        .files = &.{"src/gl.c"},
    });
    profiler.root_module.addCSourceFiles(.{
        .root = glfw_dep.path("src"),
        .files = &glfw_base_sources,
    });
    profiler.root_module.addCSourceFiles(.{
        .root = b.path("cimgui"),
        .files = &imgui_srcs,
        .flags = &.{
            "-DCIMGUI_USE_GLFW",
            "-DCIMGUI_USE_OPENGL3",
            "-DIMGUI_IMPL_API=extern \"C\" ",
            "-DIMGUI_DISABLE_OBSOLETE_FUNCTIONS",
        },
        .language = .cpp,
    });
    switch (target.result.os.tag) {
        .windows => {
            profiler.root_module.linkSystemLibrary("gdi32", .{});
            profiler.root_module.linkSystemLibrary("user32", .{});
            profiler.root_module.linkSystemLibrary("shell32", .{});
            profiler.root_module.linkSystemLibrary("opengl32", .{});

            profiler.root_module.addCMacro("_GLFW_WIN32", "1");
            profiler.root_module.addCSourceFiles(.{
                .root = glfw_dep.path("src"),
                .files = &glfw_windows_sources,
            });
        },
        else => {},
    }

    const capture = builder.addExecutable(.{
        .name = "capture",
        .root_module = b.createModule(.{
            .target = target,
            .optimize = optimize,
            .root_source_file = b.path("capture.zig"),
        }),
    });
    capture.root_module.addImport("fimo_std", fimo_std_pkg.root_module);
}

pub fn build(b: *std.Build) void {
    _ = b;
}

const imgui_srcs = [_][]const u8{
    "imgui/backends/imgui_impl_glfw.cpp",
    "imgui/backends/imgui_impl_opengl3.cpp",
    "imgui/imgui.cpp",
    "imgui/imgui_demo.cpp",
    "imgui/imgui_draw.cpp",
    "imgui/imgui_tables.cpp",
    "imgui/imgui_widgets.cpp",
    "cimgui.cpp",
};

const glfw_base_sources = [_][]const u8{
    "context.c",
    "egl_context.c",
    "init.c",
    "input.c",
    "monitor.c",
    "null_init.c",
    "null_joystick.c",
    "null_monitor.c",
    "null_window.c",
    "osmesa_context.c",
    "platform.c",
    "vulkan.c",
    "window.c",
};

const glfw_linux_sources = [_][]const u8{
    "linux_joystick.c",
    "posix_module.c",
    "posix_poll.c",
    "posix_thread.c",
    "posix_time.c",
    "xkb_unicode.c",
};

const glfw_linux_wl_sources = [_][]const u8{
    "wl_init.c",
    "wl_monitor.c",
    "wl_window.c",
};

const glfw_linux_x11_sources = [_][]const u8{
    "glx_context.c",
    "x11_init.c",
    "x11_monitor.c",
    "x11_window.c",
};

const glfw_windows_sources = [_][]const u8{
    "wgl_context.c",
    "win32_init.c",
    "win32_joystick.c",
    "win32_module.c",
    "win32_monitor.c",
    "win32_thread.c",
    "win32_time.c",
    "win32_window.c",
};

const glfw_macos_sources = [_][]const u8{
    // C sources
    "cocoa_time.c",
    "posix_module.c",
    "posix_thread.c",

    // ObjC sources
    "cocoa_init.m",
    "cocoa_joystick.m",
    "cocoa_monitor.m",
    "cocoa_window.m",
    "nsgl_context.m",
};
