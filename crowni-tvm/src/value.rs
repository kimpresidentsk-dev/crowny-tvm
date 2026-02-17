///! VM Value 타입 — GPT 명세 기반
///! 정수(i64), 실수(f64), 논리(bool), 트릿(i8), 주소(usize),
///! 문자열, 배열(Vec<Value>), 객체(HashMap), 없음

use crate::trit::Trit;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),                         // 정수
    Float(f64),                       // 실수
    Bool(bool),                       // 논리 (2진 호환)
    Trit(Trit),                       // 트릿 (-1,0,+1)
    Addr(usize),                      // 주소 (힙/프로그램)
    Str(String),                      // 문자열 (UTF-8)
    Array(Vec<Value>),                // 배열 (텐서=다차원배열)
    Object(HashMap<String, Value>),   // 객체
    Nil,                              // 없음/Om
}

impl Value {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            Value::Trit(t) => Some(t.to_i8() as i64),
            Value::Bool(b) => Some(if *b { 1 } else { 0 }),
            Value::Addr(a) => Some(*a as i64),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            Value::Trit(t) => Some(t.to_i8() as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self { Some(s) } else { None }
    }

    pub fn as_addr(&self) -> Option<usize> {
        match self {
            Value::Addr(a) => Some(*a),
            Value::Int(n) if *n >= 0 => Some(*n as usize),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Trit(Trit::P) => true,
            Value::Trit(_) => false,
            Value::Str(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Nil => false,
            _ => true,
        }
    }

    /// 3진 변환: P=양, T=음, O=중립
    pub fn to_trit(&self) -> Trit {
        match self {
            Value::Int(0) => Trit::O,
            Value::Int(n) if *n > 0 => Trit::P,
            Value::Int(_) => Trit::T,
            Value::Float(f) if *f == 0.0 => Trit::O,
            Value::Float(f) if *f > 0.0 => Trit::P,
            Value::Float(_) => Trit::T,
            Value::Bool(true) => Trit::P,
            Value::Bool(false) => Trit::T,
            Value::Trit(t) => *t,
            Value::Nil => Trit::T,
            Value::Str(s) if s.is_empty() => Trit::O,
            _ => Trit::P,
        }
    }

    pub fn type_name_kr(&self) -> &'static str {
        match self {
            Value::Int(_) => "정수",
            Value::Float(_) => "실수",
            Value::Bool(_) => "논리",
            Value::Trit(_) => "트릿",
            Value::Addr(_) => "주소",
            Value::Str(_) => "문자열",
            Value::Array(_) => "배열",
            Value::Object(_) => "객체",
            Value::Nil => "없음",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(v) => write!(f, "{:.6}", v),
            Value::Bool(b) => write!(f, "{}", if *b { "참" } else { "거짓" }),
            Value::Trit(t) => write!(f, "{}", t),
            Value::Addr(a) => write!(f, "&{}", a),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Nil => write!(f, "없음"),
        }
    }
}
