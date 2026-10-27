// The backend reports positions as UTF-8 byte offsets into a file's text. CodeMirror counts UTF-16 code units and
// stores every line break (\r\n included) as a single character, so raw offsets drift in CRLF files (TF2's own files
// are CRLF) and after any non-ASCII text.

/** Maps a UTF-8 byte offset into `source` to the matching CodeMirror document position. */
export function editorPos(source: string, byteOffset: number): number {
  let bytes = 0;
  let pos = 0;
  for (let i = 0; i < source.length && bytes < byteOffset; i++) {
    const c = source.charCodeAt(i);
    if (c === 13 && source.charCodeAt(i + 1) === 10) {
      bytes += 1; // the \r of \r\n: CodeMirror keeps only one break character
      continue;
    }
    if (c >= 0xd800 && c <= 0xdbff && i + 1 < source.length) {
      bytes += 4; // surrogate pair: one 4-byte character, two code units
      pos += 2;
      i++;
      continue;
    }
    bytes += c < 0x80 ? 1 : c < 0x800 ? 2 : 3;
    pos++;
  }
  return pos;
}

/** The line break a file uses, so editing it in the Source view keeps it. */
export function lineBreakOf(source: string): '\r\n' | '\n' {
  return source.includes('\r\n') ? '\r\n' : '\n';
}
