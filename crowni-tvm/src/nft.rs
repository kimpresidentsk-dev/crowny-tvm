// ═══════════════════════════════════════════════════════════════
// Crowny NFT — 3진 NFT 시스템
// 발행(민트) · 컬렉션 · 마켓플레이스 · 경매 · 로열티 · 메타데이터
// 모든 NFT에 P/O/T trit 상태 + CTP 헤더
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
// NFT 메타데이터
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct NFTMetadata {
    pub name: String,
    pub description: String,
    pub image_uri: String,
    pub attributes: Vec<(String, String)>,
    pub trit_attributes: Vec<(String, i8)>,   // 3진 속성
}

impl NFTMetadata {
    pub fn new(name: &str, desc: &str, image: &str) -> Self {
        Self {
            name: name.into(), description: desc.into(), image_uri: image.into(),
            attributes: Vec::new(), trit_attributes: Vec::new(),
        }
    }
    pub fn attr(mut self, key: &str, val: &str) -> Self { self.attributes.push((key.into(), val.into())); self }
    pub fn trit_attr(mut self, key: &str, val: i8) -> Self { self.trit_attributes.push((key.into(), val)); self }
}

// ═══════════════════════════════════════
// NFT
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum NFTRarity { Common, Uncommon, Rare, Epic, Legendary, Mythic }

impl NFTRarity {
    pub fn trit(&self) -> i8 {
        match self { Self::Legendary | Self::Mythic => 1, Self::Common | Self::Uncommon => -1, _ => 0 }
    }
    pub fn multiplier(&self) -> f64 {
        match self { Self::Common => 1.0, Self::Uncommon => 1.5, Self::Rare => 3.0,
            Self::Epic => 7.0, Self::Legendary => 15.0, Self::Mythic => 50.0 }
    }
}

impl std::fmt::Display for NFTRarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Common => write!(f, "일반"), Self::Uncommon => write!(f, "비범"),
            Self::Rare => write!(f, "희귀"), Self::Epic => write!(f, "에픽"),
            Self::Legendary => write!(f, "전설"), Self::Mythic => write!(f, "신화") }
    }
}

#[derive(Debug, Clone)]
pub struct NFT {
    pub id: String,
    pub token_id: u64,
    pub collection_id: String,
    pub owner: String,
    pub creator: String,
    pub metadata: NFTMetadata,
    pub rarity: NFTRarity,
    pub royalty_bps: u64,           // 로열티 (basis points)
    pub trit_state: i8,
    pub hash: String,
    pub transfer_count: u32,
    pub minted_at: u64,
    pub listed: bool,
    pub price: Option<u64>,
}

impl NFT {
    pub fn trit_label(&self) -> &str { match self.trit_state { 1 => "P", -1 => "T", _ => "O" } }
}

impl std::fmt::Display for NFT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let listed = if self.listed { format!(" 📢{} CRWN", self.price.unwrap_or(0)) } else { String::new() };
        write!(f, "[{}] #{} \"{}\" ({}) — {} | royalty:{}%{}",
            self.trit_label(), self.token_id, self.metadata.name, self.rarity,
            self.owner, self.royalty_bps as f64 / 100.0, listed)
    }
}

// ═══════════════════════════════════════
// 컬렉션
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub creator: String,
    pub description: String,
    pub max_supply: Option<u64>,
    pub minted: u64,
    pub royalty_bps: u64,
    pub floor_price: u64,
    pub total_volume: u64,
    pub nft_ids: Vec<String>,
    pub trit_state: i8,
    pub created_at: u64,
}

impl Collection {
    pub fn new(name: &str, symbol: &str, creator: &str, desc: &str, max_supply: Option<u64>, royalty_bps: u64) -> Self {
        Self {
            id: trit_hash(&format!("col:{}:{}", name, now_ms())),
            name: name.into(), symbol: symbol.into(), creator: creator.into(),
            description: desc.into(), max_supply, minted: 0, royalty_bps,
            floor_price: 0, total_volume: 0, nft_ids: Vec::new(),
            trit_state: 1, created_at: now_ms(),
        }
    }

    pub fn can_mint(&self) -> bool {
        self.max_supply.map(|m| self.minted < m).unwrap_or(true)
    }
}

impl std::fmt::Display for Collection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let supply = self.max_supply.map(|m| format!("{}/{}", self.minted, m)).unwrap_or(format!("{}/∞", self.minted));
        write!(f, "[P] {} ({}) — {} | floor:{} CRWN | vol:{} CRWN | by {}",
            self.name, self.symbol, supply, self.floor_price, self.total_volume, self.creator)
    }
}

// ═══════════════════════════════════════
// 경매
// ═══════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
pub enum AuctionStatus { Active, Ended, Cancelled }

#[derive(Debug, Clone)]
pub struct Bid {
    pub bidder: String,
    pub amount: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Auction {
    pub id: String,
    pub nft_id: String,
    pub seller: String,
    pub start_price: u64,
    pub reserve_price: u64,
    pub current_bid: u64,
    pub bids: Vec<Bid>,
    pub status: AuctionStatus,
    pub started_at: u64,
    pub duration_ms: u64,
}

impl Auction {
    pub fn new(nft_id: &str, seller: &str, start: u64, reserve: u64, duration_ms: u64) -> Self {
        Self {
            id: trit_hash(&format!("auction:{}:{}", nft_id, now_ms())),
            nft_id: nft_id.into(), seller: seller.into(),
            start_price: start, reserve_price: reserve, current_bid: start,
            bids: Vec::new(), status: AuctionStatus::Active,
            started_at: now_ms(), duration_ms,
        }
    }

    pub fn place_bid(&mut self, bidder: &str, amount: u64) -> Result<(), String> {
        if self.status != AuctionStatus::Active { return Err("경매 종료됨".into()); }
        if amount <= self.current_bid { return Err(format!("최소 {} CRWN 이상", self.current_bid + 1)); }
        self.current_bid = amount;
        self.bids.push(Bid { bidder: bidder.into(), amount, timestamp: now_ms() });
        Ok(())
    }

    pub fn end(&mut self) -> Option<Bid> {
        self.status = AuctionStatus::Ended;
        if self.current_bid >= self.reserve_price {
            self.bids.last().cloned()
        } else {
            None // reserve 미달
        }
    }

    pub fn highest_bidder(&self) -> Option<&Bid> { self.bids.last() }
}

impl std::fmt::Display for Auction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = match self.status { AuctionStatus::Active => "🔴진행중", AuctionStatus::Ended => "✅종료", AuctionStatus::Cancelled => "✗취소" };
        write!(f, "{} NFT:{} — 현재:{} CRWN | 입찰:{} | {}",
            status, &self.nft_id[..12], self.current_bid, self.bids.len(),
            self.seller)
    }
}

// ═══════════════════════════════════════
// 마켓 거래 기록
// ═══════════════════════════════════════

#[derive(Debug, Clone)]
pub struct MarketTx {
    pub nft_id: String,
    pub from: String,
    pub to: String,
    pub price: u64,
    pub royalty_paid: u64,
    pub fee: u64,
    pub tx_type: MarketTxType,
    pub hash: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum MarketTxType { Sale, AuctionWin, Transfer }

impl std::fmt::Display for MarketTx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = match &self.tx_type { MarketTxType::Sale => "판매", MarketTxType::AuctionWin => "경매낙찰", MarketTxType::Transfer => "전송" };
        write!(f, "[P] {} {} → {} | {} CRWN (royalty:{}, fee:{})",
            ty, self.from, self.to, self.price, self.royalty_paid, self.fee)
    }
}

// ═══════════════════════════════════════
// NFT 마켓플레이스
// ═══════════════════════════════════════

pub struct CrownyNFT {
    pub collections: HashMap<String, Collection>,
    pub nfts: HashMap<String, NFT>,
    pub auctions: Vec<Auction>,
    pub market_history: Vec<MarketTx>,
    pub balances: HashMap<String, u64>,   // user → CRWN balance
    pub token_counter: u64,
    pub market_fee_bps: u64,              // 마켓 수수료 (2.5%)
    pub total_volume: u64,
    pub total_fees: u64,
    pub total_royalties: u64,
}

impl CrownyNFT {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(), nfts: HashMap::new(),
            auctions: Vec::new(), market_history: Vec::new(),
            balances: HashMap::new(), token_counter: 0,
            market_fee_bps: 250, total_volume: 0, total_fees: 0, total_royalties: 0,
        }
    }

    pub fn fund(&mut self, user: &str, amount: u64) {
        *self.balances.entry(user.into()).or_insert(0) += amount;
    }

    pub fn balance(&self, user: &str) -> u64 { self.balances.get(user).copied().unwrap_or(0) }

    /// 컬렉션 생성
    pub fn create_collection(&mut self, name: &str, symbol: &str, creator: &str, desc: &str, max_supply: Option<u64>, royalty_bps: u64) -> String {
        let col = Collection::new(name, symbol, creator, desc, max_supply, royalty_bps);
        let id = col.id.clone();
        self.collections.insert(id.clone(), col);
        id
    }

    /// NFT 민트
    pub fn mint(&mut self, collection_id: &str, owner: &str, metadata: NFTMetadata, rarity: NFTRarity) -> Result<String, String> {
        let col = self.collections.get_mut(collection_id).ok_or("컬렉션 없음")?;
        if !col.can_mint() { return Err("최대 발행량 도달".into()); }

        let token_id = self.token_counter;
        self.token_counter += 1;
        let nft_id = trit_hash(&format!("nft:{}:{}:{}", collection_id, token_id, now_ms()));

        let nft = NFT {
            id: nft_id.clone(), token_id, collection_id: collection_id.into(),
            owner: owner.into(), creator: owner.into(), metadata,
            rarity, royalty_bps: col.royalty_bps,
            trit_state: 1, hash: trit_hash(&format!("hash:{}:{}", token_id, now_ms())),
            transfer_count: 0, minted_at: now_ms(), listed: false, price: None,
        };

        col.minted += 1;
        col.nft_ids.push(nft_id.clone());
        self.nfts.insert(nft_id.clone(), nft);
        Ok(nft_id)
    }

    /// NFT 리스팅 (판매 등록)
    pub fn list(&mut self, nft_id: &str, price: u64) -> Result<(), String> {
        let nft = self.nfts.get_mut(nft_id).ok_or("NFT 없음")?;
        nft.listed = true;
        nft.price = Some(price);
        nft.trit_state = 0; // 대기 상태
        Ok(())
    }

    /// NFT 구매
    pub fn buy(&mut self, nft_id: &str, buyer: &str) -> Result<MarketTx, String> {
        let nft = self.nfts.get(nft_id).ok_or("NFT 없음")?.clone();
        if !nft.listed { return Err("리스팅되지 않음".into()); }
        let price = nft.price.ok_or("가격 미설정")?;
        let buyer_bal = self.balance(buyer);
        if buyer_bal < price { return Err(format!("잔액 부족: {} < {}", buyer_bal, price)); }
        if buyer == nft.owner { return Err("자기 자신에게 구매 불가".into()); }

        let fee = price * self.market_fee_bps / 10000;
        let royalty = price * nft.royalty_bps / 10000;
        let seller_receives = price - fee - royalty;

        // 잔액 이동
        *self.balances.get_mut(buyer).unwrap() -= price;
        *self.balances.entry(nft.owner.clone()).or_insert(0) += seller_receives;
        *self.balances.entry(nft.creator.clone()).or_insert(0) += royalty;

        let seller = nft.owner.clone();
        let creator = nft.creator.clone();

        // NFT 소유권 이전
        let nft_mut = self.nfts.get_mut(nft_id).unwrap();
        nft_mut.owner = buyer.into();
        nft_mut.listed = false;
        nft_mut.price = None;
        nft_mut.transfer_count += 1;
        nft_mut.trit_state = 1;

        // 컬렉션 통계 업데이트
        if let Some(col) = self.collections.get_mut(&nft.collection_id) {
            col.total_volume += price;
            // floor price 업데이트
            let floor = self.nfts.values()
                .filter(|n| n.collection_id == nft.collection_id && n.listed)
                .filter_map(|n| n.price)
                .min().unwrap_or(0);
            col.floor_price = floor;
        }

        let tx = MarketTx {
            nft_id: nft_id.into(), from: seller, to: buyer.into(),
            price, royalty_paid: royalty, fee,
            tx_type: MarketTxType::Sale,
            hash: trit_hash(&format!("sale:{}:{}:{}", nft_id, price, now_ms())),
            timestamp: now_ms(),
        };

        self.total_volume += price;
        self.total_fees += fee;
        self.total_royalties += royalty;
        self.market_history.push(tx.clone());
        Ok(tx)
    }

    /// 경매 시작
    pub fn start_auction(&mut self, nft_id: &str, start_price: u64, reserve: u64, duration_ms: u64) -> Result<usize, String> {
        let nft = self.nfts.get_mut(nft_id).ok_or("NFT 없음")?;
        nft.listed = true;
        nft.trit_state = 0;
        let seller = nft.owner.clone();
        let auction = Auction::new(nft_id, &seller, start_price, reserve, duration_ms);
        self.auctions.push(auction);
        Ok(self.auctions.len() - 1)
    }

    /// 경매 입찰
    pub fn bid(&mut self, auction_idx: usize, bidder: &str, amount: u64) -> Result<(), String> {
        let bal = self.balance(bidder);
        if bal < amount { return Err("잔액 부족".into()); }
        self.auctions.get_mut(auction_idx).ok_or("경매 없음")?.place_bid(bidder, amount)
    }

    /// 경매 종료 + 정산
    pub fn end_auction(&mut self, auction_idx: usize) -> Result<Option<MarketTx>, String> {
        let auction = self.auctions.get_mut(auction_idx).ok_or("경매 없음")?;
        let winner = auction.end();

        if let Some(winning_bid) = winner {
            let nft_id = auction.nft_id.clone();
            let seller = auction.seller.clone();
            let nft = self.nfts.get(&nft_id).ok_or("NFT 없음")?.clone();
            let price = winning_bid.amount;

            let fee = price * self.market_fee_bps / 10000;
            let royalty = price * nft.royalty_bps / 10000;
            let seller_receives = price - fee - royalty;

            *self.balances.entry(winning_bid.bidder.clone()).or_insert(0) -= price.min(self.balance(&winning_bid.bidder));
            *self.balances.entry(seller.clone()).or_insert(0) += seller_receives;
            *self.balances.entry(nft.creator.clone()).or_insert(0) += royalty;

            let nft_mut = self.nfts.get_mut(&nft_id).unwrap();
            nft_mut.owner = winning_bid.bidder.clone();
            nft_mut.listed = false;
            nft_mut.transfer_count += 1;
            nft_mut.trit_state = 1;

            if let Some(col) = self.collections.get_mut(&nft.collection_id) {
                col.total_volume += price;
            }

            let tx = MarketTx {
                nft_id, from: seller, to: winning_bid.bidder,
                price, royalty_paid: royalty, fee,
                tx_type: MarketTxType::AuctionWin,
                hash: trit_hash(&format!("auction:{}:{}", price, now_ms())),
                timestamp: now_ms(),
            };
            self.total_volume += price;
            self.total_fees += fee;
            self.total_royalties += royalty;
            self.market_history.push(tx.clone());
            Ok(Some(tx))
        } else {
            // reserve 미달 → 유찰
            let nft_id = &self.auctions[auction_idx].nft_id;
            if let Some(nft) = self.nfts.get_mut(nft_id) {
                nft.listed = false;
                nft.trit_state = -1;
            }
            Ok(None)
        }
    }

    /// NFT 전송
    pub fn transfer(&mut self, nft_id: &str, to: &str) -> Result<(), String> {
        let nft = self.nfts.get_mut(nft_id).ok_or("NFT 없음")?;
        nft.owner = to.into();
        nft.transfer_count += 1;
        Ok(())
    }

    pub fn nfts_by_owner(&self, owner: &str) -> Vec<&NFT> {
        self.nfts.values().filter(|n| n.owner == owner).collect()
    }

    pub fn summary(&self) -> String {
        format!("CrownyNFT 마켓플레이스\n  컬렉션: {} | NFT: {} | 경매: {} | 거래: {}\n  볼륨: {} CRWN | 수수료: {} | 로열티: {}",
            self.collections.len(), self.nfts.len(), self.auctions.len(),
            self.market_history.len(), self.total_volume, self.total_fees, self.total_royalties)
    }
}

// ═══ 데모 ═══

pub fn demo_nft() {
    println!("╔═══════════════════════════════════════════════╗");
    println!("║  Crowny NFT — 3진 NFT 마켓플레이스              ║");
    println!("║  민트 · 컬렉션 · 마켓 · 경매 · 로열티            ║");
    println!("╚═══════════════════════════════════════════════╝");
    println!();

    let mut market = CrownyNFT::new();

    // 1. 사용자 자금
    println!("━━━ 1. 사용자 자금 ━━━");
    market.fund("alice", 500_000);
    market.fund("bob", 300_000);
    market.fund("carol", 200_000);
    market.fund("dave", 100_000);
    for u in &["alice", "bob", "carol", "dave"] {
        println!("  {} — {} CRWN", u, market.balance(u));
    }
    println!();

    // 2. 컬렉션 생성
    println!("━━━ 2. 컬렉션 ━━━");
    let col_art = market.create_collection(
        "Trit Genesis", "TGEN", "alice",
        "3진법 기반 제네시스 아트 컬렉션", Some(100), 500, // 5% 로열티
    );
    let col_avatar = market.create_collection(
        "Crowny Avatars", "CAVT", "bob",
        "Crowny 네트워크 프로필 아바타", Some(1000), 300, // 3% 로열티
    );
    let col_music = market.create_collection(
        "한선 사운드", "HSSND", "carol",
        "한선어로 만든 음악 NFT", None, 750, // 7.5% 로열티
    );
    for col in market.collections.values() { println!("  {}", col); }
    println!();

    // 3. NFT 민트
    println!("━━━ 3. NFT 민트 ━━━");
    let nfts_data = vec![
        (&col_art, "alice", "삼위일체 #1", "3진법의 아름다움", "crwn://art/trinity1.png", NFTRarity::Legendary,
            vec![("색상", "삼원색"), ("차원", "27")], vec![("밸런스", 1i8), ("조화", 1)]),
        (&col_art, "alice", "트릿 파동 #2", "균형 잡힌 파동 패턴", "crwn://art/wave2.png", NFTRarity::Epic,
            vec![("패턴", "파동"), ("주파수", "3Hz")], vec![("에너지", 0), ("안정", 1)]),
        (&col_art, "alice", "P-O-T 만다라", "3진 만다라 아트", "crwn://art/mandala.png", NFTRarity::Rare,
            vec![("형태", "만다라"), ("대칭", "3중")], vec![("복잡도", 1)]),
        (&col_avatar, "bob", "노드 가디언", "블록체인 수호자", "crwn://avatar/guardian.png", NFTRarity::Epic,
            vec![("클래스", "수호자"), ("레벨", "27")], vec![("방어", 1), ("공격", 0)]),
        (&col_avatar, "bob", "트릿 워리어", "3진 전사", "crwn://avatar/warrior.png", NFTRarity::Rare,
            vec![("클래스", "전사"), ("무기", "삼지창")], vec![("공격", 1), ("속도", 1)]),
        (&col_avatar, "bob", "합의 현자", "합의 알고리즘의 현자", "crwn://avatar/sage.png", NFTRarity::Legendary,
            vec![("클래스", "현자"), ("지혜", "최고")], vec![("합의", 1), ("통찰", 1)]),
        (&col_music, "carol", "삼진 비트", "3/4 박자의 전자 음악", "crwn://music/tritbeat.mp3", NFTRarity::Uncommon,
            vec![("장르", "일렉트로닉"), ("BPM", "129")], vec![("리듬", 1)]),
        (&col_music, "carol", "밸런스 소나타", "균형의 소나타", "crwn://music/sonata.mp3", NFTRarity::Mythic,
            vec![("장르", "클래식"), ("악장", "3")], vec![("감성", 1), ("깊이", 1)]),
    ];

    let mut minted_ids = Vec::new();
    for (col_id, owner, name, desc, img, rarity, attrs, trit_attrs) in &nfts_data {
        let mut meta = NFTMetadata::new(name, desc, img);
        for (k, v) in attrs { meta = meta.attr(k, v); }
        for (k, v) in trit_attrs { meta = meta.trit_attr(k, *v); }
        match market.mint(col_id, owner, meta, rarity.clone()) {
            Ok(id) => {
                let nft = market.nfts.get(&id).unwrap();
                println!("  {}", nft);
                minted_ids.push(id);
            }
            Err(e) => println!("  [T] 민트 실패: {}", e),
        }
    }
    println!();

    // 4. 마켓 리스팅
    println!("━━━ 4. 마켓 리스팅 ━━━");
    let listings = vec![
        (0, 50_000), (1, 25_000), (2, 10_000),
        (3, 30_000), (4, 15_000), (6, 5_000),
    ];
    for (idx, price) in &listings {
        if let Some(id) = minted_ids.get(*idx) {
            market.list(id, *price).ok();
            let nft = market.nfts.get(id).unwrap();
            println!("  📢 {} — {} CRWN", nft.metadata.name, price);
        }
    }
    println!();

    // 5. 구매
    println!("━━━ 5. 구매 ━━━");
    let purchases = vec![
        (0, "bob"), (2, "dave"), (4, "alice"), (6, "dave"),
    ];
    for (idx, buyer) in &purchases {
        if let Some(id) = minted_ids.get(*idx) {
            match market.buy(id, buyer) {
                Ok(tx) => println!("  {}", tx),
                Err(e) => println!("  [T] {}: {}", buyer, e),
            }
        }
    }
    println!();

    // 6. 경매
    println!("━━━ 6. 경매 ━━━");
    if let Some(legend_id) = minted_ids.get(5) {
        let ai = market.start_auction(legend_id, 20_000, 40_000, 86_400_000).unwrap();
        println!("  경매 시작: {} — 시작가 20,000 CRWN | 최소 40,000 CRWN", market.nfts.get(legend_id).unwrap().metadata.name);

        market.bid(ai, "alice", 25_000).ok();
        println!("  💰 alice: 25,000 CRWN");
        market.bid(ai, "dave", 35_000).ok();
        println!("  💰 dave: 35,000 CRWN");
        market.bid(ai, "alice", 45_000).ok();
        println!("  💰 alice: 45,000 CRWN");

        match market.end_auction(ai) {
            Ok(Some(tx)) => println!("  🏆 낙찰! {}", tx),
            Ok(None) => println!("  [T] 유찰 (reserve 미달)"),
            Err(e) => println!("  [T] {}", e),
        }
    }

    // 7. 밸런스 소나타 경매 (Mythic)
    if let Some(mythic_id) = minted_ids.get(7) {
        let ai = market.start_auction(mythic_id, 50_000, 80_000, 86_400_000).unwrap();
        println!("\n  경매 시작: {} — 시작가 50,000 CRWN | 최소 80,000 CRWN", market.nfts.get(mythic_id).unwrap().metadata.name);
        market.bid(ai, "bob", 60_000).ok();
        println!("  💰 bob: 60,000 CRWN");
        market.bid(ai, "alice", 85_000).ok();
        println!("  💰 alice: 85,000 CRWN");
        market.bid(ai, "bob", 100_000).ok();
        println!("  💰 bob: 100,000 CRWN");
        match market.end_auction(ai) {
            Ok(Some(tx)) => println!("  🏆 낙찰! {}", tx),
            Ok(None) => println!("  [T] 유찰"),
            Err(e) => println!("  [T] {}", e),
        }
    }
    println!();

    // 8. 컬렉션 현황
    println!("━━━ 7. 컬렉션 현황 ━━━");
    for col in market.collections.values() { println!("  {}", col); }
    println!();

    // 9. 소유 현황
    println!("━━━ 8. 소유 현황 ━━━");
    for u in &["alice", "bob", "carol", "dave"] {
        let owned = market.nfts_by_owner(u);
        let names: Vec<String> = owned.iter().map(|n| format!("\"{}\"({})", n.metadata.name, n.rarity)).collect();
        println!("  {} [{}CRWN] — {} NFT: {}", u, market.balance(u), owned.len(),
            if names.is_empty() { "-".into() } else { names.join(", ") });
    }
    println!();

    // 10. 거래 이력
    println!("━━━ 9. 거래 이력 ━━━");
    for tx in &market.market_history { println!("  {}", tx); }
    println!();

    // 11. 요약
    println!("━━━ 10. 요약 ━━━");
    println!("{}", market.summary());
    println!();
    println!("✓ Crowny NFT 데모 완료");
}

// ═══ 테스트 ═══

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_create() {
        let col = Collection::new("Test", "TST", "alice", "desc", Some(100), 500);
        assert_eq!(col.name, "Test");
        assert!(col.can_mint());
    }

    #[test]
    fn test_collection_max_supply() {
        let mut col = Collection::new("T", "T", "a", "d", Some(1), 0);
        col.minted = 1;
        assert!(!col.can_mint());
    }

    #[test]
    fn test_nft_mint() {
        let mut m = CrownyNFT::new();
        let col = m.create_collection("T", "T", "alice", "d", None, 500);
        let meta = NFTMetadata::new("Test NFT", "desc", "img.png");
        let id = m.mint(&col, "alice", meta, NFTRarity::Rare).unwrap();
        assert!(m.nfts.contains_key(&id));
        assert_eq!(m.nfts[&id].owner, "alice");
    }

    #[test]
    fn test_nft_list_and_buy() {
        let mut m = CrownyNFT::new();
        m.fund("bob", 100_000);
        let col = m.create_collection("T", "T", "alice", "d", None, 500);
        let meta = NFTMetadata::new("Art", "d", "i.png");
        let id = m.mint(&col, "alice", meta, NFTRarity::Rare).unwrap();
        m.list(&id, 10_000).ok();
        let tx = m.buy(&id, "bob").unwrap();
        assert_eq!(tx.price, 10_000);
        assert_eq!(m.nfts[&id].owner, "bob");
        assert!(m.balance("alice") > 0); // got paid
    }

    #[test]
    fn test_buy_insufficient() {
        let mut m = CrownyNFT::new();
        m.fund("bob", 10);
        let col = m.create_collection("T", "T", "alice", "d", None, 0);
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).unwrap();
        m.list(&id, 10_000).ok();
        assert!(m.buy(&id, "bob").is_err());
    }

    #[test]
    fn test_buy_self_error() {
        let mut m = CrownyNFT::new();
        m.fund("alice", 100_000);
        let col = m.create_collection("T", "T", "alice", "d", None, 0);
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).unwrap();
        m.list(&id, 1_000).ok();
        assert!(m.buy(&id, "alice").is_err());
    }

    #[test]
    fn test_royalty_payment() {
        let mut m = CrownyNFT::new();
        m.fund("bob", 100_000);
        let col = m.create_collection("T", "T", "alice", "d", None, 1000); // 10%
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).unwrap();
        m.list(&id, 10_000).ok();
        let tx = m.buy(&id, "bob").unwrap();
        assert_eq!(tx.royalty_paid, 1000); // 10%
        assert_eq!(tx.fee, 250); // 2.5%
    }

    #[test]
    fn test_auction_flow() {
        let mut m = CrownyNFT::new();
        m.fund("bob", 100_000);
        m.fund("carol", 100_000);
        let col = m.create_collection("T", "T", "alice", "d", None, 500);
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Epic).unwrap();
        let ai = m.start_auction(&id, 1000, 5000, 86400000).unwrap();
        m.bid(ai, "bob", 3000).ok();
        m.bid(ai, "carol", 6000).ok();
        let result = m.end_auction(ai).unwrap();
        assert!(result.is_some());
        assert_eq!(m.nfts[&id].owner, "carol");
    }

    #[test]
    fn test_auction_no_reserve() {
        let mut m = CrownyNFT::new();
        m.fund("bob", 100_000);
        let col = m.create_collection("T", "T", "alice", "d", None, 0);
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).unwrap();
        let ai = m.start_auction(&id, 1000, 50000, 86400000).unwrap();
        m.bid(ai, "bob", 2000).ok();
        let result = m.end_auction(ai).unwrap();
        assert!(result.is_none()); // 유찰
    }

    #[test]
    fn test_auction_bid_too_low() {
        let mut auction = Auction::new("nft1", "alice", 1000, 5000, 86400000);
        auction.place_bid("bob", 2000).ok();
        assert!(auction.place_bid("carol", 1500).is_err()); // too low
    }

    #[test]
    fn test_nft_transfer() {
        let mut m = CrownyNFT::new();
        let col = m.create_collection("T", "T", "alice", "d", None, 0);
        let id = m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).unwrap();
        m.transfer(&id, "bob").ok();
        assert_eq!(m.nfts[&id].owner, "bob");
        assert_eq!(m.nfts[&id].transfer_count, 1);
    }

    #[test]
    fn test_metadata_builder() {
        let meta = NFTMetadata::new("Test", "desc", "img.png")
            .attr("color", "red").attr("size", "large")
            .trit_attr("quality", 1);
        assert_eq!(meta.attributes.len(), 2);
        assert_eq!(meta.trit_attributes.len(), 1);
    }

    #[test]
    fn test_rarity_properties() {
        assert_eq!(NFTRarity::Common.multiplier(), 1.0);
        assert_eq!(NFTRarity::Mythic.multiplier(), 50.0);
        assert_eq!(NFTRarity::Legendary.trit(), 1);
        assert_eq!(NFTRarity::Common.trit(), -1);
    }

    #[test]
    fn test_nfts_by_owner() {
        let mut m = CrownyNFT::new();
        let col = m.create_collection("T", "T", "alice", "d", None, 0);
        m.mint(&col, "alice", NFTMetadata::new("A", "d", "i"), NFTRarity::Common).ok();
        m.mint(&col, "alice", NFTMetadata::new("B", "d", "i"), NFTRarity::Rare).ok();
        m.mint(&col, "bob", NFTMetadata::new("C", "d", "i"), NFTRarity::Epic).ok();
        assert_eq!(m.nfts_by_owner("alice").len(), 2);
        assert_eq!(m.nfts_by_owner("bob").len(), 1);
    }

    #[test]
    fn test_summary() {
        let m = CrownyNFT::new();
        assert!(m.summary().contains("CrownyNFT"));
    }
}
