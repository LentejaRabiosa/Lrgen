use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    usize,
};

type SymbolId = usize;
type Rhs = Vec<SymbolId>;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Symbol {
    Terminal(String),
    Nonterminal(String),
}

struct Symbols {
    collection: Vec<Symbol>,
    index: HashMap<Symbol, SymbolId>,
}

impl Symbols {
    fn new() -> Symbols {
        Symbols {
            collection: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn name(&self, symbol_id: SymbolId) -> &str {
        match &self.collection[symbol_id] {
            Symbol::Terminal(name) => name,
            Symbol::Nonterminal(name) => name,
        }
    }

    fn add_symbol(&mut self, symbol: Symbol) -> SymbolId {
        match self.index.get(&symbol) {
            Some(&id) => id,
            None => {
                let id = self.collection.len();
                self.index.insert(symbol.clone(), id);
                self.collection.push(symbol);
                id
            }
        }
    }

    fn is_terminal(&self, symbol_id: SymbolId) -> bool {
        let symbol = match self.collection.get(symbol_id) {
            Some(symbol) => symbol,
            None => return false,
        };

        matches!(symbol, Symbol::Terminal(_))
    }

    fn is_nonterminal(&self, symbol_id: SymbolId) -> bool {
        let symbol = match self.collection.get(symbol_id) {
            Some(symbol) => symbol,
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
    rhs: Vec<SymbolId>,
}

#[derive(Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    rule: RuleId,
    lookahead: SymbolId,
    position: usize,
}

impl Item {
    fn render(&self, symbols: &Symbols) {
        print!("[{} ->", symbols.name(self.rule.lhs));
        let mut rhs_names: Vec<&str> = self.rule.rhs.iter().map(|&rhs| symbols.name(rhs)).collect();
        rhs_names.insert(self.position, "·");
        for rhs in rhs_names {
            print!(" {}", rhs);
        }
        println!(", {}]", symbols.name(self.lookahead));
    }

    fn end(&self) -> bool {
        self.position >= self.rule.rhs.len()
    }

    fn lookahead(&self, symbols: &Symbols) -> SymbolId {
        for &symbol in self.rule.rhs[self.position + 1..].iter() {
            if symbols.is_terminal(symbol) {
                return symbol;
            }
        }

        self.lookahead
    }

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

fn get_new_states(set: &BTreeSet<Item>) -> HashMap<SymbolId, BTreeSet<Item>> {
    let mut new_states: HashMap<SymbolId, BTreeSet<Item>> = HashMap::new();

    for item in set {
        let next_symbol = match item.next_symbol() {
            Some(symbol) => symbol,
            None => continue,
        };

        let new_item = item.advanced();

        if let Some(new_set) = new_states.get_mut(&next_symbol) {
            new_set.insert(new_item);
        } else {
            let new_set = BTreeSet::from([new_item]);
            new_states.insert(next_symbol, new_set);
        }
    }

    new_states
}

fn render_actions(actions: &HashMap<SymbolId, Action>, symbols: &Symbols) {
    for (&symbol, action) in actions {
        action.render(symbols.name(symbol));
    }
}

fn render_states(
    states: &HashMap<BTreeSet<Item>, usize>,
    actions: &Vec<HashMap<SymbolId, Action>>,
    symbols: &Symbols,
) {
    for (set, &number) in states {
        println!("\n{number}");
        println!("--- items ---");
        for item in set {
            item.render(&symbols);
        }
        println!("--- actions ---");
        render_actions(&actions[number], symbols);
    }
}

enum Action {
    Goto(usize),
    Shift(usize),
    Reduce(usize, usize),
}

impl Action {
    fn render(&self, symbol: &str) {
        match self {
            Self::Goto(next_state) => println!("goto({symbol}, {next_state})"),
            Self::Shift(next_state) => println!("shift({symbol}, {next_state})"),
            Self::Reduce(rhs_len, lhs) => println!("reduce({symbol}, {rhs_len}, {lhs})"),
        }
    }
}

struct Grammar {
    symbols: Symbols,
    rules: HashMap<usize, Vec<Rhs>>,
    rules_lhs: Vec<usize>,
    rules_len: Vec<usize>,
}

impl Grammar {
    fn new() -> Self {
        Grammar {
            symbols: Symbols::new(),
            rules: HashMap::new(),
            rules_lhs: Vec::new(),
            rules_len: Vec::new(),
        }
    }

    fn add_rule(&mut self, rule: Rule) -> RuleId {
        let lhs = self.symbols.add_symbol(rule.lhs);
        let rhs: Vec<SymbolId> = rule
            .rhs
            .into_iter()
            .map(|rhs| self.symbols.add_symbol(rhs))
            .collect();

        if let Some(rules) = self.rules.get_mut(&lhs) {
            rules.push(rhs.clone());
        } else {
            self.rules.insert(lhs, Vec::from([rhs.clone()]));
        }

        self.rules_lhs.push(lhs);
        self.rules_len.push(rhs.len());

        RuleId { lhs, rhs }
    }

    fn get_rules_by_lhs(&self, lhs: SymbolId) -> Vec<RuleId> {
        self.rules[&lhs]
            .iter()
            .map(|rhs| RuleId {
                lhs,
                rhs: rhs.clone(),
            })
            .collect()
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

            let lookahead = item_to_close.lookahead(&self.symbols);
            for rule in self.get_rules_by_lhs(next_symbol) {
                let new_item = Item {
                    rule,
                    position: 0,
                    lookahead,
                };

                if set.insert(new_item.clone()) {
                    to_close.push(new_item);
                }
            }
        }

        set
    }

    fn compact(
        &self,
        states: &HashMap<BTreeSet<Item>, usize>,
        actions: &Vec<HashMap<SymbolId, Action>>,
    ) {
    }

    fn build(&mut self, start: Symbol) {
        let rule = self.add_rule(Rule::new(Symbol::Nonterminal("S'".to_string()), start));
        let lookahead = self.symbols.add_symbol(Symbol::Terminal("$".to_string()));
        let start_production = Item {
            rule,
            position: 0,
            lookahead,
        };

        let mut states: HashMap<BTreeSet<Item>, usize> = HashMap::new();
        let mut states_stack: VecDeque<BTreeSet<Item>> =
            VecDeque::from([self.closure(BTreeSet::from([start_production]))]);

        let mut actions: Vec<HashMap<SymbolId, Action>> = Vec::new();

        while let Some(set) = states_stack.pop_front() {
            let mut new_actions: HashMap<SymbolId, Action> = HashMap::new();
            let new_states = get_new_states(&set);

            for item in &set {
                if item.end() {
                    new_actions.insert(
                        item.lookahead,
                        Action::Reduce(item.rule.rhs.len(), item.rule.lhs),
                    );
                }
            }

            for (symbol_id, mut new_set) in new_states {
                new_set = self.closure(new_set);

                let next_state = match states.get(&new_set) {
                    Some(&existing_state) => existing_state,
                    None => states.len() + states_stack.len() + 1,
                };

                let action = match self.symbols.collection[symbol_id] {
                    Symbol::Terminal(_) => Action::Shift(next_state),
                    Symbol::Nonterminal(_) => Action::Goto(next_state),
                };

                new_actions.insert(symbol_id, action);
                states_stack.push_back(new_set);
            }

            if !states.contains_key(&set) {
                states.insert(set, states.len());
                actions.push(new_actions);
            }
        }

        render_states(&states, &actions, &self.symbols);
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

    grammar.build(Symbol::Nonterminal("EXPRESSION".to_string()));

    println!("{:?}", grammar.rules_lhs);
    println!("{:?}", grammar.rules_len);
}
