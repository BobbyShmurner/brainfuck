#![feature(string_remove_matches)]

use core::panic;
use std::{
    env::args,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use inkwell::{
    context::Context,
    module::{Linkage, Module},
    targets::*,
    values::IntValue,
    AddressSpace, IntPredicate, OptimizationLevel,
};

static VALID_CHARS: [char; 8] = ['+', '-', '<', '>', '[', ']', ',', '.'];

fn get_loop_closing_index(instructions: &[char], open_loop_index: usize) -> usize {
    let mut loop_depth = 0;
    let mut i = open_loop_index;

    loop {
        if i >= instructions.len() {
            panic!("Failed to find closing bracket");
        }

        if instructions[i] == '[' {
            loop_depth += 1;
        }

        if instructions[i] == ']' {
            loop_depth -= 1;
            if loop_depth == 0 {
                break;
            }
        }

        i += 1;
    }

    i
}

fn interperate(instructions: &[char]) {
    let mut data: Vec<i8> = vec![0];
    let mut loops: Vec<(usize, usize)> = Vec::new();
    let mut ptr: usize = 0;
    let mut pc: usize = 0;
    let mut stdout = io::stdout();

    loop {
        if pc >= instructions.len() {
            stdout.flush().expect("Failed to flush stdout");
            break;
        }

        match instructions[pc] {
            '+' => data[ptr] += 1,
            '-' => data[ptr] -= 1,
            '>' => {
                ptr += 1;

                if ptr >= data.len() {
                    data.push(0);
                }
            }
            '<' => {
                if ptr == 0 {
                    panic!("Cannot have a negative pointer");
                }

                ptr -= 1;
            }
            '[' => {
                let loop_bounds = (pc, get_loop_closing_index(&instructions, pc));

                if data[ptr] == 0 {
                    pc = loop_bounds.1;
                } else {
                    loops.push(loop_bounds);
                }
            }
            ']' => {
                if data[ptr] == 0 {
                    loops.pop();
                } else {
                    pc = loops.last().expect("Extra ']' Found").0;
                }
            }
            ',' => {
                let input;
                stdout.flush().expect("Failed to flush stdout");

                unsafe {
                    input = libc::getchar();
                }

                data[ptr] = input.try_into().expect("Invalid Input");
            }
            '.' => {
                print!("{}", data[ptr] as u8 as char);
            }
            other => {
                panic!("Invalid Instruction {other}");
            }
        }

        pc += 1;
    }
}

fn compile_module(module: &Module) {
    Target::initialize_all(&InitializationConfig::default());

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).unwrap();
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .unwrap();

    module.set_data_layout(&machine.get_target_data().get_data_layout());
    module.set_triple(&triple);

    machine
        .write_to_file(module, FileType::Assembly, Path::new("./test.asm"))
        .unwrap();

    machine
        .write_to_file(module, FileType::Object, Path::new("./test.o"))
        .unwrap();
}

fn compile(instructions: &[char], module_name: &str) {
    // Setup module
    let context = Context::create();
    let builder = context.create_builder();
    let module = context.create_module(module_name);

    // Types
    let i32_type = context.i32_type();
    let i8_type = context.i8_type();
    let i64_type = context.i64_type();
    let void_type = context.void_type();

    let data_contents_type = context.i8_type();
    let data_type = data_contents_type.ptr_type(AddressSpace::default());

    // #define DATA_SIZE 30000;
    let data_size = i32_type.const_int(30_000, false);

    // Link to printf
    let printf_type = i32_type.fn_type(&[i8_type.ptr_type(AddressSpace::default()).into()], true);
    let printf = module.add_function("printf", printf_type, Some(Linkage::External));

    // Link to getchar
    let getchar_type = i32_type.fn_type(&[], true);
    let getchar = module.add_function("getchar", getchar_type, Some(Linkage::External));

    // int main() {}
    let main_type = i32_type.fn_type(&Vec::new(), false);
    let main_func = module.add_function("main", main_type, None);

    let main_block = context.append_basic_block(main_func, "");

    // -- Zero Out Data --

    // void zero_data(char* data) {}
    let zero_data = module.add_function(
        "zero_data",
        void_type.fn_type(&[data_type.into()], true),
        None,
    );

    {
        let entry_block = context.append_basic_block(zero_data, "entry");
        let loop_block = context.append_basic_block(zero_data, "loop");
        let break_block = context.append_basic_block(zero_data, "break");
        let data = zero_data.get_first_param().unwrap();

        // -- Entry --
        builder.position_at_end(entry_block);

        // int i = 0
        let i = builder.build_alloca(i32_type, "");
        builder.build_store(i, i32_type.const_zero());

        // Goto Loop
        builder.build_unconditional_branch(loop_block);

        // -- Loop --
        builder.position_at_end(loop_block);

        let i_val = builder.build_load(i32_type, i, "").into_int_value();

        // char* data_entry = data + i;
        let data_entry = unsafe {
            builder.build_gep(data_contents_type, data.into_pointer_value(), &[i_val], "")
        };

        // *data_entry = 0;
        builder.build_store(data_entry, i32_type.const_zero());

        // i = i + 1;
        let i_val = builder.build_int_add(i_val, i32_type.const_int(1, false), "");
        builder.build_store(i, i_val);

        // if i < DATA_SIZE Goto Loop else Goto Break
        builder.build_conditional_branch(
            builder.build_int_compare(IntPredicate::ULT, i_val, data_size, ""),
            loop_block,
            break_block,
        );

        // -- Break --
        builder.position_at_end(break_block);

        // return;
        builder.build_return(None);
    }

    // -- MAIN --
    builder.position_at_end(main_block);

    // char data[DATA_SIZE];
    let data = builder
        .build_array_malloc(data_contents_type, data_size, "")
        .unwrap();

    builder.build_call(zero_data, &[data.into()], "");

    // long ptr = 0;
    let ptr = builder.build_alloca(i64_type, "");
    builder.build_store(ptr, i64_type.const_zero());

    // for instruction in instructions {
    //     match instruction {
    //         '+' => {
    //             let ptr_val = builder.build_load(i64_type, ptr, "").into_int_value();
    //             let data_offset = unsafe { builder.build_gep(i8_type, data, &[ptr_val], "") };

    //             let current_val = builder.build_load(i8_type, data_offset, "");
    //         }
    //         // '-' => data[ptr] -= 1,
    //         // '>' => {
    //         //     ptr += 1;

    //         //     if ptr >= data.len() {
    //         //         data.push(0);
    //         //     }
    //         // }
    //         // '<' => {
    //         //     if ptr == 0 {
    //         //         panic!("Cannot have a negative pointer");
    //         //     }

    //         //     ptr -= 1;
    //         // }
    //         // '[' => {
    //         //     let loop_bounds = (pc, get_loop_closing_index(&instructions, pc));

    //         //     if data[ptr] == 0 {
    //         //         pc = loop_bounds.1;
    //         //     } else {
    //         //         loops.push(loop_bounds);
    //         //     }
    //         // }
    //         // ']' => {
    //         //     if data[ptr] == 0 {
    //         //         loops.pop();
    //         //     } else {
    //         //         pc = loops.last().expect("Extra ']' Found").0;
    //         //     }
    //         // }
    //         // ',' => {
    //         //     let input;
    //         //     stdout.flush().expect("Failed to flush stdout");

    //         //     unsafe {
    //         //         input = libc::getchar();
    //         //     }

    //         //     data[ptr] = input.try_into().expect("Invalid Input");
    //         // }
    //         // '.' => {
    //         //     print!("{}", data[ptr] as u8 as char);
    //         // }
    //         other => {
    //             panic!("Invalid Instruction {other}");
    //         }
    //     }
    // }

    // return 0;
    builder.build_return(Some(&context.i32_type().const_zero()));

    compile_module(&module);
}

fn main() {
    let args: Vec<_> = args().collect();
    let module_name = match args.get(2) {
        Some(val) => val.as_str(),
        None => "bf_mod",
    };

    let mut file = File::open(args.get(1).expect("Please supply an input file argument"))
        .expect("Failed to open file");
    let mut contents = String::new();

    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    contents.remove_matches(|c: char| !VALID_CHARS.contains(&c));
    let instructions: Vec<char> = contents.chars().collect();

    // interperate(&instructions);
    compile(&instructions, module_name);
}
