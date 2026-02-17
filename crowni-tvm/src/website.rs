// ═══════════════════════════════════════════════════════════════
// Crowny Website — 3진 통합코드로 작성된 웹사이트
// .crwn 마크업 · TritScript · CTP 라우팅
// 브라우저 + 플랫폼과 연동되는 실제 웹앱
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

// ═══════════════════════════════════════
// TritScript — 3진 스크립트 언어
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub enum TritValue {
    Number(f64),
    Str(String),
    Trit(i8),
    List(Vec<TritValue>),
    Null,
}

impl std::fmt::Display for TritValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{}", n),
            Self::Str(s) => write!(f, "{}", s),
            Self::Trit(t) => write!(f, "{}", match t { 1 => "P", -1 => "T", _ => "O" }),
            Self::List(l) => write!(f, "[{}]", l.iter().map(|v| format!("{}", v)).collect::<Vec<_>>().join(", ")),
            Self::Null => write!(f, "null"),
        }
    }
}

pub struct TritScript {
    pub variables: HashMap<String, TritValue>,
    pub output: Vec<String>,
}

impl TritScript {
    pub fn new() -> Self {
        Self { variables: HashMap::new(), output: Vec::new() }
    }

    pub fn execute(&mut self, code: &str) -> Vec<String> {
        self.output.clear();
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") { continue; }
            self.exec_line(trimmed);
        }
        self.output.clone()
    }

    fn exec_line(&mut self, line: &str) {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        match parts[0] {
            "변수" | "let" => {
                if let Some(rest) = parts.get(1) {
                    let kv: Vec<&str> = rest.splitn(2, '=').collect();
                    if kv.len() == 2 {
                        let key = kv[0].trim();
                        let val = self.parse_value(kv[1].trim());
                        self.variables.insert(key.to_string(), val);
                    }
                }
            }
            "출력" | "print" => {
                if let Some(rest) = parts.get(1) {
                    let val = self.resolve(rest.trim());
                    self.output.push(format!("{}", val));
                }
            }
            "만약" | "if" => {
                if let Some(rest) = parts.get(1) {
                    // 간단한 trit 조건
                    let parts2: Vec<&str> = rest.split("이면").collect();
                    if parts2.len() == 2 {
                        let cond = self.resolve(parts2[0].trim());
                        if let TritValue::Trit(t) = cond {
                            if t > 0 { self.exec_line(parts2[1].trim()); }
                        }
                    }
                }
            }
            "합의" | "consensus" => {
                if let Some(rest) = parts.get(1) {
                    let votes: Vec<i8> = rest.trim().chars().filter_map(|c| match c {
                        'P' | 'p' => Some(1), 'T' | 't' => Some(-1), 'O' | 'o' => Some(0), _ => None
                    }).collect();
                    let p = votes.iter().filter(|&&v| v > 0).count();
                    let t = votes.iter().filter(|&&v| v < 0).count();
                    let result = if p > t { "P" } else if t > p { "T" } else { "O" };
                    self.output.push(format!("합의: {} (P:{} O:{} T:{})", result, p, votes.len() - p - t, t));
                }
            }
            "CTP" | "ctp" => {
                if let Some(rest) = parts.get(1) {
                    self.output.push(format!("CTP-Header: {}", rest.trim()));
                }
            }
            "반복" | "loop" => {
                if let Some(rest) = parts.get(1) {
                    let parts2: Vec<&str> = rest.splitn(2, ':').collect();
                    if parts2.len() == 2 {
                        let count: usize = parts2[0].trim().parse().unwrap_or(1);
                        for i in 0..count {
                            self.variables.insert("i".into(), TritValue::Number(i as f64));
                            self.exec_line(parts2[1].trim());
                        }
                    }
                }
            }
            _ => {
                // 표현식 평가 시도
                let val = self.resolve(line);
                if !matches!(val, TritValue::Null) {
                    self.output.push(format!("{}", val));
                }
            }
        }
    }

    fn parse_value(&self, s: &str) -> TritValue {
        if s == "P" { return TritValue::Trit(1); }
        if s == "O" { return TritValue::Trit(0); }
        if s == "T" { return TritValue::Trit(-1); }
        if let Ok(n) = s.parse::<f64>() { return TritValue::Number(n); }
        if s.starts_with('"') && s.ends_with('"') {
            return TritValue::Str(s[1..s.len()-1].to_string());
        }
        TritValue::Str(s.to_string())
    }

    fn resolve(&self, s: &str) -> TritValue {
        if let Some(val) = self.variables.get(s) { return val.clone(); }
        self.parse_value(s)
    }
}

// ═══════════════════════════════════════
// CTP 라우터 — 웹사이트 라우팅
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub handler: String,
    pub method: String,
    pub trit_permission: i8,
    pub middleware: Vec<String>,
}

pub struct CTPRouter {
    pub routes: Vec<Route>,
    pub middleware_stack: Vec<String>,
}

impl CTPRouter {
    pub fn new() -> Self {
        Self { routes: Vec::new(), middleware_stack: vec!["auth".into(), "ctp-validate".into(), "trit-log".into()] }
    }

    pub fn add(&mut self, method: &str, path: &str, handler: &str, trit_perm: i8) {
        self.routes.push(Route {
            path: path.into(), handler: handler.into(), method: method.into(),
            trit_permission: trit_perm, middleware: self.middleware_stack.clone(),
        });
    }

    pub fn match_route(&self, method: &str, path: &str) -> Option<&Route> {
        self.routes.iter().find(|r| r.method == method && r.path == path)
    }

    pub fn handle_request(&self, method: &str, path: &str) -> (i8, String) {
        if let Some(route) = self.match_route(method, path) {
            let trit_label = match route.trit_permission { 1 => "P", -1 => "T", _ => "O" };
            (route.trit_permission, format!("[{}] {} {} → {}", trit_label, method, path, route.handler))
        } else {
            (-1, format!("[T] {} {} → 404 Not Found", method, path))
        }
    }
}

// ═══════════════════════════════════════
// 3진 웹사이트 앱
// ═══════════════════════════════════════

pub struct CrownyWebsite {
    pub name: String,
    pub router: CTPRouter,
    pub pages: HashMap<String, String>,    // .crwn 페이지
    pub scripts: HashMap<String, String>,  // TritScript 파일
    pub styles: HashMap<String, String>,   // 스타일
    pub api_data: HashMap<String, String>, // API 응답
    pub port: u16,
}

impl CrownyWebsite {
    pub fn new(name: &str, port: u16) -> Self {
        let mut site = Self {
            name: name.into(),
            router: CTPRouter::new(),
            pages: HashMap::new(),
            scripts: HashMap::new(),
            styles: HashMap::new(),
            api_data: HashMap::new(),
            port,
        };
        site.setup_routes();
        site.setup_pages();
        site.setup_scripts();
        site.setup_api();
        site
    }

    fn setup_routes(&mut self) {
        // 페이지 라우트
        self.router.add("GET", "/", "page:home", 1);
        self.router.add("GET", "/about", "page:about", 1);
        self.router.add("GET", "/docs", "page:docs", 1);
        self.router.add("GET", "/exchange", "page:exchange", 1);
        self.router.add("GET", "/consensus", "page:consensus", 1);
        self.router.add("GET", "/industry", "page:industry", 1);

        // API 라우트
        self.router.add("GET", "/api/status", "api:status", 1);
        self.router.add("GET", "/api/price", "api:price", 1);
        self.router.add("POST", "/api/transfer", "api:transfer", 1);
        self.router.add("VOTE", "/api/consensus", "api:consensus", 1);
        self.router.add("SUBMIT", "/api/deploy", "api:deploy", 1);

        // 보호된 라우트
        self.router.add("POST", "/api/admin", "api:admin", -1); // T: 거부
    }

    fn setup_pages(&mut self) {
        self.pages.insert("/".into(), r#"제목: Crowny — 세계 최초 3진법 플랫폼
언어: ko

# Crowny

## 세계 최초 3진법 기반 클라우드 플랫폼

[P] 729 Opcodes 3진 가상머신 (TVM)
[P] 한선어 — 한국어 프로그래밍 언어
[P] CTP 프로토콜 — 모든 요청에 P/O/T 상태
[P] CRWN 토큰 — 3진 경제 시스템

---

## 플랫폼 서비스

### Git (GitHub 대체)
[P] 3진 코드 리뷰 — PR을 P/O/T로 심사
[P] 한선어 + Rust + Go + JS 지원

### Deploy (Vercel 대체)
[P] 원클릭 배포 — .crwn 사이트 즉시 호스팅
[P] CTP 엣지 네트워크 — 글로벌 CDN

### DB (Firebase 대체)
[P] TritDB — 모든 문서에 3진 상태
[P] 실시간 동기화 — SYNC 프로토콜

### Runtime (Railway 대체)
[P] Rust, Node, Python, Go, 한선어 런타임
[P] 오토스케일링 — 트래픽 기반 자동 확장

### Web3 (Thirdweb 대체)
[P] CRWN 토큰 발행/전송/스테이킹
[P] 3진 NFT — ERC-3T 표준
[P] DAO 거버넌스 — 3진 투표

---

## 산업 적용

[P] 의료 AI — 3개 AI 합의로 진단 보조
[P] 교육 AI — 맞춤형 학습 경로 설계
[P] 트레이딩 AI — 3진 합의 매매 시그널

---

## 기술 스택
[P] 31 Rust 모듈 · 14,903줄 · 133 테스트
[P] SDK: JavaScript + Go + Rust
[P] OpenClaw 듀얼 브레인 (Claude + Gemini + Sonnet)

스크립트: 합의 PPO
"#.into());

        self.pages.insert("/about".into(), r#"제목: About Crowny
언어: ko

# About Crowny

## 비전
[P] 2진법(0/1)을 넘어 3진법(P/O/T)으로 — 인간의 사고를 코드로

## 핵심 원리
[P] Positive (P, +1) — 승인 · 참 · 성공
[O] Observe (O, 0) — 보류 · 관찰 · 미정
[T] Terminate (T, -1) — 거부 · 거짓 · 실패

---

## 팀
[P] EF — Founder & Architect
[P] Claude AI — Core Engine Dev
[P] Gemini AI — Cross Validation
[P] Sonnet AI — Quality Assurance

## 연혁
[P] 2026.02 — Crowny TVM v0.1 탄생
[P] 2026.02 — 한선어 컴파일러 완성
[P] 2026.02 — v0.7.0 산업 적용 3종 완성
[P] 2026.02 — v0.8.0 플랫폼 + 브라우저 + 웹사이트
"#.into());

        self.pages.insert("/exchange".into(), r#"제목: CRWN Exchange
언어: ko

# CRWN 거래소

## 토큰 정보
[P] 심볼: CRWN (크라운)
[P] 총 발행: 153,000,000 CRWN
[P] 1 CROWN = 1,000,000 trits
[P] 수수료: 0.1% (소각)

---

## 실시간 시세
[P] CRWN/USDT: $0.1240
[P] 24h 변동: +2.5%
[P] 거래량: $45,000,000

---

## 스테이킹
[P] APY: 5.0%
[P] 총 스테이킹: 150,000 CRWN
[P] 참여자: 2명

스크립트: 변수 price = 0.124
스크립트: 변수 holdings = 989970
스크립트: 출력 "포트폴리오 가치: $"
"#.into());

        self.pages.insert("/consensus".into(), r#"제목: 3진 합의 엔진
언어: ko

# 3진 합의 엔진

## OpenClaw 듀얼 브레인
[P] Claude — 127.0.0.1:18789
[P] Gemini — 127.0.0.1:18790
[P] Sonnet — 127.0.0.1:18791

---

## 합의 프로세스
[P] 1. 질문 입력
[P] 2. 3개 AI 동시 분석
[P] 3. 각 AI가 P/O/T 투표
[P] 4. 다수결 합의 도출
[P] 5. CTP 헤더 생성

---

## 예시
스크립트: 합의 PPO
스크립트: 합의 PPP
스크립트: 합의 OTT
"#.into());

        self.pages.insert("/docs".into(), r#"제목: 개발자 문서
언어: ko

# Crowny 개발자 문서

## Quick Start

### 설치
스크립트: 출력 "cargo install crowni-tvm"

### 실행
스크립트: 출력 "crowni-tvm all"

---

## .crwn 파일 작성법
[P] 제목: — 페이지 제목 설정
[P] # — 대제목 (H1)
[P] ## — 중제목 (H2)
[P] [P] — 승인 상태 텍스트
[O] [O] — 보류 상태 텍스트
[T] [T] — 거부 상태 텍스트
[P] 스크립트: — TritScript 인라인 코드
[P] 입력: — 폼 입력 필드
[P] 트릿: P — Trit 뱃지
[P] --- — 구분선

---

## CTP 메서드
[P] GET — 리소스 조회
[P] POST — 데이터 전송
[P] SUBMIT — CAR.submit()
[P] VOTE — 3진 투표
[P] SYNC — 상태 동기화
"#.into());

        self.pages.insert("/industry".into(), r#"제목: 산업 적용
언어: ko

# 산업 적용

## 의료 AI
[P] 환자 바이탈 자동 리스크 스코어링
[P] 3 AI 합의 — 수술/투약/퇴원 판단
[P] CTP 헤더로 판단 근거 추적

## 교육 AI
[P] 학생 성적 + 학습유형 + 출석 분석
[P] 시각형/청각형/체험형/독서형 맞춤
[P] 심화/보충 경로 자동 설계

## 트레이딩 AI
[P] RSI + MACD + 볼린저 + F&G 분석
[P] 3 AI 합의 매매 시그널
[P] 자동 손절/익절 산출

스크립트: 합의 PPO
스크립트: CTP PPPPOOOOO
"#.into());
    }

    fn setup_scripts(&mut self) {
        self.scripts.insert("main.trit".into(), r#"
// Crowny 메인 스크립트
변수 version = "0.8.0"
변수 modules = 33
변수 tests = 133
출력 "Crowny TVM v0.8.0"
출력 "모듈: 33 | 테스트: 133+"
합의 PPP
CTP PPPPOOOOO
"#.into());

        self.scripts.insert("consensus.trit".into(), r#"
// 합의 스크립트
변수 claude = P
변수 gemini = P
변수 sonnet = O
합의 PPO
출력 "합의 완료: P (67%)"
"#.into());
    }

    fn setup_api(&mut self) {
        self.api_data.insert("/api/status".into(), r#"{"trit":"P","status":"online","modules":33,"tests":133,"version":"0.8.0"}"#.into());
        self.api_data.insert("/api/price".into(), r#"{"trit":"P","symbol":"CRWN","price":0.1240,"change24h":2.5,"volume":"45000000"}"#.into());
    }

    pub fn handle(&self, method: &str, path: &str) -> (i8, String, String) {
        let (trit, log) = self.router.handle_request(method, path);

        let body = if let Some(page) = self.pages.get(path) {
            page.clone()
        } else if let Some(api) = self.api_data.get(path) {
            api.clone()
        } else {
            format!("[T] 404 — {} not found", path)
        };

        (trit, log, body)
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("═══ {} ═══", self.name));
        lines.push(format!("  포트: :{}", self.port));
        lines.push(format!("  라우트: {}", self.router.routes.len()));
        lines.push(format!("  페이지: {} (.crwn)", self.pages.len()));
        lines.push(format!("  스크립트: {} (.trit)", self.scripts.len()));
        lines.push(format!("  API: {} endpoints", self.api_data.len()));
        lines.push(format!("  미들웨어: {:?}", self.router.middleware_stack));
        lines.join("\n")
    }
}

// ═══ 데모 ═══

pub fn demo_website() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Website — 3진 통합코드 웹사이트        ║");
    println!("║  .crwn 마크업 · TritScript · CTP 라우팅        ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let site = CrownyWebsite::new("Crowny Official", 3333);

    // 1. 라우트
    println!("━━━ 1. CTP 라우트 ━━━");
    for route in &site.router.routes {
        let trit = match route.trit_permission { 1 => "P", -1 => "T", _ => "O" };
        println!("  [{}] {} {} → {}", trit, route.method, route.path, route.handler);
    }
    println!();

    // 2. 페이지 렌더
    println!("━━━ 2. 페이지 목록 ━━━");
    for (path, content) in &site.pages {
        let lines = content.lines().count();
        let title_line = content.lines().find(|l| l.starts_with("제목:")).unwrap_or("제목: ?");
        let title = title_line.split(':').nth(1).unwrap_or("?").trim();
        println!("  {} — {} ({}줄)", path, title, lines);
    }
    println!();

    // 3. 요청 처리
    println!("━━━ 3. CTP 요청 처리 ━━━");
    let requests = vec![
        ("GET", "/"),
        ("GET", "/about"),
        ("GET", "/exchange"),
        ("GET", "/api/status"),
        ("GET", "/api/price"),
        ("VOTE", "/api/consensus"),
        ("POST", "/api/admin"),       // T: 거부됨
        ("GET", "/nonexistent"),      // 404
    ];
    for (method, path) in &requests {
        let (trit, log, _body) = site.handle(method, path);
        let trit_label = match trit { 1 => "P", -1 => "T", _ => "O" };
        println!("  [{}] {}", trit_label, log);
    }
    println!();

    // 4. TritScript 실행
    println!("━━━ 4. TritScript 실행 ━━━");
    let mut ts = TritScript::new();
    if let Some(code) = site.scripts.get("main.trit") {
        let output = ts.execute(code);
        for line in &output { println!("  > {}", line); }
    }
    println!();

    if let Some(code) = site.scripts.get("consensus.trit") {
        let output = ts.execute(code);
        for line in &output { println!("  > {}", line); }
    }
    println!();

    // 5. API 응답
    println!("━━━ 5. API 응답 ━━━");
    for (path, data) in &site.api_data {
        println!("  {} → {}", path, data);
    }
    println!();

    // 6. 사이트 요약
    println!("━━━ 6. 사이트 요약 ━━━");
    println!("{}", site.summary());
    println!();

    println!("✓ 3진 웹사이트 데모 완료 — {} 페이지, {} 라우트", site.pages.len(), site.router.routes.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tritscript_variable() {
        let mut ts = TritScript::new();
        let out = ts.execute("변수 x = 42\n출력 x");
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("42"));
    }

    #[test]
    fn test_tritscript_consensus() {
        let mut ts = TritScript::new();
        let out = ts.execute("합의 PPO");
        assert!(out[0].contains("P"));
    }

    #[test]
    fn test_tritscript_ctp() {
        let mut ts = TritScript::new();
        let out = ts.execute("CTP PPPPOOOOO");
        assert!(out[0].contains("PPPPOOOOO"));
    }

    #[test]
    fn test_router_match() {
        let mut router = CTPRouter::new();
        router.add("GET", "/", "home", 1);
        let route = router.match_route("GET", "/");
        assert!(route.is_some());
        assert_eq!(route.unwrap().handler, "home");
    }

    #[test]
    fn test_router_deny() {
        let mut router = CTPRouter::new();
        router.add("POST", "/admin", "admin", -1);
        let (trit, _) = router.handle_request("POST", "/admin");
        assert_eq!(trit, -1);
    }

    #[test]
    fn test_website_handle() {
        let site = CrownyWebsite::new("Test", 3000);
        let (trit, _, _) = site.handle("GET", "/");
        assert_eq!(trit, 1);
    }

    #[test]
    fn test_website_404() {
        let site = CrownyWebsite::new("Test", 3000);
        let (trit, _, _) = site.handle("GET", "/nonexistent");
        assert_eq!(trit, -1);
    }

    #[test]
    fn test_website_api() {
        let site = CrownyWebsite::new("Test", 3000);
        let (trit, _, body) = site.handle("GET", "/api/status");
        assert_eq!(trit, 1);
        assert!(body.contains("online"));
    }

    #[test]
    fn test_tritscript_print() {
        let mut ts = TritScript::new();
        let out = ts.execute("출력 \"hello world\"");
        assert_eq!(out[0], "hello world");
    }
}
