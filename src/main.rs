#![feature(string_remove_matches)]

use std::{
    env::args,
    fs::File,
    io::{self, Read, Write},
};

static VALID_CHARS: [char; 8] = ['+', '-', '<', '>', '[', ']', ',', '.'];

fn get_loop_closing_index(instructions: &[char], open_loop_index: usize) -> usize {
    let mut loop_depth = 0;
    let mut i = open_loop_index;

    loop {
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

fn main() {
    let args: Vec<_> = args().collect();
    let mut file = File::open(&args[1]).unwrap();
    let mut contents = String::new();

    file.read_to_string(&mut contents).unwrap();

    contents.remove_matches(|c: char| !VALID_CHARS.contains(&c));
    let instructions: Vec<char> = contents.chars().collect();

    let mut data: Vec<i8> = vec![0];
    let mut loops: Vec<(usize, usize)> = Vec::new();
    let mut ptr: usize = 0;
    let mut pc: usize = 0;
    let mut stdout = io::stdout();

    loop {
        if pc >= contents.len() {
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
            '<' => ptr -= 1,
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
                    pc = loops.last().unwrap().0;
                }
            }
            ',' => {
                let input;

                unsafe {
                    input = libc::getchar();
                }

                data[ptr] = input.try_into().expect("Invalid Input!");
            }
            '.' => {
                let out = char::from_u32(data[ptr] as u32).unwrap();
                print!("{out}");
                stdout.flush().unwrap();
            }
            other => {
                panic!("Invalid Instruction {other}");
            }
        }

        pc += 1;
    }
}
