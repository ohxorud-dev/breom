import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let runStatusBarItem: vscode.StatusBarItem | undefined;

async function hasMainFunction(uri: vscode.Uri | undefined): Promise<boolean> {
    if (!uri || uri.scheme !== 'file' || path.extname(uri.fsPath) !== '.brm') {
        return false;
    }
    try {
        const bytes = await vscode.workspace.fs.readFile(uri);
        const content = Buffer.from(bytes).toString('utf8');
        return /\bfn\s+main\s*\(/.test(content);
    } catch {
        return false;
    }
}

function findNearestProjectDir(startPath: string): string | undefined {
    let current = path.dirname(startPath);
    while (true) {
        if (fs.existsSync(path.join(current, 'project.breom'))) {
            return current;
        }
        const parent = path.dirname(current);
        if (parent === current) {
            return undefined;
        }
        current = parent;
    }
}

function shellQuote(value: string): string {
    return `"${value.replace(/(["\\$`])/g, '\\$1')}"`;
}

function formatArg(value: string): string {
    return /^[A-Za-z0-9_./:-]+$/.test(value) ? value : shellQuote(value);
}

async function runBreomTask(
    cwd: string,
    args: string[]
): Promise<void> {
    const config = vscode.workspace.getConfiguration('breom');
    const configuredCliPath = config.get<string>('cliPath', '').trim();
    const breomHome = config.get<string>('home', '').trim() || (process.env.BREOM_HOME ?? '').trim();
    const cliPath = configuredCliPath || (breomHome ? path.join(breomHome, 'bin', 'breom') : '');
    if (!cliPath) {
        vscode.window.showErrorMessage('BREOM_HOME is not set. Configure breom.home or set BREOM_HOME.');
        return;
    }
    const env = breomHome ? { ...process.env, BREOM_HOME: breomHome } : undefined;

    const terminal = vscode.window.createTerminal({
        name: 'Breom Run',
        cwd,
        env,
    });
    terminal.show(true);
    const command = `${formatArg(cliPath)} ${args.map(formatArg).join(' ')}`.trim();
    terminal.sendText(command, true);
}

export function activate(context: vscode.ExtensionContext) {
    outputChannel = vscode.window.createOutputChannel('Breom Extension');
    outputChannel.appendLine('Breom extension activating...');
    outputChannel.show(true);

    const config = vscode.workspace.getConfiguration('breom');
    const serverPath = config.get<string>('serverPath', 'breom');

    outputChannel.appendLine(`Server path: ${serverPath}`);

    const serverOptions: ServerOptions = {
        command: serverPath,
        args: ['lsp'],
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'breom' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{brm,breom}'),
        },
        outputChannelName: 'Breom Language Server',
    };

    client = new LanguageClient(
        'breom',
        'Breom Language Server',
        serverOptions,
        clientOptions
    );

    client.start().then(() => {
        outputChannel.appendLine('LSP client started successfully');
    }).catch((err) => {
        outputChannel.appendLine(`LSP client failed to start: ${err}`);
    });

    const runFileCommand = vscode.commands.registerCommand('breom.runFile', async (uri?: vscode.Uri) => {
        const target = uri ?? vscode.window.activeTextEditor?.document.uri;
        if (!target || target.scheme !== 'file' || path.extname(target.fsPath) !== '.brm') {
            vscode.window.showErrorMessage('Select a .brm file to run.');
            return;
        }
        if (!(await hasMainFunction(target))) {
            vscode.window.showErrorMessage('Current file has no main function. Use Breom: Run Project.');
            return;
        }

        const projectDir = findNearestProjectDir(target.fsPath);
        const cwd = projectDir ?? path.dirname(target.fsPath);
        await runBreomTask(cwd, ['run', target.fsPath]);
    });

    const runProjectCommand = vscode.commands.registerCommand('breom.runProject', async (uri?: vscode.Uri) => {
        const activeUri = uri ?? vscode.window.activeTextEditor?.document.uri;
        const fromActive = activeUri?.scheme === 'file' ? findNearestProjectDir(activeUri.fsPath) : undefined;
        const workspaceDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        const cwd = fromActive ?? workspaceDir;

        if (!cwd) {
            vscode.window.showErrorMessage('Open a Breom project folder to run.');
            return;
        }

        await runBreomTask(cwd, ['run']);
    });

    runStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 110);
    runStatusBarItem.command = 'breom.runFile';
    runStatusBarItem.text = '$(play) Breom Run';
    runStatusBarItem.tooltip = 'Run current Breom file';

    const updateStatusBarVisibility = async () => {
        const editor = vscode.window.activeTextEditor;
        const hasMain = editor?.document.languageId === 'breom' && await hasMainFunction(editor.document.uri);
        await vscode.commands.executeCommand('setContext', 'breom.hasMainFunction', Boolean(hasMain));
        if (hasMain) {
            runStatusBarItem?.show();
        } else {
            runStatusBarItem?.hide();
        }
    };

    void updateStatusBarVisibility();
    const statusWatcher = vscode.window.onDidChangeActiveTextEditor(() => { void updateStatusBarVisibility(); });
    const textWatcher = vscode.workspace.onDidChangeTextDocument((event) => {
        const active = vscode.window.activeTextEditor;
        if (active && event.document.uri.toString() === active.document.uri.toString()) {
            void updateStatusBarVisibility();
        }
    });

    context.subscriptions.push(runFileCommand, runProjectCommand, runStatusBarItem, statusWatcher, textWatcher);

    context.subscriptions.push({
        dispose: () => {
            if (client) {
                client.stop();
            }
        },
    });

    outputChannel.appendLine('Breom extension activated!');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
