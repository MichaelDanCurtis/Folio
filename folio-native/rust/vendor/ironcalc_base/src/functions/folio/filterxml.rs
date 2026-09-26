//! FILTERXML(xml, xpath) with the XPath 1.0 subset spreadsheets use:
//! child (`/`) and descendant (`//`) steps, names, `*`, `@attr`, `@*`,
//! `text()`, `.` and `..`, and predicates with positions, `last()`,
//! comparisons, `contains()`, `starts-with()`, `and` and `or`.
use crate::{
    calc_result::CalcResult,
    expressions::{
        parser::{ArrayNode, Node},
        token::Error,
        types::CellReferenceIndex,
    },
    model::Model,
};
use roxmltree::{Document, Node as XNode};

#[derive(Clone)]
enum Item<'a, 'i> {
    Node(XNode<'a, 'i>),
    Attr(String),
}

impl Item<'_, '_> {
    fn text(&self) -> String {
        match self {
            Item::Attr(v) => v.clone(),
            Item::Node(n) if n.is_text() => n.text().unwrap_or("").to_string(),
            Item::Node(n) => n
                .descendants()
                .filter(|d| d.is_text())
                .filter_map(|d| d.text())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Test {
    Name(String),
    Any,
    Attr(String),
    AnyAttr,
    Text,
    AnyNode,
    SelfNode,
    Parent,
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Path(Vec<Step>),
    Literal(String),
    Number(f64),
    Last,
    Position,
    Call(String, Vec<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
struct Step {
    descendant: bool,
    test: Test,
    predicates: Vec<Expr>,
}

struct Parser<'s> {
    s: &'s [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }
    fn eat(&mut self, t: &str) -> bool {
        self.ws();
        if self.s[self.i..].starts_with(t.as_bytes()) {
            self.i += t.len();
            true
        } else {
            false
        }
    }
    fn name(&mut self) -> Option<String> {
        self.ws();
        let start = self.i;
        while self.i < self.s.len()
            && (self.s[self.i].is_ascii_alphanumeric()
                || matches!(self.s[self.i], b'_' | b'-' | b'.' | b':')
                || self.s[self.i] >= 0x80)
        {
            self.i += 1;
        }
        (self.i > start).then(|| String::from_utf8_lossy(&self.s[start..self.i]).to_string())
    }

    fn path(&mut self) -> Option<Vec<Step>> {
        let mut steps = vec![];
        let mut descendant = if self.eat("//") {
            true
        } else {
            self.eat("/");
            false
        };
        loop {
            let test = if self.eat("@*") {
                Test::AnyAttr
            } else if self.eat("@") {
                Test::Attr(self.name()?)
            } else if self.eat("..") {
                Test::Parent
            } else if self.eat(".") {
                Test::SelfNode
            } else if self.eat("*") {
                Test::Any
            } else if self.eat("text()") {
                Test::Text
            } else if self.eat("node()") {
                Test::AnyNode
            } else {
                Test::Name(self.name()?)
            };
            let mut predicates = vec![];
            while self.eat("[") {
                predicates.push(self.or()?);
                if !self.eat("]") {
                    return None;
                }
            }
            steps.push(Step {
                descendant,
                test,
                predicates,
            });
            if self.eat("//") {
                descendant = true;
            } else if self.eat("/") {
                descendant = false;
            } else {
                return Some(steps);
            }
        }
    }

    fn or(&mut self) -> Option<Expr> {
        let mut left = self.and()?;
        while self.eat(" or ") || self.eat("or ") {
            left = Expr::Binary("or".into(), Box::new(left), Box::new(self.and()?));
        }
        Some(left)
    }
    fn and(&mut self) -> Option<Expr> {
        let mut left = self.compare()?;
        while self.eat("and ") {
            left = Expr::Binary("and".into(), Box::new(left), Box::new(self.compare()?));
        }
        Some(left)
    }
    fn compare(&mut self) -> Option<Expr> {
        let left = self.additive()?;
        for op in ["!=", "<=", ">=", "=", "<", ">"] {
            if self.eat(op) {
                return Some(Expr::Binary(
                    op.into(),
                    Box::new(left),
                    Box::new(self.additive()?),
                ));
            }
        }
        Some(left)
    }
    fn additive(&mut self) -> Option<Expr> {
        let mut left = self.primary()?;
        loop {
            self.ws();
            // `-` inside names is part of the name; here it is subtraction.
            let op = if self.eat("+") {
                "+"
            } else if self.s.get(self.i) == Some(&b'-') {
                self.i += 1;
                "-"
            } else {
                return Some(left);
            };
            left = Expr::Binary(op.into(), Box::new(left), Box::new(self.primary()?));
        }
    }
    fn primary(&mut self) -> Option<Expr> {
        self.ws();
        let c = *self.s.get(self.i)?;
        if c == b'\'' || c == b'"' {
            let end = self.s[self.i + 1..].iter().position(|b| *b == c)? + self.i + 1;
            let text = String::from_utf8_lossy(&self.s[self.i + 1..end]).to_string();
            self.i = end + 1;
            return Some(Expr::Literal(text));
        }
        if c.is_ascii_digit() {
            let start = self.i;
            while self.i < self.s.len()
                && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.')
            {
                self.i += 1;
            }
            return std::str::from_utf8(&self.s[start..self.i])
                .ok()?
                .parse()
                .ok()
                .map(Expr::Number);
        }
        if self.eat("(") {
            let e = self.or()?;
            return self.eat(")").then_some(e);
        }
        if self.eat("last()") {
            return Some(Expr::Last);
        }
        if self.eat("position()") {
            return Some(Expr::Position);
        }
        for f in [
            "contains",
            "starts-with",
            "not",
            "string-length",
            "normalize-space",
            "count",
        ] {
            let save = self.i;
            if self.eat(f) && self.eat("(") {
                let mut args = vec![];
                if !self.eat(")") {
                    loop {
                        args.push(self.or()?);
                        if self.eat(")") {
                            break;
                        }
                        if !self.eat(",") {
                            return None;
                        }
                    }
                }
                return Some(Expr::Call(f.into(), args));
            }
            self.i = save;
        }
        self.path().map(Expr::Path)
    }
}

fn parse_xpath(xpath: &str) -> Option<Vec<Step>> {
    let mut p = Parser {
        s: xpath.trim().as_bytes(),
        i: 0,
    };
    let steps = p.path()?;
    p.ws();
    (p.i == p.s.len()).then_some(steps)
}

fn step<'a, 'i>(from: &[Item<'a, 'i>], s: &Step) -> Vec<Item<'a, 'i>> {
    let mut out: Vec<Item> = vec![];
    for item in from {
        let Item::Node(n) = item else { continue };
        let context: Vec<XNode> = if s.descendant {
            n.descendants().collect()
        } else {
            vec![*n]
        };
        let mut matched: Vec<Item> = vec![];
        for c in context {
            match &s.test {
                Test::Attr(name) => matched.extend(
                    c.attributes()
                        .filter(|a| a.name() == name.rsplit(':').next().unwrap_or(name))
                        .map(|a| Item::Attr(a.value().to_string())),
                ),
                Test::AnyAttr => {
                    matched.extend(c.attributes().map(|a| Item::Attr(a.value().to_string())))
                }
                Test::SelfNode => matched.push(Item::Node(c)),
                Test::Parent => matched.extend(c.parent().map(Item::Node)),
                test => matched.extend(
                    c.children()
                        .filter(|k| match test {
                            Test::Name(name) => {
                                k.is_element()
                                    && k.tag_name().name()
                                        == name.rsplit(':').next().unwrap_or(name)
                            }
                            Test::Any => k.is_element(),
                            Test::Text => k.is_text(),
                            _ => k.is_element() || k.is_text(),
                        })
                        .map(Item::Node),
                ),
            }
            // Predicates apply to each context node's own matches.
            let mut kept = std::mem::take(&mut matched);
            for p in &s.predicates {
                let size = kept.len();
                kept = kept
                    .into_iter()
                    .enumerate()
                    .filter(|(i, item)| predicate(item, p, i + 1, size))
                    .map(|(_, item)| item)
                    .collect();
            }
            for item in kept {
                let seen = out.iter().any(|o| match (o, &item) {
                    (Item::Node(a), Item::Node(b)) => a == b,
                    _ => false,
                });
                if !seen {
                    out.push(item);
                }
            }
        }
    }
    out
}

#[derive(Clone)]
enum Value<'a, 'i> {
    Nodes(Vec<Item<'a, 'i>>),
    Text(String),
    Number(f64),
    Bool(bool),
}

impl Value<'_, '_> {
    fn number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::Bool(b) => f64::from(u8::from(*b)),
            other => other.text().trim().parse().unwrap_or(f64::NAN),
        }
    }
    fn text(&self) -> String {
        match self {
            Value::Nodes(n) => n.first().map(Item::text).unwrap_or_default(),
            Value::Text(t) => t.clone(),
            Value::Number(n) => format!("{n}"),
            Value::Bool(b) => b.to_string(),
        }
    }
    fn truth(&self) -> bool {
        match self {
            Value::Nodes(n) => !n.is_empty(),
            Value::Text(t) => !t.is_empty(),
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Bool(b) => *b,
        }
    }
}

fn eval<'a, 'i>(item: &Item<'a, 'i>, e: &Expr, position: usize, size: usize) -> Value<'a, 'i> {
    match e {
        Expr::Literal(s) => Value::Text(s.clone()),
        Expr::Number(n) => Value::Number(*n),
        Expr::Last => Value::Number(size as f64),
        Expr::Position => Value::Number(position as f64),
        Expr::Path(steps) => {
            let mut set = vec![item.clone()];
            for s in steps {
                set = step(&set, s);
            }
            Value::Nodes(set)
        }
        Expr::Call(f, args) => {
            let a: Vec<Value> = args.iter().map(|x| eval(item, x, position, size)).collect();
            let text = |i: usize| a.get(i).map(Value::text).unwrap_or_else(|| item.text());
            match f.as_str() {
                "contains" => Value::Bool(text(0).contains(&text(1))),
                "starts-with" => Value::Bool(text(0).starts_with(&text(1))),
                "not" => Value::Bool(!a.first().is_some_and(Value::truth)),
                "string-length" => Value::Number(text(0).chars().count() as f64),
                "normalize-space" => {
                    Value::Text(text(0).split_whitespace().collect::<Vec<_>>().join(" "))
                }
                _ => Value::Number(match a.first() {
                    Some(Value::Nodes(n)) => n.len() as f64,
                    _ => 0.0,
                }),
            }
        }
        Expr::Binary(op, l, r) => {
            let (l, r) = (eval(item, l, position, size), eval(item, r, position, size));
            match op.as_str() {
                "and" => Value::Bool(l.truth() && r.truth()),
                "or" => Value::Bool(l.truth() || r.truth()),
                "+" => Value::Number(l.number() + r.number()),
                "-" => Value::Number(l.number() - r.number()),
                op => {
                    // A node set compares true when any of its nodes does.
                    let sides = |v: &Value<'a, 'i>| match v {
                        Value::Nodes(n) => n.iter().map(|i| Value::Text(i.text())).collect(),
                        v => vec![v.clone()],
                    };
                    let numeric = matches!(l, Value::Number(_))
                        || matches!(r, Value::Number(_))
                        || !matches!(op, "=" | "!=");
                    Value::Bool(sides(&l).iter().any(|a| {
                        sides(&r).iter().any(|b| {
                            if numeric {
                                let (x, y) = (a.number(), b.number());
                                match op {
                                    "=" => x == y,
                                    "!=" => x != y,
                                    "<" => x < y,
                                    ">" => x > y,
                                    "<=" => x <= y,
                                    _ => x >= y,
                                }
                            } else if op == "=" {
                                a.text() == b.text()
                            } else {
                                a.text() != b.text()
                            }
                        })
                    }))
                }
            }
        }
    }
}

fn predicate(item: &Item, p: &Expr, position: usize, size: usize) -> bool {
    match eval(item, p, position, size) {
        Value::Number(n) => n == position as f64,
        v => v.truth(),
    }
}

/// The text of each node the XPath selects, in document order.
pub(crate) fn filter_xml(xml: &str, xpath: &str) -> Option<Vec<String>> {
    let doc = Document::parse(xml).ok()?;
    let steps = parse_xpath(xpath)?;
    let mut set = vec![Item::Node(doc.root())];
    for s in &steps {
        set = step(&set, s);
    }
    Some(set.iter().map(Item::text).collect())
}

impl Model<'_> {
    pub(crate) fn fn_filterxml(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 2 {
            return CalcResult::new_args_number_error(cell);
        }
        let xml = match self.get_string(&args[0], cell) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let xpath = match self.get_string(&args[1], cell) {
            Ok(s) => s,
            Err(e) => return e,
        };
        match filter_xml(&xml, &xpath) {
            Some(values) if !values.is_empty() => super::array_result(
                values
                    .into_iter()
                    .map(|v| {
                        vec![match v.trim().parse::<f64>() {
                            Ok(n) if n.is_finite() && !v.trim().is_empty() => ArrayNode::Number(n),
                            _ => ArrayNode::String(v),
                        }]
                    })
                    .collect(),
                cell,
            ),
            _ => CalcResult::new_error(Error::VALUE, cell, "Nothing matched".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::filter_xml;

    const XML: &str = r#"<rss><channel><item id="1"><title>Alpha</title><n>5</n></item><item id="2"><title>Beta</title><n>12</n></item><item id="3" hot="yes"><title>Gamma</title><n>7</n></item></channel></rss>"#;

    fn f(xpath: &str) -> Vec<String> {
        filter_xml(XML, xpath).unwrap()
    }

    #[test]
    fn paths_and_predicates() {
        assert_eq!(f("//title"), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(f("/rss/channel/item/title"), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(f("//item[2]/title"), ["Beta"]);
        assert_eq!(f("//item[last()]/title"), ["Gamma"]);
        assert_eq!(f("//item[last()-1]/title"), ["Beta"]);
        assert_eq!(f("//item/@id"), ["1", "2", "3"]);
        assert_eq!(f("//item[@hot]/title"), ["Gamma"]);
        assert_eq!(f("//item[@id='2']/title"), ["Beta"]);
        assert_eq!(f("//item[n>6]/title"), ["Beta", "Gamma"]);
        assert_eq!(f("//item[title='Alpha']/n"), ["5"]);
        assert_eq!(f("//title[contains(., 'mm')]"), ["Gamma"]);
        assert_eq!(f("//item[position()<3 and n>6]/title"), ["Beta"]);
        assert_eq!(f("//title/text()"), ["Alpha", "Beta", "Gamma"]);
        assert_eq!(f("//n/.."), ["Alpha5", "Beta12", "Gamma7"]);
        assert!(f("//missing").is_empty());
        assert!(filter_xml("<a>", "//a").is_none());
        assert!(filter_xml(XML, "//item[").is_none());
    }
}
