// ═══════════════════════════════════════════════════════════════
// Crowny Live Consensus — 실제 HTTP 3포트 합의
// TCP 소켓 연결 · JSON 요청/응답 · 타임아웃 · 폴백
// Claude:18789 · Gemini:18790 · Sonnet:18791
// ═══════════════════════════════════════════════════════════════

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

// ═══════════════════════════════════════
// 노드 설정
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ConsensusNode {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub api_path: String,
    pub timeout_ms: u64,
    pub status: NodeStatus,
    pub latency_ms: Option<u64>,
    pub last_response: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Timeout,
    Error(String),
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "● 온라인"),
            Self::Offline => write!(f, "○ 오프라인"),
            Self::Timeout => write!(f, "⏳ 타임아웃"),
            Self::Error(e) => write!(f, "✗ {}", e),
        }
    }
}

impl ConsensusNode {
    pub fn new(name: &str, host: &str, port: u16, path: &str) -> Self {
        Self {
            name: name.into(), host: host.into(), port, api_path: path.into(),
            timeout_ms: 5000, status: NodeStatus::Offline,
            latency_ms: None, last_response: None,
        }
    }

    /// TCP 연결 + HTTP POST 요청 전송
    pub fn send_request(&mut self, query: &str) -> Result<HttpResponse, String> {
        let start = Instant::now();
        let addr = format!("{}:{}", self.host, self.port);

        // 1. TCP 연결
        let timeout = Duration::from_millis(self.timeout_ms);
        let mut stream = match TcpStream::connect_timeout(
            &addr.parse().map_err(|e| format!("주소 파싱 실패: {}", e))?,
            timeout
        ) {
            Ok(s) => s,
            Err(e) => {
                self.status = if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::ConnectionRefused {
                    NodeStatus::Offline
                } else {
                    NodeStatus::Error(e.to_string())
                };
                return Err(format!("{} 연결 실패: {}", self.name, e));
            }
        };

        stream.set_read_timeout(Some(timeout)).ok();
        stream.set_write_timeout(Some(timeout)).ok();

        // 2. HTTP POST 요청 생성
        let body = format!(
            r#"{{"query":"{}","model":"{}","trit_mode":"consensus","ctp":"PPPPOOOOO","timestamp":{}}}"#,
            query.replace('"', r#"\""#), self.name, now_ms()
        );

        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\nX-CTP: PPPPOOOOO\r\nX-Trit-Mode: consensus\r\n\r\n{}",
            self.api_path, self.host, self.port, body.len(), body
        );

        // 3. 전송
        if let Err(e) = stream.write_all(request.as_bytes()) {
            self.status = NodeStatus::Error(format!("전송 실패: {}", e));
            return Err(format!("{} 전송 실패: {}", self.name, e));
        }

        // 4. 응답 수신
        let mut response_buf = Vec::new();
        match stream.read_to_end(&mut response_buf) {
            Ok(_) => {}
            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
                    self.status = NodeStatus::Timeout;
                    return Err(format!("{} 타임아웃 ({}ms)", self.name, self.timeout_ms));
                }
                // 연결이 닫히면서 데이터를 받은 경우 OK
                if response_buf.is_empty() {
                    self.status = NodeStatus::Error(format!("수신 실패: {}", e));
                    return Err(format!("{} 수신 실패: {}", self.name, e));
                }
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        self.latency_ms = Some(elapsed);

        let raw = String::from_utf8_lossy(&response_buf).to_string();
        self.last_response = Some(raw.clone());
        self.status = NodeStatus::Online;

        Ok(HttpResponse::parse(&raw, elapsed))
    }

    /// 핑 테스트 (TCP 연결만)
    pub fn ping(&mut self) -> Result<u64, String> {
        let start = Instant::now();
        let addr = format!("{}:{}", self.host, self.port);
        let timeout = Duration::from_millis(2000);

        match TcpStream::connect_timeout(
            &addr.parse().map_err(|e| format!("{}", e))?,
            timeout
        ) {
            Ok(_) => {
                let ms = start.elapsed().as_millis() as u64;
                self.status = NodeStatus::Online;
                self.latency_ms = Some(ms);
                Ok(ms)
            }
            Err(e) => {
                self.status = NodeStatus::Offline;
                Err(format!("{}", e))
            }
        }
    }
}

// ═══════════════════════════════════════
// HTTP 응답 파서
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub latency_ms: u64,
    pub raw: String,
}

impl HttpResponse {
    pub fn parse(raw: &str, latency_ms: u64) -> Self {
        let mut headers = HashMap::new();
        let mut status_code = 0u16;
        let mut body = String::new();

        let parts: Vec<&str> = raw.splitn(2, "\r\n\r\n").collect();
        if let Some(header_section) = parts.first() {
            for (i, line) in header_section.lines().enumerate() {
                if i == 0 {
                    // HTTP/1.1 200 OK
                    let p: Vec<&str> = line.split_whitespace().collect();
                    if p.len() >= 2 { status_code = p[1].parse().unwrap_or(0); }
                } else if let Some(colon) = line.find(':') {
                    let key = line[..colon].trim().to_string();
                    let val = line[colon + 1..].trim().to_string();
                    headers.insert(key, val);
                }
            }
        }
        if parts.len() > 1 { body = parts[1].to_string(); }

        Self { status_code, headers, body, latency_ms, raw: raw.to_string() }
    }

    pub fn is_ok(&self) -> bool { self.status_code >= 200 && self.status_code < 300 }
}

// ═══════════════════════════════════════
// 합의 투표
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ConsensusVote {
    pub node_name: String,
    pub trit: i8,           // P=1, O=0, T=-1
    pub reason: String,
    pub latency_ms: u64,
    pub status: NodeStatus,
    pub raw_response: Option<String>,
}

impl std::fmt::Display for ConsensusVote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.trit { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} — {} ({}ms)", label, self.node_name, self.reason, self.latency_ms)
    }
}

#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub query: String,
    pub votes: Vec<ConsensusVote>,
    pub consensus_trit: i8,
    pub confidence: f64,
    pub total_latency_ms: u64,
    pub ctp_header: [i8; 9],
    pub timestamp: u64,
    pub nodes_online: usize,
    pub nodes_total: usize,
}

impl ConsensusResult {
    pub fn label(&self) -> &str { match self.consensus_trit { 1 => "P", -1 => "T", _ => "O" } }
    pub fn ctp_string(&self) -> String {
        self.ctp_header.iter().map(|t| match t { 1 => 'P', -1 => 'T', _ => 'O' }).collect()
    }
}

impl std::fmt::Display for ConsensusResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] 합의 완료 — 신뢰도:{:.0}% | 노드:{}/{} | {}ms | CTP:{}",
            self.label(), self.confidence * 100.0,
            self.nodes_online, self.nodes_total,
            self.total_latency_ms, self.ctp_string())
    }
}

// ═══════════════════════════════════════
// 라이브 합의 엔진
// ═══════════════════════════════════════

pub struct LiveConsensus {
    pub nodes: Vec<ConsensusNode>,
    pub history: Vec<ConsensusResult>,
    pub fallback_enabled: bool,
}

impl LiveConsensus {
    pub fn new() -> Self {
        Self {
            nodes: vec![
                ConsensusNode::new("Claude", "127.0.0.1", 18789, "/v1/consensus"),
                ConsensusNode::new("Gemini", "127.0.0.1", 18790, "/v1/consensus"),
                ConsensusNode::new("Sonnet", "127.0.0.1", 18791, "/v1/consensus"),
            ],
            history: Vec::new(),
            fallback_enabled: true,
        }
    }

    pub fn with_nodes(nodes: Vec<ConsensusNode>) -> Self {
        Self { nodes, history: Vec::new(), fallback_enabled: true }
    }

    /// 모든 노드 핑 체크
    pub fn health_check(&mut self) -> Vec<(String, Result<u64, String>)> {
        let mut results = Vec::new();
        for node in &mut self.nodes {
            let result = node.ping();
            results.push((node.name.clone(), result));
        }
        results
    }

    /// 3포트 실제 HTTP 합의 실행
    pub fn execute(&mut self, query: &str) -> ConsensusResult {
        let start = Instant::now();
        let mut votes = Vec::new();
        let mut online = 0;

        for node in &mut self.nodes {
            let vote = match node.send_request(query) {
                Ok(response) => {
                    online += 1;
                    // JSON 응답에서 trit 파싱
                    let trit = Self::parse_trit_from_response(&response.body);
                    let reason = Self::parse_reason_from_response(&response.body)
                        .unwrap_or_else(|| format!("HTTP {} ({}ms)", response.status_code, response.latency_ms));

                    ConsensusVote {
                        node_name: node.name.clone(),
                        trit,
                        reason,
                        latency_ms: response.latency_ms,
                        status: NodeStatus::Online,
                        raw_response: Some(response.body),
                    }
                }
                Err(err) => {
                    // 폴백: 시뮬레이션 투표
                    if self.fallback_enabled {
                        let fallback_trit = Self::fallback_vote(query, &node.name);
                        ConsensusVote {
                            node_name: node.name.clone(),
                            trit: fallback_trit,
                            reason: format!("(폴백) {} — {}", err, Self::fallback_reason(fallback_trit)),
                            latency_ms: 0,
                            status: node.status.clone(),
                            raw_response: None,
                        }
                    } else {
                        ConsensusVote {
                            node_name: node.name.clone(),
                            trit: 0,
                            reason: format!("오프라인: {}", err),
                            latency_ms: 0,
                            status: node.status.clone(),
                            raw_response: None,
                        }
                    }
                }
            };
            votes.push(vote);
        }

        // 합의 계산
        let p = votes.iter().filter(|v| v.trit > 0).count();
        let t = votes.iter().filter(|v| v.trit < 0).count();
        let consensus_trit = if p > t { 1 } else if t > p { -1 } else { 0 };

        let max_agree = p.max(t).max(votes.len() - p - t);
        let confidence = if votes.is_empty() { 0.0 } else { max_agree as f64 / votes.len() as f64 };

        let total_latency = start.elapsed().as_millis() as u64;

        // CTP 헤더
        let mut ctp = [0i8; 9];
        ctp[0] = consensus_trit;
        ctp[1] = 1; // permission
        ctp[2] = if p == votes.len() || t == votes.len() { 1 } else { 0 }; // unanimous
        ctp[3] = if online >= 2 { 1 } else { 0 }; // quorum
        ctp[4] = 1; // routing
        for (i, v) in votes.iter().take(4).enumerate() {
            ctp[5 + i] = v.trit;
        }

        let result = ConsensusResult {
            query: query.into(), votes, consensus_trit, confidence,
            total_latency_ms: total_latency, ctp_header: ctp,
            timestamp: now_ms(), nodes_online: online, nodes_total: self.nodes.len(),
        };

        self.history.push(result.clone());
        result
    }

    // JSON에서 trit 값 추출
    fn parse_trit_from_response(body: &str) -> i8 {
        // {"trit":"P",...} 또는 {"trit":1,...}
        if body.contains("\"P\"") || body.contains("\"positive\"") || body.contains(":1") { return 1; }
        if body.contains("\"T\"") || body.contains("\"negative\"") || body.contains(":-1") { return -1; }
        // 응답 내용 분석
        let lower = body.to_lowercase();
        let positive = ["approve", "agree", "yes", "good", "accept", "승인", "동의", "적합"].iter()
            .filter(|k| lower.contains(*k)).count();
        let negative = ["reject", "deny", "no", "bad", "refuse", "거부", "부적합", "위험"].iter()
            .filter(|k| lower.contains(*k)).count();
        if positive > negative { 1 } else if negative > positive { -1 } else { 0 }
    }

    fn parse_reason_from_response(body: &str) -> Option<String> {
        // {"reason":"..."} 추출
        if let Some(start) = body.find("\"reason\"") {
            let rest = &body[start..];
            if let Some(colon) = rest.find(':') {
                let value_part = rest[colon + 1..].trim();
                if value_part.starts_with('"') {
                    if let Some(end) = value_part[1..].find('"') {
                        return Some(value_part[1..=end].to_string());
                    }
                }
            }
        }
        None
    }

    // 폴백 투표 (노드 오프라인 시)
    fn fallback_vote(query: &str, node_name: &str) -> i8 {
        let hash = query.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let node_offset = node_name.bytes().fold(0u64, |acc, b| acc.wrapping_mul(17).wrapping_add(b as u64));
        match (hash.wrapping_add(node_offset)) % 6 {
            0 | 1 | 2 | 3 => 1,  // 67% P
            4 => 0,              // 17% O
            _ => -1,             // 17% T
        }
    }

    fn fallback_reason(trit: i8) -> &'static str {
        match trit { 1 => "분석 완료, 적합 판단", 0 => "추가 검토 필요", _ => "리스크 요소 감지" }
    }

    pub fn status_summary(&self) -> String {
        let online = self.nodes.iter().filter(|n| n.status == NodeStatus::Online).count();
        let mut lines = Vec::new();
        lines.push(format!("━━━ OpenClaw Live Consensus ━━━"));
        lines.push(format!("  노드: {}/{} 온라인", online, self.nodes.len()));
        for node in &self.nodes {
            let latency = node.latency_ms.map(|l| format!("{}ms", l)).unwrap_or("-".into());
            lines.push(format!("  {} :{} — {} ({})", node.name, node.port, node.status, latency));
        }
        lines.push(format!("  이력: {} 합의 완료", self.history.len()));
        lines.push(format!("  폴백: {}", if self.fallback_enabled { "활성" } else { "비활성" }));
        lines.join("\n")
    }
}

// ═══════════════════════════════════════
// 간이 HTTP 서버 (테스트용)
// ═══════════════════════════════════════

use std::net::TcpListener;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

pub struct MockConsensusServer {
    pub name: String,
    pub port: u16,
    pub running: Arc<AtomicBool>,
}

impl MockConsensusServer {
    pub fn new(name: &str, port: u16) -> Self {
        Self { name: name.into(), port, running: Arc::new(AtomicBool::new(false)) }
    }

    /// 백그라운드에서 간이 서버 시작 (테스트용)
    pub fn start(&self) -> Result<(), String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.port))
            .map_err(|e| format!("바인딩 실패 :{} — {}", self.port, e))?;
        listener.set_nonblocking(true).ok();
        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();
        let name = self.name.clone();
        let port = self.port;

        std::thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 4096];
                        let n = stream.read(&mut buf).unwrap_or(0);
                        let request = String::from_utf8_lossy(&buf[..n]).to_string();

                        // 요청에서 query 추출
                        let query = Self::extract_query(&request);
                        let trit = Self::decide_trit(&query, &name);
                        let trit_label = match trit { 1 => "P", -1 => "T", _ => "O" };
                        let reason = match trit {
                            1 => "분석 완료, 승인 권고",
                            -1 => "리스크 요소 감지, 거부",
                            _ => "추가 정보 필요, 보류",
                        };

                        let body = format!(
                            r#"{{"trit":"{}","node":"{}","port":{},"reason":"{}","query":"{}","timestamp":{}}}"#,
                            trit_label, name, port, reason, query.replace('"', ""), now_ms()
                        );

                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-CTP: PPPPOOOOO\r\nX-Trit: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            trit_label, body.len(), body
                        );
                        stream.write_all(response.as_bytes()).ok();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(())
    }

    pub fn stop(&self) { self.running.store(false, Ordering::SeqCst); }

    fn extract_query(request: &str) -> String {
        if let Some(body_start) = request.find("\r\n\r\n") {
            let body = &request[body_start + 4..];
            if let Some(q_start) = body.find("\"query\"") {
                let rest = &body[q_start..];
                if let Some(colon) = rest.find(':') {
                    let val = rest[colon + 1..].trim();
                    if val.starts_with('"') {
                        if let Some(end) = val[1..].find('"') {
                            return val[1..=end].to_string();
                        }
                    }
                }
            }
        }
        "unknown".into()
    }

    fn decide_trit(query: &str, node_name: &str) -> i8 {
        let hash = query.bytes().chain(node_name.bytes())
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        match hash % 5 {
            0 | 1 | 2 => 1,  // 60% P
            3 => 0,          // 20% O
            _ => -1,         // 20% T
        }
    }
}

// ═══ 데모 ═══

pub fn demo_live_consensus() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  OpenClaw Live Consensus — 실제 HTTP 합의      ║");
    println!("║  Claude:18789 · Gemini:18790 · Sonnet:18791   ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    // 1. 간이 서버 시작
    println!("━━━ 1. 합의 노드 시작 ━━━");
    let servers = vec![
        MockConsensusServer::new("Claude", 18789),
        MockConsensusServer::new("Gemini", 18790),
        MockConsensusServer::new("Sonnet", 18791),
    ];

    let mut all_started = true;
    for server in &servers {
        match server.start() {
            Ok(_) => println!("  [P] {} :{} 시작", server.name, server.port),
            Err(e) => {
                println!("  [T] {} :{} — {}", server.name, server.port, e);
                all_started = false;
            }
        }
    }
    if !all_started {
        println!("  ⚠ 일부 노드 시작 실패 — 폴백 모드 사용");
    }
    // 서버 준비 대기
    std::thread::sleep(Duration::from_millis(200));
    println!();

    // 2. 헬스 체크
    println!("━━━ 2. 헬스 체크 ━━━");
    let mut consensus = LiveConsensus::new();
    let health = consensus.health_check();
    for (name, result) in &health {
        match result {
            Ok(ms) => println!("  [P] {} — {}ms", name, ms),
            Err(e) => println!("  [T] {} — {}", name, e),
        }
    }
    println!();

    // 3. 합의 실행
    println!("━━━ 3. 합의 실행 ━━━");
    let queries = vec![
        "CRWN 토큰 상장 적합성 평가",
        "TVM 스마트 컨트랙트 보안 감사",
        "의료 AI 진단 보조 승인 여부",
        "신규 밸리데이터 alice 스테이킹 검증",
        "한선어 v2.0 릴리스 승인",
    ];

    for query in &queries {
        println!("  질문: \"{}\"", query);
        let result = consensus.execute(query);

        for vote in &result.votes {
            let online = if vote.status == NodeStatus::Online { "📡" } else { "📴" };
            println!("    {} {}", online, vote);
        }
        println!("  ──→ {}", result);
        println!();
    }

    // 4. 상세 응답 확인
    println!("━━━ 4. 원시 HTTP 응답 ━━━");
    if let Some(last) = consensus.history.last() {
        for vote in &last.votes {
            println!("  [{}] {}:", if vote.status == NodeStatus::Online { "LIVE" } else { "FALLBACK" }, vote.node_name);
            if let Some(raw) = &vote.raw_response {
                let display: String = raw.chars().take(100).collect();
                println!("    {}", display);
            } else {
                println!("    (폴백 응답)");
            }
        }
    }
    println!();

    // 5. 상태 요약
    println!("━━━ 5. 상태 요약 ━━━");
    println!("{}", consensus.status_summary());
    println!();

    // 6. 합의 이력
    println!("━━━ 6. 합의 이력 ━━━");
    for (i, result) in consensus.history.iter().enumerate() {
        println!("  #{} [{}] \"{}\" — {:.0}% | {}ms | CTP:{}",
            i + 1, result.label(), result.query,
            result.confidence * 100.0, result.total_latency_ms, result.ctp_string());
    }
    println!();

    // 서버 중지
    for server in &servers { server.stop(); }
    std::thread::sleep(Duration::from_millis(100));

    println!("✓ OpenClaw Live Consensus 데모 완료 — {} 합의, {} 노드",
        consensus.history.len(), consensus.nodes.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = ConsensusNode::new("Test", "127.0.0.1", 9999, "/api");
        assert_eq!(node.name, "Test");
        assert_eq!(node.port, 9999);
        assert_eq!(node.status, NodeStatus::Offline);
    }

    #[test]
    fn test_http_response_parse() {
        let raw = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Trit: P\r\n\r\n{\"trit\":\"P\",\"reason\":\"ok\"}";
        let resp = HttpResponse::parse(raw, 10);
        assert_eq!(resp.status_code, 200);
        assert!(resp.is_ok());
        assert!(resp.body.contains("trit"));
    }

    #[test]
    fn test_http_response_404() {
        let raw = "HTTP/1.1 404 Not Found\r\n\r\n";
        let resp = HttpResponse::parse(raw, 5);
        assert_eq!(resp.status_code, 404);
        assert!(!resp.is_ok());
    }

    #[test]
    fn test_parse_trit_positive() {
        let trit = LiveConsensus::parse_trit_from_response(r#"{"trit":"P","reason":"ok"}"#);
        assert_eq!(trit, 1);
    }

    #[test]
    fn test_parse_trit_negative() {
        let trit = LiveConsensus::parse_trit_from_response(r#"{"trit":"T","reason":"reject bad"}"#);
        assert_eq!(trit, -1);
    }

    #[test]
    fn test_parse_trit_content_analysis() {
        assert_eq!(LiveConsensus::parse_trit_from_response("I approve this request"), 1);
        assert_eq!(LiveConsensus::parse_trit_from_response("I reject and deny this"), -1);
    }

    #[test]
    fn test_fallback_vote_deterministic() {
        let v1 = LiveConsensus::fallback_vote("test query", "Claude");
        let v2 = LiveConsensus::fallback_vote("test query", "Claude");
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_mock_server_start_stop() {
        let server = MockConsensusServer::new("Test", 19999);
        assert!(server.start().is_ok());
        std::thread::sleep(Duration::from_millis(100));
        server.stop();
    }

    #[test]
    fn test_live_consensus_with_mock() {
        // 빈 포트에 서버 시작
        let server = MockConsensusServer::new("TestNode", 19876);
        if server.start().is_ok() {
            std::thread::sleep(Duration::from_millis(200));

            let mut consensus = LiveConsensus::with_nodes(vec![
                ConsensusNode::new("TestNode", "127.0.0.1", 19876, "/v1/consensus"),
            ]);

            let result = consensus.execute("테스트 합의");
            assert!(result.votes.len() == 1);
            assert!(result.votes[0].latency_ms < 5000);

            server.stop();
        }
    }

    #[test]
    fn test_consensus_result_ctp() {
        let result = ConsensusResult {
            query: "test".into(),
            votes: vec![
                ConsensusVote { node_name: "A".into(), trit: 1, reason: "ok".into(), latency_ms: 10, status: NodeStatus::Online, raw_response: None },
                ConsensusVote { node_name: "B".into(), trit: 1, reason: "ok".into(), latency_ms: 15, status: NodeStatus::Online, raw_response: None },
            ],
            consensus_trit: 1, confidence: 1.0, total_latency_ms: 25,
            ctp_header: [1, 1, 1, 1, 1, 1, 1, 0, 0], timestamp: 0,
            nodes_online: 2, nodes_total: 2,
        };
        assert_eq!(result.label(), "P");
        assert_eq!(result.ctp_string(), "PPPPPPPOO");
    }

    #[test]
    fn test_offline_fallback() {
        let mut consensus = LiveConsensus::with_nodes(vec![
            ConsensusNode::new("Offline", "127.0.0.1", 59999, "/api"),
        ]);
        consensus.fallback_enabled = true;
        let result = consensus.execute("폴백 테스트");
        assert_eq!(result.votes.len(), 1);
        // 폴백이므로 응답은 있지만 raw_response는 None
        assert!(result.votes[0].raw_response.is_none());
    }
}
