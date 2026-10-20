import { test } from 'node:test';
import assert from 'node:assert/strict';
import { expand, evaluate, interpolate } from './anim.ts';
import { parseRGBA } from './scheme.ts';
import type { AnimEvent } from '../api.ts';

const ev = (name: string, commands: any[]): AnimEvent => ({ name, file: 'f', editable: true, line: 1, start: 0, end: 0, commands: commands.map((c) => ({ duration: 0, line: 1, ...c })) });

test('expands RunEvent chains and loops within the horizon', () => {
  const events = [
    ev('Pulse', [{ kind: 'Animate', panel: 'P', property: 'Alpha', value: '0', interpolator: 'Linear', start: 0, duration: 0.1 }, { kind: 'RunEvent', event: 'Loop', start: 0.3 }]),
    ev('Loop', [{ kind: 'RunEvent', event: 'Pulse', start: 0 }]),
  ];
  const cmds = expand(events, 'Pulse', 1);
  assert.ok(cmds.filter((c) => c.kind === 'Animate').length >= 3);
});

test('evaluate animates from the value at start', () => {
  const events = [ev('E', [
    { kind: 'Animate', panel: 'P', property: 'Alpha', value: '0', interpolator: 'Linear', start: 0, duration: 1 },
    { kind: 'Animate', panel: 'P', property: 'BgColor', value: 'Red', interpolator: 'Linear', start: 0, duration: 1 },
    { kind: 'SetVisible', panel: 'Q', value: '0', start: 0.5 },
  ])];
  const cmds = expand(events, 'E');
  const color = (r: string) => (r === 'Red' ? [255, 0, 0, 255] as any : parseRGBA(r));
  const v = evaluate(cmds, 0.5, (_p, prop) => (prop === 'alpha' ? '255' : '0 0 0 255'), color);
  assert.equal(v.get('p')!.get('alpha'), '127.5');
  assert.equal(v.get('p')!.get('bgcolor'), '128 0 0 255');
  assert.equal(v.get('q')!.get('visible'), '0');
});

test('interpolators', () => {
  assert.equal(interpolate('Linear', 0.5), 0.5);
  assert.equal(interpolate('Accel', 0.5), 0.25);
  assert.equal(interpolate('Spline', 0.5), 0.5);
  assert.ok(Math.abs(interpolate('Pulse', 0.25, 2) - 1) < 1e-9);
});
