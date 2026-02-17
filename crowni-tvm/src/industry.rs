// ═══════════════════════════════════════════════════════════════
// Crowny Industry Applications
// 산업 적용 — 의료 AI · 교육 AI · 트레이딩 AI
// 모두 3진 합의 (P/O/T) 기반 의사결정
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════
// 공통: 3진 판정
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum Trit { P, O, T }

impl Trit {
    pub fn label(&self) -> &str { match self { Trit::P => "P", Trit::O => "O", Trit::T => "T" } }
    pub fn kr(&self) -> &str { match self { Trit::P => "승인", Trit::O => "보류", Trit::T => "거부" } }
    pub fn val(&self) -> i8 { match self { Trit::P => 1, Trit::O => 0, Trit::T => -1 } }

    pub fn consensus(votes: &[Trit]) -> Trit {
        let p = votes.iter().filter(|v| **v == Trit::P).count();
        let t = votes.iter().filter(|v| **v == Trit::T).count();
        if p > t { Trit::P } else if t > p { Trit::T } else { Trit::O }
    }

    pub fn confidence(votes: &[Trit]) -> f64 {
        let con = Self::consensus(votes);
        let agree = votes.iter().filter(|v| **v == con).count();
        if votes.is_empty() { 0.0 } else { agree as f64 / votes.len() as f64 }
    }
}

impl std::fmt::Display for Trit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.label(), self.kr())
    }
}

#[derive(Debug, Clone)]
pub struct IndustryDecision {
    pub category: String,
    pub query: String,
    pub ai_votes: Vec<(String, Trit, String)>, // (모델명, 판정, 근거)
    pub consensus: Trit,
    pub confidence: f64,
    pub risk_level: RiskLevel,
    pub recommendation: String,
    pub ctp: [i8; 9],
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum RiskLevel { Low, Medium, High, Critical }

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "🟢 낮음"),
            Self::Medium => write!(f, "🟡 중간"),
            Self::High => write!(f, "🟠 높음"),
            Self::Critical => write!(f, "🔴 위험"),
        }
    }
}

impl std::fmt::Display for IndustryDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ctp: String = self.ctp.iter().map(|t| match t { 1 => 'P', -1 => 'T', _ => 'O' }).collect();
        write!(f, "[{}] {} — {} ({:.0}%) | 위험: {} | CTP: {}",
            self.category, self.consensus, self.recommendation,
            self.confidence * 100.0, self.risk_level, ctp)
    }
}

fn build_ctp(consensus: &Trit, votes: &[Trit]) -> [i8; 9] {
    let mut h = [0i8; 9];
    h[0] = consensus.val();
    h[1] = 1; // permission OK
    h[2] = if votes.iter().all(|v| v == consensus) { 1 } else { 0 };
    h[3] = if votes.len() >= 2 { 1 } else { 0 };
    h[4] = 1; // routing OK
    for (i, v) in votes.iter().take(4).enumerate() { h[5 + i] = v.val(); }
    h
}

// ═══════════════════════════════════════
// 1. 의료 AI 판단 시스템
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Patient {
    pub id: String,
    pub name: String,
    pub age: u32,
    pub gender: String,
    pub symptoms: Vec<String>,
    pub vitals: Vitals,
    pub history: Vec<String>,
    pub allergies: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Vitals {
    pub bp_systolic: u32,
    pub bp_diastolic: u32,
    pub heart_rate: u32,
    pub temperature: f32,
    pub spo2: u32,
    pub blood_sugar: u32,
}

impl Vitals {
    pub fn risk_score(&self) -> f64 {
        let mut score: f64 = 0.0;
        if self.bp_systolic > 140 || self.bp_systolic < 90 { score += 0.2; }
        if self.bp_diastolic > 90 || self.bp_diastolic < 60 { score += 0.15; }
        if self.heart_rate > 100 || self.heart_rate < 50 { score += 0.2; }
        if self.temperature > 38.5 || self.temperature < 35.0 { score += 0.15; }
        if self.spo2 < 95 { score += 0.2; }
        if self.blood_sugar > 200 || self.blood_sugar < 70 { score += 0.1; }
        if score > 1.0 { 1.0 } else { score }
    }
}

#[derive(Debug, Clone)]
pub struct MedicalDecision {
    pub patient: Patient,
    pub question: String,
    pub decision: IndustryDecision,
    pub suggested_tests: Vec<String>,
    pub contraindications: Vec<String>,
}

pub struct MedicalAI {
    pub decisions: Vec<MedicalDecision>,
}

impl MedicalAI {
    pub fn new() -> Self { Self { decisions: Vec::new() } }

    pub fn evaluate(&mut self, patient: &Patient, question: &str) -> MedicalDecision {
        let risk_score = patient.vitals.risk_score();
        let is_surgery = question.contains("수술") || question.contains("시술");
        let is_medication = question.contains("약") || question.contains("투약") || question.contains("처방");
        let is_discharge = question.contains("퇴원") || question.contains("외래");

        // 3개 AI 투표
        let claude_vote = if risk_score < 0.3 {
            (Trit::P, "바이탈 안정, 임상 지표 양호".to_string())
        } else if risk_score < 0.6 {
            (Trit::O, "일부 지표 이상, 추가 검사 권장".to_string())
        } else {
            (Trit::T, "복수 지표 이상, 즉각 중재 필요".to_string())
        };

        let gemini_vote = if is_surgery && patient.age > 70 {
            (Trit::O, "고령 환자, 비침습적 대안 우선 검토".to_string())
        } else if is_surgery && risk_score < 0.4 {
            (Trit::P, "수술 적응증 충족, 마취 위험 낮음".to_string())
        } else if is_medication && !patient.allergies.is_empty() {
            (Trit::O, "알레르기 이력 확인 필요".to_string())
        } else if risk_score > 0.5 {
            (Trit::T, "현 상태에서 추가 안정화 필요".to_string())
        } else {
            (Trit::P, "임상적으로 진행 가능".to_string())
        };

        let sonnet_vote = if is_discharge && patient.vitals.spo2 < 95 {
            (Trit::T, "SpO2 저하, 퇴원 부적합".to_string())
        } else if patient.symptoms.len() > 3 && risk_score > 0.3 {
            (Trit::O, "다증상 + 지표 이상, 경과 관찰 권장".to_string())
        } else if risk_score < 0.25 {
            (Trit::P, "전반적 양호, 진행 추천".to_string())
        } else {
            (Trit::O, "주의 관찰 하에 조건부 진행".to_string())
        };

        let votes = vec![claude_vote.0.clone(), gemini_vote.0.clone(), sonnet_vote.0.clone()];
        let consensus = Trit::consensus(&votes);
        let confidence = Trit::confidence(&votes);

        let risk_level = if risk_score > 0.6 { RiskLevel::Critical }
            else if risk_score > 0.4 { RiskLevel::High }
            else if risk_score > 0.2 { RiskLevel::Medium }
            else { RiskLevel::Low };

        let recommendation = match (&consensus, is_surgery) {
            (Trit::P, true) => "수술 진행 승인 — 표준 프로토콜 적용".to_string(),
            (Trit::P, false) => "치료 진행 승인".to_string(),
            (Trit::O, true) => "수술 보류 — 추가 검사 후 재평가".to_string(),
            (Trit::O, false) => "경과 관찰 후 재판단 필요".to_string(),
            (Trit::T, _) => "현 시점 진행 불가 — 안정화 우선".to_string(),
        };

        let suggested_tests = if consensus != Trit::P {
            vec!["CBC (전혈구검사)".into(), "CRP (C반응성단백)".into(), "심전도".into()]
        } else { Vec::new() };

        let contraindications = patient.allergies.iter()
            .map(|a| format!("{} 알레르기 주의", a))
            .collect();

        let ai_votes = vec![
            ("Claude".to_string(), claude_vote.0, claude_vote.1),
            ("Gemini".to_string(), gemini_vote.0, gemini_vote.1),
            ("Sonnet".to_string(), sonnet_vote.0, sonnet_vote.1),
        ];

        let decision = IndustryDecision {
            category: "의료".to_string(),
            query: question.to_string(),
            ai_votes,
            consensus: consensus.clone(),
            confidence,
            risk_level,
            recommendation,
            ctp: build_ctp(&consensus, &votes),
            timestamp: now_ms(),
        };

        let med_decision = MedicalDecision {
            patient: patient.clone(),
            question: question.to_string(),
            decision,
            suggested_tests,
            contraindications,
        };
        self.decisions.push(med_decision.clone());
        med_decision
    }
}

// ═══════════════════════════════════════
// 2. 교육 AI 시스템
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Student {
    pub id: String,
    pub name: String,
    pub grade: String,
    pub subjects: Vec<SubjectScore>,
    pub learning_style: LearningStyle,
    pub attendance_rate: f64,
}

#[derive(Debug, Clone)]
pub struct SubjectScore {
    pub subject: String,
    pub score: f64,
    pub trend: Trit,   // P: 상승, O: 유지, T: 하락
}

#[derive(Debug, Clone, PartialEq)]
pub enum LearningStyle { Visual, Auditory, Kinesthetic, ReadWrite }

impl std::fmt::Display for LearningStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Visual => write!(f, "시각형"),
            Self::Auditory => write!(f, "청각형"),
            Self::Kinesthetic => write!(f, "체험형"),
            Self::ReadWrite => write!(f, "독서형"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EducationPlan {
    pub student: Student,
    pub decision: IndustryDecision,
    pub recommended_path: String,
    pub focus_subjects: Vec<String>,
    pub methods: Vec<String>,
    pub weekly_hours: u32,
}

pub struct EducationAI {
    pub plans: Vec<EducationPlan>,
}

impl EducationAI {
    pub fn new() -> Self { Self { plans: Vec::new() } }

    pub fn evaluate(&mut self, student: &Student, question: &str) -> EducationPlan {
        let avg_score = if student.subjects.is_empty() { 0.0 }
            else { student.subjects.iter().map(|s| s.score).sum::<f64>() / student.subjects.len() as f64 };
        let weak_subjects: Vec<_> = student.subjects.iter()
            .filter(|s| s.score < 60.0 || s.trend == Trit::T).collect();
        let strong_subjects: Vec<_> = student.subjects.iter()
            .filter(|s| s.score >= 80.0 && s.trend == Trit::P).collect();
        let is_advanced = question.contains("심화") || question.contains("영재") || question.contains("올림피아드");
        let is_remedial = question.contains("보충") || question.contains("기초") || question.contains("부진");

        // 3개 AI 투표
        let claude_vote = if avg_score >= 80.0 && student.attendance_rate > 0.9 {
            (Trit::P, format!("평균 {:.0}점, 출석 {:.0}%, 학업 역량 우수", avg_score, student.attendance_rate * 100.0))
        } else if avg_score >= 60.0 {
            (Trit::O, format!("평균 {:.0}점, 부분적 보강 필요", avg_score))
        } else {
            (Trit::T, format!("평균 {:.0}점, 기초 학력 강화 시급", avg_score))
        };

        let gemini_vote = if is_advanced && !weak_subjects.is_empty() {
            (Trit::O, "심화 진행 전 취약 과목 보강 우선".to_string())
        } else if is_advanced && avg_score >= 85.0 {
            (Trit::P, "심화 과정 적합, 도전 학습 권장".to_string())
        } else if is_remedial || weak_subjects.len() >= 2 {
            (Trit::O, "맞춤형 보충 학습 프로그램 필요".to_string())
        } else {
            (Trit::P, "현 커리큘럼 진행 적합".to_string())
        };

        let sonnet_vote = match &student.learning_style {
            LearningStyle::Visual if strong_subjects.len() >= 2 =>
                (Trit::P, "시각형 학습자, 인포그래픽/영상 교재 활용 추천".to_string()),
            LearningStyle::Kinesthetic =>
                (Trit::O, "체험형 학습자, 실습 위주 커리큘럼 조정 권장".to_string()),
            _ if student.attendance_rate < 0.8 =>
                (Trit::T, format!("출석률 {:.0}%, 학습 동기 부여 프로그램 필요", student.attendance_rate * 100.0)),
            _ => (Trit::P, "현 학습 방향 유지 적합".to_string()),
        };

        let votes = vec![claude_vote.0.clone(), gemini_vote.0.clone(), sonnet_vote.0.clone()];
        let consensus = Trit::consensus(&votes);
        let confidence = Trit::confidence(&votes);

        let risk_level = if avg_score < 40.0 { RiskLevel::Critical }
            else if avg_score < 60.0 { RiskLevel::High }
            else if weak_subjects.len() >= 2 { RiskLevel::Medium }
            else { RiskLevel::Low };

        let recommended_path = match &consensus {
            Trit::P => if is_advanced { "심화 과정 진행".to_string() }
                else { "정규 커리큘럼 유지".to_string() },
            Trit::O => "맞춤형 보강 프로그램 편성".to_string(),
            Trit::T => "기초 학력 회복 프로그램 긴급 편성".to_string(),
        };

        let focus_subjects = weak_subjects.iter().map(|s| s.subject.clone()).collect();
        let methods = match &student.learning_style {
            LearningStyle::Visual => vec!["인포그래픽".into(), "영상 강의".into(), "마인드맵".into()],
            LearningStyle::Auditory => vec!["토론 수업".into(), "오디오북".into(), "그룹 학습".into()],
            LearningStyle::Kinesthetic => vec!["실험/실습".into(), "프로젝트 기반".into(), "현장 학습".into()],
            LearningStyle::ReadWrite => vec!["독서 과제".into(), "에세이 작성".into(), "노트 필기".into()],
        };

        let weekly_hours = if consensus == Trit::T { 15 } else if consensus == Trit::O { 10 } else { 6 };

        let ai_votes = vec![
            ("Claude".to_string(), claude_vote.0, claude_vote.1),
            ("Gemini".to_string(), gemini_vote.0, gemini_vote.1),
            ("Sonnet".to_string(), sonnet_vote.0, sonnet_vote.1),
        ];

        let plan = EducationPlan {
            student: student.clone(),
            decision: IndustryDecision {
                category: "교육".to_string(),
                query: question.to_string(),
                ai_votes,
                consensus,
                confidence,
                risk_level,
                recommendation: recommended_path.clone(),
                ctp: build_ctp(&Trit::consensus(&votes), &votes),
                timestamp: now_ms(),
            },
            recommended_path,
            focus_subjects,
            methods,
            weekly_hours,
        };
        self.plans.push(plan.clone());
        plan
    }
}

// ═══════════════════════════════════════
// 3. 트레이딩 AI 시그널 시스템
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct MarketData {
    pub symbol: String,
    pub price: f64,
    pub change_24h: f64,
    pub volume_24h: f64,
    pub rsi: f64,          // 0-100
    pub macd: f64,
    pub bollinger_pos: f64, // 0.0 (하단) ~ 1.0 (상단)
    pub fear_greed: u32,    // 0-100
    pub support: f64,
    pub resistance: f64,
}

#[derive(Debug, Clone)]
pub enum TradeAction { Buy, Hold, Sell, StrongBuy, StrongSell }

impl std::fmt::Display for TradeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Buy => write!(f, "🟢 매수"),
            Self::Hold => write!(f, "🟡 관망"),
            Self::Sell => write!(f, "🔴 매도"),
            Self::StrongBuy => write!(f, "🟢🟢 강력 매수"),
            Self::StrongSell => write!(f, "🔴🔴 강력 매도"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TradeSignal {
    pub market: MarketData,
    pub decision: IndustryDecision,
    pub action: TradeAction,
    pub entry_price: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub position_size_pct: f64,
}

pub struct TradingAI {
    pub signals: Vec<TradeSignal>,
}

impl TradingAI {
    pub fn new() -> Self { Self { signals: Vec::new() } }

    pub fn analyze(&mut self, market: &MarketData) -> TradeSignal {
        let is_oversold = market.rsi < 30.0;
        let is_overbought = market.rsi > 70.0;
        let near_support = market.price < market.support * 1.02;
        let near_resistance = market.price > market.resistance * 0.98;
        let bullish_macd = market.macd > 0.0;
        let high_fear = market.fear_greed < 25;
        let high_greed = market.fear_greed > 75;

        // 3개 AI 투표
        let claude_vote = if is_oversold && near_support && bullish_macd {
            (Trit::P, format!("RSI {:.0} 과매도 + 지지선 근접 + MACD 상승 → 매수 신호", market.rsi))
        } else if is_overbought && near_resistance {
            (Trit::T, format!("RSI {:.0} 과매수 + 저항선 근접 → 매도 고려", market.rsi))
        } else {
            (Trit::O, format!("RSI {:.0}, 명확한 방향성 없음 → 관망", market.rsi))
        };

        let gemini_vote = if market.change_24h > 5.0 && high_greed {
            (Trit::T, format!("24h +{:.1}% + 탐욕 {} → 과열, 차익 실현", market.change_24h, market.fear_greed))
        } else if market.change_24h < -5.0 && high_fear {
            (Trit::P, format!("24h {:.1}% + 공포 {} → 패닉 매도, 역발상 매수", market.change_24h, market.fear_greed))
        } else if market.volume_24h > 1_000_000_000.0 && bullish_macd {
            (Trit::P, "높은 거래량 + MACD 양전환 → 상승 모멘텀".to_string())
        } else {
            (Trit::O, "혼재된 시그널 → 추가 확인 필요".to_string())
        };

        let sonnet_vote = if market.bollinger_pos < 0.1 {
            (Trit::P, format!("볼린저 하단 {:.2} → 반등 기대", market.bollinger_pos))
        } else if market.bollinger_pos > 0.9 {
            (Trit::T, format!("볼린저 상단 {:.2} → 하락 반전 가능", market.bollinger_pos))
        } else if is_oversold && market.change_24h < -3.0 {
            (Trit::P, "기술적 과매도 + 급락 → 단기 반등 유력".to_string())
        } else {
            (Trit::O, "중립 구간, 브레이크아웃 대기".to_string())
        };

        let votes = vec![claude_vote.0.clone(), gemini_vote.0.clone(), sonnet_vote.0.clone()];
        let consensus = Trit::consensus(&votes);
        let confidence = Trit::confidence(&votes);

        let action = match (&consensus, confidence) {
            (Trit::P, c) if c >= 0.99 => TradeAction::StrongBuy,
            (Trit::P, _) => TradeAction::Buy,
            (Trit::T, c) if c >= 0.99 => TradeAction::StrongSell,
            (Trit::T, _) => TradeAction::Sell,
            _ => TradeAction::Hold,
        };

        let risk_level = if market.change_24h.abs() > 10.0 { RiskLevel::Critical }
            else if market.change_24h.abs() > 5.0 { RiskLevel::High }
            else if market.rsi < 25.0 || market.rsi > 75.0 { RiskLevel::Medium }
            else { RiskLevel::Low };

        let stop_loss = match &action {
            TradeAction::Buy | TradeAction::StrongBuy => market.price * 0.95,
            TradeAction::Sell | TradeAction::StrongSell => market.price * 1.05,
            TradeAction::Hold => market.price,
        };
        let take_profit = match &action {
            TradeAction::Buy | TradeAction::StrongBuy => market.price * 1.10,
            TradeAction::Sell | TradeAction::StrongSell => market.price * 0.90,
            TradeAction::Hold => market.price,
        };
        let position_size_pct = match &risk_level {
            RiskLevel::Low => 10.0,
            RiskLevel::Medium => 5.0,
            RiskLevel::High => 2.0,
            RiskLevel::Critical => 1.0,
        };

        let ai_votes = vec![
            ("Claude".to_string(), claude_vote.0, claude_vote.1),
            ("Gemini".to_string(), gemini_vote.0, gemini_vote.1),
            ("Sonnet".to_string(), sonnet_vote.0, sonnet_vote.1),
        ];

        let signal = TradeSignal {
            market: market.clone(),
            decision: IndustryDecision {
                category: "트레이딩".to_string(),
                query: format!("{} 매매 판단", market.symbol),
                ai_votes,
                consensus,
                confidence,
                risk_level,
                recommendation: format!("{}", action),
                ctp: build_ctp(&Trit::consensus(&votes), &votes),
                timestamp: now_ms(),
            },
            action,
            entry_price: market.price,
            stop_loss,
            take_profit,
            position_size_pct,
        };
        self.signals.push(signal.clone());
        signal
    }
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

// ═══ 데모 ═══

pub fn demo_industry() {
    println!("╔═══════════════════════════════════════════╗");
    println!("║  Crowny Industry Applications             ║");
    println!("║  산업 적용 — 의료 · 교육 · 트레이딩 AI     ║");
    println!("╚═══════════════════════════════════════════╝");
    println!();

    // ━━━ 1. 의료 AI ━━━
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  🏥 의료 AI 판단 시스템");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut med_ai = MedicalAI::new();

    // 케이스 1: 안정 환자
    let patient1 = Patient {
        id: "P001".into(), name: "김환자".into(), age: 45, gender: "M".into(),
        symptoms: vec!["경미한 흉통".into(), "피로감".into()],
        vitals: Vitals { bp_systolic: 125, bp_diastolic: 80, heart_rate: 72, temperature: 36.5, spo2: 98, blood_sugar: 110 },
        history: vec!["고혈압 가족력".into()],
        allergies: vec![],
    };
    let d1 = med_ai.evaluate(&patient1, "관상동맥 조영술 시행 여부?");
    println!("\n  환자: {} ({}/{}세)", patient1.name, patient1.gender, patient1.age);
    println!("  증상: {:?}", patient1.symptoms);
    println!("  BP: {}/{} | HR: {} | SpO2: {}% | 체온: {}°C",
        patient1.vitals.bp_systolic, patient1.vitals.bp_diastolic,
        patient1.vitals.heart_rate, patient1.vitals.spo2, patient1.vitals.temperature);
    println!("  질문: {}", d1.question);
    for (name, trit, reason) in &d1.decision.ai_votes {
        println!("    {} → {} — {}", name, trit, reason);
    }
    println!("  ──────────────────────────");
    println!("  {}", d1.decision);
    if !d1.suggested_tests.is_empty() {
        println!("  추가 검사: {:?}", d1.suggested_tests);
    }

    // 케이스 2: 고위험 환자
    let patient2 = Patient {
        id: "P002".into(), name: "이위급".into(), age: 78, gender: "F".into(),
        symptoms: vec!["심한 흉통".into(), "호흡곤란".into(), "발한".into(), "구역질".into()],
        vitals: Vitals { bp_systolic: 165, bp_diastolic: 95, heart_rate: 112, temperature: 37.8, spo2: 91, blood_sugar: 245 },
        history: vec!["당뇨".into(), "심근경색 이력".into()],
        allergies: vec!["페니실린".into()],
    };
    let d2 = med_ai.evaluate(&patient2, "응급 수술 시행 여부?");
    println!("\n  환자: {} ({}/{}세)", patient2.name, patient2.gender, patient2.age);
    println!("  증상: {:?}", patient2.symptoms);
    println!("  BP: {}/{} | HR: {} | SpO2: {}% | 혈당: {}",
        patient2.vitals.bp_systolic, patient2.vitals.bp_diastolic,
        patient2.vitals.heart_rate, patient2.vitals.spo2, patient2.vitals.blood_sugar);
    println!("  질문: {}", d2.question);
    for (name, trit, reason) in &d2.decision.ai_votes {
        println!("    {} → {} — {}", name, trit, reason);
    }
    println!("  ──────────────────────────");
    println!("  {}", d2.decision);
    if !d2.contraindications.is_empty() {
        println!("  금기사항: {:?}", d2.contraindications);
    }

    // ━━━ 2. 교육 AI ━━━
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  📚 교육 AI 어시스턴트");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut edu_ai = EducationAI::new();

    let student1 = Student {
        id: "S001".into(), name: "박학생".into(), grade: "고2".into(),
        subjects: vec![
            SubjectScore { subject: "수학".into(), score: 92.0, trend: Trit::P },
            SubjectScore { subject: "영어".into(), score: 85.0, trend: Trit::P },
            SubjectScore { subject: "과학".into(), score: 88.0, trend: Trit::O },
            SubjectScore { subject: "국어".into(), score: 78.0, trend: Trit::T },
        ],
        learning_style: LearningStyle::Visual,
        attendance_rate: 0.95,
    };
    let e1 = edu_ai.evaluate(&student1, "심화 수학 올림피아드 과정 진행?");
    println!("\n  학생: {} ({})", student1.name, student1.grade);
    println!("  성적: {}", student1.subjects.iter()
        .map(|s| format!("{}:{:.0}({})", s.subject, s.score, s.trend.label()))
        .collect::<Vec<_>>().join(" | "));
    println!("  학습유형: {} | 출석: {:.0}%", student1.learning_style, student1.attendance_rate * 100.0);
    println!("  질문: {}", e1.decision.query);
    for (name, trit, reason) in &e1.decision.ai_votes {
        println!("    {} → {} — {}", name, trit, reason);
    }
    println!("  ──────────────────────────");
    println!("  {}", e1.decision);
    println!("  경로: {} | 주 {}시간", e1.recommended_path, e1.weekly_hours);
    println!("  방법: {:?}", e1.methods);

    let student2 = Student {
        id: "S002".into(), name: "최부진".into(), grade: "중3".into(),
        subjects: vec![
            SubjectScore { subject: "수학".into(), score: 38.0, trend: Trit::T },
            SubjectScore { subject: "영어".into(), score: 45.0, trend: Trit::T },
            SubjectScore { subject: "과학".into(), score: 52.0, trend: Trit::O },
        ],
        learning_style: LearningStyle::Kinesthetic,
        attendance_rate: 0.72,
    };
    let e2 = edu_ai.evaluate(&student2, "기초 보충 학습 계획?");
    println!("\n  학생: {} ({})", student2.name, student2.grade);
    println!("  성적: {}", student2.subjects.iter()
        .map(|s| format!("{}:{:.0}({})", s.subject, s.score, s.trend.label()))
        .collect::<Vec<_>>().join(" | "));
    println!("  질문: {}", e2.decision.query);
    for (name, trit, reason) in &e2.decision.ai_votes {
        println!("    {} → {} — {}", name, trit, reason);
    }
    println!("  ──────────────────────────");
    println!("  {}", e2.decision);
    println!("  집중 과목: {:?} | 주 {}시간", e2.focus_subjects, e2.weekly_hours);

    // ━━━ 3. 트레이딩 AI ━━━
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  📈 트레이딩 AI 시그널");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut trade_ai = TradingAI::new();

    let markets = vec![
        MarketData {
            symbol: "BTC/USDT".into(), price: 67250.0, change_24h: -6.2,
            volume_24h: 2_800_000_000.0, rsi: 28.0, macd: -120.0,
            bollinger_pos: 0.08, fear_greed: 22, support: 65000.0, resistance: 72000.0,
        },
        MarketData {
            symbol: "ETH/USDT".into(), price: 3820.0, change_24h: 3.5,
            volume_24h: 1_200_000_000.0, rsi: 62.0, macd: 15.0,
            bollinger_pos: 0.55, fear_greed: 58, support: 3500.0, resistance: 4200.0,
        },
        MarketData {
            symbol: "CRWN/USDT".into(), price: 0.124, change_24h: 12.5,
            volume_24h: 45_000_000.0, rsi: 78.0, macd: 0.008,
            bollinger_pos: 0.92, fear_greed: 82, support: 0.095, resistance: 0.130,
        },
    ];

    for market in &markets {
        let signal = trade_ai.analyze(market);
        println!("\n  {} — ${:.2} ({:+.1}%)", market.symbol, market.price, market.change_24h);
        println!("  RSI: {:.0} | MACD: {:.2} | BB: {:.2} | F&G: {}",
            market.rsi, market.macd, market.bollinger_pos, market.fear_greed);
        for (name, trit, reason) in &signal.decision.ai_votes {
            println!("    {} → {} — {}", name, trit, reason);
        }
        println!("  ──────────────────────────");
        println!("  {}", signal.decision);
        println!("  액션: {} | 진입: ${:.2} | SL: ${:.2} | TP: ${:.2} | 포지션: {:.0}%",
            signal.action, signal.entry_price, signal.stop_loss, signal.take_profit, signal.position_size_pct);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ 산업 적용 데모 완료");
    println!("  의료: {} 판단 | 교육: {} 계획 | 트레이딩: {} 시그널",
        med_ai.decisions.len(), edu_ai.plans.len(), trade_ai.signals.len());
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_consensus() {
        assert_eq!(Trit::consensus(&[Trit::P, Trit::P, Trit::T]), Trit::P);
        assert_eq!(Trit::consensus(&[Trit::T, Trit::T, Trit::P]), Trit::T);
        assert_eq!(Trit::consensus(&[Trit::P, Trit::O, Trit::T]), Trit::O);
    }

    #[test]
    fn test_trit_confidence() {
        assert!((Trit::confidence(&[Trit::P, Trit::P, Trit::P]) - 1.0).abs() < 0.01);
        assert!((Trit::confidence(&[Trit::P, Trit::P, Trit::T]) - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_vitals_risk_score() {
        let normal = Vitals { bp_systolic: 120, bp_diastolic: 80, heart_rate: 72, temperature: 36.5, spo2: 98, blood_sugar: 100 };
        assert!(normal.risk_score() < 0.1);

        let abnormal = Vitals { bp_systolic: 180, bp_diastolic: 100, heart_rate: 120, temperature: 39.0, spo2: 88, blood_sugar: 300 };
        assert!(abnormal.risk_score() > 0.5);
    }

    #[test]
    fn test_medical_ai_stable() {
        let mut ai = MedicalAI::new();
        let patient = Patient {
            id: "T1".into(), name: "정상".into(), age: 35, gender: "M".into(),
            symptoms: vec!["경미한 두통".into()],
            vitals: Vitals { bp_systolic: 120, bp_diastolic: 75, heart_rate: 68, temperature: 36.4, spo2: 99, blood_sugar: 95 },
            history: Vec::new(), allergies: Vec::new(),
        };
        let d = ai.evaluate(&patient, "퇴원 가능?");
        assert_eq!(d.decision.consensus, Trit::P);
    }

    #[test]
    fn test_medical_ai_critical() {
        let mut ai = MedicalAI::new();
        let patient = Patient {
            id: "T2".into(), name: "위급".into(), age: 80, gender: "F".into(),
            symptoms: vec!["흉통".into(), "호흡곤란".into(), "발한".into(), "구역".into()],
            vitals: Vitals { bp_systolic: 170, bp_diastolic: 100, heart_rate: 115, temperature: 38.5, spo2: 89, blood_sugar: 280 },
            history: vec!["심근경색".into()], allergies: vec!["아스피린".into()],
        };
        let d = ai.evaluate(&patient, "응급 수술?");
        assert!(d.decision.consensus == Trit::T || d.decision.consensus == Trit::O);
    }

    #[test]
    fn test_education_ai_good_student() {
        let mut ai = EducationAI::new();
        let student = Student {
            id: "S1".into(), name: "우수".into(), grade: "고1".into(),
            subjects: vec![
                SubjectScore { subject: "수학".into(), score: 95.0, trend: Trit::P },
                SubjectScore { subject: "영어".into(), score: 90.0, trend: Trit::P },
            ],
            learning_style: LearningStyle::Visual, attendance_rate: 0.98,
        };
        let plan = ai.evaluate(&student, "심화 과정?");
        assert_eq!(plan.decision.consensus, Trit::P);
    }

    #[test]
    fn test_education_ai_struggling() {
        let mut ai = EducationAI::new();
        let student = Student {
            id: "S2".into(), name: "부진".into(), grade: "중2".into(),
            subjects: vec![
                SubjectScore { subject: "수학".into(), score: 35.0, trend: Trit::T },
                SubjectScore { subject: "영어".into(), score: 40.0, trend: Trit::T },
            ],
            learning_style: LearningStyle::Kinesthetic, attendance_rate: 0.65,
        };
        let plan = ai.evaluate(&student, "기초 보충?");
        assert!(plan.decision.consensus == Trit::T || plan.decision.consensus == Trit::O);
        assert!(plan.weekly_hours >= 10);
    }

    #[test]
    fn test_trading_ai_oversold() {
        let mut ai = TradingAI::new();
        let market = MarketData {
            symbol: "TEST".into(), price: 100.0, change_24h: -8.0,
            volume_24h: 1e9, rsi: 22.0, macd: -5.0,
            bollinger_pos: 0.05, fear_greed: 15, support: 98.0, resistance: 120.0,
        };
        let signal = ai.analyze(&market);
        assert!(matches!(signal.action, TradeAction::Buy | TradeAction::StrongBuy));
    }

    #[test]
    fn test_trading_ai_overbought() {
        let mut ai = TradingAI::new();
        let market = MarketData {
            symbol: "TEST".into(), price: 100.0, change_24h: 8.0,
            volume_24h: 1e9, rsi: 82.0, macd: 5.0,
            bollinger_pos: 0.95, fear_greed: 85, support: 80.0, resistance: 101.0,
        };
        let signal = ai.analyze(&market);
        assert!(matches!(signal.action, TradeAction::Sell | TradeAction::StrongSell | TradeAction::Hold));
    }

    #[test]
    fn test_ctp_header() {
        let votes = vec![Trit::P, Trit::P, Trit::T];
        let h = build_ctp(&Trit::P, &votes);
        assert_eq!(h[0], 1);  // consensus P
        assert_eq!(h[2], 0);  // not unanimous
        assert_eq!(h[5], 1);  // vote 0: P
        assert_eq!(h[7], -1); // vote 2: T
    }
}
