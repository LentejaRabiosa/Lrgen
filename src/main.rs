use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env, fs,
};

#[derive(Debug, Deserialize)]
struct Symbols {
    terminal: HashSet<String>,
    non_terminal: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct Grammar {
    symbols: Symbols,
    start: Vec<String>,
    rules: HashMap<String, Vec<Vec<String>>>,
}

struct ExtensionData<'a> {
    symbol: &'a str,
    lookahead: &'a str,
}

// TODO optimizable by separating the position? idk
// TODO some of this fields could be references
#[derive(Clone, Hash, Eq, PartialEq)]
struct Item<'a> {
    symbol: &'a str,
    derivation: &'a Vec<String>,
    position: usize,
    lookahead: &'a str,
}

impl<'a> Item<'a> {
    fn new(symbol: &'a str, derivation: &'a Vec<String>, lookahead: &'a str) -> Self {
        Item {
            symbol,
            derivation,
            position: 0,
            lookahead,
        }
    }

    fn extended_lookahead(&self, symbols: &'a Symbols) -> Option<ExtensionData<'a>> {
        let next_symbol = match self.derivation.get(self.position) {
            Some(symbol) => symbol,
            None => return None,
        };

        if !symbols.non_terminal.contains(next_symbol) {
            return None;
        }

        let mut terminal_symbols: Vec<&str> = Vec::new();
        for symbol in self.derivation[self.position + 1..].iter() {
            if symbols.terminal.contains(symbol) {
                terminal_symbols.push(symbol);
            }
        }

        terminal_symbols.push(self.lookahead);
        return Some(ExtensionData {
            symbol: next_symbol,
            lookahead: terminal_symbols.first().unwrap(),
        });
    }

    fn print(&self) {
        let mut symbols = self.derivation.clone();
        symbols.insert(self.position, "·".to_string());
        println!("[{}, {}]", symbols.join(" "), self.lookahead);
    }
}

struct State<'a> {
    index: usize,
    set: HashSet<Item<'a>>,
}

impl<'a> State<'a> {
    fn print(&self) {
        println!("state {}", self.index);
        for item in self.set.iter() {
            item.print();
        }
    }
}

#[derive(Debug)]
enum Action {
    Shift,
    Reduce,
    Goto(usize),
    Accept,
}

fn main() {
    println!("LR(1) Table Generator");

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("error: expected 1 single argument");
        return;
    }

    let grammar_file_name = &args[1];
    let grammar_yaml = fs::read_to_string(grammar_file_name).expect(&format!(
        "Should have been able to read the file '{}'",
        grammar_file_name
    ));

    let grammar: Grammar = serde_yaml::from_str(&grammar_yaml).expect("Bad grammar");
    let start_item = Item::new("S", &grammar.start, "$");

    let mut state_stack: Vec<State> = Vec::from([State {
        index: 0,
        set: HashSet::from([start_item]),
    }]);

    // TODO refactor, more functions
    loop {
        let mut state = match state_stack.pop() {
            Some(state) => state,
            None => break,
        };

        let mut to_close = state.set.clone();
        loop {
            let mut new_items: HashSet<Item> = HashSet::new();
            for item in to_close {
                let extension_data = match item.extended_lookahead(&grammar.symbols) {
                    Some(symbol) => symbol,
                    None => continue,
                };

                let matches = match grammar.rules.get(extension_data.symbol) {
                    Some(rules) => rules,
                    None => continue, // TODO idk
                };

                for rule in matches {
                    let new_item = Item::new(
                        extension_data.symbol,
                        rule,
                        extension_data.lookahead,
                    );

                    if state.set.contains(&new_item) {
                        continue;
                    }

                    new_items.insert(new_item);
                }
            }

            if new_items.is_empty() {
                break;
            }

            to_close = new_items.clone();
            state.set.extend(new_items);
        }

        state.print();
    }
}
