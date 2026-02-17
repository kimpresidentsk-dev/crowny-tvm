// ═══════════════════════════════════════════════════════════════
// Crowny Platform — 통합 클라우드 플랫폼
// Git(GitHub) + Deploy(Vercel) + DB(Firebase) + Run(Railway) + Web3(Thirdweb)
// 모든 API는 3진 CTP 헤더 기반
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
fn short_hash() -> String { format!("{:07x}", now_ms() % 0xFFFFFFF) }

// ═══════════════════════════════════════
// 공통: CTP 응답
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CTPResponse {
    pub trit: i8,
    pub ctp: String,
    pub message: String,
    pub data: Option<String>,
}

impl CTPResponse {
    pub fn ok(msg: &str, data: Option<String>) -> Self {
        Self { trit: 1, ctp: "PPPPOOOOO".into(), message: msg.into(), data }
    }
    pub fn pending(msg: &str) -> Self {
        Self { trit: 0, ctp: "OOPOOOOOO".into(), message: msg.into(), data: None }
    }
    pub fn fail(msg: &str) -> Self {
        Self { trit: -1, ctp: "TTTOOOOOO".into(), message: msg.into(), data: None }
    }
    pub fn label(&self) -> &str { match self.trit { 1 => "P", -1 => "T", _ => "O" } }
}

impl std::fmt::Display for CTPResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} — CTP:{}", self.label(), self.message, self.ctp)
    }
}

// ═══════════════════════════════════════
// 1. Git 호스팅 (GitHub 기능)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub name: String,
    pub owner: String,
    pub description: String,
    pub is_private: bool,
    pub default_branch: String,
    pub commits: Vec<GitCommit>,
    pub branches: Vec<String>,
    pub issues: Vec<GitIssue>,
    pub pull_requests: Vec<PullRequest>,
    pub stars: u32,
    pub forks: u32,
    pub created_at: u64,
    pub lang: String,
}

#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct GitIssue {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub state: String, // open, closed
    pub labels: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub from_branch: String,
    pub to_branch: String,
    pub state: String, // open, merged, closed
    pub trit_review: i8, // P: approved, O: pending, T: rejected
}

pub struct GitService {
    pub repos: HashMap<String, GitRepo>,
}

impl GitService {
    pub fn new() -> Self { Self { repos: HashMap::new() } }

    pub fn create_repo(&mut self, owner: &str, name: &str, desc: &str, lang: &str) -> CTPResponse {
        let key = format!("{}/{}", owner, name);
        if self.repos.contains_key(&key) {
            return CTPResponse::fail(&format!("리포 이미 존재: {}", key));
        }
        self.repos.insert(key.clone(), GitRepo {
            name: name.into(), owner: owner.into(), description: desc.into(),
            is_private: false, default_branch: "main".into(),
            commits: Vec::new(), branches: vec!["main".into()],
            issues: Vec::new(), pull_requests: Vec::new(),
            stars: 0, forks: 0, created_at: now_ms(), lang: lang.into(),
        });
        CTPResponse::ok(&format!("리포 생성: {}", key), Some(key))
    }

    pub fn commit(&mut self, repo_key: &str, author: &str, message: &str, files: u32, ins: u32, del: u32) -> CTPResponse {
        let repo = match self.repos.get_mut(repo_key) {
            Some(r) => r, None => return CTPResponse::fail("리포 없음"),
        };
        let hash = short_hash();
        repo.commits.push(GitCommit {
            hash: hash.clone(), message: message.into(), author: author.into(),
            files_changed: files, insertions: ins, deletions: del, timestamp: now_ms(),
        });
        CTPResponse::ok(&format!("[{}] {}", &hash[..7], message), Some(hash))
    }

    pub fn create_pr(&mut self, repo_key: &str, author: &str, title: &str, from: &str, to: &str) -> CTPResponse {
        let repo = match self.repos.get_mut(repo_key) {
            Some(r) => r, None => return CTPResponse::fail("리포 없음"),
        };
        let id = repo.pull_requests.len() as u32 + 1;
        repo.pull_requests.push(PullRequest {
            id, title: title.into(), author: author.into(),
            from_branch: from.into(), to_branch: to.into(),
            state: "open".into(), trit_review: 0,
        });
        CTPResponse::ok(&format!("PR #{} 생성: {}", id, title), None)
    }

    pub fn review_pr(&mut self, repo_key: &str, pr_id: u32, trit: i8) -> CTPResponse {
        let repo = match self.repos.get_mut(repo_key) {
            Some(r) => r, None => return CTPResponse::fail("리포 없음"),
        };
        let pr = match repo.pull_requests.iter_mut().find(|p| p.id == pr_id) {
            Some(p) => p, None => return CTPResponse::fail("PR 없음"),
        };
        pr.trit_review = trit;
        if trit > 0 { pr.state = "merged".into(); }
        else if trit < 0 { pr.state = "closed".into(); }
        let label = match trit { 1 => "승인+머지", -1 => "거부", _ => "보류" };
        CTPResponse::ok(&format!("PR #{} → {}", pr_id, label), None)
    }
}

// ═══════════════════════════════════════
// 2. 배포 서비스 (Vercel 기능)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Deployment {
    pub id: String,
    pub project: String,
    pub url: String,
    pub status: DeployStatus,
    pub framework: String,
    pub build_time_ms: u64,
    pub domain: String,
    pub env_vars: HashMap<String, String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeployStatus { Building, Ready, Error, Stopped }

impl std::fmt::Display for DeployStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Building => write!(f, "⏳ Building"),
            Self::Ready => write!(f, "✓ Ready"),
            Self::Error => write!(f, "✗ Error"),
            Self::Stopped => write!(f, "■ Stopped"),
        }
    }
}

pub struct DeployService {
    pub deployments: Vec<Deployment>,
    pub domains: HashMap<String, String>, // domain → deployment_id
}

impl DeployService {
    pub fn new() -> Self { Self { deployments: Vec::new(), domains: HashMap::new() } }

    pub fn deploy(&mut self, project: &str, framework: &str, domain: &str) -> CTPResponse {
        let id = format!("dep-{}", short_hash());
        let url = format!("https://{}.crowny.app", project);
        let build_time = 1200 + (now_ms() % 3000);

        self.deployments.push(Deployment {
            id: id.clone(), project: project.into(), url: url.clone(),
            status: DeployStatus::Ready, framework: framework.into(),
            build_time_ms: build_time, domain: domain.into(),
            env_vars: HashMap::new(), created_at: now_ms(),
        });
        self.domains.insert(domain.into(), id.clone());
        CTPResponse::ok(&format!("배포 완료: {} → {} ({}ms)", project, url, build_time), Some(url))
    }

    pub fn set_env(&mut self, dep_id: &str, key: &str, value: &str) -> CTPResponse {
        if let Some(dep) = self.deployments.iter_mut().find(|d| d.id == dep_id) {
            dep.env_vars.insert(key.into(), value.into());
            CTPResponse::ok(&format!("환경변수 설정: {}={}", key, "***"), None)
        } else {
            CTPResponse::fail("배포 없음")
        }
    }

    pub fn rollback(&mut self, project: &str) -> CTPResponse {
        let deploys: Vec<_> = self.deployments.iter()
            .filter(|d| d.project == project && d.status == DeployStatus::Ready)
            .collect();
        if deploys.len() >= 2 {
            CTPResponse::ok(&format!("{} 롤백 완료", project), None)
        } else {
            CTPResponse::fail("이전 배포 없음")
        }
    }
}

// ═══════════════════════════════════════
// 3. 데이터베이스 (Firebase 기능)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct TritDocument {
    pub id: String,
    pub data: HashMap<String, String>,
    pub trit_state: i8,
    pub created_at: u64,
    pub updated_at: u64,
}

pub struct TritDB {
    pub collections: HashMap<String, Vec<TritDocument>>,
    pub rules: HashMap<String, i8>, // collection → trit permission
}

impl TritDB {
    pub fn new() -> Self {
        Self { collections: HashMap::new(), rules: HashMap::new() }
    }

    pub fn create_collection(&mut self, name: &str) -> CTPResponse {
        self.collections.insert(name.into(), Vec::new());
        self.rules.insert(name.into(), 1); // P: 읽기/쓰기 허용
        CTPResponse::ok(&format!("컬렉션 생성: {}", name), None)
    }

    pub fn insert(&mut self, collection: &str, data: HashMap<String, String>) -> CTPResponse {
        let perm = self.rules.get(collection).copied().unwrap_or(0);
        if perm < 0 { return CTPResponse::fail("쓰기 권한 없음"); }

        let docs = match self.collections.get_mut(collection) {
            Some(d) => d, None => return CTPResponse::fail("컬렉션 없음"),
        };
        let id = format!("doc-{}", short_hash());
        docs.push(TritDocument {
            id: id.clone(), data, trit_state: 1, created_at: now_ms(), updated_at: now_ms(),
        });
        CTPResponse::ok(&format!("{}/{} 저장", collection, id), Some(id))
    }

    pub fn query(&self, collection: &str, field: &str, value: &str) -> Vec<&TritDocument> {
        match self.collections.get(collection) {
            Some(docs) => docs.iter().filter(|d| d.data.get(field).map(|v| v == value).unwrap_or(false)).collect(),
            None => Vec::new(),
        }
    }

    pub fn delete(&mut self, collection: &str, doc_id: &str) -> CTPResponse {
        let docs = match self.collections.get_mut(collection) {
            Some(d) => d, None => return CTPResponse::fail("컬렉션 없음"),
        };
        let before = docs.len();
        docs.retain(|d| d.id != doc_id);
        if docs.len() < before {
            CTPResponse::ok(&format!("{}/{} 삭제", collection, doc_id), None)
        } else {
            CTPResponse::fail("문서 없음")
        }
    }

    pub fn stats(&self) -> (usize, usize) {
        let cols = self.collections.len();
        let docs: usize = self.collections.values().map(|v| v.len()).sum();
        (cols, docs)
    }
}

// ═══════════════════════════════════════
// 4. 앱 런타임 (Railway 기능)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct AppInstance {
    pub id: String,
    pub name: String,
    pub runtime: String,   // rust, node, python, go, hanseon
    pub port: u16,
    pub status: AppStatus,
    pub cpu_usage: f64,
    pub memory_mb: u64,
    pub replicas: u32,
    pub env: HashMap<String, String>,
    pub logs: Vec<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppStatus { Running, Stopped, Crashed, Deploying }

impl std::fmt::Display for AppStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "● Running"),
            Self::Stopped => write!(f, "○ Stopped"),
            Self::Crashed => write!(f, "✗ Crashed"),
            Self::Deploying => write!(f, "⏳ Deploying"),
        }
    }
}

pub struct AppRuntime {
    pub instances: Vec<AppInstance>,
    pub port_counter: u16,
}

impl AppRuntime {
    pub fn new() -> Self { Self { instances: Vec::new(), port_counter: 8000 } }

    pub fn deploy_app(&mut self, name: &str, runtime: &str, replicas: u32) -> CTPResponse {
        let port = self.port_counter;
        self.port_counter += 1;
        let id = format!("app-{}", short_hash());

        self.instances.push(AppInstance {
            id: id.clone(), name: name.into(), runtime: runtime.into(),
            port, status: AppStatus::Running,
            cpu_usage: 0.12, memory_mb: 128, replicas,
            env: HashMap::new(),
            logs: vec![format!("[{}] App started on :{}", name, port)],
            created_at: now_ms(),
        });
        CTPResponse::ok(&format!("{} 실행 — :{}  ({}x{})", name, port, runtime, replicas),
            Some(format!("http://localhost:{}", port)))
    }

    pub fn scale(&mut self, app_id: &str, replicas: u32) -> CTPResponse {
        if let Some(app) = self.instances.iter_mut().find(|a| a.id == app_id) {
            let old = app.replicas;
            app.replicas = replicas;
            app.logs.push(format!("Scaled {}→{} replicas", old, replicas));
            CTPResponse::ok(&format!("{} 스케일: {}→{}", app.name, old, replicas), None)
        } else {
            CTPResponse::fail("앱 없음")
        }
    }

    pub fn stop(&mut self, app_id: &str) -> CTPResponse {
        if let Some(app) = self.instances.iter_mut().find(|a| a.id == app_id) {
            app.status = AppStatus::Stopped;
            app.logs.push("App stopped".into());
            CTPResponse::ok(&format!("{} 중지", app.name), None)
        } else {
            CTPResponse::fail("앱 없음")
        }
    }

    pub fn logs(&self, app_id: &str) -> Vec<String> {
        self.instances.iter().find(|a| a.id == app_id)
            .map(|a| a.logs.clone()).unwrap_or_default()
    }
}

// ═══════════════════════════════════════
// 5. Web3 토큰 서비스 (Thirdweb 기능)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SmartContract {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub contract_type: ContractType,
    pub abi: Vec<String>,
    pub state: HashMap<String, String>,
    pub deployed: bool,
    pub address: String,
}

#[derive(Debug, Clone)]
pub enum ContractType { Token, NFT, Marketplace, Governance, Custom }

impl std::fmt::Display for ContractType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token => write!(f, "ERC-3T (토큰)"),
            Self::NFT => write!(f, "ERC-3T NFT"),
            Self::Marketplace => write!(f, "마켓플레이스"),
            Self::Governance => write!(f, "거버넌스"),
            Self::Custom => write!(f, "커스텀"),
        }
    }
}

pub struct Web3Service {
    pub contracts: Vec<SmartContract>,
    pub wallets: HashMap<String, u64>, // address → CRWN balance
}

impl Web3Service {
    pub fn new() -> Self {
        let mut s = Self { contracts: Vec::new(), wallets: HashMap::new() };
        s.wallets.insert("treasury".into(), 153_000_000);
        s
    }

    pub fn deploy_contract(&mut self, name: &str, owner: &str, ctype: ContractType) -> CTPResponse {
        let id = format!("ct-{}", short_hash());
        let addr = format!("0xCRWN{}", short_hash().to_uppercase());
        self.contracts.push(SmartContract {
            id: id.clone(), name: name.into(), owner: owner.into(),
            contract_type: ctype, abi: vec!["transfer".into(), "approve".into(), "balanceOf".into()],
            state: HashMap::new(), deployed: true, address: addr.clone(),
        });
        CTPResponse::ok(&format!("컨트랙트 배포: {} → {}", name, addr), Some(addr))
    }

    pub fn mint_nft(&mut self, owner: &str, name: &str, metadata: &str) -> CTPResponse {
        let id = format!("nft-{}", short_hash());
        CTPResponse::ok(&format!("NFT 발행: {} → {} ({})", id, name, owner), Some(id))
    }

    pub fn connect_wallet(&mut self, address: &str, balance: u64) -> CTPResponse {
        self.wallets.insert(address.into(), balance);
        CTPResponse::ok(&format!("지갑 연결: {} ({} CRWN)", address, balance), None)
    }

    pub fn transfer_token(&mut self, from: &str, to: &str, amount: u64) -> CTPResponse {
        let bal = self.wallets.get(from).copied().unwrap_or(0);
        if bal < amount { return CTPResponse::fail("잔액 부족"); }
        *self.wallets.entry(from.into()).or_insert(0) -= amount;
        *self.wallets.entry(to.into()).or_insert(0) += amount;
        CTPResponse::ok(&format!("{} → {} {} CRWN", from, to, amount), None)
    }
}

// ═══════════════════════════════════════
// 통합 플랫폼
// ═══════════════════════════════════════

pub struct CrownyPlatform {
    pub git: GitService,
    pub deploy: DeployService,
    pub db: TritDB,
    pub runtime: AppRuntime,
    pub web3: Web3Service,
}

impl CrownyPlatform {
    pub fn new() -> Self {
        Self {
            git: GitService::new(),
            deploy: DeployService::new(),
            db: TritDB::new(),
            runtime: AppRuntime::new(),
            web3: Web3Service::new(),
        }
    }

    pub fn summary(&self) -> String {
        let (cols, docs) = self.db.stats();
        let running = self.runtime.instances.iter().filter(|a| a.status == AppStatus::Running).count();
        let deployed = self.deploy.deployments.iter().filter(|d| d.status == DeployStatus::Ready).count();
        let mut lines = Vec::new();
        lines.push("═══ Crowny Platform ═══".to_string());
        lines.push(format!("  Git: {} repos | {} total commits",
            self.git.repos.len(),
            self.git.repos.values().map(|r| r.commits.len()).sum::<usize>()));
        lines.push(format!("  Deploy: {} active", deployed));
        lines.push(format!("  DB: {} collections, {} docs", cols, docs));
        lines.push(format!("  Runtime: {} running", running));
        lines.push(format!("  Web3: {} contracts, {} wallets",
            self.web3.contracts.len(), self.web3.wallets.len()));
        lines.join("\n")
    }
}

// ═══ 데모 ═══

pub fn demo_platform() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Platform                              ║");
    println!("║  Git + Deploy + DB + Runtime + Web3           ║");
    println!("║  (GitHub+Vercel+Firebase+Railway+Thirdweb)    ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut platform = CrownyPlatform::new();

    // ── 1. Git ──
    println!("━━━ 1. Git 호스팅 (GitHub) ━━━");
    println!("  {}", platform.git.create_repo("crowny", "tvm-core", "3진 VM 코어", "Rust"));
    println!("  {}", platform.git.create_repo("crowny", "hanseon-lang", "한선어 컴파일러", "한선어"));
    println!("  {}", platform.git.create_repo("crowny", "exchange-ui", "CRWN 거래소", "React"));
    println!("  {}", platform.git.commit("crowny/tvm-core", "ef", "feat: 729 opcodes 구현", 12, 1500, 200));
    println!("  {}", platform.git.commit("crowny/tvm-core", "ef", "fix: trit overflow 수정", 3, 45, 12));
    println!("  {}", platform.git.create_pr("crowny/tvm-core", "alice", "WASM 노드 지원", "feat/wasm", "main"));
    println!("  {}", platform.git.review_pr("crowny/tvm-core", 1, 1)); // P: 승인
    println!();

    // ── 2. Deploy ──
    println!("━━━ 2. 배포 서비스 (Vercel) ━━━");
    println!("  {}", platform.deploy.deploy("tvm-docs", "Next.js", "docs.crowny.dev"));
    println!("  {}", platform.deploy.deploy("exchange", "React", "exchange.crowny.dev"));
    println!("  {}", platform.deploy.deploy("api-gateway", "Rust", "api.crowny.dev"));
    println!();

    // ── 3. DB ──
    println!("━━━ 3. TritDB (Firebase) ━━━");
    println!("  {}", platform.db.create_collection("users"));
    println!("  {}", platform.db.create_collection("transactions"));
    println!("  {}", platform.db.create_collection("consensus_logs"));
    let mut user_data = HashMap::new();
    user_data.insert("name".into(), "Alice".into());
    user_data.insert("role".into(), "validator".into());
    user_data.insert("crwn".into(), "989970".into());
    println!("  {}", platform.db.insert("users", user_data));
    let mut tx_data = HashMap::new();
    tx_data.insert("from".into(), "alice".into());
    tx_data.insert("to".into(), "bob".into());
    tx_data.insert("amount".into(), "10000".into());
    tx_data.insert("trit".into(), "P".into());
    println!("  {}", platform.db.insert("transactions", tx_data));
    let results = platform.db.query("users", "role", "validator");
    println!("  쿼리 'role=validator' → {} 결과", results.len());
    println!();

    // ── 4. Runtime ──
    println!("━━━ 4. 앱 런타임 (Railway) ━━━");
    println!("  {}", platform.runtime.deploy_app("tvm-server", "rust", 3));
    println!("  {}", platform.runtime.deploy_app("consensus-node", "rust", 5));
    println!("  {}", platform.runtime.deploy_app("hanseon-repl", "hanseon", 1));
    println!("  {}", platform.runtime.deploy_app("api-worker", "node", 2));
    if let Some(app) = platform.runtime.instances.first() {
        println!("  {}", platform.runtime.scale(&app.id.clone(), 5));
    }
    println!();

    // ── 5. Web3 ──
    println!("━━━ 5. Web3 토큰 (Thirdweb) ━━━");
    println!("  {}", platform.web3.deploy_contract("CRWN Token", "crowny", ContractType::Token));
    println!("  {}", platform.web3.deploy_contract("Crowny NFT", "crowny", ContractType::NFT));
    println!("  {}", platform.web3.deploy_contract("DAO Governance", "crowny", ContractType::Governance));
    println!("  {}", platform.web3.connect_wallet("alice", 989970));
    println!("  {}", platform.web3.connect_wallet("bob", 505000));
    println!("  {}", platform.web3.transfer_token("alice", "bob", 5000));
    println!("  {}", platform.web3.mint_nft("alice", "Crowny Genesis #001", "ipfs://Qm..."));
    println!();

    // ── 6. 통합 요약 ──
    println!("━━━ 6. 플랫폼 요약 ━━━");
    println!("{}", platform.summary());
    println!();
    println!("✓ Crowny Platform 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_create_repo() {
        let mut git = GitService::new();
        let r = git.create_repo("user", "repo", "desc", "Rust");
        assert_eq!(r.trit, 1);
        assert_eq!(git.repos.len(), 1);
    }

    #[test]
    fn test_git_commit() {
        let mut git = GitService::new();
        git.create_repo("u", "r", "", "Rust");
        let r = git.commit("u/r", "a", "msg", 1, 10, 5);
        assert_eq!(r.trit, 1);
    }

    #[test]
    fn test_deploy() {
        let mut ds = DeployService::new();
        let r = ds.deploy("proj", "React", "proj.crowny.dev");
        assert_eq!(r.trit, 1);
        assert_eq!(ds.deployments.len(), 1);
    }

    #[test]
    fn test_db_insert_query() {
        let mut db = TritDB::new();
        db.create_collection("users");
        let mut data = HashMap::new();
        data.insert("name".into(), "alice".into());
        db.insert("users", data);
        let results = db.query("users", "name", "alice");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_runtime() {
        let mut rt = AppRuntime::new();
        let r = rt.deploy_app("test", "rust", 1);
        assert_eq!(r.trit, 1);
        assert_eq!(rt.instances.len(), 1);
    }

    #[test]
    fn test_web3_transfer() {
        let mut w3 = Web3Service::new();
        w3.connect_wallet("alice", 1000);
        let r = w3.transfer_token("alice", "bob", 500);
        assert_eq!(r.trit, 1);
        assert_eq!(*w3.wallets.get("alice").unwrap(), 500);
        assert_eq!(*w3.wallets.get("bob").unwrap(), 500);
    }

    #[test]
    fn test_web3_insufficient() {
        let mut w3 = Web3Service::new();
        w3.connect_wallet("alice", 100);
        let r = w3.transfer_token("alice", "bob", 500);
        assert_eq!(r.trit, -1);
    }

    #[test]
    fn test_platform_summary() {
        let p = CrownyPlatform::new();
        let s = p.summary();
        assert!(s.contains("Crowny Platform"));
    }
}
