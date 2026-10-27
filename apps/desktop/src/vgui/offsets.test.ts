import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EditorState } from '@codemirror/state';
import { editorPos } from './offsets.ts';

const utf8 = (s: string) => new TextEncoder().encode(s).length;

test('editorPos matches plain LF text exactly', () => {
  const s = '"a"\n{\n\t"x" "1"\n}\n';
  assert.equal(editorPos(s, s.indexOf('"x"')), s.indexOf('"x"'));
});

test('editorPos lands on the same line and column in CRLF files', () => {
  const lines = Array.from({ length: 200 }, (_, i) => `\t"key${i}"\t\t"value"`);
  const s = lines.join('\r\n');
  const target = s.indexOf('"key150"');
  const doc = EditorState.create({ doc: s }).doc;
  const pos = editorPos(s, utf8(s.slice(0, target)));
  assert.equal(doc.lineAt(pos).number, 151);
  assert.equal(doc.sliceString(pos, pos + 8), '"key150"');
});

test('editorPos handles BOMs, accents and emoji', () => {
  const s = '\ufeff// café 🎯\r\n"HudX"\r\n';
  const doc = EditorState.create({ doc: s }).doc;
  const pos = editorPos(s, utf8(s.slice(0, s.indexOf('"HudX"'))));
  assert.equal(doc.sliceString(pos, pos + 6), '"HudX"');
});
