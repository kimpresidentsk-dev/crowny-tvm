///! ═══════════════════════════════════════════════════
///! CPM — Crowny Package Manager v0.1
///! ═══════════════════════════════════════════════════
///!
///! 생태계 확장의 핵심.
///! 없으면 IDE, 디버거, 교육 모두 의미 없음.
///!
///! 기능:
///!   - 패키지 등록/조회/설치/삭제
///!   - 의존성 트리 해석
///!   - Trit 권한 검사 (P/O/T)
///!   - 버전 관리 (SemVer + Trit 상태)
///!   - import 구문 지원
///!
///! 구조:
///!   crowny.toml → 프로젝트 매니페스트
///!   .crowny/    → 로컬 패키지 캐시
///!   registry/   → 원격 저장소 (시뮬레이션)

use std::collections::HashMap;
use crate::car::TritState;

// ─────────────────────────────────────────────
// 버전
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 { return None; }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    /// 호환성 체크 (SemVer: major 동일 = 호환)
    pub fn compatible(&self, other: &Version) -> bool {
        self.major == other.major && (self.minor < other.minor ||
            (self.minor == other.minor && self.patch <= other.patch))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ─────────────────────────────────────────────
// 패키지 메타데이터
// ─────────────────────────────────────────────

/// 패키지 카테고리
#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    Core,       // 코어 라이브러리
    Ai,         // AI/LLM
    Web,        // 웹 서비스
    Data,       // 데이터/DB
    Crypto,     // 암호/보안
    Edu,        // 교육
    Medical,    // 의료
    Finance,    // 금융
    Iot,        // IoT/하드웨어
    Util,       // 유틸리티
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Core => write!(f, "코어"),
            Category::Ai => write!(f, "AI"),
            Category::Web => write!(f, "웹"),
            Category::Data => write!(f, "데이터"),
            Category::Crypto => write!(f, "암호"),
            Category::Edu => write!(f, "교육"),
            Category::Medical => write!(f, "의료"),
            Category::Finance => write!(f, "금융"),
            Category::Iot => write!(f, "IoT"),
            Category::Util => write!(f, "유틸"),
        }
    }
}

/// Trit 권한 등급
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TritTrust {
    Trusted,    // P: 검증 완료
    Review,     // O: 검토 필요
    Untrusted,  // T: 비신뢰
}

impl std::fmt::Display for TritTrust {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TritTrust::Trusted => write!(f, "P(신뢰)"),
            TritTrust::Review => write!(f, "O(검토)"),
            TritTrust::Untrusted => write!(f, "T(비신뢰)"),
        }
    }
}

/// 패키지 정보
#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: Version,
    pub author: String,
    pub description: String,
    pub category: Category,
    pub trust: TritTrust,
    pub dependencies: Vec<Dependency>,
    pub exports: Vec<String>,    // 공개 함수/모듈 목록
    pub source_size: usize,      // 소스 크기 (bytes)
    pub tvm_opcodes: Vec<u8>,    // 사용하는 섹터 목록
}

/// 의존성
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version_req: String,  // ">=0.1.0", "^1.0", "=2.3.1"
}

impl Dependency {
    pub fn new(name: &str, ver: &str) -> Self {
        Self { name: name.to_string(), version_req: ver.to_string() }
    }
}

// ─────────────────────────────────────────────
// 프로젝트 매니페스트 (crowny.toml)
// ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    pub author: String,
    pub description: String,
    pub entry: String,           // 진입점 파일
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub trit_policy: TritTrust,  // 최소 신뢰 등급
}

impl Manifest {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: Version::new(0, 1, 0),
            author: String::new(),
            description: String::new(),
            entry: "main.cws".to_string(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            trit_policy: TritTrust::Review,
        }
    }

    pub fn add_dep(&mut self, name: &str, ver: &str) {
        self.dependencies.push(Dependency::new(name, ver));
    }

    /// crowny.toml 형식 생성
    pub fn to_toml(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("[package]\nname = \"{}\"\n", self.name));
        out.push_str(&format!("version = \"{}\"\n", self.version));
        out.push_str(&format!("author = \"{}\"\n", self.author));
        out.push_str(&format!("description = \"{}\"\n", self.description));
        out.push_str(&format!("entry = \"{}\"\n", self.entry));
        out.push_str(&format!("trit_policy = \"{}\"\n\n", self.trit_policy));

        if !self.dependencies.is_empty() {
            out.push_str("[dependencies]\n");
            for dep in &self.dependencies {
                out.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
            }
        }
        out
    }
}

// ─────────────────────────────────────────────
// CPM — 패키지 매니저
// ─────────────────────────────────────────────

/// 설치 결과
#[derive(Debug)]
pub struct InstallResult {
    pub state: TritState,
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

pub struct CrownyPM {
    // 로컬 레지스트리 (시뮬레이션)
    registry: HashMap<String, Vec<Package>>,
    // 설치된 패키지
    installed: HashMap<String, Package>,
    // 설치 이력
    history: Vec<(String, Version, TritState)>,
}

impl CrownyPM {
    pub fn new() -> Self {
        let mut cpm = Self {
            registry: HashMap::new(),
            installed: HashMap::new(),
            history: Vec::new(),
        };
        cpm.seed_registry();
        cpm
    }

    /// 내장 패키지 등록
    fn seed_registry(&mut self) {
        // 코어 패키지들
        self.register(Package {
            name: "crowny.core".into(),
            version: Version::new(0, 3, 0),
            author: "KPS".into(),
            description: "Crowny 코어 라이브러리 — Trit 타입, 상태 관리".into(),
            category: Category::Core,
            trust: TritTrust::Trusted,
            dependencies: vec![],
            exports: vec!["Trit".into(), "TritState".into(), "TritResult".into()],
            source_size: 2048,
            tvm_opcodes: vec![0],
        });

        self.register(Package {
            name: "crowny.ai".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "AI/LLM 통합 — 다중 모델 합의, 추론, NLP".into(),
            category: Category::Ai,
            trust: TritTrust::Trusted,
            dependencies: vec![Dependency::new("crowny.core", ">=0.3.0")],
            exports: vec!["LlmCall".into(), "Consensus".into(), "Sentiment".into()],
            source_size: 4096,
            tvm_opcodes: vec![0, 1],
        });

        self.register(Package {
            name: "crowny.web".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "웹서버 프레임워크 — CTP 라우터, HTTP".into(),
            category: Category::Web,
            trust: TritTrust::Trusted,
            dependencies: vec![Dependency::new("crowny.core", ">=0.3.0")],
            exports: vec!["Server".into(), "Router".into(), "CtpHeader".into()],
            source_size: 3072,
            tvm_opcodes: vec![0, 6],
        });

        self.register(Package {
            name: "crowny.crypto".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "암호/해시/서명 — 3진 보안 체계".into(),
            category: Category::Crypto,
            trust: TritTrust::Trusted,
            dependencies: vec![Dependency::new("crowny.core", ">=0.3.0")],
            exports: vec!["Hash".into(), "Encrypt".into(), "Sign".into(), "Token".into()],
            source_size: 2560,
            tvm_opcodes: vec![0, 5, 6],
        });

        self.register(Package {
            name: "crowny.data".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "데이터 영속화 — Trit 상태 저장, 캐시, JSON".into(),
            category: Category::Data,
            trust: TritTrust::Trusted,
            dependencies: vec![Dependency::new("crowny.core", ">=0.3.0")],
            exports: vec!["Store".into(), "Cache".into(), "Json".into()],
            source_size: 3584,
            tvm_opcodes: vec![0, 3, 4],
        });

        self.register(Package {
            name: "crowny.edu".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "교육 서비스 — 3진 사고 훈련, 한선어 튜토리얼".into(),
            category: Category::Edu,
            trust: TritTrust::Trusted,
            dependencies: vec![
                Dependency::new("crowny.core", ">=0.3.0"),
                Dependency::new("crowny.ai", ">=0.1.0"),
            ],
            exports: vec!["Lesson".into(), "Quiz".into(), "TritTrainer".into()],
            source_size: 5120,
            tvm_opcodes: vec![0, 1],
        });

        self.register(Package {
            name: "crowny.medical".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "의료 시스템 — 3진 판단, 바이오 데이터, 장비 인터페이스".into(),
            category: Category::Medical,
            trust: TritTrust::Review,
            dependencies: vec![
                Dependency::new("crowny.core", ">=0.3.0"),
                Dependency::new("crowny.ai", ">=0.1.0"),
                Dependency::new("crowny.crypto", ">=0.1.0"),
            ],
            exports: vec!["Diagnosis".into(), "BioData".into(), "DeviceCtl".into()],
            source_size: 6144,
            tvm_opcodes: vec![0, 1, 2, 6],
        });

        self.register(Package {
            name: "crowny.finance".into(),
            version: Version::new(0, 1, 0),
            author: "KPS".into(),
            description: "금융/거래 — 3진 토큰, 트레이딩 엔진, 거래소".into(),
            category: Category::Finance,
            trust: TritTrust::Review,
            dependencies: vec![
                Dependency::new("crowny.core", ">=0.3.0"),
                Dependency::new("crowny.crypto", ">=0.1.0"),
                Dependency::new("crowny.data", ">=0.1.0"),
            ],
            exports: vec!["TritToken".into(), "Exchange".into(), "Trade".into()],
            source_size: 7168,
            tvm_opcodes: vec![0, 3, 5, 6],
        });
    }

    fn register(&mut self, pkg: Package) {
        self.registry
            .entry(pkg.name.clone())
            .or_insert_with(Vec::new)
            .push(pkg);
    }

    /// 패키지 검색
    pub fn search(&self, query: &str) -> Vec<&Package> {
        self.registry.values()
            .flat_map(|versions| versions.iter())
            .filter(|p| p.name.contains(query) || p.description.contains(query))
            .collect()
    }

    /// 패키지 정보 조회
    pub fn info(&self, name: &str) -> Option<&Package> {
        self.registry.get(name)
            .and_then(|versions| versions.last())
    }

    /// 패키지 설치
    pub fn install(&mut self, name: &str) -> InstallResult {
        let mut result = InstallResult {
            state: TritState::Pending,
            installed: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
        };

        // 이미 설치 확인
        if self.installed.contains_key(name) {
            result.skipped.push(name.to_string());
            result.state = TritState::Success;
            return result;
        }

        // 레지스트리 조회
        let pkg = match self.registry.get(name).and_then(|v| v.last()).cloned() {
            Some(p) => p,
            None => {
                result.failed.push(format!("{} — 레지스트리에 없음", name));
                result.state = TritState::Failed;
                return result;
            }
        };

        // Trit 권한 검사
        if pkg.trust == TritTrust::Untrusted {
            result.failed.push(format!("{} — 비신뢰(T) 패키지", name));
            result.state = TritState::Failed;
            return result;
        }

        // 의존성 재귀 설치
        for dep in &pkg.dependencies {
            if !self.installed.contains_key(&dep.name) {
                let dep_result = self.install(&dep.name);
                result.installed.extend(dep_result.installed);
                result.failed.extend(dep_result.failed);
            }
        }

        // 설치
        self.installed.insert(name.to_string(), pkg.clone());
        self.history.push((name.to_string(), pkg.version.clone(), TritState::Success));
        result.installed.push(format!("{} v{}", name, pkg.version));
        result.state = TritState::Success;
        result
    }

    /// 패키지 제거
    pub fn uninstall(&mut self, name: &str) -> TritState {
        if self.installed.remove(name).is_some() {
            self.history.push((name.to_string(), Version::new(0,0,0), TritState::Failed));
            TritState::Success
        } else {
            TritState::Failed
        }
    }

    /// 의존성 트리 표시
    pub fn dep_tree(&self, name: &str, depth: usize) -> String {
        let mut out = String::new();
        let indent = "  ".repeat(depth);
        if let Some(pkg) = self.registry.get(name).and_then(|v| v.last()) {
            let marker = if self.installed.contains_key(name) { "✓" } else { "○" };
            out.push_str(&format!("{}{}  {} v{} [{}]\n",
                indent, marker, pkg.name, pkg.version, pkg.trust));
            for dep in &pkg.dependencies {
                out.push_str(&self.dep_tree(&dep.name, depth + 1));
            }
        } else {
            out.push_str(&format!("{}✗  {} (없음)\n", indent, name));
        }
        out
    }

    /// 설치 목록
    pub fn list_installed(&self) -> Vec<&Package> {
        self.installed.values().collect()
    }

    /// import 해석: "crowny.ai" → 해당 패키지 exports 반환
    pub fn resolve_import(&self, import_path: &str) -> Option<Vec<String>> {
        self.installed.get(import_path).map(|p| p.exports.clone())
    }

    /// 레지스트리 통계
    pub fn stats(&self) -> (usize, usize, usize) {
        let total = self.registry.values().map(|v| v.len()).sum();
        let installed = self.installed.len();
        let categories = self.registry.values()
            .flat_map(|v| v.iter().map(|p| std::mem::discriminant(&p.category)))
            .collect::<std::collections::HashSet<_>>()
            .len();
        (total, installed, categories)
    }
}

// ─────────────────────────────────────────────
// import 구문 파서
// ─────────────────────────────────────────────

/// import 문 파싱
/// "가져와 crowny.ai { LlmCall, Consensus }"
/// "import crowny.web { Server }"
pub fn parse_import(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    let (_, rest) = if line.starts_with("가져와") {
        ("가져와", line.strip_prefix("가져와")?.trim())
    } else if line.starts_with("import") {
        ("import", line.strip_prefix("import")?.trim())
    } else {
        return None;
    };

    // "crowny.ai { LlmCall, Consensus }"
    if let Some(brace_start) = rest.find('{') {
        let pkg_name = rest[..brace_start].trim().to_string();
        let brace_end = rest.find('}')?;
        let items: Vec<String> = rest[brace_start+1..brace_end]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some((pkg_name, items))
    } else {
        // "crowny.ai" (전체 임포트)
        Some((rest.to_string(), vec!["*".to_string()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_registry() {
        let cpm = CrownyPM::new();
        let (total, _, _) = cpm.stats();
        assert!(total >= 8, "최소 8개 패키지 필요");
    }

    #[test]
    fn test_search() {
        let cpm = CrownyPM::new();
        let results = cpm.search("AI");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_install() {
        let mut cpm = CrownyPM::new();
        let result = cpm.install("crowny.ai");
        assert_eq!(result.state, TritState::Success);
        // crowny.core도 의존성으로 설치됨
        assert!(cpm.installed.contains_key("crowny.core"));
        assert!(cpm.installed.contains_key("crowny.ai"));
    }

    #[test]
    fn test_install_deep_deps() {
        let mut cpm = CrownyPM::new();
        let result = cpm.install("crowny.medical");
        assert_eq!(result.state, TritState::Success);
        assert!(cpm.installed.len() >= 3); // core + ai + crypto + medical
    }

    #[test]
    fn test_dep_tree() {
        let cpm = CrownyPM::new();
        let tree = cpm.dep_tree("crowny.medical", 0);
        assert!(tree.contains("crowny.core"));
        assert!(tree.contains("crowny.ai"));
    }

    #[test]
    fn test_import_parse() {
        let (pkg, items) = parse_import("가져와 crowny.ai { LlmCall, Consensus }").unwrap();
        assert_eq!(pkg, "crowny.ai");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_manifest() {
        let mut m = Manifest::new("my-app");
        m.add_dep("crowny.core", ">=0.3.0");
        let toml = m.to_toml();
        assert!(toml.contains("my-app"));
        assert!(toml.contains("crowny.core"));
    }
}
