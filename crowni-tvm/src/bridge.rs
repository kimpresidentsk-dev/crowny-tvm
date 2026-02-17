///! ═══════════════════════════════════════════════════
///! FPGA Bridge — 균형3진↔2진 물리 매핑 계층
///! ═══════════════════════════════════════════════════
///!
///! FPGA 이전 전략:
///!   지금: 2진 CPU에서 Trit를 2bit로 매핑하여 에뮬레이션
///!   미래: 균형3진 FPGA/ASIC에서 네이티브 실행
///!
///! 물리 매핑 규격:
///!   1 Trit = 2 bits:
///!     T(-1) = 0b00
///!     O( 0) = 0b01
///!     P(+1) = 0b10
///!     무효   = 0b11 (패딩/오류)
///!
///!   1 Tryte = 3 Trits = 6 bits  (범위: -13~+13, 27가지)
///!   1 Word  = 6 Trits = 12 bits (범위: -364~+364, 729가지)
///!   1 DWord = 12 Trits = 24 bits
///!   1 QWord = 24 Trits = 48 bits
///!
///! 메모리 레이아웃:
///!   FPGA 이전: 1 Trit → 1 byte (편의상, 낭비 허용)
///!   FPGA 이후: 1 Trit → 2 bit (물리 와이어)
///!
///! 이 모듈은 두 모드를 모두 지원하여
///! FPGA 전환 시 상위 코드 변경 불필요.

// ─────────────────────────────────────────────
// 물리 매핑 상수
// ─────────────────────────────────────────────

/// 물리 매핑 모드
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalMode {
    /// 에뮬레이션: 1 trit = 1 byte (개발/디버그)
    Emulated,
    /// 패킹: 1 trit = 2 bits, 4 trits per byte
    Packed,
    /// FPGA: 네이티브 3진 와이어 (미래)
    Native,
}

/// 현재 모드 (컴파일 타임 선택)
pub const CURRENT_MODE: PhysicalMode = PhysicalMode::Packed;

// ─────────────────────────────────────────────
// Trit Word 타입 (크기별)
// ─────────────────────────────────────────────

/// Tryte = 3 Trits (범위: -13 ~ +13, 27가지)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tryte {
    pub trits: [i8; 3],
}

impl Tryte {
    pub fn from_decimal(mut val: i8) -> Self {
        assert!((-13..=13).contains(&val), "Tryte 범위 초과: {}", val);
        let mut trits = [0i8; 3];
        for i in 0..3 {
            let mut r = val % 3;
            val /= 3;
            if r > 1 { r -= 3; val += 1; }
            else if r < -1 { r += 3; val -= 1; }
            trits[i] = r;
        }
        Self { trits }
    }

    pub fn to_decimal(&self) -> i8 {
        self.trits[0] + self.trits[1] * 3 + self.trits[2] * 9
    }

    /// 2진 바이트로 패킹 (6 bits used, MST first)
    pub fn to_packed_byte(&self) -> u8 {
        let mut byte: u8 = 0;
        for i in (0..3).rev() {
            let bits = match self.trits[i] {
                -1 => 0b00u8,
                0 => 0b01,
                1 => 0b10,
                _ => 0b11,
            };
            byte = (byte << 2) | bits;
        }
        byte
    }

    pub fn from_packed_byte(byte: u8) -> Self {
        let mut trits = [0i8; 3];
        let mut b = byte;
        for i in 0..3 {
            trits[i] = match b & 0b11 {
                0b00 => -1,
                0b01 => 0,
                0b10 => 1,
                _ => 0, // 무효 → O
            };
            b >>= 2;
        }
        Self { trits }
    }
}

impl std::fmt::Display for Tryte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in (0..3).rev() {
            match self.trits[i] {
                -1 => write!(f, "T")?,
                0 => write!(f, "O")?,
                1 => write!(f, "P")?,
                _ => write!(f, "?")?,
            }
        }
        Ok(())
    }
}

/// TritWord = 6 Trits (범위: -364 ~ +364, 729가지)
/// = opcode 단위
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TritWord {
    pub trits: [i8; 6],
}

impl TritWord {
    pub fn from_decimal(mut val: i16) -> Self {
        assert!((-364..=364).contains(&val), "TritWord 범위 초과: {}", val);
        let mut trits = [0i8; 6];
        for i in 0..6 {
            let mut r = (val % 3) as i8;
            val /= 3;
            if r > 1 { r -= 3; val += 1; }
            else if r < -1 { r += 3; val -= 1; }
            trits[i] = r;
        }
        Self { trits }
    }

    pub fn to_decimal(&self) -> i16 {
        let mut val: i16 = 0;
        let mut base: i16 = 1;
        for i in 0..6 {
            val += self.trits[i] as i16 * base;
            base *= 3;
        }
        val
    }

    /// 2바이트(12 bits)로 패킹
    pub fn to_packed_u16(&self) -> u16 {
        let mut word: u16 = 0;
        for i in (0..6).rev() {
            let bits: u16 = match self.trits[i] {
                -1 => 0b00,
                0 => 0b01,
                1 => 0b10,
                _ => 0b11,
            };
            word = (word << 2) | bits;
        }
        word
    }

    pub fn from_packed_u16(word: u16) -> Self {
        let mut trits = [0i8; 6];
        let mut w = word;
        for i in 0..6 {
            trits[i] = match w & 0b11 {
                0b00 => -1,
                0b01 => 0,
                0b10 => 1,
                _ => 0,
            };
            w >>= 2;
        }
        Self { trits }
    }

    /// opcode 분해: (sector, group, command) 각 0..8
    pub fn decode_opcode(&self) -> (u8, u8, u8) {
        let sector = (self.trits[5] * 3 + self.trits[4] + 4) as u8;
        let group  = (self.trits[3] * 3 + self.trits[2] + 4) as u8;
        let cmd    = (self.trits[1] * 3 + self.trits[0] + 4) as u8;
        (sector, group, cmd)
    }
}

impl std::fmt::Display for TritWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in (0..6).rev() {
            match self.trits[i] {
                -1 => write!(f, "T")?,
                0 => write!(f, "O")?,
                1 => write!(f, "P")?,
                _ => write!(f, "?")?,
            }
        }
        Ok(())
    }
}

/// TritDWord = 12 Trits (범위: ±265720)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TritDWord {
    pub trits: [i8; 12],
}

impl TritDWord {
    pub fn from_decimal(mut val: i32) -> Self {
        assert!((-265720..=265720).contains(&val), "TritDWord 범위 초과");
        let mut trits = [0i8; 12];
        for i in 0..12 {
            let mut r = (val % 3) as i8;
            val /= 3;
            if r > 1 { r -= 3; val += 1; }
            else if r < -1 { r += 3; val -= 1; }
            trits[i] = r;
        }
        Self { trits }
    }

    pub fn to_decimal(&self) -> i32 {
        let mut val: i32 = 0;
        let mut base: i32 = 1;
        for i in 0..12 {
            val += self.trits[i] as i32 * base;
            base *= 3;
        }
        val
    }

    /// 3바이트(24 bits)로 패킹
    pub fn to_packed_bytes(&self) -> [u8; 3] {
        let mut bits: u32 = 0;
        for i in (0..12).rev() {
            let b: u32 = match self.trits[i] {
                -1 => 0b00,
                0 => 0b01,
                1 => 0b10,
                _ => 0b11,
            };
            bits = (bits << 2) | b;
        }
        [(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]
    }
}

impl std::fmt::Display for TritDWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for i in (0..12).rev() {
            match self.trits[i] {
                -1 => write!(f, "T")?,
                0 => write!(f, "O")?,
                1 => write!(f, "P")?,
                _ => write!(f, "?")?,
            }
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────
// FPGA Register Model (미래 하드웨어 매핑)
// ─────────────────────────────────────────────

/// FPGA 레지스터 뱅크 — 9개 레지스터 (3진답게)
/// 각 레지스터 = 24 Trits = TritDWord × 2
pub struct FpgaRegisterBank {
    /// R0~R8: 각 12 trit (TritDWord)
    pub regs: [TritDWord; 9],
    /// 프로그램 카운터: 12 trit
    pub pc: TritDWord,
    /// 상태 레지스터: 3 trit (플래그 3개)
    ///   [0] = 비교 결과 (P/O/T)
    ///   [1] = 오버플로우 (P=발생, O=없음, T=언더플로우)
    ///   [2] = 인터럽트 (P=활성, O=비활성, T=마스크됨)
    pub status: Tryte,
}

impl FpgaRegisterBank {
    pub fn new() -> Self {
        Self {
            regs: [TritDWord { trits: [0; 12] }; 9],
            pc: TritDWord { trits: [0; 12] },
            status: Tryte { trits: [0; 3] },
        }
    }

    /// 전체 레지스터 바이트 수 (패킹 모드)
    /// 9 regs × 3 bytes + pc 3 bytes + status 1 byte = 31 bytes
    pub fn packed_size() -> usize {
        9 * 3 + 3 + 1 // 31 bytes
    }

    pub fn dump(&self) {
        println!("╔══ FPGA 레지스터 뱅크 ═══════════════════╗");
        for i in 0..9 {
            let val = self.regs[i].to_decimal();
            if val != 0 {
                println!("║ R{}: {} (={})", i, self.regs[i], val);
            }
        }
        println!("║ PC: {} (={})", self.pc, self.pc.to_decimal());
        println!("║ SR: {} [비교:{} 오버:{} 인터:{}]",
            self.status,
            match self.status.trits[0] { 1=>"P", -1=>"T", _=>"O" },
            match self.status.trits[1] { 1=>"P", -1=>"T", _=>"O" },
            match self.status.trits[2] { 1=>"P", -1=>"T", _=>"O" },
        );
        println!("╚═══════════════════════════════════════════╝");
    }
}

// ─────────────────────────────────────────────
// Memory Map (FPGA 메모리 모델)
// ─────────────────────────────────────────────

/// FPGA 메모리 영역
/// 3^12 = 531441 trit-addressable locations (12-trit 주소)
/// 실제 2진 메모리: 531441 × 3 bytes = ~1.5 MB
pub struct TritMemory {
    /// 메모리 (각 위치 = TritDWord = 12 trits)
    data: Vec<i8>,
    /// 크기 (trit 단위)
    size: usize,
}

impl TritMemory {
    /// 크기를 trit 단위로 지정하여 생성
    pub fn new(size_trits: usize) -> Self {
        Self {
            data: vec![0i8; size_trits],
            size: size_trits,
        }
    }

    /// 3진 주소로 단일 trit 읽기
    pub fn read_trit(&self, addr: usize) -> i8 {
        if addr < self.size { self.data[addr] } else { 0 }
    }

    /// 3진 주소로 단일 trit 쓰기
    pub fn write_trit(&mut self, addr: usize, val: i8) {
        if addr < self.size {
            self.data[addr] = val.clamp(-1, 1);
        }
    }

    /// TritWord(6 trits) 읽기
    pub fn read_word(&self, addr: usize) -> TritWord {
        let mut trits = [0i8; 6];
        for i in 0..6 {
            trits[i] = self.read_trit(addr + i);
        }
        TritWord { trits }
    }

    /// TritWord(6 trits) 쓰기
    pub fn write_word(&mut self, addr: usize, word: &TritWord) {
        for i in 0..6 {
            self.write_trit(addr + i, word.trits[i]);
        }
    }

    /// TritDWord(12 trits) 읽기
    pub fn read_dword(&self, addr: usize) -> TritDWord {
        let mut trits = [0i8; 12];
        for i in 0..12 {
            trits[i] = self.read_trit(addr + i);
        }
        TritDWord { trits }
    }

    /// TritDWord(12 trits) 쓰기
    pub fn write_dword(&mut self, addr: usize, dword: &TritDWord) {
        for i in 0..12 {
            self.write_trit(addr + i, dword.trits[i]);
        }
    }

    /// 사용량
    pub fn used_trits(&self) -> usize {
        self.data.iter().filter(|&&t| t != 0).count()
    }

    /// 2진 환산 바이트 (패킹 시)
    pub fn packed_bytes(&self) -> usize {
        (self.size + 3) / 4  // 4 trits per byte
    }

    pub fn dump(&self, start: usize, count: usize) {
        println!("╔══ 3진 메모리 (시작: {}, 표시: {}) ══╗", start, count);
        let end = (start + count).min(self.size);
        for addr in (start..end).step_by(6) {
            let word = self.read_word(addr);
            let val = word.to_decimal();
            if val != 0 {
                println!("║ [{:06}] {} = {}", addr, word, val);
            }
        }
        println!("║ 사용: {}/{} trits ({}%)",
            self.used_trits(), self.size,
            self.used_trits() * 100 / self.size.max(1));
        println!("║ 2진 환산: {} bytes (packed)", self.packed_bytes());
        println!("╚═══════════════════════════════════════════╝");
    }
}

// ─────────────────────────────────────────────
// FPGA Transition Roadmap
// ─────────────────────────────────────────────

/// FPGA 전환 단계
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionPhase {
    /// Phase 1: 순수 소프트웨어 에뮬레이션 (현재)
    SoftwareEmulation,
    /// Phase 2: FPGA 프로토타입 (3진 ALU)
    FpgaPrototype,
    /// Phase 3: FPGA 완전 구현 (3진 CPU)
    FpgaFull,
    /// Phase 4: ASIC 전환 (양산)
    AsicProduction,
}

impl std::fmt::Display for TransitionPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransitionPhase::SoftwareEmulation =>
                write!(f, "Phase 1: 소프트웨어 에뮬레이션 (2진 CPU 위 Rust)"),
            TransitionPhase::FpgaPrototype =>
                write!(f, "Phase 2: FPGA 프로토타입 (3진 ALU + 2진 제어)"),
            TransitionPhase::FpgaFull =>
                write!(f, "Phase 3: FPGA 완전 (3진 CPU + 3진 메모리)"),
            TransitionPhase::AsicProduction =>
                write!(f, "Phase 4: ASIC 양산 (균형3진 전용 칩)"),
        }
    }
}

/// 로드맵 정보 출력
pub fn print_roadmap() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║          CROWNIN FPGA 이전 로드맵                             ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║                                                             ║");
    println!("║  Phase 1: 소프트웨어 에뮬레이션 ← [현재]                      ║");
    println!("║  ├─ 1 Trit = 1 byte (i8) 또는 2-bit packed                  ║");
    println!("║  ├─ Rust TVM이 729 opcode 실행                              ║");
    println!("║  ├─ 모든 3진 논리를 2진 CPU에서 에뮬레이트                      ║");
    println!("║  └─ 상위 API는 100% 3진 인터페이스                            ║");
    println!("║                                                             ║");
    println!("║  Phase 2: FPGA 프로토타입                                    ║");
    println!("║  ├─ Xilinx/Intel FPGA에 3진 ALU 구현                        ║");
    println!("║  ├─ 2-bit 매핑: T=00, O=01, P=10                           ║");
    println!("║  ├─ 3진 가산기/승산기 하드웨어 구현                             ║");
    println!("║  └─ TVM 명령어 일부를 FPGA 가속                              ║");
    println!("║                                                             ║");
    println!("║  Phase 3: FPGA 완전 구현                                     ║");
    println!("║  ├─ 3진 CPU: 9 레지스터, 12-trit 주소 버스                    ║");
    println!("║  ├─ 3진 메모리: 3^12 = 531,441 trit 주소공간                 ║");
    println!("║  ├─ 3진 I/O 버스: CTP 프로토콜 네이티브                       ║");
    println!("║  └─ Crowny Kernel이 FPGA 위에서 직접 실행                    ║");
    println!("║                                                             ║");
    println!("║  Phase 4: ASIC 양산                                         ║");
    println!("║  ├─ 균형3진 전용 칩 설계                                      ║");
    println!("║  ├─ 3진 메모리 셀 (TRAM)                                     ║");
    println!("║  └─ 완전한 균형3진 컴퓨터                                     ║");
    println!("║                                                             ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║  물리 매핑 규격:                                              ║");
    println!("║  ┌────────┬────────┬────────┬─────────┐                      ║");
    println!("║  │  단위   │ Trits  │ 2진 Bits│  범위   │                      ║");
    println!("║  ├────────┼────────┼────────┼─────────┤                      ║");
    println!("║  │ Trit   │   1    │   2    │ -1~+1   │                      ║");
    println!("║  │ Tryte  │   3    │   6    │ ±13     │                      ║");
    println!("║  │ Word   │   6    │  12    │ ±364    │                      ║");
    println!("║  │ DWord  │  12    │  24    │ ±265720 │                      ║");
    println!("║  │ QWord  │  24    │  48    │ ±~1.4억  │                      ║");
    println!("║  └────────┴────────┴────────┴─────────┘                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tryte_roundtrip() {
        for v in -13..=13i8 {
            let t = Tryte::from_decimal(v);
            assert_eq!(t.to_decimal(), v, "Tryte failed at {}", v);

            let packed = t.to_packed_byte();
            let restored = Tryte::from_packed_byte(packed);
            assert_eq!(restored.to_decimal(), v, "Tryte pack/unpack failed at {}", v);
        }
    }

    #[test]
    fn test_tritword_roundtrip() {
        for v in [-364, -100, -1, 0, 1, 42, 100, 364] {
            let w = TritWord::from_decimal(v);
            assert_eq!(w.to_decimal(), v, "TritWord failed at {}", v);

            let packed = w.to_packed_u16();
            let restored = TritWord::from_packed_u16(packed);
            assert_eq!(restored.to_decimal(), v, "TritWord pack failed at {}", v);
        }
    }

    #[test]
    fn test_tritdword_roundtrip() {
        for v in [-265720, -1000, 0, 1000, 265720] {
            let d = TritDWord::from_decimal(v);
            assert_eq!(d.to_decimal(), v, "TritDWord failed at {}", v);
        }
    }

    #[test]
    fn test_trit_memory() {
        let mut mem = TritMemory::new(729 * 6); // 729 words
        let word = TritWord::from_decimal(42);
        mem.write_word(0, &word);
        let read = mem.read_word(0);
        assert_eq!(read.to_decimal(), 42);
    }

    #[test]
    fn test_fpga_registers() {
        let mut bank = FpgaRegisterBank::new();
        bank.regs[0] = TritDWord::from_decimal(12345);
        bank.pc = TritDWord::from_decimal(100);
        bank.status = Tryte::from_decimal(1); // 비교=P

        assert_eq!(bank.regs[0].to_decimal(), 12345);
        assert_eq!(bank.pc.to_decimal(), 100);
    }
}
