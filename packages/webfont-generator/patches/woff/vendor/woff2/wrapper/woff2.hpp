#ifndef WOFF2_H
#define WOFF2_H

#ifdef __cplusplus
extern "C" {
#endif

int ConvertTTFToWOFF2(
    const uint8_t *data,
    size_t length,
    uint8_t *result,
    size_t *result_length,
    const char* extended_metadata,
    size_t extended_metadata_length,
    int brotli_quality,
    int allow_transforms
);

int ConvertWOFF2ToTTF(
    uint8_t *result,
    size_t result_length,
    const uint8_t *data,
    size_t length
);

size_t ComputeTTFToWOFF2Size(
    const uint8_t *data,
    size_t length,
    const char* extended_metadata,
    size_t extended_metadata_length
);

size_t ComputeWOFF2ToTTFSize(const uint8_t *data, size_t length);

#ifdef __cplusplus
}
#endif

#endif
