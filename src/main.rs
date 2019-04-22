#![feature(slice_concat_ext)]
#[macro_use]
extern crate failure;

use failure::Error;
use std::collections::HashSet;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Write};
use std::slice::SliceConcatExt;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

type Memories = Vec<String>;

enum Command {
    None,
    Implies(String, String),
    Details(String),
}

fn main() -> Result<(), Error> {
    let file = File::open("memories.lp").or_else(|_| File::create("memories.lp"))?;
    let mem = BufReader::new(file)
        .lines()
        .collect::<Result<Memories, _>>()?;
    let mut file = File::open("memories.lp")?;

    let mem_tex = Arc::new((Mutex::new(mem), Condvar::new()));
    let save_tex = mem_tex.clone();
    std::thread::spawn(move || -> Result<(), Error> {
        loop {
            let mem_lock = save_tex.1.wait(save_tex.0.lock().unwrap()).unwrap();
            file.write(&mem_lock.join("\n").as_bytes())?;
        }
    });
    println!("Hello!");
    println!("Tell me something about your world!");

    let mut user_input = String::new();
    'repl: loop {
        stdin().read_line(&mut user_input)?;

        user_input.make_ascii_lowercase();
        let mut words = String::new();
        let mut command = Command::None;
        for word in user_input.split_whitespace() {
            if word == "is" || word == "are" {
                command = Command::Implies(words, String::new());
                words = String::new();
                continue;
            }
            if words.len() > 0 {
                words += "_";
            }
            words += word;
            if words == "tell_me_about" {
                command = Command::Details(String::new());
                words = String::new();
                continue;
            }
        }
        match command {
            Command::Details(_) => {
                command = Command::Details(words);
            }
            Command::Implies(a, _) => {
                command = Command::Implies(a, words);
            }
            _ => (),
        }

        match command {
            Command::Details(a) => {
                let memory = mem_tex.0.lock().unwrap();
                let mut ctl = clingo::Control::new(Vec::new())?;
                let mut prog = memory.join("\n");
                drop(memory);
                prog += &format!("\n{}(1).", a);
                println!("Hmmm...");
                ctl.add("base", &[], &prog)?;
                ctl.ground(&[clingo::Part::new("base", &[])?])?;
                // eprintln!("Grounded.");
                let mut handle = ctl.solve(clingo::SolveMode::YIELD, &[])?;
                let mut set_vec: Vec<HashSet<String>> = Vec::new();
                loop {
                    // eprintln!("Solving...");
                    let res = handle.model()?;
                    if let Some(model) = res {
                        set_vec.push(
                            model
                                .symbols(clingo::ShowType::ALL)?
                                .into_iter()
                                .map(|s| s.name().map(|a| a.to_owned()))
                                .collect::<Result<HashSet<String>, _>>()?,
                        );
                        handle.resume()?;
                    } else {
                        break;
                    }
                }
                let seed = set_vec.get(0).ok_or(format_err!("IMPOSSIBLE!"))?;
                let mut all_set = set_vec.clone().into_iter().fold(seed.clone(), |acc, val| {
                    acc.intersection(&val).cloned().collect()
                });
                all_set.remove(&a);
                let mut some_set: HashSet<String> = set_vec
                    .into_iter()
                    .fold(HashSet::new(), |acc, val| {
                        acc.union(&val).cloned().collect()
                    })
                    .difference(&all_set)
                    .cloned()
                    .collect();
                some_set.remove(&a);
                if all_set.len() == 0 && some_set.len() == 0 {
                    println!("I don't know anything about {}.", a);
                }
                for item in all_set {
                    println!("All {} is {}.", a, item);
                }
                for item in some_set {
                    println!("Some {} is {}.", a, item);
                }
            }
            _ => (),
        }
        user_input = String::new();
    }
}
