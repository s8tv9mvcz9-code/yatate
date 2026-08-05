/* 矢立の核 — Apple 側の入口（C ABI）。
 *
 * 実体は Rust（apple/src/lib.rs）で、ここはその名乗りである。
 * **手で書いてゐるが、機械が縛つてゐる**——`cargo test` の
 * `頭書と実装が一致する` が、この頭書と `#[no_mangle]` の集合を突き合はせる。
 * 片方だけ直せばテストが落ちるので、静かにずれることはない。
 *
 * ## 約束
 *
 *   文字列  返り値は UTF-8 の NUL 終端。呼んだ側が yatate_string_free で返す
 *   文字    uint32_t のスカラ値。0 は「無い」
 *   手綱    NULL を渡しても落ちない（殻の誤りでアプリごと死なせない）
 */

#ifndef YATATE_FFI_H
#define YATATE_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── 行為の符号（殻への指示） ────────────────────────────── */

/** 矢立の鍵ではない。殻は OS へ素通しする。 */
#define YATATE_ACT_PASSTHROUGH 0
/** 未確定文字列が変はつた。殻は描き直す。 */
#define YATATE_ACT_UPDATE 1
/** 食つたが見た目は変はらない。 */
#define YATATE_ACT_SWALLOW 2
/** 確定した。yatate_henkan_take_commit で文字列を取る。 */
#define YATATE_ACT_COMMIT 3
/** 確定した上で、続けて新しい未確定が立つてゐる。 */
#define YATATE_ACT_COMMIT_THEN_UPDATE 4
/** 捨てて閉ぢる。 */
#define YATATE_ACT_CANCEL 5

/** 仮名を積んでゐる段。 */
#define YATATE_PHASE_KANA 0
/** 変換して文節を選び直してゐる段。 */
#define YATATE_PHASE_HENKAN 1

/** 仮名を足す。 */
#define YATATE_EDIT_INSERT 0
/** 直前の一字を差し替へる（濁点・半濁点の後置打鍵）。 */
#define YATATE_EDIT_REPLACE_LAST 1
/** 目に見える変化なし（前置シフトが立つた等）。 */
#define YATATE_EDIT_NONE 2
/** この鍵は原器に無い。 */
#define YATATE_EDIT_UNMAPPED 3

/* ── 文字列 ──────────────────────────────────────────────── */

void yatate_string_free(char *s);

/* ── 卓（変換まで含む一つの入力欄） ──────────────────────── */

typedef struct YatateHenkanHandle YatateHenkanHandle;

YatateHenkanHandle *yatate_henkan_new(void);
void yatate_henkan_free(YatateHenkanHandle *h);

int32_t yatate_henkan_key(YatateHenkanHandle *h, uint32_t genki);
int32_t yatate_henkan_insert_kana(YatateHenkanHandle *h, const char *kana);
int32_t yatate_henkan_convert(YatateHenkanHandle *h);
int32_t yatate_henkan_next_candidate(YatateHenkanHandle *h);
int32_t yatate_henkan_prev_candidate(YatateHenkanHandle *h);
int32_t yatate_henkan_choose(YatateHenkanHandle *h, size_t index);
int32_t yatate_henkan_focus_next(YatateHenkanHandle *h);
int32_t yatate_henkan_focus_prev(YatateHenkanHandle *h);
int32_t yatate_henkan_grow_focus(YatateHenkanHandle *h);
int32_t yatate_henkan_shrink_focus(YatateHenkanHandle *h);
int32_t yatate_henkan_backspace(YatateHenkanHandle *h);
int32_t yatate_henkan_unconvert(YatateHenkanHandle *h);
int32_t yatate_henkan_cancel(YatateHenkanHandle *h);
int32_t yatate_henkan_commit(YatateHenkanHandle *h);
void yatate_henkan_reset(YatateHenkanHandle *h);

char *yatate_henkan_take_commit(YatateHenkanHandle *h);
char *yatate_henkan_preedit(YatateHenkanHandle *h);
char *yatate_henkan_yomi(YatateHenkanHandle *h);
char *yatate_henkan_candidates(YatateHenkanHandle *h);
char *yatate_henkan_segments(YatateHenkanHandle *h);
char *yatate_henkan_kehai(YatateHenkanHandle *h);

int32_t yatate_henkan_phase(YatateHenkanHandle *h);
int32_t yatate_henkan_is_composing(YatateHenkanHandle *h);
int32_t yatate_henkan_is_shifted(YatateHenkanHandle *h);
int32_t yatate_henkan_wants_key(YatateHenkanHandle *h, uint32_t genki);
size_t yatate_henkan_focus(YatateHenkanHandle *h);
size_t yatate_henkan_chosen(YatateHenkanHandle *h);
void yatate_henkan_focus_range(YatateHenkanHandle *h, size_t *start, size_t *len);

/* ── 素の打鍵（変換を持たない道） ────────────────────────── */

typedef struct YatateSessionHandle YatateSessionHandle;

YatateSessionHandle *yatate_session_new(void);
void yatate_session_free(YatateSessionHandle *h);

int32_t yatate_session_key(YatateSessionHandle *h, uint32_t genki);
int32_t yatate_session_insert_kana(YatateSessionHandle *h, const char *kana);
int32_t yatate_session_commit(YatateSessionHandle *h);
int32_t yatate_session_backspace(YatateSessionHandle *h);
void yatate_session_cancel(YatateSessionHandle *h);

char *yatate_session_take_commit(YatateSessionHandle *h);
char *yatate_session_preedit(YatateSessionHandle *h);
char *yatate_session_kehai(YatateSessionHandle *h);

int32_t yatate_session_is_composing(YatateSessionHandle *h);
int32_t yatate_session_is_shifted(YatateSessionHandle *h);
int32_t yatate_session_wants_key(YatateSessionHandle *h, uint32_t genki);

/* ── 原器の状態機械（前置シフトの逐次性） ────────────────── */

typedef struct YatateGenkiHandle YatateGenkiHandle;

YatateGenkiHandle *yatate_genki_new(void);
void yatate_genki_free(YatateGenkiHandle *h);
int32_t yatate_genki_press(YatateGenkiHandle *h, uint32_t key, uint32_t last, char **text);
int32_t yatate_genki_is_shifted(YatateGenkiHandle *h);
void yatate_genki_reset(YatateGenkiHandle *h);

/* ── 表（すべて核から起こす） ────────────────────────────── */

char *yatate_to_kyuji(const char *text);
char *yatate_type_keys(const char *keys);
char *yatate_kagi_table(void);
char *yatate_genki_planes(void);
char *yatate_genki_special_keys(void);
char *yatate_gojuon_table(void);
char *yatate_gojuon_lines(void);
char *yatate_gojuon_reverse(void);
char *yatate_kyuji_table(void);
char *yatate_kehai_field(const char *prev);
uint32_t yatate_kehai_min_evidence(void);
char *yatate_segment(const char *yomi);

/* ── 位置から原器の文字を引く ────────────────────────────── */

uint32_t yatate_genki_of_mac(uint16_t mac);
uint32_t yatate_genki_of_hid(uint16_t hid);
uint32_t yatate_genki_of_scan(uint16_t scan);
uint32_t yatate_genki_of_code(const char *code);
uint32_t yatate_dakuten(uint32_t kana);
uint32_t yatate_handakuten(uint32_t kana);

#ifdef __cplusplus
}
#endif

#endif /* YATATE_FFI_H */
