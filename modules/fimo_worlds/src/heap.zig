const std = @import("std");

pub const MultiGenerationAllocator = @import("heap/MultiGenerationAllocator.zig");
pub const SingleGenerationAllocator = @import("heap/SingleGenerationAllocator.zig");
pub const TracingAllocator = @import("heap/TracingAllocator.zig");

test {
    std.testing.refAllDeclsRecursive(SingleGenerationAllocator);
    std.testing.refAllDeclsRecursive(MultiGenerationAllocator);
    std.testing.refAllDeclsRecursive(TracingAllocator);
}
