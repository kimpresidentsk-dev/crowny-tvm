//! # crowny-sdk
//!
//! Crowny 균형3진 플랫폼 Rust SDK
//!
//! ## 사용법
//!
//! ```rust,no_run
//! use crowny_sdk::{CrownyClient, Trit};
//!
//! let mut client = CrownyClient::new("http://localhost:7293");
//! let result = client.run("넣어 42\n종료");
//! println!("{}", result.state);
//! ```

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// ═══════════════════════════════════════════════
// Trit
// ═══════════════════════════════════════════════

/// 균형3진 값: P(+1), O(0), T(-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trit {
    /// +1 성공/참/승인
    P,
    /// 0 보류/모름/대기
    O,
    /// -1 실패/거짓/거부
    T,
}

impl Trit {
    pub fn from_i8(n: i8) -> Self {
        if n > 0 { Trit::P } else if n < 0 { Trit::T } else { Trit::O }
    }

    pub fn to_i8(self) -> i8 {
        match self { Trit::P => 1, Trit::O => 0, Trit::T => -1 }
    }

    pub fn to_korean(self) -> &'static str {
        match self { Trit::P => "성공", Trit::O => "보류", Trit::T => "실패" }
    }

    pub fn not(self) -> Self {
        match self { Trit::P => Trit::T, Trit::T => Trit::P, Trit::O => Trit::O }
    }

    pub fn and(self, other: Self) -> Self {
        Trit::from_i8(self.to_i8().min(other.to_i8()))
    }

    pub fn or(self, other: Self) -> Self {
        Trit::from_i8(self.to_i8().max(other.to_i8()))
    }

    /// 다수결 합의
    pub fn consensus(trits: &[Trit]) -> Trit {
        let p = trits.iter().filter(|t| **t == Trit::P).count();
        let t = trits.iter().filter(|t| **t == Trit::T).count();
        if p > t { Trit::P } else if t > p { Trit::T } else { Trit::O }
    }

    pub fn from_str(s: &str) -> Self {
        let s = s.to_uppercase();
        if s.contains('P') || s.contains("성공") || s.contains("SUCCESS") { Trit::P }
        else if s.contains('T') || s.contains("실패") || s.contains("FAILED") { Trit::T }
        else { Trit::O }
    }
}

impl fmt::Display for Trit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Trit::P => write!(f, "P"), Trit::O => write!(f, "O"), Trit::T => write!(f, "T") }
    }
}

// ═══════════════════════════════════════════════
// TritResult
// ═══════════════════════════════════════════════

/// 표준 반환 구조체
#[derive(Debug, Clone)]
pub struct TritResult {
    pub state: Trit,
    pub data: ResultData,
    pub elapsed_ms: u64,
    pub task_id: u64,
}

impl TritResult {
    pub fn success(data: ResultData, elapsed_ms: u64, task_id: u64) -> Self {
        Self { state: Trit::P, data, elapsed_ms, task_id }
    }
    pub fn pending(data: ResultData, elapsed_ms: u64, task_id: u64) -> Self {
        Self { state: Trit::O, data, elapsed_ms, task_id }
    }
    pub fn failed(data: ResultData, elapsed_ms: u64, task_id: u64) -> Self {
        Self { state: Trit::T, data, elapsed_ms, task_id }
    }
    pub fn is_success(&self) -> bool { self.state == Trit::P }
    pub fn is_pending(&self) -> bool { self.state == Trit::O }
    pub fn is_failed(&self) -> bool { self.state == Trit::T }
}

/// 반환 데이터 타입
#[derive(Debug, Clone)]
pub enum ResultData {
    None,
    Integer(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Trit(i8),
    Json(String),
}

impl fmt::Display for ResultData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResultData::None => write!(f, "None"),
            ResultData::Integer(n) => write!(f, "{}", n),
            ResultData::Float(n) => write!(f, "{}", n),
            ResultData::Text(s) => write!(f, "{}", s),
            ResultData::Bytes(b) => write!(f, "[{} bytes]", b.len()),
            ResultData::Trit(t) => write!(f, "Trit({})", t),
            ResultData::Json(s) => write!(f, "{}", s),
        }
    }
}

// ═══════════════════════════════════════════════
// CTP Header
// ═══════════════════════════════════════════════

/// 9-Trit CTP 프로토콜 헤더
#[derive(Debug, Clone)]
pub struct CtpHeader {
    pub trits: [Trit; 9],
}

impl CtpHeader {
    pub fn new() -> Self {
        Self { trits: [Trit::O; 9] }
    }

    pub fn success() -> Self {
        let mut h = Self::new();
        h.trits[0] = Trit::P;
        h.trits[1] = Trit::P;
        h.trits[2] = Trit::P;
        h
    }

    pub fn failed() -> Self {
        let mut h = Self::new();
        h.trits[0] = Trit::T;
        h.trits[1] = Trit::T;
        h.trits[2] = Trit::T;
        h
    }

    pub fn parse(s: &str) -> Self {
        let mut h = Self::new();
        for (i, c) in s.chars().enumerate() {
            if i >= 9 { break; }
            h.trits[i] = match c {
                'P' | '+' | '1' => Trit::P,
                'T' | '-' => Trit::T,
                _ => Trit::O,
            };
        }
        h
    }

    pub fn overall_state(&self) -> Trit {
        if self.trits.iter().any(|t| *t == Trit::T) { return Trit::T; }
        if self.trits.iter().all(|t| *t == Trit::P) { return Trit::P; }
        Trit::O
    }
}

impl fmt::Display for CtpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for t in &self.trits { write!(f, "{}", t)?; }
        Ok(())
    }
}

impl Default for CtpHeader {
    fn default() -> Self { Self::new() }
}

// ═══════════════════════════════════════════════
// CrownyClient
// ═══════════════════════════════════════════════

/// Crowny 서버 클라이언트
pub struct CrownyClient {
    base_url: String,
    timeout: Duration,
    ctp: CtpHeader,
    task_counter: u64,
    history: Vec<TritResult>,
}

impl CrownyClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            timeout: Duration::from_secs(30),
            ctp: CtpHeader::success(),
            task_counter: 0,
            history: Vec::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_ctp(mut self, ctp: CtpHeader) -> Self {
        self.ctp = ctp;
        self
    }

    /// 핵심: CAR.submit() 래핑
    pub fn submit_sync(
        &mut self,
        task_type: &str,
        subject: &str,
        payload: &str,
        params: HashMap<String, String>,
    ) -> TritResult {
        let start = Instant::now();
        self.task_counter += 1;
        let task_id = self.task_counter;

        // HTTP 요청 (blocking — async 버전은 별도)
        let result = match ureq_post(
            &format!("{}/run", self.base_url),
            &self.ctp,
            task_type, subject, payload, &params,
        ) {
            Ok((state, data, resp_ctp)) => {
                if let Some(c) = resp_ctp { self.ctp = c; }
                TritResult { state, data, elapsed_ms: start.elapsed().as_millis() as u64, task_id }
            }
            Err(e) => {
                TritResult::failed(
                    ResultData::Text(e),
                    start.elapsed().as_millis() as u64,
                    task_id,
                )
            }
        };

        self.history.push(result.clone());
        result
    }

    /// 한선어 소스 실행
    pub fn run(&mut self, source: &str) -> TritResult {
        self.submit_sync("execute", "sdk-rs", source, HashMap::new())
    }

    /// WASM 컴파일
    pub fn compile(&mut self, source: &str) -> TritResult {
        self.submit_sync("compile", "sdk-rs", source, HashMap::new())
    }

    /// LLM 호출
    pub fn ask(&mut self, prompt: &str) -> TritResult {
        self.submit_sync("llm", "claude", prompt, HashMap::new())
    }

    /// 특정 모델 LLM 호출
    pub fn ask_model(&mut self, prompt: &str, model: &str) -> TritResult {
        self.submit_sync("llm", model, prompt, HashMap::new())
    }

    /// 다중 모델 합의
    pub fn consensus_call(&mut self, prompt: &str, models: &[&str]) -> ConsensusResult {
        let start = Instant::now();
        let models = if models.is_empty() {
            vec!["claude", "gpt4", "gemini"]
        } else {
            models.to_vec()
        };

        let mut results = Vec::new();
        for model in &models {
            let r = self.ask_model(prompt, model);
            results.push(ModelResult { model: model.to_string(), result: r });
        }

        let trits: Vec<Trit> = results.iter().map(|r| r.result.state).collect();
        let con = Trit::consensus(&trits);

        ConsensusResult {
            consensus: con,
            models: results,
            trits,
            elapsed_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// 서버 핑
    pub fn ping(&mut self) -> TritResult {
        let start = Instant::now();
        let task_id = { self.task_counter += 1; self.task_counter };
        // 간단한 GET 요청 시도
        match std::net::TcpStream::connect_timeout(
            &self.base_url.replace("http://", "").parse().unwrap_or(
                "127.0.0.1:7293".parse().unwrap()
            ),
            Duration::from_secs(5),
        ) {
            Ok(_) => TritResult::success(ResultData::Text("ok".into()), start.elapsed().as_millis() as u64, task_id),
            Err(e) => TritResult::failed(ResultData::Text(e.to_string()), start.elapsed().as_millis() as u64, task_id),
        }
    }

    pub fn history(&self) -> &[TritResult] { &self.history }

    pub fn stats(&self) -> (usize, usize, usize, usize) {
        let total = self.history.len();
        let p = self.history.iter().filter(|r| r.state == Trit::P).count();
        let o = self.history.iter().filter(|r| r.state == Trit::O).count();
        let t = self.history.iter().filter(|r| r.state == Trit::T).count();
        (total, p, o, t)
    }
}

/// 합의 결과
#[derive(Debug)]
pub struct ConsensusResult {
    pub consensus: Trit,
    pub models: Vec<ModelResult>,
    pub trits: Vec<Trit>,
    pub elapsed_ms: u64,
}

/// 단일 모델 결과
#[derive(Debug)]
pub struct ModelResult {
    pub model: String,
    pub result: TritResult,
}

// ── HTTP helper (no external deps) ──

fn ureq_post(
    url: &str,
    ctp: &CtpHeader,
    task_type: &str,
    subject: &str,
    payload: &str,
    _params: &HashMap<String, String>,
) -> Result<(Trit, ResultData, Option<CtpHeader>), String> {
    // Minimal HTTP POST without external deps
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let url_parts: Vec<&str> = url.strip_prefix("http://").unwrap_or(url).splitn(2, '/').collect();
    let host = url_parts[0];
    let path = if url_parts.len() > 1 { format!("/{}", url_parts[1]) } else { "/".to_string() };

    let body = format!(
        r#"{{"type":"{}","subject":"{}","payload":"{}"}}"#,
        task_type,
        subject,
        payload.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
    );

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nX-Crowny-Trit: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        path, host, ctp, body.len(), body
    );

    let mut stream = TcpStream::connect(host).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| e.to_string())?;

    // Parse response CTP header
    let resp_ctp = response.lines()
        .find(|l| l.to_lowercase().starts_with("x-crowny-trit:"))
        .map(|l| CtpHeader::parse(l.split(':').nth(1).unwrap_or("").trim()));

    // Extract body (after \r\n\r\n)
    let body_text = response.split("\r\n\r\n").nth(1).unwrap_or("");

    // Simple state detection
    let state = if body_text.contains("성공") || body_text.contains("Success") || body_text.contains("\"P\"") {
        Trit::P
    } else if body_text.contains("실패") || body_text.contains("Failed") || body_text.contains("\"T\"") {
        Trit::T
    } else {
        Trit::O
    };

    Ok((state, ResultData::Json(body_text.to_string()), resp_ctp))
}

// ═══════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_values() {
        assert_eq!(Trit::P.to_i8(), 1);
        assert_eq!(Trit::O.to_i8(), 0);
        assert_eq!(Trit::T.to_i8(), -1);
    }

    #[test]
    fn test_trit_korean() {
        assert_eq!(Trit::P.to_korean(), "성공");
        assert_eq!(Trit::O.to_korean(), "보류");
        assert_eq!(Trit::T.to_korean(), "실패");
    }

    #[test]
    fn test_trit_not() {
        assert_eq!(Trit::P.not(), Trit::T);
        assert_eq!(Trit::T.not(), Trit::P);
        assert_eq!(Trit::O.not(), Trit::O);
    }

    #[test]
    fn test_trit_and() {
        assert_eq!(Trit::P.and(Trit::P), Trit::P);
        assert_eq!(Trit::P.and(Trit::O), Trit::O);
        assert_eq!(Trit::P.and(Trit::T), Trit::T);
    }

    #[test]
    fn test_trit_or() {
        assert_eq!(Trit::P.or(Trit::T), Trit::P);
        assert_eq!(Trit::T.or(Trit::T), Trit::T);
    }

    #[test]
    fn test_consensus() {
        assert_eq!(Trit::consensus(&[Trit::P, Trit::P, Trit::P]), Trit::P);
        assert_eq!(Trit::consensus(&[Trit::T, Trit::T, Trit::T]), Trit::T);
        assert_eq!(Trit::consensus(&[Trit::P, Trit::P, Trit::T]), Trit::P);
        assert_eq!(Trit::consensus(&[Trit::P, Trit::T, Trit::T]), Trit::T);
        assert_eq!(Trit::consensus(&[Trit::P, Trit::O, Trit::T]), Trit::O);
    }

    #[test]
    fn test_ctp_header() {
        let h = CtpHeader::success();
        assert_eq!(h.trits[0], Trit::P);
        assert_eq!(h.trits[1], Trit::P);
        assert_eq!(h.trits[2], Trit::P);
        assert_eq!(format!("{}", h), "PPPOOOOOO");
    }

    #[test]
    fn test_ctp_parse() {
        let h = CtpHeader::parse("PPTOOOOO0");
        assert_eq!(h.trits[0], Trit::P);
        assert_eq!(h.trits[2], Trit::T);
    }

    #[test]
    fn test_ctp_overall() {
        let h = CtpHeader::failed();
        assert_eq!(h.overall_state(), Trit::T);

        let h2 = CtpHeader::new();
        assert_eq!(h2.overall_state(), Trit::O);
    }

    #[test]
    fn test_trit_result() {
        let r = TritResult::success(ResultData::Integer(42), 10, 1);
        assert!(r.is_success());
        assert!(!r.is_pending());
        assert!(!r.is_failed());
    }

    #[test]
    fn test_client_stats() {
        let c = CrownyClient::new("http://localhost:7293");
        let (total, p, o, t) = c.stats();
        assert_eq!(total, 0);
        assert_eq!(p, 0);
        assert_eq!(o, 0);
        assert_eq!(t, 0);
    }
}
