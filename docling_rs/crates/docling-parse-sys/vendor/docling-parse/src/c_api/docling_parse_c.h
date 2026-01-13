// Stub C API header for docling-parse
// This is a temporary stub to allow workspace builds to succeed
// The actual C API implementation is not yet available
// See: reports/feature-phase-a-archives/N14_DOCLING_PARSE_C_API_BLOCKER_2025-11-07-11-01.md
//
// TODO: Implement full C API wrapper around docling-parse C++ library

#ifndef DOCLING_PARSE_C_H
#define DOCLING_PARSE_C_H

#include <stddef.h>  // For size_t

#ifdef __cplusplus
extern "C" {
#endif

// Opaque parser handle
typedef struct DoclingParser DoclingParser;

// String type for returning text data
typedef struct {
    char* data;
    size_t length;
} DoclingString;

// Error codes
typedef enum {
    DOCLING_OK = 0,
    DOCLING_ERROR_OUT_OF_MEMORY = 1,
    DOCLING_ERROR_NOT_LOADED = 2,
    DOCLING_ERROR_INVALID_PARAM = 3,
    DOCLING_ERROR_FILE_NOT_FOUND = 4,
    DOCLING_ERROR_LOAD_FAILED = 5,
    DOCLING_ERROR_PARSE_FAILED = 6,
    DOCLING_ERROR_NOT_IMPLEMENTED = 99,
} DoclingError;

// Parser lifecycle - STUB IMPLEMENTATIONS
DoclingParser* docling_parser_new(const char* loglevel);
void docling_parser_free(DoclingParser* parser);

// Document management - STUB IMPLEMENTATIONS
DoclingError docling_parser_load_document(DoclingParser* parser,
                                          const char* key,
                                          const char* filename,
                                          const char* password);
DoclingError docling_parser_unload_document(DoclingParser* parser,
                                            const char* key);
int docling_parser_is_loaded(DoclingParser* parser, const char* key);
int docling_parser_number_of_pages(DoclingParser* parser, const char* key);

// Parsing - STUB IMPLEMENTATIONS
DoclingError docling_parser_parse_page(DoclingParser* parser,
                                       const char* key,
                                       int page_num,
                                       DoclingString* output);
DoclingError docling_parser_parse_all_pages(DoclingParser* parser,
                                             const char* key,
                                             DoclingString* output);

// Memory management - STUB IMPLEMENTATIONS
void docling_string_free(DoclingString* str);

#ifdef __cplusplus
}
#endif

#endif // DOCLING_PARSE_C_H
