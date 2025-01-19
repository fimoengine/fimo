//! Cross-platform and language-agnostic interface of a module debug info.
const std = @import("std");

const c = @import("../../../c.zig");

data: ?*anyopaque,
vtable: *const VTable,

const Self = @This();

/// Symbol debug info.
pub const Symbol = extern struct {
    data: ?*anyopaque,
    vtable: *const Symbol.VTable,

    /// VTable of a `ValueSymbol`.
    pub const VTable = extern struct {
        ref: *const fn (data: ?*anyopaque) callconv(.c) void,
        unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        get_symbol_id: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_type_id: *const fn (data: ?*anyopaque, id: *usize) callconv(.c) bool,
        get_table_index: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_declaration_index: *const fn (data: ?*anyopaque) callconv(.c) usize,
        is_import: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_export: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_static_export: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_dynamic_export: *const fn (data: ?*anyopaque) callconv(.c) bool,
    };

    /// Increases the reference count of the `Symbol` by one.
    pub fn ref(self: Symbol) void {
        self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the `Symbol` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: Symbol) void {
        self.vtable.unref(self.data);
    }

    /// Fetches the unique id of the symbol.
    ///
    /// The id is unique for the current module.
    pub fn getSymbolId(self: Symbol) usize {
        return self.vtable.get_symbol_id(self.data);
    }

    /// Fetches the unique id of the symbol type.
    ///
    /// The id is unique for the current module.
    pub fn getTypeId(self: Symbol) ?usize {
        var id: usize = undefined;
        return if (self.vtable.get_type_id(self.data, &id)) id else null;
    }

    /// Fetches the index of the symbol in the module import or export table.
    pub fn getTableIndex(self: Symbol) usize {
        return self.vtable.get_table_index(self.data);
    }

    /// Fetches the index in the respective `Export` array.
    pub fn getDeclarationIndex(self: Symbol) usize {
        return self.vtable.get_declaration_index(self.data);
    }

    /// Checks whether the symbol is an import.
    pub fn isImport(self: Symbol) bool {
        return self.vtable.is_import(self.data);
    }

    /// Checks whether the symbol is an export.
    pub fn isExport(self: Symbol) bool {
        return self.vtable.is_export(self.data);
    }

    /// Checks whether the symbol is a static export.
    pub fn isStaticExport(self: Symbol) bool {
        return self.vtable.is_static_export(self.data);
    }

    /// Checks whether the symbol is a dynamic export.
    pub fn isDynamicExport(self: Symbol) bool {
        return self.vtable.is_dynamic_export(self.data);
    }
};

/// Tag of a type.
pub const TypeTag = enum(i32) {
    void = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_VOID,
    bool = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_BOOL,
    int = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_INT,
    float = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_FLOAT,
    pointer = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_POINTER,
    array = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_ARRAY,
    @"struct" = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_STRUCT,
    @"enum" = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_ENUM,
    @"union" = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_UNION,
    @"fn" = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_FN,
    @"opaque" = c.FIMO_MODULE_DEBUG_INFO_TYPE_TAG_OPAQUE,
    _,
};

/// A type representing nothing.
pub const VoidType = extern struct {
    data: ?*anyopaque,
    vtable: *const VoidType.VTable,

    /// VTable of a `VoidType`.
    pub const VTable = extern struct {
        base: Type.VTable,
    };

    /// Increases the reference count of the `VoidType` by one.
    pub fn ref(self: VoidType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `VoidType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: VoidType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: VoidType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: VoidType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }
};

/// A type representing a boolean.
pub const BoolType = extern struct {
    data: ?*anyopaque,
    vtable: *const BoolType.VTable,

    /// VTable of a `BoolType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
    };

    /// Increases the reference count of the `BoolType` by one.
    pub fn ref(self: BoolType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `BoolType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: BoolType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: BoolType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: BoolType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: BoolType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: BoolType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: BoolType) u8 {
        return self.vtable.get_alignment(self.data);
    }
};

/// A type representing an integer.
pub const IntType = extern struct {
    data: ?*anyopaque,
    vtable: *const IntType.VTable,

    /// VTable of a `IntType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        is_unsigned: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_signed: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_bitwidth: *const fn (data: ?*anyopaque) callconv(.c) u16,
    };

    /// Increases the reference count of the `IntType` by one.
    pub fn ref(self: IntType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `IntType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: IntType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: IntType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: IntType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: IntType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: IntType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: IntType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches whether the integer type is unsigned.
    pub fn isUnsigned(self: IntType) bool {
        return self.vtable.is_unsigned(self.data);
    }

    /// Fetches whether the integer type is signed.
    pub fn isSigned(self: IntType) bool {
        return self.vtable.is_signed(self.data);
    }

    /// Fetches the width of the integer in bits.
    pub fn getBitwidth(self: IntType) u16 {
        return self.vtable.get_bitwidth(self.data);
    }
};

/// A type representing a floating point.
pub const FloatType = extern struct {
    data: ?*anyopaque,
    vtable: *const FloatType.VTable,

    /// VTable of a `FloatType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_bitwidth: *const fn (data: ?*anyopaque) callconv(.c) u16,
    };

    /// Increases the reference count of the `FloatType` by one.
    pub fn ref(self: FloatType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `FloatType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: FloatType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: FloatType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: FloatType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: FloatType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: FloatType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: FloatType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches the width of the float in bits.
    pub fn getBitwidth(self: FloatType) u16 {
        return self.vtable.get_bitwidth(self.data);
    }
};

/// A type representing a pointer.
pub const PointerType = extern struct {
    data: ?*anyopaque,
    vtable: *const PointerType.VTable,

    /// VTable of a `FloatType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_pointee_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        is_const: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_volatile: *const fn (data: ?*anyopaque) callconv(.c) bool,
        is_nonzero: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_child_id: *const fn (data: ?*anyopaque) callconv(.c) usize,
    };

    /// Increases the reference count of the `PointerType` by one.
    pub fn ref(self: PointerType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `PointerType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: PointerType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: PointerType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: PointerType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: PointerType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: PointerType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: PointerType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches the log of the alignment of the pointee.
    pub fn getPointeeAlignment(self: PointerType) u8 {
        return self.vtable.get_pointee_alignment(self.data);
    }

    /// Fetches whether the pointee is constant.
    pub fn isConst(self: PointerType) bool {
        return self.vtable.is_const(self.data);
    }

    /// Fetches whether the pointee is volatile.
    pub fn isVolatile(self: PointerType) bool {
        return self.vtable.is_volatile(self.data);
    }

    /// Fetches whether the pointer may not be null.
    pub fn isNonzero(self: PointerType) bool {
        return self.vtable.is_nonzero(self.data);
    }

    /// Fetches the type of the pointee.
    pub fn getChildId(self: PointerType) usize {
        return self.vtable.get_child_id(self.data);
    }
};

/// A type representing an array.
pub const ArrayType = extern struct {
    data: ?*anyopaque,
    vtable: *const ArrayType.VTable,

    /// VTable of a `ArrayType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_length: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_child_id: *const fn (data: ?*anyopaque) callconv(.c) usize,
    };

    /// Increases the reference count of the `ArrayType` by one.
    pub fn ref(self: ArrayType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `ArrayType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: ArrayType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: ArrayType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: ArrayType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: ArrayType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: ArrayType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: ArrayType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches the length of the array.
    pub fn getLength(self: ArrayType) usize {
        return self.vtable.get_length(self.data);
    }

    /// Fetches the element type.
    pub fn getChildId(self: ArrayType) usize {
        return self.vtable.get_child_id(self.data);
    }
};

/// A type representing a struct.
pub const StructType = extern struct {
    data: ?*anyopaque,
    vtable: *const StructType.VTable,

    /// VTable of a `StructType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        is_packed_layout: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_field_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_field_name: *const fn (
            data: ?*anyopaque,
            index: usize,
            name: *[*:0]const u8,
        ) callconv(.c) bool,
        get_field_type_id: *const fn (
            data: ?*anyopaque,
            index: usize,
            id: *usize,
        ) callconv(.c) bool,
        get_field_offset: *const fn (
            data: ?*anyopaque,
            index: usize,
            offset: *usize,
        ) callconv(.c) bool,
        get_field_bit_offset: *const fn (
            data: ?*anyopaque,
            index: usize,
            offset: *u8,
        ) callconv(.c) bool,
        get_field_alignment: *const fn (
            data: ?*anyopaque,
            index: usize,
            alignment: *u8,
        ) callconv(.c) bool,
    };

    /// Increases the reference count of the `StructType` by one.
    pub fn ref(self: StructType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `StructType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: StructType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: StructType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: StructType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: StructType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: StructType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: StructType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Checks whether the structure includes any padding bytes.
    pub fn isPackedLayout(self: StructType) bool {
        return self.vtable.is_packed_layout(self.data);
    }

    /// Fetches the number of fields in the structure.
    pub fn getFieldCount(self: StructType) usize {
        return self.vtable.get_field_count(self.data);
    }

    /// Fetches the name of the field at the index.
    pub fn getFieldName(self: StructType, index: usize) ?[:0]const u8 {
        var name: [*:0]const u8 = undefined;
        return if (self.vtable.get_field_name(self.data, index, &name))
            std.mem.span(name)
        else
            null;
    }

    /// Fetches the type of the field at the index.
    pub fn getFieldTypeId(self: StructType, index: usize) ?usize {
        var id: usize = undefined;
        return if (self.vtable.get_field_type_id(self.data, index, &id)) id else null;
    }

    /// Fetches the byte offset to the field.
    pub fn getFieldOffset(self: StructType, index: usize) ?usize {
        var offset: usize = undefined;
        return if (self.vtable.get_field_offset(self.data, index, &offset))
            offset
        else
            null;
    }

    /// Fetches the sub-byte offset to the field.
    pub fn getFieldBitOffset(self: StructType, index: usize) ?u3 {
        var offset: u8 = undefined;
        return if (self.vtable.get_field_bit_offset(self.data, index, &offset))
            @truncate(offset)
        else
            null;
    }

    /// Fetches the log alignment of the field at the index.
    pub fn getFieldAlignment(self: StructType, index: usize) ?u8 {
        var al: u8 = undefined;
        return if (self.vtable.get_field_alignment(self.data, index, &al)) al else null;
    }
};

/// A type representing an enumeration.
pub const EnumType = extern struct {
    data: ?*anyopaque,
    vtable: *const EnumType.VTable,

    /// VTable of a `EnumType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_tag_id: *const fn (data: ?*anyopaque) callconv(.c) usize,
    };

    /// Increases the reference count of the `EnumType` by one.
    pub fn ref(self: EnumType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `EnumType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: EnumType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: EnumType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: EnumType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: EnumType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: EnumType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: EnumType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches the type of the tag.
    pub fn getTagId(self: EnumType) usize {
        return self.vtable.get_tag_id(self.data);
    }
};

/// A type representing an union.
pub const UnionType = extern struct {
    data: ?*anyopaque,
    vtable: *const UnionType.VTable,

    /// VTable of a `UnionType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        get_size: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_bit_size: *const fn (data: ?*anyopaque) callconv(.c) u8,
        get_alignment: *const fn (data: ?*anyopaque) callconv(.c) u8,
        is_packed_layout: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_field_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_field_name: *const fn (
            data: ?*anyopaque,
            index: usize,
            name: *[*:0]const u8,
        ) callconv(.c) bool,
        get_field_type_id: *const fn (
            data: ?*anyopaque,
            index: usize,
            id: *usize,
        ) callconv(.c) bool,
        get_field_alignment: *const fn (
            data: ?*anyopaque,
            index: usize,
            alignment: *u8,
        ) callconv(.c) bool,
    };

    /// Increases the reference count of the `UnionType` by one.
    pub fn ref(self: UnionType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `UnionType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: UnionType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: UnionType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: UnionType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Fetches the size of the type in full bytes.
    pub fn getSize(self: UnionType) usize {
        return self.vtable.get_size(self.data);
    }

    /// Fetches the sub-byte size of the type.
    pub fn getBitSize(self: UnionType) u3 {
        return @truncate(self.vtable.get_size(self.data));
    }

    /// Fetches the log of the type alignment.
    pub fn getAlignment(self: UnionType) u8 {
        return self.vtable.get_alignment(self.data);
    }

    /// Fetches whether the union includes any padding bytes.
    pub fn isPackedLayout(self: UnionType) bool {
        return self.vtable.is_packed_layout(self.data);
    }

    /// Fetches the number of fields in the union.
    pub fn getFieldCount(self: UnionType) usize {
        return self.vtable.get_field_count(self.data);
    }

    /// Fetches the name of the field at the index.
    pub fn getFieldName(self: UnionType, index: usize) ?[:0]const u8 {
        var name: [*:0]const u8 = undefined;
        return if (self.vtable.get_field_name(self.data, index, &name))
            std.mem.span(name)
        else
            null;
    }

    /// Fetches the type of the field at the index.
    pub fn getFieldTypeId(self: UnionType, index: usize) ?usize {
        var id: usize = undefined;
        return if (self.vtable.get_field_type_id(self.data, index, &id)) id else null;
    }

    /// Fetches the log alignment of the field at the index.
    pub fn getFieldAlignment(self: UnionType, index: usize) ?u8 {
        var al: u8 = undefined;
        return if (self.vtable.get_field_alignment(self.data, index, &al)) al else null;
    }
};

/// A type representing a function.
pub const FnType = extern struct {
    data: ?*anyopaque,
    vtable: *const FnType.VTable,

    /// Recognized calling conventions.
    pub const CallingConvention = enum(i32) {
        x86_64_sysv = c.FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_X86_64_SYSV,
        x86_64_win = c.FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_X86_64_WIN,
        aarch64_aapcs = c.FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS,
        aarch64_aapcs_darwin = c.FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_DARWIN,
        aarch64_aapcs_win = c.FIMO_MODULE_DEBUG_INFO_CALLING_CONVENTION_AARCH64_AAPCS_WIN,
        _,
    };

    /// VTable of a `FnType`.
    pub const VTable = extern struct {
        base: Type.VTable,
        is_default_calling_convention: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_calling_convention: *const fn (data: ?*anyopaque, cc: *CallingConvention) callconv(.c) bool,
        get_stack_alignment: *const fn (data: ?*anyopaque, al: *u8) callconv(.c) bool,
        is_var_args: *const fn (data: ?*anyopaque) callconv(.c) bool,
        get_return_type_id: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_parameter_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
        get_parameter_type_id: *const fn (
            data: ?*anyopaque,
            index: usize,
            id: *usize,
        ) callconv(.c) bool,
    };

    /// Increases the reference count of the `FnType` by one.
    pub fn ref(self: FnType) void {
        self.vtable.base.ref(self.data);
    }

    /// Decreases the reference count of the `FnType` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: FnType) void {
        self.vtable.base.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: FnType) TypeTag {
        return self.vtable.base.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: FnType) [:0]const u8 {
        return std.mem.span(self.vtable.base.get_name(self.data));
    }

    /// Checks whether the calling convention of the function is the
    /// default for the C Abi of the target.
    pub fn isDefaultCallingConvention(self: FnType) bool {
        return self.vtable.is_default_calling_convention(self.data);
    }

    /// Fetches the calling convention of the function.
    pub fn getCallingConvention(self: FnType) ?CallingConvention {
        var cc: CallingConvention = undefined;
        return if (self.vtable.get_calling_convention(self.data, &cc)) cc else null;
    }

    /// Fetches the alignment of the stack.
    pub fn getStackAlignment(self: FnType) ?u8 {
        var al: u8 = undefined;
        return if (self.vtable.get_stack_alignment(self.data, &al)) al else null;
    }

    /// Checks whether the function supports a variable number of arguments.
    pub fn isVarArgs(self: FnType) bool {
        return self.vtable.is_var_args(self.data);
    }

    /// Fetches the type id of the return value.
    pub fn getReturnTypeId(self: FnType) usize {
        return self.vtable.get_return_type_id(self.data);
    }

    /// Fetches the number of parameters.
    pub fn getParameterCount(self: FnType) usize {
        return self.vtable.get_parameter_count(self.data);
    }

    /// Fetches the type id of the parameter.
    pub fn getParameterTypeId(self: FnType, index: usize) ?usize {
        var id: usize = undefined;
        return if (self.vtable.get_parameter_type_id(self.data, index, &id))
            id
        else
            null;
    }
};

/// An opaque type.
pub const Type = extern struct {
    data: ?*anyopaque,
    vtable: *const Type.VTable,

    /// VTable of a `Type`.
    pub const VTable = extern struct {
        ref: *const fn (data: ?*anyopaque) callconv(.c) void,
        unref: *const fn (data: ?*anyopaque) callconv(.c) void,
        get_type_tag: *const fn (data: ?*anyopaque) callconv(.c) TypeTag,
        get_name: *const fn (data: ?*anyopaque) callconv(.c) [*:0]const u8,
        next: ?*const anyopaque = null,
    };

    /// Increases the reference count of the `Type` by one.
    pub fn ref(self: Type) void {
        self.vtable.ref(self.data);
    }

    /// Decreases the reference count of the `Type` by one.
    ///
    /// The instance is freed if the reference count reaches zero.
    pub fn unref(self: Type) void {
        self.vtable.unref(self.data);
    }

    /// Fetches the tag of the type.
    pub fn getTypeTag(self: Type) TypeTag {
        return self.vtable.get_type_tag(self.data);
    }

    /// Fetches the name of the type.
    pub fn getName(self: Type) [:0]const u8 {
        return std.mem.span(self.vtable.get_name(self.data));
    }

    /// Increases the reference count by one and casts the `Type` to a `VoidType`.
    pub fn getVoid(self: Type) VoidType {
        std.debug.assert(self.getTypeTag() == .void);
        self.ref();
        return VoidType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `BoolType`.
    pub fn getBool(self: Type) BoolType {
        std.debug.assert(self.getTypeTag() == .bool);
        self.ref();
        return BoolType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `IntType`.
    pub fn getInt(self: Type) IntType {
        std.debug.assert(self.getTypeTag() == .int);
        self.ref();
        return IntType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `FloatType`.
    pub fn getFloat(self: Type) FloatType {
        std.debug.assert(self.getTypeTag() == .float);
        self.ref();
        return FloatType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `PointerType`.
    pub fn getPointer(self: Type) PointerType {
        std.debug.assert(self.getTypeTag() == .pointer);
        self.ref();
        return PointerType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `ArrayType`.
    pub fn getArray(self: Type) ArrayType {
        std.debug.assert(self.getTypeTag() == .array);
        self.ref();
        return ArrayType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `StructType`.
    pub fn getStruct(self: Type) StructType {
        std.debug.assert(self.getTypeTag() == .@"struct");
        self.ref();
        return StructType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `EnumType`.
    pub fn getEnum(self: Type) EnumType {
        std.debug.assert(self.getTypeTag() == .@"enum");
        self.ref();
        return EnumType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `UnionType`.
    pub fn getUnion(self: Type) UnionType {
        std.debug.assert(self.getTypeTag() == .@"union");
        self.ref();
        return UnionType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }

    /// Increases the reference count by one and casts the `Type` to a `FnType`.
    pub fn getFn(self: Type) FnType {
        std.debug.assert(self.getTypeTag() == .@"fn");
        self.ref();
        return FnType{
            .data = self.data,
            .vtable = @alignCast(@ptrCast(self.vtable)),
        };
    }
};

/// VTable of a `DebugInfo`.
pub const VTable = extern struct {
    ref: *const fn (data: ?*anyopaque) callconv(.c) void,
    unref: *const fn (data: ?*anyopaque) callconv(.c) void,
    get_symbol_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_import_symbol_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_export_symbol_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_static_export_symbol_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_dynamic_export_symbol_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_symbol_id_by_import_index: *const fn (
        data: ?*anyopaque,
        index: usize,
        id: *usize,
    ) callconv(.c) bool,
    get_symbol_id_by_export_index: *const fn (
        data: ?*anyopaque,
        index: usize,
        id: *usize,
    ) callconv(.c) bool,
    get_symbol_id_by_static_export_index: *const fn (
        data: ?*anyopaque,
        index: usize,
        id: *usize,
    ) callconv(.c) bool,
    get_symbol_id_by_dynamic_export_index: *const fn (
        data: ?*anyopaque,
        index: usize,
        id: *usize,
    ) callconv(.c) bool,
    get_symbol_by_id: *const fn (
        data: ?*anyopaque,
        id: usize,
        symbol: *Symbol,
    ) callconv(.c) bool,
    get_type_count: *const fn (data: ?*anyopaque) callconv(.c) usize,
    get_type_by_id: *const fn (
        data: ?*anyopaque,
        id: usize,
        @"type": *Type,
    ) callconv(.c) bool,
};

/// Increases the reference count of the `DebugInfo` by one.
pub fn ref(self: Self) void {
    self.vtable.ref(self.data);
}

/// Decreases the reference count of the `DebugInfo` by one.
///
/// The instance is freed if the reference count reaches zero.
pub fn unref(self: Self) void {
    self.vtable.unref(self.data);
}

/// Fetches the number of symbols contained in the `DebugInfo`.
pub fn getSymbolCount(self: Self) usize {
    return self.vtable.get_symbol_count(self.data);
}

/// Fetches the number of imported symbols contained in the `DebugInfo`.
pub fn getImportSymbolCount(self: Self) usize {
    return self.vtable.get_import_symbol_count(self.data);
}

/// Fetches the number of exported symbols contained in the `DebugInfo`.
pub fn getExportSymbolCount(self: Self) usize {
    return self.vtable.get_export_symbol_count(self.data);
}

/// Fetches the number of exported static symbols contained in the `DebugInfo`.
pub fn getStaticExportSymbolCount(self: Self) usize {
    return self.vtable.get_static_export_symbol_count(self.data);
}

/// Fetches the number of exported dynamic symbols contained in the `DebugInfo`.
pub fn getDynamicExportSymbolCount(self: Self) usize {
    return self.vtable.get_dynamic_export_symbol_count(self.data);
}

/// Fetches the symbol id for the symbol at the index of the import table.
pub fn getSymbolIdByImportIndex(self: Self, index: usize) ?usize {
    var id: usize = undefined;
    return if (self.vtable.get_symbol_id_by_import_index(
        self.data,
        index,
        &id,
    )) id else null;
}

/// Fetches the symbol id for the symbol at the index of the export table.
pub fn getSymbolIdByExportIndex(self: Self, index: usize) ?usize {
    var id: usize = undefined;
    return if (self.vtable.get_symbol_id_by_export_index(
        self.data,
        index,
        &id,
    )) id else null;
}

/// Fetches the symbol id for the symbol at the index of the static export list.
pub fn getSymbolIdByStaticExportIndex(self: Self, index: usize) ?usize {
    var id: usize = undefined;
    return if (self.vtable.get_symbol_id_by_static_export_index(
        self.data,
        index,
        &id,
    )) id else null;
}

/// Fetches the symbol id for the symbol at the index of the dynamic export list.
pub fn getSymbolIdByDynamicExportIndex(self: Self, index: usize) ?usize {
    var id: usize = undefined;
    return if (self.vtable.get_symbol_id_by_dynamic_export_index(
        self.data,
        index,
        &id,
    )) id else null;
}

/// Fetches the symbol with the given id.
pub fn getSymbolById(self: Self, id: usize) ?Symbol {
    var symbol: Symbol = undefined;
    return if (self.vtable.get_symbol_by_id(self.data, id, &symbol)) symbol else null;
}

/// Fetches the number of types contained in the `DebugInfo`.
pub fn getTypeCount(self: Self) usize {
    return self.vtable.get_type_count(self.data);
}

/// Fetches the type with the given id.
pub fn getTypeById(self: Self, id: usize) ?Type {
    var @"type": Type = undefined;
    return if (self.vtable.get_type_by_id(self.data, id, &@"type")) @"type" else null;
}

/// Implementation of the debug info using comptime reflection.
pub const Builder = struct {
    imports: []const Builder.Symbol = &.{},
    exports: []const Builder.Symbol = &.{},
    dynamic_exports: []const Builder.Symbol = &.{},
    types: []const Builder.Type = &.{},

    const Symbol = struct {
        id: usize,
        type_id: usize,
    };

    const Type = struct {
        type: type,
        info: Impl.Type,
    };

    /// Registers a type as an import export.
    pub fn addImport(comptime self: *Builder, comptime T: type) void {
        const type_id = self.addType(T);

        var symbols_tmp: [self.imports.len + 1]Builder.Symbol = undefined;
        @memcpy(symbols_tmp[0..self.imports.len], self.imports);
        symbols_tmp[self.imports.len] = .{
            .id = self.imports.len,
            .type_id = type_id,
        };

        const symbols: [symbols_tmp.len]Builder.Symbol = symbols_tmp;
        self.imports = &symbols;
    }

    /// Registers a type as a static export.
    pub fn addExport(comptime self: *Builder, comptime T: type) void {
        const type_id = self.addType(T);

        var symbols_tmp: [self.exports.len + 1]Builder.Symbol = undefined;
        @memcpy(symbols_tmp[0..self.exports.len], self.exports);
        symbols_tmp[self.exports.len] = .{
            .id = self.exports.len,
            .type_id = type_id,
        };

        const symbols: [symbols_tmp.len]Builder.Symbol = symbols_tmp;
        self.exports = &symbols;
    }

    /// Registers a type as a dynamic export.
    pub fn addDynamicExport(comptime self: *Builder, comptime T: type) void {
        const type_id = self.addType(T);

        var symbols_tmp: [self.dynamic_exports.len + 1]Builder.Symbol = undefined;
        @memcpy(symbols_tmp[0..self.dynamic_exports.len], self.dynamic_exports);
        symbols_tmp[self.dynamic_exports.len] = .{
            .id = self.dynamic_exports.len,
            .type_id = type_id,
        };

        const symbols: [symbols_tmp.len]Builder.Symbol = symbols_tmp;
        self.dynamic_exports = &symbols;
    }

    /// Adds a type to the debug info.
    ///
    /// Duplicate types are merged. If a type does not have a guaranteed memory layout,
    /// it will be registered as an opaque type. An exception to this are function types,
    /// where the debug info stores the signature of the functions, irrespective of it being
    /// callable with a known abi.
    pub fn addType(comptime self: *@This(), comptime T: type) usize {
        for (self.types) |ty| {
            if (ty.type == T) return ty.info.id;
        }

        // Reserve a slot for the current type, such that cycles don't end up
        // recursing endlessly.
        var types_tmp: [self.types.len + 1]Builder.Type = undefined;
        @memcpy(types_tmp[0..self.types.len], self.types);
        const t_id = self.types.len;
        types_tmp[t_id] = .{
            .type = T,
            .info = .{
                .id = t_id,
                .name = @typeName(T),
                .data = undefined,
            },
        };
        self.types = &types_tmp;

        var t: Builder.Type = undefined;
        t.type = T;
        t.info.id = t_id;
        t.info.name = @typeName(T);
        t.info.data = switch (@typeInfo(T)) {
            .void, .noreturn => .{ .void = .{} },
            .bool => .{ .bool = .{} },
            .int => |v| blk: {
                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));

                break :blk .{
                    .int = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .bits = v.bits,
                        .signedness = v.signedness,
                    },
                };
            },
            .float => |v| blk: {
                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));
                const bits = v.bits;

                break :blk .{
                    .float = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .bits = bits,
                    },
                };
            },
            .pointer => |v| blk: {
                if (v.size == .slice) break :blk .{ .@"opaque" = .{} };
                if (v.address_space != .generic) break :blk .{ .@"opaque" = .{} };

                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));
                const pointee_alignment: u8 = @intCast(std.math.log2_int(u16, v.alignment));
                const is_const = v.is_const;
                const is_volatile = v.is_volatile;
                const is_allowzero = v.is_allowzero;
                const child_id = self.addType(v.child);

                break :blk .{
                    .pointer = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .pointee_alignment = pointee_alignment,
                        .is_const = is_const,
                        .is_volatile = is_volatile,
                        .is_allowzero = is_allowzero,
                        .child_id = child_id,
                    },
                };
            },
            .array => |v| blk: {
                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));
                const length: usize = v.len;
                const child_id = self.addType(v.child);

                if (self.types[child_id].info.data == .@"opaque") break :blk .{ .@"opaque" = .{} };

                break :blk .{
                    .array = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .length = length,
                        .child_id = child_id,
                    },
                };
            },
            .@"struct" => |v| blk: {
                if (v.layout != .@"extern" and v.layout != .@"packed") break :blk .{ .@"opaque" = .{} };
                if (v.is_tuple) break :blk .{ .@"opaque" = .{} };

                var fields_tmp: [v.fields.len]Impl.Type.StructField = undefined;
                for (v.fields, &fields_tmp) |src, *dst| {
                    const type_id = self.addType(src.type);
                    if (self.types[type_id].info.data == .@"opaque") break :blk .{ .@"opaque" = .{} };

                    dst.name = src.name;
                    dst.type_id = type_id;
                    dst.offset = @offsetOf(T, src.name);
                    dst.bit_offset = @truncate(@bitOffsetOf(T, src.name) % 8);
                    dst.alignment = @intCast(std.math.log2_int(u16, src.alignment));
                }

                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));
                const layout = v.layout;
                const fields: [v.fields.len]Impl.Type.StructField = fields_tmp;

                break :blk .{
                    .@"struct" = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .layout = layout,
                        .fields = &fields,
                    },
                };
            },
            .optional => |v| blk: {
                const child_info = @typeInfo(v.child);
                if (child_info != .pointer) break :blk .{ .@"opaque" = .{} };

                const child_id = self.addType(v.child);
                const child = self.types[child_id].info;
                if (child.data != .pointer) break :blk .{ .@"opaque" = .{} };

                const pointer = child.data.pointer;
                if (pointer.is_allowzero) break :blk .{ .@"opaque" = .{} };

                break :blk .{
                    .pointer = .{
                        .size = pointer.size,
                        .bitsize = pointer.bitsize,
                        .alignment = pointer.alignment,
                        .pointee_alignment = pointer.pointee_alignment,
                        .is_const = pointer.is_const,
                        .is_volatile = pointer.is_volatile,
                        .is_allowzero = true,
                        .child_id = pointer.child_id,
                    },
                };
            },
            .@"enum" => |v| blk: {
                const tag_id = self.addType(v.tag_type);
                const tag = self.types[tag_id].info;
                if (tag.data != .int) break :blk .{ .@"opaque" = .{} };

                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));

                break :blk .{
                    .@"enum" = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .tag_id = tag_id,
                    },
                };
            },
            .@"union" => |v| blk: {
                if (v.layout != .@"extern" and v.layout != .@"packed") break :blk .{ .@"opaque" = .{} };
                if (v.tag_type != null) break :blk .{ .@"opaque" = .{} };

                var fields_tmp: [v.fields.len]Impl.Type.UnionField = undefined;
                for (v.fields, &fields_tmp) |src, *dst| {
                    const type_id = self.addType(src.type);
                    if (self.types[type_id].info.data == .@"opaque") break :blk .{ .@"opaque" = .{} };

                    dst.name = src.name;
                    dst.type_id = type_id;
                    dst.alignment = @intCast(std.math.log2_int(u16, src.alignment));
                }

                const size: usize = @sizeOf(T);
                const bitsize: u3 = @bitSizeOf(T) % 8;
                const alignment: u8 = @intCast(std.math.log2_int(u16, @alignOf(T)));
                const layout = v.layout;
                const fields: [v.fields.len]Impl.Type.UnionField = fields_tmp;

                break :blk .{
                    .@"union" = .{
                        .size = size,
                        .bitsize = bitsize,
                        .alignment = alignment,
                        .layout = layout,
                        .fields = &fields,
                    },
                };
            },
            .@"fn" => |v| blk: {
                var parameters_tmp: [v.params.len]Impl.Type.FnParameter = undefined;
                for (v.params, &parameters_tmp) |src, *dst| {
                    dst.type_id = if (src.type) |ty| self.addType(ty) else null;
                }

                const calling_convention = v.calling_convention;
                const is_var_args = v.is_var_args;
                const ret_type_id = self.addType(v.return_type orelse void);
                const parameters: [v.params.len]Impl.Type.FnParameter = parameters_tmp;

                break :blk .{
                    .@"fn" = .{
                        .calling_convention = calling_convention,
                        .is_var_args = is_var_args,
                        .return_type_id = ret_type_id,
                        .parameters = &parameters,
                    },
                };
            },
            else => .{ .@"opaque" = .{} },
        };

        var types: [self.types.len]Builder.Type = undefined;
        @memcpy(types[0..], self.types);
        types[t_id] = t;

        const types_c: [self.types.len]Builder.Type = types;
        self.types = &types_c;

        return t_id;
    }

    pub fn build(comptime self: *const @This()) Impl {
        const import_count = self.imports.len;
        const static_export_count = self.exports.len;
        const dynamic_export_count = self.dynamic_exports.len;
        const symbol_count = import_count + static_export_count + dynamic_export_count;
        const symbols: [symbol_count]Impl.Symbol = blk: {
            var x: [symbol_count]Impl.Symbol = undefined;
            for (self.imports, 0..) |sym, i| {
                x[sym.id] = .{
                    .id = sym.id,
                    .type_id = sym.type_id,
                    .table_index = i,
                    .decl_index = i,
                    .type = .import,
                };
            }
            for (self.exports, 0..) |sym, i| {
                x[sym.id] = .{
                    .id = sym.id,
                    .type_id = sym.type_id,
                    .table_index = i,
                    .decl_index = i,
                    .type = .static_export,
                };
            }
            for (self.exports, 0..) |sym, i| {
                x[sym.id] = .{
                    .id = sym.id,
                    .type_id = sym.type_id,
                    .table_index = static_export_count + i,
                    .decl_index = i,
                    .type = .dynamic_export,
                };
            }
            break :blk x;
        };
        const types: [self.types.len]Impl.Type = blk: {
            var x: [self.types.len]Impl.Type = undefined;
            for (self.types, &x) |src, *dst| dst.* = src.info;
            break :blk x;
        };

        return .{
            .import_count = import_count,
            .static_export_count = static_export_count,
            .dynamic_export_count = dynamic_export_count,
            .symbols = &symbols,
            .types = &types,
        };
    }
};

/// Implementation of the debug info constructed with the builder.
pub const Impl = struct {
    import_count: usize,
    static_export_count: usize,
    dynamic_export_count: usize,
    symbols: []const Impl.Symbol,
    types: []const Impl.Type,

    const Symbol = struct {
        id: usize,
        type_id: usize,
        table_index: usize,
        decl_index: usize,
        type: enum { import, static_export, dynamic_export },

        fn ref(self: ?*anyopaque) callconv(.c) void {
            _ = self;
        }

        fn unref(self: ?*anyopaque) callconv(.c) void {
            _ = self;
        }

        fn getSymbolId(self: ?*anyopaque) callconv(.c) usize {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.id;
        }

        fn getTypeId(self: ?*anyopaque, id: *usize) callconv(.c) bool {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            id.* = sel.type_id;
            return true;
        }

        fn getTableIndex(self: ?*anyopaque) callconv(.c) usize {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.table_index;
        }

        fn getDeclarationIndex(self: ?*anyopaque) callconv(.c) usize {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.decl_index;
        }

        fn isImport(self: ?*anyopaque) callconv(.c) bool {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.type == .import;
        }

        fn isExport(self: ?*anyopaque) callconv(.c) bool {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.type == .static_export or sel.type == .dynamic_export;
        }

        fn isStaticExport(self: ?*anyopaque) callconv(.c) bool {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.type == .static_export;
        }

        fn isDynamicExport(self: ?*anyopaque) callconv(.c) bool {
            const sel: *const Impl.Symbol = @constCast(@alignCast(@ptrCast(self)));
            return sel.type == .dynamic_export;
        }

        fn asFfi(self: *const Impl.Symbol) Self.Symbol {
            const vtable = Self.Symbol.VTable{
                .ref = &Impl.Symbol.ref,
                .unref = &Impl.Symbol.unref,
                .get_symbol_id = &Impl.Symbol.getSymbolId,
                .get_type_id = &Impl.Symbol.getTypeId,
                .get_table_index = &Impl.Symbol.getTableIndex,
                .get_declaration_index = &Impl.Symbol.getDeclarationIndex,
                .is_import = &Impl.Symbol.isImport,
                .is_export = &Impl.Symbol.isExport,
                .is_static_export = &Impl.Symbol.isStaticExport,
                .is_dynamic_export = &Impl.Symbol.isDynamicExport,
            };

            return .{
                .data = @constCast(self),
                .vtable = @ptrCast(&vtable),
            };
        }
    };

    const Type = struct {
        id: usize,
        name: [:0]const u8,
        data: union(enum) {
            void: VoidData,
            bool: BoolData,
            int: IntData,
            float: FloatData,
            pointer: PointerData,
            array: ArrayData,
            @"struct": StructData,
            @"enum": EnumData,
            @"union": UnionData,
            @"fn": FnData,
            @"opaque": OpaqueData,
        },

        const VoidData = struct {
            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.VoidType.VTable{
                    .base = OpaqueData.vtable,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const BoolData = struct {
            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                _ = self;
                return @sizeOf(bool);
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                _ = self;
                return @bitSizeOf(bool);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                _ = self;
                return @alignOf(bool);
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.BoolType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &BoolData.getSize,
                    .get_bit_size = &BoolData.getBitSize,
                    .get_alignment = &BoolData.getAlignment,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const IntData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            bits: u16,
            signedness: std.builtin.Signedness,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.int.size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.int.bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.int.alignment;
            }

            fn isUnsigned(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.int.signedness == .unsigned;
            }

            fn isSigned(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.int.signedness == .signed;
            }

            fn getBitwidth(self: ?*anyopaque) callconv(.c) u16 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.int.bits;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.IntType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &IntData.getSize,
                    .get_bit_size = &IntData.getBitSize,
                    .get_alignment = &IntData.getAlignment,
                    .is_unsigned = &IntData.isUnsigned,
                    .is_signed = &IntData.isSigned,
                    .get_bitwidth = &IntData.getBitwidth,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const FloatData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            bits: u16,

            const ref = OpaqueData.ref;
            const unref = OpaqueData.unref;
            const getTypeTag = OpaqueData.getTypeTag;
            const getName = OpaqueData.getName;

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.float.size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.float.bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.float.alignment;
            }

            fn getBitwidth(self: ?*anyopaque) callconv(.c) u16 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.float.bits;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.FloatType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &FloatData.getSize,
                    .get_bit_size = &FloatData.getBitSize,
                    .get_alignment = &FloatData.getAlignment,
                    .get_bitwidth = &FloatData.getBitwidth,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const PointerData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            pointee_alignment: u8,
            is_const: bool,
            is_volatile: bool,
            is_allowzero: bool,
            child_id: usize,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.pointer.bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.alignment;
            }

            fn getPointeeAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.alignment;
            }

            fn isConst(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.is_const;
            }

            fn isVolatile(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.is_volatile;
            }

            fn isNonzero(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return !sel.data.pointer.is_allowzero;
            }

            fn getChildId(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.pointer.child_id;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.PointerType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &PointerData.getSize,
                    .get_bit_size = &PointerData.getBitSize,
                    .get_alignment = &PointerData.getAlignment,
                    .get_pointee_alignment = &PointerData.getPointeeAlignment,
                    .is_const = &PointerData.isConst,
                    .is_volatile = &PointerData.isVolatile,
                    .is_nonzero = &PointerData.isNonzero,
                    .get_child_id = &PointerData.getChildId,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const ArrayData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            length: usize,
            child_id: usize,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.array.size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.array.bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.array.alignment;
            }

            fn getLength(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.array.length;
            }

            fn getChildId(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.array.child_id;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.ArrayType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &ArrayData.getSize,
                    .get_bit_size = &ArrayData.getBitSize,
                    .get_alignment = &ArrayData.getAlignment,
                    .get_length = &ArrayData.getLength,
                    .get_child_id = &ArrayData.getChildId,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const StructData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            layout: std.builtin.Type.ContainerLayout,
            fields: []const StructField,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"struct".size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.@"struct".bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"struct".alignment;
            }

            fn isPackedLayout(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"struct".layout == .@"packed";
            }

            fn getFieldCount(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"struct".fields.len;
            }

            fn getFieldName(
                self: ?*anyopaque,
                index: usize,
                name: *[*:0]const u8,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"struct".fields;
                if (index >= fields.len) return false;
                name.* = fields[index].name.ptr;
                return true;
            }

            fn getFieldTypeId(
                self: ?*anyopaque,
                index: usize,
                id: *usize,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"struct".fields;
                if (index >= fields.len) return false;
                id.* = fields[index].type_id;
                return true;
            }

            fn getFieldOffset(
                self: ?*anyopaque,
                index: usize,
                offset: *usize,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"struct".fields;
                if (index >= fields.len) return false;
                offset.* = fields[index].offset;
                return true;
            }

            fn getFieldBitOffset(
                self: ?*anyopaque,
                index: usize,
                offset: *u8,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"struct".fields;
                if (index >= fields.len) return false;
                offset.* = @intCast(fields[index].bit_offset);
                return true;
            }

            fn getFieldAlignment(
                self: ?*anyopaque,
                index: usize,
                alignment: *u8,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"struct".fields;
                if (index >= fields.len) return false;
                alignment.* = fields[index].alignment;
                return true;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.StructType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &StructData.getSize,
                    .get_bit_size = &StructData.getBitSize,
                    .get_alignment = &StructData.getAlignment,
                    .is_packed_layout = &StructData.isPackedLayout,
                    .get_field_count = &StructData.getFieldCount,
                    .get_field_name = &StructData.getFieldName,
                    .get_field_type_id = &StructData.getFieldTypeId,
                    .get_field_offset = &StructData.getFieldOffset,
                    .get_field_bit_offset = &StructData.getFieldBitOffset,
                    .get_field_alignment = &StructData.getFieldAlignment,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const StructField = struct {
            name: [:0]const u8,
            type_id: usize,
            offset: usize,
            bit_offset: u3,
            alignment: u8,
        };

        const EnumData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            tag_id: usize,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"enum".size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.@"enum".bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"enum".alignment;
            }

            fn getTagId(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"enum".tag_id;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.EnumType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &EnumData.getSize,
                    .get_bit_size = &EnumData.getBitSize,
                    .get_alignment = &EnumData.getAlignment,
                    .get_tag_id = &EnumData.getTagId,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const UnionData = struct {
            size: usize,
            bitsize: u3,
            alignment: u8,
            layout: std.builtin.Type.ContainerLayout,
            fields: []const UnionField,

            fn getSize(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"union".size;
            }

            fn getBitSize(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intCast(sel.data.@"union".bitsize);
            }

            fn getAlignment(self: ?*anyopaque) callconv(.c) u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"union".alignment;
            }

            fn isPackedLayout(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"union".layout == .@"packed";
            }

            fn getFieldCount(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"union".fields.len;
            }

            fn getFieldName(
                self: ?*anyopaque,
                index: usize,
                name: *[*:0]const u8,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"union".fields;
                if (index >= fields.len) return false;
                name.* = fields[index].name.ptr;
                return true;
            }

            fn getFieldTypeId(
                self: ?*anyopaque,
                index: usize,
                id: *usize,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"union".fields;
                if (index >= fields.len) return false;
                id.* = fields[index].type_id;
                return true;
            }

            fn getFieldAlignment(
                self: ?*anyopaque,
                index: usize,
                alignment: *u8,
            ) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const fields = sel.data.@"union".fields;
                if (index >= fields.len) return false;
                alignment.* = fields[index].alignment;
                return true;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.UnionType.VTable{
                    .base = OpaqueData.vtable,
                    .get_size = &UnionData.getSize,
                    .get_bit_size = &UnionData.getBitSize,
                    .get_alignment = &UnionData.getAlignment,
                    .is_packed_layout = &UnionData.isPackedLayout,
                    .get_field_count = &UnionData.getFieldCount,
                    .get_field_name = &UnionData.getFieldName,
                    .get_field_type_id = &UnionData.getFieldTypeId,
                    .get_field_alignment = &UnionData.getFieldAlignment,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const UnionField = struct {
            name: [:0]const u8,
            type_id: usize,
            alignment: u8,
        };

        const FnData = struct {
            calling_convention: std.builtin.CallingConvention,
            is_var_args: bool,
            return_type_id: usize,
            parameters: []const FnParameter,

            fn isDefaultCallingConvention(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return @intFromEnum(sel.data.@"fn".calling_convention) == @intFromEnum(std.builtin.CallingConvention.c);
            }

            fn getCallingConvention(self: ?*anyopaque, cc: *FnType.CallingConvention) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                cc.* = switch (sel.data.@"fn".calling_convention) {
                    .x86_64_sysv => .x86_64_sysv,
                    .x86_64_win => .x86_64_win,
                    .aarch64_aapcs => .aarch64_aapcs,
                    .aarch64_aapcs_darwin => .aarch64_aapcs_darwin,
                    .aarch64_aapcs_win => .aarch64_aapcs_win,
                    else => return false,
                };
                return true;
            }

            fn getStackAlignment(self: ?*anyopaque, al: *u8) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const cfg = switch (sel.data.@"fn".calling_convention) {
                    .x86_64_sysv => |v| v,
                    .x86_64_win => |v| v,
                    .aarch64_aapcs => |v| v,
                    .aarch64_aapcs_darwin => |v| v,
                    .aarch64_aapcs_win => |v| v,
                    else => return false,
                };
                al.* = if (cfg.incoming_stack_alignment) |x|
                    @intCast(std.math.log2_int(u64, x))
                else
                    return false;
                return true;
            }

            fn isVarArgs(self: ?*anyopaque) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"fn".is_var_args;
            }

            fn getReturnTypeId(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"fn".return_type_id;
            }

            fn getParameterCount(self: ?*anyopaque) callconv(.c) usize {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.data.@"fn".parameters.len;
            }

            fn getParameterTypeId(self: ?*anyopaque, index: usize, id: *usize) callconv(.c) bool {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                const parameters = sel.data.@"fn".parameters;
                if (index >= parameters.len) return false;
                id.* = if (parameters[index].type_id) |x| x else return false;
                return true;
            }

            fn asFfi(self: *const Impl.Type) Self.Type {
                const vtable = Self.FnType.VTable{
                    .base = OpaqueData.vtable,
                    .is_default_calling_convention = &FnData.isDefaultCallingConvention,
                    .get_calling_convention = &FnData.getCallingConvention,
                    .get_stack_alignment = &FnData.getStackAlignment,
                    .is_var_args = &FnData.isVarArgs,
                    .get_return_type_id = &FnData.getReturnTypeId,
                    .get_parameter_count = &FnData.getParameterCount,
                    .get_parameter_type_id = &FnData.getParameterTypeId,
                };

                return .{
                    .data = @constCast(self),
                    .vtable = @ptrCast(&vtable),
                };
            }
        };

        const FnParameter = struct {
            type_id: ?usize,
        };

        const OpaqueData = struct {
            fn ref(self: ?*anyopaque) callconv(.c) void {
                _ = self;
            }

            fn unref(self: ?*anyopaque) callconv(.c) void {
                _ = self;
            }

            fn getTypeTag(self: ?*anyopaque) callconv(.c) Self.TypeTag {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return switch (sel.data) {
                    .void => Self.TypeTag.void,
                    .bool => Self.TypeTag.bool,
                    .int => Self.TypeTag.int,
                    .float => Self.TypeTag.float,
                    .pointer => Self.TypeTag.pointer,
                    .array => Self.TypeTag.array,
                    .@"struct" => Self.TypeTag.@"struct",
                    .@"enum" => Self.TypeTag.@"enum",
                    .@"union" => Self.TypeTag.@"union",
                    .@"fn" => Self.TypeTag.@"fn",
                    .@"opaque" => Self.TypeTag.@"opaque",
                };
            }

            fn getName(self: ?*anyopaque) callconv(.c) [*:0]const u8 {
                const sel: *const Impl.Type = @constCast(@alignCast(@ptrCast(self)));
                return sel.name.ptr;
            }

            const vtable = Self.Type.VTable{
                .ref = &OpaqueData.ref,
                .unref = &OpaqueData.unref,
                .get_type_tag = &OpaqueData.getTypeTag,
                .get_name = &OpaqueData.getName,
            };

            fn asFfi(self: *const Impl.Type) Self.Type {
                return .{
                    .data = @constCast(self),
                    .vtable = &vtable,
                };
            }
        };

        fn asFfi(self: *const Impl.Type) Self.Type {
            return switch (self.data) {
                .void => VoidData.asFfi(self),
                .bool => BoolData.asFfi(self),
                .int => IntData.asFfi(self),
                .float => FloatData.asFfi(self),
                .pointer => PointerData.asFfi(self),
                .array => ArrayData.asFfi(self),
                .@"struct" => StructData.asFfi(self),
                .@"enum" => EnumData.asFfi(self),
                .@"union" => UnionData.asFfi(self),
                .@"fn" => FnData.asFfi(self),
                .@"opaque" => OpaqueData.asFfi(self),
            };
        }
    };

    /// Casts the implementation to the C interface.
    pub fn asFfi(self: *const @This()) Self {
        const VTableImpl = struct {
            fn ref(data: ?*anyopaque) callconv(.c) void {
                _ = data;
            }

            fn unref(data: ?*anyopaque) callconv(.c) void {
                _ = data;
            }

            fn getSymbolCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.symbols.len;
            }

            fn getImportSymbolCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.import_count;
            }

            fn getExportSymbolCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.static_export_count + this.dynamic_export_count;
            }

            fn getStaticExportSymbolCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.static_export_count;
            }

            fn getDynamicExportSymbolCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.dynamic_export_count;
            }

            fn getSymbolIdByImportIndex(data: ?*anyopaque, index: usize, id: *usize) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (index >= this.import_count) return false;
                for (this.symbols) |sym| {
                    if (sym.type == .import and sym.table_index == index) {
                        id.* = sym.id;
                        return true;
                    }
                }
                return false;
            }

            fn getSymbolIdByExportIndex(data: ?*anyopaque, index: usize, id: *usize) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (index >= this.static_export_count + this.dynamic_export_count) return false;
                for (this.symbols) |sym| {
                    if ((sym.type == .static_export or sym.type == .dynamic_export) and sym.table_index == index) {
                        id.* = sym.id;
                        return true;
                    }
                }
                return false;
            }

            fn getSymbolIdByStaticExportIndex(data: ?*anyopaque, index: usize, id: *usize) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (index >= this.static_export_count) return false;
                for (this.symbols) |sym| {
                    if (sym.type == .static_export and sym.decl_index == index) {
                        id.* = sym.id;
                        return true;
                    }
                }
                return false;
            }

            fn getSymbolIdByDynamicExportIndex(data: ?*anyopaque, index: usize, id: *usize) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (index >= this.dynamic_export_count) return false;
                for (this.symbols) |sym| {
                    if (sym.type == .dynamic_export and sym.decl_index == index) {
                        id.* = sym.id;
                        return true;
                    }
                }
                return false;
            }

            fn getSymbolById(data: ?*anyopaque, id: usize, symbol: *Self.Symbol) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (id >= this.symbols.len) return false;
                symbol.* = this.symbols[id].asFfi();
                return true;
            }

            fn getTypeCount(data: ?*anyopaque) callconv(.c) usize {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                return this.types.len;
            }

            fn getTypeById(data: ?*anyopaque, id: usize, @"type": *Self.Type) callconv(.c) bool {
                const this: *const Impl = @alignCast(@ptrCast(@constCast(data)));
                if (id < this.types.len) @"type".* = this.types[id].asFfi() else return false;
                return false;
            }
        };

        const vtable = Self.VTable{
            .ref = &VTableImpl.ref,
            .unref = &VTableImpl.unref,
            .get_symbol_count = &VTableImpl.getSymbolCount,
            .get_import_symbol_count = &VTableImpl.getImportSymbolCount,
            .get_export_symbol_count = &VTableImpl.getExportSymbolCount,
            .get_static_export_symbol_count = &VTableImpl.getStaticExportSymbolCount,
            .get_dynamic_export_symbol_count = &VTableImpl.getDynamicExportSymbolCount,
            .get_symbol_id_by_import_index = &VTableImpl.getSymbolIdByImportIndex,
            .get_symbol_id_by_export_index = &VTableImpl.getSymbolIdByExportIndex,
            .get_symbol_id_by_static_export_index = &VTableImpl.getSymbolIdByStaticExportIndex,
            .get_symbol_id_by_dynamic_export_index = &VTableImpl.getSymbolIdByDynamicExportIndex,
            .get_symbol_by_id = &VTableImpl.getSymbolById,
            .get_type_count = &VTableImpl.getTypeCount,
            .get_type_by_id = &VTableImpl.getTypeById,
        };

        return .{
            .data = @constCast(self),
            .vtable = &vtable,
        };
    }
};
