const fimo_tasks_meta = @import("fimo_tasks_meta");
const meta_task = fimo_tasks_meta.task;
const MetaId = meta_task.Id;
const MetaTask = meta_task.OpaqueTask;

const context = @import("context.zig");
const Transfer = context.Transfer;
const Stack = context.Stack;

const Self = @This();

id: MetaId,
task: *MetaTask,
stack: Stack,
transfer: ?Transfer,
next: ?*Self = null,
