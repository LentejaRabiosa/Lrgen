use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
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
            Action::Shift(state) => format!("Shift({})", state),
            Action::Reduce => format!("Reduce"),
            _ => "None".to_string(),
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
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

    fn advanced(&self) -> Item<'a> {
        let mut new_item = self.clone();
        new_item.position += 1;
        new_item
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
        println!(
            "[{} -> {}, {}]",
            self.symbol,
            symbols.join(" "),
            self.lookahead
        );
    }
}

fn print_states(states: &HashMap<BTreeSet<Item>, usize>) {
    println!("\nSTATES");
    for (state, index) in states {
        println!("\nstate {}", index);
        for item in state {
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

    let mut actions: Vec<HashMap<&str, Action>> = Vec::new();
    let mut states: HashMap<BTreeSet<Item>, usize> = HashMap::new();
    let mut states_stack: VecDeque<BTreeSet<Item>> = VecDeque::from([BTreeSet::from([start_item])]);

    while let Some(mut state) = states_stack.pop_front() {
        let mut to_close: Vec<Item> = state.iter().cloned().collect(); // can't be reference :(

        // close state
        while let Some(item) = to_close.pop() {
            if let Some(extension) = item.extended_lookahead(&grammar.symbols) {
                if let Some(rules) = grammar.rules.get(extension.symbol) {
                    for rule in rules {
                        let new_item = Item::new(extension.symbol, rule, extension.lookahead);
                        if state.insert(new_item.clone()) {
                            to_close.push(new_item);
                        }
                    }
                }
            }
        }

        // process state
        let mut new_states: HashMap<&str, BTreeSet<Item>> = HashMap::new();
        let mut new_actions: HashMap<&str, Action> = HashMap::new();
        for item in &state {
            if let Some(next_symbol) = item.derivation.get(item.position) {
                let new_item = item.advanced();

                if let Some(new_state) = new_states.get_mut(next_symbol.as_str()) {
                    new_state.insert(new_item);
                } else {
                    let new_state = BTreeSet::from([new_item]);
                    new_states.insert(next_symbol, new_state);
                }
            } else {
                new_actions.insert(item.lookahead, Action::Reduce);
            }
        }

        // insert states and actions
        if !states.contains_key(&state) {
            states.insert(state, states.len());
        }

        for (symbol, state) in new_states {
            let index = match states.get(&state) {
                Some(existing_state) => *existing_state,
                None => {
                    states_stack.push_back(state);
                    states.len() + states_stack.len() - 1
                },
            };

            if grammar.symbols.non_terminal.contains(symbol) {
                new_actions.insert(symbol, Action::Goto(index));
            } else {
                new_actions.insert(symbol, Action::Shift(index));
            }
        }

        actions.push(new_actions);
    }

    print_states(&states);
    print_actions(&actions);
}
