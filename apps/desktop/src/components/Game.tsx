// Launching TF2 to test the HUD, the game console (console.log + remote console) and quick test commands.
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { ChevronDown, Play, RefreshCw, Send, Trash2 } from 'lucide-react';
import { api, errorText } from '../api.ts';
import { addGameLines, gameCommand, refreshGame, set, toast, updateSettings, useStore } from '../store.ts';
import { showMenu } from './External.tsx';
import { CLASS_CMDS, GAME_COMMAND_GROUPS, commandFor } from './gameCommands.ts';

const isWindows = typeof navigator !== 'undefined' && /windows/i.test(navigator.userAgent);
const isMac = typeof navigator !== 'undefined' && /mac os/i.test(navigator.userAgent) && !/windows/i.test(navigator.userAgent);

export async function launchGame(bits?: '64' | '32') {
  try {
    const info = await api.gameLaunch(bits);
    set({ gameLog: [], sidebar: 'game' });
    addGameLines('info', [
      `Starting TF2${info.method === 'steam' ? ' through Steam' : ''}${bits === '32' ? ' (32-bit, legacy)' : ''}…`,
      `${info.program} ${info.args.join(' ')}`,
      info.rconPort ? `Remote console on 127.0.0.1:${info.rconPort} (password made for this session)` : 'Remote console is off (Settings → Launching TF2).',
      info.logPath ? `Following ${info.logPath}` : 'Game log is off (Settings → Launching TF2).',
    ]);
    toast('Starting TF2…', 'ok');
    void refreshGame();
  } catch (e) {
    toast(errorText(e), 'error');
  }
}

/** Toolbar button: launch with the saved defaults, or pick 64/32-bit and options from the menu. */
export function PlayButton() {
  const app = useStore((s) => s.app);
  const game = useStore((s) => s.game);
  if (!app || isMac) return null;
  const l = app.settings.launch;
  const menu = (e: React.MouseEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const fake = { preventDefault() {}, clientX: r.left, clientY: r.bottom + 4 } as unknown as React.MouseEvent;
    showMenu(fake, [
      { label: 'Launch TF2 (64-bit)', run: () => void launchGame('64') },
      ...(isWindows ? [{ label: 'Launch TF2 (32-bit, legacy)', run: () => void launchGame('32') }] : []),
      { label: `${l.itemtest ? '✓' : '  '} Start in itemtest`, run: () => void updateSettings({ launch: { ...l, itemtest: !l.itemtest } }) },
      { label: `${l.rcon ? '✓' : '  '} Remote console`, run: () => void updateSettings({ launch: { ...l, rcon: !l.rcon } }) },
      { label: `${l.reloadOnSave ? '✓' : '  '} Reload HUD after saving`, run: () => void updateSettings({ launch: { ...l, reloadOnSave: !l.reloadOnSave } }) },
      { label: 'Game console', run: () => set({ sidebar: 'game' }) },
      { label: 'Launch settings…', run: () => set({ dialog: { kind: 'settings' } }) },
    ]);
  };
  return (
    <div className="split-btn">
      {game?.rcon && (
        <button className="tb" onClick={() => void gameCommand('hud_reloadscheme')} title="Reload the HUD in TF2 (hud_reloadscheme)"><RefreshCw size={15} /><span>Reload HUD</span></button>
      )}
      <button className="tb" onClick={() => void launchGame()} title={`Launch TF2 (${l.bits === '32' ? '32-bit' : '64-bit'}${l.itemtest ? ', itemtest' : ''})`}><Play size={15} /><span>Play</span></button>
      <button className="tb icon caret" onPointerDown={(e) => e.stopPropagation()} onClick={menu} title="Launch options" aria-label="Launch options"><ChevronDown size={14} /></button>
    </div>
  );
}

/** Sidebar: the game's console output, a command line, and test commands. */
export function GamePanel() {
  const log = useStore((s) => s.gameLog);
  const game = useStore((s) => s.game);
  const app = useStore((s) => s.app);
  const [cmd, setCmd] = useState('');
  const [hist, setHist] = useState<string[]>([]);
  const [hi, setHi] = useState(-1);
  const [filter, setFilter] = useState('');
  const [open, setOpen] = useState<Record<string, boolean>>({ hud: true });
  const box = useRef<HTMLDivElement>(null);
  const stick = useRef(true);
  useEffect(() => {
    void refreshGame();
    const t = setInterval(() => void refreshGame(), 4000);
    return () => clearInterval(t);
  }, []);
  const shown = useMemo(() => {
    const f = filter.trim().toLowerCase();
    const l = f ? log.filter((x) => x.text.toLowerCase().includes(f)) : log;
    return l.slice(-800);
  }, [log, filter]);
  useEffect(() => {
    const el = box.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [shown]);
  const send = async (c: string) => {
    const t = c.trim();
    if (!t) return;
    setHist((h) => [t, ...h.filter((x) => x !== t)].slice(0, 50));
    setHi(-1);
    setCmd('');
    await gameCommand(t);
  };
  const rconOn = !!game?.rcon;
  const started = !!game && (game.running !== null || game.rcon || !!game.logPath);
  const status = !started ? 'Not started' : game!.running === false ? 'TF2 closed' : rconOn ? 'Connected' : game!.logPath ? 'Following the log' : 'Running';
  return (
    <>
      <div className="panel-head"><h2>Game</h2><span className={'pill' + (rconOn ? ' ok' : '')}>{status}</span></div>
      <div className="game-actions">
        <button className="btn primary" onClick={() => void launchGame()}><Play size={13} /> Launch TF2</button>
        <button className="btn" disabled={!rconOn} onClick={() => void gameCommand('hud_reloadscheme')}><RefreshCw size={13} /> Reload HUD</button>
        <button className="btn icon-only" title="Clear the log" onClick={() => set({ gameLog: [] })}><Trash2 size={13} /></button>
      </div>
      <input className="prop-filter" placeholder="Filter the log" value={filter} onChange={(e) => setFilter(e.target.value)} />
      <div className="game-log" ref={box} onScroll={(e) => { const el = e.currentTarget; stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24; }} role="log" aria-live="polite">
        {shown.length === 0 && <p className="hint pad">Launch TF2 from here to see its console. {app?.settings.launch.consoleLog === false ? 'The game log is off in Settings.' : ''}</p>}
        {shown.map((l) => <div key={l.id} className={'gl gl-' + l.kind}>{l.text}</div>)}
      </div>
      <form className="game-input" onSubmit={(e) => { e.preventDefault(); void send(cmd); }}>
        <input value={cmd} disabled={!rconOn} placeholder={rconOn ? 'Console command (↑ for history)' : 'Launch TF2 with the remote console on to send commands'} spellCheck={false}
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowUp' && hist.length) { e.preventDefault(); const n = Math.min(hist.length - 1, hi + 1); setHi(n); setCmd(hist[n]); }
            if (e.key === 'ArrowDown') { e.preventDefault(); const n = hi - 1; setHi(n); setCmd(n >= 0 ? hist[n] : ''); }
          }} />
        <button className="btn icon-only" disabled={!rconOn || !cmd.trim()} title="Send"><Send size={13} /></button>
      </form>
      <div className="game-tests">
        <section className="gt-group">
          <button className="gt-head" onClick={() => setOpen((o) => ({ ...o, class: !o.class }))}><ChevronDown size={13} className={open.class ? '' : 'closed'} /> Class</button>
          {open.class && <div className="gt-items">{CLASS_CMDS.map((c) => <button key={c.id} className="chip" disabled={!rconOn} onClick={() => void gameCommand(`joinclass ${c.id}`)}>{c.label}</button>)}</div>}
        </section>
        {GAME_COMMAND_GROUPS.map((g) => (
          <section key={g.id} className="gt-group">
            <button className="gt-head" onClick={() => setOpen((o) => ({ ...o, [g.id]: !o[g.id] }))}><ChevronDown size={13} className={open[g.id] ? '' : 'closed'} /> {g.title}</button>
            {open[g.id] && (
              <>
                {g.note && <p className="hint">{g.note}</p>}
                <div className="gt-items">
                  {g.items.map((c) => <button key={c.label} className={'chip' + (c.cheat ? ' cheat' : '')} disabled={!rconOn} title={`${c.hint ? c.hint + '\n' : ''}${commandFor(c)}`} onClick={() => void gameCommand(commandFor(c))}>{c.label}</button>)}
                </div>
              </>
            )}
          </section>
        ))}
        <p className="hint pad">Buttons marked with a dot turn on <code>sv_cheats</code>, which only works in a local game such as itemtest.</p>
      </div>
    </>
  );
}
