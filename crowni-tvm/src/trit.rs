///! 균형 3진법 기본 타입 — 크라우니수 / 티옴타 (Trit-Om-Ta)
///! T(타) = -1 (음, Negative)
///! O(옴) =  0 (중, Zero/Om)  
///! P(티) = +1 (양, Positive)

use std::fmt;

// ─────────────────────────────────────────────
// Trit — 균형3진법 최소 단위
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum Trit {
    T = -1, // 타 (Negative)
    O = 0,  // 옴 (Zero/Om)
    P = 1,  // 티 (Positive)
}

impl Trit {
    pub fn from_i8(v: i8) -> Self {
        match v {
            -1 => Trit::T,
            0 => Trit::O,
            1 => Trit::P,
            _ => panic!("Trit 범위 초과: {} (허용: -1, 0, +1)", v),
        }
    }

    pub fn to_i8(self) -> i8 {
        self as i8
    }

    /// 균형3진 NOT (반전): T↔P, O→O
    pub fn not(self) -> Self {
        match self {
            Trit::T => Trit::P,
            Trit::O => Trit::O,
            Trit::P => Trit::T,
        }
    }

    /// 균형3진 AND (min)
    pub fn and(self, other: Trit) -> Trit {
        Trit::from_i8(self.to_i8().min(other.to_i8()))
    }

    /// 균형3진 OR (max)
    pub fn or(self, other: Trit) -> Trit {
        Trit::from_i8(self.to_i8().max(other.to_i8()))
    }

    /// 문자 → Trit
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'T' | 't' | 'ㄴ' | '타' => Some(Trit::T),
            'O' | 'o' | '0' | 'ㅇ' | '옴' => Some(Trit::O),
            'P' | 'p' | '1' | 'ㅍ' | '티' => Some(Trit::P),
            _ => None,
        }
    }
}

impl fmt::Display for Trit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Trit::T => write!(f, "T"),
            Trit::O => write!(f, "O"),
            Trit::P => write!(f, "P"),
        }
    }
}

// ─────────────────────────────────────────────
// Word6 — 6-trit 워드 (opcode 단위, 3^6=729)
// ─────────────────────────────────────────────
// 구조: [S1 S2][G1 G2][C1 C2]
//       sector  group  command
// trits[5..4]  trits[3..2]  trits[1..0]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Word6 {
    pub trits: [Trit; 6],
}

/// 2-trit → index(0..8) 매핑 테이블
const PAIR_TO_INDEX: [(Trit, Trit); 9] = [
    (Trit::T, Trit::T), // 0: val=-4
    (Trit::T, Trit::O), // 1: val=-3
    (Trit::T, Trit::P), // 2: val=-2
    (Trit::O, Trit::T), // 3: val=-1
    (Trit::O, Trit::O), // 4: val= 0
    (Trit::O, Trit::P), // 5: val=+1
    (Trit::P, Trit::T), // 6: val=+2
    (Trit::P, Trit::O), // 7: val=+3
    (Trit::P, Trit::P), // 8: val=+4
];

impl Word6 {
    pub fn new(trits: [Trit; 6]) -> Self {
        Self { trits }
    }

    /// 정수(i16) → 균형3진 6트릿. 범위: -364 ~ +364
    pub fn from_decimal(mut val: i16) -> Self {
        assert!((-364..=364).contains(&val), "6-trit 범위 초과: {}", val);
        let mut trits = [Trit::O; 6];
        for i in 0..6 {
            let mut r = val % 3;
            val /= 3;
            if r > 1 { r -= 3; val += 1; }
            else if r < -1 { r += 3; val -= 1; }
            trits[i] = Trit::from_i8(r as i8);
        }
        Self { trits }
    }

    /// 균형3진 6트릿 → 정수(i16)
    pub fn to_decimal(&self) -> i16 {
        let mut val: i16 = 0;
        let mut base: i16 = 1;
        for i in 0..6 {
            val += self.trits[i].to_i8() as i16 * base;
            base *= 3;
        }
        val
    }

    /// opcode 분해: (sector, group, command) 각 0..8
    /// GPT 명세: pair_value = t0*3 + t1, normalized = pair_value + 4
    pub fn decode_opcode(&self) -> (u8, u8, u8) {
        let sector = Self::pair_to_idx(self.trits[5], self.trits[4]);
        let group  = Self::pair_to_idx(self.trits[3], self.trits[2]);
        let cmd    = Self::pair_to_idx(self.trits[1], self.trits[0]);
        (sector, group, cmd)
    }

    /// (sector, group, command) → Word6
    pub fn encode_opcode(sector: u8, group: u8, command: u8) -> Self {
        assert!(sector < 9 && group < 9 && command < 9);
        let (s1, s0) = PAIR_TO_INDEX[sector as usize];
        let (g1, g0) = PAIR_TO_INDEX[group as usize];
        let (c1, c0) = PAIR_TO_INDEX[command as usize];
        Self::new([c0, c1, g0, g1, s0, s1])
    }

    fn pair_to_idx(hi: Trit, lo: Trit) -> u8 {
        let val = hi.to_i8() as i16 * 3 + lo.to_i8() as i16; // -4..+4
        (val + 4) as u8
    }

    /// "TOOPPT" 같은 문자열에서 파싱 (상위→하위 순)
    pub fn from_trit_str(s: &str) -> Option<Self> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 6 { return None; }
        let mut trits = [Trit::O; 6];
        for (i, c) in chars.iter().rev().enumerate() {
            trits[i] = Trit::from_char(*c)?;
        }
        Some(Self { trits })
    }
}

impl fmt::Display for Word6 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in (0..6).rev() {
            write!(f, "{}", self.trits[i])?;
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trit_logic() {
        assert_eq!(Trit::T.not(), Trit::P);
        assert_eq!(Trit::P.not(), Trit::T);
        assert_eq!(Trit::O.not(), Trit::O);
        assert_eq!(Trit::T.and(Trit::P), Trit::T);
        assert_eq!(Trit::P.or(Trit::T), Trit::P);
    }

    #[test]
    fn decimal_roundtrip() {
        for v in -364..=364i16 {
            let w = Word6::from_decimal(v);
            assert_eq!(w.to_decimal(), v, "Failed at {}", v);
        }
    }

    #[test]
    fn opcode_roundtrip_all_729() {
        for s in 0..9u8 {
            for g in 0..9u8 {
                for c in 0..9u8 {
                    let w = Word6::encode_opcode(s, g, c);
                    let (ds, dg, dc) = w.decode_opcode();
                    assert_eq!((ds, dg, dc), (s, g, c),
                        "({},{},{}) → {} → ({},{},{})", s, g, c, w, ds, dg, dc);
                }
            }
        }
    }

    #[test]
    fn center_is_zero() {
        // (4,4,4) = 중심 = OOOOOO = decimal 0
        let w = Word6::encode_opcode(4, 4, 4);
        assert_eq!(w.to_decimal(), 0);
        assert_eq!(format!("{}", w), "OOOOOO");
    }
}
