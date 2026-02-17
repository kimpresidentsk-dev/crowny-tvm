// ═══════════════════════════════════════════════════════════════
// Crowny DEX — 3진 탈중앙 거래소
// AMM(자동 시장 조성) · 유동성 풀 · 오더북 · 스왑 · LP 보상
// 모든 거래에 P/O/T 상태 + CTP 9-trit 헤더
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

// ═══════════════════════════════════════
// 토큰
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Token {
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub trit_state: i8,
}

impl Token {
    pub fn new(symbol: &str, name: &str, supply: u64) -> Self {
        Self { symbol: symbol.into(), name: name.into(), decimals: 9, total_supply: supply, trit_state: 1 }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}) — supply: {}", self.symbol, self.name, self.total_supply)
    }
}

// ═══════════════════════════════════════
// 유동성 풀 (AMM: x * y = k)
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct LiquidityPool {
    pub id: String,
    pub token_a: String,
    pub token_b: String,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub k: u128,                    // x * y = k 불변량
    pub fee_bps: u64,               // 수수료 (basis points, 30 = 0.3%)
    pub total_lp_shares: u64,       // LP 토큰 총량
    pub lp_holders: HashMap<String, u64>,
    pub volume_24h: u64,
    pub fees_collected: u64,
    pub swap_count: u64,
    pub trit_state: i8,
    pub created_at: u64,
}

impl LiquidityPool {
    pub fn new(token_a: &str, token_b: &str, fee_bps: u64) -> Self {
        let id = format!("{}-{}", token_a, token_b);
        Self {
            id: id.clone(), token_a: token_a.into(), token_b: token_b.into(),
            reserve_a: 0, reserve_b: 0, k: 0, fee_bps,
            total_lp_shares: 0, lp_holders: HashMap::new(),
            volume_24h: 0, fees_collected: 0, swap_count: 0,
            trit_state: 0, created_at: now_ms(),
        }
    }

    /// 유동성 추가
    pub fn add_liquidity(&mut self, provider: &str, amount_a: u64, amount_b: u64) -> LPReceipt {
        let shares = if self.total_lp_shares == 0 {
            // 최초 공급: sqrt(a * b)
            ((amount_a as f64 * amount_b as f64).sqrt()) as u64
        } else {
            // 기존 비율 기준
            let share_a = amount_a as u128 * self.total_lp_shares as u128 / self.reserve_a as u128;
            let share_b = amount_b as u128 * self.total_lp_shares as u128 / self.reserve_b as u128;
            share_a.min(share_b) as u64
        };

        self.reserve_a += amount_a;
        self.reserve_b += amount_b;
        self.k = self.reserve_a as u128 * self.reserve_b as u128;
        self.total_lp_shares += shares;
        *self.lp_holders.entry(provider.into()).or_insert(0) += shares;
        self.trit_state = 1;

        LPReceipt {
            pool_id: self.id.clone(), provider: provider.into(),
            amount_a, amount_b, shares_minted: shares,
            action: LPAction::Add, trit: 1, timestamp: now_ms(),
        }
    }

    /// 유동성 제거
    pub fn remove_liquidity(&mut self, provider: &str, shares: u64) -> Result<LPReceipt, String> {
        let held = self.lp_holders.get(provider).copied().unwrap_or(0);
        if held < shares { return Err("LP 지분 부족".into()); }

        let amount_a = (shares as u128 * self.reserve_a as u128 / self.total_lp_shares as u128) as u64;
        let amount_b = (shares as u128 * self.reserve_b as u128 / self.total_lp_shares as u128) as u64;

        self.reserve_a -= amount_a;
        self.reserve_b -= amount_b;
        self.k = self.reserve_a as u128 * self.reserve_b as u128;
        self.total_lp_shares -= shares;
        *self.lp_holders.get_mut(provider).unwrap() -= shares;

        Ok(LPReceipt {
            pool_id: self.id.clone(), provider: provider.into(),
            amount_a, amount_b, shares_minted: shares,
            action: LPAction::Remove, trit: 1, timestamp: now_ms(),
        })
    }

    /// 스왑 (A → B)
    pub fn swap_a_to_b(&mut self, amount_in: u64) -> Result<SwapResult, String> {
        if self.reserve_a == 0 || self.reserve_b == 0 { return Err("유동성 없음".into()); }

        let fee = amount_in * self.fee_bps / 10000;
        let amount_after_fee = amount_in - fee;

        // x * y = k → new_y = k / new_x
        let new_reserve_a = self.reserve_a as u128 + amount_after_fee as u128;
        let new_reserve_b = self.k / new_reserve_a;
        let amount_out = self.reserve_b as u128 - new_reserve_b;

        if amount_out == 0 { return Err("출력량 0".into()); }

        let price_impact = 1.0 - (new_reserve_b as f64 * self.reserve_a as f64) / (self.reserve_b as f64 * new_reserve_a as f64);

        self.reserve_a = new_reserve_a as u64;
        self.reserve_b = new_reserve_b as u64;
        self.k = self.reserve_a as u128 * self.reserve_b as u128;
        self.fees_collected += fee;
        self.volume_24h += amount_in;
        self.swap_count += 1;

        Ok(SwapResult {
            pool_id: self.id.clone(),
            token_in: self.token_a.clone(), token_out: self.token_b.clone(),
            amount_in, amount_out: amount_out as u64, fee,
            price_impact, trit: if price_impact < 0.01 { 1 } else if price_impact < 0.05 { 0 } else { -1 },
            hash: trit_hash(&format!("swap:{}:{}:{}", self.id, amount_in, now_ms())),
            timestamp: now_ms(),
        })
    }

    /// 스왑 (B → A)
    pub fn swap_b_to_a(&mut self, amount_in: u64) -> Result<SwapResult, String> {
        if self.reserve_a == 0 || self.reserve_b == 0 { return Err("유동성 없음".into()); }

        let fee = amount_in * self.fee_bps / 10000;
        let amount_after_fee = amount_in - fee;

        let new_reserve_b = self.reserve_b as u128 + amount_after_fee as u128;
        let new_reserve_a = self.k / new_reserve_b;
        let amount_out = self.reserve_a as u128 - new_reserve_a;

        if amount_out == 0 { return Err("출력량 0".into()); }

        let price_impact = 1.0 - (new_reserve_a as f64 * self.reserve_b as f64) / (self.reserve_a as f64 * new_reserve_b as f64);

        self.reserve_a = new_reserve_a as u64;
        self.reserve_b = new_reserve_b as u64;
        self.k = self.reserve_a as u128 * self.reserve_b as u128;
        self.fees_collected += fee;
        self.volume_24h += amount_in;
        self.swap_count += 1;

        Ok(SwapResult {
            pool_id: self.id.clone(),
            token_in: self.token_b.clone(), token_out: self.token_a.clone(),
            amount_in, amount_out: amount_out as u64, fee,
            price_impact, trit: if price_impact < 0.01 { 1 } else if price_impact < 0.05 { 0 } else { -1 },
            hash: trit_hash(&format!("swap:{}:{}:{}", self.id, amount_in, now_ms())),
            timestamp: now_ms(),
        })
    }

    /// 현재 가격 (A 기준 B)
    pub fn price_a_in_b(&self) -> f64 {
        if self.reserve_a == 0 { return 0.0; }
        self.reserve_b as f64 / self.reserve_a as f64
    }

    pub fn price_b_in_a(&self) -> f64 {
        if self.reserve_b == 0 { return 0.0; }
        self.reserve_a as f64 / self.reserve_b as f64
    }

    /// TVL (Total Value Locked)
    pub fn tvl(&self, price_a_usd: f64, price_b_usd: f64) -> f64 {
        self.reserve_a as f64 * price_a_usd + self.reserve_b as f64 * price_b_usd
    }

    /// APR 추정 (연간 수수료 수익률)
    pub fn estimated_apr(&self, price_a_usd: f64, price_b_usd: f64) -> f64 {
        let tvl = self.tvl(price_a_usd, price_b_usd);
        if tvl == 0.0 { return 0.0; }
        // 24h 수수료 * 365
        let daily_fees = self.fees_collected as f64 * price_a_usd;
        (daily_fees * 365.0 / tvl) * 100.0
    }
}

impl std::fmt::Display for LiquidityPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit_state { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} — {}/{} | price:{:.6} | swaps:{} | fees:{}",
            trit, self.id, self.reserve_a, self.reserve_b,
            self.price_a_in_b(), self.swap_count, self.fees_collected)
    }
}

// ═══════════════════════════════════════
// 스왑/LP 결과
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub pool_id: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub price_impact: f64,
    pub trit: i8,
    pub hash: String,
    pub timestamp: u64,
}

impl std::fmt::Display for SwapResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let trit = match self.trit { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} {} → {} {} (fee:{}, impact:{:.2}%)",
            trit, self.amount_in, self.token_in, self.amount_out, self.token_out,
            self.fee, self.price_impact * 100.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LPAction { Add, Remove }

#[derive(Debug, Clone)]
pub struct LPReceipt {
    pub pool_id: String,
    pub provider: String,
    pub amount_a: u64,
    pub amount_b: u64,
    pub shares_minted: u64,
    pub action: LPAction,
    pub trit: i8,
    pub timestamp: u64,
}

impl std::fmt::Display for LPReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let act = if self.action == LPAction::Add { "추가" } else { "제거" };
        write!(f, "[P] LP {} — {} +{}/{} → {} shares",
            act, self.provider, self.amount_a, self.amount_b, self.shares_minted)
    }
}

// ═══════════════════════════════════════
// 오더북 (리밋 주문)
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum OrderSide { Buy, Sell }

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStatus { Open, Filled, PartialFill, Cancelled }

#[derive(Debug, Clone)]
pub struct LimitOrder {
    pub id: String,
    pub owner: String,
    pub pool_id: String,
    pub side: OrderSide,
    pub price: f64,
    pub amount: u64,
    pub filled: u64,
    pub status: OrderStatus,
    pub trit: i8,
    pub created_at: u64,
}

impl std::fmt::Display for LimitOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let side = if self.side == OrderSide::Buy { "매수" } else { "매도" };
        let status = match &self.status {
            OrderStatus::Open => "대기", OrderStatus::Filled => "체결",
            OrderStatus::PartialFill => "부분체결", OrderStatus::Cancelled => "취소",
        };
        let trit = match self.trit { 1 => "P", -1 => "T", _ => "O" };
        write!(f, "[{}] {} {} @ {:.6} — {}/{} ({})",
            trit, side, self.pool_id, self.price, self.filled, self.amount, status)
    }
}

pub struct OrderBook {
    pub orders: Vec<LimitOrder>,
    pub order_counter: u64,
}

impl OrderBook {
    pub fn new() -> Self { Self { orders: Vec::new(), order_counter: 0 } }

    pub fn place_order(&mut self, owner: &str, pool_id: &str, side: OrderSide, price: f64, amount: u64) -> &LimitOrder {
        let id = format!("ORD-{}", self.order_counter);
        self.order_counter += 1;
        self.orders.push(LimitOrder {
            id, owner: owner.into(), pool_id: pool_id.into(),
            side, price, amount, filled: 0,
            status: OrderStatus::Open, trit: 0, created_at: now_ms(),
        });
        self.orders.last().unwrap()
    }

    pub fn match_orders(&mut self, pool_id: &str) -> Vec<(usize, usize, u64)> {
        let mut matches = Vec::new();
        let buys: Vec<usize> = self.orders.iter().enumerate()
            .filter(|(_, o)| o.pool_id == pool_id && o.side == OrderSide::Buy && o.status == OrderStatus::Open)
            .map(|(i, _)| i).collect();
        let sells: Vec<usize> = self.orders.iter().enumerate()
            .filter(|(_, o)| o.pool_id == pool_id && o.side == OrderSide::Sell && o.status == OrderStatus::Open)
            .map(|(i, _)| i).collect();

        for &bi in &buys {
            for &si in &sells {
                let buy_price = self.orders[bi].price;
                let sell_price = self.orders[si].price;
                if buy_price >= sell_price {
                    let buy_remaining = self.orders[bi].amount - self.orders[bi].filled;
                    let sell_remaining = self.orders[si].amount - self.orders[si].filled;
                    let fill = buy_remaining.min(sell_remaining);
                    if fill > 0 {
                        self.orders[bi].filled += fill;
                        self.orders[si].filled += fill;
                        self.orders[bi].trit = 1;
                        self.orders[si].trit = 1;
                        if self.orders[bi].filled >= self.orders[bi].amount { self.orders[bi].status = OrderStatus::Filled; }
                        else { self.orders[bi].status = OrderStatus::PartialFill; }
                        if self.orders[si].filled >= self.orders[si].amount { self.orders[si].status = OrderStatus::Filled; }
                        else { self.orders[si].status = OrderStatus::PartialFill; }
                        matches.push((bi, si, fill));
                    }
                }
            }
        }
        matches
    }

    pub fn cancel(&mut self, order_idx: usize) {
        if let Some(o) = self.orders.get_mut(order_idx) {
            o.status = OrderStatus::Cancelled;
            o.trit = -1;
        }
    }

    pub fn open_orders(&self, pool_id: &str) -> Vec<&LimitOrder> {
        self.orders.iter().filter(|o| o.pool_id == pool_id && o.status == OrderStatus::Open).collect()
    }
}

// ═══════════════════════════════════════
// DEX 본체
// ═══════════════════════════════════════

pub struct CrownyDEX {
    pub pools: HashMap<String, LiquidityPool>,
    pub tokens: HashMap<String, Token>,
    pub balances: HashMap<String, HashMap<String, u64>>,  // user → token → amount
    pub order_book: OrderBook,
    pub swap_history: Vec<SwapResult>,
    pub lp_history: Vec<LPReceipt>,
    pub total_volume: u64,
    pub total_fees: u64,
}

impl CrownyDEX {
    pub fn new() -> Self {
        let mut dex = Self {
            pools: HashMap::new(), tokens: HashMap::new(),
            balances: HashMap::new(), order_book: OrderBook::new(),
            swap_history: Vec::new(), lp_history: Vec::new(),
            total_volume: 0, total_fees: 0,
        };
        // 기본 토큰
        dex.register_token("CRWN", "Crowny Token", 153_000_000);
        dex.register_token("USDT", "Tether USD", 1_000_000_000);
        dex.register_token("ETH", "Ethereum", 120_000_000);
        dex.register_token("BTC", "Bitcoin", 21_000_000);
        dex.register_token("TRIT", "Trit Governance", 27_000_000);
        dex
    }

    pub fn register_token(&mut self, symbol: &str, name: &str, supply: u64) {
        self.tokens.insert(symbol.into(), Token::new(symbol, name, supply));
    }

    pub fn mint(&mut self, user: &str, token: &str, amount: u64) {
        *self.balances.entry(user.into()).or_default().entry(token.into()).or_insert(0) += amount;
    }

    pub fn balance(&self, user: &str, token: &str) -> u64 {
        self.balances.get(user).and_then(|m| m.get(token)).copied().unwrap_or(0)
    }

    pub fn create_pool(&mut self, token_a: &str, token_b: &str, fee_bps: u64) -> String {
        let pool = LiquidityPool::new(token_a, token_b, fee_bps);
        let id = pool.id.clone();
        self.pools.insert(id.clone(), pool);
        id
    }

    pub fn add_liquidity(&mut self, user: &str, pool_id: &str, amount_a: u64, amount_b: u64) -> Result<LPReceipt, String> {
        let pool = self.pools.get(pool_id).ok_or("풀 없음")?.clone();
        let bal_a = self.balance(user, &pool.token_a);
        let bal_b = self.balance(user, &pool.token_b);
        if bal_a < amount_a { return Err(format!("{} 잔액 부족 ({})", pool.token_a, bal_a)); }
        if bal_b < amount_b { return Err(format!("{} 잔액 부족 ({})", pool.token_b, bal_b)); }

        // 차감
        *self.balances.get_mut(user).unwrap().get_mut(&pool.token_a).unwrap() -= amount_a;
        *self.balances.get_mut(user).unwrap().get_mut(&pool.token_b).unwrap() -= amount_b;

        let receipt = self.pools.get_mut(pool_id).unwrap().add_liquidity(user, amount_a, amount_b);
        self.lp_history.push(receipt.clone());
        Ok(receipt)
    }

    pub fn swap(&mut self, user: &str, pool_id: &str, token_in: &str, amount_in: u64) -> Result<SwapResult, String> {
        let pool = self.pools.get(pool_id).ok_or("풀 없음")?;
        let is_a_to_b = token_in == pool.token_a;
        let token_out = if is_a_to_b { pool.token_b.clone() } else { pool.token_a.clone() };

        let bal = self.balance(user, token_in);
        if bal < amount_in { return Err(format!("{} 잔액 부족 ({})", token_in, bal)); }

        // 차감
        *self.balances.get_mut(user).unwrap().get_mut(token_in).unwrap() -= amount_in;

        let result = if is_a_to_b {
            self.pools.get_mut(pool_id).unwrap().swap_a_to_b(amount_in)?
        } else {
            self.pools.get_mut(pool_id).unwrap().swap_b_to_a(amount_in)?
        };

        // 지급
        *self.balances.entry(user.into()).or_default().entry(token_out).or_insert(0) += result.amount_out;

        self.total_volume += amount_in;
        self.total_fees += result.fee;
        self.swap_history.push(result.clone());
        Ok(result)
    }

    pub fn place_order(&mut self, user: &str, pool_id: &str, side: OrderSide, price: f64, amount: u64) -> String {
        let order = self.order_book.place_order(user, pool_id, side, price, amount);
        order.id.clone()
    }

    pub fn match_orders(&mut self, pool_id: &str) -> Vec<(usize, usize, u64)> {
        self.order_book.match_orders(pool_id)
    }

    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("CrownyDEX"));
        lines.push(format!("  토큰: {} | 풀: {} | 스왑: {} | 주문: {}",
            self.tokens.len(), self.pools.len(), self.swap_history.len(), self.order_book.orders.len()));
        lines.push(format!("  총 거래량: {} | 총 수수료: {}", self.total_volume, self.total_fees));
        for pool in self.pools.values() {
            lines.push(format!("  {}", pool));
        }
        lines.join("\n")
    }
}

// ═══ 데모 ═══

pub fn demo_dex() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny DEX — 3진 탈중앙 거래소                ║");
    println!("║  AMM · 유동성 풀 · 오더북 · 스왑 · LP 보상      ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut dex = CrownyDEX::new();

    // 1. 토큰 등록
    println!("━━━ 1. 등록 토큰 ━━━");
    for token in dex.tokens.values() { println!("  {}", token); }
    println!();

    // 2. 사용자 잔액 배정
    println!("━━━ 2. 초기 잔액 ━━━");
    let users = vec![
        ("alice", vec![("CRWN", 500_000), ("USDT", 100_000), ("ETH", 50), ("TRIT", 10_000)]),
        ("bob", vec![("CRWN", 300_000), ("USDT", 80_000), ("BTC", 2), ("TRIT", 5_000)]),
        ("carol", vec![("CRWN", 200_000), ("USDT", 50_000), ("ETH", 30)]),
    ];
    for (user, tokens) in &users {
        for (token, amount) in tokens {
            dex.mint(user, token, *amount);
        }
        let bals: Vec<String> = tokens.iter().map(|(t, a)| format!("{} {}", a, t)).collect();
        println!("  {} — {}", user, bals.join(", "));
    }
    println!();

    // 3. 유동성 풀 생성
    println!("━━━ 3. 유동성 풀 ━━━");
    let pool_crwn_usdt = dex.create_pool("CRWN", "USDT", 30);
    let pool_crwn_eth = dex.create_pool("CRWN", "ETH", 30);
    let pool_crwn_trit = dex.create_pool("CRWN", "TRIT", 50);

    // CRWN-USDT 풀에 유동성 추가
    let r = dex.add_liquidity("alice", &pool_crwn_usdt, 200_000, 25_000).unwrap();
    println!("  {}", r);
    let r = dex.add_liquidity("bob", &pool_crwn_usdt, 100_000, 12_500).unwrap();
    println!("  {}", r);

    // CRWN-ETH 풀
    let r = dex.add_liquidity("alice", &pool_crwn_eth, 100_000, 10).unwrap();
    println!("  {}", r);
    let r = dex.add_liquidity("carol", &pool_crwn_eth, 80_000, 8).unwrap();
    println!("  {}", r);

    // CRWN-TRIT 풀
    let r = dex.add_liquidity("alice", &pool_crwn_trit, 50_000, 5_000).unwrap();
    println!("  {}", r);
    println!();

    // 풀 현황
    println!("━━━ 4. 풀 현황 ━━━");
    for pool in dex.pools.values() {
        println!("  {}", pool);
        println!("    TVL: {} + {} | LP: {} shares | LP 수: {}",
            pool.reserve_a, pool.reserve_b, pool.total_lp_shares, pool.lp_holders.len());
    }
    println!();

    // 5. 스왑 실행
    println!("━━━ 5. 스왑 거래 ━━━");
    let swaps = vec![
        ("alice", "CRWN-USDT", "CRWN", 10_000),
        ("bob", "CRWN-USDT", "USDT", 5_000),
        ("carol", "CRWN-ETH", "CRWN", 20_000),
        ("alice", "CRWN-TRIT", "TRIT", 1_000),
        ("bob", "CRWN-USDT", "CRWN", 15_000),
        ("carol", "CRWN-ETH", "ETH", 3),
        ("alice", "CRWN-USDT", "USDT", 3_000),
        ("bob", "CRWN-USDT", "CRWN", 8_000),
    ];
    for (user, pool_id, token_in, amount) in &swaps {
        match dex.swap(user, pool_id, token_in, *amount) {
            Ok(r) => println!("  {} — {}", user, r),
            Err(e) => println!("  [T] {} — {}", user, e),
        }
    }
    println!();

    // 6. 오더북
    println!("━━━ 6. 리밋 주문 ━━━");
    dex.place_order("alice", "CRWN-USDT", OrderSide::Buy, 0.130, 5_000);
    dex.place_order("alice", "CRWN-USDT", OrderSide::Buy, 0.128, 3_000);
    dex.place_order("bob", "CRWN-USDT", OrderSide::Sell, 0.125, 4_000);
    dex.place_order("bob", "CRWN-USDT", OrderSide::Sell, 0.132, 2_000);
    dex.place_order("carol", "CRWN-USDT", OrderSide::Buy, 0.126, 6_000);

    println!("  대기 주문:");
    for order in &dex.order_book.orders {
        println!("    {} — {}", order.owner, order);
    }

    let matches = dex.match_orders("CRWN-USDT");
    println!("  매칭 결과: {} 체결", matches.len());
    for (bi, si, fill) in &matches {
        println!("    매수#{} ↔ 매도#{} — {} 체결",
            dex.order_book.orders[*bi].id, dex.order_book.orders[*si].id, fill);
    }

    println!("  주문 상태:");
    for order in &dex.order_book.orders {
        println!("    {} — {}", order.owner, order);
    }
    println!();

    // 7. 최종 잔액
    println!("━━━ 7. 최종 잔액 ━━━");
    for (user, _) in &users {
        let bals = dex.balances.get(*user).unwrap();
        let parts: Vec<String> = bals.iter()
            .filter(|(_, v)| **v > 0)
            .map(|(t, v)| format!("{} {}", v, t))
            .collect();
        println!("  {} — {}", user, parts.join(", "));
    }
    println!();

    // 8. 풀 최종 상태
    println!("━━━ 8. 풀 최종 상태 ━━━");
    for pool in dex.pools.values() {
        let price = pool.price_a_in_b();
        let apr = pool.estimated_apr(0.124, 1.0);
        println!("  {} | 가격: {:.6} | 수수료: {} | APR: {:.1}%",
            pool.id, price, pool.fees_collected, apr);
    }
    println!();

    // 9. DEX 요약
    println!("━━━ 9. DEX 요약 ━━━");
    println!("{}", dex.summary());
    println!();
    println!("✓ Crowny DEX 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_create() {
        let t = Token::new("CRWN", "Crowny", 153_000_000);
        assert_eq!(t.symbol, "CRWN");
        assert_eq!(t.total_supply, 153_000_000);
    }

    #[test]
    fn test_pool_add_liquidity() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        let r = pool.add_liquidity("user", 1000, 2000);
        assert_eq!(r.trit, 1);
        assert!(pool.reserve_a == 1000);
        assert!(pool.reserve_b == 2000);
        assert!(pool.total_lp_shares > 0);
    }

    #[test]
    fn test_pool_swap_a_to_b() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        pool.add_liquidity("lp", 100_000, 100_000);
        let r = pool.swap_a_to_b(1000).unwrap();
        assert!(r.amount_out > 0);
        assert!(r.amount_out < 1000); // slippage
        assert!(r.fee > 0);
    }

    #[test]
    fn test_pool_swap_b_to_a() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        pool.add_liquidity("lp", 100_000, 100_000);
        let r = pool.swap_b_to_a(1000).unwrap();
        assert!(r.amount_out > 0);
    }

    #[test]
    fn test_pool_no_liquidity() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        assert!(pool.swap_a_to_b(100).is_err());
    }

    #[test]
    fn test_pool_price() {
        let mut pool = LiquidityPool::new("CRWN", "USDT", 30);
        pool.add_liquidity("lp", 100_000, 12_500);
        let price = pool.price_a_in_b();
        assert!((price - 0.125).abs() < 0.001);
    }

    #[test]
    fn test_pool_remove_liquidity() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        let receipt = pool.add_liquidity("user", 10_000, 10_000);
        let shares = receipt.shares_minted;
        let r = pool.remove_liquidity("user", shares / 2).unwrap();
        assert!(r.amount_a > 0);
        assert!(r.amount_b > 0);
    }

    #[test]
    fn test_pool_remove_insufficient() {
        let mut pool = LiquidityPool::new("A", "B", 30);
        pool.add_liquidity("user", 1000, 1000);
        assert!(pool.remove_liquidity("user", 999_999).is_err());
    }

    #[test]
    fn test_dex_create_swap() {
        let mut dex = CrownyDEX::new();
        dex.mint("alice", "CRWN", 100_000);
        dex.mint("alice", "USDT", 20_000);
        let pool = dex.create_pool("CRWN", "USDT", 30);
        dex.add_liquidity("alice", &pool, 50_000, 10_000).unwrap();
        let r = dex.swap("alice", &pool, "CRWN", 1_000).unwrap();
        assert!(r.amount_out > 0);
        assert!(dex.balance("alice", "USDT") > 10_000);
    }

    #[test]
    fn test_dex_insufficient_balance() {
        let mut dex = CrownyDEX::new();
        dex.mint("alice", "CRWN", 100);
        dex.mint("alice", "USDT", 100);
        let pool = dex.create_pool("CRWN", "USDT", 30);
        dex.add_liquidity("alice", &pool, 50, 50).unwrap();
        assert!(dex.swap("alice", &pool, "CRWN", 999_999).is_err());
    }

    #[test]
    fn test_order_book_match() {
        let mut ob = OrderBook::new();
        ob.place_order("buyer", "A-B", OrderSide::Buy, 1.0, 100);
        ob.place_order("seller", "A-B", OrderSide::Sell, 0.9, 80);
        let matches = ob.match_orders("A-B");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].2, 80);
        assert_eq!(ob.orders[0].status, OrderStatus::PartialFill);
        assert_eq!(ob.orders[1].status, OrderStatus::Filled);
    }

    #[test]
    fn test_order_book_no_match() {
        let mut ob = OrderBook::new();
        ob.place_order("buyer", "A-B", OrderSide::Buy, 0.5, 100);
        ob.place_order("seller", "A-B", OrderSide::Sell, 1.0, 100);
        let matches = ob.match_orders("A-B");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_order_cancel() {
        let mut ob = OrderBook::new();
        ob.place_order("user", "A-B", OrderSide::Buy, 1.0, 100);
        ob.cancel(0);
        assert_eq!(ob.orders[0].status, OrderStatus::Cancelled);
        assert_eq!(ob.orders[0].trit, -1);
    }

    #[test]
    fn test_dex_summary() {
        let dex = CrownyDEX::new();
        let s = dex.summary();
        assert!(s.contains("CrownyDEX"));
        assert!(s.contains("토큰: 5"));
    }
}
