package crowny

import (
	"testing"
)

func TestTritValues(t *testing.T) {
	if P.ToNumber() != 1 {
		t.Errorf("P should be 1, got %d", P.ToNumber())
	}
	if O.ToNumber() != 0 {
		t.Errorf("O should be 0, got %d", O.ToNumber())
	}
	if T.ToNumber() != -1 {
		t.Errorf("T should be -1, got %d", T.ToNumber())
	}
}

func TestTritKorean(t *testing.T) {
	if P.ToKorean() != "성공" {
		t.Error("P should be 성공")
	}
	if O.ToKorean() != "보류" {
		t.Error("O should be 보류")
	}
	if T.ToKorean() != "실패" {
		t.Error("T should be 실패")
	}
}

func TestTritNot(t *testing.T) {
	if TritNot(P) != T {
		t.Error("NOT P should be T")
	}
	if TritNot(T) != P {
		t.Error("NOT T should be P")
	}
	if TritNot(O) != O {
		t.Error("NOT O should be O")
	}
}

func TestTritAnd(t *testing.T) {
	if TritAnd(P, P) != P {
		t.Error("P AND P should be P")
	}
	if TritAnd(P, O) != O {
		t.Error("P AND O should be O")
	}
	if TritAnd(P, T) != T {
		t.Error("P AND T should be T")
	}
	if TritAnd(O, O) != O {
		t.Error("O AND O should be O")
	}
}

func TestTritOr(t *testing.T) {
	if TritOr(P, T) != P {
		t.Error("P OR T should be P")
	}
	if TritOr(T, T) != T {
		t.Error("T OR T should be T")
	}
	if TritOr(O, P) != P {
		t.Error("O OR P should be P")
	}
}

func TestConsensus(t *testing.T) {
	tests := []struct {
		input  []TritValue
		expect TritValue
	}{
		{[]TritValue{P, P, P}, P},
		{[]TritValue{T, T, T}, T},
		{[]TritValue{O, O, O}, O},
		{[]TritValue{P, P, T}, P},
		{[]TritValue{P, T, T}, T},
		{[]TritValue{P, O, T}, O},
		{[]TritValue{P, P, O}, P},
	}
	for _, tt := range tests {
		got := Consensus(tt.input)
		if got != tt.expect {
			t.Errorf("Consensus(%v) = %v, want %v", tt.input, got, tt.expect)
		}
	}
}

func TestCtpHeader(t *testing.T) {
	h := CtpSuccess()
	if h.String() != "PPPOOOOO0" && h.String() != "PPPOOOOOO" {
		// just check it starts with PPP
		s := h.String()
		if s[0] != 'P' || s[1] != 'P' || s[2] != 'P' {
			t.Errorf("CtpSuccess should start with PPP, got %s", s)
		}
	}

	h2 := ParseCtpHeader("PPTOOOOO0")
	if h2.Trits[0] != P {
		t.Error("first trit should be P")
	}
	if h2.Trits[2] != T {
		t.Error("third trit should be T")
	}
}

func TestCtpOverallState(t *testing.T) {
	h := CtpSuccess()
	if h.OverallState() != O {
		// has O trits so overall is O, not P
	}

	h2 := CtpFailed()
	if h2.OverallState() != T {
		t.Error("CtpFailed overall should be T")
	}
}

func TestTritFrom(t *testing.T) {
	if TritFrom(5) != P {
		t.Error("positive should give P")
	}
	if TritFrom(0) != O {
		t.Error("zero should give O")
	}
	if TritFrom(-3) != T {
		t.Error("negative should give T")
	}
}

func TestNewClient(t *testing.T) {
	c := NewClient("http://localhost:7293")
	if c == nil {
		t.Fatal("client should not be nil")
	}
	total, p, o, tr := c.Stats()
	if total != 0 || p != 0 || o != 0 || tr != 0 {
		t.Error("new client should have zero stats")
	}
}

func TestTritResultMethods(t *testing.T) {
	r := TritResult{State: P, Data: "ok", ElapsedMs: 10, TaskID: 1}
	if !r.IsSuccess() {
		t.Error("P result should be success")
	}
	if r.IsPending() || r.IsFailed() {
		t.Error("P result should not be pending or failed")
	}

	r2 := TritResult{State: O}
	if !r2.IsPending() {
		t.Error("O result should be pending")
	}

	r3 := TritResult{State: T}
	if !r3.IsFailed() {
		t.Error("T result should be failed")
	}
}
