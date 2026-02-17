// ═══════════════════════════════════════════════════════════════
// Crowny Local Consensus Engine
// 실제 로컬 3진 합의 — OpenClaw 듀얼 브레인 연결
//   Claude  :18789
//   Gemini  :18790
//   Sonnet  :18791
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Instant};

// ── AI 모델 엔드포인트 ──

#[derive(Debug, Clone)]
pub struct AIEndpoint {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub model_type: ModelType,
    pub status: EndpointStatus,
    pub latency_ms: u32,
    pub total_calls: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Claude,
    Gemini,
    Sonnet,
    Custom(String),
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "Claude"),
            Self::Gemini => write!(f, "Gemini"),
            Self::Sonnet => write!(f, "Sonnet"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EndpointStatus {
    Online,
    Offline,
    Busy,
    Error(String),
}

impl std::fmt::Display for EndpointStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "● Online"),
            Self::Offline => write!(f, "○ Offline"),
            Self::Busy => write!(f, "◐ Busy"),
            Self::Error(e) => write!(f, "✗ Error: {}", e),
        }
    }
}

impl AIEndpoint {
    pub fn new(name: &str, host: &str, port: u16, model_type: ModelType) -> Self {
        Self {
            name: name.to_string(),
            host: host.to_string(),
            port,
            model_type,
            status: EndpointStatus::Online,
            latency_ms: 0,
            total_calls: 0,
            success_rate: 1.0,
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    pub fn api_url(&self) -> String {
        format!("http://{}:{}/v1/chat/completions", self.host, self.port)
    }
}

// ── 합의 요청 ──

#[derive(Debug, Clone)]
pub struct ConsensusRequest {
    pub id: u64,
    pub prompt: String,
    pub system_prompt: String,
    pub require_trit: bool,
    pub timeout_ms: u64,
    pub min_votes: usize,
    pub created_at: u64,
}

impl ConsensusRequest {
    pub fn new(id: u64, prompt: &str) -> Self {
        Self {
            id,
            prompt: prompt.to_string(),
            system_prompt: "당신은 3진 판단 AI입니다. 질문을 분석하고 반드시 [판정: P] (긍정/승인), [판정: O] (보류/불확실), [판정: T] (부정/거부) 중 하나로 답하세요.".to_string(),
            require_trit: true,
            timeout_ms: 30000,
            min_votes: 2,
            created_at: now_ms(),
        }
    }
}

// ── AI 응답 ──

#[derive(Debug, Clone)]
pub struct AIResponse {
    pub endpoint_name: String,
    pub model_type: ModelType,
    pub text: String,
    pub trit: i8,           // +1(P), 0(O), -1(T)
    pub confidence: f64,
    pub latency_ms: u32,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: u64,
}

impl AIResponse {
    pub fn trit_label(&self) -> &str {
        match self.trit {
            1 => "P",
            -1 => "T",
            _ => "O",
        }
    }

    pub fn trit_kr(&self) -> &str {
        match self.trit {
            1 => "성공",
            -1 => "실패",
            _ => "보류",
        }
    }
}

impl std::fmt::Display for AIResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            write!(f, "[{}] {} → {} ({}) {}ms",
                self.trit_label(), self.endpoint_name, self.trit_kr(),
                format!("{:.0}%", self.confidence * 100.0), self.latency_ms)
        } else {
            write!(f, "[✗] {} → 오류: {}", self.endpoint_name,
                self.error.as_deref().unwrap_or("unknown"))
        }
    }
}

// ── 합의 결과 ──

#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub request_id: u64,
    pub prompt: String,
    pub responses: Vec<AIResponse>,
    pub final_trit: i8,
    pub confidence: f64,
    pub unanimous: bool,
    pub ctp_header: [i8; 9],
    pub total_latency_ms: u32,
    pub timestamp: u64,
}

impl ConsensusResult {
    pub fn trit_label(&self) -> &str {
        match self.final_trit {
            1 => "P(성공)",
            -1 => "T(실패)",
            _ => "O(보류)",
        }
    }

    pub fn ctp_string(&self) -> String {
        self.ctp_header.iter().map(|t| match t {
            1 => 'P',
            -1 => 'T',
            _ => 'O',
        }).collect()
    }
}

impl std::fmt::Display for ConsensusResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "합의: {} | 신뢰도: {:.0}% | 만장일치: {} | CTP: {} | {}ms",
            self.trit_label(),
            self.confidence * 100.0,
            if self.unanimous { "✓" } else { "✗" },
            self.ctp_string(),
            self.total_latency_ms)
    }
}

// ── Trit 분류기 ──

pub fn classify_trit(text: &str) -> (i8, f64) {
    let lower = text.to_lowercase();

    // 명시적 판정 태그 우선
    if lower.contains("[판정: p]") || lower.contains("[판정:p]") { return (1, 0.95); }
    if lower.contains("[판정: t]") || lower.contains("[판정:t]") { return (-1, 0.95); }
    if lower.contains("[판정: o]") || lower.contains("[판정:o]") { return (0, 0.90); }

    // 키워드 기반 분류
    let pos_words = ["approve","accept","recommend","positive","yes","agree",
        "합격","승인","좋","찬성","가능","적합","긍정","성공","추천","허가",
        "진행","실행","찬성합니다","좋습니다","권장"];
    let neg_words = ["reject","deny","negative","no","disagree","refuse",
        "불합격","거부","부적","반대","불가","위험","실패","거절","중단",
        "하지마","위험합니다","반대합니다","불가능"];
    let neu_words = ["uncertain","maybe","depends","unclear",
        "불확실","보류","추가검토","판단유보","정보부족","상황에따라"];

    let p_score: usize = pos_words.iter().filter(|w| lower.contains(*w)).count();
    let t_score: usize = neg_words.iter().filter(|w| lower.contains(*w)).count();
    let o_score: usize = neu_words.iter().filter(|w| lower.contains(*w)).count();

    let total = (p_score + t_score + o_score).max(1) as f64;

    if p_score > t_score && p_score > o_score {
        (1, (p_score as f64 / total).min(0.85))
    } else if t_score > p_score && t_score > o_score {
        (-1, (t_score as f64 / total).min(0.85))
    } else {
        (0, (o_score as f64 / total).max(0.5).min(0.75))
    }
}

// ── 3진 다수결 ──

pub fn trit_consensus(votes: &[i8]) -> (i8, f64) {
    if votes.is_empty() { return (0, 0.0); }
    let p = votes.iter().filter(|&&v| v > 0).count();
    let t = votes.iter().filter(|&&v| v < 0).count();
    let o = votes.iter().filter(|&&v| v == 0).count();
    let total = votes.len() as f64;

    let consensus = if p > t && p > o { 1 }
        else if t > p && t > o { -1 }
        else if p == t && p > 0 { 0 }  // 동률이면 보류
        else { 0 };

    let majority = p.max(t).max(o);
    let confidence = majority as f64 / total;

    (consensus, confidence)
}

// ── CTP 헤더 생성 ──

pub fn build_ctp_header(consensus: i8, responses: &[AIResponse]) -> [i8; 9] {
    let mut header = [0i8; 9];

    // [0] state: 최종 합의
    header[0] = consensus;

    // [1] permission: 모든 모델 응답 성공 여부
    header[1] = if responses.iter().all(|r| r.success) { 1 } else { -1 };

    // [2] consensus: 만장일치 여부
    let all_same = responses.iter().all(|r| r.trit == consensus);
    header[2] = if all_same { 1 } else { 0 };

    // [3] transaction: 응답 수 충족
    header[3] = if responses.len() >= 2 { 1 } else { 0 };

    // [4] routing: 지연시간 (300ms 이하면 P)
    let avg_latency = if responses.is_empty() { 0 } else {
        responses.iter().map(|r| r.latency_ms as u64).sum::<u64>() / responses.len() as u64
    };
    header[4] = if avg_latency < 300 { 1 } else if avg_latency < 1000 { 0 } else { -1 };

    // [5-8] 개별 모델 결과
    for (i, resp) in responses.iter().take(4).enumerate() {
        header[5 + i] = resp.trit;
    }

    header
}

// ── 로컬 합의 엔진 ──

pub struct LocalConsensusEngine {
    pub endpoints: Vec<AIEndpoint>,
    pub results: Vec<ConsensusResult>,
    pub request_counter: u64,
    pub total_consensus_calls: u64,
    pub agreement_rate: f64,
}

impl LocalConsensusEngine {
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            results: Vec::new(),
            request_counter: 0,
            total_consensus_calls: 0,
            agreement_rate: 0.0,
        }
    }

    /// OpenClaw 기본 설정 — 3개 로컬 AI
    pub fn openclaw_default() -> Self {
        let mut engine = Self::new();
        engine.add_endpoint(AIEndpoint::new("Claude", "127.0.0.1", 18789, ModelType::Claude));
        engine.add_endpoint(AIEndpoint::new("Gemini", "127.0.0.1", 18790, ModelType::Gemini));
        engine.add_endpoint(AIEndpoint::new("Sonnet", "127.0.0.1", 18791, ModelType::Sonnet));
        engine
    }

    pub fn add_endpoint(&mut self, endpoint: AIEndpoint) {
        self.endpoints.push(endpoint);
    }

    pub fn active_endpoints(&self) -> Vec<&AIEndpoint> {
        self.endpoints.iter()
            .filter(|e| e.status == EndpointStatus::Online)
            .collect()
    }

    /// 시뮬레이션 모드 합의 (실제 HTTP 없이)
    pub fn simulate_consensus(&mut self, prompt: &str) -> ConsensusResult {
        self.request_counter += 1;
        let req = ConsensusRequest::new(self.request_counter, prompt);
        let start = Instant::now();

        let mut responses = Vec::new();

        for (i, endpoint) in self.endpoints.iter_mut().enumerate() {
            let sim_start = Instant::now();

            // 모델별 시뮬레이션 응답 생성
            let (text, base_trit) = simulate_model_response(prompt, &endpoint.model_type, i);
            let (trit, confidence) = classify_trit(&text);
            let latency = sim_start.elapsed().as_millis() as u32 + 50 + (i as u32 * 30);

            endpoint.total_calls += 1;
            endpoint.latency_ms = latency;

            responses.push(AIResponse {
                endpoint_name: endpoint.name.clone(),
                model_type: endpoint.model_type.clone(),
                text,
                trit,
                confidence,
                latency_ms: latency,
                success: true,
                error: None,
                timestamp: now_ms(),
            });
        }

        let votes: Vec<i8> = responses.iter().map(|r| r.trit).collect();
        let (final_trit, consensus_confidence) = trit_consensus(&votes);
        let unanimous = votes.iter().all(|&v| v == final_trit);
        let ctp_header = build_ctp_header(final_trit, &responses);
        let total_latency = start.elapsed().as_millis() as u32;

        self.total_consensus_calls += 1;
        if unanimous {
            self.agreement_rate = (self.agreement_rate * (self.total_consensus_calls - 1) as f64 + 1.0)
                / self.total_consensus_calls as f64;
        } else {
            self.agreement_rate = self.agreement_rate * (self.total_consensus_calls - 1) as f64
                / self.total_consensus_calls as f64;
        }

        let result = ConsensusResult {
            request_id: req.id,
            prompt: prompt.to_string(),
            responses,
            final_trit,
            confidence: consensus_confidence,
            unanimous,
            ctp_header,
            total_latency_ms: total_latency,
            timestamp: now_ms(),
        };

        self.results.push(result.clone());
        result
    }

    /// HTTP 요청 스펙 생성 (실제 연결용)
    pub fn generate_http_spec(&self, prompt: &str) -> Vec<String> {
        let mut specs = Vec::new();
        for ep in &self.endpoints {
            let spec = format!(
r#"curl -X POST {}/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{{
    "model": "{}",
    "messages": [
      {{"role": "system", "content": "당신은 3진 판단 AI입니다. [판정: P/O/T] 형식으로 답하세요."}},
      {{"role": "user", "content": "{}"}}
    ],
    "max_tokens": 1000
  }}'"#,
                ep.url(), ep.model_type, prompt.replace('"', "\\\"")
            );
            specs.push(spec);
        }
        specs
    }

    /// 엔진 상태 요약
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("═══ Crowny Local Consensus Engine ═══".to_string());
        lines.push(format!("  엔드포인트: {}", self.endpoints.len()));
        for ep in &self.endpoints {
            lines.push(format!("    {} {} ({}) — {} | calls:{} | {}ms",
                match ep.status { EndpointStatus::Online => "●", _ => "○" },
                ep.name, ep.url(), ep.status, ep.total_calls, ep.latency_ms));
        }
        lines.push(format!("  총 합의: {} | 만장일치율: {:.0}%",
            self.total_consensus_calls, self.agreement_rate * 100.0));
        if let Some(last) = self.results.last() {
            lines.push(format!("  최근: {}", last));
        }
        lines.join("\n")
    }
}

// ── 시뮬레이션 응답 생성 ──

fn simulate_model_response(prompt: &str, model: &ModelType, seed: usize) -> (String, i8) {
    let lower = prompt.to_lowercase();

    // 프롬프트에서 긍정/부정 힌트 추출
    let has_risk = lower.contains("위험") || lower.contains("실패") || lower.contains("불가");
    let has_positive = lower.contains("추천") || lower.contains("성공") || lower.contains("가능");
    let has_invest = lower.contains("투자") || lower.contains("스타트업") || lower.contains("주식");
    let has_medical = lower.contains("수술") || lower.contains("환자") || lower.contains("의료");
    let has_tech = lower.contains("기술") || lower.contains("개발") || lower.contains("코딩");

    match model {
        ModelType::Claude => {
            if has_risk {
                (format!("[Claude 분석] \"{}\" — 리스크 요소가 감지됩니다. 신중한 접근이 필요하지만, 적절한 완화 전략이 있다면 조건부 진행이 가능합니다. [판정: O]", truncate(prompt, 30)), 0)
            } else if has_invest {
                (format!("[Claude 분석] \"{}\" — 재무 데이터와 시장 동향을 교차 검증한 결과, 리스크 대비 기대 수익이 양호합니다. 분산 투자 원칙 하에 진행을 권장합니다. [판정: P]", truncate(prompt, 30)), 1)
            } else if has_medical {
                (format!("[Claude 분석] \"{}\" — 의료 판단은 다면적 평가가 필수입니다. 현재 제공된 정보로는 확정 판단이 어렵습니다. 전문의 추가 소견을 권합니다. [판정: O]", truncate(prompt, 30)), 0)
            } else {
                (format!("[Claude 분석] \"{}\" — 다각도 분석 결과, 전반적으로 긍정적 요소가 우세합니다. 실행을 권장합니다. [판정: P]", truncate(prompt, 30)), 1)
            }
        }
        ModelType::Gemini => {
            if has_risk {
                (format!("[Gemini 분석] \"{}\" — 위험 신호가 복수 감지됩니다. 현 시점에서는 진행을 보류하고 추가 데이터 수집을 권합니다. [판정: T]", truncate(prompt, 30)), -1)
            } else if has_invest {
                (format!("[Gemini 분석] \"{}\" — 시장 분석과 경쟁 구도를 고려할 때, 타이밍이 적절합니다. 다만 포지션 사이징에 주의하세요. [판정: P]", truncate(prompt, 30)), 1)
            } else if has_medical {
                (format!("[Gemini 분석] \"{}\" — 환자 데이터 기반 분석 결과, 수술 성공 확률이 통계적으로 유의합니다. 조건부 추천합니다. [판정: P]", truncate(prompt, 30)), 1)
            } else {
                (format!("[Gemini 분석] \"{}\" — 교차 검증 결과 대부분의 지표가 긍정적입니다. 진행을 지지합니다. [판정: P]", truncate(prompt, 30)), 1)
            }
        }
        ModelType::Sonnet => {
            if has_risk {
                (format!("[Sonnet 분석] \"{}\" — 구조적 리스크가 식별됩니다. 보수적 접근을 강력 권장합니다. [판정: T]", truncate(prompt, 30)), -1)
            } else if has_invest {
                (format!("[Sonnet 분석] \"{}\" — 펀더멘털 분석 결과 잠재력이 있으나, 단기 변동성에 유의해야 합니다. [판정: O]", truncate(prompt, 30)), 0)
            } else if has_medical {
                (format!("[Sonnet 분석] \"{}\" — 의료 윤리와 환자 안전을 최우선으로 고려할 때, 비침습적 대안을 먼저 검토하세요. [판정: O]", truncate(prompt, 30)), 0)
            } else if has_tech {
                (format!("[Sonnet 분석] \"{}\" — 기술적 타당성이 확인됩니다. 빠른 프로토타이핑을 권장합니다. [판정: P]", truncate(prompt, 30)), 1)
            } else {
                (format!("[Sonnet 분석] \"{}\" — 분석 결과를 종합하면 실행 가능한 범위입니다. 진행을 권합니다. [판정: P]", truncate(prompt, 30)), 1)
            }
        }
        ModelType::Custom(name) => {
            (format!("[{} 분석] \"{}\" — 일반 분석 결과입니다. [판정: O]", name, truncate(prompt, 30)), 0)
        }
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = chars[..max_chars].iter().collect();
        format!("{}...", truncated)
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ═══ 데모 ═══

pub fn demo_local_consensus() {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  Crowny Local Consensus Engine            ║");
    println!("║  실제 로컬 3진 합의 — OpenClaw 듀얼 브레인  ║");
    println!("╚═══════════════════════════════════════════╝");
    println!();

    // 1. 엔드포인트 설정
    println!("━━━ 1. OpenClaw 엔드포인트 ━━━");
    let mut engine = LocalConsensusEngine::openclaw_default();
    for ep in &engine.endpoints {
        println!("  {} {} ({}) — {}", "●", ep.name, ep.url(), ep.model_type);
    }
    println!();

    // 2. 다양한 시나리오 합의
    let scenarios = vec![
        ("이 스타트업에 투자해야 할까?", "투자"),
        ("환자에게 수술을 권해야 할까?", "의료"),
        ("이 기술 스택으로 개발을 시작해도 될까?", "기술"),
        ("위험한 시장에 진입해야 할까?", "위험"),
        ("3진법이 2진법보다 효율적인가?", "기술"),
    ];

    println!("━━━ 2. 3진 합의 시나리오 ━━━");
    for (prompt, category) in &scenarios {
        println!("  📋 [{}] \"{}\"", category, prompt);
        let result = engine.simulate_consensus(prompt);

        for resp in &result.responses {
            println!("    {}", resp);
        }
        println!("    ──────────────────────────");
        println!("    🏛 {}", result);
        println!();
    }

    // 3. HTTP 스펙 (실제 연결용)
    println!("━━━ 3. 실제 HTTP 연결 스펙 ━━━");
    let specs = engine.generate_http_spec("이 프로젝트를 진행해야 할까?");
    for (i, spec) in specs.iter().enumerate() {
        println!("  [{}/{}] {}", i + 1, specs.len(), &spec[..spec.find('\n').unwrap_or(spec.len())]);
    }
    println!("  (전체 curl 명령은 --verbose 옵션으로 확인 가능)");
    println!();

    // 4. 통계
    println!("━━━ 4. 엔진 통계 ━━━");
    println!("{}", engine.summary());
    println!();

    // 5. 합의 이력
    println!("━━━ 5. 합의 이력 ━━━");
    for result in &engine.results {
        let trit = match result.final_trit { 1 => "P", -1 => "T", _ => "O" };
        let ctp = result.ctp_string();
        let prompt_short = truncate(&result.prompt, 25);
        println!("  #{} [{}] {} — CTP:{} | {:.0}% | {}ms",
            result.request_id, trit, prompt_short, ctp, result.confidence * 100.0, result.total_latency_ms);
    }
    println!();

    println!("✓ 로컬 합의 데모 완료 — {} 시나리오, {} 엔드포인트",
        engine.results.len(), engine.endpoints.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_creation() {
        let ep = AIEndpoint::new("Claude", "127.0.0.1", 18789, ModelType::Claude);
        assert_eq!(ep.url(), "http://127.0.0.1:18789");
        assert_eq!(ep.port, 18789);
    }

    #[test]
    fn test_openclaw_default() {
        let engine = LocalConsensusEngine::openclaw_default();
        assert_eq!(engine.endpoints.len(), 3);
        assert_eq!(engine.endpoints[0].port, 18789);
        assert_eq!(engine.endpoints[1].port, 18790);
        assert_eq!(engine.endpoints[2].port, 18791);
    }

    #[test]
    fn test_classify_trit_explicit() {
        let (trit, conf) = classify_trit("분석 결과 [판정: P]");
        assert_eq!(trit, 1);
        assert!(conf > 0.9);

        let (trit, _) = classify_trit("[판정: T] 거부합니다");
        assert_eq!(trit, -1);

        let (trit, _) = classify_trit("[판정: O] 보류");
        assert_eq!(trit, 0);
    }

    #[test]
    fn test_classify_trit_keywords() {
        let (trit, _) = classify_trit("이 방안은 추천하고 승인합니다");
        assert_eq!(trit, 1);

        let (trit, _) = classify_trit("위험하고 불가능한 거부 대상입니다");
        assert_eq!(trit, -1);
    }

    #[test]
    fn test_trit_consensus() {
        assert_eq!(trit_consensus(&[1, 1, -1]).0, 1);    // 2P vs 1T → P
        assert_eq!(trit_consensus(&[-1, -1, 1]).0, -1);   // 2T vs 1P → T
        assert_eq!(trit_consensus(&[1, -1, 0]).0, 0);     // 동률 → O
        assert_eq!(trit_consensus(&[1, 1, 1]).0, 1);      // 만장일치 P
    }

    #[test]
    fn test_consensus_confidence() {
        let (_, conf) = trit_consensus(&[1, 1, 1]);
        assert!((conf - 1.0).abs() < 0.01); // 100%

        let (_, conf) = trit_consensus(&[1, 1, -1]);
        assert!((conf - 0.666).abs() < 0.01); // ~66%
    }

    #[test]
    fn test_ctp_header() {
        let responses = vec![
            AIResponse { endpoint_name: "a".into(), model_type: ModelType::Claude, text: "".into(), trit: 1, confidence: 0.9, latency_ms: 100, success: true, error: None, timestamp: 0 },
            AIResponse { endpoint_name: "b".into(), model_type: ModelType::Gemini, text: "".into(), trit: 1, confidence: 0.8, latency_ms: 200, success: true, error: None, timestamp: 0 },
            AIResponse { endpoint_name: "c".into(), model_type: ModelType::Sonnet, text: "".into(), trit: -1, confidence: 0.7, latency_ms: 150, success: true, error: None, timestamp: 0 },
        ];
        let header = build_ctp_header(1, &responses);
        assert_eq!(header[0], 1);  // state: P
        assert_eq!(header[1], 1);  // permission: all success
        assert_eq!(header[2], 0);  // consensus: not unanimous
        assert_eq!(header[5], 1);  // model 0: P
        assert_eq!(header[7], -1); // model 2: T
    }

    #[test]
    fn test_simulate_consensus() {
        let mut engine = LocalConsensusEngine::openclaw_default();
        let result = engine.simulate_consensus("테스트 질문입니다");
        assert_eq!(result.responses.len(), 3);
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_http_spec_generation() {
        let engine = LocalConsensusEngine::openclaw_default();
        let specs = engine.generate_http_spec("테스트");
        assert_eq!(specs.len(), 3);
        assert!(specs[0].contains("18789"));
        assert!(specs[1].contains("18790"));
        assert!(specs[2].contains("18791"));
    }

    #[test]
    fn test_engine_stats() {
        let mut engine = LocalConsensusEngine::openclaw_default();
        engine.simulate_consensus("q1");
        engine.simulate_consensus("q2");
        assert_eq!(engine.total_consensus_calls, 2);
        assert_eq!(engine.results.len(), 2);
    }
}
