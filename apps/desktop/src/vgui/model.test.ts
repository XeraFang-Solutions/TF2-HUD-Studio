import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildComposite } from './model.ts';
import type { RNode, Resolved } from '../api.ts';

let id = 0;
const kv = (key: string, value: string, file: string): RNode => ({ key, value, file, editable: true, id: String(id++) });
const block = (key: string, file: string, props: [string, string][], children: RNode[] = []): RNode =>
  ({ key, file, editable: true, id: String(id++), children: [...props.map(([k, v]) => kv(k, v, file)), ...children] });
const res = (path: string, root: RNode): Resolved => ({ path, origin: { kind: 'project', path }, roots: [root], chain: [path], missing: [] });

test('an element file\'s self-entry overrides its hudlayout entry (bxhud chat)', () => {
  const L = 'scripts/hudlayout.res', C = 'resource/ui/basechat.res';
  const hudlayout = res(L, block(L, L, [], [block('HudChat', L, [['fieldName', 'HudChat'], ['xpos', '10'], ['ypos', '275'], ['wide', '320'], ['zpos', '1']])]));
  const chat = res(C, block(C, C, [], [
    block('HudChat', C, [['fieldName', 'HudChat'], ['xpos', '2'], ['ypos', '2'], ['wide', '260']]),
    block('HudChatHistory', C, [['xpos', '5'], ['ypos', '5']]),
  ]));
  const [el] = buildComposite(hudlayout, { HudChat: [C] }, { [C]: chat }, new Set(['hudchat']));
  assert.equal(el.props.get('xpos')?.value, '2');
  assert.equal(el.props.get('ypos')?.value, '2');
  assert.equal(el.props.get('wide')?.value, '260');
  assert.equal(el.props.get('xpos')?.file, C, 'edits go to the file that wins');
  assert.equal(el.props.get('zpos')?.value, '1', 'keys only in hudlayout still apply');
  assert.equal(el.children.map((c) => c.name).join(), 'HudChatHistory');
});
