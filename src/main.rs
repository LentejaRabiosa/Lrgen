use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    env,
    fs::File,
    io::{BufRead, BufReader, Lines},
    isize,
    iter::Peekable,
    str::Chars,
    usize,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Symbol {
    Terminal(String),
    NonTerminal(String),
}

impl Symbol {
    fn get_type(&self) -> bool {
        match self {
            Symbol::Terminal(_) => true,
            Symbol::NonTerminal(_) => false,
        }
    }

    // fn print(&self) {
    //     match self {
    //         Symbol::Terminal(symbol) => println!("terminal({symbol})"),
    //         Symbol::NonTerminal(symbol) => println!("non_terminal({symbol})"),
    //     }
    // }
}

fn parse_symbol(chars: &mut Peekable<Chars>) -> Result<Symbol, ()> {
    let mut symbol = String::new();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_whitespace() {
            if !symbol.is_empty() {
                break;
            }

            continue;
        }

        symbol.push(ch);
    }

    if !symbol.is_empty() {
        if symbol.chars().all(|ch| ch.is_ascii_uppercase()) {
            return Ok(Symbol::NonTerminal(symbol));
        } else if symbol.chars().all(|ch| ch.is_ascii_lowercase()) {
            return Ok(Symbol::Terminal(symbol));
        } else {
            eprintln!("error: bad symbol");
        }
    }

    Err(())
}

#[derive(Hash, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Rule {
    rhs: Vec<usize>,
    rule_number: usize,
}

struct Extension {
    symbol: usize,
    lookahead: usize,
}

#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Item<'a> {
    lhr: usize,
    rule: &'a Rule,
    lookahead: usize,
    position: usize,
}

impl<'a> Item<'a> {
    fn advance(&self) -> Self {
        let mut new_item = self.clone();
        new_item.position += 1;
        new_item
    }

    fn next_symbol(&self) -> Option<usize> {
        self.rule.rhs.get(self.position).copied()
    }

    fn extended_lookahead(&self, terminals: &Vec<bool>) -> Option<Extension> {
        if let Some(next_symbol) = self.next_symbol() {
            if terminals[next_symbol] {
                // TODO this is wrong because the queue might include non terminals
                let mut terminals_queue: Vec<&usize> =
                    self.rule.rhs[self.position + 1..].iter().collect();
                terminals_queue.push(&self.lookahead);

                return Some(Extension {
                    symbol: next_symbol,
                    lookahead: **terminals_queue.first().unwrap(),
                });
            }
        }

        None
    }
}

#[derive(Debug)]
struct Grammar {
    symbols_type: Vec<bool>, // true -> terminal, false -> non terminal
    symbols: BTreeMap<Symbol, usize>,
    rules: BTreeMap<usize, Vec<Rule>>,
    rules_number: usize,
}

impl Grammar {
    fn print_rules(&self) {
        println!("RULES");
        for (lhs, rules) in &self.rules {
            for rule in rules {
                print!("{lhs} >");
                for symbol in &rule.rhs {
                    print!(" {symbol}");
                }
                println!();
            }
        }
    }

    fn new(mut lines: Lines<BufReader<File>>) -> Result<Self, ()> {
        let mut grammar = Grammar {
            symbols_type: Vec::new(),
            symbols: BTreeMap::new(),
            rules: BTreeMap::new(),
            rules_number: 0,
        };

        while let Some(Ok(line)) = lines.next() {
            let mut chars = line.chars().peekable();

            let lhs = match parse_symbol(&mut chars) {
                Ok(symbol) => symbol,
                Err(_) => return Err(()),
            };

            if let Symbol::Terminal(_) = lhs {
                eprintln!("error: expected non terminal");
                return Err(());
            }

            let arrow = parse_arrow(&mut chars);
            if !arrow {
                eprintln!("error: missing arrow after lhs");
                return Err(());
            }

            let mut rhs: Vec<Symbol> = Vec::new();
            while let Ok(symbol) = parse_symbol(&mut chars) {
                rhs.push(symbol);
            }

            if rhs.is_empty() {
                eprintln!("error: expected rhs");
                return Err(());
            }

            // println!("{:?}", lhs);
            // println!("{}", arrow);
            // println!("{:?}", rhs);

            if !grammar.symbols.contains_key(&lhs) {
                grammar.symbols_type.push(lhs.get_type());
                grammar.symbols.insert(lhs.clone(), grammar.symbols.len());
            }

            let lhs_index = *grammar.symbols.get(&lhs).unwrap();
            let rules = match grammar.rules.get_mut(&lhs_index) {
                Some(rules) => rules,
                None => {
                    grammar.rules.insert(lhs_index, Vec::new());
                    grammar.rules.get_mut(&lhs_index).unwrap()
                }
            };

            let mut rhs_indexes: Vec<usize> = Vec::new();
            for symbol in rhs {
                if !grammar.symbols.contains_key(&symbol) {
                    grammar.symbols_type.push(symbol.get_type());
                    grammar
                        .symbols
                        .insert(symbol.clone(), grammar.symbols.len());
                }

                rhs_indexes.push(*grammar.symbols.get(&symbol).unwrap());
            }

            rules.push(Rule {
                rhs: rhs_indexes,
                rule_number: grammar.rules_number,
            });
            grammar.rules_number += 1;
        }

        Ok(grammar)
    }

    fn closure<'a>(&'a self, mut set: BTreeSet<Item<'a>>) -> BTreeSet<Item<'a>> {
        let mut to_close: Vec<Item> = set.iter().cloned().collect();

        while let Some(item) = to_close.pop() {
            if let Some(extension) = item.extended_lookahead(&self.symbols_type) {
                if let Some(rules) = self.rules.get(&extension.symbol) {
                    for rule in rules {
                        let new_item = Item {
                            lhr: extension.symbol,
                            rule,
                            lookahead: extension.lookahead,
                            position: 0,
                        };

                        if set.insert(new_item.clone()) {
                            to_close.push(new_item);
                        }
                    }
                }
            }
        }

        set
    }

    fn successors<'a>(&self, set: &BTreeSet<Item<'a>>) -> HashMap<usize, BTreeSet<Item<'a>>> {
        let mut new_states: HashMap<usize, BTreeSet<Item>> = HashMap::new();

        for item in set {
            let next_symbol = match item.next_symbol() {
                Some(symbol) => symbol,
                None => continue,
            };

            let new_item = item.advance();
            if let Some(new_state) = new_states.get_mut(&next_symbol) {
                new_state.insert(new_item);
            } else {
                new_states.insert(next_symbol, BTreeSet::from([new_item]));
            }
        }

        new_states
    }

    fn actions<'a>(&self, set: &BTreeSet<Item<'a>>) {
        for item in set {

        }
    }
}

#[derive(Debug)]
struct Tables {
    yyr1: Vec<usize>,
    yyr2: Vec<usize>,
    yytable: Vec<isize>,
}

impl Tables {
    fn print(&self) {
        println!("yyr1: {:?}", self.yyr1);
        println!("yyr2: {:?}", self.yyr2);
    }
}


fn build_tables(grammar: Grammar) -> Tables {
    let mut tables = Tables {
        yyr1: Vec::new(),
        yyr2: Vec::new(),
        yytable: Vec::new(),
    };

    tables.yyr1.reserve(grammar.rules_number);
    tables.yyr2.reserve(grammar.rules_number);

    for (&lhs, rules) in &grammar.rules {
        for rule in rules {
            tables.yyr1.push(lhs);
            tables.yyr2.push(rule.rhs.len());
        }
    }

    let mut states: HashMap<BTreeSet<Item>, usize> = HashMap::new();
    let mut states_stack: Vec<BTreeSet<Item>> = Vec::new(); // State { BtreeSet<Item>, index } ?
    while let Some(set) = states_stack.pop() {
        let state_number = states.len();
        let new_states = grammar.successors(&set);
        if !states.contains_key(&set) {
            states.insert(set, state_number);
        }
        
        for new_state in new_states {

        }
    }

    tables
}

fn parse_arrow(chars: &mut Peekable<Chars>) -> bool {
    let mut arrow = false;
    while let Some(ch) = chars.next() {
        if ch.is_ascii_whitespace() {
            if arrow {
                return true;
            }
        } else if ch == '>' {
            arrow = true;
        }
    }

    false
}

fn main() {
    println!("lr 1 generator");
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("usage: {} <grammar>", args[0]);
        return;
    }

    let lines = match File::open(&args[1]) {
        Ok(file) => BufReader::new(file).lines(),
        Err(_) => {
            eprintln!("error: no file {}", args[1]);
            return;
        }
    };

    let grammar = match Grammar::new(lines) {
        Ok(grammar) => grammar,
        Err(_) => return,
    };

    let tables = build_tables(grammar);
    tables.print();
}

// fn main() {
//     while let Some(state) = states_stack.pop_front() {
//         let current_state_index = states.len();
//
//         // create new states (not completed states)
//         let mut new_states: HashMap<&str, BTreeSet<Set>> = HashMap::new();
//         for item in &state {
//             let next_symbol = match item.derivation.get(item.position) {
//                 Some(symbol) => symbol,
//                 None => continue,
//             };
//
//             let new_item = item.advanced();
//
//             if let Some(new_state) = new_states.get_mut(next_symbol) {
//                 new_state.insert(new_item);
//             } else {
//                 let new_state = BTreeSet::from([new_item]);
//                 new_states.insert(next_symbol, new_state);
//             }
//
//             // else {
//             //     start_item.position = start_item.derivation.len();
//             //     if state.contains(&start_item) && item.lookahead == "$" {
//             //         new_actions.insert(item.lookahead, Action::Accept);
//             //         yytable.push(0);
//             //         yycheck.push(current_state_index);
//             //     } else {
//             //         new_actions.insert(item.lookahead, Action::Reduce(item.rule));
//             //         if let (Some(&last_yytable), Some(&last_yycheck)) = (yytable.last(), yycheck.last()) {
//             //             if last_yytable != -(item.rule as isize) || last_yycheck != current_state_index {
//             //                 yytable.push(-(item.rule as isize));
//             //                 yycheck.push(current_state_index);
//             //             }
//             //         }
//             //     }
//             //
//             // }
//         }
//
//         // insert the current state
//         if !states.contains_key(&state) {
//             states.insert(state, current_state_index);
//         }
//
//         // create new actions
//         let mut new_actions: HashMap<&str, Action> = HashMap::new();
//         for (symbol, mut state) in new_states {
//             state = close_items(state, &symbols, &grammar);
//
//             let index = match states.get(&state) {
//                 Some(existing_state) => *existing_state,
//                 None => {
//                     states_stack.push_back(state);
//                     states.len() + states_stack.len() - 1
//                 }
//             };
//
//             if symbols.non_terminals.contains(symbol) {
//                 new_actions.insert(symbol, Action::Goto(index));
//             } else {
//                 new_actions.insert(symbol, Action::Shift(index));
//             }
//
//             // if let (Some(&last_yytable), Some(&last_yycheck)) = (yytable.last(), yycheck.last()) {
//             //     if last_yytable != index as isize || last_yycheck != current_status_index {
//             //         yytable.push(index as isize);
//             //         yycheck.push(current_status_index);
//             //     }
//             // }
//
//             yytable.push(index as isize);
//             yycheck.push(current_state_index);
//         }
//
//         actions.push(new_actions);
//     }
// }
