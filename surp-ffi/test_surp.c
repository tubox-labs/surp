/*
 * test_surp.c — Simple test program for the Surp C FFI.
 *
 * Build:
 *   cargo build --release -p surp-ffi
 *   cc -o test_surp test_surp.c -L target/release -lsurp_ffi
 *
 * Run:
 *   LD_LIBRARY_PATH=target/release ./test_surp
 *   # or on macOS: DYLD_LIBRARY_PATH=target/release ./test_surp
 */

#include <stdio.h>
#include <string.h>
#include "surp.h"

int main(void) {
    const char *json = "{\"name\":\"Alice\",\"age\":30}";
    uint8_t *surp_buf = NULL;
    size_t surp_len = 0;

    printf("Input JSON: %s\n", json);

    /* Encode JSON → Surp */
    int rc = surp_encode_buffer(
        (const uint8_t *)json, strlen(json),
        &surp_buf, &surp_len
    );
    if (rc != 0) {
        fprintf(stderr, "Encode failed!\n");
        return 1;
    }
    printf("Encoded to %zu bytes of Surp binary\n", surp_len);

    /* Decode Surp → JSON */
    uint8_t *json_out = NULL;
    size_t json_len = 0;
    rc = surp_decode_buffer(surp_buf, surp_len, &json_out, &json_len);
    if (rc != 0) {
        fprintf(stderr, "Decode failed!\n");
        surp_free(surp_buf, surp_len);
        return 1;
    }

    printf("Decoded JSON (%zu bytes):\n", json_len);
    fwrite(json_out, 1, json_len, stdout);
    printf("\n");

    /* Cleanup */
    surp_free(surp_buf, surp_len);
    surp_free(json_out, json_len);

    printf("SUCCESS: FFI roundtrip complete.\n");
    return 0;
}
