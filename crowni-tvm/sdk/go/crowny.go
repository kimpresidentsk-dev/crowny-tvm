// Package crowny provides a Go SDK for the Crowny Balanced Ternary Platform.
//
// Usage:
//
//	client := crowny.NewClient("http://localhost:7293")
//	result, err := client.Run("넣어 42\n종료")
//	fmt.Println(result.State) // "P"
package crowny

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

// ═══════════════════════════════════════════════
// Trit
// ═══════════════════════════════════════════════

// TritValue represents a balanced ternary value: P(+1), O(0), T(-1)
type TritValue string

const (
	P TritValue = "P" // +1 성공/참/승인
	O TritValue = "O" //  0 보류/모름/대기
	T TritValue = "T" // -1 실패/거짓/거부
)

// ToNumber converts TritValue to int
func (t TritValue) ToNumber() int {
	switch t {
	case P:
		return 1
	case T:
		return -1
	default:
		return 0
	}
}

// ToKorean returns Korean representation
func (t TritValue) ToKorean() string {
	switch t {
	case P:
		return "성공"
	case T:
		return "실패"
	default:
		return "보류"
	}
}

// String implements Stringer
func (t TritValue) String() string {
	return string(t)
}

// TritFrom converts a number to TritValue
func TritFrom(n int) TritValue {
	if n > 0 {
		return P
	}
	if n < 0 {
		return T
	}
	return O
}

// TritNot returns logical NOT
func TritNot(t TritValue) TritValue {
	switch t {
	case P:
		return T
	case T:
		return P
	default:
		return O
	}
}

// TritAnd returns logical AND (min)
func TritAnd(a, b TritValue) TritValue {
	na, nb := a.ToNumber(), b.ToNumber()
	if na < nb {
		return TritFrom(na)
	}
	return TritFrom(nb)
}

// TritOr returns logical OR (max)
func TritOr(a, b TritValue) TritValue {
	na, nb := a.ToNumber(), b.ToNumber()
	if na > nb {
		return TritFrom(na)
	}
	return TritFrom(nb)
}

// Consensus performs majority vote
func Consensus(trits []TritValue) TritValue {
	pCount, tCount := 0, 0
	for _, t := range trits {
		switch t {
		case P:
			pCount++
		case T:
			tCount++
		}
	}
	if pCount > tCount {
		return P
	}
	if tCount > pCount {
		return T
	}
	return O
}

// ═══════════════════════════════════════════════
// TritResult
// ═══════════════════════════════════════════════

// TritResult is the standard return type for all operations
type TritResult struct {
	State     TritValue   `json:"state"`
	Data      interface{} `json:"data"`
	ElapsedMs int64       `json:"elapsed_ms"`
	TaskID    int64       `json:"task_id"`
}

// IsSuccess returns true if state is P
func (r TritResult) IsSuccess() bool { return r.State == P }

// IsPending returns true if state is O
func (r TritResult) IsPending() bool { return r.State == O }

// IsFailed returns true if state is T
func (r TritResult) IsFailed() bool { return r.State == T }

// ═══════════════════════════════════════════════
// CTP Header
// ═══════════════════════════════════════════════

// CtpHeader represents a 9-Trit CTP protocol header
type CtpHeader struct {
	Trits [9]TritValue
}

// NewCtpHeader creates a default header
func NewCtpHeader() CtpHeader {
	return CtpHeader{Trits: [9]TritValue{O, O, O, O, O, O, O, O, O}}
}

// CtpSuccess creates a success header
func CtpSuccess() CtpHeader {
	return CtpHeader{Trits: [9]TritValue{P, P, P, O, O, O, O, O, O}}
}

// CtpFailed creates a failure header
func CtpFailed() CtpHeader {
	return CtpHeader{Trits: [9]TritValue{T, T, T, O, O, O, O, O, O}}
}

// ParseCtpHeader parses "PPPOOOOOT" string
func ParseCtpHeader(s string) CtpHeader {
	h := NewCtpHeader()
	for i, c := range s {
		if i >= 9 {
			break
		}
		switch c {
		case 'P', '+', '1':
			h.Trits[i] = P
		case 'T', '-':
			h.Trits[i] = T
		default:
			h.Trits[i] = O
		}
	}
	return h
}

// String returns the 9-character header
func (h CtpHeader) String() string {
	var sb strings.Builder
	for _, t := range h.Trits {
		sb.WriteString(string(t))
	}
	return sb.String()
}

// OverallState returns the combined state
func (h CtpHeader) OverallState() TritValue {
	for _, t := range h.Trits {
		if t == T {
			return T
		}
	}
	allP := true
	for _, t := range h.Trits {
		if t != P {
			allP = false
			break
		}
	}
	if allP {
		return P
	}
	return O
}

// ═══════════════════════════════════════════════
// Client
// ═══════════════════════════════════════════════

// Client is the main Crowny SDK client
type Client struct {
	baseURL    string
	httpClient *http.Client
	ctp        CtpHeader
	taskCount  int64
	history    []TritResult
	mu         sync.Mutex
}

// NewClient creates a new Crowny client
func NewClient(baseURL string) *Client {
	return &Client{
		baseURL: strings.TrimRight(baseURL, "/"),
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
		ctp:     CtpSuccess(),
		history: make([]TritResult, 0),
	}
}

// WithTimeout sets the HTTP timeout
func (c *Client) WithTimeout(d time.Duration) *Client {
	c.httpClient.Timeout = d
	return c
}

// WithCTP sets the CTP header
func (c *Client) WithCTP(ctp CtpHeader) *Client {
	c.ctp = ctp
	return c
}

// ── 핵심: Submit ──

// Submit sends a task to the Crowny server via CAR
func (c *Client) Submit(taskType, subject, payload string, params map[string]string) (TritResult, error) {
	c.mu.Lock()
	c.taskCount++
	taskID := c.taskCount
	c.mu.Unlock()

	start := time.Now()

	body := map[string]interface{}{
		"type":    taskType,
		"subject": subject,
		"payload": payload,
		"params":  params,
	}

	data, err := json.Marshal(body)
	if err != nil {
		result := TritResult{State: T, Data: err.Error(), ElapsedMs: elapsed(start), TaskID: taskID}
		c.addHistory(result)
		return result, err
	}

	req, err := http.NewRequest("POST", c.baseURL+"/run", bytes.NewReader(data))
	if err != nil {
		result := TritResult{State: T, Data: err.Error(), ElapsedMs: elapsed(start), TaskID: taskID}
		c.addHistory(result)
		return result, err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("X-Crowny-Trit", c.ctp.String())
	req.Header.Set("X-Crowny-Version", "1.0")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		result := TritResult{State: T, Data: err.Error(), ElapsedMs: elapsed(start), TaskID: taskID}
		c.addHistory(result)
		return result, err
	}
	defer resp.Body.Close()

	// Parse CTP response header
	if ctpStr := resp.Header.Get("X-Crowny-Trit"); ctpStr != "" {
		c.ctp = ParseCtpHeader(ctpStr)
	}

	respBody, _ := io.ReadAll(resp.Body)
	var respData map[string]interface{}
	json.Unmarshal(respBody, &respData)

	state := parseTritFromResponse(respData)
	result := TritResult{
		State:     state,
		Data:      respData,
		ElapsedMs: elapsed(start),
		TaskID:    taskID,
	}
	c.addHistory(result)
	return result, nil
}

// ── 편의 메서드 ──

// Run executes 한선어 source code
func (c *Client) Run(source string) (TritResult, error) {
	return c.Submit("execute", "sdk-go", source, nil)
}

// Compile compiles to WASM
func (c *Client) Compile(source string) (TritResult, error) {
	return c.Submit("compile", "sdk-go", source, nil)
}

// Ask calls an LLM
func (c *Client) Ask(prompt string) (TritResult, error) {
	return c.Submit("llm", "claude", prompt, nil)
}

// AskModel calls a specific LLM model
func (c *Client) AskModel(prompt, model string) (TritResult, error) {
	return c.Submit("llm", model, prompt, nil)
}

// ── 합의 ──

// ConsensusResult holds multi-model consensus
type ConsensusResult struct {
	Consensus TritValue
	Models    []ModelResult
	Trits     []TritValue
	CTP       CtpHeader
	ElapsedMs int64
}

// ModelResult is a single model's result
type ModelResult struct {
	Model  string
	Result TritResult
}

// ConsensusCall performs multi-model consensus
func (c *Client) ConsensusCall(prompt string, models []string) (ConsensusResult, error) {
	start := time.Now()

	if len(models) == 0 {
		models = []string{"claude", "gpt4", "gemini"}
	}

	var wg sync.WaitGroup
	results := make([]ModelResult, len(models))
	errors := make([]error, len(models))

	for i, model := range models {
		wg.Add(1)
		go func(idx int, m string) {
			defer wg.Done()
			r, err := c.AskModel(prompt, m)
			results[idx] = ModelResult{Model: m, Result: r}
			errors[idx] = err
		}(i, model)
	}
	wg.Wait()

	trits := make([]TritValue, len(results))
	for i, r := range results {
		trits[i] = r.Result.State
	}

	con := Consensus(trits)

	return ConsensusResult{
		Consensus: con,
		Models:    results,
		Trits:     trits,
		CTP:       CtpHeader{Trits: [9]TritValue{con, trits[0], trits[1], trits[2], O, O, O, O, O}},
		ElapsedMs: elapsed(start),
	}, nil
}

// ── 상태 ──

// Ping checks server connectivity
func (c *Client) Ping() (TritResult, error) {
	start := time.Now()
	resp, err := c.httpClient.Get(c.baseURL + "/")
	if err != nil {
		return TritResult{State: T, Data: "unreachable", ElapsedMs: elapsed(start)}, err
	}
	defer resp.Body.Close()

	return TritResult{State: P, Data: "ok", ElapsedMs: elapsed(start)}, nil
}

// History returns all past results
func (c *Client) History() []TritResult {
	c.mu.Lock()
	defer c.mu.Unlock()
	h := make([]TritResult, len(c.history))
	copy(h, c.history)
	return h
}

// Stats returns P/O/T counts
func (c *Client) Stats() (total, p, o, t int) {
	c.mu.Lock()
	defer c.mu.Unlock()
	total = len(c.history)
	for _, r := range c.history {
		switch r.State {
		case P:
			p++
		case O:
			o++
		case T:
			t++
		}
	}
	return
}

// ── 내부 ──

func (c *Client) addHistory(r TritResult) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.history = append(c.history, r)
	if len(c.history) > 1000 {
		c.history = c.history[len(c.history)-1000:]
	}
}

func elapsed(start time.Time) int64 {
	return time.Since(start).Milliseconds()
}

func parseTritFromResponse(data map[string]interface{}) TritValue {
	for _, key := range []string{"상태", "state", "status"} {
		if v, ok := data[key]; ok {
			s := fmt.Sprintf("%v", v)
			if strings.Contains(s, "P") || strings.Contains(s, "성공") || strings.Contains(s, "Success") {
				return P
			}
			if strings.Contains(s, "T") || strings.Contains(s, "실패") || strings.Contains(s, "Failed") {
				return T
			}
		}
	}
	return O
}
