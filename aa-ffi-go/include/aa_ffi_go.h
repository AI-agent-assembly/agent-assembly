#ifndef AA_FFI_GO_H
#define AA_FFI_GO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int32_t aa_status;

enum {
  AA_STATUS_OK = 0,
  AA_STATUS_NULL_POINTER = 1,
  AA_STATUS_INVALID_UTF8 = 2,
  AA_STATUS_NOT_CONNECTED = 3,
  AA_STATUS_MUTEX_POISONED = 4,
};

typedef struct aa_client_handle aa_client_handle;

typedef struct aa_bytes {
  uint8_t* ptr;
  size_t len;
} aa_bytes;

typedef struct aa_string {
  char* ptr;
} aa_string;

aa_status aa_connect(const char* endpoint, aa_client_handle** out_client);
aa_status aa_send_event(aa_client_handle* client, const char* event_json);
aa_status aa_query_policy(
    aa_client_handle* client,
    const char* query_json,
    char** out_response
);

#ifdef __cplusplus
}
#endif

#endif // AA_FFI_GO_H
