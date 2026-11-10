/* Host byte codec only: no engine semantics or rendering. Caller owns all buffers.
 * Spans/entries are validated before use. SHA256 uses the system CommonCrypto.
 * ABI v1; 64-bit macOS, fixed uint32 layouts, no allocation or retained pointer. */
#include <CommonCrypto/CommonDigest.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>

typedef struct { uint32_t base, length, offset; } MacOSEntry;
typedef struct { uint32_t offset, length, zero; } MacOSSpan;
uint32_t macos_projection_codec_version(void) { return 1; }
static uint32_t le32(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1]<<8) | ((uint32_t)p[2]<<16) | ((uint32_t)p[3]<<24);
}
/* 0 = exact known layout; 1 = changed/invalid layout, parse strictly in caller. */
int32_t macos_projection_validate(const uint8_t *bytes, uint32_t length,
                                 const MacOSEntry *entries, uint32_t count) {
    if (!bytes || length<4 || count>65536 || (count && !entries) || le32(bytes)!=count) return 1;
    uint64_t next=4, end=0, total=0;
    for (uint32_t i=0;i<count;i++) {
        MacOSEntry e=entries[i];
        if (!e.length || next+8>length || next+8!=e.offset || e.base<end ||
            (uint64_t)e.base+e.length>0x100000000ULL || next+8+e.length>length ||
            le32(bytes+next)!=e.base || le32(bytes+next+4)!=e.length) return 1;
        next+=8+e.length; end=(uint64_t)e.base+e.length; total+=e.length;
        if (total>64*1024*1024) return 1;
    }
    return next==length ? 0 : 1;
}
/* 0 success; 1 invalid spans/arguments. Digest untouched on validation failure. */
int32_t macos_projection_sha256(const uint8_t *bytes, uint32_t length,
                              const MacOSSpan *spans, uint32_t count, uint8_t *digest) {
    if (!bytes || !digest || (count && !spans) || count>400000) return 1;
    for (uint32_t i=0;i<count;i++) {
        MacOSSpan s=spans[i];
        if (s.zero>1 || (!s.zero && (uint64_t)s.offset+s.length>length)) return 1;
    }
    static const uint8_t zeros[4096]={0};
    CC_SHA256_CTX ctx;
    CC_SHA256_Init(&ctx);
    for (uint32_t i=0;i<count;i++) {
        MacOSSpan s=spans[i];
        if (!s.zero) CC_SHA256_Update(&ctx,bytes+s.offset,s.length);
        else {
            uint32_t remaining=s.length;
            while (remaining) {
                uint32_t n=remaining>sizeof(zeros) ? (uint32_t)sizeof(zeros) : remaining;
                CC_SHA256_Update(&ctx,zeros,n); remaining-=n;
            }
        }
    }
    CC_SHA256_Final(digest,&ctx);
    return 0;
}
