use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    I32(i32),
    I64(i64),
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::I32(value)
    }
}

impl From<Value> for i32 {
    fn from(value: Value) -> Self {
        match value {
            Value::I32(value) => value,
            _ => panic!("type mismatch"),
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::I32(if value { 1 } else { 0 })
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::I64(value)
    }
}

impl std::ops::Add for Value {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::I32(left), Value::I32(right)) => Value::I32(left.wrapping_add(right)),
            (Value::I64(left), Value::I64(right)) => Value::I64(left.wrapping_add(right)),
            _ => panic!("type mismatch"),
        }
    }
}

impl std::ops::Sub for Value {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Value::I32(left), Value::I32(right)) => Value::I32(left - right),
            (Value::I64(left), Value::I64(right)) => Value::I64(left - right),
            _ => panic!("type mismatch"),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::I32(a), Value::I32(b)) => a.partial_cmp(b),
            (Value::I64(a), Value::I64(b)) => a.partial_cmp(b),
            _ => panic!("type mismatch"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LabelKind {
    If,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub kind: LabelKind,
    pub pc: usize,
    pub sp: usize,
    pub arity: usize,
}
