export type Sheet = { id: string; name: string; cells: Record<string, string> };
export type Workspace = { format: 'folio'; version: 1; id: string; title: string; revision: number; document: any; sheets: Sheet[] };
export type Proposal = { baseRevision: number; summary: string; appendParagraphs?: string[]; replaceDocument?: string[]; cells?: { sheetId: string; address: string; value: string }[] };
export const COLS = 12, ROWS = 60;
export const address = (r: number, c: number) => `${String.fromCharCode(65 + c)}${r + 1}`;
export function parseAddress(a: string): [number, number] {
  const m = /^([A-L])([1-9]\d?)$/.exec(a.toUpperCase());
  if (!m || +m[2] > ROWS) throw Error(`Cell outside A1:L${ROWS}: ${a}`);
  return [+m[2] - 1, m[1].charCodeAt(0) - 65];
}
export function seed(): Workspace {
  return { format: 'folio', version: 1, id: crypto.randomUUID(), title: 'A little room to think', revision: 0,
    document: { type: 'doc', content: [
      { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'Good ideas start here.' }] },
      { type: 'paragraph', content: [{ type: 'text', text: 'A document, a spreadsheet, and an assistant. Together in one quiet workspace.' }] },
      { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Make something meaningful' }] },
      { type: 'paragraph', content: [{ type: 'text', text: 'Use this space for your next proposal, a project plan, or the first draft of something new. Your workbook is right next door.' }] },
      { type: 'bulletList', content: ['Shape your thoughts with headings and lists.', 'Explore the example budget in your workbook.', 'Ask the assistant to turn your numbers into a narrative.'].map(text => ({ type: 'listItem', content: [{ type: 'paragraph', content: [{ type: 'text', text }] }] })) },
      { type: 'paragraph', content: [{ type: 'text', marks: [{ type: 'italic' }], text: 'This is a sample workspace. Everything here is yours to change.' }] }
    ] }, sheets: [{ id: 'budget', name: 'Project budget', cells: { A1: 'PROJECT BUDGET', A3: 'Category', B3: 'Planned', C3: 'Actual', D3: 'Remaining', A4: 'Research', B4: '2400', C4: '1850', D4: '=B4-C4', A5: 'Design', B5: '4800', C5: '4200', D5: '=B5-C5', A6: 'Development', B6: '9600', C6: '7200', D6: '=B6-C6', A7: 'Launch', B7: '3200', C7: '1500', D7: '=B7-C7', A9: 'Total', B9: '=SUM(B4:B7)', C9: '=SUM(C4:C7)', D9: '=B9-C9' } }] };
}
export function validateWorkspace(value: unknown): Workspace {
  const w = value as Workspace;
  if (!w || w.format !== 'folio' || w.version !== 1 || typeof w.title !== 'string' || typeof w.id !== 'string' || !Number.isSafeInteger(w.revision) || w.revision < 0 || w.document?.type !== 'doc' || !Array.isArray(w.sheets) || w.sheets.length < 1 || w.sheets.length > 30) throw Error('This is not a supported Folio workspace.');
  const ids = new Set();
  for (const s of w.sheets) {
    if (!s || typeof s.id !== 'string' || ids.has(s.id) || typeof s.name !== 'string' || !s.cells || Array.isArray(s.cells) || typeof s.cells !== 'object') throw Error('Invalid sheet.');
    ids.add(s.id);
    for (const [key, val] of Object.entries(s.cells)) { parseAddress(key); if (typeof val !== 'string' || val.length > 10000) throw Error('Invalid cell.'); }
  }
  return structuredClone(w);
}
export function applyProposal(w: Workspace, p: Proposal): Workspace {
  if (!p || p.baseRevision !== w.revision) throw Error('The workspace has changed. Ask the assistant for an updated proposal.');
  if (typeof p.summary !== 'string') throw Error('A change summary is required.');
  const n = structuredClone(w);
  for (const key of ['appendParagraphs', 'replaceDocument'] as const) {
    if (p[key] !== undefined && (!Array.isArray(p[key]) || p[key]!.length > 200 || p[key]!.some(t => typeof t !== 'string' || t.length > 30000))) throw Error('Invalid document changes.');
  }
  const paragraphs = (texts: string[]) => texts.map(text => ({ type: 'paragraph', ...(text ? { content: [{ type: 'text', text }] } : {}) }));
  if (p.replaceDocument) n.document = { type: 'doc', content: paragraphs(p.replaceDocument.length ? p.replaceDocument : ['']) };
  if (p.appendParagraphs) n.document.content = [...(n.document.content || []), ...paragraphs(p.appendParagraphs)];
  if (p.cells !== undefined && (!Array.isArray(p.cells) || p.cells.length > 720)) throw Error('Too many cell changes.');
  for (const cell of p.cells || []) {
    parseAddress(cell.address);
    const s = n.sheets.find(s => s.id === cell.sheetId);
    if (!s || typeof cell.value !== 'string' || cell.value.length > 10000) throw Error('Invalid spreadsheet change.');
    s.cells[cell.address.toUpperCase()] = cell.value;
  }
  n.revision++;
  return n;
}
export function csv(s: Sheet): string {
  const used = Object.keys(s.cells).filter(a => s.cells[a] !== '').map(parseAddress);
  if (!used.length) return '';
  const maxRow = Math.max(...used.map(a => a[0])), maxCol = Math.max(...used.map(a => a[1]));
  return Array.from({ length: maxRow + 1 }, (_, r) => Array.from({ length: maxCol + 1 }, (_, c) => {
    const v = String(evaluate(s, address(r, c))); return /[",\n\r]/.test(v) ? `"${v.replaceAll('"', '""')}"` : v;
  }).join(',')).join('\r\n');
}
// A bounded expression parser: no eval, JavaScript execution, external references, or macros.
export function evaluate(sheet: Sheet, addr: string, visiting = new Set<string>(), cache = new Map<string, string | number>()): string | number {
  if (cache.has(addr)) return cache.get(addr)!;
  const value = sheet.cells[addr] || '';
  if (!value.startsWith('=')) return value !== '' && Number.isFinite(Number(value)) ? Number(value) : value;
  if (visiting.has(addr)) return '#CYCLE!';
  if (visiting.size > 100) return '#DEPTH!';
  const seen = new Set(visiting).add(addr);
  try {
    const source = value.slice(1).toUpperCase().replace(/\s+/g, '');
    const tokens = source.match(/(?:\d+(?:\.\d*)?|\.\d+)(?:E[+-]?\d+)?|[A-Z]+\d*|[+\-*/^(),:]/g) || [];
    if (tokens.join('') !== source || tokens.length > 500) throw Error('#VALUE!');
    let i = 0;
    const num = (cell: string) => { parseAddress(cell); const v = evaluate(sheet, cell, seen, cache); if (typeof v === 'string' && v.startsWith('#')) throw Error(v); return typeof v === 'number' ? v : 0; };
    const atom = (): number => {
      const t = tokens[i++];
      if (t === '(') { const n = expr(); if (tokens[i++] !== ')') throw Error('#ERROR!'); return n; }
      if (/^(?:\d|\.)/.test(t || '')) return Number(t);
      if (/^[A-Z]+\d+$/.test(t || '')) return num(t);
      if (['SUM','AVERAGE','MIN','MAX','COUNT'].includes(t) && tokens[i++] === '(') {
        const vals: number[] = [];
        while (tokens[i] !== ')') {
          if (i >= tokens.length) throw Error('#ERROR!');
          if (/^[A-Z]+\d+$/.test(tokens[i]) && tokens[i + 1] === ':') {
            const [r1,c1] = parseAddress(tokens[i]); const [r2,c2] = parseAddress(tokens[i + 2]); i += 3;
            for (let r = Math.min(r1,r2); r <= Math.max(r1,r2); r++) for (let c = Math.min(c1,c2); c <= Math.max(c1,c2); c++) {
              const v = evaluate(sheet, address(r,c), seen, cache); if (typeof v === 'number') vals.push(v); else if (v.startsWith('#')) throw Error(v);
            }
          } else vals.push(expr());
          if (tokens[i] === ',') i++; else if (tokens[i] !== ')') throw Error('#ERROR!');
        }
        i++;
        if (t === 'COUNT') return vals.length;
        if (t === 'AVERAGE') { if (!vals.length) throw Error('#DIV/0!'); return vals.reduce((a,b)=>a+b,0)/vals.length; }
        if (t === 'MIN') return vals.length ? Math.min(...vals) : 0;
        if (t === 'MAX') return vals.length ? Math.max(...vals) : 0;
        return vals.reduce((a,b)=>a+b,0);
      }
      throw Error('#NAME?');
    };
    const power = (): number => { const n = atom(); return tokens[i] === '^' ? (i++, n ** unary()) : n; };
    const unary = (): number => tokens[i] === '-' ? (i++, -unary()) : tokens[i] === '+' ? (i++, unary()) : power();
    const term = (): number => { let n = unary(); while (tokens[i] === '*' || tokens[i] === '/') { const op = tokens[i++], v = unary(); if (op === '/' && v === 0) throw Error('#DIV/0!'); n = op === '*' ? n * v : n / v; } return n; };
    const expr = (): number => { let n = term(); while (tokens[i] === '+' || tokens[i] === '-') { const op = tokens[i++], v = term(); n = op === '+' ? n + v : n - v; } return n; };
    const result = expr(); if (i !== tokens.length || !Number.isFinite(result)) throw Error('#ERROR!'); cache.set(addr, result); return result;
  } catch (e) { const m = (e as Error).message; const error = m.startsWith('#') ? m : '#REF!'; cache.set(addr, error); return error; }
}
