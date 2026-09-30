// Bloom chain + cinematic composite (tonemap, grading, vignette, grain).

struct PostParams {
    a: vec4<f32>,     // exposure, bloom strength, vignette, grain
    b: vec4<f32>,     // time, saturation, contrast, threshold
    tint: vec4<f32>,  // grading multiply (rgb), w: chromatic aberration
    lift: vec4<f32>,  // shadow tint (rgb), w: fade to black
    c: vec4<f32>,     // x: letterbox (0..1), y: desaturate (0..1), z: flash, w: unused
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
@group(0) @binding(2) var<uniform> pp: PostParams;
@group(0) @binding(3) var extra: texture_2d<f32>;

struct FsIn {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) vi: u32) -> FsIn {
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    var o: FsIn;
    o.pos = vec4<f32>(x, y, 0.0, 1.0);
    o.uv = vec2<f32>(x * 0.5 + 0.5, 0.5 - y * 0.5);
    return o;
}

fn s(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(src, smp, uv, 0.0).rgb;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn down13(uv: vec2<f32>) -> vec3<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    let a = s(uv + t * vec2<f32>(-2.0, -2.0));
    let b = s(uv + t * vec2<f32>(0.0, -2.0));
    let c = s(uv + t * vec2<f32>(2.0, -2.0));
    let d = s(uv + t * vec2<f32>(-1.0, -1.0));
    let e = s(uv + t * vec2<f32>(1.0, -1.0));
    let f = s(uv + t * vec2<f32>(-2.0, 0.0));
    let g = s(uv);
    let h = s(uv + t * vec2<f32>(2.0, 0.0));
    let i = s(uv + t * vec2<f32>(-1.0, 1.0));
    let j = s(uv + t * vec2<f32>(1.0, 1.0));
    let k = s(uv + t * vec2<f32>(-2.0, 2.0));
    let l = s(uv + t * vec2<f32>(0.0, 2.0));
    let m = s(uv + t * vec2<f32>(2.0, 2.0));
    var r = (d + e + i + j) * 0.125;
    r = r + (a + b + f + g) * 0.03125;
    r = r + (b + c + g + h) * 0.03125;
    r = r + (f + g + k + l) * 0.03125;
    r = r + (g + h + l + m) * 0.03125;
    return r;
}

@fragment
fn fs_down_first(i: FsIn) -> @location(0) vec4<f32> {
    var c = down13(i.uv);
    // soft threshold with knee, clamp fireflies
    let th = pp.b.w;
    let br = max(max(c.r, c.g), c.b);
    let knee = th * 0.5;
    var soft = clamp(br - th + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 1e-4);
    let contrib = max(soft, br - th) / max(br, 1e-4);
    c = c * contrib;
    c = min(c, vec3<f32>(40.0));
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_down(i: FsIn) -> @location(0) vec4<f32> {
    return vec4<f32>(down13(i.uv), 1.0);
}

@fragment
fn fs_up(i: FsIn) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    var r = s(i.uv) * 4.0;
    r = r + (s(i.uv + vec2<f32>(t.x, 0.0)) + s(i.uv - vec2<f32>(t.x, 0.0)) + s(i.uv + vec2<f32>(0.0, t.y)) + s(i.uv - vec2<f32>(0.0, t.y))) * 2.0;
    r = r + s(i.uv + t) + s(i.uv - t) + s(i.uv + vec2<f32>(t.x, -t.y)) + s(i.uv + vec2<f32>(-t.x, t.y));
    return vec4<f32>(r / 16.0, 1.0);
}

// ACES fitted (Stephen Hill)
fn rrt_odt(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn aces(color: vec3<f32>) -> vec3<f32> {
    let m1 = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777));
    let m2 = mat3x3<f32>(
        vec3<f32>(1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108, 1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605, 1.07602));
    var v = m1 * color;
    v = rrt_odt(v);
    return clamp(m2 * v, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

@fragment
fn fs_composite(i: FsIn) -> @location(0) vec4<f32> {
    let uv = i.uv;
    let center = uv - 0.5;
    let ca = pp.tint.w;
    var hdr: vec3<f32>;
    if (ca > 0.0) {
        let off = center * ca * 0.006;
        hdr = vec3<f32>(
            textureSampleLevel(src, smp, uv - off, 0.0).r,
            textureSampleLevel(src, smp, uv, 0.0).g,
            textureSampleLevel(src, smp, uv + off, 0.0).b);
    } else {
        hdr = textureSampleLevel(src, smp, uv, 0.0).rgb;
    }
    let bloom = textureSampleLevel(extra, smp, uv, 0.0).rgb;
    var c = hdr + bloom * pp.a.y;
    c = c * pp.a.x;
    // grading in linear space
    let l0 = luma(c);
    c = c * pp.tint.rgb + pp.lift.rgb * (1.0 - smoothstep(0.0, 0.6, l0)) * 0.05;
    c = aces(c);
    // saturation / contrast in display-ish space
    let l = luma(c);
    let sat = pp.b.y * (1.0 - pp.c.y);
    c = mix(vec3<f32>(l), c, sat);
    c = clamp((c - 0.5) * pp.b.z + 0.5, vec3<f32>(0.0), vec3<f32>(1.0));
    // vignette
    let vig = 1.0 - pp.a.z * smoothstep(0.35, 0.95, length(center * vec2<f32>(1.1, 1.0)) * 1.25);
    c = c * vig;
    // flash
    c = mix(c, vec3<f32>(1.0), pp.c.z);
    // fade
    c = c * (1.0 - pp.lift.w);
    var out = to_srgb(c);
    // film grain
    let gr = hash(i.pos.xy + vec2<f32>(pp.b.x * 60.0, pp.b.x * 37.0)) - 0.5;
    out = out + gr * pp.a.w;
    // letterbox
    let lb = pp.c.x * 0.11;
    if (uv.y < lb || uv.y > 1.0 - lb) {
        out = vec3<f32>(0.0);
    }
    return vec4<f32>(out, 1.0);
}
