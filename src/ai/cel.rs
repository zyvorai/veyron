// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! The subset of CEL that VeyronPolicy rules use, for dry runs in the API.
//!
//! Supports literals (bool, int, uint, double, string, lists), variables, map
//! indexing and field selection, `!`, `&&`, `||`, comparisons, `in`, `+`/`-`,
//! `has(m.key)`, `size(x)`, and the string methods `startsWith`, `endsWith`,
//! `contains`, `matches`, `lowerAscii`, `upperAscii`, `size`. Numbers compare
//! across int/uint/double like the operator (`CrossTypeNumericComparisons`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    List(Vec<Val>),
    Map(BTreeMap<String, Val>),
}

pub type Env = BTreeMap<String, Val>;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() {
            let start = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            let s: String = c[start..i].iter().collect();
            if i < c.len() && (c[i] == 'u' || c[i] == 'U') {
                i += 1;
            }
            out.push(Tok::Num(s.parse().map_err(|_| format!("bad number {s}"))?));
            continue;
        }
        if ch == '\'' || ch == '"' {
            let q = ch;
            i += 1;
            let mut s = String::new();
            while i < c.len() && c[i] != q {
                if c[i] == '\\' && i + 1 < c.len() {
                    i += 1;
                }
                s.push(c[i]);
                i += 1;
            }
            if i >= c.len() {
                return Err("unterminated string".into());
            }
            i += 1;
            out.push(Tok::Str(s));
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(c[start..i].iter().collect()));
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        let op = match two.as_str() {
            "==" => Some("=="),
            "!=" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            "&&" => Some("&&"),
            "||" => Some("||"),
            _ => None,
        };
        if let Some(op) = op {
            out.push(Tok::Op(op));
            i += 2;
            continue;
        }
        let op = match ch {
            '<' => "<",
            '>' => ">",
            '!' => "!",
            '(' => "(",
            ')' => ")",
            '[' => "[",
            ']' => "]",
            ',' => ",",
            '.' => ".",
            '+' => "+",
            '-' => "-",
            _ => return Err(format!("unexpected '{ch}'")),
        };
        out.push(Tok::Op(op));
        i += 1;
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum Expr {
    Lit(Val),
    Var(String),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Call(String, Option<Box<Expr>>, Vec<Expr>),
    List(Vec<Expr>),
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("expected '{op}'"))
        }
    }
    fn or(&mut self) -> Result<Expr, String> {
        let mut l = self.and()?;
        while self.eat("||") {
            l = Expr::Bin("||", Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Expr, String> {
        let mut l = self.rel()?;
        while self.eat("&&") {
            l = Expr::Bin("&&", Box::new(l), Box::new(self.rel()?));
        }
        Ok(l)
    }
    fn rel(&mut self) -> Result<Expr, String> {
        let l = self.add()?;
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.eat(op) {
                return Ok(Expr::Bin(op, Box::new(l), Box::new(self.add()?)));
            }
        }
        if matches!(self.peek(), Some(Tok::Ident(s)) if s == "in") {
            self.i += 1;
            return Ok(Expr::Bin("in", Box::new(l), Box::new(self.add()?)));
        }
        Ok(l)
    }
    fn add(&mut self) -> Result<Expr, String> {
        let mut l = self.unary()?;
        loop {
            if self.eat("+") {
                l = Expr::Bin("+", Box::new(l), Box::new(self.unary()?));
            } else if self.eat("-") {
                l = Expr::Bin("-", Box::new(l), Box::new(self.unary()?));
            } else {
                return Ok(l);
            }
        }
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat("!") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        if self.eat("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.postfix()
    }
    fn args(&mut self) -> Result<Vec<Expr>, String> {
        let mut a = Vec::new();
        if self.eat(")") {
            return Ok(a);
        }
        loop {
            a.push(self.or()?);
            if self.eat(")") {
                return Ok(a);
            }
            self.expect(",")?;
        }
    }
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            if self.eat("[") {
                let idx = self.or()?;
                self.expect("]")?;
                e = Expr::Index(Box::new(e), Box::new(idx));
            } else if self.eat(".") {
                let Some(Tok::Ident(name)) = self.peek().cloned() else {
                    return Err("expected a name after '.'".into());
                };
                self.i += 1;
                if self.eat("(") {
                    let args = self.args()?;
                    e = Expr::Call(name, Some(Box::new(e)), args);
                } else {
                    e = Expr::Field(Box::new(e), name);
                }
            } else {
                return Ok(e);
            }
        }
    }
    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.i += 1;
                Ok(Expr::Lit(Val::Num(n)))
            }
            Some(Tok::Str(s)) => {
                self.i += 1;
                Ok(Expr::Lit(Val::Str(s)))
            }
            Some(Tok::Ident(id)) => {
                self.i += 1;
                match id.as_str() {
                    "true" => return Ok(Expr::Lit(Val::Bool(true))),
                    "false" => return Ok(Expr::Lit(Val::Bool(false))),
                    "null" => return Ok(Expr::Lit(Val::Null)),
                    _ => {}
                }
                if self.eat("(") {
                    let args = self.args()?;
                    return Ok(Expr::Call(id, None, args));
                }
                Ok(Expr::Var(id))
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                let e = self.or()?;
                self.expect(")")?;
                Ok(e)
            }
            Some(Tok::Op("[")) => {
                self.i += 1;
                let mut items = Vec::new();
                if !self.eat("]") {
                    loop {
                        items.push(self.or()?);
                        if self.eat("]") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(Expr::List(items))
            }
            other => Err(format!("unexpected {other:?}")),
        }
    }
}

pub struct Program {
    expr: Expr,
}

impl Program {
    pub fn compile(src: &str) -> Result<Self, String> {
        let mut p = Parser { t: lex(src)?, i: 0 };
        let expr = p.or()?;
        if p.i != p.t.len() {
            return Err("unexpected trailing input".into());
        }
        Ok(Self { expr })
    }

    /// Variables the expression reads (excluding `has()` field names).
    pub fn variables(&self) -> Vec<String> {
        fn walk(e: &Expr, out: &mut Vec<String>) {
            match e {
                Expr::Var(v) => {
                    if !out.contains(v) {
                        out.push(v.clone());
                    }
                }
                Expr::Lit(_) => {}
                Expr::Not(x) | Expr::Neg(x) | Expr::Field(x, _) => walk(x, out),
                Expr::Bin(_, a, b) | Expr::Index(a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                Expr::Call(_, recv, args) => {
                    if let Some(r) = recv {
                        walk(r, out);
                    }
                    args.iter().for_each(|a| walk(a, out));
                }
                Expr::List(items) => items.iter().for_each(|a| walk(a, out)),
            }
        }
        let mut out = Vec::new();
        walk(&self.expr, &mut out);
        out
    }

    pub fn eval_bool(&self, env: &Env) -> Result<bool, String> {
        match eval(&self.expr, env)? {
            Val::Bool(b) => Ok(b),
            other => Err(format!("rule must return a bool, got {other:?}")),
        }
    }
}

fn truthy(v: Val) -> Result<bool, String> {
    match v {
        Val::Bool(b) => Ok(b),
        other => Err(format!("expected bool, got {other:?}")),
    }
}

fn eval(e: &Expr, env: &Env) -> Result<Val, String> {
    Ok(match e {
        Expr::Lit(v) => v.clone(),
        Expr::Var(v) => env
            .get(v)
            .cloned()
            .ok_or_else(|| format!("undeclared reference to '{v}'"))?,
        Expr::Not(x) => Val::Bool(!truthy(eval(x, env)?)?),
        Expr::Neg(x) => match eval(x, env)? {
            Val::Num(n) => Val::Num(-n),
            o => return Err(format!("cannot negate {o:?}")),
        },
        Expr::List(items) => Val::List(
            items
                .iter()
                .map(|i| eval(i, env))
                .collect::<Result<_, _>>()?,
        ),
        Expr::Bin("&&", a, b) => {
            // CEL is commutative on errors for && and ||; short-circuit is enough here.
            Val::Bool(truthy(eval(a, env)?)? && truthy(eval(b, env)?)?)
        }
        Expr::Bin("||", a, b) => Val::Bool(truthy(eval(a, env)?)? || truthy(eval(b, env)?)?),
        Expr::Bin(op, a, b) => {
            let (l, r) = (eval(a, env)?, eval(b, env)?);
            match (*op, l, r) {
                ("==", l, r) => Val::Bool(l == r),
                ("!=", l, r) => Val::Bool(l != r),
                ("in", l, Val::List(items)) => Val::Bool(items.contains(&l)),
                ("in", Val::Str(k), Val::Map(m)) => Val::Bool(m.contains_key(&k)),
                ("+", Val::Num(x), Val::Num(y)) => Val::Num(x + y),
                ("+", Val::Str(x), Val::Str(y)) => Val::Str(x + &y),
                ("-", Val::Num(x), Val::Num(y)) => Val::Num(x - y),
                (op, Val::Num(x), Val::Num(y)) => Val::Bool(match op {
                    "<" => x < y,
                    "<=" => x <= y,
                    ">" => x > y,
                    ">=" => x >= y,
                    _ => return Err(format!("no overload for {op}")),
                }),
                (op, Val::Str(x), Val::Str(y)) => Val::Bool(match op {
                    "<" => x < y,
                    "<=" => x <= y,
                    ">" => x > y,
                    ">=" => x >= y,
                    _ => return Err(format!("no overload for {op}")),
                }),
                (op, l, r) => return Err(format!("no overload for {l:?} {op} {r:?}")),
            }
        }
        Expr::Index(m, k) => match (eval(m, env)?, eval(k, env)?) {
            (Val::Map(m), Val::Str(k)) => m
                .get(&k)
                .cloned()
                .ok_or_else(|| format!("no such key: {k}"))?,
            (Val::List(l), Val::Num(i)) => {
                l.get(i as usize).cloned().ok_or("index out of range")?
            }
            (a, b) => return Err(format!("cannot index {a:?} with {b:?}")),
        },
        Expr::Field(m, f) => match eval(m, env)? {
            Val::Map(m) => m
                .get(f)
                .cloned()
                .ok_or_else(|| format!("no such key: {f}"))?,
            o => return Err(format!("cannot select '{f}' on {o:?}")),
        },
        Expr::Call(name, None, args) if name == "has" => match args.as_slice() {
            [Expr::Field(m, f)] => match eval(m, env)? {
                Val::Map(m) => Val::Bool(m.contains_key(f)),
                _ => Val::Bool(false),
            },
            _ => return Err("has() takes a field selection like has(labels.env)".into()),
        },
        Expr::Call(name, None, args) if name == "size" && args.len() == 1 => {
            size(eval(&args[0], env)?)?
        }
        Expr::Call(name, Some(recv), args) => {
            let r = eval(recv, env)?;
            let a: Vec<Val> = args
                .iter()
                .map(|x| eval(x, env))
                .collect::<Result<_, _>>()?;
            match (name.as_str(), r, a.as_slice()) {
                ("size", v, []) => size(v)?,
                ("startsWith", Val::Str(s), [Val::Str(p)]) => Val::Bool(s.starts_with(p.as_str())),
                ("endsWith", Val::Str(s), [Val::Str(p)]) => Val::Bool(s.ends_with(p.as_str())),
                ("contains", Val::Str(s), [Val::Str(p)]) => Val::Bool(s.contains(p.as_str())),
                ("lowerAscii", Val::Str(s), []) => Val::Str(s.to_ascii_lowercase()),
                ("upperAscii", Val::Str(s), []) => Val::Str(s.to_ascii_uppercase()),
                ("matches", Val::Str(s), [Val::Str(p)]) => Val::Bool(
                    regex::Regex::new(p)
                        .map_err(|e| e.to_string())?
                        .is_match(&s),
                ),
                (n, r, _) => return Err(format!("no function {n} on {r:?}")),
            }
        }
        Expr::Call(name, _, _) => return Err(format!("unknown function {name}")),
    })
}

fn size(v: Val) -> Result<Val, String> {
    Ok(Val::Num(match v {
        Val::Str(s) => s.chars().count() as f64,
        Val::List(l) => l.len() as f64,
        Val::Map(m) => m.len() as f64,
        o => return Err(format!("size() of {o:?}")),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Env {
        Env::from([
            ("cpu_cores".into(), Val::Num(8.0)),
            ("memory_gib".into(), Val::Num(16.0)),
            ("rdp_exposed".into(), Val::Bool(true)),
            ("name".into(), Val::Str("web-prod-1".into())),
            (
                "labels".into(),
                Val::Map(BTreeMap::from([("env".into(), Val::Str("prod".into()))])),
            ),
        ])
    }

    fn run(src: &str) -> Result<bool, String> {
        Program::compile(src)?.eval_bool(&env())
    }

    #[test]
    fn comparisons_and_logic() {
        assert!(run("cpu_cores > 4 && memory_gib <= 16.0").unwrap());
        assert!(run("cpu_cores > 4u").unwrap());
        assert!(!run("!(cpu_cores >= 8) || rdp_exposed == false").unwrap());
        assert!(run("cpu_cores in [2, 4, 8]").unwrap());
        assert!(run("-1 < 0").unwrap());
    }

    #[test]
    fn maps_and_strings() {
        assert!(run("labels['env'] == 'prod'").unwrap());
        assert!(run("labels.env == \"prod\"").unwrap());
        assert!(run("'env' in labels && has(labels.env)").unwrap());
        assert!(!run("has(labels.team)").unwrap());
        assert!(run("!('team' in labels) || labels['team'] != ''").unwrap());
        assert!(
            run(
                "name.startsWith('web') && name.contains('prod') && name.matches('^web-.*-[0-9]+$')"
            )
            .unwrap()
        );
        assert!(run("size(name) == 10 && name.size() == 10").unwrap());
    }

    #[test]
    fn errors_are_reported() {
        assert!(
            run("labels['team'] == 'x'").is_err(),
            "missing key is an error, like CEL"
        );
        assert!(run("nope == 1").is_err());
        assert!(run("cpu_cores").is_err(), "non-bool result");
        assert!(Program::compile("cpu_cores >").is_err());
        assert!(Program::compile("a b").is_err());
    }

    #[test]
    fn lists_variables() {
        let p =
            Program::compile("cpu_cores > 2 && labels['env'] == 'prod' && has(labels.x)").unwrap();
        assert_eq!(p.variables(), vec!["cpu_cores", "labels"]);
    }
}
