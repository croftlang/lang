/* Std.Http through libcurl, for the CurlHttp adapter. Written out only for
   a target that binds Http to CurlHttp; link the program with -lcurl. */
#include <curl/curl.h>
#include "program.h"

/* How many redirects a request follows before it fails with Unreachable. */
#define CROFT_HTTP_REDIRECTS 10L

/* Where libcurl puts the answer, and whether it ran out of room. */
typedef struct {
    croft_bytes *body;
    croft_bool too_large;
} croft_http_answer;

static size_t collect(char *data, size_t size, size_t count, void *user)
{
    croft_http_answer *answer = (croft_http_answer *) user;
    size_t length = size * count;
    size_t index;
    for (index = 0; index < length; index++) {
        if (answer->body->length >= CROFT_BYTES_CAPACITY) {
            answer->too_large = 1;
            return 0;
        }
        answer->body->items[answer->body->length] = (unsigned char) data[index];
        answer->body->length++;
    }
    return length;
}

/* Whether a header line is a status line, such as "HTTP/1.1 301 Moved",
   which starts each response of a redirect chain. */
static croft_bool starts_response(const char *data, size_t length)
{
    return length >= 5 && data[0] == 'H' && data[1] == 'T' && data[2] == 'T'
        && data[3] == 'P' && data[4] == '/';
}

/* Each header line, "Name: value", kept with its name in lower case. A new
   status line starts the headers again, so only the last response's are kept.
   A header whose name or value is longer than a string is left out, rather
   than kept cut short. */
static size_t keep_header(char *data, size_t size, size_t count, void *user)
{
    croft_result_Std_Http_Response_error *out = (croft_result_Std_Http_Response_error *) user;
    size_t length = size * count;
    size_t colon = 0;
    size_t start;
    size_t end;
    size_t index;
    croft_u16 at;
    if (starts_response(data, length)) {
        croft_clear(&out->value.headers, sizeof out->value.headers);
        return length;
    }
    at = out->value.headers.length;
    while (colon < length && data[colon] != ':') {
        colon++;
    }
    if (colon == length || at >= sizeof out->value.headers.keys / sizeof out->value.headers.keys[0]) {
        return length;
    }
    start = colon + 1;
    while (start < length && data[start] == ' ') {
        start++;
    }
    end = start;
    while (end < length && data[end] != '\r' && data[end] != '\n') {
        end++;
    }
    if (colon > CROFT_STRING_CAPACITY || end - start > CROFT_STRING_CAPACITY) {
        return length;
    }
    for (index = 0; index < colon; index++) {
        char c = data[index];
        out->value.headers.keys[at].text[index] = (char) (c >= 'A' && c <= 'Z' ? c - 'A' + 'a' : c);
    }
    out->value.headers.keys[at].length = (croft_u16) colon;
    for (index = start; index < end; index++) {
        out->value.headers.values[at].text[index - start] = data[index];
    }
    out->value.headers.values[at].length = (croft_u16) (end - start);
    out->value.headers.length++;
    return length;
}

static void request(const char *method, croft_string_view url, const croft_bytes *body,
                    croft_result_Std_Http_Response_error *out)
{
    char address[1024];
    croft_http_answer answer;
    CURL *curl;
    CURLcode code;
    long status = 0;
    croft_u16 index;
    croft_clear(out, sizeof *out);
    for (index = 0; index < url.length && index < sizeof address - 1; index++) {
        address[index] = url.text[index];
    }
    address[index] = '\0';
    answer.body = &out->value.body;
    answer.too_large = 0;
    curl = curl_easy_init();
    if (curl == 0) {
        out->error.code = CROFT_ERROR_Std_Http_Unreachable;
        return;
    }
    curl_easy_setopt(curl, CURLOPT_URL, address);
    curl_easy_setopt(curl, CURLOPT_CUSTOMREQUEST, method);
    curl_easy_setopt(curl, CURLOPT_FOLLOWLOCATION, 1L);
    curl_easy_setopt(curl, CURLOPT_MAXREDIRS, CROFT_HTTP_REDIRECTS);
#if LIBCURL_VERSION_NUM >= 0x075500
    curl_easy_setopt(curl, CURLOPT_PROTOCOLS_STR, "http,https");
    curl_easy_setopt(curl, CURLOPT_REDIR_PROTOCOLS_STR, "http,https");
#else
    curl_easy_setopt(curl, CURLOPT_PROTOCOLS, (long) (CURLPROTO_HTTP | CURLPROTO_HTTPS));
    curl_easy_setopt(curl, CURLOPT_REDIR_PROTOCOLS, (long) (CURLPROTO_HTTP | CURLPROTO_HTTPS));
#endif
    curl_easy_setopt(curl, CURLOPT_TIMEOUT, 30L);
    curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, collect);
    curl_easy_setopt(curl, CURLOPT_WRITEDATA, &answer);
    curl_easy_setopt(curl, CURLOPT_HEADERFUNCTION, keep_header);
    curl_easy_setopt(curl, CURLOPT_HEADERDATA, out);
    if (body != 0) {
        curl_easy_setopt(curl, CURLOPT_POSTFIELDS, (const char *) body->items);
        curl_easy_setopt(curl, CURLOPT_POSTFIELDSIZE, (long) body->length);
    }
    code = curl_easy_perform(curl);
    curl_easy_getinfo(curl, CURLINFO_RESPONSE_CODE, &status);
    curl_easy_cleanup(curl);
    if (answer.too_large) {
        croft_clear(out, sizeof *out);
        out->error.code = CROFT_ERROR_Std_Http_TooLarge;
    } else if (code == CURLE_OPERATION_TIMEDOUT) {
        out->error.code = CROFT_ERROR_Std_Http_TimedOut;
    } else if (code != CURLE_OK) {
        croft_clear(out, sizeof *out);
        out->error.code = CROFT_ERROR_Std_Http_Unreachable;
    } else {
        out->ok = 1;
        out->value.status = status;
    }
}

void croft_http_get(croft_string_view url, croft_result_Std_Http_Response_error *out)
{
    request("GET", url, 0, out);
}

void croft_http_post(croft_string_view url, const croft_bytes *body, croft_result_Std_Http_Response_error *out)
{
    request("POST", url, body, out);
}

void croft_http_put(croft_string_view url, const croft_bytes *body, croft_result_Std_Http_Response_error *out)
{
    request("PUT", url, body, out);
}

void croft_http_delete(croft_string_view url, croft_result_Std_Http_Response_error *out)
{
    request("DELETE", url, 0, out);
}
