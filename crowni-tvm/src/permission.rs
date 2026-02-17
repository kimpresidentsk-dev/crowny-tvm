///! ═══════════════════════════════════════════════════
///! Trit Permission Engine — 균형3진 권한 엔진
///! ═══════════════════════════════════════════════════
///!
///! 모든 요청은 3단 판정:
///!   P(+1) = 허용 (Allow)
///!   O( 0) = 검토 (Review/Pending)
///!   T(-1) = 차단 (Deny)
///!
///! 정책(Policy) 기반: 주체(Subject) × 대상(Object) × 행위(Action)
///! OOP 접근제어도 3진: public(P) / protected(O) / private(T)
///!
///! 2진의 allow/deny를 절대 노출하지 않는다.
///! 모든 판정은 3진 논리로 이루어진다.

use std::collections::HashMap;

// ─────────────────────────────────────────────
// 3진 권한 타입
// ─────────────────────────────────────────────

/// 권한 판정 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum TritPermission {
    Allow  =  1,  // P = 허용
    Review =  0,  // O = 검토 (보류/승인 필요)
    Deny   = -1,  // T = 차단
}

impl TritPermission {
    pub fn symbol(&self) -> char {
        match self {
            TritPermission::Allow => 'P',
            TritPermission::Review => 'O',
            TritPermission::Deny => 'T',
        }
    }

    pub fn name_kr(&self) -> &'static str {
        match self {
            TritPermission::Allow => "허용",
            TritPermission::Review => "검토",
            TritPermission::Deny => "차단",
        }
    }

    /// 3진 AND: 두 권한 중 더 제한적인 쪽 (min)
    /// P AND O = O, P AND T = T, O AND T = T
    pub fn and(self, other: TritPermission) -> TritPermission {
        let v = (self as i8).min(other as i8);
        match v {
            1 => TritPermission::Allow,
            0 => TritPermission::Review,
            _ => TritPermission::Deny,
        }
    }

    /// 3진 OR: 두 권한 중 더 관대한 쪽 (max)
    pub fn or(self, other: TritPermission) -> TritPermission {
        let v = (self as i8).max(other as i8);
        match v {
            1 => TritPermission::Allow,
            0 => TritPermission::Review,
            _ => TritPermission::Deny,
        }
    }

    /// 반전
    pub fn not(self) -> TritPermission {
        match self {
            TritPermission::Allow => TritPermission::Deny,
            TritPermission::Review => TritPermission::Review,
            TritPermission::Deny => TritPermission::Allow,
        }
    }
}

impl std::fmt::Display for TritPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.symbol(), self.name_kr())
    }
}

// ─────────────────────────────────────────────
// 접근 수준 (OOP 3진 확장)
// ─────────────────────────────────────────────

/// OOP 접근 수준 — 3진
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum AccessLevel {
    Public    =  1,  // P(+1) = 공개
    Protected =  0,  // O( 0) = 보호 (같은 계층만)
    Private   = -1,  // T(-1) = 비공개
}

impl AccessLevel {
    pub fn to_permission(self) -> TritPermission {
        match self {
            AccessLevel::Public => TritPermission::Allow,
            AccessLevel::Protected => TritPermission::Review,
            AccessLevel::Private => TritPermission::Deny,
        }
    }
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessLevel::Public => write!(f, "공개(P)"),
            AccessLevel::Protected => write!(f, "보호(O)"),
            AccessLevel::Private => write!(f, "비공개(T)"),
        }
    }
}

// ─────────────────────────────────────────────
// 정책 (Policy)
// ─────────────────────────────────────────────

/// 행위 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Read,       // 읽기
    Write,      // 쓰기
    Execute,    // 실행
    Delete,     // 삭제
    Admin,      // 관리
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Read => write!(f, "읽기"),
            Action::Write => write!(f, "쓰기"),
            Action::Execute => write!(f, "실행"),
            Action::Delete => write!(f, "삭제"),
            Action::Admin => write!(f, "관리"),
        }
    }
}

/// 정책 규칙
#[derive(Debug, Clone)]
pub struct PolicyRule {
    pub subject: String,    // 주체 (누가)
    pub object: String,     // 대상 (무엇을)
    pub action: Action,     // 행위 (어떻게)
    pub permission: TritPermission,  // 판정
    pub reason: String,     // 사유
}

// ─────────────────────────────────────────────
// Permission Engine
// ─────────────────────────────────────────────

/// 균형3진 권한 엔진
pub struct PermissionEngine {
    /// 정책 규칙 목록
    policies: Vec<PolicyRule>,
    /// 기본 권한 (정책 없을 때)
    pub default_permission: TritPermission,
    /// 감사 로그
    audit_log: Vec<AuditEntry>,
    /// 판정 통계
    pub stats_allow: u64,
    pub stats_review: u64,
    pub stats_deny: u64,
}

/// 감사 로그 엔트리
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub subject: String,
    pub object: String,
    pub action: Action,
    pub result: TritPermission,
    pub matched_rule: Option<usize>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            default_permission: TritPermission::Review, // 기본: 검토(O)
            audit_log: Vec::new(),
            stats_allow: 0,
            stats_review: 0,
            stats_deny: 0,
        }
    }

    /// 정책 추가
    pub fn add_policy(&mut self, subject: &str, object: &str, action: Action,
                      permission: TritPermission, reason: &str) {
        self.policies.push(PolicyRule {
            subject: subject.to_string(),
            object: object.to_string(),
            action,
            permission,
            reason: reason.to_string(),
        });
    }

    /// 권한 확인 — 핵심 함수
    /// 정책을 순서대로 검색, 첫 매칭 규칙의 판정 반환
    /// 매칭 없으면 default_permission
    pub fn check(&mut self, subject: &str, object: &str, action: Action) -> TritPermission {
        let mut matched_idx = None;
        let mut result = self.default_permission;

        for (i, rule) in self.policies.iter().enumerate() {
            // 와일드카드(*) 지원
            let sub_match = rule.subject == "*" || rule.subject == subject;
            let obj_match = rule.object == "*" || rule.object == object;
            let act_match = rule.action == action;

            if sub_match && obj_match && act_match {
                result = rule.permission;
                matched_idx = Some(i);
                break;
            }
        }

        // 통계 업데이트
        match result {
            TritPermission::Allow => self.stats_allow += 1,
            TritPermission::Review => self.stats_review += 1,
            TritPermission::Deny => self.stats_deny += 1,
        }

        // 감사 로그
        self.audit_log.push(AuditEntry {
            subject: subject.to_string(),
            object: object.to_string(),
            action,
            result,
            matched_rule: matched_idx,
        });

        result
    }

    /// 다중 권한 체크 (모든 조건 AND)
    pub fn check_all(&mut self, subject: &str, checks: &[(&str, Action)]) -> TritPermission {
        let mut combined = TritPermission::Allow;
        for (object, action) in checks {
            let r = self.check(subject, object, *action);
            combined = combined.and(r);
            // T(차단)이면 즉시 반환 (단락 평가)
            if combined == TritPermission::Deny {
                return TritPermission::Deny;
            }
        }
        combined
    }

    /// 감사 로그 조회
    pub fn audit_count(&self) -> usize {
        self.audit_log.len()
    }

    /// 상태 덤프
    pub fn dump(&self) {
        println!("╔══ 권한 엔진 상태 ══════════════════════════╗");
        println!("║ 정책: {} 개 | 기본: {}", self.policies.len(), self.default_permission);
        println!("║ 통계: 허용:{} 검토:{} 차단:{}",
            self.stats_allow, self.stats_review, self.stats_deny);
        println!("║ ── 정책 목록 ──");
        for (i, p) in self.policies.iter().enumerate() {
            println!("║   [{}] {}→{}.{} = {} ({})",
                i, p.subject, p.object, p.action, p.permission, p.reason);
        }
        if !self.audit_log.is_empty() {
            println!("║ ── 최근 감사 로그 (최대 5) ──");
            for entry in self.audit_log.iter().rev().take(5) {
                let rule_info = entry.matched_rule
                    .map(|i| format!("규칙#{}", i))
                    .unwrap_or_else(|| "기본".to_string());
                println!("║   {}→{}.{} = {} [{}]",
                    entry.subject, entry.object, entry.action, entry.result, rule_info);
            }
        }
        println!("╚═══════════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_basic() {
        let mut engine = PermissionEngine::new();

        engine.add_policy("관리자", "*", Action::Admin, TritPermission::Allow, "관리자 전권");
        engine.add_policy("사용자", "파일", Action::Read, TritPermission::Allow, "파일 읽기 허용");
        engine.add_policy("사용자", "파일", Action::Delete, TritPermission::Deny, "파일 삭제 금지");
        engine.add_policy("*", "시스템", Action::Write, TritPermission::Deny, "시스템 쓰기 금지");

        assert_eq!(engine.check("관리자", "모든것", Action::Admin), TritPermission::Allow);
        assert_eq!(engine.check("사용자", "파일", Action::Read), TritPermission::Allow);
        assert_eq!(engine.check("사용자", "파일", Action::Delete), TritPermission::Deny);
        assert_eq!(engine.check("누구든", "시스템", Action::Write), TritPermission::Deny);
        // 정책 없음 → 기본(검토)
        assert_eq!(engine.check("손님", "비밀", Action::Read), TritPermission::Review);
    }

    #[test]
    fn test_permission_and() {
        assert_eq!(TritPermission::Allow.and(TritPermission::Allow), TritPermission::Allow);
        assert_eq!(TritPermission::Allow.and(TritPermission::Review), TritPermission::Review);
        assert_eq!(TritPermission::Allow.and(TritPermission::Deny), TritPermission::Deny);
        assert_eq!(TritPermission::Review.and(TritPermission::Deny), TritPermission::Deny);
    }

    #[test]
    fn test_access_level() {
        assert_eq!(AccessLevel::Public.to_permission(), TritPermission::Allow);
        assert_eq!(AccessLevel::Protected.to_permission(), TritPermission::Review);
        assert_eq!(AccessLevel::Private.to_permission(), TritPermission::Deny);
    }
}
