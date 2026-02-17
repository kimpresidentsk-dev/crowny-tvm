///! ═══════════════════════════════════════════════════
///! Crowny Meta-Kernel — 균형3진 미들웨어 커널
///! ═══════════════════════════════════════════════════
///!
///! "균형3진 Meta-OS"
///! 2진 OS 위에서 돌아가지만, 논리 세계는 3진으로 완전 독립.
///!
///! 구성요소:
///!   1. TVM         — 균형3진 가상머신 (실행 엔진)
///!   2. Scheduler   — 3진 스케줄러 (태스크 관리)
///!   3. Permission  — 3진 권한 엔진 (접근 제어)
///!   4. Transaction — 3진 트랜잭션 (상태 관리)
///!
///! ┌─────────────────────────────────────────┐
///! │           Crowny Meta-Kernel            │
///! │                                         │
///! │  ┌─────────┐  ┌───────┐  ┌──────────┐  │
///! │  │Scheduler│  │ Perm  │  │   TxEng  │  │
///! │  │ P/O/T   │  │ P/O/T │  │  P/O/T   │  │
///! │  └────┬────┘  └───┬───┘  └────┬─────┘  │
///! │       └────────┬──┘───────────┘         │
///! │           ┌────┴────┐                   │
///! │           │   TVM   │                   │
///! │           │ 729 ops │                   │
///! │           └─────────┘                   │
///! └─────────────────────────────────────────┘
///!              ↓ Host Runtime ↓
///! ┌─────────────────────────────────────────┐
///! │        2진 OS (Linux/macOS)             │
///! └─────────────────────────────────────────┘

use crate::vm::TVM;
use crate::scheduler::{TritScheduler, TritPriority, TritResult, TaskFn};
use crate::permission::{PermissionEngine, TritPermission, Action};
use crate::transaction::{TransactionEngine, TxState, TxId};

// ─────────────────────────────────────────────
// Kernel Config
// ─────────────────────────────────────────────

/// 커널 설정
pub struct KernelConfig {
    pub debug: bool,
    pub max_tasks: usize,
    pub default_permission: TritPermission,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            debug: false,
            max_tasks: 729,  // 3^6, 한선어답게
            default_permission: TritPermission::Review,
        }
    }
}

// ─────────────────────────────────────────────
// Crowny Kernel
// ─────────────────────────────────────────────

/// Crowny Meta-Kernel — 통합 허브
pub struct CrownyKernel {
    /// 균형3진 가상머신
    pub vm: TVM,
    /// 3진 스케줄러
    pub scheduler: TritScheduler,
    /// 3진 권한 엔진
    pub permission: PermissionEngine,
    /// 3진 트랜잭션 엔진
    pub transaction: TransactionEngine,
    /// 커널 상태 (3진)
    pub state: KernelState,
    /// 설정
    pub config: KernelConfig,
    /// 부팅 이후 실행된 총 연산 수
    pub total_ops: u64,
}

/// 커널 상태 (3진)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum KernelState {
    Running  =  1,  // P = 실행 중
    Standby  =  0,  // O = 대기
    Shutdown = -1,  // T = 종료
}

impl std::fmt::Display for KernelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelState::Running => write!(f, "P(실행)"),
            KernelState::Standby => write!(f, "O(대기)"),
            KernelState::Shutdown => write!(f, "T(종료)"),
        }
    }
}

impl CrownyKernel {
    /// 커널 부팅
    pub fn boot(config: KernelConfig) -> Self {
        let mut kernel = Self {
            vm: TVM::new(),
            scheduler: TritScheduler::new(),
            permission: PermissionEngine::new(),
            transaction: TransactionEngine::new(),
            state: KernelState::Standby,
            config,
            total_ops: 0,
        };

        // 기본 권한 정책 설정
        kernel.init_default_policies();
        kernel.state = KernelState::Running;

        if kernel.config.debug {
            eprintln!("[KERNEL] Crowny Meta-Kernel 부팅 완료");
        }

        kernel
    }

    /// 기본 정책 초기화
    fn init_default_policies(&mut self) {
        // 커널 자체는 전권
        self.permission.add_policy("kernel", "*", Action::Admin,
            TritPermission::Allow, "커널 전권");
        // 기본: 읽기 허용
        self.permission.add_policy("*", "*", Action::Read,
            TritPermission::Allow, "기본 읽기 허용");
        // 기본: 쓰기 검토
        self.permission.add_policy("*", "*", Action::Write,
            TritPermission::Review, "기본 쓰기 검토");
        // 기본: 삭제 차단
        self.permission.add_policy("*", "*", Action::Delete,
            TritPermission::Deny, "기본 삭제 차단");
        // 기본: 실행 검토
        self.permission.add_policy("*", "*", Action::Execute,
            TritPermission::Review, "기본 실행 검토");
    }

    // ── 통합 API ──

    /// 권한 확인 후 태스크 실행 (커널의 핵심 흐름)
    /// 1. 권한 확인 (Permission)
    /// 2. 트랜잭션 시작 (Transaction)
    /// 3. 태스크 스케줄링 (Scheduler)
    /// 4. 실행 및 결과에 따라 commit/rollback
    pub fn execute_guarded(
        &mut self,
        subject: &str,
        object: &str,
        action: Action,
        task_name: &str,
        priority: TritPriority,
        task_fn: TaskFn,
    ) -> GuardedResult {
        self.total_ops += 1;

        // Step 1: 권한 확인
        let perm = self.permission.check(subject, object, action);

        if perm == TritPermission::Deny {
            return GuardedResult {
                permission: perm,
                tx_state: None,
                task_result: None,
                message: format!("차단: {}→{}.{}", subject, object, action),
            };
        }

        // Step 2: 트랜잭션 시작
        let tx_id = self.transaction.begin(task_name);

        // Step 3: 스케줄링
        let effective_priority = if perm == TritPermission::Review {
            // 검토 상태면 우선순위 한 단계 낮춤
            match priority {
                TritPriority::High => TritPriority::Normal,
                _ => TritPriority::Low,
            }
        } else {
            priority
        };

        self.scheduler.submit(task_name, effective_priority, task_fn);

        // Step 4: 실행
        let exec_result = self.scheduler.execute_one();

        // Step 5: 결과에 따라 commit/rollback
        let (task_result, tx_state) = match exec_result {
            Some((_, TritResult::Success)) => {
                let state = self.transaction.commit(tx_id).unwrap_or(TxState::RolledBack);
                (Some(TritResult::Success), Some(state))
            }
            Some((_, TritResult::Pending)) => {
                // 보류 → 트랜잭션도 보류 유지
                (Some(TritResult::Pending), Some(TxState::Pending))
            }
            Some((_, TritResult::Failed)) | None => {
                let state = self.transaction.rollback(tx_id).unwrap_or(TxState::RolledBack);
                (Some(TritResult::Failed), Some(state))
            }
        };

        GuardedResult {
            permission: perm,
            tx_state,
            task_result,
            message: format!("{}→{}.{}: 권한:{} TX:{} 결과:{}",
                subject, object, action, perm,
                tx_state.map(|s| format!("{}", s)).unwrap_or("N/A".into()),
                task_result.map(|r| format!("{}", r)).unwrap_or("N/A".into()),
            ),
        }
    }

    /// 간단한 태스크 실행 (권한 없이)
    pub fn execute_task(&mut self, name: &str, priority: TritPriority, action: TaskFn) -> TritResult {
        self.total_ops += 1;
        self.scheduler.submit(name, priority, action);
        match self.scheduler.execute_one() {
            Some((_, result)) => result,
            None => TritResult::Failed,
        }
    }

    /// TVM 프로그램 실행 (어셈블리 소스)
    pub fn execute_program(&mut self, source: &str) -> Result<(), String> {
        self.total_ops += 1;
        let program = crate::assembler::assemble(source);
        if program.is_empty() {
            return Err("프로그램이 비어있습니다".into());
        }
        self.vm.load(program);
        self.vm.run().map_err(|e| format!("{}", e))
    }

    /// 커널 종료
    pub fn shutdown(&mut self) {
        // 모든 활성 트랜잭션 롤백
        let active_txs: Vec<TxId> = self.transaction.active.keys().cloned().collect();
        for tx_id in active_txs {
            let _ = self.transaction.rollback(tx_id);
        }

        self.state = KernelState::Shutdown;
        if self.config.debug {
            eprintln!("[KERNEL] Crowny Meta-Kernel 종료");
        }
    }

    /// 전체 상태 덤프
    pub fn dump(&self) {
        println!("╔═══════════════════════════════════════════════════╗");
        println!("║          Crowny Meta-Kernel 상태                  ║");
        println!("║  커널: {} | 총 연산: {}                     ║",
            self.state, self.total_ops);
        println!("╠═══════════════════════════════════════════════════╣");
        self.scheduler.dump();
        self.permission.dump();
        self.transaction.dump();
        println!("║  TVM: IP={} 스택={} 힙={} 사이클={}",
            self.vm.ip, self.vm.stack.len(), self.vm.heap.alive_count(), self.vm.cycles);
        println!("╚═══════════════════════════════════════════════════╝");
    }
}

/// 보호된 실행 결과
#[derive(Debug)]
pub struct GuardedResult {
    pub permission: TritPermission,
    pub tx_state: Option<TxState>,
    pub task_result: Option<TritResult>,
    pub message: String,
}

impl std::fmt::Display for GuardedResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_boot() {
        let kernel = CrownyKernel::boot(KernelConfig::default());
        assert_eq!(kernel.state, KernelState::Running);
    }

    #[test]
    fn test_guarded_execution() {
        let mut kernel = CrownyKernel::boot(KernelConfig::default());

        // 읽기 → 허용
        let result = kernel.execute_guarded(
            "사용자", "데이터", Action::Read,
            "데이터읽기", TritPriority::Normal,
            Box::new(|| TritResult::Success),
        );
        assert_eq!(result.permission, TritPermission::Allow);
        assert_eq!(result.task_result, Some(TritResult::Success));

        // 삭제 → 차단
        let result = kernel.execute_guarded(
            "사용자", "데이터", Action::Delete,
            "데이터삭제", TritPriority::High,
            Box::new(|| TritResult::Success),
        );
        assert_eq!(result.permission, TritPermission::Deny);
        assert_eq!(result.task_result, None); // 실행 안 됨
    }

    #[test]
    fn test_kernel_shutdown() {
        let mut kernel = CrownyKernel::boot(KernelConfig::default());
        kernel.shutdown();
        assert_eq!(kernel.state, KernelState::Shutdown);
    }
}
