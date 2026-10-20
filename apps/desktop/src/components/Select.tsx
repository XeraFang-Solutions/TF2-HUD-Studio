// Themed replacements for native <select> popups and <datalist> suggestions, which the OS draws in its own
// colours (white GTK menus on Linux, light lists on Windows) instead of the app's palette.
import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown } from 'lucide-react';

type Item = { value: string; label: string; disabled?: boolean; group?: string; detail?: string };

function textOf(node: React.ReactNode): string {
  if (node === null || node === undefined || typeof node === 'boolean') return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(textOf).join('');
  if (React.isValidElement(node)) return textOf((node.props as { children?: React.ReactNode }).children);
  return '';
}

/** Reads <option>/<optgroup> children (including fragments and arrays) into a flat list. */
function collect(children: React.ReactNode, group?: string, out: Item[] = []): Item[] {
  React.Children.forEach(children, (child) => {
    if (!React.isValidElement(child)) return;
    const props = child.props as { value?: string | number; label?: string; disabled?: boolean; children?: React.ReactNode };
    if (child.type === React.Fragment) collect(props.children, group, out);
    else if (child.type === 'optgroup') collect(props.children, props.label, out);
    else if (child.type === 'option') {
      const label = textOf(props.children);
      out.push({ value: props.value !== undefined ? String(props.value) : label, label, disabled: props.disabled, group });
    }
  });
  return out;
}

/** A floating list anchored under (or above) an element. */
function Popup({ anchor, items, active, selected, onPick, onHover, minWidth }: {
  anchor: HTMLElement; items: Item[]; active: number; selected?: string; onPick: (it: Item) => void; onHover: (i: number) => void; minWidth?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<React.CSSProperties>({ visibility: 'hidden' });
  useLayoutEffect(() => {
    const r = anchor.getBoundingClientRect();
    const h = Math.min(ref.current?.scrollHeight ?? 300, 320);
    const below = window.innerHeight - r.bottom - 8;
    const up = below < h && r.top > below;
    const width = Math.max(r.width, minWidth ? 160 : 0);
    setPos({
      left: Math.max(4, Math.min(r.left, window.innerWidth - width - 4)),
      top: up ? Math.max(4, r.top - h - 4) : r.bottom + 4,
      minWidth: width,
      maxWidth: Math.max(width, 420),
      maxHeight: up ? Math.min(320, r.top - 8) : Math.min(320, below),
    });
  }, [anchor, items.length, minWidth]);
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>(`[data-i="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [active]);
  let lastGroup: string | undefined;
  return createPortal(
    <div ref={ref} className="dropdown" style={pos} role="listbox" onPointerDown={(e) => e.preventDefault()} onMouseDown={(e) => e.preventDefault()}>
      {items.map((it, i) => {
        const head = it.group !== lastGroup && it.group ? <div className="dropdown-group">{it.group}</div> : null;
        lastGroup = it.group;
        return (
          <React.Fragment key={i}>
            {head}
            <div
              data-i={i}
              role="option"
              aria-selected={it.value === selected}
              aria-disabled={it.disabled}
              className={'dropdown-item' + (i === active ? ' active' : '') + (it.value === selected ? ' chosen' : '') + (it.disabled ? ' disabled' : '') + (it.group ? ' grouped' : '')}
              onPointerEnter={() => onHover(i)}
              onClick={() => !it.disabled && onPick(it)}
            >
              <span className="dropdown-label">{it.label || ' '}</span>
              {it.detail && <span className="dropdown-detail">{it.detail}</span>}
              {it.value === selected && <Check size={13} />}
            </div>
          </React.Fragment>
        );
      })}
    </div>,
    document.body,
  );
}

function useOutside(open: boolean, close: () => void, inside: React.RefObject<HTMLElement | null>) {
  useEffect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      const t = e.target as Node;
      if (inside.current?.contains(t) || (t instanceof Element && t.closest('.dropdown'))) return;
      close();
    };
    const scroll = (e: Event) => {
      if (e.target instanceof Element && e.target.closest('.dropdown')) return;
      close();
    };
    window.addEventListener('pointerdown', down, true);
    window.addEventListener('scroll', scroll, true);
    window.addEventListener('resize', close);
    window.addEventListener('blur', close);
    return () => {
      window.removeEventListener('pointerdown', down, true);
      window.removeEventListener('scroll', scroll, true);
      window.removeEventListener('resize', close);
      window.removeEventListener('blur', close);
    };
  }, [open, close, inside]);
}

const step = (items: Item[], from: number, dir: number) => {
  for (let i = from + dir; i >= 0 && i < items.length; i += dir) if (!items[i].disabled) return i;
  return from;
};

/** Drop-in for <select>: same children, and onChange receives `{ target: { value } }`. */
export function Select({ value, onChange, className, title, children, disabled }: {
  value: string | number; onChange: (e: { target: { value: string } }) => void; className?: string; title?: string; children: React.ReactNode; disabled?: boolean;
}) {
  const items = useMemo(() => collect(children), [children]);
  const current = String(value);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const btn = useRef<HTMLButtonElement>(null);
  const typed = useRef({ text: '', at: 0 });
  const close = useCallback(() => setOpen(false), []);
  useOutside(open, close, btn);
  const show = () => {
    setActive(Math.max(0, items.findIndex((it) => it.value === current)));
    setOpen(true);
  };
  const pick = (it: Item) => {
    setOpen(false);
    btn.current?.focus();
    if (it.value !== current) onChange({ target: { value: it.value } });
  };
  const onKey = (e: React.KeyboardEvent) => {
    const k = e.key;
    if (!open) {
      if (k === 'ArrowDown' || k === 'ArrowUp' || k === 'Enter' || k === ' ') { e.preventDefault(); show(); }
      return;
    }
    if (k === 'Escape' || k === 'Tab') { if (k === 'Escape') e.preventDefault(); e.stopPropagation(); setOpen(false); }
    else if (k === 'ArrowDown') { e.preventDefault(); setActive((a) => step(items, a, 1)); }
    else if (k === 'ArrowUp') { e.preventDefault(); setActive((a) => step(items, a, -1)); }
    else if (k === 'Home') { e.preventDefault(); setActive(step(items, -1, 1)); }
    else if (k === 'End') { e.preventDefault(); setActive(step(items, items.length, -1)); }
    else if (k === 'Enter' || k === ' ') { e.preventDefault(); if (items[active] && !items[active].disabled) pick(items[active]); }
    else if (k.length === 1) {
      const now = Date.now();
      typed.current = { text: (now - typed.current.at < 700 ? typed.current.text : '') + k.toLowerCase(), at: now };
      const i = items.findIndex((it) => !it.disabled && it.label.toLowerCase().startsWith(typed.current.text));
      if (i >= 0) setActive(i);
    }
  };
  const label = items.find((it) => it.value === current)?.label ?? current;
  return (
    <>
      <button
        ref={btn}
        type="button"
        className={'select' + (className ? ' ' + className : '') + (open ? ' open' : '')}
        title={open ? undefined : title}
        disabled={disabled}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : show())}
        onKeyDown={onKey}
      >
        <span className="select-label">{label}</span>
        <ChevronDown size={13} className="select-chevron" />
      </button>
      {open && btn.current && <Popup anchor={btn.current} items={items} active={active} selected={current} onPick={pick} onHover={setActive} minWidth />}
    </>
  );
}

/** Options of a <datalist> on the page, read when needed so the lists can stay where they are. */
function datalistItems(id: string): Item[] {
  const el = document.getElementById(id);
  if (!(el instanceof HTMLDataListElement)) return [];
  return Array.from(el.options).map((o) => ({ value: o.value, label: o.value, detail: o.textContent && o.textContent !== o.value ? o.textContent : undefined }));
}

/** An <input> with a themed suggestion list taken from a <datalist> id (replaces the native `list` popup). */
export function SuggestInput({ list, value, onChange, onKeyDown, onBlur, onPick, ...rest }: Omit<React.InputHTMLAttributes<HTMLInputElement>, 'list' | 'value' | 'onChange'> & {
  list?: string; value: string; onChange: (v: string) => void; onPick?: (v: string) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const [all, setAll] = useState<Item[]>([]);
  const close = useCallback(() => setOpen(false), []);
  useOutside(open, close, input);
  const items = useMemo(() => {
    const q = value.trim().toLowerCase();
    if (!q) return all.slice(0, 200);
    const starts = all.filter((it) => it.value.toLowerCase().startsWith(q));
    const has = all.filter((it) => !it.value.toLowerCase().startsWith(q) && it.value.toLowerCase().includes(q));
    return [...starts, ...has].slice(0, 200);
  }, [all, value]);
  const exact = items.length === 1 && items[0].value === value;
  const show = () => {
    if (!list) return;
    setAll(datalistItems(list));
    setActive(-1);
    setOpen(true);
  };
  const pick = (it: Item) => {
    setOpen(false);
    onChange(it.value);
    onPick?.(it.value);
  };
  return (
    <>
      <input
        ref={input}
        {...rest}
        value={value}
        autoComplete="off"
        onFocus={(e) => { show(); rest.onFocus?.(e); }}
        onClick={() => !open && show()}
        onChange={(e) => { onChange(e.target.value); if (!open) show(); setActive(-1); }}
        onBlur={(e) => { setOpen(false); onBlur?.(e); }}
        onKeyDown={(e) => {
          if (open && items.length && !exact) {
            if (e.key === 'ArrowDown') { e.preventDefault(); setActive((a) => Math.min(items.length - 1, a + 1)); return; }
            if (e.key === 'ArrowUp') { e.preventDefault(); setActive((a) => Math.max(-1, a - 1)); return; }
            if (e.key === 'Enter' && active >= 0) { e.preventDefault(); pick(items[active]); return; }
            if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); setOpen(false); return; }
          }
          onKeyDown?.(e);
        }}
      />
      {open && list && items.length > 0 && !exact && input.current && (
        <Popup anchor={input.current} items={items} active={active} selected={value} onPick={pick} onHover={setActive} />
      )}
    </>
  );
}
