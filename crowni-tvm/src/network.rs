///! ═══════════════════════════════════════════════════
///! Trit Network Adapter — 균형3진 네트워크 프로토콜
///! ═══════════════════════════════════════════════════
///!
///! 2진 TCP/IP를 균형3진 프로토콜로 감싸는 어댑터.
///! 상위 레이어는 절대 2진 네트워크를 직접 보지 않는다.
///!
///! 프로토콜 구조:
///! ┌──────────────────────────────────────────────┐
///! │           Crowny Trit Protocol (CTP)         │
///! ├──────────────────────────────────────────────┤
///! │ [Magic 6-trit: PTOPTP]                      │
///! │ [Version: 2-trit]                           │
///! │ [MessageType: 2-trit]                       │
///! │ [Status: 1-trit (P/O/T)]                    │
///! │ [PayloadLen: 6-trit (0~728)]                │
///! │ [Payload: N trits]                          │
///! │ [Checksum: 6-trit]                          │
///! └──────────────────────────────────────────────┘
///!
///! HTTP 위에 얹는 방식:
///!   X-Crowny-State: P/O/T
///!   X-Crowny-Version: 1.0
///!   Content-Type: application/x-crowny-trit
///!
///! 내부적으로는 2진 바이트로 직렬화하지만
///! API는 100% 3진 인터페이스.

use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};

// ─────────────────────────────────────────────
// Trit Encoding (물리 매핑)
// ─────────────────────────────────────────────

/// 단일 Trit → 2bit 물리 매핑 (FPGA 이전 전략)
/// T(-1) = 0b00
/// O( 0) = 0b01  
/// P(+1) = 0b10
/// 0b11  = 무효(사용 안 함)
///
/// 1 바이트에 4 trit 저장 가능 (8bit / 2bit = 4)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum NetTrit {
    T = -1,
    O = 0,
    P = 1,
}

impl NetTrit {
    pub fn to_2bit(self) -> u8 {
        match self {
            NetTrit::T => 0b00,
            NetTrit::O => 0b01,
            NetTrit::P => 0b10,
        }
    }

    pub fn from_2bit(v: u8) -> Option<Self> {
        match v & 0b11 {
            0b00 => Some(NetTrit::T),
            0b01 => Some(NetTrit::O),
            0b10 => Some(NetTrit::P),
            _ => None, // 0b11 = 무효
        }
    }

    pub fn symbol(self) -> char {
        match self {
            NetTrit::T => 'T',
            NetTrit::O => 'O',
            NetTrit::P => 'P',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'T' | 't' => Some(NetTrit::T),
            'O' | 'o' | '0' => Some(NetTrit::O),
            'P' | 'p' => Some(NetTrit::P),
            _ => None,
        }
    }
}

impl std::fmt::Display for NetTrit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

// ─────────────────────────────────────────────
// Trit Buffer (트릿 직렬화/역직렬화)
// ─────────────────────────────────────────────

/// 트릿 버퍼 — 3진 데이터를 2진 바이트로 직렬화
#[derive(Debug, Clone)]
pub struct TritBuffer {
    pub trits: Vec<NetTrit>,
}

impl TritBuffer {
    pub fn new() -> Self {
        Self { trits: Vec::new() }
    }

    pub fn from_trits(trits: Vec<NetTrit>) -> Self {
        Self { trits }
    }

    pub fn push(&mut self, t: NetTrit) {
        self.trits.push(t);
    }

    pub fn push_i8(&mut self, v: i8) {
        self.push(match v {
            -1 => NetTrit::T,
            0 => NetTrit::O,
            1 => NetTrit::P,
            _ => NetTrit::O,
        });
    }

    /// 정수를 6-trit 균형3진으로 인코딩 (범위: -364~+364)
    pub fn push_word6(&mut self, mut val: i16) {
        let mut trits = [NetTrit::O; 6];
        for i in 0..6 {
            let mut r = val % 3;
            val /= 3;
            if r > 1 { r -= 3; val += 1; }
            else if r < -1 { r += 3; val -= 1; }
            trits[i] = match r {
                -1 => NetTrit::T,
                0 => NetTrit::O,
                1 => NetTrit::P,
                _ => NetTrit::O,
            };
        }
        // 상위 트릿부터 push (MST first)
        for i in (0..6).rev() {
            self.push(trits[i]);
        }
    }

    /// 6-trit → 정수 디코딩
    pub fn read_word6(&self, offset: usize) -> Option<i16> {
        if offset + 6 > self.trits.len() { return None; }
        let mut val: i16 = 0;
        for i in 0..6 {
            val = val * 3 + self.trits[offset + i] as i8 as i16;
        }
        Some(val)
    }

    /// 문자열을 트릿 시퀀스로 인코딩 (각 char → 6-trit)
    pub fn push_string(&mut self, s: &str) {
        for ch in s.chars() {
            self.push_word6(ch as i16);
        }
    }

    // ── 2진 직렬화 (물리 전송용) ──

    /// 트릿 버퍼 → 바이트 배열 (4 trits per byte)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let chunks = self.trits.chunks(4);
        for chunk in chunks {
            let mut byte: u8 = 0;
            for (i, t) in chunk.iter().enumerate() {
                byte |= t.to_2bit() << (6 - i * 2);
            }
            // 남은 자리는 0b11(무효)로 패딩
            for i in chunk.len()..4 {
                byte |= 0b11 << (6 - i * 2);
            }
            bytes.push(byte);
        }
        bytes
    }

    /// 바이트 배열 → 트릿 버퍼
    pub fn from_bytes(bytes: &[u8], trit_count: usize) -> Self {
        let mut trits = Vec::with_capacity(trit_count);
        for byte in bytes {
            for i in 0..4 {
                if trits.len() >= trit_count { break; }
                let bits = (byte >> (6 - i * 2)) & 0b11;
                if let Some(t) = NetTrit::from_2bit(bits) {
                    trits.push(t);
                }
            }
        }
        Self { trits }
    }

    /// 길이
    pub fn len(&self) -> usize {
        self.trits.len()
    }

    /// 트릿 문자열 표현
    pub fn to_trit_string(&self) -> String {
        self.trits.iter().map(|t| t.symbol()).collect()
    }
}

impl std::fmt::Display for TritBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_trit_string())
    }
}

// ─────────────────────────────────────────────
// CTP Message (Crowny Trit Protocol)
// ─────────────────────────────────────────────

/// 메시지 타입 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum MessageType {
    Request  =  1,  // P = 요청
    Info     =  0,  // O = 정보/알림
    Response = -1,  // T = 응답
}

impl MessageType {
    pub fn to_trit(self) -> NetTrit {
        match self {
            MessageType::Request => NetTrit::P,
            MessageType::Info => NetTrit::O,
            MessageType::Response => NetTrit::T,
        }
    }

    pub fn from_trit(t: NetTrit) -> Self {
        match t {
            NetTrit::P => MessageType::Request,
            NetTrit::O => MessageType::Info,
            NetTrit::T => MessageType::Response,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Request => write!(f, "요청(P)"),
            MessageType::Info => write!(f, "정보(O)"),
            MessageType::Response => write!(f, "응답(T)"),
        }
    }
}

/// 상태 코드 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum StatusCode {
    Success =  1,  // P = 성공
    Neutral =  0,  // O = 중립/진행중
    Error   = -1,  // T = 오류
}

impl StatusCode {
    pub fn to_trit(self) -> NetTrit {
        match self {
            StatusCode::Success => NetTrit::P,
            StatusCode::Neutral => NetTrit::O,
            StatusCode::Error => NetTrit::T,
        }
    }

    pub fn from_trit(t: NetTrit) -> Self {
        match t {
            NetTrit::P => StatusCode::Success,
            NetTrit::O => StatusCode::Neutral,
            NetTrit::T => StatusCode::Error,
        }
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusCode::Success => write!(f, "P(성공)"),
            StatusCode::Neutral => write!(f, "O(중립)"),
            StatusCode::Error => write!(f, "T(오류)"),
        }
    }
}

/// CTP 메시지
#[derive(Debug, Clone)]
pub struct CtpMessage {
    pub version: u8,           // 프로토콜 버전 (1)
    pub msg_type: MessageType, // 메시지 종류
    pub status: StatusCode,    // 상태
    pub payload: TritBuffer,   // 페이로드 (트릿 데이터)
}

/// 매직 넘버: "PTOPTP" (6-trit)
const MAGIC: [NetTrit; 6] = [
    NetTrit::P, NetTrit::T, NetTrit::O,
    NetTrit::P, NetTrit::T, NetTrit::P,
];

impl CtpMessage {
    pub fn new(msg_type: MessageType, status: StatusCode, payload: TritBuffer) -> Self {
        Self {
            version: 1,
            msg_type,
            status,
            payload,
        }
    }

    /// 요청 메시지 생성
    pub fn request(payload: TritBuffer) -> Self {
        Self::new(MessageType::Request, StatusCode::Neutral, payload)
    }

    /// 응답 메시지 생성
    pub fn response(status: StatusCode, payload: TritBuffer) -> Self {
        Self::new(MessageType::Response, status, payload)
    }

    /// 직렬화 → 트릿 버퍼
    pub fn serialize(&self) -> TritBuffer {
        let mut buf = TritBuffer::new();

        // Magic (6 trits)
        for t in &MAGIC { buf.push(*t); }

        // Version (2 trits) — version 1 = OO→OP
        buf.push_i8(0);
        buf.push_i8(self.version as i8);

        // MessageType (2 trits)
        buf.push(self.msg_type.to_trit());
        buf.push(NetTrit::O); // 예약

        // Status (1 trit)
        buf.push(self.status.to_trit());

        // PayloadLen (6 trits)
        buf.push_word6(self.payload.len() as i16);

        // Payload
        for t in &self.payload.trits {
            buf.push(*t);
        }

        // Checksum (6 trits) — 간단한 체크섬: 모든 트릿 합의 mod 729
        let sum: i32 = buf.trits.iter().map(|t| *t as i8 as i32).sum();
        buf.push_word6((sum % 364) as i16);

        buf
    }

    /// 역직렬화 ← 트릿 버퍼
    pub fn deserialize(buf: &TritBuffer) -> Result<Self, String> {
        if buf.len() < 18 { // 최소: magic(6) + ver(2) + type(2) + status(1) + len(6) + checksum(6) = 23
            return Err("메시지 너무 짧음".into());
        }

        // Magic 확인
        for i in 0..6 {
            if buf.trits[i] != MAGIC[i] {
                return Err("매직 넘버 불일치".into());
            }
        }

        // Version
        let version = buf.trits[7] as i8;

        // MessageType
        let msg_type = MessageType::from_trit(buf.trits[8]);

        // Status
        let status = StatusCode::from_trit(buf.trits[10]);

        // PayloadLen
        let payload_len = buf.read_word6(11)
            .ok_or("페이로드 길이 읽기 실패")? as usize;

        // Payload
        let payload_start = 17;
        let payload_end = payload_start + payload_len;
        if payload_end > buf.len() {
            return Err("페이로드 길이 초과".into());
        }

        let payload = TritBuffer::from_trits(
            buf.trits[payload_start..payload_end].to_vec()
        );

        Ok(Self {
            version: version as u8,
            msg_type,
            status,
            payload,
        })
    }

    /// HTTP 헤더 생성 (기존 HTTP 위에 얹기)
    pub fn to_http_headers(&self) -> Vec<(String, String)> {
        vec![
            ("X-Crowny-Version".into(), format!("{}", self.version)),
            ("X-Crowny-Type".into(), format!("{}", self.msg_type)),
            ("X-Crowny-State".into(), format!("{}", self.status.to_trit())),
            ("X-Crowny-PayloadLen".into(), format!("{}", self.payload.len())),
            ("Content-Type".into(), "application/x-crowny-trit".into()),
        ]
    }
}

impl std::fmt::Display for CtpMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CTP[v{} {} {} payload:{}trits]",
            self.version, self.msg_type, self.status, self.payload.len())
    }
}

// ─────────────────────────────────────────────
// Trit Network Adapter (TCP 래퍼)
// ─────────────────────────────────────────────

/// 3진 네트워크 어댑터
/// 2진 TCP를 감싸서 CTP 메시지를 주고받음
pub struct TritNetAdapter;

impl TritNetAdapter {
    /// CTP 메시지를 TCP 스트림으로 전송
    pub fn send(stream: &mut TcpStream, msg: &CtpMessage) -> io::Result<usize> {
        let trit_buf = msg.serialize();
        let bytes = trit_buf.to_bytes();

        // 프레임: [trit_count: 4 bytes BE][trit_data: N bytes]
        let trit_count = trit_buf.len() as u32;
        stream.write_all(&trit_count.to_be_bytes())?;
        stream.write_all(&bytes)?;
        stream.flush()?;

        Ok(4 + bytes.len())
    }

    /// TCP 스트림에서 CTP 메시지 수신
    pub fn recv(stream: &mut TcpStream) -> io::Result<CtpMessage> {
        // trit_count 읽기
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let trit_count = u32::from_be_bytes(len_buf) as usize;

        // 바이트 데이터 읽기
        let byte_count = (trit_count + 3) / 4; // ceil(trit_count/4)
        let mut data = vec![0u8; byte_count];
        stream.read_exact(&mut data)?;

        let trit_buf = TritBuffer::from_bytes(&data, trit_count);
        CtpMessage::deserialize(&trit_buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// 3진 TCP 서버 시작 (간단한 에코 서버)
    pub fn start_server(addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr)?;
        println!("[CTP서버] {} 에서 대기 중...", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("[CTP서버] 연결: {}", stream.peer_addr()?);
                    match Self::recv(&mut stream) {
                        Ok(msg) => {
                            println!("[CTP서버] 수신: {}", msg);
                            // 에코 응답
                            let resp = CtpMessage::response(
                                StatusCode::Success,
                                msg.payload.clone(),
                            );
                            let _ = Self::send(&mut stream, &resp);
                        }
                        Err(e) => eprintln!("[CTP서버] 수신 오류: {}", e),
                    }
                }
                Err(e) => eprintln!("[CTP서버] 연결 오류: {}", e),
            }
        }
        Ok(())
    }

    /// 3진 TCP 클라이언트 — 메시지 전송 후 응답 수신
    pub fn send_request(addr: &str, msg: &CtpMessage) -> io::Result<CtpMessage> {
        let mut stream = TcpStream::connect(addr)?;
        Self::send(&mut stream, msg)?;
        Self::recv(&mut stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_2bit_roundtrip() {
        for t in [NetTrit::T, NetTrit::O, NetTrit::P] {
            let bits = t.to_2bit();
            let back = NetTrit::from_2bit(bits).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn test_trit_buffer_bytes_roundtrip() {
        let trits = vec![
            NetTrit::P, NetTrit::T, NetTrit::O, NetTrit::P,
            NetTrit::T, NetTrit::P, NetTrit::O,
        ];
        let buf = TritBuffer::from_trits(trits.clone());
        let bytes = buf.to_bytes();
        let restored = TritBuffer::from_bytes(&bytes, trits.len());
        assert_eq!(buf.to_trit_string(), restored.to_trit_string());
    }

    #[test]
    fn test_word6_encoding() {
        let mut buf = TritBuffer::new();
        buf.push_word6(42);
        assert_eq!(buf.len(), 6);
        let val = buf.read_word6(0).unwrap();
        assert_eq!(val, 42);
    }

    #[test]
    fn test_ctp_message_serialize() {
        let mut payload = TritBuffer::new();
        payload.push_word6(100);
        payload.push_word6(200);

        let msg = CtpMessage::request(payload);
        let serialized = msg.serialize();

        // Magic 확인
        assert_eq!(serialized.trits[0], NetTrit::P);
        assert_eq!(serialized.trits[1], NetTrit::T);
        assert_eq!(serialized.trits[2], NetTrit::O);

        println!("직렬화: {}", serialized);
        println!("바이트: {} bytes", serialized.to_bytes().len());
    }

    #[test]
    fn test_http_headers() {
        let msg = CtpMessage::response(StatusCode::Success, TritBuffer::new());
        let headers = msg.to_http_headers();
        assert!(headers.iter().any(|(k, _)| k == "X-Crowny-State"));
        assert!(headers.iter().any(|(k, v)| k == "Content-Type" && v == "application/x-crowny-trit"));
    }
}
