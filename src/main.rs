#![feature(slice_concat_ext)]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;

use failure::Error;
use std::collections::HashSet;
use std::fs::File;
use std::io::stdin;
use std::slice::SliceConcatExt;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

#[derive(Deserialize, Serialize)]
struct Memories {
    pub rules: Vec<String>,
    pub terms: Vec<String>,
}
impl Memories {
    pub fn new() -> Self {
        Memories {
            rules: Vec::new(),
            terms: Vec::new(),
        }
    }
}

enum Command {
    None,
    Implies(String, String),
    Details(String),
}

fn main() -> Result<(), Error> {
    let mem: Memories = File::open("memories.cbor")
        .map_err(Error::from)
        .and_then(|f| serde_cbor::from_reader(f).map_err(Error::from))
        .or_else(|_| Ok::<Memories, Error>(Memories::new()))?;

    let mem_tex = Arc::new((Mutex::new(mem), Condvar::new()));
    let save_tex = mem_tex.clone();
    std::thread::spawn(move || -> Result<(), Error> {
        loop {
            let mem_lock = save_tex.1.wait(save_tex.0.lock().unwrap()).unwrap();
            let mut file = File::create("memories.cbor")?;
            serde_cbor::to_writer(&mut file, &*mem_lock)?;
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
            if words == "exit" {
                println!("Goodbye!");
                std::process::abort();
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
                let mut prog = memory.rules.join("\n");
                drop(memory);
                prog += &format!("\n{}(1).", a);
                println!("Hmmm...");
                ctl.add("base", &[], &prog)?;
                ctl.ground(&[clingo::Part::new("base", &[])?])?;
                let mut handle = ctl.solve(clingo::SolveMode::YIELD, &[])?;
                let mut set_vec: Vec<HashSet<String>> = Vec::new();
                loop {
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
                    println!("All {} are {}.", a, item);
                }
                for item in some_set {
                    println!("Some {} are {}.", a, item);
                }
            }
            Command::Implies(a, b) => {
                let mut memory = mem_tex.0.lock().unwrap();
                let mut ctl = clingo::Control::new(Vec::new())?;
                let mut prog = memory.rules.join("\n");
                let rule_to_add = format!("{}(A) :- {}(A).", b, a);
                prog += &format!("\n{}", rule_to_add);
                for (i, term) in memory.terms.iter().enumerate() {
                    prog += &format!("\n{}({}).", term, i);
                }
                println!("Hmmm...");
                ctl.add("base", &[], &prog)?;
                ctl.ground(&[clingo::Part::new("base", &[])?])?;
                let mut handle = ctl.solve(clingo::SolveMode::YIELD, &[])?;
                let res = handle.get()?;
                if !res.contains(clingo::SolveResult::SATISFIABLE) {
                    println!("That doesn't seem right...");
                // TODO remove a rule
                } else {
                    println!("Ok!");
                    memory.rules.push(rule_to_add);
                    mem_tex.1.notify_all();
                }
            }
            _ => (),
        }
        user_input = String::new();
    }
}
