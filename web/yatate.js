// 矢立の web 殻 — 核（wasm）を呼び、原器で打ち、手修正を覚える。
//
// ここに**頭脳は無い**。配列も旧字確定も辞書も文節分割も核の仕事で、
// この頁がやるのは「鍵を取る・描く・覚えたことを仕舞ふ」だけである
// （docs/ime/cross-platform.md §3「殻に頭脳を置かない」）。
//
// 覚えたことの置き場は localStorage ひとつ。送信も同期もしない。

'use strict';

const STORE = 'yatate/manabi/v1';

// ── 核への繋ぎ ──────────────────────────────────────────────

class Core {
  static async load(url = 'yatate.wasm') {
    const bytes = await (await fetch(url)).arrayBuffer();
    // **入口を一つも渡さない。** 渡すものが無いのがこの殻の性質である
    // （wasm 側に import section が無いことを CI が検めてゐる）。
    const { instance } = await WebAssembly.instantiate(bytes, {});
    return new Core(instance);
  }

  constructor(instance) {
    this.x = instance.exports;
    this.h = this.x.yatate_new();
    this.enc = new TextEncoder();
    this.dec = new TextDecoder();
  }

  // メモリは伸びることがあるので、その都度取り直す（掴んだまま持たない）
  get mem() { return new Uint8Array(this.x.memory.buffer); }

  _send(str, fn) {
    const bytes = this.enc.encode(str);
    const ptr = this.x.yatate_alloc(bytes.length);
    this.mem.set(bytes, ptr);
    try { return fn(ptr, bytes.length); }
    finally { this.x.yatate_dealloc(ptr, bytes.length); }
  }

  _recv() {
    const ptr = this.x.yatate_out_ptr(this.h);
    const len = this.x.yatate_out_len(this.h);
    if (!len) return '';
    return this.dec.decode(this.mem.subarray(ptr, ptr + len));
  }

  // 0 = 素通し / 1 = 未確定が変はつた / 2 = 呑んだ
  press(code) { return this._send(code, (p, n) => this.x.yatate_press(this.h, p, n)); }
  backspace() { return this.x.yatate_backspace(this.h) === 1; }
  cancel() { this.x.yatate_cancel(this.h); }
  isShifted() { return this.x.yatate_is_shifted(this.h) === 1; }

  preedit() { this.x.yatate_preedit(this.h); return this._recv(); }
  commit() { this.x.yatate_commit(this.h); return this._recv(); }
  kyuji(text) { this._send(text, (p, n) => this.x.yatate_kyuji(this.h, p, n)); return this._recv(); }

  /** 読み → [[表記, 度数], …]。当たらなければ空 */
  suggest(yomi) {
    this._send(yomi, (p, n) => this.x.yatate_suggest(this.h, p, n));
    return lines(this._recv()).map(l => l.split('\t'));
  }

  /** 読み → [{yomi, chosen, candidates:[表記,…]}, …] */
  convert(yomi) {
    this._send(yomi, (p, n) => this.x.yatate_convert(this.h, p, n));
    return lines(this._recv()).map(l => {
      const cols = l.split('\t');
      return { yomi: cols[0], chosen: Number(cols[1]), candidates: cols.slice(2) };
    });
  }

  /** code → {first, second} の表。頁は配列表を持たない */
  layout() {
    this.x.yatate_layout(this.h);
    const out = new Map();
    for (const l of lines(this._recv())) {
      const [code, first, second] = l.split('\t');
      out.set(code, { first, second });
    }
    return out;
  }

  /** code → 墨（0〜1） */
  kehai() {
    this.x.yatate_kehai(this.h);
    const out = new Map();
    for (const l of lines(this._recv())) {
      const [code, ink] = l.split('\t');
      out.set(code, Number(ink));
    }
    return out;
  }
}

const lines = (s) => s ? s.split('\n').filter(Boolean) : [];

// ── 覚えたこと ──────────────────────────────────────────────
//
// 形は { 読み: { 表記: 度数 } }。**消さない**——文語では「今日／けふ」の
// 書き分けが普通に起きるので、同じ読みに複数の表記が付いてよい。

const manabi = {
  data: {},

  load() {
    try { this.data = JSON.parse(localStorage.getItem(STORE)) || {}; }
    catch { this.data = {}; }          // 壊れてゐたら黙つて捨てる（頁ごと死なせない）
    return this.data;
  },
  save() {
    try { localStorage.setItem(STORE, JSON.stringify(this.data)); }
    catch (e) { console.warn('矢立: 覚えを仕舞へなかつた', e); }
  },
  learn(yomi, surface) {
    if (!yomi || !surface) return false;
    const bucket = this.data[yomi] || (this.data[yomi] = {});
    bucket[surface] = (bucket[surface] || 0) + 1;
    this.save();
    return true;
  },
  forgetOne(yomi, surface) {
    if (!this.data[yomi]) return;
    delete this.data[yomi][surface];
    if (!Object.keys(this.data[yomi]).length) delete this.data[yomi];
    this.save();
  },
  /** 読み → [[表記, 度数], …]（度数の多い順・同数は表記順） */
  of(yomi) {
    return Object.entries(this.data[yomi] || {})
      .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1));
  },
  count() {
    return Object.values(this.data).reduce((n, b) => n + Object.keys(b).length, 0);
  },
  merge(incoming) {
    let added = 0;
    for (const [yomi, bucket] of Object.entries(incoming || {})) {
      if (typeof bucket !== 'object' || !bucket) continue;
      const mine = this.data[yomi] || (this.data[yomi] = {});
      for (const [surface, n] of Object.entries(bucket)) {
        if (typeof n !== 'number' || n < 0) continue;
        mine[surface] = (mine[surface] || 0) + n;
        added++;
      }
    }
    this.save();
    return added;
  },
};

// ── 頁 ──────────────────────────────────────────────────────

const $ = (sel) => document.querySelector(sel);

const paper = $('#paper');
const bar = $('#bar');
const barYomi = $('#bar-yomi');
const candList = $('#cands');
const board = $('#board');

// 物理鍵盤の段の並び。**これは QWERTY 型の機の形であつて、原器の中身ではない**
// （どの鍵が何の仮名かは核が返す — Core#layout）。原器に無い鍵は空欄で描く。
const ROWS = [
  ['Digit1', 'Digit2', 'Digit3', 'Digit4', 'Digit5', 'Digit6', 'Digit7', 'Digit8', 'Digit9', 'Digit0', 'Minus', 'Equal'],
  ['KeyQ', 'KeyW', 'KeyE', 'KeyR', 'KeyT', 'KeyY', 'KeyU', 'KeyI', 'KeyO', 'KeyP'],
  ['KeyA', 'KeyS', 'KeyD', 'KeyF', 'KeyG', 'KeyH', 'KeyJ', 'KeyK', 'KeyL', 'Semicolon', 'Quote'],
  ['KeyZ', 'KeyX', 'KeyC', 'KeyV', 'KeyB', 'KeyN', 'KeyM'],
];

let core = null;
let layout = new Map();
let cands = [];
let sel = 0;
let blockId = 0;
let harvestTimer = null;

// ── 未確定の描画 ────────────────────────────────────────────

function preeditNode() {
  return paper.querySelector('.preedit');
}

/** 未確定の場所を作る（無ければキャレットの位置へ挿す） */
function ensurePreedit() {
  let node = preeditNode();
  if (node) return node;
  node = document.createElement('span');
  node.className = 'preedit';
  const range = caretRange();
  range.insertNode(node);
  return node;
}

function caretRange() {
  const s = window.getSelection();
  if (s && s.rangeCount && paper.contains(s.anchorNode)) {
    const r = s.getRangeAt(0);
    r.deleteContents();
    return r;
  }
  const r = document.createRange();
  r.selectNodeContents(paper);
  r.collapse(false);
  return r;
}

function caretAfter(node) {
  const r = document.createRange();
  r.setStartAfter(node);
  r.collapse(true);
  const s = window.getSelection();
  s.removeAllRanges();
  s.addRange(r);
}

function render() {
  const yomi = core.preedit();
  const node = yomi ? ensurePreedit() : preeditNode();
  if (!yomi) {
    if (node) { caretAfter(node); node.remove(); }
    hideBar();
    paintBoard();
    return;
  }
  node.textContent = yomi;
  node.classList.toggle('shifted', core.isShifted());
  caretAfter(node);
  showCandidates(yomi);
  paintBoard();
}

// ── 候補 ────────────────────────────────────────────────────

function candidatesFor(yomi) {
  const out = [];
  const seen = new Set();
  const add = (surface, source) => {
    if (!surface || seen.has(surface)) return;
    seen.add(surface);
    out.push({ surface, source });
  };

  // ① 学んだ対（使つた回数の多い順）— どこから来た提案かを示す
  for (const [surface, n] of manabi.of(yomi)) add(surface, `あなたが ${n} 度書いた`);
  // ② 辞書
  for (const [surface] of core.suggest(yomi)) add(surface, '辞書');
  // ③ 文節に分けての変換（二字以上のとき）
  if ([...yomi].length >= 2) {
    const segs = core.convert(yomi);
    if (segs.length) add(segs.map(s => s.candidates[s.chosen]).join(''), '変換');
  }
  // ④ 仮名のまま
  add(yomi, '仮名のまま');
  return out;
}

function showCandidates(yomi) {
  cands = candidatesFor(yomi);
  sel = 0;
  // 覚えが無ければ何も出さない（空の候補窓を出さない）
  if (cands.length <= 1) { hideBar(); return; }
  barYomi.textContent = yomi;
  candList.replaceChildren(...cands.map((c, i) => {
    const li = document.createElement('li');
    li.className = i === sel ? 'on' : '';
    li.innerHTML = '';
    const s = document.createElement('span');
    s.className = 'surface';
    s.textContent = c.surface;
    const src = document.createElement('span');
    src.className = 'source';
    src.textContent = c.source;
    li.append(s, src);
    li.addEventListener('mousedown', (ev) => { ev.preventDefault(); sel = i; take(); });
    return li;
  }));
  bar.hidden = false;
}

function hideBar() {
  bar.hidden = true;
  candList.replaceChildren();
  cands = [];
  sel = 0;
}

function advance(step) {
  if (!cands.length) return;
  sel = (sel + step + cands.length) % cands.length;
  [...candList.children].forEach((li, i) => li.classList.toggle('on', i === sel));
}

// ── 確定 ────────────────────────────────────────────────────

/** 読みを持つた塊として紙へ置く */
function place(yomi, surface) {
  const node = preeditNode();
  const block = document.createElement('span');
  block.className = 'kata';
  block.dataset.yomi = yomi;
  block.dataset.id = String(++blockId);
  // いま見えてゐる字を控へておく。次に見たとき違つてゐたら、それが教はつた内容である。
  block.dataset.taught = surface;
  block.textContent = surface;

  if (node) node.replaceWith(block);
  else caretRange().insertNode(block);
  caretAfter(block);
}

function take() {
  const yomi = core.commit();          // 仮名（＝読み）を貰ひ、作業帯を空ける
  if (!yomi) { hideBar(); return; }
  const chosen = cands[sel] ? cands[sel].surface : yomi;
  // **旧字は差し込むときに機械で決まる。** 使ひ手が旧字を覚える必要は無い。
  place(yomi, core.kyuji(chosen));
  hideBar();
  paintBoard();
}

/**
 * 未確定を**仮名のまま**置いて終はる。
 *
 * 欄から焦点が外れたときに使ふ。打つた仮名を捨てるのは論外だが、
 * 選んでもゐない候補を勝手に採るのも違ふ——だから仮名のまま置く。
 * （実物を動かして分かつた穴。塞がないと、欄の外を触つた瞬間に
 *  未確定の一続きが宙に浮いたまま残る。）
 */
function commitKana() {
  const yomi = core.commit();
  if (!yomi) { hideBar(); return; }
  place(yomi, core.kyuji(yomi));
  hideBar();
  paintBoard();
}

// ── 打鍵 ────────────────────────────────────────────────────

paper.addEventListener('keydown', (ev) => {
  if (!core) return;
  // 短絡キー（Ctrl+C 等）は食はない。判断は殻が先にする。
  if (ev.ctrlKey || ev.metaKey || ev.altKey) return;

  const composing = core.preedit().length > 0;
  if (composing) {
    switch (ev.code) {
      case 'Escape':
        core.cancel(); ev.preventDefault(); render(); return;
      case 'Backspace':
        core.backspace(); ev.preventDefault(); render(); return;
      case 'Space':
        ev.preventDefault();
        if (cands.length) advance(ev.shiftKey ? -1 : 1); else take();
        return;
      case 'Enter':
        ev.preventDefault(); take(); return;
      case 'ArrowDown': case 'ArrowRight':
        if (cands.length) { ev.preventDefault(); advance(1); return; }
        break;
      case 'ArrowUp': case 'ArrowLeft':
        if (cands.length) { ev.preventDefault(); advance(-1); return; }
        break;
    }
  }

  // ここから先は原器の領分か否か。判定は核が持つ地図で決まる（頁は写しを持たない）。
  const r = core.press(ev.code);
  if (r === 0) return;                 // 素通し（Space・Enter・記号・機能キー）
  ev.preventDefault();
  render();
});

// ── 学習の収穫 ──────────────────────────────────────────────
//
// 契機は input の落ち着き（デバウンス）と、欄から離れたとき。

paper.addEventListener('input', () => {
  clearTimeout(harvestTimer);
  harvestTimer = setTimeout(harvest, 600);
});
paper.addEventListener('blur', () => {
  clearTimeout(harvestTimer);
  if (core && core.preedit()) commitKana();   // 未確定を宙に浮かせない
  harvest();
});

function harvest() {
  mendSplitBlocks();
  let learned = 0;
  for (const el of paper.querySelectorAll('.kata')) {
    const yomi = el.dataset.yomi;
    const now = el.textContent;
    if (!yomi || !now || now === el.dataset.taught) continue;
    if (manabi.learn(yomi, now)) { el.dataset.taught = now; learned++; }
  }
  if (learned) drawManabi();
}

/**
 * contenteditable の癖の手当て。
 *
 * 塊の中で改行したり貼り付けたりすると、ブラウザは span を**複製して割る**ことがある。
 * 割れたまま収穫すると、一つの読みに対して半端な表記を二つ覚えてしまふ。
 * 同じ id を持つ span が複数居たら、隣り合ふものを繋ぎ直してから収穫する。
 *
 * （§9「編輯欄の実体」は contenteditable と textarea の二択で未決だつたが、
 *  読みを塊に貼り付けたまま編輯させられるのは前者だけなので採つた。
 *  代償がこの手当てで、実物を見て分かつた癖はここに集めてある。）
 */
function mendSplitBlocks() {
  const byId = new Map();
  for (const el of paper.querySelectorAll('.kata')) {
    const id = el.dataset.id;
    if (!id) continue;
    if (!byId.has(id)) { byId.set(id, el); continue; }
    const first = byId.get(id);
    first.textContent += el.textContent;
    el.remove();
  }
}

// ── 原器の図 ────────────────────────────────────────────────

function drawBoard() {
  board.replaceChildren(...ROWS.map((row) => {
    const div = document.createElement('div');
    div.className = 'row';
    for (const code of row) {
      const key = document.createElement('kbd');
      key.dataset.code = code;
      const entry = layout.get(code);
      if (!entry) { key.className = 'dead'; div.append(key); continue; }
      const kana = document.createElement('b');
      kana.textContent = entry.first || '';
      const alt = document.createElement('i');
      alt.textContent = entry.second || '';
      if (!entry.first && !entry.second) key.classList.add('mod');
      key.append(kana, alt);
      div.append(key);
    }
    return div;
  }));
  // 面と逸らしの鍵に名前を付ける（仮名を持たないので図では読めない）
  const label = { Equal: '前置', KeyB: '濁', KeyV: '半濁' };
  for (const [code, text] of Object.entries(label)) {
    const el = board.querySelector(`[data-code="${code}"] b`);
    if (el) el.textContent = text;
  }
}

function paintBoard() {
  const shifted = core.isShifted();
  board.classList.toggle('shifted', shifted);
  const ink = core.kehai();
  for (const key of board.querySelectorAll('kbd[data-code]')) {
    key.style.setProperty('--ink', String(ink.get(key.dataset.code) || 0));
  }
}

// ── 覚えたことの一覧・持ち運び ──────────────────────────────

function drawManabi() {
  $('#manabi-count').textContent = String(manabi.count());
  const tbody = $('#manabi tbody');
  const rows = [];
  for (const yomi of Object.keys(manabi.data).sort()) {
    for (const [surface, n] of manabi.of(yomi)) {
      const tr = document.createElement('tr');
      for (const text of [yomi, surface, String(n)]) {
        const td = document.createElement('td');
        td.textContent = text;
        tr.append(td);
      }
      const td = document.createElement('td');
      const b = document.createElement('button');
      b.type = 'button';
      b.textContent = '忘れる';
      b.addEventListener('click', () => { manabi.forgetOne(yomi, surface); drawManabi(); });
      td.append(b);
      tr.append(td);
      rows.push(tr);
    }
  }
  tbody.replaceChildren(...rows);
}

$('#export').addEventListener('click', () => {
  // 書き出しは端末の中で完結する（cross-platform.md §9-2 の A 案）
  const blob = new Blob([JSON.stringify(manabi.data, null, 1)], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = 'yatate-manabi.json';
  a.click();
  URL.revokeObjectURL(a.href);
});

$('#import-file').addEventListener('change', async (ev) => {
  const file = ev.target.files && ev.target.files[0];
  if (!file) return;
  try {
    const added = manabi.merge(JSON.parse(await file.text()));
    drawManabi();
    setStatus(`${added} 対を読み込みました。`);
  } catch {
    setStatus('読み込めませんでした（JSON が壊れてゐます）。');
  }
  ev.target.value = '';
});

$('#forget').addEventListener('click', () => {
  if (!confirm('覚えた対を全部忘れます。よろしいですか。')) return;
  manabi.data = {};
  manabi.save();
  drawManabi();
});

function setStatus(text) { $('#status').textContent = text; }

// ── 起動 ────────────────────────────────────────────────────

(async () => {
  try {
    core = await Core.load();
  } catch (e) {
    setStatus('核を読み込めませんでした。web/build.sh を走らせて yatate.wasm を置いてください。');
    console.error(e);
    return;
  }
  layout = core.layout();
  manabi.load();
  drawBoard();
  paintBoard();
  drawManabi();
  setStatus(`原器 ${layout.size} 鍵・覚えた対 ${manabi.count()}。通信はこの頁と核の取得だけです。`);
  paper.focus();
})();
