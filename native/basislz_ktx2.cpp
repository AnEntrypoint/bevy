#include "basisu_transcoder.h"

namespace {

void ensure_transcoder_init() {
    static const bool initialized = [] {
        basist::basisu_transcoder_init();
        return true;
    }();
    (void)initialized;
}

basist::transcoder_texture_format output_format(uint32_t format) {
    switch (format) {
        case 1:
            return basist::transcoder_texture_format::cTFBC7_RGBA;
        case 2:
            return basist::transcoder_texture_format::cTFETC2_RGBA;
        case 3:
            return basist::transcoder_texture_format::cTFASTC_4x4_RGBA;
        default:
            return basist::transcoder_texture_format::cTFRGBA32;
    }
}

}

extern "C" {

int32_t bevy_basislz_info(const uint8_t* data, uint32_t size, uint32_t* width, uint32_t* height, uint32_t* levels, uint32_t* layers, uint32_t* faces) {
    ensure_transcoder_init();
    basist::ktx2_transcoder transcoder;
    if (!transcoder.init(data, size)) {
        return 1;
    }
    *width = transcoder.get_width();
    *height = transcoder.get_height();
    *levels = transcoder.get_levels();
    *layers = transcoder.get_header().m_layer_count;
    *faces = transcoder.get_faces();
    return 0;
}

int64_t bevy_basislz_transcode_level(const uint8_t* data, uint32_t size, uint32_t level, uint8_t* out, uint64_t capacity, uint32_t format) {
    ensure_transcoder_init();
    basist::ktx2_transcoder transcoder;
    if (!transcoder.init(data, size) || !transcoder.start_transcoding()) {
        return -1;
    }
    basist::ktx2_image_level_info info;
    if (!transcoder.get_image_level_info(info, level, 0, 0)) {
        return -2;
    }
    const basist::transcoder_texture_format block_format = output_format(format);
    if (block_format != basist::transcoder_texture_format::cTFRGBA32) {
        const uint64_t blocks = static_cast<uint64_t>(info.m_num_blocks_x) * info.m_num_blocks_y;
        if (capacity < blocks * 16) {
            return -3;
        }
        if (!transcoder.transcode_image_level(level, 0, 0, out, static_cast<uint32_t>(blocks), block_format)) {
            return -4;
        }
        return static_cast<int64_t>(blocks * 16);
    }
    const uint64_t pixels = static_cast<uint64_t>(info.m_orig_width) * info.m_orig_height;
    if (capacity < pixels * 4) {
        return -3;
    }
    if (!transcoder.transcode_image_level(level, 0, 0, out, static_cast<uint32_t>(pixels), basist::transcoder_texture_format::cTFRGBA32)) {
        return -4;
    }
    return static_cast<int64_t>(pixels * 4);
}

}
