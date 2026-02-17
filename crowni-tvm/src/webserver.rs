///! ═══════════════════════════════════════════════════
///! Crowny 웹서버 + LLM 호출기 v0.1
///! ═══════════════════════════════════════════════════
///!
///! GPT Spec:
///!   "Go 웹서버는 최적 선택" → Rust 내장 버전 (경량)
///!   "LLM 호출기는 하이브리드" → API + 로컬 라우팅
///!
///! 웹서버:
///!   HTTP 요청 → CTP Header 파싱 → CAR.submit() → TritResult 반환
///!
///! LLM 호출기:
///!   프롬프트 → 모델 선택 → API 호출 → Trit 판정 → TritResult 반환
///!
///! 모든 실행은 CAR 경유. 직접 Meta-Kernel 호출 금지.

use std::collections::HashMap;
use crate::car::{TritState, TritResult, ResultData, AppTask, TaskType, CrownyRuntime};

// ═══════════════════════════════════════════════
// CTP (Crowny Trit Protocol) 요청/응답
// ═══════════════════════════════════════════════

/// CTP 9-Trit 헤더
#[derive(Debug, Clone)]
pub struct CtpHeader {
    pub state: i8,       // [0] 상태
    pub permission: i8,  // [1] 권한
    pub consensus: i8,   // [2] 합의
    pub transaction: i8, // [3] 트랜잭션
    pub routing: i8,     // [4] 라우팅
    pub reserved: [i8; 4], // [5..8]
}

impl CtpHeader {
    pub fn new() -> Self {
        Self {
            state: 0, permission: 0, consensus: 0,
            transaction: 0, routing: 0, reserved: [0; 4],
        }
    }

    pub fn success() -> Self {
        Self { state: 1, permission: 1, consensus: 1, transaction: 1, routing: 1, reserved: [0; 4] }
    }

    pub fn failed() -> Self {
        Self { state: -1, permission: 0, consensus: 0, transaction: -1, routing: 0, reserved: [0; 4] }
    }

    /// X-Crowny-Trit 헤더 문자열 파싱
    pub fn from_header_str(s: &str) -> Self {
        let trits: Vec<i8> = s.chars()
            .filter_map(|c| match c {
                'P' | '+' | '1' => Some(1),
                'O' | '0' => Some(0),
                'T' | '-' => Some(-1),
                _ => None,
            })
            .collect();

        let get = |i: usize| trits.get(i).copied().unwrap_or(0);

        Self {
            state: get(0), permission: get(1), consensus: get(2),
            transaction: get(3), routing: get(4),
            reserved: [get(5), get(6), get(7), get(8)],
        }
    }

    /// 9-Trit 문자열
    pub fn to_header_str(&self) -> String {
        let t = |v: i8| match v { 1 => 'P', -1 => 'T', _ => 'O' };
        format!("{}{}{}{}{}{}{}{}{}",
            t(self.state), t(self.permission), t(self.consensus),
            t(self.transaction), t(self.routing),
            t(self.reserved[0]), t(self.reserved[1]),
            t(self.reserved[2]), t(self.reserved[3]))
    }

    /// Trit 상태
    pub fn overall_state(&self) -> TritState {
        // 하나라도 -1이면 실패 (하향 안정성 원칙)
        if self.state == -1 || self.permission == -1 { return TritState::Failed; }
        if self.state == 1 && self.permission >= 0 { return TritState::Success; }
        TritState::Pending
    }
}

impl std::fmt::Display for CtpHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[CTP:{}]", self.to_header_str())
    }
}

// ═══════════════════════════════════════════════
// HTTP 요청/응답 (경량 구조체)
// ═══════════════════════════════════════════════

/// HTTP 메서드
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
        }
    }
}

/// HTTP 요청
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub ctp: CtpHeader,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, path: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            headers: HashMap::new(),
            body: String::new(),
            ctp: CtpHeader::new(),
        }
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn with_ctp(mut self, ctp: CtpHeader) -> Self {
        self.ctp = ctp;
        self
    }

    pub fn with_header(mut self, key: &str, val: &str) -> Self {
        self.headers.insert(key.to_string(), val.to_string());
        self
    }
}

/// HTTP 응답
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub ctp: CtpHeader,
    pub trit_result: TritResult,
}

// ═══════════════════════════════════════════════
// 라우터
// ═══════════════════════════════════════════════

/// 라우트 핸들러 타입
type HandlerFn = Box<dyn Fn(&HttpRequest, &mut CrownyRuntime) -> HttpResponse>;

/// 라우트
struct Route {
    method: HttpMethod,
    path: String,
    handler: HandlerFn,
}

/// Crowny 웹서버 (경량)
pub struct CrownyServer {
    routes: Vec<Route>,
    port: u16,
    request_count: u64,
}

impl CrownyServer {
    pub fn new(port: u16) -> Self {
        println!("[서버] Crowny Web Server 초기화 — 포트 {}", port);
        Self { routes: Vec::new(), port, request_count: 0 }
    }

    /// 라우트 등록
    pub fn route(
        &mut self,
        method: HttpMethod,
        path: &str,
        handler: impl Fn(&HttpRequest, &mut CrownyRuntime) -> HttpResponse + 'static,
    ) {
        self.routes.push(Route {
            method,
            path: path.to_string(),
            handler: Box::new(handler),
        });
    }

    /// 요청 처리 (시뮬레이션)
    pub fn handle(&mut self, req: &HttpRequest, car: &mut CrownyRuntime) -> HttpResponse {
        self.request_count += 1;

        // CTP 헤더 검증
        let ctp_state = req.ctp.overall_state();
        if ctp_state == TritState::Failed {
            return HttpResponse {
                status: 403,
                headers: HashMap::new(),
                body: "{\"상태\":\"T\",\"오류\":\"CTP 권한 거부\"}".into(),
                ctp: CtpHeader::failed(),
                trit_result: TritResult {
                    state: TritState::Failed,
                    data: ResultData::Text("CTP 권한 거부".into()),
                    elapsed_ms: 0,
                    task_id: 0,
                },
            };
        }

        // 라우트 매칭
        for route in &self.routes {
            if route.method == req.method && route.path == req.path {
                return (route.handler)(req, car);
            }
        }

        // 404
        HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: "{\"상태\":\"T\",\"오류\":\"경로 없음\"}".into(),
            ctp: CtpHeader::failed(),
            trit_result: TritResult {
                state: TritState::Failed,
                data: ResultData::Text("404".into()),
                elapsed_ms: 0,
                task_id: 0,
            },
        }
    }

    pub fn stats(&self) -> String {
        format!("[서버] 포트:{} 라우트:{} 요청:{}", self.port, self.routes.len(), self.request_count)
    }
}

// ═══════════════════════════════════════════════
// LLM 호출기
// ═══════════════════════════════════════════════

/// LLM 모델 종류
#[derive(Debug, Clone, PartialEq)]
pub enum LlmModel {
    Claude,
    Gpt4,
    Gemini,
    Local,    // 로컬 모델
    Custom(String),
}

impl std::fmt::Display for LlmModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmModel::Claude => write!(f, "Claude"),
            LlmModel::Gpt4 => write!(f, "GPT-4"),
            LlmModel::Gemini => write!(f, "Gemini"),
            LlmModel::Local => write!(f, "Local"),
            LlmModel::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// LLM 요청
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: LlmModel,
    pub prompt: String,
    pub system: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub params: HashMap<String, String>,
}

impl LlmRequest {
    pub fn new(model: LlmModel, prompt: &str) -> Self {
        Self {
            model,
            prompt: prompt.to_string(),
            system: None,
            temperature: 0.7,
            max_tokens: 1024,
            params: HashMap::new(),
        }
    }

    pub fn with_system(mut self, sys: &str) -> Self {
        self.system = Some(sys.to_string());
        self
    }

    pub fn with_temp(mut self, t: f32) -> Self {
        self.temperature = t;
        self
    }
}

/// LLM 응답
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub model: LlmModel,
    pub tokens_used: u32,
    pub trit_state: TritState,
}

/// Crowny LLM 호출기 — 다중 모델 라우터
pub struct CrownyLlm {
    default_model: LlmModel,
    call_count: u64,
    total_tokens: u64,
    // 모델별 API 키 (시뮬레이션)
    api_keys: HashMap<String, String>,
}

impl CrownyLlm {
    pub fn new() -> Self {
        println!("[LLM] Crowny LLM 호출기 초기화");
        Self {
            default_model: LlmModel::Claude,
            call_count: 0,
            total_tokens: 0,
            api_keys: HashMap::new(),
        }
    }

    pub fn set_api_key(&mut self, model: &str, key: &str) {
        self.api_keys.insert(model.to_string(), key.to_string());
    }

    pub fn set_default_model(&mut self, model: LlmModel) {
        self.default_model = model;
    }

    /// LLM 호출 (CAR 경유)
    pub fn call(&mut self, req: LlmRequest, car: &mut CrownyRuntime) -> TritResult {
        let model_name = req.model.to_string();
        let prompt = req.prompt.clone();

        let task = AppTask::new(TaskType::LlmCall, &model_name, &prompt)
            .with_param("temperature", &req.temperature.to_string())
            .with_param("max_tokens", &req.max_tokens.to_string());

        // 실제 API 호출 시뮬레이션
        let call_count = &mut self.call_count;
        let total_tokens = &mut self.total_tokens;

        car.submit(task, |t| {
            // ── 실제 프로덕션에서는 여기서 HTTP API 호출 ──
            // 지금은 시뮬레이션
            let response = simulate_llm_response(&t.payload, &model_name);

            *call_count += 1;
            *total_tokens += response.tokens_used as u64;

            // Trit 판정
            let state = response.trit_state;
            (state, ResultData::Text(response.text))
        })
    }

    /// 간편 호출
    pub fn ask(&mut self, prompt: &str, car: &mut CrownyRuntime) -> TritResult {
        let req = LlmRequest::new(self.default_model.clone(), prompt);
        self.call(req, car)
    }

    /// 다중 모델 동시 호출 (합의)
    pub fn consensus_call(
        &mut self,
        prompt: &str,
        models: &[LlmModel],
        car: &mut CrownyRuntime,
    ) -> TritResult {
        let mut results: Vec<TritState> = Vec::new();
        let mut texts: Vec<String> = Vec::new();

        for model in models {
            let req = LlmRequest::new(model.clone(), prompt);
            let result = self.call(req, car);
            results.push(result.state);
            if let ResultData::Text(t) = &result.data {
                texts.push(format!("[{}] {}", model, t));
            }
        }

        // 3진 다수결
        let pos = results.iter().filter(|s| **s == TritState::Success).count();
        let neg = results.iter().filter(|s| **s == TritState::Failed).count();
        let final_state = if pos > neg { TritState::Success }
            else if neg > pos { TritState::Failed }
            else { TritState::Pending };

        TritResult {
            state: final_state,
            data: ResultData::Text(texts.join("\n")),
            elapsed_ms: 0,
            task_id: 0,
        }
    }

    pub fn stats(&self) -> String {
        format!("[LLM] 호출:{} 토큰:{} 기본모델:{}", self.call_count, self.total_tokens, self.default_model)
    }
}

/// LLM 응답 시뮬레이션
fn simulate_llm_response(prompt: &str, model: &str) -> LlmResponse {
    let text = format!("[{} 응답] 입력 '{}' 에 대한 균형3진 기반 분석 결과입니다.", model, prompt);
    let tokens = (prompt.len() as u32 / 2) + 50; // 대략적 토큰 수

    LlmResponse {
        text,
        model: LlmModel::Custom(model.to_string()),
        tokens_used: tokens,
        trit_state: TritState::Success,
    }
}

// ═══════════════════════════════════════════════
// 기본 라우트 생성 헬퍼
// ═══════════════════════════════════════════════

/// 기본 Crowny 서버 생성 (데모용 라우트 포함)
pub fn create_demo_server() -> CrownyServer {
    let mut server = CrownyServer::new(7293);

    // GET /
    server.route(HttpMethod::Get, "/", |_req, car| {
        let result = car.run_source("system", "넣어 1\n종료");
        HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: format!("{{\"상태\":\"{}\",\"메시지\":\"Crowny 서버 작동중\"}}", result.state),
            ctp: CtpHeader::success(),
            trit_result: result,
        }
    });

    // POST /run — 한선어 실행
    server.route(HttpMethod::Post, "/run", |req, car| {
        let result = car.run_source("web", &req.body);
        let status = match result.state {
            TritState::Success => 200,
            TritState::Pending => 202,
            TritState::Failed => 500,
        };
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: format!("{{\"상태\":\"{}\",\"결과\":\"{}\"}}", result.state, result.data),
            ctp: if result.state == TritState::Success { CtpHeader::success() } else { CtpHeader::failed() },
            trit_result: result,
        }
    });

    // POST /compile — WASM 컴파일
    server.route(HttpMethod::Post, "/compile", |req, car| {
        let result = car.compile_wasm("web", &req.body);
        let status = if result.state == TritState::Success { 200 } else { 500 };
        let body_text = match &result.data {
            ResultData::Bytes(b) => format!("{{\"상태\":\"{}\",\"크기\":{}}}", result.state, b.len()),
            _ => format!("{{\"상태\":\"{}\"}}", result.state),
        };
        HttpResponse {
            status,
            headers: HashMap::new(),
            body: body_text,
            ctp: if result.state == TritState::Success { CtpHeader::success() } else { CtpHeader::failed() },
            trit_result: result,
        }
    });

    server
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ctp_header() {
        let h = CtpHeader::from_header_str("PPPOOOOOT");
        assert_eq!(h.state, 1);
        assert_eq!(h.permission, 1);
        assert_eq!(h.consensus, 1);
        assert_eq!(h.to_header_str(), "PPPOOOOOT");
    }

    #[test]
    fn test_server_handle() {
        let mut server = create_demo_server();
        let mut car = CrownyRuntime::new();

        // GET /
        let req = HttpRequest::new(HttpMethod::Get, "/").with_ctp(CtpHeader::success());
        let resp = server.handle(&req, &mut car);
        assert_eq!(resp.status, 200);

        // POST /run
        let req = HttpRequest::new(HttpMethod::Post, "/run")
            .with_body("넣어 10\n넣어 20\n더해\n종료")
            .with_ctp(CtpHeader::success());
        let resp = server.handle(&req, &mut car);
        assert_eq!(resp.status, 200);
        assert_eq!(resp.trit_result.state, TritState::Success);
    }

    #[test]
    fn test_ctp_denied() {
        let mut server = create_demo_server();
        let mut car = CrownyRuntime::new();

        let req = HttpRequest::new(HttpMethod::Get, "/").with_ctp(CtpHeader::failed());
        let resp = server.handle(&req, &mut car);
        assert_eq!(resp.status, 403);
    }

    #[test]
    fn test_llm_call() {
        let mut car = CrownyRuntime::new();
        let mut llm = CrownyLlm::new();

        let result = llm.ask("테스트 질문", &mut car);
        assert_eq!(result.state, TritState::Success);
    }

    #[test]
    fn test_llm_consensus() {
        let mut car = CrownyRuntime::new();
        let mut llm = CrownyLlm::new();

        let result = llm.consensus_call(
            "균형3진 효율?",
            &[LlmModel::Claude, LlmModel::Gpt4, LlmModel::Gemini],
            &mut car,
        );
        assert_eq!(result.state, TritState::Success);
    }

    #[test]
    fn test_404() {
        let mut server = create_demo_server();
        let mut car = CrownyRuntime::new();

        let req = HttpRequest::new(HttpMethod::Get, "/없는경로").with_ctp(CtpHeader::success());
        let resp = server.handle(&req, &mut car);
        assert_eq!(resp.status, 404);
    }
}
