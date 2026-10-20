import React, { useEffect, useRef } from 'react';
import { Annotation, EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, highlightSpecialChars } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { StreamLanguage, HighlightStyle, syntaxHighlighting, bracketMatching, foldGutter, foldKeymap, indentOnInput } from '@codemirror/language';
import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
import { autocompletion, closeBrackets, type CompletionContext } from '@codemirror/autocomplete';
import { linter, lintGutter, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
import { tags as t } from '@lezer/highlight';
import { getState, setSource, useStore } from '../store.ts';
import { PROPS } from '../vgui/catalog.ts';

type KvState = { strings: number; anim: boolean };
const External = Annotation.define<boolean>();
/** Last cursor line per file, so "Open in editor" can jump to where you were. */
export const cursorLines = new Map<string, number>();

const ANIM_WORDS = /^(event|animate|runevent|stopevent|stopanimation|stoppanelanimations|setfont|settexture|setstring|setvisible|setinputenabled|runeventchild|firecommand|playsound)$/i;
const INTERP = /^(linear|accel|deaccel|spline|pulse|flicker|gain|bias|bounce)$/i;

function kvLanguage(anim: boolean) {
  return StreamLanguage.define<KvState>({
    startState: () => ({ strings: 0, anim }),
    token(stream, state) {
      if (stream.sol()) state.strings = 0;
      if (stream.eatSpace()) return null;
      if (stream.match('//')) {
        stream.skipToEnd();
        return 'comment';
      }
      if (stream.match('/*')) {
        while (!stream.eol() && !stream.match('*/')) stream.next();
        return 'comment';
      }
      const ch = stream.peek();
      if (ch === '{' || ch === '}') {
        stream.next();
        state.strings = 0;
        return 'brace';
      }
      if (ch === '[') {
        stream.skipTo(']');
        stream.next();
        return 'meta';
      }
      if (ch === '"') {
        stream.next();
        let escaped = false;
        while (!stream.eol()) {
          const c = stream.next();
          if (c === '"' && !escaped) break;
          escaped = c === '\\' && !escaped;
        }
        state.strings++;
        if (state.anim) return state.strings > 1 ? 'string' : 'variableName';
        return state.strings % 2 === 1 ? 'propertyName' : 'string';
      }
      stream.eatWhile(/[^\s{}"\[]/);
      const word = stream.current();
      state.strings++;
      if (word.startsWith('#')) return 'keyword';
      if (state.anim) {
        if (ANIM_WORDS.test(word)) return 'keyword';
        if (INTERP.test(word)) return 'typeName';
        if (/^-?\d/.test(word)) return 'number';
        return 'variableName';
      }
      if (/^-?[\d.]+$/.test(word)) return 'number';
      return state.strings % 2 === 1 ? 'propertyName' : 'string';
    },
  });
}

const highlight = HighlightStyle.define([
  { tag: t.comment, color: 'var(--code-comment)', fontStyle: 'italic' },
  { tag: t.propertyName, color: 'var(--code-key)' },
  { tag: t.string, color: 'var(--code-string)' },
  { tag: t.number, color: 'var(--code-number)' },
  { tag: t.keyword, color: 'var(--code-keyword)', fontWeight: '600' },
  { tag: t.meta, color: 'var(--code-meta)' },
  { tag: t.brace, color: 'var(--code-brace)' },
  { tag: t.typeName, color: 'var(--code-type)' },
  { tag: t.variableName, color: 'var(--code-var)' },
]);

const theme = EditorView.theme({
  '&': { height: '100%', fontSize: '13px', backgroundColor: 'var(--code-bg)', color: 'var(--text)' },
  '.cm-content': { fontFamily: 'var(--mono)', caretColor: 'var(--accent)' },
  '.cm-gutters': { backgroundColor: 'var(--code-bg)', color: 'var(--faint)', border: 'none' },
  '.cm-activeLine': { backgroundColor: 'rgba(240,164,58,.06)' },
  '.cm-activeLineGutter': { backgroundColor: 'rgba(240,164,58,.1)', color: 'var(--text)' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': { backgroundColor: 'rgba(240,164,58,.25) !important' },
  '.cm-cursor': { borderLeftColor: 'var(--accent)' },
  '.cm-tooltip': { backgroundColor: 'var(--raised)', border: '1px solid var(--line)', color: 'var(--text)' },
  '.cm-tooltip-autocomplete ul li[aria-selected]': { backgroundColor: 'var(--accent)', color: '#241b16' },
  '.cm-panels': { backgroundColor: 'var(--panel)', color: 'var(--text)' },
  '.cm-searchMatch': { backgroundColor: 'rgba(240,164,58,.3)' },
}, { dark: true });

function complete(ctx: CompletionContext) {
  const word = ctx.matchBefore(/"?[\w$#.]*$/);
  if (!word || (word.from === word.to && !ctx.explicit)) return null;
  const line = ctx.state.doc.lineAt(ctx.pos);
  const before = line.text.slice(0, ctx.pos - line.from);
  const quotesBefore = (before.match(/"/g) ?? []).length;
  const isValue = quotesBefore >= 2;
  const s = getState().schemeIndex.scheme;
  const from = word.text.startsWith('"') ? word.from + 1 : word.from;
  if (!isValue) return { from, options: Object.entries(PROPS).map(([k, v]) => ({ label: k, type: 'property', info: v.doc })) };
  const key = /"([^"]+)"[^"]*$/.exec(before.slice(0, before.lastIndexOf('"')))?.[1]?.toLowerCase() ?? '';
  if (key.endsWith('font')) return { from, options: (s?.fonts ?? []).map((f) => ({ label: f.name, type: 'constant' })) };
  if (key.includes('color')) return { from, options: (s?.colors ?? []).map((c) => ({ label: c.name, type: 'constant', detail: c.raw })) };
  if (key === 'border') return { from, options: (s?.borders ?? []).map((b) => ({ label: b.key, type: 'constant' })) };
  return null;
}

export function CodeEditor({ path }: { path: string }) {
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const doc = useStore((s) => s.docs[path]);
  const reveal = useStore((s) => s.reveal);

  useEffect(() => {
    if (!host.current) return;
    const anim = /hudanimations/i.test(path) && !/manifest/i.test(path);
    const extensions: Extension[] = [
      lineNumbers(), highlightActiveLineGutter(), highlightSpecialChars(), history(), foldGutter(), drawSelection(), indentOnInput(), bracketMatching(), closeBrackets(),
      highlightActiveLine(), highlightSelectionMatches(), autocompletion({ override: [complete] }), lintGutter(),
      linter((v) => {
        const d = getState().docs[path];
        if (!d) return [];
        return d.diagnostics.map((x): CmDiagnostic => ({ from: Math.min(x.span.start, v.state.doc.length), to: Math.min(Math.max(x.span.end, x.span.start + 1), v.state.doc.length), severity: 'error', message: x.message }));
      }, { delay: 400 }),
      keymap.of([...defaultKeymap, ...historyKeymap, ...foldKeymap, ...searchKeymap, indentWithTab]),
      kvLanguage(anim), syntaxHighlighting(highlight), theme,
      EditorState.tabSize.of(4),
      EditorView.updateListener.of((u) => {
        if (u.selectionSet || u.docChanged) cursorLines.set(path, u.state.doc.lineAt(u.state.selection.main.head).number);
        if (!u.docChanged || u.transactions.every((tr) => tr.annotation(External))) return;
        if (timer.current) clearTimeout(timer.current);
        const text = u.state.doc.toString();
        timer.current = setTimeout(() => {
          timer.current = null;
          void setSource(path, text);
        }, 350);
      }),
    ];
    const v = new EditorView({ parent: host.current, state: EditorState.create({ doc: getState().docs[path]?.source ?? '', extensions }) });
    view.current = v;
    return () => {
      if (timer.current) {
        clearTimeout(timer.current);
        void setSource(path, v.state.doc.toString());
      }
      v.destroy();
      view.current = null;
    };
  }, [path]);

  // backend changed the text (undo, designer edits, reload): replace the editor content
  useEffect(() => {
    const v = view.current;
    if (!v || !doc) return;
    const current = v.state.doc.toString();
    if (current === doc.source || timer.current) return;
    const sel = Math.min(v.state.selection.main.head, doc.source.length);
    v.dispatch({ changes: { from: 0, to: current.length, insert: doc.source }, selection: { anchor: sel }, annotations: External.of(true) });
  }, [doc?.source]);

  useEffect(() => {
    const v = view.current;
    if (!v || !reveal || reveal.file !== path) return;
    const len = v.state.doc.length;
    let pos = 0;
    if (reveal.offset !== undefined) pos = Math.min(reveal.offset, len);
    else if (reveal.line) pos = v.state.doc.line(Math.min(Math.max(1, reveal.line), v.state.doc.lines)).from;
    const line = v.state.doc.lineAt(pos);
    v.dispatch({ selection: { anchor: line.from, head: line.to }, effects: EditorView.scrollIntoView(pos, { y: 'center' }) });
    v.focus();
  }, [reveal?.nonce]);

  return <div className="code-host" ref={host} />;
}

