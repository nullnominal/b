use crate::nob::*;
use core::ffi::*;
use core::mem::zeroed;
use crate::printf;

// We keep this field separate so that it may fail on unsupported versions
const version: u8 = 0x00;
#[derive(Clone, Copy)]
pub struct IRSections {
    pub extern_section: u64,
    pub data_section: u64,
    pub globals_section: u64,
    pub functs_section: u64
}

#[derive(Clone, Copy)]
pub struct IRString {
    pub length: u64,
    pub content: String_Builder
}
#[derive(Clone, Copy)]
pub struct IRExterns {
    pub count: u64,
    pub externs: Array<IRString>
}
#[derive(Clone, Copy)]
pub struct IRData {
    pub length: u64,
    pub data: String_Builder
}

#[derive(Clone, Copy)]
pub enum IRValue {
    Name(IRString),
    Literal(u64),
    Offset(u64),
}

// TODO: Parse to a straight Global
#[derive(Clone, Copy)]
pub struct IRGlobal {
    pub name: IRString,
    pub values: Array<IRValue>,

    pub is_vec: bool,
    pub min_size: u64
}
#[derive(Clone, Copy)]
pub struct IRFunction {
    // TODO
    pub name: IRString,
    pub file: IRString,

    pub params: u64,
    pub autovars: u64,
    pub bodysize: u64,
}

#[derive(Clone, Copy)]
pub struct IRInfo {
    pub source: *mut String_Builder,
    pub sections: IRSections,

    pub externs: IRExterns,
    pub data: IRData,
    pub globals: Array<IRGlobal>,
    pub functions: Array<IRFunction>

}

pub unsafe fn load8(output: *mut String_Builder, offset: usize) -> u8 {
    *((*output).items.add(offset)) as u8
}
pub unsafe fn load64(output: *mut String_Builder, offset: usize) -> u64 {
    let mut val: u64 = 0u64;
    for i in 0..8 {
        // IR integers are little endian
        let byte = 7 - i;
        val <<= 8;
        val |= (*((*output).items.add(byte+offset)) as u8) as u64;
    }

    val
}
pub unsafe fn loadstr(output: *mut String_Builder, offset: usize) -> IRString {
    let mut string: IRString = zeroed();

    string.length = load64(output, offset);
    string.content = zeroed();
    for i in 0u64..string.length {
        let off: usize = i as usize + offset + 8;
        da_append(&mut (string.content), *((*output).items.add(off)));
    }
    da_append(&mut (string.content), 0 as c_char);

    string
}
pub unsafe fn load_externs(ir: *mut IRInfo) {
    let mut offset: usize = (*ir).sections.extern_section as usize;
    let externs: u64 = load64((*ir).source, offset);

    offset += 8;
    for _ in 0u64..externs {
        let string: IRString = loadstr((*ir).source, offset);
        da_append(&mut (*ir).externs.externs, string);
        offset += (8 + string.length) as usize;
    }
    (*ir).externs.count = externs;
}
pub unsafe fn load_data(ir: *mut IRInfo) {
    let mut offset: usize = (*ir).sections.data_section as usize;
    let count: u64 = load64((*ir).source, offset);

    offset += 8;
    for _ in 0u64..count {
        let byte: c_char = *(*(*ir).source).items.add(offset);
        da_append(&mut (*ir).data.data, byte);
        offset += 1;
    }
    (*ir).data.length = count;
}
pub unsafe fn load_globals(ir: *mut IRInfo) {
    let mut offset: usize = (*ir).sections.globals_section as usize;
    let count: u64 = load64((*ir).source, offset);
    offset += 8;

    for _ in 0u64..count {
        // Try loading a global value
        let name: IRString = loadstr((*ir).source, offset);
        offset += (8 + name.length) as usize;
        let count: u64 = load64((*ir).source, offset);
        offset += 8;
        let mut global: IRGlobal = zeroed();

        global.name = name;
        global.values = zeroed();
        
        for __ in 0..count {
            let kind: u8 = load8((*ir).source, offset);
            let global_val: IRValue;
            offset += 1;
            if kind == 0x00 {
                let vname: IRString = loadstr((*ir).source, offset);
                global_val = IRValue::Name(vname);
                offset += (8 + vname.length) as usize;
            } else if kind == 0x01 {
                let literal: u64 = load64((*ir).source, offset);
                offset += 8;
                global_val = IRValue::Literal(literal);
            } else if kind == 0x02 {
                let off: u64 = load64((*ir).source, offset);
                global_val = IRValue::Literal(off);
                offset += 8;
            } else {
                unreachable!("bogus-amogus");
            }
            da_append(&mut global.values, global_val);
        }
        
        let is_vec: bool = load8((*ir).source, offset) != 0;
        offset += 1;
        let min_size: u64 = load64((*ir).source, offset);
        offset += 8;

        global.is_vec = is_vec;
        global.min_size = min_size;
        da_append(&mut (*ir).globals, global);
    }
    (*ir).data.length = count;
}
pub unsafe fn load_functions(ir: *mut IRInfo) {
    let mut offset: usize = (*ir).sections.functs_section as usize;
    let count: u64 = load64((*ir).source, offset);

    offset += 8;
    for _ in 0u64..count {
        // TODO: Load function
        let mut function: IRFunction = zeroed();

        function.name = loadstr((*ir).source, offset);
        offset += (8 + function.name.length) as usize;
        function.file = loadstr((*ir).source, offset);
        offset += (8 + function.file.length) as usize;

        function.params = load64((*ir).source, offset);
        offset += 8;
        function.autovars = load64((*ir).source, offset);
        offset += 8;
        function.bodysize = load64((*ir).source, offset);
        offset += 8;

        // TODO: Manage opcodes

        da_append(&mut (*ir).functions, function);
    }
    (*ir).data.length = count;
}

pub unsafe fn load_bytecode(ir: *mut IRInfo, output: *mut String_Builder, bytecode_path: *const c_char) -> Option<()> {
    read_entire_file(bytecode_path, output)?;
    let magic: [u8;2] = [ *((*output).items.add(0)) as u8, *((*output).items.add(1)) as u8 ];
    let bvers: u8 = *((*output).items.add(2)) as u8;

    if magic[0] != 0xDE || magic[1] != 0xBC || bvers != version {
        // Invalid/incompatible header
        None
    } else {
        let mut off: usize = (*output).count;

        (*ir).source = output;

        // Start loading the sections
        off -= 8;
        (*ir).sections.functs_section = load64(output, off);
        off -= 8;
        (*ir).sections.globals_section = load64(output, off);
        off -= 8;
        (*ir).sections.data_section = load64(output, off);
        off -= 8;
        (*ir).sections.extern_section = load64(output, off);

        // Now, start loading each substructure
        load_externs(ir);
        load_data(ir);
        load_globals(ir);

        load_functions(ir);
        Some(())
    }
}

pub unsafe fn run(_cmd: *mut Cmd, output: *mut String_Builder, bytecode_path: *const c_char, _stdout_path: Option<*const c_char>) -> Option<()> {
    let mut ir: IRInfo = zeroed();
    (*output).count = 0;
    load_bytecode(&mut ir, output, bytecode_path)?;

    printf(c!("functions=0x%X globals=0x%X data=0x%X extrn=0x%X\n"), 
        ir.sections.functs_section as c_uint,
        ir.sections.globals_section as c_uint,
        ir.sections.data_section as c_uint,
        ir.sections.extern_section as c_uint
    );
    todo!("actually implement the bytecode runner");
}


