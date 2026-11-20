const ETC2_RGBA8 = 0x9278;
const DXT1 = 0x83f0;
const canvas = document.createElement('canvas');
canvas.width = 4;
canvas.height = 4;
const gl = canvas.getContext('webgl2');
if (!gl) {
    return { webgl2: false };
}
const formats = Array.from(gl.getParameter(gl.COMPRESSED_TEXTURE_FORMATS));
gl.getExtension('WEBGL_compressed_texture_s3tc');
const vertexSource = [
    '#version 300 es',
    'void main() {',
    '    vec2 corner = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));',
    '    gl_Position = vec4(corner * 2.0 - 1.0, 0.0, 1.0);',
    '}',
].join('\n');
const fragmentSource = [
    '#version 300 es',
    'precision highp float;',
    'uniform highp sampler2D tex;',
    'out vec4 color;',
    'void main() {',
    '    color = texelFetch(tex, ivec2(gl_FragCoord.xy), 0);',
    '}',
].join('\n');
function compile(type, source) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
        throw new Error(gl.getShaderInfoLog(shader));
    }
    return shader;
}
const program = gl.createProgram();
gl.attachShader(program, compile(gl.VERTEX_SHADER, vertexSource));
gl.attachShader(program, compile(gl.FRAGMENT_SHADER, fragmentSource));
gl.linkProgram(program);
if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
}
gl.useProgram(program);
gl.uniform1i(gl.getUniformLocation(program, 'tex'), 0);
function readback(format, width, height, bytes) {
    canvas.width = width;
    canvas.height = height;
    gl.viewport(0, 0, width, height);
    const texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.compressedTexImage2D(gl.TEXTURE_2D, 0, format, width, height, 0, bytes);
    const error = gl.getError();
    if (error) {
        gl.deleteTexture(texture);
        return { error: '0x' + error.toString(16) };
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    const pixels = new Uint8Array(width * height * 4);
    gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    gl.deleteTexture(texture);
    return { pixels: Array.from(pixels) };
}
function matches(result, expected) {
    if (!result.pixels) {
        return false;
    }
    for (let i = 0; i < result.pixels.length; i += 4) {
        for (let c = 0; c < 4; c += 1) {
            if (result.pixels[i + c] !== expected[c]) {
                return false;
            }
        }
    }
    return true;
}
const fixtures = [
    { name: 'BC1 flat red (c0=0xF800, index 0)', format: DXT1, bytes: [0x00, 0xf8, 0x00, 0xf8, 0, 0, 0, 0], expected: [255, 0, 0, 255] },
    { name: 'BC1 flat green (c0=0x07E0, index 0)', format: DXT1, bytes: [0xe0, 0x07, 0xe0, 0x07, 0, 0, 0, 0], expected: [0, 255, 0, 255] },
];
const fixtureResults = fixtures.map((fixture) => {
    const result = readback(fixture.format, 4, 4, new Uint8Array(fixture.bytes));
    return { name: fixture.name, ok: matches(result, fixture.expected), first: result.pixels ? result.pixels.slice(0, 4) : result.error };
});
const etc2Bytes = [0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];
const etc2 = formats.includes(ETC2_RGBA8)
    ? Object.assign({ name: 'ETC2 RGBA8 flat (index 01 = +8)' }, (() => { const result = readback(ETC2_RGBA8, 4, 4, new Uint8Array(etc2Bytes)); return { ok: matches(result, [8, 8, 8, 255]), first: result.pixels ? result.pixels.slice(0, 4) : result.error }; })())
    : { name: 'ETC2 RGBA8 flat (index 01 = +8)', skipped: 'ETC2 RGBA8 is not in COMPRESSED_TEXTURE_FORMATS' };
return {
    webgl2: true,
    nativeCompressedFormats: formats.length,
    etc2Rgba8Listed: formats.includes(ETC2_RGBA8),
    fixtures: fixtureResults,
    etc2: etc2,
    pass: fixtureResults.every((r) => r.ok) && (etc2.skipped !== undefined || etc2.ok === true),
};
