//! The meta-program that analyses <root>/src/codegen/ folder and makes the custom pluggable codegens
//! available to b and btest.
//!
//! Rust's proc macros suck btw jfyi
#![no_main]
#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_macros)]

#[macro_use]
pub mod crust;
pub mod nob;

use core::ffi::*;
use core::mem::zeroed;
use nob::*;
use crust::libc::*;
use crust::*;

pub unsafe fn aggregate_libb(children: *mut File_Paths) -> Option<()> {
    if !mkdir_if_not_exists(c!("./build/libb/")) { return None; }

    if !read_entire_dir(c!("./libb/"), children) { return None; }

    for i in 0..(*children).count {
        let child = *(*children).items.add(i);
        if *child == '.' as c_char { continue; }
        if !copy_file(
            temp_sprintf(c!("./libb/%s"), child),
            temp_sprintf(c!("./build/libb/%s"), child),
        ) { return None; }
    }
    Some(())
}

pub unsafe fn main(mut _argc: i32, mut _argv: *mut*mut c_char) -> Option<()> {
    let mut children: File_Paths = zeroed();

    if !mkdir_if_not_exists(c!("./build/")) { return None; }

    aggregate_libb(&mut children)?;

    let parent = c!("./src/codegen");
    children.count = 0;
    if !read_entire_dir(parent, &mut children) { return None; }
    qsort(children.items as *mut c_void, children.count, size_of::<*const c_char>(), compar_cstr);
    let mut sb: String_Builder = zeroed();
    log(Log_Level::INFO, c!("CODEGENS:"));
    sb_appendf(&mut sb, c!("codegens! {\n"));
    for i in 0..children.count {
        let child = *children.items.add(i);
        if *child == '.' as c_char { continue; }
        if strcmp(child, c!("mod.rs")) == 0 { continue; }
        // TODO: skip the modules that have invalid Rust names.
        //   Or is there any way to accomodate them into the Rust module system too?
        //   In any case we should do something with the invalid module names.
        let child = temp_strip_suffix(child, c!(".rs")).unwrap_or(child);
        sb_appendf(&mut sb, c!("    %s,\n"), child);
        log(Log_Level::INFO, c!("    %s"), child);
    }
    sb_appendf(&mut sb, c!("}\n"));
    let output_path = temp_sprintf(c!("%s/.INDEX.rs"), parent);
    write_entire_file(output_path, sb.items as _, sb.count)?;
    log(Log_Level::INFO, c!("Generated %s"), output_path);
    Some(())
}
