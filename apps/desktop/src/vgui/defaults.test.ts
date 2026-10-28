import { test } from 'node:test';
import assert from 'node:assert/strict';
import { defaultFor } from './defaults.ts';
import { controlKind } from './catalog.ts';

const scheme = (entries: Record<string, string>) => (n: string) => entries[n];
const props = (entries: Record<string, string>) => (k: string) => entries[k];

test('image buttons are buttons (text, colors and borders draw)', () => {
  assert.equal(controlKind('cteximagebutton'), 'button');
  assert.equal(controlKind('cexbutton'), 'button');
  assert.equal(controlKind('ctfimagepanel'), 'image');
});

test('button colors fall back to the scheme, then to the engine defaults', () => {
  const base = scheme({ 'Button.BgColor': 'TanDarker', 'Button.TextColor': 'TanLight' });
  assert.deepEqual(defaultFor('cexbutton', 'defaultBgColor_override', props({}), base), { value: 'TanDarker', from: 'scheme Button.BgColor' });
  assert.equal(defaultFor('cexbutton', 'defaultbgcolor_override', props({}), scheme({}))?.value, '0 0 0 255');
  assert.equal(defaultFor('cexbutton', 'defaultfgcolor_override', props({}), base)?.value, 'TanLight');
});

test('armed colors fall back to the button’s own default colors', () => {
  const own = props({ defaultbgcolor_override: '46 43 42 255' });
  assert.deepEqual(defaultFor('cexbutton', 'armedbgcolor_override', own, scheme({})), { value: '46 43 42 255', from: 'defaultbgcolor_override' });
  // a scheme ArmedBgColor wins over the default state, like vgui::Button
  assert.equal(defaultFor('cexbutton', 'armedbgcolor_override', own, scheme({ 'Button.ArmedBgColor': 'Orange' }))?.value, 'Orange');
});

test('a button’s plain fgcolor_override is its default text color', () => {
  assert.equal(defaultFor('cexbutton', 'defaultfgcolor_override', props({ fgcolor_override: '1 2 3 255' }), scheme({}))?.value, '1 2 3 255');
});

test('labels and images', () => {
  assert.equal(defaultFor('cexlabel', 'fgcolor', props({}), scheme({ 'Label.TextColor': 'TanLight' }))?.value, 'TanLight');
  assert.equal(defaultFor('imagepanel', 'drawcolor', props({}), scheme({}))?.value, '255 255 255 255');
  assert.equal(defaultFor('editablepanel', 'bgcolor', props({}), scheme({ 'Panel.BgColor': 'Black' })), undefined, 'panels never gain a painted background');
  assert.equal(defaultFor('cexlabel', 'xpos', props({}), scheme({})), undefined);
});
