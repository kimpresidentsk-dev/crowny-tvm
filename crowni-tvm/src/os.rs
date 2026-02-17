// ═══════════════════════════════════════════════════════════════
// Crowny OS — 3진 운영체제 레이어
// 프로세스 관리 · 3진 파일시스템 · TritShell
// 모든 시스템 콜이 P/O/T 상태를 반환
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

// ═══════════════════════════════════════
// 시스템 콜 응답
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SysCall {
    pub trit: i8,
    pub code: u32,
    pub message: String,
    pub data: Option<String>,
}

impl SysCall {
    pub fn ok(msg: &str, data: Option<String>) -> Self { Self { trit: 1, code: 0, message: msg.into(), data } }
    pub fn pending(msg: &str) -> Self { Self { trit: 0, code: 1, message: msg.into(), data: None } }
    pub fn fail(msg: &str, code: u32) -> Self { Self { trit: -1, code, message: msg.into(), data: None } }
    pub fn label(&self) -> &str { match self.trit { 1 => "P", -1 => "T", _ => "O" } }
}

impl std::fmt::Display for SysCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.label(), self.message)
    }
}

// ═══════════════════════════════════════
// 1. 프로세스 관리
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessState {
    Running,    // P: 실행 중
    Sleeping,   // O: 대기
    Stopped,    // T: 중지
    Zombie,     // T: 좀비
    Ready,      // O: 준비
}

impl ProcessState {
    pub fn trit(&self) -> i8 {
        match self { Self::Running => 1, Self::Sleeping | Self::Ready => 0, _ => -1 }
    }
}

impl std::fmt::Display for ProcessState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "●Running"),
            Self::Sleeping => write!(f, "◐Sleep"),
            Self::Stopped => write!(f, "■Stop"),
            Self::Zombie => write!(f, "✗Zombie"),
            Self::Ready => write!(f, "○Ready"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProcessPriority { High, Normal, Low, Idle }

impl ProcessPriority {
    pub fn trit(&self) -> i8 {
        match self { Self::High => 1, Self::Normal => 0, Self::Low => -1, Self::Idle => -1 }
    }
    pub fn nice(&self) -> i8 {
        match self { Self::High => -10, Self::Normal => 0, Self::Low => 10, Self::Idle => 19 }
    }
}

impl std::fmt::Display for ProcessPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "높음"),
            Self::Normal => write!(f, "보통"),
            Self::Low => write!(f, "낮음"),
            Self::Idle => write!(f, "유휴"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub state: ProcessState,
    pub priority: ProcessPriority,
    pub parent_pid: Option<u32>,
    pub children: Vec<u32>,
    pub cpu_usage: f64,
    pub memory_kb: u64,
    pub trit_state: i8,
    pub owner: String,
    pub started_at: u64,
    pub syscalls: u64,
}

pub struct ProcessManager {
    pub processes: Vec<Process>,
    pub pid_counter: u32,
    pub cpu_total: f64,
    pub memory_total_kb: u64,
    pub memory_used_kb: u64,
    pub uptime_ms: u64,
}

impl ProcessManager {
    pub fn new(memory_mb: u64) -> Self {
        let mut pm = Self {
            processes: Vec::new(),
            pid_counter: 0,
            cpu_total: 0.0,
            memory_total_kb: memory_mb * 1024,
            memory_used_kb: 0,
            uptime_ms: now_ms(),
        };
        // PID 0: 커널
        pm.spawn("crowny-kernel", "root", ProcessPriority::High, 2048);
        // PID 1: init
        pm.spawn("trit-init", "root", ProcessPriority::High, 1024);
        pm
    }

    pub fn spawn(&mut self, name: &str, owner: &str, priority: ProcessPriority, mem_kb: u64) -> SysCall {
        if self.memory_used_kb + mem_kb > self.memory_total_kb {
            return SysCall::fail(&format!("메모리 부족: {}KB 필요, {}KB 남음",
                mem_kb, self.memory_total_kb - self.memory_used_kb), 12);
        }

        let pid = self.pid_counter;
        self.pid_counter += 1;
        self.memory_used_kb += mem_kb;

        let parent = if pid > 1 { Some(1) } else { None };

        self.processes.push(Process {
            pid, name: name.into(), state: ProcessState::Running,
            priority, parent_pid: parent, children: Vec::new(),
            cpu_usage: 0.0, memory_kb: mem_kb, trit_state: 1,
            owner: owner.into(), started_at: now_ms(), syscalls: 0,
        });

        // 부모에 자식 등록
        if let Some(ppid) = parent {
            if let Some(parent_proc) = self.processes.iter_mut().find(|p| p.pid == ppid) {
                parent_proc.children.push(pid);
            }
        }

        SysCall::ok(&format!("spawn PID:{} '{}' ({}KB, {})", pid, name, mem_kb, priority), Some(pid.to_string()))
    }

    pub fn kill(&mut self, pid: u32) -> SysCall {
        if pid <= 1 { return SysCall::fail("커널/init 프로세스 종료 불가", 1); }
        if let Some(proc) = self.processes.iter_mut().find(|p| p.pid == pid) {
            proc.state = ProcessState::Zombie;
            proc.trit_state = -1;
            self.memory_used_kb = self.memory_used_kb.saturating_sub(proc.memory_kb);
            let name = proc.name.clone();
            SysCall::ok(&format!("kill PID:{} '{}'", pid, name), None)
        } else {
            SysCall::fail(&format!("PID:{} 없음", pid), 3)
        }
    }

    pub fn sleep_proc(&mut self, pid: u32) -> SysCall {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.pid == pid) {
            proc.state = ProcessState::Sleeping;
            proc.trit_state = 0;
            SysCall::ok(&format!("sleep PID:{}", pid), None)
        } else {
            SysCall::fail(&format!("PID:{} 없음", pid), 3)
        }
    }

    pub fn wake(&mut self, pid: u32) -> SysCall {
        if let Some(proc) = self.processes.iter_mut().find(|p| p.pid == pid) {
            proc.state = ProcessState::Running;
            proc.trit_state = 1;
            SysCall::ok(&format!("wake PID:{}", pid), None)
        } else {
            SysCall::fail(&format!("PID:{} 없음", pid), 3)
        }
    }

    pub fn ps(&self) -> Vec<&Process> {
        self.processes.iter().filter(|p| p.state != ProcessState::Zombie).collect()
    }

    pub fn find(&self, name: &str) -> Option<&Process> {
        self.processes.iter().find(|p| p.name == name && p.state != ProcessState::Zombie)
    }

    pub fn running_count(&self) -> usize {
        self.processes.iter().filter(|p| p.state == ProcessState::Running).count()
    }

    pub fn summary(&self) -> String {
        let running = self.running_count();
        let sleeping = self.processes.iter().filter(|p| p.state == ProcessState::Sleeping).count();
        let zombies = self.processes.iter().filter(|p| p.state == ProcessState::Zombie).count();
        let mem_pct = self.memory_used_kb as f64 / self.memory_total_kb as f64 * 100.0;
        format!("프로세스: {} (실행:{} 대기:{} 좀비:{}) | 메모리: {}/{}KB ({:.1}%)",
            self.processes.len(), running, sleeping, zombies,
            self.memory_used_kb, self.memory_total_kb, mem_pct)
    }
}

// ═══════════════════════════════════════
// 2. 3진 파일시스템 (TritFS)
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum FileType { File, Directory, SymLink, Device, Pipe }

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File => write!(f, "-"), Self::Directory => write!(f, "d"),
            Self::SymLink => write!(f, "l"), Self::Device => write!(f, "c"),
            Self::Pipe => write!(f, "p"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TritPermission {
    pub owner: i8,  // P: rwx, O: r--, T: ---
    pub group: i8,
    pub other: i8,
}

impl TritPermission {
    pub fn full() -> Self { Self { owner: 1, group: 1, other: 0 } }
    pub fn readonly() -> Self { Self { owner: 1, group: 0, other: 0 } }
    pub fn private() -> Self { Self { owner: 1, group: -1, other: -1 } }

    pub fn can_read(&self, is_owner: bool, is_group: bool) -> bool {
        if is_owner { self.owner >= 0 }
        else if is_group { self.group >= 0 }
        else { self.other >= 0 }
    }

    pub fn can_write(&self, is_owner: bool, is_group: bool) -> bool {
        if is_owner { self.owner > 0 }
        else if is_group { self.group > 0 }
        else { self.other > 0 }
    }
}

impl std::fmt::Display for TritPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn p(t: i8) -> &'static str { match t { 1 => "rwx", 0 => "r--", _ => "---" } }
        write!(f, "{}{}{}", p(self.owner), p(self.group), p(self.other))
    }
}

#[derive(Debug, Clone)]
pub struct INode {
    pub id: u64,
    pub name: String,
    pub file_type: FileType,
    pub permission: TritPermission,
    pub owner: String,
    pub group: String,
    pub size_bytes: u64,
    pub content: Option<String>,
    pub children: Vec<u64>,     // 디렉토리면 자식 inode 목록
    pub parent: Option<u64>,
    pub trit_state: i8,         // P: 정상, O: 수정중, T: 삭제예정
    pub created_at: u64,
    pub modified_at: u64,
}

pub struct TritFS {
    pub inodes: HashMap<u64, INode>,
    pub inode_counter: u64,
    pub cwd: u64,               // 현재 디렉토리 inode
    pub mount_point: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
}

impl TritFS {
    pub fn new(total_mb: u64) -> Self {
        let mut fs = Self {
            inodes: HashMap::new(),
            inode_counter: 0,
            cwd: 0,
            mount_point: "/".into(),
            total_bytes: total_mb * 1024 * 1024,
            used_bytes: 0,
        };
        // 루트 디렉토리
        fs.create_inode("/", FileType::Directory, TritPermission::full(), "root", None);
        // 기본 디렉토리 구조
        let root_id = 0;
        let dirs = vec![
            ("bin", "시스템 바이너리"),
            ("etc", "설정 파일"),
            ("home", "사용자 홈"),
            ("usr", "사용자 프로그램"),
            ("var", "가변 데이터"),
            ("tmp", "임시 파일"),
            ("dev", "디바이스"),
            ("proc", "프로세스 정보"),
            ("crwn", "크라운 시스템"),
        ];
        for (name, _desc) in &dirs {
            fs.mkdir_at(root_id, name, "root");
        }
        // /crwn 하위
        let crwn_id = fs.find_child(root_id, "crwn").unwrap();
        for name in &["tvm", "hanseon", "consensus", "tokens", "nodes", "platform"] {
            fs.mkdir_at(crwn_id, name, "root");
        }
        // 기본 파일들
        let etc_id = fs.find_child(root_id, "etc").unwrap();
        fs.create_file_at(etc_id, "crowny.conf", "root",
            "# Crowny OS Configuration\nversion=0.9.0\ntrit_mode=balanced\nconsensus=3\nport=3333\n");
        fs.create_file_at(etc_id, "hosts", "root",
            "127.0.0.1  localhost\n127.0.0.1  crowny\n127.0.0.1  tvm.local\n");

        let bin_id = fs.find_child(root_id, "bin").unwrap();
        for cmd in &["tvm", "hanseon", "crwnsh", "trit", "cpm", "consensus", "deploy", "wallet"] {
            fs.create_file_at(bin_id, cmd, "root", &format!("#!/bin/tvm\n# {} command binary\n", cmd));
        }

        let home_id = fs.find_child(root_id, "home").unwrap();
        let ef_id = fs.mkdir_at(home_id, "ef", "ef");
        fs.create_file_at(ef_id, ".crwnrc", "ef",
            "# ef's shell config\nPROMPT=\"crwn> \"\nPATH=/bin:/usr/bin\nEDITOR=trit-vim\nTHEME=dark\n");

        fs
    }

    fn create_inode(&mut self, name: &str, file_type: FileType, perm: TritPermission, owner: &str, parent: Option<u64>) -> u64 {
        let id = self.inode_counter;
        self.inode_counter += 1;
        self.inodes.insert(id, INode {
            id, name: name.into(), file_type, permission: perm,
            owner: owner.into(), group: owner.into(),
            size_bytes: 0, content: None, children: Vec::new(),
            parent, trit_state: 1, created_at: now_ms(), modified_at: now_ms(),
        });
        id
    }

    pub fn mkdir_at(&mut self, parent_id: u64, name: &str, owner: &str) -> u64 {
        let id = self.create_inode(name, FileType::Directory, TritPermission::full(), owner, Some(parent_id));
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.push(id);
        }
        id
    }

    pub fn create_file_at(&mut self, parent_id: u64, name: &str, owner: &str, content: &str) -> u64 {
        let size = content.len() as u64;
        let id = self.create_inode(name, FileType::File, TritPermission::full(), owner, Some(parent_id));
        if let Some(inode) = self.inodes.get_mut(&id) {
            inode.content = Some(content.into());
            inode.size_bytes = size;
        }
        self.used_bytes += size;
        if let Some(parent) = self.inodes.get_mut(&parent_id) {
            parent.children.push(id);
        }
        id
    }

    pub fn find_child(&self, parent_id: u64, name: &str) -> Option<u64> {
        if let Some(parent) = self.inodes.get(&parent_id) {
            for &child_id in &parent.children {
                if let Some(child) = self.inodes.get(&child_id) {
                    if child.name == name { return Some(child_id); }
                }
            }
        }
        None
    }

    pub fn ls(&self, dir_id: u64) -> Vec<&INode> {
        if let Some(dir) = self.inodes.get(&dir_id) {
            dir.children.iter().filter_map(|id| self.inodes.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    pub fn cat(&self, file_id: u64) -> SysCall {
        if let Some(inode) = self.inodes.get(&file_id) {
            if inode.file_type == FileType::Directory {
                return SysCall::fail("디렉토리입니다", 21);
            }
            SysCall::ok(&inode.name, inode.content.clone())
        } else {
            SysCall::fail("파일 없음", 2)
        }
    }

    pub fn write(&mut self, file_id: u64, content: &str) -> SysCall {
        if let Some(inode) = self.inodes.get_mut(&file_id) {
            let old_size = inode.size_bytes;
            inode.content = Some(content.into());
            inode.size_bytes = content.len() as u64;
            inode.modified_at = now_ms();
            inode.trit_state = 1;
            self.used_bytes = self.used_bytes - old_size + inode.size_bytes;
            SysCall::ok(&format!("write '{}' {}B", inode.name, inode.size_bytes), None)
        } else {
            SysCall::fail("파일 없음", 2)
        }
    }

    pub fn rm(&mut self, file_id: u64) -> SysCall {
        if let Some(inode) = self.inodes.get_mut(&file_id) {
            if inode.file_type == FileType::Directory && !inode.children.is_empty() {
                return SysCall::fail("비어있지 않은 디렉토리", 39);
            }
            inode.trit_state = -1;
            let name = inode.name.clone();
            self.used_bytes = self.used_bytes.saturating_sub(inode.size_bytes);
            SysCall::ok(&format!("rm '{}'", name), None)
        } else {
            SysCall::fail("파일 없음", 2)
        }
    }

    pub fn resolve_path(&self, path: &str) -> Option<u64> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current = 0u64; // root
        for part in parts {
            match self.find_child(current, part) {
                Some(id) => current = id,
                None => return None,
            }
        }
        Some(current)
    }

    pub fn tree(&self, id: u64, depth: usize, max_depth: usize) -> String {
        if depth > max_depth { return String::new(); }
        let mut out = String::new();
        if let Some(inode) = self.inodes.get(&id) {
            let pad = "  ".repeat(depth);
            let trit = match inode.trit_state { 1 => "●", -1 => "✗", _ => "○" };
            let icon = match inode.file_type {
                FileType::Directory => "📁",
                FileType::File => "📄",
                FileType::SymLink => "🔗",
                FileType::Device => "💾",
                FileType::Pipe => "🔀",
            };
            let size = if inode.file_type == FileType::File { format!(" ({}B)", inode.size_bytes) } else { String::new() };
            out.push_str(&format!("{}{}{} {}{}\n", pad, trit, icon, inode.name, size));
            for &child_id in &inode.children {
                out.push_str(&self.tree(child_id, depth + 1, max_depth));
            }
        }
        out
    }

    pub fn stat(&self) -> String {
        let pct = self.used_bytes as f64 / self.total_bytes as f64 * 100.0;
        format!("TritFS: {} inodes | {}/{}B ({:.2}%) | mount: {}",
            self.inodes.len(), self.used_bytes, self.total_bytes, pct, self.mount_point)
    }
}

// ═══════════════════════════════════════
// 3. TritShell — 3진 쉘
// ═══════════════════════════════════════

pub struct TritShell {
    pub user: String,
    pub hostname: String,
    pub history: Vec<String>,
    pub env: HashMap<String, String>,
    pub aliases: HashMap<String, String>,
    pub exit_trit: i8,
    pub output: Vec<String>,
}

impl TritShell {
    pub fn new(user: &str) -> Self {
        let mut env = HashMap::new();
        env.insert("PATH".into(), "/bin:/usr/bin:/crwn/bin".into());
        env.insert("HOME".into(), format!("/home/{}", user));
        env.insert("USER".into(), user.into());
        env.insert("SHELL".into(), "/bin/crwnsh".into());
        env.insert("EDITOR".into(), "trit-vim".into());
        env.insert("LANG".into(), "ko_KR.UTF-8".into());
        env.insert("TRIT_MODE".into(), "balanced".into());

        let mut aliases = HashMap::new();
        aliases.insert("ll".into(), "ls -la".into());
        aliases.insert("cls".into(), "clear".into());
        aliases.insert("..".into(), "cd ..".into());

        Self {
            user: user.into(),
            hostname: "crowny".into(),
            history: Vec::new(),
            env, aliases,
            exit_trit: 1,
            output: Vec::new(),
        }
    }

    pub fn prompt(&self) -> String {
        let trit = match self.exit_trit { 1 => "P", -1 => "T", _ => "O" };
        format!("[{}] {}@{} crwn> ", trit, self.user, self.hostname)
    }

    pub fn execute(&mut self, cmd: &str, pm: &mut ProcessManager, fs: &mut TritFS) -> Vec<String> {
        self.history.push(cmd.into());
        self.output.clear();
        let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
        if parts.is_empty() { return Vec::new(); }

        let resolved = self.aliases.get(parts[0]).cloned();
        let actual_cmd = resolved.as_deref().unwrap_or(parts[0]);

        match actual_cmd {
            "ps" => {
                self.output.push("  PID  STATE     PRI    MEM     NAME".into());
                self.output.push("  ---  -----     ---    ---     ----".into());
                for proc in pm.ps() {
                    let trit = match proc.trit_state { 1 => "P", -1 => "T", _ => "O" };
                    self.output.push(format!("  [{}] {:>3}  {:<10} {:<6} {:>6}KB  {}",
                        trit, proc.pid, proc.state, proc.priority, proc.memory_kb, proc.name));
                }
                self.exit_trit = 1;
            }
            "spawn" => {
                let name = parts.get(1).unwrap_or(&"untitled");
                let mem: u64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(512);
                let result = pm.spawn(name, &self.user, ProcessPriority::Normal, mem);
                self.output.push(format!("  {}", result));
                self.exit_trit = result.trit;
            }
            "kill" => {
                let pid: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                let result = pm.kill(pid);
                self.output.push(format!("  {}", result));
                self.exit_trit = result.trit;
            }
            "ls" => {
                let entries = fs.ls(fs.cwd);
                for inode in entries {
                    if inode.trit_state < 0 { continue; }
                    let trit = match inode.trit_state { 1 => "P", -1 => "T", _ => "O" };
                    let size = if inode.file_type == FileType::File {
                        format!("{:>6}B", inode.size_bytes)
                    } else { "     -".into() };
                    self.output.push(format!("  [{}] {} {} {} {}",
                        trit, inode.file_type, inode.permission, size, inode.name));
                }
                self.exit_trit = 1;
            }
            "cd" => {
                let target = parts.get(1).unwrap_or(&"/");
                if *target == "/" { fs.cwd = 0; self.exit_trit = 1; }
                else if *target == ".." {
                    if let Some(inode) = fs.inodes.get(&fs.cwd) {
                        fs.cwd = inode.parent.unwrap_or(0);
                    }
                    self.exit_trit = 1;
                } else if let Some(id) = fs.find_child(fs.cwd, target) {
                    fs.cwd = id;
                    self.exit_trit = 1;
                } else {
                    self.output.push(format!("  [T] cd: '{}' 없음", target));
                    self.exit_trit = -1;
                }
            }
            "cat" => {
                let name = parts.get(1).unwrap_or(&"");
                if let Some(id) = fs.find_child(fs.cwd, name) {
                    let result = fs.cat(id);
                    if let Some(data) = &result.data {
                        for line in data.lines() { self.output.push(format!("  {}", line)); }
                    }
                    self.exit_trit = result.trit;
                } else {
                    self.output.push(format!("  [T] cat: '{}' 없음", name));
                    self.exit_trit = -1;
                }
            }
            "mkdir" => {
                let name = parts.get(1).unwrap_or(&"new_dir");
                fs.mkdir_at(fs.cwd, name, &self.user);
                self.output.push(format!("  [P] mkdir '{}'", name));
                self.exit_trit = 1;
            }
            "touch" => {
                let name = parts.get(1).unwrap_or(&"new_file");
                fs.create_file_at(fs.cwd, name, &self.user, "");
                self.output.push(format!("  [P] touch '{}'", name));
                self.exit_trit = 1;
            }
            "tree" => {
                let tree_str = fs.tree(fs.cwd, 0, 3);
                for line in tree_str.lines() { self.output.push(format!("  {}", line)); }
                self.exit_trit = 1;
            }
            "pwd" => {
                // 경로 추적
                let mut path_parts = Vec::new();
                let mut cur = fs.cwd;
                loop {
                    if let Some(inode) = fs.inodes.get(&cur) {
                        path_parts.push(inode.name.clone());
                        if let Some(parent) = inode.parent {
                            cur = parent;
                        } else { break; }
                    } else { break; }
                }
                path_parts.reverse();
                let path = if path_parts.len() <= 1 { "/".into() }
                    else { path_parts.join("/").replacen("//", "/", 1) };
                self.output.push(format!("  {}", path));
                self.exit_trit = 1;
            }
            "env" => {
                for (k, v) in &self.env {
                    self.output.push(format!("  {}={}", k, v));
                }
                self.exit_trit = 1;
            }
            "export" => {
                if let Some(kv) = parts.get(1) {
                    let pair: Vec<&str> = kv.splitn(2, '=').collect();
                    if pair.len() == 2 {
                        self.env.insert(pair[0].into(), pair[1].into());
                        self.output.push(format!("  [P] export {}={}", pair[0], pair[1]));
                    }
                }
                self.exit_trit = 1;
            }
            "stat" => {
                self.output.push(format!("  {}", pm.summary()));
                self.output.push(format!("  {}", fs.stat()));
                self.exit_trit = 1;
            }
            "history" => {
                for (i, h) in self.history.iter().enumerate() {
                    self.output.push(format!("  {:>4}  {}", i + 1, h));
                }
                self.exit_trit = 1;
            }
            "whoami" => {
                self.output.push(format!("  {}", self.user));
                self.exit_trit = 1;
            }
            "hostname" => {
                self.output.push(format!("  {}", self.hostname));
                self.exit_trit = 1;
            }
            "uname" => {
                self.output.push("  CrownyOS 0.9.0 (Balanced Ternary) aarch64".into());
                self.exit_trit = 1;
            }
            "help" => {
                self.output.push("  ━━━ TritShell 명령어 ━━━".into());
                self.output.push("  ps            프로세스 목록".into());
                self.output.push("  spawn <n> <m> 프로세스 생성 (이름, 메모리KB)".into());
                self.output.push("  kill <pid>    프로세스 종료".into());
                self.output.push("  ls            파일 목록".into());
                self.output.push("  cd <dir>      디렉토리 이동".into());
                self.output.push("  cat <file>    파일 읽기".into());
                self.output.push("  mkdir <name>  디렉토리 생성".into());
                self.output.push("  touch <name>  빈 파일 생성".into());
                self.output.push("  tree          디렉토리 트리".into());
                self.output.push("  pwd           현재 경로".into());
                self.output.push("  env           환경 변수".into());
                self.output.push("  stat          시스템 상태".into());
                self.output.push("  uname         OS 정보".into());
                self.output.push("  whoami        현재 사용자".into());
                self.output.push("  history       명령어 이력".into());
                self.exit_trit = 1;
            }
            _ => {
                self.output.push(format!("  [T] crwnsh: '{}' 명령어를 찾을 수 없습니다", actual_cmd));
                self.exit_trit = -1;
            }
        }
        self.output.clone()
    }
}

// ═══ 통합 OS ═══

pub struct CrownyOS {
    pub pm: ProcessManager,
    pub fs: TritFS,
    pub shell: TritShell,
    pub booted: bool,
    pub version: String,
}

impl CrownyOS {
    pub fn boot() -> Self {
        let mut os = Self {
            pm: ProcessManager::new(512),  // 512MB RAM
            fs: TritFS::new(1024),         // 1GB disk
            shell: TritShell::new("ef"),
            booted: true,
            version: "0.9.0".into(),
        };
        // 시스템 데몬
        os.pm.spawn("trit-scheduler", "root", ProcessPriority::High, 4096);
        os.pm.spawn("consensus-daemon", "root", ProcessPriority::High, 8192);
        os.pm.spawn("tvm-runtime", "root", ProcessPriority::Normal, 16384);
        os.pm.spawn("ctp-server", "root", ProcessPriority::Normal, 4096);
        os.pm.spawn("trit-logger", "root", ProcessPriority::Low, 1024);
        os.pm.spawn("wallet-daemon", "root", ProcessPriority::Low, 2048);
        os
    }
}

// ═══ 데모 ═══

pub fn demo_os() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  CrownyOS v0.9.0 — 3진 운영체제               ║");
    println!("║  프로세스 관리 · TritFS · TritShell             ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    // 부팅
    println!("━━━ BOOT SEQUENCE ━━━");
    println!("  [P] 커널 로딩... crowny-kernel");
    println!("  [P] Init 시스템... trit-init");
    let mut os = CrownyOS::boot();
    println!("  [P] 스케줄러... trit-scheduler");
    println!("  [P] 합의 데몬... consensus-daemon");
    println!("  [P] TVM 런타임... tvm-runtime");
    println!("  [P] CTP 서버... ctp-server");
    println!("  [P] 로거... trit-logger");
    println!("  [P] 지갑 데몬... wallet-daemon");
    println!("  ✓ CrownyOS v{} 부팅 완료", os.version);
    println!();

    // 쉘 세션
    println!("━━━ TritShell 세션 ━━━");
    let commands = vec![
        "uname",
        "whoami",
        "ps",
        "stat",
        "ls",
        "cd etc",
        "ls",
        "cat crowny.conf",
        "cd /",
        "cd home/ef",
        "cat .crwnrc",
        "cd /",
        "tree",
        "spawn web-server 2048",
        "spawn api-worker 1024",
        "ps",
        "kill 9",
        "ps",
        "cd crwn",
        "ls",
        "mkdir apps",
        "ls",
        "history",
        "stat",
    ];

    for cmd in &commands {
        println!("{}{}", os.shell.prompt(), cmd);
        let output = os.shell.execute(cmd, &mut os.pm, &mut os.fs);
        for line in &output { println!("{}", line); }
        println!();
    }

    println!("━━━ OS 최종 상태 ━━━");
    println!("  {}", os.pm.summary());
    println!("  {}", os.fs.stat());
    println!();
    println!("✓ CrownyOS 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_spawn() {
        let mut pm = ProcessManager::new(128);
        let r = pm.spawn("test", "user", ProcessPriority::Normal, 512);
        assert_eq!(r.trit, 1);
        assert!(pm.processes.len() >= 3); // kernel + init + test
    }

    #[test]
    fn test_process_kill() {
        let mut pm = ProcessManager::new(128);
        pm.spawn("victim", "user", ProcessPriority::Normal, 512);
        let r = pm.kill(2);
        assert_eq!(r.trit, 1);
    }

    #[test]
    fn test_kill_kernel_denied() {
        let mut pm = ProcessManager::new(128);
        let r = pm.kill(0);
        assert_eq!(r.trit, -1);
    }

    #[test]
    fn test_process_sleep_wake() {
        let mut pm = ProcessManager::new(128);
        pm.spawn("sleeper", "user", ProcessPriority::Normal, 256);
        pm.sleep_proc(2);
        assert_eq!(pm.processes[2].state, ProcessState::Sleeping);
        pm.wake(2);
        assert_eq!(pm.processes[2].state, ProcessState::Running);
    }

    #[test]
    fn test_memory_limit() {
        let mut pm = ProcessManager::new(1); // 1MB
        let r = pm.spawn("big", "user", ProcessPriority::Normal, 2048 * 1024); // 2GB
        assert_eq!(r.trit, -1);
    }

    #[test]
    fn test_fs_basic() {
        let fs = TritFS::new(100);
        assert!(fs.inodes.len() > 10); // root + defaults
        assert!(fs.resolve_path("/etc").is_some());
        assert!(fs.resolve_path("/bin").is_some());
        assert!(fs.resolve_path("/crwn").is_some());
    }

    #[test]
    fn test_fs_mkdir_and_find() {
        let mut fs = TritFS::new(100);
        let root = 0;
        let id = fs.mkdir_at(root, "test_dir", "user");
        assert!(fs.find_child(root, "test_dir").is_some());
        assert_eq!(fs.find_child(root, "test_dir").unwrap(), id);
    }

    #[test]
    fn test_fs_file_create_and_read() {
        let mut fs = TritFS::new(100);
        let root = 0;
        let id = fs.create_file_at(root, "hello.txt", "user", "Hello World");
        let result = fs.cat(id);
        assert_eq!(result.trit, 1);
        assert_eq!(result.data.unwrap(), "Hello World");
    }

    #[test]
    fn test_fs_write() {
        let mut fs = TritFS::new(100);
        let id = fs.create_file_at(0, "f.txt", "user", "old");
        fs.write(id, "new content");
        let r = fs.cat(id);
        assert_eq!(r.data.unwrap(), "new content");
    }

    #[test]
    fn test_fs_resolve_path() {
        let fs = TritFS::new(100);
        assert!(fs.resolve_path("/etc/crowny.conf").is_some());
        assert!(fs.resolve_path("/crwn/tvm").is_some());
        assert!(fs.resolve_path("/nonexistent").is_none());
    }

    #[test]
    fn test_trit_permission() {
        let p = TritPermission::full();
        assert!(p.can_read(true, false));
        assert!(p.can_write(true, false));
        let priv_p = TritPermission::private();
        assert!(priv_p.can_read(true, false));
        assert!(!priv_p.can_read(false, false)); // other: T = ---
    }

    #[test]
    fn test_shell_ps() {
        let mut os = CrownyOS::boot();
        let out = os.shell.execute("ps", &mut os.pm, &mut os.fs);
        assert!(out.len() > 2); // header + processes
    }

    #[test]
    fn test_shell_ls() {
        let mut os = CrownyOS::boot();
        let out = os.shell.execute("ls", &mut os.pm, &mut os.fs);
        assert!(out.len() > 5); // bin, etc, home, ...
    }

    #[test]
    fn test_shell_cd_and_cat() {
        let mut os = CrownyOS::boot();
        os.shell.execute("cd etc", &mut os.pm, &mut os.fs);
        let out = os.shell.execute("cat crowny.conf", &mut os.pm, &mut os.fs);
        assert!(out.iter().any(|l| l.contains("version")));
    }

    #[test]
    fn test_shell_unknown_cmd() {
        let mut os = CrownyOS::boot();
        os.shell.execute("nonexistent", &mut os.pm, &mut os.fs);
        assert_eq!(os.shell.exit_trit, -1);
    }

    #[test]
    fn test_os_boot() {
        let os = CrownyOS::boot();
        assert!(os.booted);
        assert!(os.pm.running_count() >= 6);
    }
}
