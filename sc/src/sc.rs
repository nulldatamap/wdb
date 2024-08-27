use super::parser::*;

#[derive(Debug, PartialEq, Eq, Clone)]
struct Symbol {
    symbol: String,
}

#[derive(Debug)]
struct Rule {
    name: String,
    pattern: Vec<Symbol>,
    result: Vec<Symbol>
}

impl Rule {
    fn apply(&self, w: &mut Word) {
        let mut matches = Vec::new();
        let mut new = Vec::new();
        // Find all matches
        'outer: for i in 0..(w.symbols.len() - self.patten.len())  {
            for j in 0..self.pattern.len() {
                if self.pattern[j] != w.symbols[i + j] { break 'outer }
            }
            matches.push((i, i + self.pattern.len(), &self.result[..]));
        }

        // Filter overlapping matches
        let last_end = 0;
        matches.retain(|(start, end, _)| {
            if last_end > start {
                return false
            }
            last_end = end;
        });

        // Build a new word from the remaining matches
        let n = w.symbols.len();
        let mut head = 0;
        let mut symbols = w.symbols.drain();
        for (start, end, content) in matches.into_iter() {
            if head < start {
                for _ in 0..(start - head) {
                    new.push(symbols.next().unwrap());
                    head += 1;
                }
            }
            assert!(head == start);
            new.append(content);
            head = end;
        }
        w.symbols = new;
    }
}

#[derive(Debug, Clone)]
struct Word {
    symbols: Vec<Symbol>,
}

#[derive(Debug)]
struct Lexurgy {
    rules: Vec<Rule>,
}

impl Lexurgy {
    fn from_ast(ast: Vec<Stmt>) -> Lexurgy {
    }

    fn apply(&self, ws: &mut Vec<Word>) {
        for rule in self.rules {
            for word in ws.iter_mut() {
                rule.apply(&mut w);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rules() {
    }
}
