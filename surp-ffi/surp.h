/*
 * surp.h — C FFI header for the Surp binary format library.
 *
 * Memory ownership:
 *   - surp_encode_buffer: allocates *out; caller must free with surp_free(*out, *out_len).
 *   - surp_decode_buffer: allocates *json_out; caller must free with surp_free(*json_out, *json_len).
 *   - surp_free: frees memory allocated by this library. Safe to call with NULL.
 *
 * All functions return 0 on success, -1 on error.
 *
 * License: MIT OR Apache-2.0
 */

#ifndef SURP_H
#define SURP_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Encode a JSON string to Surp binary format.
 *
 * @param in_ptr    Pointer to JSON text (UTF-8).
 * @param in_len    Length of JSON text in bytes.
 * @param out_ptr   [out] Pointer to allocated Surp binary output.
 * @param out_len   [out] Length of output in bytes.
 * @return          0 on success, -1 on error.
 */
int surp_encode_buffer(
    const uint8_t *in_ptr,
    size_t in_len,
    uint8_t **out_ptr,
    size_t *out_len
);

/**
 * Decode Surp binary data to a JSON string.
 *
 * @param in_ptr    Pointer to Surp binary data.
 * @param in_len    Length of input in bytes.
 * @param json_out  [out] Pointer to allocated JSON string (UTF-8, not null-terminated).
 * @param json_len  [out] Length of JSON string in bytes.
 * @return          0 on success, -1 on error.
 */
int surp_decode_buffer(
    const uint8_t *in_ptr,
    size_t in_len,
    uint8_t **json_out,  /* actually char** but using uint8_t for ABI */
    size_t *json_len
);

/**
 * Free memory allocated by surp_encode_buffer or surp_decode_buffer.
 *
 * @param ptr   Pointer to free (may be NULL — no-op).
 * @param len   Length of the allocation.
 */
void surp_free(uint8_t *ptr, size_t len);

#ifdef __cplusplus
}
#endif

#endif /* SURP_H */
