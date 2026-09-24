use crate::json_patch::{apply, apply_inverse, diff, Key, Op};
use serde_json::{json, Map, Value};

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_value(r: &mut Lcg, depth: u32) -> Value {
    let kind = if depth >= 4 { r.below(4) } else { r.below(6) };
    match kind {
        0 => Value::Null,
        1 => json!(r.below(2) == 0),
        2 => json!(r.below(100)),
        3 => json!(format!("s{}", r.below(20))),
        4 => Value::Array(
            (0..r.below(4))
                .map(|_| random_value(r, depth + 1))
                .collect(),
        ),
        _ => {
            let mut o = Map::new();
            for _ in 0..r.below(5) {
                o.insert(format!("k{}", r.below(6)), random_value(r, depth + 1));
            }
            Value::Object(o)
        }
    }
}

/// Mutates a random node: change a leaf, set or remove an object key, grow or
/// shrink an array, or replace a container with a different type.
fn mutate(r: &mut Lcg, v: &mut Value, depth: u32) {
    let descend = r.below(3) != 0;
    match v {
        Value::Object(o) if descend && !o.is_empty() => {
            let keys: Vec<_> = o.keys().cloned().collect();
            let k = &keys[r.below(keys.len() as u64) as usize];
            mutate(r, o.get_mut(k).unwrap(), depth + 1);
        }
        Value::Array(a) if descend && !a.is_empty() => {
            let i = r.below(a.len() as u64) as usize;
            mutate(r, &mut a[i], depth + 1);
        }
        Value::Object(o) => match r.below(3) {
            0 if !o.is_empty() => {
                let keys: Vec<_> = o.keys().cloned().collect();
                o.remove(&keys[r.below(keys.len() as u64) as usize]);
            }
            1 => {
                o.insert(format!("k{}", r.below(8)), random_value(r, depth + 1));
            }
            _ => *v = random_value(r, depth),
        },
        Value::Array(a) => match r.below(3) {
            0 if !a.is_empty() => {
                a.pop();
            }
            1 => a.push(random_value(r, depth + 1)),
            _ => *v = random_value(r, depth),
        },
        _ => *v = random_value(r, depth),
    }
}

#[test]
fn diff_apply_and_inverse_round_trip() {
    let mut r = Lcg(7);
    for case in 0..2000 {
        let before = random_value(&mut r, 0);
        let mut after = before.clone();
        for _ in 0..=r.below(4) {
            mutate(&mut r, &mut after, 0);
        }
        let patch = diff(&before, &after);
        assert_eq!(patch.is_empty(), before == after, "case {case}");
        let mut forward = before.clone();
        apply(&mut forward, &patch).unwrap();
        assert_eq!(forward, after, "case {case}: forward");
        let mut target = before.clone();
        let inverse = apply_inverse(&mut target, patch.clone()).unwrap();
        assert_eq!(target, after, "case {case}: apply_inverse forward");
        let redo = apply_inverse(&mut target, inverse).unwrap();
        assert_eq!(target, before, "case {case}: inverse restores");
        apply(&mut target, &redo).unwrap();
        assert_eq!(target, after, "case {case}: inverse of inverse");
    }
}

#[test]
fn one_cell_change_in_a_large_workbook_is_one_small_op() {
    let workbook = |value: &str| {
        let sheets: Vec<Value> = (1..=20)
            .map(|s| {
                let mut cells = Map::new();
                for row in 1..=2500 {
                    cells.insert(format!("A{row}"), json!(row.to_string()));
                    cells.insert(format!("B{row}"), json!(format!("=A{row}*2")));
                }
                if s == 1 {
                    cells.insert("A1".into(), json!(value));
                }
                json!({"id": format!("s{s}"), "cells": cells})
            })
            .collect();
        json!({"revision": 1, "sheets": sheets})
    };
    let patch = diff(&workbook("1"), &workbook("2"));
    assert_eq!(
        patch.ops,
        vec![Op::Set {
            path: vec![
                Key::Field("sheets".into()),
                Key::Index(0),
                Key::Field("cells".into()),
                Key::Field("A1".into()),
            ],
            value: json!("2"),
        }]
    );
    assert!(patch.bytes() < 256);
}

#[test]
fn without_field_drops_only_that_top_level_field() {
    let patch = diff(
        &json!({"revision": 1, "title": "a"}),
        &json!({"revision": 2, "title": "b"}),
    )
    .without_field("revision");
    assert_eq!(
        patch.ops,
        vec![Op::Set {
            path: vec![Key::Field("title".into())],
            value: json!("b")
        }]
    );
}
