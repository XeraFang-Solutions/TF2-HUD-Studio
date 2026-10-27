import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PRESETS, exportTheme, fontStack, importTheme, parseTheme, themePrefs, themeVars } from './theme.ts';

test('presets are complete, valid themes', () => {
  for (const p of PRESETS) assert.deepEqual(parseTheme(p), p, p.id);
  assert.equal(PRESETS[0].id, 'tf2');
});

test('theme files round-trip and get a fresh id', () => {
  const t = { ...PRESETS[1], id: 'custom-abc', name: 'Mine', author: 'Me' };
  const back = importTheme(exportTheme(t));
  assert.notEqual(back.id, t.id);
  assert.ok(back.id.startsWith('custom-'));
  assert.deepEqual({ ...back, id: t.id }, t);
});

test('untrusted theme values cannot inject CSS', () => {
  const t = parseTheme({
    colors: { bg: 'red; background: url(https://evil)', text: '#ABC', accent: 'url(x)' },
    fonts: { display: "x'; } body { background: url(evil)", ui: 'tf2secondary' },
    shape: { radius: 999 },
    name: '<b>hi</b>',
  })!;
  assert.equal(t.colors.bg, PRESETS[0].colors.bg);
  assert.equal(t.colors.text, '#aabbcc');
  assert.equal(t.colors.accent, PRESETS[0].colors.accent);
  assert.equal(t.shape.radius, 16);
  assert.ok(!/[;{}()'<>]/.test(t.fonts.display), t.fonts.display);
  assert.ok(!t.name.includes('<'));
  for (const v of Object.values(themeVars(t))) assert.ok(!/url\(|[{};]/.test(v), v);
});

test('import rejects files that are not themes', () => {
  assert.throws(() => importTheme('nope'), /JSON/);
  assert.throws(() => importTheme('{"colors":{}}'), /theme/);
});

test('installed font names fall back to a safe stack', () => {
  assert.match(fontStack('Verdana', 'ui'), /^'Verdana', /);
  assert.match(fontStack('tf2build', 'display'), /^'hs-tf2build'/);
});

test('settings without themes default to TF2', () => {
  const p = themePrefs({});
  assert.equal(p.themeId, 'tf2');
  assert.deepEqual(p.customThemes, []);
  assert.deepEqual(themePrefs({ customThemes: [{ id: 'tf2', colors: {} }, 'junk'] }).customThemes, []);
});
