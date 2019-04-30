#![feature(slice_concat_ext)]

#[macro_use]
extern crate failure;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate serde_derive;

use failure::Error;
use inflector::Inflector;
use pest::Parser;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::stdin;
use std::slice::SliceConcatExt;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;


#[derive(Parser)]
#[grammar = "commands.pest"]
struct CmdParser;

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
    AddRule(String),
    Details(String),
}

// fn conj_be_for(mem: &Memories, word: &str) -> &'static str {
//     if mem
//         .terms
//         .get(word)
//         .cloned()
//         .unwrap_or_else(|| word.to_singular() != word)
//     {
//         "are"
//     } else {
//         "is"
//     }
// }

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
        let mut command = Command::None;
        let pairs = CmdParser::parse(Rule::command, &user_input)
            .map(|p| p.collect::<Vec<_>>())
            .map_err(|e| println!("{}", e))
            .unwrap_or_else(|_| Vec::new())
            .get(0)
            .cloned()
            .map(|p| p.into_inner().collect::<Vec<_>>())
            .unwrap_or_else(|| Vec::new());
        if let Some(p) = pairs.get(0) {
            match p.as_rule() {
                Rule::details => {
                    let inner = p.clone().into_inner().collect::<Vec<_>>();
                    command = Command::Details(inner.get(0).unwrap().as_str().to_owned())
                }
                Rule::implies => {
                    let mut inner = p.clone().into_inner().collect::<Vec<_>>();
                    let car = inner.get(0).unwrap().clone();
                    let choice_rule = if car.as_rule() == Rule::quantifier {
                        inner.remove(0);
                        car.into_inner().next().unwrap().as_rule() == Rule::some
                    } else {
                        false
                    };
                    let term = inner.get(0).unwrap().clone().as_str();
                    let mut cadr = inner
                        .get(1)
                        .unwrap()
                        .clone()
                        .into_inner()
                        .collect::<Vec<_>>();
                    let caadr = cadr.get(0).unwrap();
                    let neg = if caadr.as_rule() == Rule::not {
                        cadr.remove(0);
                        true
                    } else {
                        false
                    };
                    let expr = cadr.get(0).unwrap().as_str();
                    let mut mem = mem_tex.0.lock().unwrap();
                    mem.terms.push(term.to_owned());
                    mem.terms.push(expr.to_owned());
                    drop(mem);
                    mem_tex.1.notify_all();
                    command = Command::AddRule(if neg {
                        format!(":- {}(A), {}(A).", term, expr)
                    } else {
                        if choice_rule {
                            format!("0 {{ {}(A) }} 1 :- {}(A).", expr, term)
                        } else {
                            format!("{}(A) :- {}(A).", expr, term)
                        }
                    });
                }
                Rule::exit => {
                    println!("Goodbye!");
                    std::process::exit(0);
                }
                _ => (),
            }
        }

        match command {
            Command::Details(a) => {
                let a = a.to_plural();
                let memory = mem_tex.0.lock().unwrap();
                let mut ctl = clingo::Control::new(vec!["-n 0".to_owned()])?;
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
            Command::AddRule(rule_to_add) => {
                let mut memory = mem_tex.0.lock().unwrap();
                let mut ctl = clingo::Control::new(Vec::new())?;
                let mut prog = memory.rules.join("\n");
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
            Command::None => {
                println!("I didn't understand that.");
            }
            _ => (),
        }
        user_input = String::new();
    }
}
