// ═══════════════════════════════════════════════════════════════
// Crowny Smart Contract VM — 3진 스마트 컨트랙트
// 배포 · 실행 · 스토리지 · 가스 · 이벤트 · ABI
// ═══════════════════════════════════════════════════════════════

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
fn trit_hash(data: &str) -> String {
    let mut h: u64 = 0xcb735a4e9f1d2b08;
    for (i, b) in data.bytes().enumerate() {
        h ^= (b as u64).wrapping_mul(0x100000001b3);
        h = h.wrapping_mul(0x517cc1b727220a95);
        h ^= (i as u64).wrapping_add(0x9e3779b97f4a7c15);
        h = h.rotate_left(17) ^ h.rotate_right(23);
    }
    let trits: String = (0..27).map(|i| match ((h >> (i * 2)) & 3) % 3 { 0 => 'P', 1 => 'O', _ => 'T' }).collect();
    format!("0t{}", trits)
}

// ── 옵코드 ──
#[derive(Debug, Clone, PartialEq)]
pub enum COP {
    Push(i64), Pop, Dup, Swap,
    TAdd, TSub, TMul, TDiv, TMod,
    TAnd, TOr, TNot, TCmp,
    SLoad(String), SStore(String),
    Jump(usize), JumpIf(usize), JumpIfNot(usize),
    Call(String), Return,
    Caller, SelfAddr, Balance, BlockHeight, Timestamp,
    Emit(String), Transfer, TritVote, ConsensusCheck,
    Halt, Revert(String), Nop,
}

impl COP {
    pub fn gas_cost(&self) -> u64 {
        match self {
            Self::Push(_)|Self::Pop|Self::Dup|Self::Swap|Self::Nop => 3,
            Self::TAdd|Self::TSub|Self::TMul|Self::TMod => 9,
            Self::TDiv => 15,
            Self::TAnd|Self::TOr|Self::TNot|Self::TCmp => 6,
            Self::SLoad(_) => 200, Self::SStore(_) => 500,
            Self::Jump(_)|Self::JumpIf(_)|Self::JumpIfNot(_) => 12,
            Self::Call(_) => 100, Self::Return => 6,
            Self::Caller|Self::SelfAddr|Self::Balance|Self::BlockHeight|Self::Timestamp => 6,
            Self::Emit(_) => 100, Self::Transfer => 2100,
            Self::TritVote => 300, Self::ConsensusCheck => 200,
            Self::Halt|Self::Revert(_) => 0,
        }
    }
}

// ── ABI ──
#[derive(Debug, Clone, PartialEq)]
pub enum ABIType { Int, Trit, Address, Bool, String_ }
#[derive(Debug, Clone, PartialEq)]
pub enum Mutability { Pure, View, Payable, NonPayable }

#[derive(Debug, Clone)]
pub struct ABIFunc {
    pub name: String,
    pub inputs: Vec<(String, ABIType)>,
    pub outputs: Vec<ABIType>,
    pub mutability: Mutability,
    pub entry_pc: usize,
}
impl std::fmt::Display for ABIFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ins: Vec<String> = self.inputs.iter().map(|(n,t)| format!("{}:{:?}", n, t)).collect();
        let outs: Vec<String> = self.outputs.iter().map(|t| format!("{:?}", t)).collect();
        let m = match self.mutability { Mutability::Pure=>"pure", Mutability::View=>"view", Mutability::Payable=>"payable", Mutability::NonPayable=>"" };
        write!(f, "{}({}) → ({}) {}", self.name, ins.join(","), outs.join(","), m)
    }
}

// ── 컨트랙트 ──
#[derive(Debug, Clone)]
pub struct Contract {
    pub address: String, pub owner: String, pub name: String,
    pub code: Vec<COP>, pub abi: Vec<ABIFunc>,
    pub storage: HashMap<String, i64>, pub balance: u64,
    pub call_count: u64, pub total_gas: u64, pub trit_state: i8,
    pub deployed_at: u64,
}
impl Contract {
    pub fn new(name: &str, owner: &str, code: Vec<COP>, abi: Vec<ABIFunc>) -> Self {
        Self { address: trit_hash(&format!("c:{}:{}:{}", name, owner, now_ms())),
            owner: owner.into(), name: name.into(), code, abi,
            storage: HashMap::new(), balance: 0,
            call_count: 0, total_gas: 0, trit_state: 1, deployed_at: now_ms() }
    }
    pub fn find_fn(&self, name: &str) -> Option<&ABIFunc> { self.abi.iter().find(|f| f.name == name) }
}
impl std::fmt::Display for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self.trit_state { 1=>"P", -1=>"T", _=>"O" };
        let a: String = self.address.chars().take(16).collect();
        write!(f, "[{}] {} — {} | {}ops | {}calls | gas:{}", t, self.name, a, self.code.len(), self.call_count, self.total_gas)
    }
}

// ── 실행 결과 ──
#[derive(Debug, Clone)]
pub struct CEvent { pub name: String, pub data: Vec<i64>, pub ts: u64 }
impl std::fmt::Display for CEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d: Vec<String> = self.data.iter().map(|v| v.to_string()).collect();
        write!(f, "📣 {}({})", self.name, d.join(","))
    }
}

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub success: bool, pub ret: Option<i64>, pub gas: u64,
    pub events: Vec<CEvent>, pub writes: Vec<(String, i64)>,
    pub error: Option<String>, pub trit: i8,
}
impl std::fmt::Display for ExecResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = match self.trit { 1=>"P", -1=>"T", _=>"O" };
        if self.success {
            let v = self.ret.map(|v| format!(" → {}", v)).unwrap_or_default();
            write!(f, "[{}] 성공{} | gas:{} | events:{} | writes:{}", t, v, self.gas, self.events.len(), self.writes.len())
        } else {
            write!(f, "[T] 실패: {} | gas:{}", self.error.as_deref().unwrap_or("?"), self.gas)
        }
    }
}

pub struct ExecCtx {
    pub caller: String, pub value: u64, pub block_h: u64, pub gas_limit: u64, pub args: Vec<i64>,
}

// ── VM ──
pub struct ContractVM {
    pub contracts: HashMap<String, Contract>,
    pub balances: HashMap<String, u64>,
    pub block_h: u64, pub deploys: u64, pub total_gas: u64,
    pub events: Vec<(String, CEvent)>,
}

impl ContractVM {
    pub fn new() -> Self {
        Self { contracts: HashMap::new(), balances: HashMap::new(), block_h: 3, deploys: 0, total_gas: 0, events: Vec::new() }
    }
    pub fn fund(&mut self, a: &str, v: u64) { *self.balances.entry(a.into()).or_insert(0) += v; }
    pub fn balance(&self, a: &str) -> u64 { self.balances.get(a).copied().unwrap_or(0) }

    pub fn deploy(&mut self, name: &str, owner: &str, code: Vec<COP>, abi: Vec<ABIFunc>) -> String {
        let c = Contract::new(name, owner, code, abi);
        let addr = c.address.clone();
        self.contracts.insert(addr.clone(), c);
        self.deploys += 1;
        addr
    }

    pub fn call(&mut self, addr: &str, func: &str, ctx: ExecCtx) -> ExecResult {
        let fail = |e: &str| ExecResult { success: false, ret: None, gas: 0, events: vec![], writes: vec![], error: Some(e.into()), trit: -1 };
        let contract = match self.contracts.get(addr) { Some(c) => c.clone(), None => return fail("컨트랙트 없음") };
        let entry = match contract.find_fn(func) { Some(f) => f.entry_pc, None => return fail(&format!("함수 없음: {}", func)) };

        let mut stack: Vec<i64> = Vec::new();
        let mut pc = entry;
        let mut gas = 0u64;
        let mut evts = Vec::new();
        let mut writes = Vec::new();
        let mut stor = contract.storage.clone();

        for arg in ctx.args.iter().rev() { stack.push(*arg); }

        loop {
            if pc >= contract.code.len() { break; }
            let op = &contract.code[pc];
            gas += op.gas_cost();
            if gas > ctx.gas_limit {
                return ExecResult { success: false, ret: None, gas, events: evts, writes, error: Some("가스 한도 초과".into()), trit: -1 };
            }
            match op {
                COP::Push(v) => stack.push(*v),
                COP::Pop => { stack.pop(); }
                COP::Dup => { if let Some(&v) = stack.last() { stack.push(v); } }
                COP::Swap => { let l = stack.len(); if l >= 2 { stack.swap(l-1, l-2); } }
                COP::TAdd => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); stack.push(a+b); } }
                COP::TSub => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); stack.push(a-b); } }
                COP::TMul => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); stack.push(a*b); } }
                COP::TDiv => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap();
                    if b==0 { return ExecResult { success:false, ret:None, gas, events:evts, writes, error:Some("0 나누기".into()), trit:-1 }; }
                    stack.push(a/b); } }
                COP::TMod => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); if b!=0 { stack.push(a%b); } } }
                COP::TAnd => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); stack.push(a.min(b)); } }
                COP::TOr => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap(); stack.push(a.max(b)); } }
                COP::TNot => { if let Some(v) = stack.pop() { stack.push(-v); } }
                COP::TCmp => { if stack.len()>=2 { let b=stack.pop().unwrap(); let a=stack.pop().unwrap();
                    stack.push(if a>b {1} else if a<b {-1} else {0}); } }
                COP::SLoad(k) => { stack.push(stor.get(k).copied().unwrap_or(0)); }
                COP::SStore(k) => { if let Some(v)=stack.pop() { stor.insert(k.clone(),v); writes.push((k.clone(),v)); } }
                COP::Jump(t) => { pc=*t; continue; }
                COP::JumpIf(t) => { if let Some(v)=stack.pop() { if v>0 { pc=*t; continue; } } }
                COP::JumpIfNot(t) => { if let Some(v)=stack.pop() { if v<0 { pc=*t; continue; } } }
                COP::Caller => { stack.push(ctx.caller.len() as i64); }
                COP::SelfAddr => { stack.push(contract.address.len() as i64); }
                COP::Balance => { stack.push(self.balance(&contract.address) as i64); }
                COP::BlockHeight => { stack.push(self.block_h as i64); }
                COP::Timestamp => { stack.push(now_ms() as i64); }
                COP::Emit(n) => { evts.push(CEvent { name: n.clone(), data: stack.clone(), ts: now_ms() }); }
                COP::Transfer => { if stack.len()>=2 { let amt=stack.pop().unwrap(); stack.pop();
                    evts.push(CEvent { name:"Transfer".into(), data:vec![amt], ts:now_ms() }); } }
                COP::TritVote => { if let Some(&v) = stack.last() {
                    let t = if v>0{1} else if v<0{-1} else {0};
                    evts.push(CEvent { name:"TritVote".into(), data:vec![t], ts:now_ms() }); } }
                COP::ConsensusCheck => { let r: Vec<i64> = stack.iter().rev().take(3).copied().collect();
                    let p=r.iter().filter(|&&v|v>0).count(); let t=r.iter().filter(|&&v|v<0).count();
                    let c = if p>t{1} else if t>p{-1} else {0};
                    stack.push(c); evts.push(CEvent { name:"Consensus".into(), data:vec![c], ts:now_ms() }); }
                COP::Halt => break,
                COP::Revert(m) => { return ExecResult { success:false, ret:None, gas, events:evts, writes:vec![], error:Some(m.clone()), trit:-1 }; }
                COP::Return => break,
                _ => {}
            }
            pc += 1;
        }

        let cm = self.contracts.get_mut(addr).unwrap();
        cm.storage = stor; cm.call_count += 1; cm.total_gas += gas;
        self.total_gas += gas;
        for e in &evts { self.events.push((addr.into(), e.clone())); }

        let ret = stack.last().copied();
        let trit = if ret.map(|v|v>0).unwrap_or(false) {1} else if ret.map(|v|v<0).unwrap_or(false) {-1} else {0};
        ExecResult { success:true, ret, gas, events:evts, writes, error:None, trit }
    }

    pub fn summary(&self) -> String {
        format!("ContractVM\n  컨트랙트:{} | 배포:{} | 가스:{} | 이벤트:{} | 블록:{}",
            self.contracts.len(), self.deploys, self.total_gas, self.events.len(), self.block_h)
    }
}

// ── 프리셋 컨트랙트 ──
pub fn token_contract() -> (Vec<COP>, Vec<ABIFunc>) {
    let code = vec![
        COP::Push(153_000_000), COP::SStore("total_supply".into()), COP::Emit("Init".into()), COP::Return, // 0: init
        COP::SLoad("total_supply".into()), COP::Return, // 4: totalSupply
        COP::SLoad("balance".into()), COP::Return, // 6: balanceOf
        COP::SLoad("balance".into()), COP::Push(0), COP::TCmp, COP::Emit("Transfer".into()), COP::Push(1), COP::Return, // 8: transfer
        COP::SLoad("total_supply".into()), COP::TAdd, COP::SStore("total_supply".into()), COP::Emit("Mint".into()), COP::Return, // 14: mint
    ];
    let abi = vec![
        ABIFunc { name:"init".into(), inputs:vec![], outputs:vec![], mutability:Mutability::NonPayable, entry_pc:0 },
        ABIFunc { name:"totalSupply".into(), inputs:vec![], outputs:vec![ABIType::Int], mutability:Mutability::View, entry_pc:4 },
        ABIFunc { name:"balanceOf".into(), inputs:vec![("addr".into(),ABIType::Address)], outputs:vec![ABIType::Int], mutability:Mutability::View, entry_pc:6 },
        ABIFunc { name:"transfer".into(), inputs:vec![("to".into(),ABIType::Address),("amt".into(),ABIType::Int)], outputs:vec![ABIType::Bool], mutability:Mutability::NonPayable, entry_pc:8 },
        ABIFunc { name:"mint".into(), inputs:vec![("amt".into(),ABIType::Int)], outputs:vec![], mutability:Mutability::NonPayable, entry_pc:14 },
    ];
    (code, abi)
}

pub fn voting_contract() -> (Vec<COP>, Vec<ABIFunc>) {
    let code = vec![
        COP::Push(0), COP::SStore("votes_p".into()), COP::Push(0), COP::SStore("votes_o".into()),
        COP::Push(0), COP::SStore("votes_t".into()), COP::Push(1), COP::SStore("active".into()),
        COP::Emit("ProposalCreated".into()), COP::Return, // 0: createProposal
        COP::Dup, COP::TritVote, COP::SLoad("votes_p".into()), COP::Push(1), COP::TAdd,
        COP::SStore("votes_p".into()), COP::Emit("Voted".into()), COP::Return, // 10: vote
        COP::SLoad("votes_p".into()), COP::SLoad("votes_t".into()), COP::TCmp,
        COP::Emit("Result".into()), COP::Return, // 18: getResult
    ];
    let abi = vec![
        ABIFunc { name:"createProposal".into(), inputs:vec![], outputs:vec![], mutability:Mutability::NonPayable, entry_pc:0 },
        ABIFunc { name:"vote".into(), inputs:vec![("trit".into(),ABIType::Trit)], outputs:vec![], mutability:Mutability::NonPayable, entry_pc:10 },
        ABIFunc { name:"getResult".into(), inputs:vec![], outputs:vec![ABIType::Trit], mutability:Mutability::View, entry_pc:18 },
    ];
    (code, abi)
}

pub fn escrow_contract() -> (Vec<COP>, Vec<ABIFunc>) {
    let code = vec![
        // 0-6: deposit
        COP::SLoad("deposited".into()), COP::TAdd, COP::SStore("deposited".into()),
        COP::Push(0), COP::SStore("released".into()), COP::Emit("Deposit".into()), COP::Return,
        // 7-15: approve
        COP::SLoad("approvals".into()), COP::Push(1), COP::TAdd, COP::Dup,
        COP::SStore("approvals".into()), COP::Push(2), COP::TCmp,
        COP::Emit("Approved".into()), COP::Return,
        // 16-24: release
        COP::SLoad("approvals".into()), COP::Push(2), COP::TCmp, COP::Push(0), COP::TCmp,
        COP::JumpIfNot(25), COP::SLoad("deposited".into()), COP::Emit("Released".into()), COP::Return,
        // 25: revert
        COP::Revert("승인 부족".into()),
        // 26-29: getStatus
        COP::SLoad("deposited".into()), COP::SLoad("approvals".into()), COP::Emit("Status".into()), COP::Return,
    ];
    let abi = vec![
        ABIFunc { name:"deposit".into(), inputs:vec![("amt".into(),ABIType::Int)], outputs:vec![], mutability:Mutability::Payable, entry_pc:0 },
        ABIFunc { name:"approve".into(), inputs:vec![], outputs:vec![ABIType::Trit], mutability:Mutability::NonPayable, entry_pc:7 },
        ABIFunc { name:"release".into(), inputs:vec![], outputs:vec![ABIType::Int], mutability:Mutability::NonPayable, entry_pc:16 },
        ABIFunc { name:"getStatus".into(), inputs:vec![], outputs:vec![ABIType::Int], mutability:Mutability::View, entry_pc:26 },
    ];
    (code, abi)
}

pub fn consensus_contract() -> (Vec<COP>, Vec<ABIFunc>) {
    let code = vec![
        COP::TritVote, COP::SLoad("count".into()), COP::Push(1), COP::TAdd,
        COP::SStore("count".into()), COP::Emit("VoteSubmitted".into()), COP::Return, // 0: submit
        COP::SLoad("count".into()), COP::Push(3), COP::TCmp, COP::ConsensusCheck,
        COP::Emit("ConsensusResult".into()), COP::Return, // 7: check
    ];
    let abi = vec![
        ABIFunc { name:"submit".into(), inputs:vec![("vote".into(),ABIType::Trit)], outputs:vec![], mutability:Mutability::NonPayable, entry_pc:0 },
        ABIFunc { name:"check".into(), inputs:vec![], outputs:vec![ABIType::Trit], mutability:Mutability::View, entry_pc:7 },
    ];
    (code, abi)
}

// ═══ 데모 ═══
pub fn demo_contract_vm() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny Smart Contract VM — 3진 스마트 컨트랙트  ║");
    println!("║  배포 · 실행 · 스토리지 · 가스 · 이벤트 · ABI    ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut vm = ContractVM::new();
    vm.fund("alice", 1_000_000); vm.fund("bob", 500_000); vm.fund("carol", 300_000);
    let ctx = |c: &str, a: Vec<i64>| ExecCtx { caller:c.into(), value:0, block_h:3, gas_limit:100_000, args:a };

    // 1. Token
    println!("━━━ 1. CRWN 토큰 컨트랙트 ━━━");
    let (code, abi) = token_contract();
    println!("  ABI:"); for f in &abi { println!("    {}", f); }
    let ta = vm.deploy("CRWNToken", "alice", code, abi);
    println!("  배포: {}", vm.contracts[&ta]);
    println!("  init: {}", vm.call(&ta, "init", ctx("alice", vec![])));
    println!("  totalSupply: {}", vm.call(&ta, "totalSupply", ctx("alice", vec![])));
    println!("  transfer: {}", vm.call(&ta, "transfer", ctx("alice", vec![100, 50000])));
    println!("  mint: {}", vm.call(&ta, "mint", ctx("alice", vec![1_000_000])));
    println!();

    // 2. Voting
    println!("━━━ 2. DAO 투표 컨트랙트 ━━━");
    let (code, abi) = voting_contract();
    let va = vm.deploy("CrownyDAO", "alice", code, abi);
    println!("  배포: {}", vm.contracts[&va]);
    println!("  create: {}", vm.call(&va, "createProposal", ctx("alice", vec![])));
    println!("  vote(P) alice: {}", vm.call(&va, "vote", ctx("alice", vec![1])));
    println!("  vote(P) bob: {}", vm.call(&va, "vote", ctx("bob", vec![1])));
    println!("  vote(T) carol: {}", vm.call(&va, "vote", ctx("carol", vec![-1])));
    println!("  result: {}", vm.call(&va, "getResult", ctx("alice", vec![])));
    println!();

    // 3. Escrow
    println!("━━━ 3. 에스크로 컨트랙트 ━━━");
    let (code, abi) = escrow_contract();
    let ea = vm.deploy("CRWNEscrow", "alice", code, abi);
    println!("  배포: {}", vm.contracts[&ea]);
    println!("  deposit(100K): {}", vm.call(&ea, "deposit", ctx("bob", vec![100_000])));
    println!("  approve(alice): {}", vm.call(&ea, "approve", ctx("alice", vec![])));
    println!("  approve(carol): {}", vm.call(&ea, "approve", ctx("carol", vec![])));
    println!("  release: {}", vm.call(&ea, "release", ctx("alice", vec![])));
    println!("  status: {}", vm.call(&ea, "getStatus", ctx("alice", vec![])));
    println!();

    // 4. Consensus
    println!("━━━ 4. 온체인 합의 컨트랙트 ━━━");
    let (code, abi) = consensus_contract();
    let ca = vm.deploy("TritConsensus", "alice", code, abi);
    println!("  배포: {}", vm.contracts[&ca]);
    println!("  submit(P): {}", vm.call(&ca, "submit", ctx("alice", vec![1])));
    println!("  submit(P): {}", vm.call(&ca, "submit", ctx("bob", vec![1])));
    println!("  submit(T): {}", vm.call(&ca, "submit", ctx("carol", vec![-1])));
    println!("  check: {}", vm.call(&ca, "check", ctx("alice", vec![])));
    println!();

    // 5. Gas test
    println!("━━━ 5. 에러 테스트 ━━━");
    println!("  gas=10: {}", vm.call(&ta, "totalSupply", ExecCtx { caller:"a".into(), value:0, block_h:3, gas_limit:10, args:vec![] }));
    println!("  bad func: {}", vm.call(&ta, "xxx", ctx("a", vec![])));
    println!("  bad addr: {}", vm.call("fake", "x", ctx("a", vec![])));
    println!();

    // 6. Events
    println!("━━━ 6. 이벤트 로그 (최근 10) ━━━");
    for (a, e) in vm.events.iter().rev().take(10) {
        println!("  {}.. — {}", &a.chars().take(12).collect::<String>(), e);
    }
    println!();

    // 7. Contracts
    println!("━━━ 7. 컨트랙트 현황 ━━━");
    for c in vm.contracts.values() { println!("  {}", c); }
    println!();
    println!("━━━ 8. 요약 ━━━");
    println!("{}", vm.summary());
    println!("\n✓ Crowny Smart Contract VM 데모 완료");
}

// ═══ 테스트 ═══
#[cfg(test)]
mod tests {
    use super::*;
    fn tctx(c: &str, a: Vec<i64>) -> ExecCtx { ExecCtx { caller:c.into(), value:0, block_h:3, gas_limit:100_000, args:a } }

    #[test] fn test_deploy() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract();
        let addr = vm.deploy("T","alice",c,a);
        assert!(vm.contracts.contains_key(&addr));
    }
    #[test] fn test_token_init() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        let r = vm.call(&addr, "init", tctx("alice", vec![]));
        assert!(r.success); assert_eq!(vm.contracts[&addr].storage["total_supply"], 153_000_000);
    }
    #[test] fn test_total_supply() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        vm.call(&addr, "init", tctx("alice", vec![]));
        let r = vm.call(&addr, "totalSupply", tctx("alice", vec![]));
        assert_eq!(r.ret, Some(153_000_000));
    }
    #[test] fn test_gas_exceeded() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        let r = vm.call(&addr, "init", ExecCtx { caller:"a".into(), value:0, block_h:3, gas_limit:5, args:vec![] });
        assert!(!r.success);
    }
    #[test] fn test_fn_not_found() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        assert!(!vm.call(&addr, "xxx", tctx("a",vec![])).success);
    }
    #[test] fn test_contract_not_found() {
        let mut vm = ContractVM::new();
        assert!(!vm.call("fake","x", tctx("a",vec![])).success);
    }
    #[test] fn test_voting() {
        let mut vm = ContractVM::new();
        let (c,a) = voting_contract(); let addr = vm.deploy("V","alice",c,a);
        vm.call(&addr, "createProposal", tctx("alice", vec![]));
        assert_eq!(vm.contracts[&addr].storage["active"], 1);
    }
    #[test] fn test_escrow_flow() {
        let mut vm = ContractVM::new();
        let (c,a) = escrow_contract(); let addr = vm.deploy("E","alice",c,a);
        vm.call(&addr, "deposit", tctx("bob", vec![50000]));
        assert_eq!(vm.contracts[&addr].storage["deposited"], 50000);
        vm.call(&addr, "approve", tctx("alice", vec![]));
        vm.call(&addr, "approve", tctx("carol", vec![]));
        assert_eq!(vm.contracts[&addr].storage["approvals"], 2);
    }
    #[test] fn test_escrow_revert() {
        let mut vm = ContractVM::new();
        let (c,a) = escrow_contract(); let addr = vm.deploy("E","alice",c,a);
        vm.call(&addr, "deposit", tctx("bob", vec![50000]));
        let r = vm.call(&addr, "release", tctx("alice", vec![]));
        assert!(!r.success);
    }
    #[test] fn test_consensus() {
        let mut vm = ContractVM::new();
        let (c,a) = consensus_contract(); let addr = vm.deploy("C","alice",c,a);
        vm.call(&addr, "submit", tctx("alice", vec![1]));
        vm.call(&addr, "submit", tctx("bob", vec![1]));
        let r = vm.call(&addr, "check", tctx("alice", vec![]));
        assert!(r.success); assert!(r.events.iter().any(|e| e.name == "ConsensusResult"));
    }
    #[test] fn test_events() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        let r = vm.call(&addr, "init", tctx("alice", vec![]));
        assert!(!r.events.is_empty());
    }
    #[test] fn test_gas_tracking() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        vm.call(&addr, "init", tctx("alice", vec![]));
        assert!(vm.contracts[&addr].total_gas > 0); assert!(vm.total_gas > 0);
    }
    #[test] fn test_op_costs() { assert_eq!(COP::Push(0).gas_cost(), 3); assert_eq!(COP::SStore("x".into()).gas_cost(), 500); }
    #[test] fn test_call_count() {
        let mut vm = ContractVM::new();
        let (c,a) = token_contract(); let addr = vm.deploy("T","alice",c,a);
        vm.call(&addr, "init", tctx("a",vec![]));
        vm.call(&addr, "totalSupply", tctx("a",vec![]));
        assert_eq!(vm.contracts[&addr].call_count, 2);
    }
    #[test] fn test_div_zero() {
        let mut vm = ContractVM::new();
        let code = vec![COP::Push(10), COP::Push(0), COP::TDiv, COP::Return];
        let abi = vec![ABIFunc { name:"test".into(), inputs:vec![], outputs:vec![], mutability:Mutability::Pure, entry_pc:0 }];
        let addr = vm.deploy("DivTest","alice",code,abi);
        let r = vm.call(&addr, "test", tctx("a",vec![]));
        assert!(!r.success);
    }
}
