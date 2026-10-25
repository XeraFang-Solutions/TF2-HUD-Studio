import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clientToUnits, computePos, encodePos, computeSize, encodeSize, pinnedPos, layoutTree, encodeMove, type LayoutInput, type LayoutBox } from './layout.ts';

test('pointer placement follows the rendered canvas at fractional and nonuniform scale', () => {
  const screen = { x: 0, y: 0, w: 480 * 16 / 9, h: 480 };
  for (const [width, height] of [[1066.5, 600.25], [640, 360], [1920, 1080], [800, 600]]) {
    const rect = { left: 137.5, top: 49.25, width, height };
    assert.deepEqual(clientToUnits({ clientX: rect.left, clientY: rect.top }, rect, screen), { x: 0, y: 0 });
    const center = clientToUnits({ clientX: rect.left + width / 2, clientY: rect.top + height / 2 }, rect, screen);
    assert.ok(Math.abs(center.x - screen.w / 2) < 1e-10);
    assert.equal(center.y, 240);
    const moved = clientToUnits({ clientX: rect.left + width / 2 + width * 10 / screen.w, clientY: rect.top + height / 2 }, rect, screen);
    assert.ok(Math.abs(moved.x - center.x - 10) < 1e-10);
  }
});

test('positions follow Panel::ComputePos', () => {
  assert.equal(computePos('10', 50, 640), 10);
  assert.equal(computePos('r50', 20, 640), 590);
  assert.equal(computePos('c-25', 50, 640), 295);
  assert.equal(computePos('cs-0.5', 50, 640), 295);
  assert.equal(computePos('rs1', 50, 640), 590);
  assert.equal(computePos('p0.25', 50, 640), 160);
  assert.equal(computePos('c-12.9', 0, 640), 308, 'atoi truncates');
});

test('encodePos round-trips every notation', () => {
  for (const raw of ['10', 'r50', 'c-25', 'cs-0.5', 'rs1', 'p0.25', 's2']) {
    const x = computePos(raw, 50, 640) + 7;
    const enc = encodePos(raw, x, 50, 640);
    assert.equal(enc[0] === raw[0] || /\d|-/.test(raw[0]), true, `${raw} -> ${enc}`);
    assert.ok(Math.abs(computePos(enc, 50, 640) - x) < 0.01, `${raw} -> ${enc}`);
  }
});

test('sizes', () => {
  assert.equal(computeSize('f0', 853, 0, 0), 853);
  assert.equal(computeSize('f20', 853, 0, 0), 833);
  assert.equal(computeSize('p0.5', 480, 0, 0), 240);
  assert.equal(computeSize('o2', 480, 30, 0), 60);
  assert.equal(encodeSize('f0', 800, 853, 0), 'f53');
  assert.equal(encodeSize('120', 99.6, 853, 0), '100');
});

test('pinning offsets point outward from the sibling corner (cases from a real HUD)', () => {
  const button = { x: 100, y: 100, w: 50, h: 20 };
  // BlueCount: my TOPLEFT at button TOPRIGHT, xpos 10 -> 10 units right of the button
  assert.deepEqual(pinnedPos(button, 1, 0, 30, 20, 10, 0), [160, 100]);
  // CompetitiveEntry: my TOPRIGHT at sibling TOPLEFT, xpos 5 -> 5 units left
  assert.deepEqual(pinnedPos(button, 0, 1, 30, 20, 5, 0), [65, 100]);
  // ScoutCustom: my BOTTOMLEFT at sibling TOPLEFT, ypos 3 -> 3 units above
  assert.deepEqual(pinnedPos(button, 0, 2, 50, 10, 0, 3), [100, 87]);
  // EngineerCustom: my TOPLEFT at sibling BOTTOMLEFT, ypos 3 -> 3 units below
  assert.deepEqual(pinnedPos(button, 2, 0, 50, 10, 0, 3), [100, 123]);
  // AvatarImage pinned into AvatarBGPanel with -2 -2 -> inset by 2
  assert.deepEqual(pinnedPos(button, 0, 0, 10, 10, -2, -2), [102, 102]);
});

function node(uid: string, props: Record<string, string>, children: LayoutInput[] = []): LayoutInput {
  return { uid, names: [uid.toLowerCase(), (props.fieldName ?? uid).toLowerCase()], get: (k) => props[k], children };
}

test('layoutTree handles pins, nesting and move encoding', () => {
  const screen = { x: 0, y: 0, w: 853.33, h: 480 };
  const out = new Map<string, LayoutBox>();
  layoutTree([
    node('Shadow', { xpos: '-1', ypos: '-1', wide: '40', tall: '20', pin_to_sibling: 'Value' }),
    node('Value', { xpos: 'c-20', ypos: 'r60', wide: '40', tall: '20' }, [node('Inner', { xpos: '5', ypos: '5', wide: 'f10', tall: '5' })]),
  ], screen, out);
  const v = out.get('Value')!;
  assert.ok(Math.abs(v.x - 406.665) < 0.01);
  assert.equal(v.y, 420);
  const s = out.get('Shadow')!;
  assert.equal(s.x, v.x + 1, 'drop shadow sits down-right');
  assert.equal(s.y, 421);
  assert.equal(out.get('Inner')!.w, 30);
  assert.equal(out.get('Inner')!.x, v.x + 5);
  const moved = encodeMove(v, 'c-20', 'r60', v.x + 10, v.y - 5);
  assert.deepEqual(moved, { xpos: 'c-10', ypos: 'r65' });
  assert.deepEqual(encodeMove(s, '-1', '-1', s.x + 3, s.y), { xpos: '-4', ypos: '-1' });
});
