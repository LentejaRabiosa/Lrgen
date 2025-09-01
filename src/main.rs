use std::{
    collections::{BTreeSet, HashMap},
    rc::Rc,
    usize,
};

type SymbolId = usize;
type Rhs = Rc<Vec<SymbolId>>;

#[derive(Clone, Hash, PartialEq, Eq)]
enum Symbol {
    Terminal(String),
    Nonterminal(String),
}

// TODO might be better to separate terminals and nonterminals in different collections
struct Symbols {
    collection: Vec<Rc<Symbol>>,
    index: HashMap<Rc<Symbol>, SymbolId>,
}

impl Symbols {
    fn new() -> Symbols {
        Symbols {
            collection: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn add_symbol(&mut self, symbol: Symbol) -> SymbolId {
        match self.index.get(&symbol) {
            Some(&id) => id,
            None => {
                let new_symbol = Rc::new(symbol);
                let id = self.collection.len();
                self.index.insert(Rc::clone(&new_symbol), id);
                self.collection.push(new_symbol);
                id
            }
        }
    }

    fn is_terminal(&self, symbol_id: SymbolId) -> bool {
        let symbol = match self.collection.get(symbol_id) {
            Some(symbol) => symbol.as_ref(),
            None => return false,
        };

        matches!(symbol, Symbol::Terminal(_))
    }

    fn is_nonterminal(&self, symbol_id: SymbolId) -> bool {
        let symbol = match self.collection.get(symbol_id) {
            Some(symbol) => symbol.as_ref(),
            None => return false,
        };

        matches!(symbol, Symbol::Nonterminal(_))
    }
}

struct Rule {
    lhs: Symbol,
    rhs: Vec<Symbol>,
}

impl Rule {
    fn new(lhs: Symbol, rhs: Symbol) -> Self {
        Rule {
            lhs,
            rhs: Vec::from([rhs]),
        }
    }

    fn rhs(mut self, rhs: Symbol) -> Self {
        self.rhs.push(rhs);
        self
    }
}

#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuleId {
    lhs: SymbolId,
    rhs: Rc<Vec<SymbolId>>,
}

#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    rule: RuleId,
    lookahead: SymbolId,
    position: usize,
}

impl Item {
    fn advanced(&self) -> Self {
        let mut new_item = self.clone();
        new_item.position += 1;
        new_item
    }

    fn next_symbol(&self) -> Option<SymbolId> {
        if let Some(symbol) = self.rule.rhs.get(self.position) {
            return Some(*symbol);
        }

        None
    }
}

struct Grammar {
    symbols: Symbols,
    rules: Vec<Vec<Rhs>>,
}

impl Grammar {
    fn new() -> Self {
        Grammar {
            symbols: Symbols::new(),
            rules: Vec::new(),
        }
    }

    fn add_rule(&mut self, rule: Rule) -> RuleId {
        let lhs = self.symbols.add_symbol(rule.lhs);
        let rhs: Rc<Vec<SymbolId>> = Rc::new(
            rule.rhs
                .into_iter()
                .map(|rhs| self.symbols.add_symbol(rhs))
                .collect(),
        );

        if let Some(rules) = self.rules.get_mut(lhs) {
            rules.push(Rc::clone(&rhs));
        } else {
            self.rules.insert(lhs, Vec::from([Rc::clone(&rhs)]));
        }

        RuleId { lhs, rhs }
    }

    fn lhs_rules(&self, lhs: SymbolId) -> Vec<RuleId> {
        self.rules[lhs]
            .iter()
            .map(|rhs| RuleId {
                lhs,
                rhs: Rc::clone(rhs),
            })
            .collect()
    }

    fn lookahead(&self, item: &Item) -> SymbolId {
        for &symbol in item.rule.rhs[item.position + 1..].iter() {
            if self.symbols.is_terminal(symbol) {
                return symbol;
            }
        }

        item.lookahead
    }

    // [S' -> · EXPRESSION, $]
    fn closure(&self, mut set: BTreeSet<Item>) -> BTreeSet<Item> {
        let mut to_close: Vec<Item> = set.iter().cloned().collect();

        while let Some(item_to_close) = to_close.pop() {
            let next_symbol = match item_to_close.next_symbol() {
                Some(symbol) => symbol,
                None => continue,
            };

            if !self.symbols.is_nonterminal(next_symbol) {
                continue;
            }

            for rule in self.lhs_rules(next_symbol) {
                let new_item = Item {
                    rule,
                    position: 0,
                    lookahead: item_to_close.lookahead,
                };

                if set.insert(new_item.clone()) {
                    to_close.push(new_item);
                }
            }
        }

        set
    }

    fn render_set(&self, set: &BTreeSet<Item>) {}

    fn build(&mut self, start: Symbol) {
        let rule = self.add_rule(Rule::new(Symbol::Nonterminal("S'".to_string()), start));
        let lookahead = self.symbols.add_symbol(Symbol::Terminal("$".to_string()));
        let start_production = Item {
            rule,
            position: 0,
            lookahead,
        };

        // let mut states: HashMap<BTreeSet<Item>, usize> = HashMap::new();
        // let mut states_stack: Vec<BTreeSet<Item>> =
        //     Vec::from([self.closure(BTreeSet::from([start]))]);
        //
        // while let Some(set) = states_stack.pop() {
        //     self.render_set(&set);
        //     let mut new_states: HashMap<SymbolId, BTreeSet<Item>> = HashMap::new();
        //     for item in &set {
        //         let next_symbol = match item.next_symbol() {
        //             Some(symbol) => symbol,
        //             None => continue,
        //         };
        //
        //         let new_item = item.advanced();
        //
        //         if let Some(new_set) = new_states.get_mut(&next_symbol) {
        //             new_set.insert(new_item);
        //         } else {
        //             let new_set = BTreeSet::from([new_item]);
        //             new_states.insert(next_symbol, new_set);
        //         }
        //     }
        //
        //     if !states.contains_key(&set) {
        //         states.insert(set, states.len());
        //     }
        //
        //     for (_, mut new_set) in new_states {
        //         new_set = self.closure(new_set);
        //         states_stack.push(new_set);
        //     }
        // }
    }
}

fn main() {
    println!("lr 1 generator");

    let mut grammar = Grammar::new();

    grammar.add_rule(
        Rule::new(
            Symbol::Nonterminal("EXPRESSION".to_string()),
            Symbol::Nonterminal("EXPRESSION".to_string()),
        )
        .rhs(Symbol::Terminal("plus".to_string()))
        .rhs(Symbol::Nonterminal("TERM".to_string())),
    );
    grammar.add_rule(Rule::new(
        Symbol::Nonterminal("EXPRESSION".to_string()),
        Symbol::Nonterminal("TERM".to_string()),
    ));
    grammar.add_rule(Rule::new(
        Symbol::Nonterminal("TERM".to_string()),
        Symbol::Terminal("number".to_string()),
    ));

    // grammar.build(start_production);
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
