/* Std.Http through libcurl, for the CurlHttp adapter. Written out only for
   a target that binds Http to CurlHttp; link the program with -lcurl. */
#include <curl/curl.h>
#include "program.h"

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

/* Each header line, "Name: value", kept with its name in lower case. */
static size_t keep_header(char *data, size_t size, size_t count, void *user)
{
    croft_result_Std_Http_Response_error *out = (croft_result_Std_Http_Response_error *) user;
    size_t length = size * count;
    size_t colon = 0;
    size_t index;
    croft_u16 at = out->value.headers.length;
    while (colon < length && data[colon] != ':') {
        colon++;
    }
    if (colon == length || at >= sizeof out->value.headers.keys / sizeof out->value.headers.keys[0]) {
        return length;
    }
    out->value.headers.keys[at].length = 0;
    for (index = 0; index < colon && index < CROFT_STRING_CAPACITY; index++) {
        char c = data[index];
        out->value.headers.keys[at].text[index] = (char) (c >= 'A' && c <= 'Z' ? c - 'A' + 'a' : c);
        out->value.headers.keys[at].length++;
    }
    index = colon + 1;
    while (index < length && data[index] == ' ') {
        index++;
    }
    out->value.headers.values[at].length = 0;
    for (; index < length && data[index] != '\r' && data[index] != '\n'
           && out->value.headers.values[at].length < CROFT_STRING_CAPACITY; index++) {
        out->value.headers.values[at].text[out->value.headers.values[at].length] = data[index];
        out->value.headers.values[at].length++;
    }
    out->value.headers.length++;
    return length;
}

static void request(const char *method, const croft_string *url, const croft_bytes *body,
                    croft_result_Std_Http_Response_error *out)
{
    char address[CROFT_STRING_CAPACITY + 1];
    croft_http_answer answer;
    CURL *curl;
    CURLcode code;
    long status = 0;
    croft_u16 index;
    croft_clear(out, sizeof *out);
    for (index = 0; index < url->length; index++) {
        address[index] = url->text[index];
    }
    address[url->length] = '\0';
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

void croft_http_get(const croft_string *url, croft_result_Std_Http_Response_error *out)
{
    request("GET", url, 0, out);
}

void croft_http_post(const croft_string *url, const croft_bytes *body, croft_result_Std_Http_Response_error *out)
{
    request("POST", url, body, out);
}

void croft_http_put(const croft_string *url, const croft_bytes *body, croft_result_Std_Http_Response_error *out)
{
    request("PUT", url, body, out);
}

void croft_http_delete(const croft_string *url, croft_result_Std_Http_Response_error *out)
{
    request("DELETE", url, 0, out);
}
