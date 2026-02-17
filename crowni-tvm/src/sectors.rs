///! ═══════════════════════════════════════════════════
///! 섹터 확장 — 나머지 8섹터 (1~8) × 81 = 648 opcodes
///! ═══════════════════════════════════════════════════
///!
///! 섹터 배치:
///!   0: 코어(Kernel)       — 기존 구현 완료
///!   1: 지능(Intelligence) — AI/LLM/추론
///!   2: 하드웨어(Hardware)  — FPGA/센서/GPIO
///!   3: 기억(Memory)       — 고급 메모리/캐시/GC
///!   4: 표현(Expression)   — 문자열/정규식/포맷
///!   5: 초월(Transcendence)— 암호/해시/양자
///!   6: 보안(Security)     — 인증/권한/감사
///!   7: 메타(Meta)         — 리플렉션/디버그/프로파일
///!   8: 확장(User)         — 사용자 정의/플러그인

use std::collections::HashMap;
use crate::opcode::{OpcodeAddr, OpMeta, Effect};

macro_rules! op {
    ($kr:expr, $en:expr, $pop:expr, $push:expr, $oper:expr, $eff:expr) => {
        OpMeta { name_kr: $kr, name_en: $en, pops: $pop, pushes: $push, operands: $oper, effect: $eff }
    };
}

/// 전체 9섹터 729 opcodes 빌드
pub fn build_all_sectors() -> HashMap<OpcodeAddr, OpMeta> {
    let mut m = crate::opcode::build_opcodes(); // 섹터 0 (81개)

    build_sector_1_intelligence(&mut m);
    build_sector_2_hardware(&mut m);
    build_sector_3_memory(&mut m);
    build_sector_4_expression(&mut m);
    build_sector_5_transcendence(&mut m);
    build_sector_6_security(&mut m);
    build_sector_7_meta(&mut m);
    build_sector_8_user(&mut m);

    m
}

// ═══════════════════════════════════════════════
// 섹터 1: 지능 (Intelligence) — AI/LLM/추론
// ═══════════════════════════════════════════════

fn build_sector_1_intelligence(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 1u8;

    // G0: LLM 호출
    m.insert(OpcodeAddr::new(s,0,0), op!("질문해",     "LLM_ASK",     1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,1), op!("요약해",     "LLM_SUMMARY", 1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,2), op!("번역해",     "LLM_TRANSLATE",2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,3), op!("분류해",     "LLM_CLASSIFY",1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,4), op!("생성해",     "LLM_GENERATE",1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,5), op!("임베딩",     "LLM_EMBED",   1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,6), op!("모델선택",   "LLM_SELECT",  1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,7), op!("온도설정",   "LLM_TEMP",    1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,8), op!("토큰제한",   "LLM_MAXTOK",  1,0,0, Effect::IO));

    // G1: 추론
    m.insert(OpcodeAddr::new(s,1,0), op!("추론해",     "INFER",    1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,1), op!("패턴찾기",   "PATTERN",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,2), op!("확률",       "PROB",     1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,3), op!("예측해",     "PREDICT",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,4), op!("군집화",     "CLUSTER",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,5), op!("유사도",     "SIMILAR",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,6), op!("순위매겨",   "RANK",     1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,7), op!("가중치",     "WEIGHT",   2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,8), op!("활성화",     "ACTIVATE", 1,1,0, Effect::Stack));

    // G2: 텐서/행렬
    m.insert(OpcodeAddr::new(s,2,0), op!("텐서생성",   "TENSOR",    1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,2,1), op!("행렬곱",     "MATMUL",    2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,2), op!("전치",       "TRANSPOSE", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,3), op!("내적",       "DOT",       2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,4), op!("외적",       "CROSS",     2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,5), op!("정규화",     "NORMALIZE", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,6), op!("소프트맥스", "SOFTMAX",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,7), op!("ReLU",       "RELU",      1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,8), op!("시그모이드", "SIGMOID",   1,1,0, Effect::Stack));

    // G3: 학습
    m.insert(OpcodeAddr::new(s,3,0), op!("학습시작",   "TRAIN_START",0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,1), op!("학습중지",   "TRAIN_STOP", 0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,2), op!("손실계산",   "LOSS",       2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,3,3), op!("역전파",     "BACKPROP",   0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,4), op!("경사하강",   "GRAD_DESC",  1,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,5), op!("에폭",       "EPOCH",      1,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,6), op!("배치",       "BATCH",      1,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,7), op!("학습률",     "LR",         1,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,3,8), op!("모델저장",   "SAVE_MODEL", 1,0,0, Effect::IO));

    // G4: 자연어
    m.insert(OpcodeAddr::new(s,4,0), op!("토크나이즈", "TOKENIZE",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,1), op!("형태소",     "MORPHEME",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,2), op!("개체명",     "NER",       1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,3), op!("감정분석",   "SENTIMENT", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,4), op!("키워드",     "KEYWORD",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,5), op!("문장분리",   "SENT_SPLIT",1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,6), op!("품사",       "POS_TAG",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,7), op!("구문분석",   "PARSE_NL",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,4,8), op!("의도파악",   "INTENT",    1,1,0, Effect::Stack));

    // G5~G8: 예약 (지능 확장용)
    for g in 5..=8 {
        for c in 0..=8 {
            let name_kr = format!("지능{}_{}", g, c);
            let name_en = format!("INTEL_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(name_kr.into_boxed_str()),
                name_en: Box::leak(name_en.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 2: 하드웨어 (Hardware) — FPGA/센서/GPIO
// ═══════════════════════════════════════════════

fn build_sector_2_hardware(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 2u8;

    // G0: FPGA
    m.insert(OpcodeAddr::new(s,0,0), op!("칩초기화",   "FPGA_INIT",   0,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,1), op!("칩쓰기",     "FPGA_WRITE",  2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,2), op!("칩읽기",     "FPGA_READ",   1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,3), op!("칩리셋",     "FPGA_RESET",  0,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,4), op!("레지스터",   "FPGA_REG",    2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,5), op!("클럭설정",   "FPGA_CLK",    1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,6), op!("비트스트림", "FPGA_BITS",   1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,7), op!("삼진ALU",    "FPGA_TALU",   2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,8), op!("칩상태",     "FPGA_STATUS", 0,1,0, Effect::IO));

    // G1: GPIO
    m.insert(OpcodeAddr::new(s,1,0), op!("핀설정",     "GPIO_SET",    2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,1), op!("핀읽기",     "GPIO_READ",   1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,2), op!("핀쓰기",     "GPIO_WRITE",  2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,3), op!("PWM",        "GPIO_PWM",    2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,4), op!("ADC",        "GPIO_ADC",    1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,5), op!("DAC",        "GPIO_DAC",    2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,6), op!("인터럽트",   "GPIO_IRQ",    2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,7), op!("I2C",        "GPIO_I2C",    2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,8), op!("SPI",        "GPIO_SPI",    2,1,0, Effect::IO));

    // G2~G8: 하드웨어 예약
    for g in 2..=8 {
        for c in 0..=8 {
            let nk = format!("하드{}_{}", g, c);
            let ne = format!("HW_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 3: 기억 (Memory) — 고급 메모리/캐시/GC
// ═══════════════════════════════════════════════

fn build_sector_3_memory(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 3u8;

    // G0: 캐시
    m.insert(OpcodeAddr::new(s,0,0), op!("캐시읽기",   "CACHE_GET",   1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,0,1), op!("캐시쓰기",   "CACHE_SET",   2,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,0,2), op!("캐시삭제",   "CACHE_DEL",   1,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,0,3), op!("캐시비움",   "CACHE_CLEAR", 0,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,0,4), op!("캐시크기",   "CACHE_SIZE",  0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,5), op!("캐시TTL",    "CACHE_TTL",   2,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,0,6), op!("캐시존재",   "CACHE_HAS",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,7), op!("캐시키목록", "CACHE_KEYS",  0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,8), op!("캐시통계",   "CACHE_STATS", 0,1,0, Effect::Stack));

    // G1: GC
    m.insert(OpcodeAddr::new(s,1,0), op!("GC실행",     "GC_RUN",    0,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,1,1), op!("GC통계",     "GC_STATS",  0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,2), op!("GC임계",     "GC_THRESH", 1,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,1,3), op!("참조수",     "REF_COUNT", 1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,4), op!("약참조",     "WEAK_REF",  1,1,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,1,5), op!("고정해",     "PIN",       1,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,1,6), op!("풀어줘",     "UNPIN",     1,0,0, Effect::Heap));
    m.insert(OpcodeAddr::new(s,1,7), op!("메모리통계", "MEM_STATS", 0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,8), op!("메모리한계", "MEM_LIMIT", 1,0,0, Effect::Heap));

    // G2~G8: 기억 예약
    for g in 2..=8 {
        for c in 0..=8 {
            let nk = format!("기억{}_{}", g, c);
            let ne = format!("MEM_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 4: 표현 (Expression) — 문자열/정규식/포맷
// ═══════════════════════════════════════════════

fn build_sector_4_expression(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 4u8;

    // G0: 문자열
    m.insert(OpcodeAddr::new(s,0,0), op!("연결",       "STR_CAT",    2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,1), op!("자르기",     "STR_SLICE",  3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,2), op!("찾기",       "STR_FIND",   2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,3), op!("바꾸기",     "STR_REPLACE",3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,4), op!("대문자",     "STR_UPPER",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,5), op!("소문자",     "STR_LOWER",  1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,6), op!("공백제거",   "STR_TRIM",   1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,7), op!("분할",       "STR_SPLIT",  2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,8), op!("포맷",       "STR_FMT",    2,1,0, Effect::Stack));

    // G1: 정규식
    m.insert(OpcodeAddr::new(s,1,0), op!("정규매치",   "RE_MATCH",   2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,1), op!("정규찾기",   "RE_FIND",    2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,2), op!("정규모두",   "RE_FINDALL", 2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,3), op!("정규바꿈",   "RE_REPLACE", 3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,1,4), op!("정규분할",   "RE_SPLIT",   2,1,0, Effect::Stack));
    for c in 5..=8 {
        let nk = format!("표현1_{}", c);
        let ne = format!("EXPR_1_{}", c);
        m.insert(OpcodeAddr::new(s, 1, c), OpMeta {
            name_kr: Box::leak(nk.into_boxed_str()),
            name_en: Box::leak(ne.into_boxed_str()),
            pops: 0, pushes: 0, operands: 0, effect: Effect::None,
        });
    }

    // G2: JSON/직렬화
    m.insert(OpcodeAddr::new(s,2,0), op!("JSON파싱",   "JSON_PARSE",    1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,1), op!("JSON생성",   "JSON_STRINGIFY",1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,2), op!("JSON읽기",   "JSON_GET",      2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,2,3), op!("JSON설정",   "JSON_SET",      3,1,0, Effect::Stack));
    for c in 4..=8 {
        let nk = format!("표현2_{}", c);
        let ne = format!("EXPR_2_{}", c);
        m.insert(OpcodeAddr::new(s, 2, c), OpMeta {
            name_kr: Box::leak(nk.into_boxed_str()),
            name_en: Box::leak(ne.into_boxed_str()),
            pops: 0, pushes: 0, operands: 0, effect: Effect::None,
        });
    }

    // G3~G8: 표현 예약
    for g in 3..=8 {
        for c in 0..=8 {
            let nk = format!("표현{}_{}", g, c);
            let ne = format!("EXPR_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 5: 초월 (Transcendence) — 암호/해시/양자
// ═══════════════════════════════════════════════

fn build_sector_5_transcendence(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 5u8;

    // G0: 해시/암호
    m.insert(OpcodeAddr::new(s,0,0), op!("해시",       "HASH",        1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,1), op!("SHA256",     "SHA256",      1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,2), op!("HMAC",       "HMAC",        2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,3), op!("암호화",     "ENCRYPT",     2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,4), op!("복호화",     "DECRYPT",     2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,5), op!("서명",       "SIGN",        2,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,6), op!("검증",       "VERIFY",      3,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,7), op!("키생성",     "KEYGEN",      1,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,8), op!("랜덤",       "RANDOM",      1,1,0, Effect::Stack));

    // G1~G8: 초월 예약
    for g in 1..=8 {
        for c in 0..=8 {
            let nk = format!("초월{}_{}", g, c);
            let ne = format!("TRANS_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 6: 보안 (Security) — 인증/권한/감사
// ═══════════════════════════════════════════════

fn build_sector_6_security(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 6u8;

    // G0: 인증
    m.insert(OpcodeAddr::new(s,0,0), op!("로그인",     "AUTH_LOGIN",  2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,1), op!("로그아웃",   "AUTH_LOGOUT", 0,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,2), op!("토큰생성",   "AUTH_TOKEN",  1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,3), op!("토큰검증",   "AUTH_VERIFY", 1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,4), op!("권한확인",   "AUTH_CHECK",  2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,5), op!("역할부여",   "AUTH_ROLE",   2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,6), op!("세션생성",   "SESSION_NEW", 0,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,7), op!("세션읽기",   "SESSION_GET", 1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,8), op!("세션삭제",   "SESSION_DEL", 1,0,0, Effect::IO));

    // G1: 감사
    m.insert(OpcodeAddr::new(s,1,0), op!("감사기록",   "AUDIT_LOG",   1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,1), op!("감사조회",   "AUDIT_QUERY", 1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,2), op!("정책설정",   "POLICY_SET",  2,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,1,3), op!("정책확인",   "POLICY_CHECK",1,1,0, Effect::IO));
    for c in 4..=8 {
        let nk = format!("보안1_{}", c);
        let ne = format!("SEC_1_{}", c);
        m.insert(OpcodeAddr::new(s, 1, c), OpMeta {
            name_kr: Box::leak(nk.into_boxed_str()),
            name_en: Box::leak(ne.into_boxed_str()),
            pops: 0, pushes: 0, operands: 0, effect: Effect::None,
        });
    }

    // G2~G8: 보안 예약
    for g in 2..=8 {
        for c in 0..=8 {
            let nk = format!("보안{}_{}", g, c);
            let ne = format!("SEC_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 7: 메타 (Meta) — 리플렉션/디버그/프로파일
// ═══════════════════════════════════════════════

fn build_sector_7_meta(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 7u8;

    // G0: 디버그
    m.insert(OpcodeAddr::new(s,0,0), op!("스택덤프",   "DBG_STACK",  0,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,1), op!("힙덤프",     "DBG_HEAP",   0,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,2), op!("중단점",     "BREAKPOINT", 0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,0,3), op!("단계실행",   "STEP",       0,0,0, Effect::Control));
    m.insert(OpcodeAddr::new(s,0,4), op!("시간측정",   "TIMER",      0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,5), op!("프로파일",   "PROFILE",    0,0,0, Effect::Meta));
    m.insert(OpcodeAddr::new(s,0,6), op!("타임스탬프", "TIMESTAMP",  0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,7), op!("버전",       "VERSION",    0,1,0, Effect::Stack));
    m.insert(OpcodeAddr::new(s,0,8), op!("정보",       "INFO",       0,1,0, Effect::Stack));

    // G1~G8: 메타 예약
    for g in 1..=8 {
        for c in 0..=8 {
            let nk = format!("메타{}_{}", g, c);
            let ne = format!("META_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

// ═══════════════════════════════════════════════
// 섹터 8: 확장 (User) — 사용자 정의/플러그인
// ═══════════════════════════════════════════════

fn build_sector_8_user(m: &mut HashMap<OpcodeAddr, OpMeta>) {
    let s = 8u8;

    // G0: 플러그인
    m.insert(OpcodeAddr::new(s,0,0), op!("플러그인",   "PLUGIN_LOAD",  1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,1), op!("플러그해제", "PLUGIN_UNLOAD",1,0,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,2), op!("플러그호출", "PLUGIN_CALL",  2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,3), op!("플러그목록", "PLUGIN_LIST",  0,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,4), op!("외부함수",   "FFI_CALL",     2,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,5), op!("WASM로드",   "WASM_LOAD",    1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,6), op!("WASM실행",   "WASM_EXEC",    1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,7), op!("시스템호출", "SYSCALL",      1,1,0, Effect::IO));
    m.insert(OpcodeAddr::new(s,0,8), op!("셸실행",     "SHELL",        1,1,0, Effect::IO));

    // G1~G8: 사용자 예약 (완전 개방)
    for g in 1..=8 {
        for c in 0..=8 {
            let nk = format!("사용자{}_{}", g, c);
            let ne = format!("USER_{}_{}", g, c);
            m.insert(OpcodeAddr::new(s, g, c), OpMeta {
                name_kr: Box::leak(nk.into_boxed_str()),
                name_en: Box::leak(ne.into_boxed_str()),
                pops: 0, pushes: 0, operands: 0, effect: Effect::None,
            });
        }
    }
}

/// 섹터 통계
pub fn sector_stats(map: &HashMap<OpcodeAddr, OpMeta>) -> Vec<(u8, &'static str, usize, usize)> {
    let sector_names = [
        "코어","지능","하드웨어","기억","표현","초월","보안","메타","확장"
    ];
    let mut stats = Vec::new();
    for s in 0..9u8 {
        let total = (0..9).flat_map(|g| (0..9).map(move |c| OpcodeAddr::new(s,g,c)))
            .filter(|a| map.contains_key(a))
            .count();
        let active = (0..9).flat_map(|g| (0..9).map(move |c| OpcodeAddr::new(s,g,c)))
            .filter(|a| map.get(a).map(|m| m.effect != Effect::None).unwrap_or(false))
            .count();
        stats.push((s, sector_names[s as usize], total, active));
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_729() {
        let map = build_all_sectors();
        assert_eq!(map.len(), 729, "전체 729 opcodes 필요, 실제: {}", map.len());
    }

    #[test]
    fn test_sector_stats() {
        let map = build_all_sectors();
        let stats = sector_stats(&map);
        for (s, name, total, active) in &stats {
            assert_eq!(*total, 81, "섹터 {} ({}) = {} opcodes", s, name, total);
            println!("  [{}] {} — {}/81 활성", s, name, active);
        }
    }
}
