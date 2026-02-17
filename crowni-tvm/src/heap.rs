///! Arena 기반 힙 메모리 — GPT 명세
///! - Arena 기반 (인덱스로 접근)
///! - GC 없음 (v1.0)
///! - 수동 해제는 명령어로 처리
///! - 주소는 usize index

use crate::value::Value;

/// 힙 셀 — 할당/해제 상태 추적
#[derive(Debug, Clone)]
struct HeapCell {
    value: Value,
    alive: bool,
}

/// Arena 기반 힙
pub struct Heap {
    cells: Vec<HeapCell>,
    free_list: Vec<usize>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            cells: Vec::with_capacity(4096),
            free_list: Vec::new(),
        }
    }

    /// 값을 힙에 할당, 주소(인덱스) 반환
    pub fn alloc(&mut self, value: Value) -> usize {
        if let Some(idx) = self.free_list.pop() {
            self.cells[idx] = HeapCell { value, alive: true };
            idx
        } else {
            let idx = self.cells.len();
            self.cells.push(HeapCell { value, alive: true });
            idx
        }
    }

    /// 주소로 값 읽기
    pub fn get(&self, addr: usize) -> Option<&Value> {
        self.cells.get(addr).and_then(|c| {
            if c.alive { Some(&c.value) } else { None }
        })
    }

    /// 주소로 값 쓰기
    pub fn set(&mut self, addr: usize, value: Value) -> bool {
        if let Some(cell) = self.cells.get_mut(addr) {
            if cell.alive {
                cell.value = value;
                return true;
            }
        }
        false
    }

    /// 수동 해제
    pub fn free(&mut self, addr: usize) -> bool {
        if let Some(cell) = self.cells.get_mut(addr) {
            if cell.alive {
                cell.alive = false;
                cell.value = Value::Nil;
                self.free_list.push(addr);
                return true;
            }
        }
        false
    }

    /// 할당된 셀 수
    pub fn alive_count(&self) -> usize {
        self.cells.iter().filter(|c| c.alive).count()
    }

    /// 전체 용량
    pub fn capacity(&self) -> usize {
        self.cells.len()
    }

    /// 덤프 (디버그용)
    pub fn dump(&self) {
        println!("=== 힙 (할당: {}/{}) ===", self.alive_count(), self.cells.len());
        for (i, cell) in self.cells.iter().enumerate() {
            if cell.alive {
                println!("  [&{}] {} ({})", i, cell.value, cell.value.type_name_kr());
            }
        }
    }
}
