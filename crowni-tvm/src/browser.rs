// ═══════════════════════════════════════════════════════════════
// Crowny Browser Engine — 3진 전용 웹브라우저
// CTP 프로토콜 · 3진 DOM · TritScript · .crwn 렌더링
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

// ═══════════════════════════════════════
// CTP 프로토콜 (HTTP 대체)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CTPRequest {
    pub method: CTPMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub trit_header: [i8; 9],
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CTPMethod { GET, POST, SUBMIT, VOTE, SYNC }

impl std::fmt::Display for CTPMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GET => write!(f, "GET"),
            Self::POST => write!(f, "POST"),
            Self::SUBMIT => write!(f, "SUBMIT"),
            Self::VOTE => write!(f, "VOTE"),
            Self::SYNC => write!(f, "SYNC"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CTPHttpResponse {
    pub status: u16,
    pub trit_state: i8,
    pub ctp_header: [i8; 9],
    pub content_type: String,
    pub body: String,
    pub latency_ms: u32,
}

impl CTPHttpResponse {
    pub fn ok(body: &str, content_type: &str) -> Self {
        Self {
            status: 200, trit_state: 1, ctp_header: [1,1,1,1,0,0,0,0,0],
            content_type: content_type.into(), body: body.into(), latency_ms: 0,
        }
    }
    pub fn not_found() -> Self {
        Self {
            status: 404, trit_state: -1, ctp_header: [-1,0,0,0,0,0,0,0,0],
            content_type: "text/plain".into(), body: "Not Found".into(), latency_ms: 0,
        }
    }
}

// ═══════════════════════════════════════
// 3진 DOM (TritDOM)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub enum TritElement {
    Document { children: Vec<TritElement>, title: String, lang: String },
    Container { tag: String, attrs: HashMap<String, String>, children: Vec<TritElement>, trit: i8 },
    Text { content: String, trit: i8 },
    Input { input_type: String, name: String, value: String, trit: i8 },
    TritBadge { state: i8, size: String },
    CTPHeader { header: [i8; 9] },
    Chart { data_type: String, data: Vec<f64> },
    Script { lang: String, code: String },
    Style { rules: String },
}

impl TritElement {
    pub fn render(&self, indent: usize) -> String {
        let pad = " ".repeat(indent);
        match self {
            Self::Document { children, title, lang } => {
                let mut out = format!("{}<!DOCTYPE crwn lang=\"{}\">\n", pad, lang);
                out.push_str(&format!("{}<문서 제목=\"{}\">\n", pad, title));
                for child in children { out.push_str(&child.render(indent + 2)); }
                out.push_str(&format!("{}</문서>\n", pad));
                out
            }
            Self::Container { tag, attrs, children, trit } => {
                let t = match trit { 1 => " trit=\"P\"", -1 => " trit=\"T\"", _ => "" };
                let a: String = attrs.iter().map(|(k, v)| format!(" {}=\"{}\"", k, v)).collect();
                let mut out = format!("{}<{}{}{}>\n", pad, tag, a, t);
                for child in children { out.push_str(&child.render(indent + 2)); }
                out.push_str(&format!("{}</{}>\n", pad, tag));
                out
            }
            Self::Text { content, trit } => {
                let t = match trit { 1 => "✓", -1 => "✗", _ => "" };
                format!("{}{}{}\n", pad, t, content)
            }
            Self::Input { input_type, name, value, .. } => {
                format!("{}<입력 종류=\"{}\" 이름=\"{}\" 값=\"{}\" />\n", pad, input_type, name, value)
            }
            Self::TritBadge { state, size } => {
                let label = match state { 1 => "P", -1 => "T", _ => "O" };
                format!("{}<트릿뱃지 상태=\"{}\" 크기=\"{}\" />\n", pad, label, size)
            }
            Self::CTPHeader { header } => {
                let h: String = header.iter().map(|t| match t { 1 => 'P', -1 => 'T', _ => 'O' }).collect();
                format!("{}<CTP헤더>{}</CTP헤더>\n", pad, h)
            }
            Self::Chart { data_type, data } => {
                format!("{}<차트 종류=\"{}\" 데이터=\"{:?}\" />\n", pad, data_type, data)
            }
            Self::Script { lang, code } => {
                format!("{}<스크립트 언어=\"{}\">\n{}  {}\n{}</스크립트>\n", pad, lang, pad, code, pad)
            }
            Self::Style { rules } => {
                format!("{}<스타일>\n{}  {}\n{}</스타일>\n", pad, pad, rules, pad)
            }
        }
    }

    pub fn child_count(&self) -> usize {
        match self {
            Self::Document { children, .. } | Self::Container { children, .. } => children.len(),
            _ => 0,
        }
    }
}

// ═══════════════════════════════════════
// 3진 마크업 파서 (.crwn)
// ═══════════════════════════════════════

pub struct CrwnParser;

impl CrwnParser {
    pub fn parse(source: &str) -> TritElement {
        let lines: Vec<&str> = source.lines().collect();
        let mut children = Vec::new();
        let mut title = "크라운 페이지".to_string();
        let mut lang = "ko".to_string();

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("제목:") || trimmed.starts_with("title:") {
                title = trimmed.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if trimmed.starts_with("언어:") || trimmed.starts_with("lang:") {
                lang = trimmed.split(':').nth(1).unwrap_or("ko").trim().to_string();
            } else if trimmed.starts_with("# ") {
                children.push(TritElement::Container {
                    tag: "제목1".into(),
                    attrs: HashMap::new(),
                    children: vec![TritElement::Text { content: trimmed[2..].to_string(), trit: 0 }],
                    trit: 0,
                });
            } else if trimmed.starts_with("## ") {
                children.push(TritElement::Container {
                    tag: "제목2".into(),
                    attrs: HashMap::new(),
                    children: vec![TritElement::Text { content: trimmed[3..].to_string(), trit: 0 }],
                    trit: 0,
                });
            } else if trimmed.starts_with("[P]") || trimmed.starts_with("[O]") || trimmed.starts_with("[T]") {
                let trit = match &trimmed[..3] { "[P]" => 1, "[T]" => -1, _ => 0 };
                children.push(TritElement::Text { content: trimmed[3..].trim().to_string(), trit });
            } else if trimmed.starts_with("트릿:") {
                let state = match trimmed.split(':').nth(1).unwrap_or("O").trim() {
                    "P" => 1, "T" => -1, _ => 0,
                };
                children.push(TritElement::TritBadge { state, size: "md".into() });
            } else if trimmed.starts_with("입력:") {
                let parts: Vec<&str> = trimmed[7..].split('|').collect();
                children.push(TritElement::Input {
                    input_type: parts.first().unwrap_or(&"text").trim().to_string(),
                    name: parts.get(1).unwrap_or(&"field").trim().to_string(),
                    value: String::new(), trit: 0,
                });
            } else if trimmed.starts_with("스크립트:") {
                let code = trimmed.split(':').nth(1).unwrap_or("").trim();
                children.push(TritElement::Script { lang: "한선어".into(), code: code.to_string() });
            } else if trimmed.starts_with("---") {
                children.push(TritElement::Container {
                    tag: "구분선".into(), attrs: HashMap::new(), children: Vec::new(), trit: 0,
                });
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                children.push(TritElement::Text { content: trimmed.to_string(), trit: 0 });
            }
        }

        TritElement::Document { children, title, lang }
    }
}

// ═══════════════════════════════════════
// 브라우저 엔진
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct BrowserTab {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub content: String,
    pub trit_state: i8,
    pub loaded: bool,
    pub history: Vec<String>,
}

pub struct CrownyBrowser {
    pub tabs: Vec<BrowserTab>,
    pub active_tab: u32,
    pub bookmarks: Vec<(String, String)>, // (title, url)
    pub pages: HashMap<String, String>,   // url → content (로컬 서빙)
    pub cache: HashMap<String, String>,
    pub extensions: Vec<String>,
    pub settings: BrowserSettings,
}

#[derive(Debug, Clone)]
pub struct BrowserSettings {
    pub default_page: String,
    pub trit_indicator: bool,
    pub dark_mode: bool,
    pub ctp_strict: bool,
    pub lang: String,
}

impl Default for BrowserSettings {
    fn default() -> Self {
        Self {
            default_page: "crwn://home".into(),
            trit_indicator: true,
            dark_mode: true,
            ctp_strict: true,
            lang: "ko".into(),
        }
    }
}

impl CrownyBrowser {
    pub fn new() -> Self {
        let mut browser = Self {
            tabs: Vec::new(),
            active_tab: 0,
            bookmarks: vec![
                ("홈".into(), "crwn://home".into()),
                ("플랫폼".into(), "crwn://platform".into()),
                ("거래소".into(), "crwn://exchange".into()),
                ("문서".into(), "crwn://docs".into()),
            ],
            pages: HashMap::new(),
            cache: HashMap::new(),
            extensions: vec!["3진 DevTools".into(), "CRWN 지갑".into(), "CTP 분석기".into()],
            settings: BrowserSettings::default(),
        };
        browser.register_builtin_pages();
        browser.new_tab("crwn://home");
        browser
    }

    fn register_builtin_pages(&mut self) {
        self.pages.insert("crwn://home".into(), r#"제목: Crowny Browser 홈
언어: ko

# 크라운 브라우저에 오신 걸 환영합니다

[P] 3진법 기반 차세대 웹 브라우저
[P] CTP 프로토콜 — HTTP를 넘어서
[P] TritDOM — 모든 요소에 3진 상태

---

## 빠른 시작
트릿: P
[P] 모든 웹 요소가 P(승인) / O(보류) / T(거부) 상태를 가집니다
[O] 합의가 필요한 작업은 3개 AI가 투표합니다
[T] 거부된 요청은 CTP 헤더에 기록됩니다

---

## 추천 사이트
[P] crwn://platform — 통합 플랫폼 대시보드
[P] crwn://exchange — CRWN 토큰 거래소
[P] crwn://docs — 개발자 문서
"#.into());

        self.pages.insert("crwn://platform".into(), r#"제목: Crowny Platform
언어: ko

# Crowny Platform

## Git 호스팅
[P] crowny/tvm-core — 3진 VM 코어 (Rust)
[P] crowny/hanseon-lang — 한선어 컴파일러
[P] crowny/exchange-ui — 거래소 UI (React)

---

## 배포 현황
[P] docs.crowny.dev — 문서 사이트 (Ready)
[P] exchange.crowny.dev — 거래소 (Ready)
[P] api.crowny.dev — API 게이트웨이 (Ready)

---

## 런타임
[P] tvm-server — :8000 (5 replicas)
[P] consensus-node — :8001 (5 replicas)
[O] hanseon-repl — :8002 (1 replica)
"#.into());

        self.pages.insert("crwn://exchange".into(), r#"제목: CRWN Exchange
언어: ko

# CRWN 거래소

## 시세
[P] CRWN/USDT: $0.1240 (+2.5%)
[O] 24h 거래량: $45,000,000
트릿: P

---

## 계정
[P] alice: 989,970 CRWN (스테이킹: 100,000)
[P] bob: 510,000 CRWN (스테이킹: 50,000)
[P] carol: 255,000 CRWN

---

## 최근 거래
[P] alice → bob: 5,000 CRWN (서비스 대금)
[P] bob → carol: 2,500 CRWN (합의 보수)
[T] nobody → alice: 999,999 CRWN (잔액 부족)
"#.into());

        self.pages.insert("crwn://docs".into(), r#"제목: 개발자 문서
언어: ko

# Crowny 개발자 문서

## .crwn 파일 형식
[P] 3진 마크업 언어 — 모든 요소에 P/O/T 상태
스크립트: 값 42  보여줘  끝

---

## CTP 프로토콜
[P] GET — 리소스 조회
[P] POST — 데이터 전송
[P] SUBMIT — CAR.submit() 실행
[P] VOTE — 3진 투표
[P] SYNC — 상태 동기화

---

## TritDOM API
[P] 문서.생성("태그") — 요소 생성
[P] 요소.트릿설정(P|O|T) — 3진 상태
[P] 요소.자식추가(자식) — 자식 추가
"#.into());
    }

    pub fn new_tab(&mut self, url: &str) -> u32 {
        let id = self.tabs.len() as u32;
        self.tabs.push(BrowserTab {
            id, url: url.into(), title: String::new(),
            content: String::new(), trit_state: 0, loaded: false,
            history: vec![url.into()],
        });
        self.active_tab = id;
        self.navigate(id, url);
        id
    }

    pub fn navigate(&mut self, tab_id: u32, url: &str) -> CTPHttpResponse {
        let content = self.pages.get(url).cloned();
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.url = url.into();
            tab.history.push(url.into());
            if let Some(ref c) = content {
                let doc = CrwnParser::parse(c);
                tab.content = doc.render(0);
                tab.loaded = true;
                tab.trit_state = 1;
                if let TritElement::Document { title, .. } = &doc {
                    tab.title = title.clone();
                }
                CTPHttpResponse::ok(&tab.content, "text/crwn")
            } else {
                tab.trit_state = -1;
                tab.loaded = false;
                CTPHttpResponse::not_found()
            }
        } else {
            CTPHttpResponse::not_found()
        }
    }

    pub fn current_tab(&self) -> Option<&BrowserTab> {
        self.tabs.iter().find(|t| t.id == self.active_tab)
    }

    pub fn render_tab_bar(&self) -> String {
        self.tabs.iter().map(|t| {
            let active = if t.id == self.active_tab { "▶" } else { " " };
            let trit = match t.trit_state { 1 => "●", -1 => "✗", _ => "○" };
            let title = if t.title.is_empty() { &t.url } else { &t.title };
            let short_title: String = title.chars().take(15).collect();
            format!("{}{} {}", active, trit, short_title)
        }).collect::<Vec<_>>().join(" | ")
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push("═══ Crowny Browser ═══".to_string());
        lines.push(format!("  탭: {} | 활성: #{}", self.tabs.len(), self.active_tab));
        lines.push(format!("  탭바: [{}]", self.render_tab_bar()));
        lines.push(format!("  북마크: {}", self.bookmarks.len()));
        lines.push(format!("  확장: {:?}", self.extensions));
        lines.push(format!("  페이지: {} 등록", self.pages.len()));
        lines.push(format!("  모드: {} | CTP: {}",
            if self.settings.dark_mode { "다크" } else { "라이트" },
            if self.settings.ctp_strict { "엄격" } else { "완화" }));
        lines.join("\n")
    }
}

// ═══ 데모 ═══

pub fn demo_browser() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Browser — 3진 전용 웹브라우저          ║");
    println!("║  CTP 프로토콜 · TritDOM · .crwn 렌더링         ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut browser = CrownyBrowser::new();

    // 1. 홈페이지
    println!("━━━ 1. crwn://home ━━━");
    println!("  [{}]", browser.render_tab_bar());
    if let Some(tab) = browser.current_tab() {
        println!("{}", tab.content);
    }

    // 2. 탭 추가
    println!("━━━ 2. 멀티탭 네비게이션 ━━━");
    browser.new_tab("crwn://platform");
    browser.new_tab("crwn://exchange");
    browser.new_tab("crwn://docs");
    println!("  [{}]", browser.render_tab_bar());
    println!();

    // 3. 각 페이지 렌더링
    for tab in &browser.tabs {
        let trit = match tab.trit_state { 1 => "P", -1 => "T", _ => "O" };
        println!("  탭#{} [{}] {} — {}", tab.id, trit, tab.title, tab.url);
    }
    println!();

    // 4. CTP 요청 데모
    println!("━━━ 3. CTP 프로토콜 요청 ━━━");
    let requests = vec![
        CTPRequest { method: CTPMethod::GET, url: "crwn://home".into(), headers: HashMap::new(), trit_header: [1,1,1,0,0,0,0,0,0], body: None },
        CTPRequest { method: CTPMethod::SUBMIT, url: "crwn://api/consensus".into(), headers: HashMap::new(), trit_header: [0,1,0,0,0,0,0,0,0], body: Some("투자 판단 요청".into()) },
        CTPRequest { method: CTPMethod::VOTE, url: "crwn://api/vote".into(), headers: HashMap::new(), trit_header: [1,1,1,1,1,0,0,0,0], body: Some("P".into()) },
    ];
    for req in &requests {
        let trit: String = req.trit_header.iter().map(|t| match t { 1 => 'P', -1 => 'T', _ => 'O' }).collect();
        println!("  {} {} — CTP:{}", req.method, req.url, trit);
    }
    println!();

    // 5. .crwn 파일 파싱
    println!("━━━ 4. .crwn 파일 파싱 ━━━");
    let custom_crwn = r#"제목: 나의 첫 크라운 페이지
언어: ko

# 안녕하세요! 크라운 웹입니다

[P] 이것은 승인된 콘텐츠입니다
[O] 이것은 보류 중인 콘텐츠입니다
[T] 이것은 거부된 콘텐츠입니다

---

## 3진 계산기
입력: number|operand1
입력: number|operand2
트릿: P

스크립트: 값 42 값 58 더하기 보여줘

[P] CTP 프로토콜로 안전하게 전송됩니다
"#;
    let doc = CrwnParser::parse(custom_crwn);
    println!("{}", doc.render(2));

    // 6. 브라우저 요약
    println!("━━━ 5. 브라우저 상태 ━━━");
    println!("{}", browser.summary());
    println!();

    println!("✓ 크라운 브라우저 데모 완료 — {} 탭, {} 페이지", browser.tabs.len(), browser.pages.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_creation() {
        let browser = CrownyBrowser::new();
        assert_eq!(browser.tabs.len(), 1);
        assert!(browser.pages.len() >= 4);
    }

    #[test]
    fn test_new_tab() {
        let mut browser = CrownyBrowser::new();
        browser.new_tab("crwn://platform");
        assert_eq!(browser.tabs.len(), 2);
        assert_eq!(browser.active_tab, 1);
    }

    #[test]
    fn test_navigate() {
        let mut browser = CrownyBrowser::new();
        let resp = browser.navigate(0, "crwn://exchange");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.trit_state, 1);
    }

    #[test]
    fn test_navigate_not_found() {
        let mut browser = CrownyBrowser::new();
        let resp = browser.navigate(0, "crwn://nonexistent");
        assert_eq!(resp.status, 404);
    }

    #[test]
    fn test_crwn_parse() {
        let doc = CrwnParser::parse("제목: Test\n# Hello\n[P] Good");
        if let TritElement::Document { children, title, .. } = &doc {
            assert_eq!(title, "Test");
            assert!(children.len() >= 2);
        } else {
            panic!("Expected Document");
        }
    }

    #[test]
    fn test_trit_element_render() {
        let elem = TritElement::TritBadge { state: 1, size: "md".into() };
        let rendered = elem.render(0);
        assert!(rendered.contains("P"));
    }

    #[test]
    fn test_ctp_response() {
        let resp = CTPHttpResponse::ok("body", "text/crwn");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.trit_state, 1);
    }

    #[test]
    fn test_tab_bar() {
        let browser = CrownyBrowser::new();
        let bar = browser.render_tab_bar();
        assert!(bar.contains("▶"));
    }
}
