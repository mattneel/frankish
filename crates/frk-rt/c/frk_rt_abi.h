/* frk_rt_abi.h — GENERATED from crates/frk-abi (M17, D-062).
 * DO NOT EDIT: `make abi` regenerates; a frk-rt test asserts drift.
 * This header IS the C twin's contract: including it makes the C
 * compiler enforce the registered ABI at every compile, on every
 * grid triple. */
#ifndef FRK_RT_ABI_H
#define FRK_RT_ABI_H

#include <stdint.h>

uint64_t frk_rt_alloc_count(void);
void *frk_rt_arena_alloc(uint64_t);
void *frk_rt_rc_alloc(uint64_t, uint64_t);
void frk_rt_rc_collect(void);
uint64_t frk_rt_rc_free_count(void);
void frk_rt_rc_release(void *);
uint64_t frk_rt_rc_release_count(void);
void frk_rt_rc_retain(void *);
void *frk_rt_str_concat(const uint8_t *, const uint8_t *);
int64_t frk_rt_str_eq(const uint8_t *, const uint8_t *);
void *frk_rt_str_from_units(const uint16_t *, uint64_t);
uint64_t frk_rt_str_len(const uint8_t *);
void *frk_rt_bstr_concat(const uint8_t *, const uint8_t *);
void *frk_rt_bstr_from_num(double);
void *frk_rt_bstr_intern(const uint8_t *, uint64_t);
void *frk_rt_bstr_rep(const uint8_t *, int64_t);
void *frk_rt_bstr_sub(const uint8_t *, int64_t, int64_t);
void frk_rt_dyn_check(int64_t, int64_t);
void frk_rt_table_init(int64_t);
int64_t frk_rt_table_len(int64_t);
void frk_rt_table_next(int64_t, int64_t, int64_t, int64_t *);
void frk_rt_table_raw_get(int64_t, int64_t, int64_t, int64_t *);
void frk_rt_table_raw_set(int64_t, int64_t, int64_t, int64_t, int64_t);
void frk_rt_ctl_abort(int64_t, int64_t, int64_t);
int64_t frk_rt_ctl_pending(void);
int64_t frk_rt_ctl_prompt_enter(void);
void frk_rt_ctl_prompt_exit(int64_t);
int64_t frk_rt_ctl_resolve(int64_t, int64_t *);
void frk_rt_print_bool(int64_t);
void frk_rt_print_f64(double);
void frk_rt_print_str(const uint8_t *);
void frk_rt_lua_error(int64_t);
void frk_rt_print_lua_str(const uint8_t *);
void frk_rt_scm_display_bool(int64_t);
void frk_rt_scm_display_num(double);
void frk_rt_scm_newline(void);

#endif /* FRK_RT_ABI_H */
