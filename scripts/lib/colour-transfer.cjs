const SRGB_LINEAR_LIMIT = 0.04045;
const SRGB_ENCODED_LIMIT = 0.0031308;
const SRGB_EXPONENT = 2.4;

function srgbDecode(value) {
    return value <= SRGB_LINEAR_LIMIT ? value / 12.92 : ((value + 0.055) / 1.055) ** SRGB_EXPONENT;
}

function srgbEncode(value) {
    return value <= SRGB_ENCODED_LIMIT ? 12.92 * value : 1.055 * value ** (1 / SRGB_EXPONENT) - 0.055;
}

function clampByte(value) {
    return value < 0 ? 0 : value > 255 ? 255 : value;
}

function srgbDecodeByte(byte) {
    return clampByte(Math.round(srgbDecode(byte / 255) * 255));
}

function srgbEncodeByte(byte) {
    return clampByte(Math.round(srgbEncode(byte / 255) * 255));
}

module.exports = { srgbDecode, srgbEncode, srgbDecodeByte, srgbEncodeByte };
