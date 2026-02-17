// Crowny 한선어 VSCode Extension
// 실행, 컴파일, 디버그, Trit 시각화

const vscode = require('vscode');
const { exec } = require('child_process');
const path = require('path');

function activate(context) {
    console.log('Crowny 한선어 Extension 활성화');

    const config = vscode.workspace.getConfiguration('crowny');
    const tvmPath = config.get('tvmPath', 'crowni-tvm');

    // ── 실행 ──
    const runCmd = vscode.commands.registerCommand('crowny.run', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const terminal = getTerminal();
        terminal.show();
        terminal.sendText(`${tvmPath} hanseon "${filePath}"`);
    });

    // ── WASM 컴파일 ──
    const compileCmd = vscode.commands.registerCommand('crowny.compile', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const wasmPath = filePath.replace(/\.(cws|hsn)$/, '.wasm');
        const terminal = getTerminal();
        terminal.show();
        terminal.sendText(`${tvmPath} compile "${filePath}" "${wasmPath}"`);
    });

    // ── 바이트코드 ──
    const bytecodeCmd = vscode.commands.registerCommand('crowny.bytecode', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const bcPath = filePath.replace(/\.(cws|hsn)$/, '.크라운');
        const terminal = getTerminal();
        terminal.show();
        terminal.sendText(`${tvmPath} bytecode "${filePath}" "${bcPath}"`);
    });

    // ── 디버그 ──
    const debugCmd = vscode.commands.registerCommand('crowny.debug', () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;

        const filePath = editor.document.fileName;
        const terminal = getTerminal();
        terminal.show();
        terminal.sendText(`${tvmPath} debug "${filePath}"`);
    });

    // ── Trit 상태 보기 ──
    const tritViewCmd = vscode.commands.registerCommand('crowny.tritView', () => {
        const panel = vscode.window.createWebviewPanel(
            'tritView', 'Trit 상태', vscode.ViewColumn.Beside,
            { enableScripts: true }
        );
        panel.webview.html = getTritViewHtml();
    });

    // ── 729 Opcode 탐색기 ──
    const sectorsCmd = vscode.commands.registerCommand('crowny.sectors', () => {
        const terminal = getTerminal();
        terminal.show();
        terminal.sendText(`${tvmPath} sectors`);
    });

    // ── 상태 바 ──
    const statusBar = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Left, 100
    );
    statusBar.text = '$(triangle-right) Crowny';
    statusBar.command = 'crowny.run';
    statusBar.tooltip = 'Crowny 한선어 실행 (Cmd+Shift+R)';
    statusBar.show();

    // ── 저장 시 자동 실행 ──
    const onSave = vscode.workspace.onDidSaveTextDocument((doc) => {
        if (config.get('autoRun', false) && isCrownyFile(doc)) {
            vscode.commands.executeCommand('crowny.run');
        }
    });

    // ── Trit 진단 (간단 린터) ──
    const diagnostics = vscode.languages.createDiagnosticCollection('crowny');
    const onEdit = vscode.workspace.onDidChangeTextDocument((e) => {
        if (isCrownyFile(e.document)) {
            updateDiagnostics(e.document, diagnostics);
        }
    });

    context.subscriptions.push(
        runCmd, compileCmd, bytecodeCmd, debugCmd,
        tritViewCmd, sectorsCmd, statusBar, onSave, onEdit, diagnostics
    );
}

function deactivate() {
    console.log('Crowny Extension 비활성화');
}

// ── 헬퍼 ──

let _terminal = null;
function getTerminal() {
    if (!_terminal || _terminal.exitStatus !== undefined) {
        _terminal = vscode.window.createTerminal('Crowny');
    }
    return _terminal;
}

function isCrownyFile(doc) {
    return doc.languageId === 'crowny' ||
           doc.fileName.endsWith('.cws') ||
           doc.fileName.endsWith('.hsn');
}

function updateDiagnostics(document, collection) {
    const diags = [];
    const text = document.getText();
    const lines = text.split('\n');

    let hasEnd = false;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();

        // 끝 확인
        if (line === '끝' || line === 'end') hasEnd = true;

        // 중괄호 균형 체크 (간단)
        const opens = (line.match(/{/g) || []).length;
        const closes = (line.match(/}/g) || []).length;

        // 빈 함수 경고
        if (/^(함수|func|fn)\s+\S+\s*\{\s*\}/.test(line)) {
            diags.push(new vscode.Diagnostic(
                new vscode.Range(i, 0, i, line.length),
                '빈 함수 정의',
                vscode.DiagnosticSeverity.Warning
            ));
        }
    }

    if (!hasEnd && lines.length > 3) {
        const lastLine = lines.length - 1;
        diags.push(new vscode.Diagnostic(
            new vscode.Range(lastLine, 0, lastLine, 1),
            '프로그램 끝에 "끝" 또는 "end" 필요',
            vscode.DiagnosticSeverity.Information
        ));
    }

    collection.set(document.uri, diags);
}

function getTritViewHtml() {
    return `<!DOCTYPE html>
<html>
<head>
<style>
  body { font-family: 'Segoe UI', sans-serif; padding: 20px; background: #1e1e1e; color: #ddd; }
  h2 { color: #4ec9b0; border-bottom: 1px solid #333; padding-bottom: 8px; }
  .trit { display: inline-block; width: 40px; height: 40px; border-radius: 50%;
          text-align: center; line-height: 40px; font-weight: bold; margin: 4px; font-size: 18px; }
  .trit-p { background: #4caf50; color: white; }
  .trit-o { background: #ff9800; color: white; }
  .trit-t { background: #f44336; color: white; }
  .header { display: flex; gap: 4px; margin: 12px 0; }
  table { border-collapse: collapse; width: 100%; margin-top: 16px; }
  td, th { padding: 8px 12px; border: 1px solid #444; }
  th { background: #2d2d2d; color: #4ec9b0; }
  .stats { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; margin-top: 16px; }
  .stat-box { background: #2d2d2d; padding: 16px; border-radius: 8px; text-align: center; }
  .stat-num { font-size: 32px; font-weight: bold; }
</style>
</head>
<body>
  <h2>Crowny Trit 상태 모니터</h2>
  <p>CTP 헤더 예시:</p>
  <div class="header">
    <span class="trit trit-p">P</span>
    <span class="trit trit-p">P</span>
    <span class="trit trit-p">P</span>
    <span class="trit trit-o">O</span>
    <span class="trit trit-o">O</span>
    <span class="trit trit-o">O</span>
    <span class="trit trit-o">O</span>
    <span class="trit trit-o">O</span>
    <span class="trit trit-o">O</span>
  </div>
  <div class="stats">
    <div class="stat-box"><div class="stat-num" style="color:#4caf50">P</div><div>성공</div></div>
    <div class="stat-box"><div class="stat-num" style="color:#ff9800">O</div><div>보류</div></div>
    <div class="stat-box"><div class="stat-num" style="color:#f44336">T</div><div>실패</div></div>
  </div>
  <h2>729 Opcode 섹터</h2>
  <table>
    <tr><th>ID</th><th>섹터</th><th>활성</th></tr>
    <tr><td>0</td><td>코어 (Kernel)</td><td>80/81</td></tr>
    <tr><td>1</td><td>지능 (Intelligence)</td><td>45/81</td></tr>
    <tr><td>2</td><td>하드웨어 (Hardware)</td><td>18/81</td></tr>
    <tr><td>3</td><td>기억 (Memory)</td><td>18/81</td></tr>
    <tr><td>4</td><td>표현 (Expression)</td><td>18/81</td></tr>
    <tr><td>5</td><td>초월 (Transcendence)</td><td>9/81</td></tr>
    <tr><td>6</td><td>보안 (Security)</td><td>13/81</td></tr>
    <tr><td>7</td><td>메타 (Meta)</td><td>9/81</td></tr>
    <tr><td>8</td><td>확장 (User)</td><td>9/81</td></tr>
  </table>
</body>
</html>`;
}

module.exports = { activate, deactivate };
