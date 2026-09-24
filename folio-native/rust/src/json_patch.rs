//! Structural patches between JSON values. Undo history stores the patch that
//! turns the current workspace back into the previous one instead of a full
//! copy, so a one-cell edit costs a one-cell history entry.
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Field(String),
    Index(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    Set { path: Vec<Key>, value: Value },
    Remove { path: Vec<Key> },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    pub ops: Vec<Op>,
}

/// Ops that turn `before` into `after`. Descends into objects key by key and
/// into arrays of equal length index by index; anything else is replaced whole.
pub fn diff(before: &Value, after: &Value) -> Patch {
    let mut patch = Patch::default();
    let mut path = Vec::new();
    walk(before, after, &mut path, &mut patch.ops);
    patch
}

fn walk(before: &Value, after: &Value, path: &mut Vec<Key>, ops: &mut Vec<Op>) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(b), Value::Object(a)) => {
            for (k, bv) in b {
                path.push(Key::Field(k.clone()));
                match a.get(k) {
                    Some(av) => walk(bv, av, path, ops),
                    None => ops.push(Op::Remove { path: path.clone() }),
                }
                path.pop();
            }
            for (k, av) in a {
                if !b.contains_key(k) {
                    path.push(Key::Field(k.clone()));
                    ops.push(Op::Set {
                        path: path.clone(),
                        value: av.clone(),
                    });
                    path.pop();
                }
            }
        }
        (Value::Array(b), Value::Array(a)) if b.len() == a.len() => {
            for (i, (bv, av)) in b.iter().zip(a).enumerate() {
                path.push(Key::Index(i));
                walk(bv, av, path, ops);
                path.pop();
            }
        }
        _ => ops.push(Op::Set {
            path: path.clone(),
            value: after.clone(),
        }),
    }
}

fn parent<'a>(target: &'a mut Value, path: &[Key]) -> Result<&'a mut Value, String> {
    let mut node = target;
    for key in path {
        node = match key {
            Key::Field(k) => node
                .as_object_mut()
                .and_then(|o| o.get_mut(k))
                .ok_or_else(|| format!("Patch path missing field {k}"))?,
            Key::Index(i) => node
                .as_array_mut()
                .and_then(|a| a.get_mut(*i))
                .ok_or_else(|| format!("Patch path missing index {i}"))?,
        };
    }
    Ok(node)
}

/// Applies one op and returns the op that undoes it.
fn apply_op(target: &mut Value, op: Op) -> Result<Op, String> {
    let (path, value) = match op {
        Op::Set { path, value } => (path, Some(value)),
        Op::Remove { path } => (path, None),
    };
    let Some((last, head)) = path.split_last() else {
        let old = std::mem::replace(target, value.ok_or("Cannot remove the patch root")?);
        return Ok(Op::Set { path, value: old });
    };
    let node = parent(target, head)?;
    let old = match (last, value) {
        (Key::Field(k), Some(v)) => node
            .as_object_mut()
            .ok_or("Patch parent is not an object")?
            .insert(k.clone(), v),
        (Key::Field(k), None) => node
            .as_object_mut()
            .ok_or("Patch parent is not an object")?
            .remove(k),
        (Key::Index(i), Some(v)) => Some(std::mem::replace(
            node.as_array_mut()
                .and_then(|a| a.get_mut(*i))
                .ok_or("Patch index out of range")?,
            v,
        )),
        (Key::Index(_), None) => return Err("Patch cannot remove array items".into()),
    };
    Ok(match old {
        Some(value) => Op::Set { path, value },
        None => Op::Remove { path },
    })
}

/// The value at `path`, if any.
pub fn get<'a>(target: &'a Value, path: &[Key]) -> Option<&'a Value> {
    path.iter().try_fold(target, |node, key| match key {
        Key::Field(k) => node.get(k),
        Key::Index(i) => node.get(*i),
    })
}

#[cfg(test)]
pub fn apply(target: &mut Value, patch: &Patch) -> Result<(), String> {
    apply_inverse(target, patch.clone()).map(|_| ())
}

/// Applies `patch` and returns the patch that undoes it. On error `target`
/// is left partly patched; callers only apply patches produced by `diff`
/// against the same value.
pub fn apply_inverse(target: &mut Value, patch: Patch) -> Result<Patch, String> {
    let mut inverse = Vec::with_capacity(patch.ops.len());
    for op in patch.ops {
        inverse.push(apply_op(target, op)?);
    }
    inverse.reverse();
    Ok(Patch { ops: inverse })
}

impl Patch {
    pub fn paths(&self) -> Vec<Vec<Key>> {
        self.ops
            .iter()
            .map(|op| match op {
                Op::Set { path, .. } | Op::Remove { path } => path.clone(),
            })
            .collect()
    }
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    /// Approximate retained size, for history limits.
    pub fn bytes(&self) -> usize {
        fn value_bytes(v: &Value) -> usize {
            match v {
                Value::Null | Value::Bool(_) | Value::Number(_) => 8,
                Value::String(s) => s.len() + 2,
                Value::Array(a) => 2 + a.iter().map(value_bytes).sum::<usize>(),
                Value::Object(o) => {
                    2 + o
                        .iter()
                        .map(|(k, v)| k.len() + 3 + value_bytes(v))
                        .sum::<usize>()
                }
            }
        }
        fn path_bytes(p: &[Key]) -> usize {
            p.iter()
                .map(|k| match k {
                    Key::Field(f) => f.len() + 1,
                    Key::Index(_) => 8,
                })
                .sum()
        }
        self.ops
            .iter()
            .map(|op| match op {
                Op::Set { path, value } => path_bytes(path) + value_bytes(value),
                Op::Remove { path } => path_bytes(path),
            })
            .sum()
    }
    /// Drops ops at or below a top-level field, e.g. the revision counter.
    pub fn without_field(mut self, field: &str) -> Self {
        self.ops.retain(|op| {
            let path = match op {
                Op::Set { path, .. } | Op::Remove { path } => path,
            };
            !matches!(path.first(), Some(Key::Field(f)) if f == field)
        });
        self
    }
}
