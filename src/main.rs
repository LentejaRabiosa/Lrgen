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

#[derive(Hash, Eq, PartialEq)]
enum Action {
    Shift(usize),
    Reduce,
    Goto(usize),
    Accept,
    // None,
}

impl Action {
    fn text(&self) -> String {
        match self {
            Action::Goto(state) => format!("Goto({})", state),
            _ => "None".to_string(),
        }
    }
}

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
            Some(symbol) => symbol.as_str(),
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
        println!("[{} -> {}, {}]", self.symbol, symbols.join(" "), self.lookahead);
    }
}

#[derive(Clone)]
struct State<'a> {
    index: usize,
    set: HashSet<Item<'a>>,
}

impl<'a> State<'a> {
    fn print(&self) {
        println!("\nstate {}", self.index);
        for item in self.set.iter() {
            item.print();
        }
    }
}

fn print_actions(actions: &Vec<HashMap<&str, Action>>) {
    println!("\nACTIONS");
    for (index, map) in actions.iter().enumerate() {
        println!("\nstate {}", index);
        for (symbol, action) in map {
            println!("{} -> {}", symbol, action.text());
        }
    }
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
    let start_item = Item::new("'", &grammar.start, "$");

    // let mut action_table: Vec<HashMap<String, Action>> = Vec::new();
    // let mut goto_table: Vec<HashMap<String, usize>> = Vec::new();
    let mut actions: Vec<HashMap<&str, Action>> = Vec::new();

    let mut state_stack: Vec<State> = Vec::from([State {
        index: 0,
        set: HashSet::from([start_item]),
    }]);

    let mut index = 0;
    while let Some(mut state) = state_stack.pop() {
        let mut to_close: Vec<Item> = state.set.iter().cloned().collect();
        let mut new_actions: HashMap<&str, Action> = HashMap::new();
        let mut new_states: HashMap<&str, State> = HashMap::new();

        while let Some(item) = to_close.pop() {
            // process item
            if let Some(next_symbol) = item.derivation.get(item.position) {
                let mut new_item = item.clone();
                new_item.position += 1;

                if let Some(new_state) = new_states.get_mut(next_symbol.as_str()) {
                    new_state.set.insert(new_item);
                } else {
                    index += 1;
                    let new_state = State {
                        index,
                        set: HashSet::from([new_item]),
                    };
                    new_states.insert(next_symbol, new_state);
                }

                if grammar.symbols.non_terminal.contains(next_symbol) {
                    new_actions.insert(next_symbol, Action::Goto(index));
                } else {
                    new_actions.insert(next_symbol, Action::Shift(index));
                }
            }

            // extend item
            if let Some(extension) = item.extended_lookahead(&grammar.symbols) {
                if let Some(rules) = grammar.rules.get(extension.symbol) {
                    for rule in rules {
                        let new_item = Item::new(extension.symbol, rule, extension.lookahead);
                        if state.set.insert(new_item.clone()) {
                            to_close.push(new_item);
                        }
                    }
                }
            }
        }

        actions.push(new_actions);
        state_stack.extend(new_states.values().cloned());
        state.print();
    }

    print_actions(&actions);
}
