// WARNING: Este algoritmo prioriza la verticalidad y legibilidad del codigo frente al rendimiento.

use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    env, fs, process::exit,
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
    Shift,
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

impl<'a> From<&Item<'a>> for Item<'a> {
    fn from(item: &Item<'a>) -> Self {
        let mut new_item = item.clone();
        new_item.position += 1;
        new_item
    }
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
        println!("state {}", self.index);
        for item in self.set.iter() {
            item.print();
        }
    }
}

fn close_items<'a>(to_close: &HashSet<Item<'a>>, set: &HashSet<Item>, grammar: &'a Grammar) -> HashSet<Item<'a>> {
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
            let new_item = Item::new(extension_data.symbol, rule, extension_data.lookahead);

            if set.contains(&new_item) {
                continue;
            }

            new_items.insert(new_item);
        }
    }

    new_items
}

fn print_actions(actions: &Vec<HashMap<&str, Action>>) {
    println!("ACTIONS");
    for (index, map) in actions.iter().enumerate() {
        println!("state {}", index);
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

    loop {
        let mut state = match state_stack.pop() {
            Some(state) => state,
            None => break,
        };

        let mut to_close = state.set.clone();
        loop {
            let new_items = close_items(&to_close, &state.set, &grammar);
            if new_items.is_empty() {
                break;
            }

            to_close = new_items.clone();
            state.set.extend(new_items);
        }

        state.print();

        // TODO refactor
        let mut new_actions: HashMap<&str, Action> = HashMap::new();
        let mut new_states: HashMap<&str, State> = HashMap::new();
        let mut index = state.index;
        for item in &state.set {
            let next_symbol = match item.derivation.get(item.position) {
                Some(symbol) => symbol.as_str(),
                None => continue, // some action
            };

            if grammar.symbols.non_terminal.contains(next_symbol) {
                if let Some(new_state) = new_states.get_mut(next_symbol) {
                    new_state.set.insert(Item::from(item));
                } else {
                    index += 1;
                    let new_state = State {
                        index,
                        set: HashSet::from([Item::from(item)]),
                    };
                    new_states.insert(next_symbol, new_state);
                    new_actions.insert(next_symbol, Action::Goto(index));
                }
            }
        }

        actions.push(new_actions);
        state_stack.extend(new_states.values().cloned());
    }

    print_actions(&actions);
}
