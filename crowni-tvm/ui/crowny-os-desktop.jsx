import { useState, useEffect, useRef, useCallback } from "react";

// ═══ CROWNY OS DESKTOP ═══
const APPS = {
  terminal: { name: "터미널", icon: "⌨", color: "#0a0a0a" },
  files: { name: "파일관리자", icon: "📁", color: "#1a1a2e" },
  monitor: { name: "시스템모니터", icon: "📊", color: "#0f1729" },
  exchange: { name: "거래소", icon: "📈", color: "#0d1117" },
  consensus: { name: "합의엔진", icon: "🗳", color: "#111827" },
  editor: { name: "에디터", icon: "✏", color: "#1e1e1e" },
  browser: { name: "브라우저", icon: "🌐", color: "#0c1222" },
  wallet: { name: "지갑", icon: "💎", color: "#0f0f1a" },
};

const TRIT_COLORS = { P: "#00e68a", O: "#ffb347", T: "#ff5555" };
const tritLabel = (t) => (t > 0 ? "P" : t < 0 ? "T" : "O");
const tritColor = (t) => TRIT_COLORS[tritLabel(t)];

// ═══ FILESYSTEM ═══
const FS_TREE = {
  "/": {
    type: "dir", children: {
      bin: { type: "dir", children: { tvm: { type: "file", size: 32 }, hanseon: { type: "file", size: 36 }, crwnsh: { type: "file", size: 35 }, cpm: { type: "file", size: 32 } } },
      etc: { type: "dir", children: { "crowny.conf": { type: "file", size: 81, content: "# Crowny OS Config\nversion=0.10.0\ntrit_mode=balanced\nconsensus=3\nport=3333" }, hosts: { type: "file", size: 60, content: "127.0.0.1 localhost\n127.0.0.1 crowny" } } },
      home: { type: "dir", children: { ef: { type: "dir", children: { ".crwnrc": { type: "file", size: 82, content: 'PROMPT="crwn> "\nPATH=/bin:/usr/bin\nTHEME=dark' }, "hello.hsn": { type: "file", size: 45, content: '값 "안녕하세요!" 보여줘 끝' }, "notes.md": { type: "file", size: 120, content: "# Crowny 개발 노트\n\n- v0.10.0 블록체인 완성\n- 36 모듈, 191 테스트\n- 다음: OS UI + 앱" } } } } },
      crwn: { type: "dir", children: { tvm: { type: "dir", children: {} }, hanseon: { type: "dir", children: {} }, consensus: { type: "dir", children: {} }, tokens: { type: "dir", children: {} }, chain: { type: "dir", children: {} } } },
      var: { type: "dir", children: { log: { type: "dir", children: { "system.log": { type: "file", size: 200, content: "[P] CrownyOS booted\n[P] 8 daemons started\n[P] TritFS mounted\n[P] Chain synced: height 3" } } } } },
      tmp: { type: "dir", children: {} },
    }
  }
};

const resolvePath = (path) => {
  const parts = path.split("/").filter(Boolean);
  let node = FS_TREE["/"];
  for (const p of parts) {
    if (!node.children || !node.children[p]) return null;
    node = node.children[p];
  }
  return node;
};

const listDir = (path) => {
  const node = resolvePath(path);
  if (!node || node.type !== "dir") return [];
  return Object.entries(node.children || {}).map(([name, n]) => ({ name, ...n }));
};

// ═══ PROCESS MANAGER ═══
const INIT_PROCS = [
  { pid: 0, name: "crowny-kernel", state: "P", cpu: 0.3, mem: 2048, pri: "높음" },
  { pid: 1, name: "trit-init", state: "P", cpu: 0.1, mem: 1024, pri: "높음" },
  { pid: 2, name: "trit-scheduler", state: "P", cpu: 1.2, mem: 4096, pri: "높음" },
  { pid: 3, name: "consensus-daemon", state: "P", cpu: 2.5, mem: 8192, pri: "높음" },
  { pid: 4, name: "tvm-runtime", state: "P", cpu: 4.1, mem: 16384, pri: "보통" },
  { pid: 5, name: "ctp-server", state: "P", cpu: 1.8, mem: 4096, pri: "보통" },
  { pid: 6, name: "trit-logger", state: "P", cpu: 0.5, mem: 1024, pri: "낮음" },
  { pid: 7, name: "wallet-daemon", state: "P", cpu: 0.8, mem: 2048, pri: "낮음" },
];

// ═══ CHAIN DATA ═══
const CHAIN_BLOCKS = [
  { index: 0, txs: 1, validator: "genesis", hash: "0tPOTPPOTTPOPPT", fees: 0, reward: 100, trit: 1 },
  { index: 1, txs: 9, validator: "Alice-Node", hash: "0tTPPPPOTPOPPTP", fees: 47, reward: 100, trit: 1 },
  { index: 2, txs: 3, validator: "Alice-Node", hash: "0tPPPTOPPPPTTOP", fees: 8, reward: 100, trit: 1 },
  { index: 3, txs: 3, validator: "Bob-Node", hash: "0tPPOPOPPTPPPPT", fees: 8, reward: 100, trit: 1 },
];

// ═══ WINDOW MANAGER ═══
function WindowFrame({ id, title, icon, color, children, onClose, onFocus, zIndex, pos }) {
  const [dragging, setDragging] = useState(false);
  const [position, setPosition] = useState(pos || { x: 60 + (id * 30) % 200, y: 40 + (id * 25) % 150 });
  const [size, setSize] = useState({ w: 680, h: 460 });
  const [maximized, setMaximized] = useState(false);
  const dragRef = useRef(null);

  const handleMouseDown = (e) => {
    if (maximized) return;
    setDragging(true);
    dragRef.current = { sx: e.clientX - position.x, sy: e.clientY - position.y };
    onFocus();
  };

  useEffect(() => {
    if (!dragging) return;
    const move = (e) => setPosition({ x: e.clientX - dragRef.current.sx, y: e.clientY - dragRef.current.sy });
    const up = () => setDragging(false);
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
    return () => { window.removeEventListener("mousemove", move); window.removeEventListener("mouseup", up); };
  }, [dragging]);

  const style = maximized
    ? { left: 0, top: 28, width: "100%", height: "calc(100% - 68px)", borderRadius: 0 }
    : { left: position.x, top: position.y, width: size.w, height: size.h };

  return (
    <div onClick={onFocus} style={{
      position: "absolute", ...style, zIndex, display: "flex", flexDirection: "column",
      background: color || "#0a0a0a", border: "1px solid #333", borderRadius: maximized ? 0 : 8,
      boxShadow: "0 8px 32px rgba(0,0,0,0.6)", overflow: "hidden", fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
    }}>
      <div onMouseDown={handleMouseDown} onDoubleClick={() => setMaximized(!maximized)} style={{
        height: 32, background: "linear-gradient(180deg, #2a2a2a 0%, #1a1a1a 100%)", display: "flex",
        alignItems: "center", padding: "0 10px", cursor: dragging ? "grabbing" : "grab", flexShrink: 0,
        borderBottom: "1px solid #333",
      }}>
        <div style={{ display: "flex", gap: 6, marginRight: 12 }}>
          <div onClick={(e) => { e.stopPropagation(); onClose(); }} style={{ width: 12, height: 12, borderRadius: "50%", background: "#ff5f57", cursor: "pointer" }} />
          <div onClick={(e) => { e.stopPropagation(); setMaximized(!maximized); }} style={{ width: 12, height: 12, borderRadius: "50%", background: "#febc2e", cursor: "pointer" }} />
          <div style={{ width: 12, height: 12, borderRadius: "50%", background: "#28c840" }} />
        </div>
        <span style={{ fontSize: 13, color: "#ccc", userSelect: "none" }}>{icon} {title}</span>
      </div>
      <div style={{ flex: 1, overflow: "auto", position: "relative" }}>{children}</div>
    </div>
  );
}

// ═══ APP: TERMINAL ═══
function TerminalApp() {
  const [lines, setLines] = useState([
    { type: "sys", text: "CrownyOS v0.10.0 (Balanced Ternary) aarch64" },
    { type: "sys", text: "TritShell v1.0 — Type 'help' for commands" },
    { type: "sys", text: "" },
  ]);
  const [input, setInput] = useState("");
  const [cwd, setCwd] = useState("/home/ef");
  const [procs, setProcs] = useState([...INIT_PROCS]);
  const [pidCounter, setPidCounter] = useState(8);
  const endRef = useRef(null);

  useEffect(() => { endRef.current?.scrollIntoView({ behavior: "smooth" }); }, [lines]);

  const exec = (cmd) => {
    const parts = cmd.trim().split(/\s+/);
    const out = [];
    const c = parts[0];

    if (c === "ls") {
      listDir(cwd).forEach((f) => {
        const icon = f.type === "dir" ? "📁" : "📄";
        const sz = f.type === "file" ? `${f.size || 0}B` : "-";
        out.push({ type: "out", text: `  [P] ${icon} ${f.name.padEnd(18)} ${sz}`, trit: 1 });
      });
    } else if (c === "cd") {
      const target = parts[1] || "/";
      if (target === "/") setCwd("/");
      else if (target === "..") setCwd(cwd.split("/").slice(0, -1).join("/") || "/");
      else {
        const newPath = cwd === "/" ? `/${target}` : `${cwd}/${target}`;
        if (resolvePath(newPath)?.type === "dir") setCwd(newPath);
        else out.push({ type: "err", text: `  [T] cd: '${target}' 없음`, trit: -1 });
      }
    } else if (c === "cat") {
      const name = parts[1];
      const path = cwd === "/" ? `/${name}` : `${cwd}/${name}`;
      const node = resolvePath(path);
      if (node?.content) node.content.split("\n").forEach((l) => out.push({ type: "out", text: `  ${l}` }));
      else out.push({ type: "err", text: `  [T] cat: '${name}' 없음`, trit: -1 });
    } else if (c === "pwd") {
      out.push({ type: "out", text: `  ${cwd}` });
    } else if (c === "ps") {
      out.push({ type: "out", text: "  PID  STATE   PRI    MEM      CPU   NAME" });
      out.push({ type: "out", text: "  ───  ─────   ───    ───      ───   ────" });
      procs.filter(p => p.state !== "Z").forEach((p) => {
        out.push({ type: "out", text: `  [${p.state}] ${String(p.pid).padStart(3)}  ${p.pri.padEnd(4)} ${String(p.mem + "KB").padStart(8)} ${(p.cpu + "%").padStart(6)}   ${p.name}`, trit: p.state === "P" ? 1 : p.state === "T" ? -1 : 0 });
      });
    } else if (c === "spawn") {
      const name = parts[1] || "app";
      const mem = parseInt(parts[2]) || 512;
      const newPid = pidCounter;
      setProcs([...procs, { pid: newPid, name, state: "P", cpu: Math.random() * 2, mem, pri: "보통" }]);
      setPidCounter(pidCounter + 1);
      out.push({ type: "out", text: `  [P] spawn PID:${newPid} '${name}' (${mem}KB)`, trit: 1 });
    } else if (c === "kill") {
      const pid = parseInt(parts[1]);
      if (pid <= 1) out.push({ type: "err", text: "  [T] 커널/init 종료 불가", trit: -1 });
      else {
        setProcs(procs.map(p => p.pid === pid ? { ...p, state: "Z" } : p));
        out.push({ type: "out", text: `  [P] kill PID:${pid}`, trit: 1 });
      }
    } else if (c === "uname") {
      out.push({ type: "out", text: "  CrownyOS 0.10.0 (Balanced Ternary) aarch64" });
    } else if (c === "whoami") {
      out.push({ type: "out", text: "  ef" });
    } else if (c === "clear") {
      setLines([]); return;
    } else if (c === "stat") {
      const running = procs.filter(p => p.state === "P").length;
      const totalMem = procs.reduce((s, p) => s + p.mem, 0);
      out.push({ type: "out", text: `  프로세스: ${procs.length} (실행:${running})` });
      out.push({ type: "out", text: `  메모리: ${totalMem}KB / 524288KB (${(totalMem / 524288 * 100).toFixed(1)}%)` });
      out.push({ type: "out", text: `  체인: 높이 3 | 4블록 | 16TX | PoT 합의` });
    } else if (c === "help") {
      ["ls, cd, cat, pwd — 파일 탐색", "ps, spawn, kill — 프로세스 관리",
       "stat, uname, whoami — 시스템 정보", "clear — 화면 지우기"].forEach(l =>
        out.push({ type: "out", text: `  ${l}` }));
    } else if (c) {
      out.push({ type: "err", text: `  [T] crwnsh: '${c}' 명령어를 찾을 수 없습니다`, trit: -1 });
    }
    return out;
  };

  const handleSubmit = () => {
    if (!input.trim()) return;
    const prompt = `[${tritLabel(1)}] ef@crowny ${cwd}> ${input}`;
    const result = exec(input);
    setLines((prev) => [...prev, { type: "prompt", text: prompt }, ...(result || [])]);
    setInput("");
  };

  return (
    <div style={{ height: "100%", background: "#0a0a0a", color: "#e0e0e0", fontSize: 13, padding: 8, display: "flex", flexDirection: "column" }}>
      <div style={{ flex: 1, overflow: "auto", whiteSpace: "pre" }}>
        {lines.map((l, i) => (
          <div key={i} style={{ color: l.type === "err" ? "#ff5555" : l.type === "prompt" ? "#00e68a" : l.trit ? tritColor(l.trit) : "#ccc", lineHeight: 1.5 }}>
            {l.text}
          </div>
        ))}
        <div ref={endRef} />
      </div>
      <div style={{ display: "flex", alignItems: "center", borderTop: "1px solid #222", paddingTop: 6 }}>
        <span style={{ color: "#00e68a", marginRight: 6, fontSize: 12 }}>[P] ef@crowny {cwd}{">"}</span>
        <input value={input} onChange={(e) => setInput(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
          style={{ flex: 1, background: "transparent", border: "none", color: "#fff", fontSize: 13, outline: "none", fontFamily: "inherit" }}
          autoFocus />
      </div>
    </div>
  );
}

// ═══ APP: FILE MANAGER ═══
function FilesApp() {
  const [cwd, setCwd] = useState("/");
  const [selected, setSelected] = useState(null);
  const entries = listDir(cwd);
  const node = selected ? resolvePath(cwd === "/" ? `/${selected}` : `${cwd}/${selected}`) : null;

  return (
    <div style={{ height: "100%", display: "flex", color: "#ddd", fontSize: 13 }}>
      <div style={{ width: 160, background: "#111", borderRight: "1px solid #333", padding: 8, overflow: "auto" }}>
        <div style={{ fontSize: 11, color: "#888", marginBottom: 8, textTransform: "uppercase", letterSpacing: 1 }}>즐겨찾기</div>
        {["/", "/home/ef", "/etc", "/crwn", "/var/log"].map((p) => (
          <div key={p} onClick={() => { setCwd(p); setSelected(null); }} style={{
            padding: "5px 8px", cursor: "pointer", borderRadius: 4, marginBottom: 2,
            background: cwd === p ? "#00e68a22" : "transparent", color: cwd === p ? "#00e68a" : "#aaa",
          }}>{p === "/" ? "📁 /" : `📁 ${p.split("/").pop()}`}</div>
        ))}
      </div>
      <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        <div style={{ padding: "8px 12px", background: "#1a1a1a", borderBottom: "1px solid #333", display: "flex", alignItems: "center", gap: 8 }}>
          <button onClick={() => { setCwd(cwd.split("/").slice(0, -1).join("/") || "/"); setSelected(null); }}
            style={{ background: "#333", border: "none", color: "#ccc", padding: "3px 10px", borderRadius: 4, cursor: "pointer", fontSize: 12 }}>⬆ 상위</button>
          <span style={{ color: "#00e68a", fontFamily: "monospace" }}>{cwd}</span>
        </div>
        <div style={{ flex: 1, padding: 12, overflow: "auto", display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(100px, 1fr))", gap: 8, alignContent: "start" }}>
          {entries.map((f) => (
            <div key={f.name} onClick={() => setSelected(f.name)} onDoubleClick={() => {
              if (f.type === "dir") { setCwd(cwd === "/" ? `/${f.name}` : `${cwd}/${f.name}`); setSelected(null); }
            }} style={{
              padding: 10, borderRadius: 6, cursor: "pointer", textAlign: "center",
              background: selected === f.name ? "#00e68a22" : "#1a1a1a",
              border: selected === f.name ? "1px solid #00e68a55" : "1px solid transparent",
            }}>
              <div style={{ fontSize: 28 }}>{f.type === "dir" ? "📁" : "📄"}</div>
              <div style={{ fontSize: 11, marginTop: 4, wordBreak: "break-all", color: f.type === "dir" ? "#00e68a" : "#ccc" }}>{f.name}</div>
              {f.type === "file" && <div style={{ fontSize: 10, color: "#666" }}>{f.size}B</div>}
            </div>
          ))}
          {entries.length === 0 && <div style={{ color: "#555", gridColumn: "1/-1", textAlign: "center", padding: 40 }}>빈 디렉토리</div>}
        </div>
      </div>
      {node?.content && (
        <div style={{ width: 220, background: "#111", borderLeft: "1px solid #333", padding: 10, overflow: "auto" }}>
          <div style={{ fontSize: 11, color: "#888", marginBottom: 6 }}>미리보기</div>
          <div style={{ fontSize: 11, color: "#00e68a", marginBottom: 8 }}>{selected}</div>
          <pre style={{ fontSize: 11, color: "#aaa", whiteSpace: "pre-wrap", margin: 0, lineHeight: 1.5 }}>{node.content}</pre>
        </div>
      )}
    </div>
  );
}

// ═══ APP: SYSTEM MONITOR ═══
function MonitorApp() {
  const [tick, setTick] = useState(0);
  const [procs] = useState(INIT_PROCS);
  useEffect(() => { const i = setInterval(() => setTick((t) => t + 1), 1500); return () => clearInterval(i); }, []);

  const cpuHistory = Array.from({ length: 20 }, (_, i) => 8 + Math.sin((tick + i) * 0.4) * 4 + Math.random() * 3);
  const memUsed = procs.reduce((s, p) => s + p.mem, 0);
  const memTotal = 524288;
  const memPct = (memUsed / memTotal * 100).toFixed(1);

  return (
    <div style={{ height: "100%", color: "#ddd", fontSize: 12, padding: 12, overflow: "auto" }}>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, marginBottom: 12 }}>
        <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
          <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>CPU 사용량</div>
          <svg width="100%" height="80" viewBox="0 0 300 80">
            <polyline fill="none" stroke="#00e68a" strokeWidth="2"
              points={cpuHistory.map((v, i) => `${i * 15},${80 - v * 2}`).join(" ")} />
            <polyline fill="url(#cpuGrad)" stroke="none"
              points={`0,80 ${cpuHistory.map((v, i) => `${i * 15},${80 - v * 2}`).join(" ")} 285,80`} />
            <defs><linearGradient id="cpuGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#00e68a" stopOpacity="0.3" /><stop offset="100%" stopColor="#00e68a" stopOpacity="0" />
            </linearGradient></defs>
          </svg>
          <div style={{ color: "#00e68a", fontSize: 18, fontWeight: "bold" }}>{cpuHistory[cpuHistory.length - 1].toFixed(1)}%</div>
        </div>
        <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
          <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>메모리</div>
          <div style={{ position: "relative", height: 80, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <svg width="80" height="80" viewBox="0 0 36 36">
              <circle cx="18" cy="18" r="15" fill="none" stroke="#222" strokeWidth="3" />
              <circle cx="18" cy="18" r="15" fill="none" stroke="#00e68a" strokeWidth="3"
                strokeDasharray={`${memPct} ${100 - memPct}`} strokeDashoffset="25" strokeLinecap="round" />
            </svg>
          </div>
          <div style={{ textAlign: "center" }}>
            <span style={{ color: "#00e68a", fontSize: 18, fontWeight: "bold" }}>{memPct}%</span>
            <span style={{ color: "#666", fontSize: 11, marginLeft: 6 }}>{(memUsed / 1024).toFixed(0)}MB / {(memTotal / 1024).toFixed(0)}MB</span>
          </div>
        </div>
      </div>
      <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
        <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>프로세스 ({procs.length})</div>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 11 }}>
          <thead><tr style={{ color: "#666", borderBottom: "1px solid #222" }}>
            <td style={{ padding: 4 }}>PID</td><td>이름</td><td>상태</td><td>CPU</td><td>메모리</td>
          </tr></thead>
          <tbody>{procs.map((p) => (
            <tr key={p.pid} style={{ borderBottom: "1px solid #1a1a1a" }}>
              <td style={{ padding: 4, color: "#888" }}>{p.pid}</td>
              <td style={{ color: "#ddd" }}>{p.name}</td>
              <td><span style={{ color: tritColor(1), fontSize: 10, padding: "1px 6px", background: "#00e68a15", borderRadius: 3 }}>● {p.state}</span></td>
              <td style={{ color: p.cpu > 3 ? "#ffb347" : "#888" }}>{(p.cpu + Math.random() * 0.5).toFixed(1)}%</td>
              <td style={{ color: "#888" }}>{p.mem}KB</td>
            </tr>
          ))}</tbody>
        </table>
      </div>
    </div>
  );
}

// ═══ APP: EXCHANGE ═══
function ExchangeApp() {
  const [tick, setTick] = useState(0);
  useEffect(() => { const i = setInterval(() => setTick((t) => t + 1), 2000); return () => clearInterval(i); }, []);
  const price = 0.124 + Math.sin(tick * 0.3) * 0.008 + Math.random() * 0.003;
  const change = ((price - 0.124) / 0.124 * 100).toFixed(2);
  const prices = Array.from({ length: 24 }, (_, i) => 0.115 + Math.sin((tick + i) * 0.25) * 0.01 + Math.random() * 0.005);
  const accounts = [
    { name: "alice", bal: 862962, staked: 100000 },
    { name: "bob", bal: 426485, staked: 80000 },
    { name: "carol", bal: 263497, staked: 50000 },
  ];

  return (
    <div style={{ height: "100%", color: "#ddd", fontSize: 12, padding: 12, overflow: "auto" }}>
      <div style={{ display: "flex", gap: 12, marginBottom: 12 }}>
        <div style={{ flex: 1, background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
          <div style={{ fontSize: 11, color: "#888" }}>CRWN/USDT</div>
          <div style={{ fontSize: 28, fontWeight: "bold", color: change >= 0 ? "#00e68a" : "#ff5555" }}>${price.toFixed(4)}</div>
          <div style={{ color: change >= 0 ? "#00e68a" : "#ff5555", fontSize: 13 }}>{change >= 0 ? "▲" : "▼"} {change}%</div>
        </div>
        <div style={{ flex: 1, background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
          <div style={{ fontSize: 11, color: "#888" }}>24h 거래량</div>
          <div style={{ fontSize: 20, fontWeight: "bold", color: "#ffb347" }}>$45.2M</div>
          <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>유통: 153,000,000 CRWN</div>
        </div>
      </div>
      <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222", marginBottom: 12 }}>
        <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>가격 차트 (24h)</div>
        <svg width="100%" height="100" viewBox="0 0 300 100">
          <polyline fill="none" stroke="#00e68a" strokeWidth="2"
            points={prices.map((v, i) => `${i * 13},${100 - (v - 0.110) * 2500}`).join(" ")} />
          <polyline fill="url(#priceGrad)" stroke="none"
            points={`0,100 ${prices.map((v, i) => `${i * 13},${100 - (v - 0.110) * 2500}`).join(" ")} 299,100`} />
          <defs><linearGradient id="priceGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#00e68a" stopOpacity="0.2" /><stop offset="100%" stopColor="#00e68a" stopOpacity="0" />
          </linearGradient></defs>
        </svg>
      </div>
      <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
        <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>계정 잔액</div>
        {accounts.map((a) => (
          <div key={a.name} style={{ display: "flex", justifyContent: "space-between", padding: "6px 0", borderBottom: "1px solid #1a1a1a" }}>
            <span style={{ color: "#00e68a" }}>💎 {a.name}</span>
            <span>{a.bal.toLocaleString()} CRWN</span>
            <span style={{ color: "#888", fontSize: 11 }}>staked: {a.staked.toLocaleString()}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ═══ APP: CONSENSUS ENGINE ═══
function ConsensusApp() {
  const [votes, setVotes] = useState([]);
  const [query, setQuery] = useState("");
  const nodes = [
    { name: "Claude", port: 18789, status: "P" },
    { name: "Gemini", port: 18790, status: "P" },
    { name: "Sonnet", port: 18791, status: "P" },
  ];
  const responses = ["기술적으로 적합", "리스크 낮음, 진행 추천", "추가 검토 필요", "데이터 부족, 보류", "부적합, 거부"];

  const runConsensus = () => {
    if (!query.trim()) return;
    const newVotes = nodes.map((n) => {
      const trit = Math.random() > 0.3 ? 1 : Math.random() > 0.5 ? 0 : -1;
      return { name: n.name, trit, reason: responses[Math.floor(Math.random() * responses.length)] };
    });
    setVotes(newVotes);
  };
  const p = votes.filter(v => v.trit > 0).length;
  const t = votes.filter(v => v.trit < 0).length;
  const consensus = votes.length > 0 ? (p > t ? 1 : t > p ? -1 : 0) : null;

  return (
    <div style={{ height: "100%", color: "#ddd", fontSize: 12, padding: 12, overflow: "auto" }}>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        {nodes.map((n) => (
          <div key={n.name} style={{ flex: 1, background: "#111", borderRadius: 8, padding: 10, border: "1px solid #222", textAlign: "center" }}>
            <div style={{ fontSize: 24, marginBottom: 4 }}>🤖</div>
            <div style={{ color: "#00e68a", fontWeight: "bold" }}>{n.name}</div>
            <div style={{ fontSize: 10, color: "#666" }}>:{n.port}</div>
            <div style={{ marginTop: 4 }}><span style={{ color: "#00e68a", fontSize: 10, padding: "2px 8px", background: "#00e68a15", borderRadius: 10 }}>● 온라인</span></div>
          </div>
        ))}
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <input value={query} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => e.key === "Enter" && runConsensus()}
          placeholder="합의 질문 입력..." style={{
            flex: 1, background: "#111", border: "1px solid #333", color: "#fff", padding: "8px 12px",
            borderRadius: 6, fontSize: 13, outline: "none", fontFamily: "inherit",
          }} />
        <button onClick={runConsensus} style={{ background: "#00e68a", color: "#000", border: "none", padding: "8px 16px", borderRadius: 6, fontWeight: "bold", cursor: "pointer", fontSize: 12 }}>
          🗳 합의
        </button>
      </div>
      {votes.length > 0 && (
        <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
          <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>투표 결과</div>
          {votes.map((v, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "6px 0", borderBottom: "1px solid #1a1a1a" }}>
              <span style={{ color: tritColor(v.trit), fontWeight: "bold", width: 24, textAlign: "center", fontSize: 14 }}>{tritLabel(v.trit)}</span>
              <span style={{ color: "#00e68a", width: 60 }}>{v.name}</span>
              <span style={{ color: "#aaa", flex: 1 }}>{v.reason}</span>
            </div>
          ))}
          <div style={{ marginTop: 12, padding: 10, borderRadius: 6, background: `${tritColor(consensus)}11`, border: `1px solid ${tritColor(consensus)}33`, textAlign: "center" }}>
            <span style={{ color: tritColor(consensus), fontSize: 20, fontWeight: "bold" }}>{tritLabel(consensus)}</span>
            <span style={{ color: "#aaa", marginLeft: 8 }}>
              ({consensus > 0 ? "승인" : consensus < 0 ? "거부" : "보류"}) — 신뢰도 {((Math.max(p, t, votes.length - p - t) / votes.length) * 100).toFixed(0)}%
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

// ═══ APP: EDITOR ═══
function EditorApp() {
  const [files] = useState([
    { name: "hello.hsn", content: '값 "안녕하세요!" 보여줘\n값 42 값 58 더하기 보여줘\n끝' },
    { name: "crowny.conf", content: "# Crowny OS Config\nversion=0.10.0\ntrit_mode=balanced\nconsensus=3" },
    { name: "index.crwn", content: '제목: My App\n\n# Hello Crowny\n\n[P] 3진법 웹페이지입니다\n[O] 보류 중인 항목\n\n스크립트: 합의 PPO' },
  ]);
  const [active, setActive] = useState(0);
  const [content, setContent] = useState(files[0].content);
  const [output, setOutput] = useState([]);

  const run = () => {
    const out = [];
    content.split("\n").forEach((line) => {
      const t = line.trim();
      if (t.startsWith("값")) out.push(`> ${t}`);
      else if (t.startsWith("[P]")) out.push(`[P] ${t.slice(3).trim()}`);
      else if (t.startsWith("[O]")) out.push(`[O] ${t.slice(3).trim()}`);
      else if (t.startsWith("[T]")) out.push(`[T] ${t.slice(3).trim()}`);
      else if (t.startsWith("합의")) out.push(`합의 결과: P (67%)`);
    });
    if (out.length === 0) out.push("[P] 실행 완료");
    setOutput(out);
  };

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", color: "#ddd", fontSize: 12 }}>
      <div style={{ display: "flex", background: "#1a1a1a", borderBottom: "1px solid #333" }}>
        {files.map((f, i) => (
          <div key={i} onClick={() => { setActive(i); setContent(f.content); setOutput([]); }}
            style={{ padding: "6px 14px", cursor: "pointer", borderRight: "1px solid #333",
              background: active === i ? "#1e1e1e" : "#141414", color: active === i ? "#00e68a" : "#888", fontSize: 11 }}>
            {f.name}
          </div>
        ))}
        <div style={{ flex: 1 }} />
        <button onClick={run} style={{ background: "#00e68a", color: "#000", border: "none", padding: "4px 14px", margin: 2, borderRadius: 4, fontWeight: "bold", cursor: "pointer", fontSize: 11 }}>
          ▶ 실행
        </button>
      </div>
      <div style={{ flex: 1, display: "flex" }}>
        <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
          <textarea value={content} onChange={(e) => setContent(e.target.value)}
            style={{ flex: 1, background: "#1e1e1e", color: "#e0e0e0", border: "none", padding: 12, fontSize: 13,
              fontFamily: "'JetBrains Mono', monospace", resize: "none", outline: "none", lineHeight: 1.6 }} />
        </div>
        {output.length > 0 && (
          <div style={{ width: 240, background: "#0a0a0a", borderLeft: "1px solid #333", padding: 10, overflow: "auto" }}>
            <div style={{ fontSize: 10, color: "#888", marginBottom: 6 }}>출력</div>
            {output.map((l, i) => (
              <div key={i} style={{ fontSize: 11, lineHeight: 1.6, color: l.startsWith("[P]") ? "#00e68a" : l.startsWith("[T]") ? "#ff5555" : l.startsWith("[O]") ? "#ffb347" : "#ccc" }}>
                {l}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ═══ APP: BROWSER ═══
function BrowserApp() {
  const [url, setUrl] = useState("crwn://home");
  const pages = {
    "crwn://home": { title: "Crowny 홈", content: [
      { type: "h1", text: "크라운 브라우저" },
      { type: "p", text: "3진법 기반 차세대 웹 브라우저", trit: 1 },
      { type: "p", text: "CTP 프로토콜 — HTTP를 넘어서", trit: 1 },
      { type: "p", text: "TritDOM — 모든 요소에 P/O/T 상태", trit: 1 },
      { type: "hr" },
      { type: "h2", text: "추천 사이트" },
      { type: "link", text: "crwn://exchange — 거래소", url: "crwn://exchange" },
      { type: "link", text: "crwn://chain — 블록체인", url: "crwn://chain" },
      { type: "link", text: "crwn://docs — 문서", url: "crwn://docs" },
    ]},
    "crwn://exchange": { title: "CRWN 거래소", content: [
      { type: "h1", text: "CRWN 거래소" },
      { type: "p", text: "CRWN/USDT: $0.1240 (+2.5%)", trit: 1 },
      { type: "p", text: "24h 거래량: $45,000,000", trit: 0 },
      { type: "hr" },
      { type: "p", text: "alice: 862,962 CRWN (staked: 100,000)", trit: 1 },
      { type: "p", text: "bob: 426,485 CRWN (staked: 80,000)", trit: 1 },
    ]},
    "crwn://chain": { title: "블록체인", content: [
      { type: "h1", text: "Crowny Chain" },
      ...CHAIN_BLOCKS.map(b => ({ type: "p", text: `Block #${b.index} [${tritLabel(b.trit)}] — ${b.txs} txs | ${b.validator} | ${b.hash}`, trit: b.trit })),
      { type: "hr" },
      { type: "p", text: "높이: 3 | 4블록 | 16TX | PoT 합의", trit: 1 },
    ]},
    "crwn://docs": { title: "개발자 문서", content: [
      { type: "h1", text: "Crowny 개발자 문서" },
      { type: "h2", text: ".crwn 파일 형식" },
      { type: "p", text: "[P] — 승인 상태 텍스트", trit: 1 },
      { type: "p", text: "[O] — 보류 상태 텍스트", trit: 0 },
      { type: "p", text: "[T] — 거부 상태 텍스트", trit: -1 },
      { type: "hr" },
      { type: "h2", text: "CTP 메서드" },
      { type: "p", text: "GET / POST / SUBMIT / VOTE / SYNC", trit: 1 },
    ]},
  };
  const page = pages[url] || { title: "404", content: [{ type: "h1", text: "404 — 페이지 없음" }, { type: "p", text: `'${url}' 을 찾을 수 없습니다`, trit: -1 }] };

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", color: "#ddd", fontSize: 13 }}>
      <div style={{ display: "flex", gap: 6, padding: 6, background: "#151525", borderBottom: "1px solid #333" }}>
        <button onClick={() => setUrl("crwn://home")} style={{ background: "#222", border: "none", color: "#aaa", padding: "4px 8px", borderRadius: 4, cursor: "pointer", fontSize: 11 }}>🏠</button>
        <input value={url} onChange={(e) => setUrl(e.target.value)} onKeyDown={(e) => e.key === "Enter" && setUrl(e.target.value)}
          style={{ flex: 1, background: "#111", border: "1px solid #333", color: "#00e68a", padding: "4px 10px", borderRadius: 4, fontSize: 12, outline: "none", fontFamily: "monospace" }} />
        <span style={{ color: "#00e68a", fontSize: 10, padding: "4px 8px", background: "#00e68a15", borderRadius: 4 }}>● CTP</span>
      </div>
      <div style={{ flex: 1, padding: 16, overflow: "auto", background: "#0c1222" }}>
        {page.content.map((el, i) => {
          if (el.type === "h1") return <div key={i} style={{ fontSize: 22, fontWeight: "bold", color: "#fff", marginBottom: 12 }}>{el.text}</div>;
          if (el.type === "h2") return <div key={i} style={{ fontSize: 16, fontWeight: "bold", color: "#ccc", marginTop: 16, marginBottom: 8 }}>{el.text}</div>;
          if (el.type === "hr") return <hr key={i} style={{ border: "none", borderTop: "1px solid #333", margin: "12px 0" }} />;
          if (el.type === "link") return <div key={i} onClick={() => setUrl(el.url)} style={{ color: "#00e68a", cursor: "pointer", padding: "3px 0", textDecoration: "underline" }}>{el.text}</div>;
          return <div key={i} style={{ padding: "3px 0", color: el.trit != null ? tritColor(el.trit) : "#ccc" }}>
            {el.trit != null && <span style={{ marginRight: 6, fontWeight: "bold" }}>[{tritLabel(el.trit)}]</span>}{el.text}
          </div>;
        })}
      </div>
    </div>
  );
}

// ═══ APP: WALLET ═══
function WalletApp() {
  const [bal] = useState(989970);
  const [staked] = useState(100000);
  const [txs] = useState([
    { type: "전송", from: "alice", to: "bob", amount: 10000, trit: 1, time: "2분 전" },
    { type: "스테이킹", from: "alice", to: "network", amount: 100000, trit: 1, time: "1시간 전" },
    { type: "보상", from: "network", to: "alice", amount: 100, trit: 1, time: "3시간 전" },
    { type: "전송", from: "carol", to: "alice", amount: 5000, trit: 1, time: "5시간 전" },
    { type: "거부", from: "alice", to: "unknown", amount: 999999, trit: -1, time: "어제" },
  ]);

  return (
    <div style={{ height: "100%", color: "#ddd", fontSize: 12, padding: 12, overflow: "auto" }}>
      <div style={{ background: "linear-gradient(135deg, #1a0a3e, #0a2a1e)", borderRadius: 12, padding: 20, marginBottom: 12, border: "1px solid #333" }}>
        <div style={{ fontSize: 11, color: "#888" }}>총 자산</div>
        <div style={{ fontSize: 32, fontWeight: "bold", color: "#00e68a", marginTop: 4 }}>{bal.toLocaleString()} <span style={{ fontSize: 16 }}>CRWN</span></div>
        <div style={{ display: "flex", gap: 16, marginTop: 12 }}>
          <div><span style={{ color: "#888", fontSize: 11 }}>스테이킹</span><div style={{ color: "#ffb347" }}>{staked.toLocaleString()}</div></div>
          <div><span style={{ color: "#888", fontSize: 11 }}>가용</span><div style={{ color: "#00e68a" }}>{(bal - staked).toLocaleString()}</div></div>
          <div><span style={{ color: "#888", fontSize: 11 }}>USD 환산</span><div style={{ color: "#ccc" }}>${(bal * 0.124).toLocaleString()}</div></div>
        </div>
      </div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        {["보내기", "받기", "스테이킹", "스왑"].map((a) => (
          <button key={a} style={{ flex: 1, background: "#111", border: "1px solid #333", color: "#ccc", padding: "10px 0", borderRadius: 8, cursor: "pointer", fontSize: 12, fontFamily: "inherit" }}>
            {a}
          </button>
        ))}
      </div>
      <div style={{ background: "#111", borderRadius: 8, padding: 12, border: "1px solid #222" }}>
        <div style={{ fontSize: 11, color: "#888", marginBottom: 8 }}>최근 거래</div>
        {txs.map((tx, i) => (
          <div key={i} style={{ display: "flex", alignItems: "center", padding: "8px 0", borderBottom: "1px solid #1a1a1a", gap: 8 }}>
            <span style={{ color: tritColor(tx.trit), fontWeight: "bold", fontSize: 14, width: 20, textAlign: "center" }}>{tritLabel(tx.trit)}</span>
            <div style={{ flex: 1 }}>
              <div style={{ color: "#ccc" }}>{tx.type}: {tx.from} → {tx.to}</div>
              <div style={{ color: "#666", fontSize: 10 }}>{tx.time}</div>
            </div>
            <span style={{ color: tx.to === "alice" ? "#00e68a" : "#ff5555", fontWeight: "bold" }}>
              {tx.to === "alice" ? "+" : "-"}{tx.amount.toLocaleString()}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

// ═══ MAIN DESKTOP ═══
export default function CrownyOSDesktop() {
  const [windows, setWindows] = useState([]);
  const [focusId, setFocusId] = useState(0);
  const [clock, setClock] = useState("");
  const [showDock, setShowDock] = useState(true);
  const idCounter = useRef(1);

  useEffect(() => {
    const tick = () => {
      const now = new Date();
      setClock(now.toLocaleTimeString("ko-KR", { hour: "2-digit", minute: "2-digit" }));
    };
    tick();
    const i = setInterval(tick, 10000);
    return () => clearInterval(i);
  }, []);

  const openApp = useCallback((appKey) => {
    const app = APPS[appKey];
    const id = idCounter.current++;
    setWindows((w) => [...w, { id, appKey, ...app }]);
    setFocusId(id);
  }, []);

  const closeWindow = useCallback((id) => {
    setWindows((w) => w.filter((win) => win.id !== id));
  }, []);

  const renderApp = (appKey) => {
    switch (appKey) {
      case "terminal": return <TerminalApp />;
      case "files": return <FilesApp />;
      case "monitor": return <MonitorApp />;
      case "exchange": return <ExchangeApp />;
      case "consensus": return <ConsensusApp />;
      case "editor": return <EditorApp />;
      case "browser": return <BrowserApp />;
      case "wallet": return <WalletApp />;
      default: return <div style={{ padding: 20, color: "#888" }}>앱 로딩...</div>;
    }
  };

  return (
    <div style={{
      width: "100%", height: "100vh", background: "linear-gradient(145deg, #0a0a14 0%, #0d1a0d 30%, #0a1020 70%, #140a14 100%)",
      fontFamily: "'JetBrains Mono', 'Fira Code', 'SF Mono', monospace", position: "relative", overflow: "hidden", userSelect: "none",
    }}>
      {/* Background Pattern */}
      <div style={{ position: "absolute", inset: 0, opacity: 0.03, backgroundImage: "radial-gradient(circle, #00e68a 1px, transparent 1px)", backgroundSize: "30px 30px" }} />

      {/* Top Bar */}
      <div style={{
        height: 28, background: "rgba(10,10,10,0.85)", backdropFilter: "blur(20px)", display: "flex",
        alignItems: "center", padding: "0 12px", justifyContent: "space-between", borderBottom: "1px solid #222", position: "relative", zIndex: 99999,
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span style={{ fontSize: 14, fontWeight: "bold", color: "#00e68a" }}>◆ CrownyOS</span>
          <span style={{ fontSize: 10, color: "#555" }}>v0.10.0</span>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 14, fontSize: 11, color: "#888" }}>
          <span title="Chain">⛓ H:3</span>
          <span title="Processes">● 8</span>
          <span title="Network" style={{ color: "#00e68a" }}>● CTP</span>
          <span>{clock}</span>
        </div>
      </div>

      {/* Windows */}
      {windows.map((win) => (
        <WindowFrame key={win.id} id={win.id} title={win.name} icon={win.icon} color={win.color}
          zIndex={win.id === focusId ? 1000 : 100 + win.id}
          onClose={() => closeWindow(win.id)} onFocus={() => setFocusId(win.id)}>
          {renderApp(win.appKey)}
        </WindowFrame>
      ))}

      {/* Desktop Icons (when no windows) */}
      {windows.length === 0 && (
        <div style={{ position: "absolute", top: 60, left: 30, display: "flex", flexDirection: "column", gap: 4 }}>
          {Object.entries(APPS).map(([key, app]) => (
            <div key={key} onDoubleClick={() => openApp(key)} style={{
              width: 80, padding: "10px 4px", textAlign: "center", cursor: "pointer", borderRadius: 8,
            }}>
              <div style={{ fontSize: 32 }}>{app.icon}</div>
              <div style={{ fontSize: 10, color: "#aaa", marginTop: 4 }}>{app.name}</div>
            </div>
          ))}
        </div>
      )}

      {/* Dock */}
      <div style={{
        position: "absolute", bottom: 8, left: "50%", transform: "translateX(-50%)",
        display: "flex", gap: 4, padding: "6px 12px", background: "rgba(20,20,20,0.8)",
        backdropFilter: "blur(20px)", borderRadius: 16, border: "1px solid #333", zIndex: 99998,
      }}>
        {Object.entries(APPS).map(([key, app]) => {
          const isOpen = windows.some((w) => w.appKey === key);
          return (
            <div key={key} onClick={() => openApp(key)} title={app.name} style={{
              width: 44, height: 44, display: "flex", alignItems: "center", justifyContent: "center",
              borderRadius: 10, cursor: "pointer", fontSize: 24, position: "relative",
              background: isOpen ? "rgba(0,230,138,0.1)" : "transparent",
              transition: "transform 0.15s, background 0.15s",
            }}
              onMouseEnter={(e) => e.currentTarget.style.transform = "scale(1.25) translateY(-6px)"}
              onMouseLeave={(e) => e.currentTarget.style.transform = "scale(1)"}
            >
              {app.icon}
              {isOpen && <div style={{ position: "absolute", bottom: -2, width: 4, height: 4, borderRadius: "50%", background: "#00e68a" }} />}
            </div>
          );
        })}
      </div>
    </div>
  );
}
