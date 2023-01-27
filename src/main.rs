#![feature(string_remove_matches)]

use std::{
    env::args,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::{Linkage, Module},
    targets::*,
    types::{BasicType, FunctionType},
    values::{FunctionValue, IntValue},
    AddressSpace, IntPredicate, OptimizationLevel,
};

static VALID_CHARS: [char; 8] = ['+', '-', '<', '>', '[', ']', ',', '.'];
static DATA_SIZE: u64 = 30_000;

struct Loop<'a> {
    entry_block: BasicBlock<'a>,
    loop_block: BasicBlock<'a>,
    exit_block: BasicBlock<'a>,
    builder: &'a Builder<'a>,
    context: &'a Context,
}

// impl<'a> Loop<'a> {
//     fn new(context: &'a Context, builder: &'a Builder<'a>, function: FunctionValue<'a>, inside_loop: fn()) -> Self {
//         let loop_block = context.append_basic_block(function, "loop");
//         let exit_block = context.append_basic_block(function, "exit");

//         // Types
//         let i32_type = context.i32_type();

//         // Goto Loop
//         builder.build_unconditional_branch(self.loop_block);

//         // -- Loop --
//         builder.position_at_end(self.loop_block);

//         loop_fn(builder, context, i_val);

//         // Step i;
//         let i_val = builder.build_int_add(i_val, i32_type.const_int(self.step, false), "");
//         builder.build_store(i, i_val);

//         // if i < DATA_SIZE Goto Loop else Goto Break
//         builder.build_conditional_branch(
//             builder.build_int_compare(IntPredicate::ULT, i_val, data_size, ""),
//             loop_block,
//             break_block,
//         );

//         // -- Break --
//         builder.position_at_end(break_block);

//         // return;
//         builder.build_return(None);

//         self.builder.position_at_end(self.loop_block);
//         loop_fn(self.builder);
//         self.builder.build_un

//         self.builder.position_at_end(self.exit_block);
//     }

//     fn build_loop(&self, loop_fn: fn(&'a Builder<'a>, &'a Context, IntValue)) {

//     }
// }

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
                let loop_bounds = (pc, get_loop_closing_index(instructions, pc));

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

    let data_contents_type = i8_type;
    let data_type = data_contents_type.ptr_type(AddressSpace::default());
    let ptr_type = i64_type;

    // #define DATA_SIZE 30000;
    let data_size = i32_type.const_int(DATA_SIZE, false);

    // Link to printf
    let printf_type = i32_type.fn_type(&[i8_type.ptr_type(AddressSpace::default()).into()], true);
    let printf = module.add_function("printf", printf_type, Some(Linkage::External));

    // Link to getchar
    let getchar_type = i32_type.fn_type(&[], true);
    let getchar = module.add_function("getchar", getchar_type, Some(Linkage::External));

    // Link to exit
    let exit_type = void_type.fn_type(&[], true);
    let exit = module.add_function("exit", exit_type, Some(Linkage::External));

    // -- Printf --

    macro_rules! build_printf {
        ($format:literal, $($vars:tt),*) => {
            builder.build_call(
                printf,
                &[
                    builder
                        .build_global_string_ptr($format, "")
                        .as_pointer_value()
                        .into(),
                    $($vars.into(),)*
                ],
                "",
            );
        };
    }

    // void panic(const char* reason, int code) {}
    let panic_type = void_type.fn_type(
        &[
            i8_type.ptr_type(AddressSpace::default()).into(),
            i32_type.into(),
        ],
        true,
    );
    let panic = module.add_function("panic", panic_type, None);
    let panic_block = context.append_basic_block(panic, "");

    // -- PANIC --
    {
        builder.position_at_end(panic_block);

        let reason = panic.get_nth_param(0).unwrap().into_pointer_value();
        let code = panic.get_nth_param(1).unwrap().into_int_value();

        builder.build_call(
            printf,
            &[
                builder
                    .build_global_string_ptr("Panic!!!!\nError Code %d: %s\n", "")
                    .as_pointer_value()
                    .into(),
                code.into(),
                reason.into(),
            ],
            "",
        );

        builder.build_call(exit, &[code.into()], "");
        builder.build_return(None);
    }

    macro_rules! build_panic {
        ($code:literal, $($reason:tt)*) => {
            let code: IntValue = i32_type.const_int($code, false);

            builder.build_call(
                panic,
                &[
                    builder
                        .build_global_string_ptr(&$($reason)*, "")
                        .as_pointer_value()
                        .into(),
                    code.into(),
                ],
                "",
            );
            builder.build_return(Some(&code));
        }
    }

    // macro_rules! build_panic_block {
    //     ($name:literal, $code:literal, $($reason:tt)*) => {
    //         {
    //         let fn_type = void_type.fn_type(&[], true);
    //         let panic_fn = module.add_function($name, fn_type, None);

    //         let block = context.append_basic_block(panic_fn, "");
    //         builder.position_at_end(block);

    //         build_panic!($code, $($reason)*);

    //         block
    //         }
    //     };
    // }

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

    // int main() {}
    let main_type = i32_type.fn_type(&[], true);
    let main = module.add_function("main", main_type, None);

    let main_block = context.append_basic_block(main, "");

    // // Panic Blocks
    // let panic_ptr_overflow = build_panic_block!(
    //     "panic_ptr_overflow",
    //     1,
    //     format!("Pointer Overflow - Data pointer must be less than {DATA_SIZE}")
    // );

    // let panic_ptr_underflow = build_panic_block!(
    //     "panic_ptr_underflow",
    //     2,
    //     "Pointer Underflow - Data pointer must be greater than 0"
    // );

    // -- MAIN --
    {
        builder.position_at_end(main_block);

        let mut loop_blocks: Vec<(BasicBlock, BasicBlock)> = Vec::new();

        // char data[DATA_SIZE];
        let data = builder
            .build_array_malloc(data_contents_type, data_size, "")
            .unwrap();

        builder.build_call(zero_data, &[data.into()], "");

        // long ptr = 0;
        let ptr = builder.build_alloca(ptr_type, "");
        builder.build_store(ptr, ptr_type.const_zero());

        for instruction in instructions {
            match instruction {
                '+' => {
                    // -- Increment value at pointer --

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    let current_val = builder
                        .build_load(data_contents_type, data_offset, "")
                        .into_int_value();
                    let new_val = builder.build_int_add(
                        current_val,
                        data_contents_type.const_int(1, true),
                        "",
                    );

                    builder.build_store(data_offset, new_val);
                }
                '-' => {
                    // -- Decrement value at pointer --

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    let current_val = builder
                        .build_load(data_contents_type, data_offset, "")
                        .into_int_value();
                    let new_val = builder.build_int_sub(
                        current_val,
                        data_contents_type.const_int(1, true),
                        "",
                    );

                    builder.build_store(data_offset, new_val);
                }
                '>' => {
                    // -- Increment Pointer --

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let ptr_val = builder.build_int_add(ptr_val, ptr_type.const_int(1, false), "");

                    builder.build_store(ptr, ptr_val);
                }
                '<' => {
                    // -- Decrement Pointer --

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let ptr_val = builder.build_int_sub(ptr_val, ptr_type.const_int(1, false), "");

                    builder.build_store(ptr, ptr_val);
                }
                '[' => {
                    // -- Open Loop --

                    let loop_block = context.append_basic_block(main, "loop");
                    let break_block = context.append_basic_block(main, "break");

                    loop_blocks.push((loop_block, break_block));

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    let current_val = builder
                        .build_load(data_contents_type, data_offset, "")
                        .into_int_value();

                    builder.build_conditional_branch(
                        builder.build_int_compare(
                            IntPredicate::EQ,
                            current_val,
                            data_contents_type.const_int(0, false),
                            "",
                        ),
                        break_block,
                        loop_block,
                    );

                    builder.position_at_end(loop_block);
                }
                ']' => {
                    // -- Close Loop --

                    let (loop_block, break_block) =
                        loop_blocks.pop().expect("Extra Closing Bracket");

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    let current_val = builder
                        .build_load(data_contents_type, data_offset, "")
                        .into_int_value();

                    builder.build_conditional_branch(
                        builder.build_int_compare(
                            IntPredicate::EQ,
                            current_val,
                            data_contents_type.const_int(0, false),
                            "",
                        ),
                        break_block,
                        loop_block,
                    );

                    builder.position_at_end(break_block);
                }
                ',' => {
                    // -- Get Input And Store At Pointer --

                    let input = builder.build_call(getchar, &[], "");
                    let input = input.try_as_basic_value().left().unwrap().into_int_value();
                    let casted_input = builder.build_int_cast(input, i8_type, "");

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    builder.build_store(data_offset, casted_input);
                }
                '.' => {
                    // -- Print --

                    let ptr_val = builder.build_load(ptr_type, ptr, "").into_int_value();
                    let data_offset =
                        unsafe { builder.build_gep(data_contents_type, data, &[ptr_val], "") };

                    let current_val = builder
                        .build_load(data_contents_type, data_offset, "")
                        .into_int_value();

                    build_printf!("%c", current_val);
                }
                other => panic!("Invalid Instruction {other}"),
            }
        }

        // return 0;
        builder.build_return(Some(&i32_type.const_zero()));
    }

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
