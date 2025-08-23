use clap::{Arg, Command, builder::styling};
use serde::Deserialize;
use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque}, fs, isize, u8, usize
};

enum SymbolType {
    Terminal,
    NonTerminal,
    Error,
}

fn get_symbol_type(symbol: &str) -> SymbolType {
    let mut uppercase: i8 = 0;
    let mut lowercase: i8 = 0;

    for ch in symbol.chars() {
        if ch == '_' {
            continue;
        } else if ch.is_uppercase() {
            uppercase += 1;
        } else if ch.is_lowercase() {
            lowercase += 1;
        }
    }

    let diff = uppercase - lowercase;
    if diff == uppercase {
        return SymbolType::NonTerminal;
    } else if diff == -lowercase {
        return SymbolType::Terminal;
    } else {
        return SymbolType::Error;
    }
}

#[derive(Debug, Deserialize)]
struct Symbols {
    terminals: HashSet<String>,
    non_terminals: HashSet<String>,
}

impl Symbols {
    fn from_raw_grammar_data(raw_grammar_data: &RawGrammarData) -> Self {
        let mut symbols = Symbols {
            terminals: HashSet::new(),
            non_terminals: HashSet::new(),
        };

        for (symbol, rules) in &raw_grammar_data.rules {
            symbols.add_symbol(symbol.to_string(), true);
            for derivation in rules {
                for symbol in derivation {
                    symbols.add_symbol(symbol.to_string(), false);
                }
            }
        }

        symbols
    }

    // TODO refactor
    fn add_symbol(&mut self, symbol: String, non_terminal_expected: bool) {
        match (
            self.terminals.contains(&symbol),
            self.non_terminals.contains(&symbol),
        ) {
            (false, false) => {
                match get_symbol_type(&symbol) {
                    SymbolType::Terminal => {
                        if non_terminal_expected {
                            panic!("LHSs must be non terminal");
                        }
                        self.terminals.insert(symbol);
                    }
                    SymbolType::NonTerminal => {
                        self.non_terminals.insert(symbol);
                    }
                    _ => panic!("bad symbol in the grammar"),
                };
            }
            (true, true) => panic!("terminal and non terminal at the same time (not possible lol)"),
            _ => {}
        };
    }

    fn get_symbol(&self, symbol: String) -> &str {
        match (self.terminals.get(&symbol), self.non_terminals.get(&symbol)) {
            (Some(terminal), None) => terminal,
            (None, Some(non_terminal)) => non_terminal,
            _ => panic!("something went wrong adding those symbols"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawGrammarData {
    start: String,
    rules: HashMap<String, Vec<Vec<String>>>,
}

#[derive(Debug)]
struct Grammar<'a> {
    start: String,
    rules: HashMap<&'a str, Vec<(usize, Vec<&'a str>)>>,
    number_of_rules: usize,
}

impl<'a> Grammar<'a> {
    fn from_raw_grammar_data(raw_grammar_data: &RawGrammarData, symbols: &'a Symbols) -> Self {
        let mut grammar = Grammar {
            start: raw_grammar_data.start.to_string(),
            rules: HashMap::new(),
            number_of_rules: 0,
        };

        let mut flatten_rules: Vec<(&str, Vec<&str>)> = Vec::new();
        for (symbol, rules) in &raw_grammar_data.rules {
            let symbol_ref = symbols.get_symbol(symbol.to_string());
            for derivation in rules {
                let mut derivation_refs: Vec<&str> = Vec::new();
                for symbol in derivation {
                    derivation_refs.push(symbols.get_symbol(symbol.to_string()));
                }
                flatten_rules.push((symbol_ref, derivation_refs));
            }
        }

        grammar.number_of_rules = flatten_rules.len();

        for (index, (lhs, rhs)) in flatten_rules.into_iter().enumerate() {
            if let Some(rules) = grammar.rules.get_mut(&lhs) {
                rules.push((index + 1, rhs));
            } else {
                grammar.rules.insert(lhs, Vec::from([(index + 1, rhs)]));
            }
        }

        grammar
    }
}

struct ExtensionData<'a> {
    symbol: &'a str,
    lookahead: &'a str,
}

#[derive(Hash, Eq, PartialEq)]
enum Action {
    Shift(usize),
    Reduce(usize),
    Goto(usize),
    Accept,
    // None,
}

impl Action {
    fn text(&self) -> String {
        match self {
            Action::Goto(state) => format!("Goto({})", state),
            Action::Shift(state) => format!("Shift({})", state),
            Action::Reduce(state) => format!("Reduce({})", state),
            Action::Accept => format!("Accept"),
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct Item<'a> {
    symbol: &'a str,
    derivation: &'a Vec<&'a str>,
    position: usize,
    lookahead: &'a str,
    rule: usize,
}

impl<'a> Item<'a> {
    fn new(symbol: &'a str, derivation: &'a Vec<&'a str>, lookahead: &'a str, rule: usize) -> Self {
        Item {
            symbol,
            derivation,
            position: 0,
            lookahead,
            rule,
        }
    }

    fn advanced(&self) -> Item<'a> {
        let mut new_item = self.clone();
        new_item.position += 1;
        new_item
    }

    fn extended_lookahead(&self, symbols: &'a Symbols) -> Option<ExtensionData<'a>> {
        let &next_symbol = match self.derivation.get(self.position) {
            Some(symbol) => symbol,
            None => return None,
        };

        if !symbols.non_terminals.contains(next_symbol) {
            return None;
        }

        let mut terminal_symbols: Vec<&str> = Vec::new();
        for &symbol in self.derivation[self.position + 1..].iter() {
            if symbols.terminals.contains(symbol) {
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
        symbols.insert(self.position, "·");
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
        println!(" * state {}", index);
        for item in state {
            print!("   ");
            item.print();
        }
    }
}

fn print_actions(actions: &Vec<HashMap<&str, Action>>) {
    println!("\nACTIONS");
    for (index, map) in actions.iter().enumerate() {
        println!(" * state {}", index);
        for (symbol, action) in map {
            println!("   {} -> {}", symbol, action.text());
        }
    }
}

fn close_items<'a>(mut items: BTreeSet<Item<'a>>, symbols: &'a Symbols, grammar: &'a Grammar) -> BTreeSet<Item<'a>> {
    let mut to_close: Vec<Item> = items.iter().cloned().collect();

    while let Some(item) = to_close.pop() {
        if let Some(extension) = item.extended_lookahead(symbols) {
            if let Some(rules) = grammar.rules.get(extension.symbol) {
                for (index, derivation) in rules {
                    let new_item =
                        Item::new(extension.symbol, derivation, extension.lookahead, *index);
                    if items.insert(new_item.clone()) {
                        to_close.push(new_item);
                    }
                }
            }
        }
    }

    items
}

// TODO refactor
fn main() {
    // println!("LR(1) Table Generator");

    const STYLES: styling::Styles = styling::Styles::styled()
        .header(styling::AnsiColor::Green.on_default().bold())
        .usage(styling::AnsiColor::Green.on_default().bold())
        .literal(styling::AnsiColor::Blue.on_default().bold())
        .placeholder(styling::AnsiColor::Cyan.on_default());

    let mut args = Command::new("lrgen")
        .author("Alejandro")
        .about("LR(1) Table Generator")
        .arg(Arg::new("grammar").help("YAML file").required(true))
        .arg(
            Arg::new("output")
                .help("Output file")
                .short('o')
                .long("output")
                .require_equals(true),
        )
        .styles(STYLES);

    // TODO context
    let grammar_file_error = args.error(clap::error::ErrorKind::ValueValidation, "bad file name");
    // grammar_file_error.insert(clap::error::ContextKind::InvalidValue, clap::error::ContextValue::String("grammar".to_owned()));

    let matches = args.get_matches();

    let grammar_file = matches
        .get_one::<String>("grammar")
        .expect("something went wrong :(");
    let grammar_yaml = match fs::read_to_string(&grammar_file) {
        Ok(data) => data,
        Err(_) => grammar_file_error.exit(),
    };

    let raw_grammar_data: RawGrammarData =
        serde_yaml::from_str(&grammar_yaml).expect("Bad grammar");
    let symbols = Symbols::from_raw_grammar_data(&raw_grammar_data);
    let grammar: Grammar = Grammar::from_raw_grammar_data(&raw_grammar_data, &symbols);

    let first_derivation = Vec::from([grammar.start.as_str()]);
    let mut start_item = Item::new("'", &first_derivation, "$", 0);
    let first_state = close_items(BTreeSet::from([start_item.clone()]), &symbols, &grammar);

    let mut actions: Vec<HashMap<&str, Action>> = Vec::new();
    let mut states: HashMap<BTreeSet<Item>, usize> = HashMap::new();
    let mut states_stack: VecDeque<BTreeSet<Item>> = VecDeque::from([first_state]);

    // let mut symbols_enum: Vec<&str> = Vec::from(["$"]);
    let mut yyr1: Vec<usize> = Vec::new();
    let mut yyr2: Vec<usize> = Vec::new();
    let mut yydefact: Vec<usize> = Vec::new();
    let mut yybase: Vec<usize> = Vec::new();
    let mut yygoto: Vec<usize> = Vec::new();
    let mut yytable: Vec<isize> = Vec::new();
    let mut yycheck: Vec<usize> = Vec::new();

    for (lhs_number, (_, derivations)) in grammar.rules.iter().enumerate() {
        for (_, rhs) in derivations {
            yyr1.push(lhs_number);
            yyr2.push(rhs.len());
        }
    }

    while let Some(state) = states_stack.pop_front() {
        let current_state_index = states.len();

        // process state
        let mut new_states: HashMap<&str, BTreeSet<Item>> = HashMap::new();
        let mut new_actions: HashMap<&str, Action> = HashMap::new();
        for item in &state {
            if let Some(next_symbol) = item.derivation.get(item.position) {
                let new_item = item.advanced();

                if let Some(new_state) = new_states.get_mut(next_symbol) {
                    new_state.insert(new_item);
                } else {
                    let new_state = BTreeSet::from([new_item]);
                    new_states.insert(next_symbol, new_state);
                }
            } else {
                start_item.position = start_item.derivation.len();
                if state.contains(&start_item) && item.lookahead == "$" {
                    new_actions.insert(item.lookahead, Action::Accept);
                    yytable.push(0);
                    yycheck.push(current_state_index);
                } else {
                    new_actions.insert(item.lookahead, Action::Reduce(item.rule));
                    if let (Some(&last_yytable), Some(&last_yycheck)) = (yytable.last(), yycheck.last()) {
                        if last_yytable != -(item.rule as isize) || last_yycheck != current_state_index {
                            yytable.push(-(item.rule as isize));
                            yycheck.push(current_state_index);
                        }
                    }
                }

            }
        }

        // insert states and actions
        if !states.contains_key(&state) {
            states.insert(state, current_state_index);
        }

        for (symbol, mut state) in new_states {
            state = close_items(state, &symbols, &grammar);
            let index = match states.get(&state) {
                Some(existing_state) => *existing_state,
                None => {
                    states_stack.push_back(state);
                    states.len() + states_stack.len() - 1
                }
            };

            if symbols.non_terminals.contains(symbol) {
                new_actions.insert(symbol, Action::Goto(index));
            } else {
                new_actions.insert(symbol, Action::Shift(index));
            }
            
            // if let (Some(&last_yytable), Some(&last_yycheck)) = (yytable.last(), yycheck.last()) {
            //     if last_yytable != index as isize || last_yycheck != current_status_index {
            //         yytable.push(index as isize);
            //         yycheck.push(current_status_index);
            //     }
            // }

            yytable.push(index as isize);
            yycheck.push(current_state_index);
        }

        actions.push(new_actions);
    }

    print_states(&states);
    print_actions(&actions);

    println!("{:?}", yyr1);
    println!("{:?}", yyr2);
    println!("{:?} len: {}", yytable, yytable.len());
    println!("{:?} len: {}", yycheck, yycheck.len());
}
