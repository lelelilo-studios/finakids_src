// UI shader: SDF rounded rectangles, glyphs, frosted glass and progress rings.

struct UiGlobals {
    screen: vec4<f32>, // width, height, time, exposure
};

@group(0) @binding(0) var<uniform> u: UiGlobals;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var glass: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;

struct VIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) local: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) color: vec4<f32>,
    @location(5) params: vec4<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) screen_uv: vec2<f32>,
};

@vertex
fn vs_ui(v: VIn) -> VOut {
    var o: VOut;
    let ndc = vec2<f32>(v.pos.x / u.screen.x * 2.0 - 1.0, 1.0 - v.pos.y / u.screen.y * 2.0);
    o.clip = vec4<f32>(ndc, 0.0, 1.0);
    o.uv = v.uv;
    o.local = v.local;
    o.size = v.size;
    o.color = v.color;
    o.params = v.params;
    o.screen_uv = v.pos / u.screen.xy;
    return o;
}

fn sd_rbox(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn rrt_odt(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn tonemap(c: vec3<f32>) -> vec3<f32> {
    let m1 = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777));
    let m2 = mat3x3<f32>(
        vec3<f32>(1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108, 1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605, 1.07602));
    let v = clamp(m2 * rrt_odt(m1 * c), vec3<f32>(0.0), vec3<f32>(1.0));
    return pow(v, vec3<f32>(1.0 / 2.2));
}

@fragment
fn fs_ui(i: VOut) -> @location(0) vec4<f32> {
    let mode = i32(i.params.w + 0.5);
    let radius = i.params.x;
    let border = i.params.y;
    let soft = max(i.params.z, 1.0);
    // always sample (uniform control flow requirements)
    let a = textureSampleLevel(atlas, smp, i.uv, 0.0).r;
    let gbg = textureSampleLevel(glass, smp, i.screen_uv, 0.0).rgb;
    if (mode == 1) {
        return vec4<f32>(i.color.rgb, i.color.a * a);
    }
    let half = i.size * 0.5;
    let d = sd_rbox(i.local, half, min(radius, min(half.x, half.y)));
    var cov = clamp(0.5 - d / soft, 0.0, 1.0);
    if (i.params.z > 1.5) {
        cov = smoothstep(soft, -soft, d);
    }
    if (border > 0.0) {
        cov = cov * clamp(0.5 + (d + border), 0.0, 1.0);
    }
    if (mode == 2) {
        // frosted glass: blurred scene behind + tint
        let bg = tonemap(gbg * u.screen.w) * 0.8;
        let c = mix(bg, i.color.rgb, i.color.a);
        return vec4<f32>(c, cov);
    }
    if (mode == 3) {
        // ring progress: uv.x = progress, radius = thickness in px
        let r = length(i.local);
        let outer = half.x;
        let ring = clamp(0.5 - (abs(r - (outer - radius * 0.5)) - radius * 0.5), 0.0, 1.0);
        var ang = atan2(i.local.x, -i.local.y) / 6.2831853;
        if (ang < 0.0) {
            ang = ang + 1.0;
        }
        let on = step(ang, i.uv.x);
        let base = vec4<f32>(1.0, 1.0, 1.0, 0.16);
        let c = mix(base, i.color, on);
        return vec4<f32>(c.rgb, c.a * ring);
    }
    if (mode == 4) {
        // circle
        let r = length(i.local);
        let c = clamp(0.5 - (r - half.x) / soft, 0.0, 1.0);
        return vec4<f32>(i.color.rgb, i.color.a * c);
    }
    return vec4<f32>(i.color.rgb, i.color.a * cov);
}
