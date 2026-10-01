// Finakids scene shader: PBR forward shading with procedural surface patterns.

struct PointLight {
    pos: vec4<f32>,   // xyz, radius
    color: vec4<f32>, // rgb * intensity, w: unused
};

struct Globals {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    shadow_mat: mat4x4<f32>,
    cam_pos: vec4<f32>,     // w: time
    sun_dir: vec4<f32>,     // xyz toward sun, w: shadow strength
    sun_color: vec4<f32>,   // rgb, w: sun disc visibility
    sky_up: vec4<f32>,      // ambient from above, w: ambient scale
    sky_down: vec4<f32>,    // ambient from below, w: env specular scale
    fog: vec4<f32>,         // rgb, density
    sky_zenith: vec4<f32>,  // w: cloud cover
    sky_horizon: vec4<f32>, // w: stars
    params: vec4<f32>,      // x: light count, y: shadow texel, z: rim strength, w: night factor
    rim: vec4<f32>,         // rim color, w: brightness of the surroundings of a cut-away room
    room: vec4<f32>,        // interior bounds: min x, min z, max x, max z
    lights: array<PointLight, 8>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var shadow_tex: texture_depth_2d;
@group(0) @binding(2) var shadow_smp: sampler_comparison;

@group(1) @binding(0) var<uniform> bones: array<mat4x4<f32>, 40>;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mat: vec4<f32>,
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(8) tint: vec4<f32>,
    @location(9) iparams: vec4<f32>,
};

struct SkinIn {
    @location(0) pos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mat: vec4<f32>,
    @location(4) m0: vec4<f32>,
    @location(5) m1: vec4<f32>,
    @location(6) m2: vec4<f32>,
    @location(7) m3: vec4<f32>,
    @location(8) tint: vec4<f32>,
    @location(9) iparams: vec4<f32>,
    @location(10) joints: vec4<u32>,
    @location(11) weights: vec4<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) wpos: vec3<f32>,
    @location(1) nrm: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) mat: vec4<f32>,
    @location(4) lpos: vec3<f32>,
    @location(5) tint: vec4<f32>,
    @location(6) iparams: vec4<f32>,
};

fn cofactor(m: mat4x4<f32>) -> mat3x3<f32> {
    let a = m[0].xyz;
    let b = m[1].xyz;
    let c = m[2].xyz;
    return mat3x3<f32>(cross(b, c), cross(c, a), cross(a, b));
}

fn finish_vertex(model: mat4x4<f32>, lp: vec3<f32>, ln: vec3<f32>, color: vec4<f32>, mat: vec4<f32>,
                 tint: vec4<f32>, ip: vec4<f32>, local_for_pattern: vec3<f32>) -> VOut {
    var o: VOut;
    let wp = model * vec4<f32>(lp, 1.0);
    o.wpos = wp.xyz;
    o.clip = g.view_proj * wp;
    var n = cofactor(model) * ln;
    // guard mirrored transforms
    if (determinant(mat3x3<f32>(model[0].xyz, model[1].xyz, model[2].xyz)) < 0.0) {
        n = -n;
    }
    o.nrm = normalize(n);
    o.color = color;
    o.mat = mat;
    o.lpos = local_for_pattern;
    o.tint = tint;
    o.iparams = ip;
    return o;
}

@vertex
fn vs_static(v: VIn) -> VOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return finish_vertex(model, v.pos, v.nrm, v.color, v.mat, v.tint, v.iparams, v.pos);
}

fn skin_matrix(j: vec4<u32>, w: vec4<f32>) -> mat4x4<f32> {
    return bones[j.x] * w.x + bones[j.y] * w.y + bones[j.z] * w.z + bones[j.w] * w.w;
}

@vertex
fn vs_skin(v: SkinIn) -> VOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let s = skin_matrix(v.joints, v.weights);
    let lp = (s * vec4<f32>(v.pos, 1.0)).xyz;
    let ln = (s * vec4<f32>(v.nrm, 0.0)).xyz;
    return finish_vertex(model, lp, ln, v.color, v.mat, v.tint, v.iparams, v.pos);
}

// ---------------------------------------------------------------- shadow pass

struct ShadowOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_shadow_static(v: VIn) -> ShadowOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    var o: ShadowOut;
    o.clip = g.shadow_mat * (model * vec4<f32>(v.pos, 1.0));
    return o;
}

@vertex
fn vs_shadow_skin(v: SkinIn) -> ShadowOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let s = skin_matrix(v.joints, v.weights);
    var o: ShadowOut;
    o.clip = g.shadow_mat * (model * (s * vec4<f32>(v.pos, 1.0)));
    return o;
}

// ---------------------------------------------------------------- noise

fn hash13(p3i: vec3<f32>) -> f32 {
    var p3 = fract(p3i * 0.1031);
    p3 = p3 + dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash13(i);
    let b = hash13(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash13(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash13(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash13(i + vec3<f32>(0.0, 0.0, 1.0));
    let f1 = hash13(i + vec3<f32>(1.0, 0.0, 1.0));
    let g1 = hash13(i + vec3<f32>(0.0, 1.0, 1.0));
    let h = hash13(i + vec3<f32>(1.0, 1.0, 1.0));
    return mix(mix(mix(a, b, u.x), mix(c, d, u.x), u.y), mix(mix(e, f1, u.x), mix(g1, h, u.x), u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 4; i = i + 1) {
        s = s + a * vnoise(q);
        q = q * 2.07 + vec3<f32>(1.7, 9.2, 3.3);
        a = a * 0.5;
    }
    return s;
}

fn vnoise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash12(i), hash12(i + vec2<f32>(1.0, 0.0)), u.x),
               mix(hash12(i + vec2<f32>(0.0, 1.0)), hash12(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn fbm2(p: vec2<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 5; i = i + 1) {
        s = s + a * vnoise2(q);
        q = q * 2.03 + vec2<f32>(3.1, 1.7);
        a = a * 0.5;
    }
    return s;
}

fn srgb_to_lin(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// ---------------------------------------------------------------- surface

struct Surface {
    albedo: vec3<f32>,
    rough: f32,
    metal: f32,
    emissive: vec3<f32>,
    n: vec3<f32>,
    ao: f32,
    sss: f32,     // subsurface / wrap amount
    sheen: f32,
    hair: f32,
    translucent: f32,
    alpha: f32,
};

fn box_axis_uv(p: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let an = abs(n);
    if (an.y > an.x && an.y > an.z) {
        return p.xz;
    }
    if (an.x > an.z) {
        return vec2<f32>(p.z, p.y);
    }
    return vec2<f32>(p.x, p.y);
}

fn apply_pattern(s: ptr<function, Surface>, kind: u32, lp: vec3<f32>, wp: vec3<f32>, ip: vec4<f32>, t: f32) {
    let n = (*s).n;
    switch kind {
        case 1u: { // skin
            let v = fbm(lp * 55.0);
            (*s).albedo = (*s).albedo * (0.94 + 0.12 * v);
            (*s).rough = clamp((*s).rough + (v - 0.5) * 0.12, 0.3, 0.8);
            (*s).sss = 1.0;
        }
        case 2u: { // fabric
            let w = sin(lp.x * 900.0) * sin(lp.y * 900.0 + lp.z * 700.0);
            let v = fbm(lp * 18.0);
            (*s).albedo = (*s).albedo * (0.9 + 0.08 * w + 0.12 * v);
            (*s).rough = 0.88;
            (*s).sheen = 0.6;
        }
        case 3u: { // denim
            let tw = sin((lp.x + lp.y + lp.z) * 1100.0);
            let fade = fbm(lp * vec3<f32>(6.0, 2.0, 6.0));
            (*s).albedo = (*s).albedo * (0.86 + 0.1 * tw) + vec3<f32>(0.06, 0.08, 0.1) * smoothstep(0.45, 0.75, fade);
            (*s).rough = 0.9;
            (*s).sheen = 0.35;
        }
        case 4u: { // hair
            let q = lp * vec3<f32>(170.0, 24.0, 170.0);
            let strands = vnoise(q) * 0.55 + vnoise(q * 2.7 + vec3<f32>(3.0)) * 0.45;
            let clump = vnoise(lp * vec3<f32>(40.0, 8.0, 40.0));
            (*s).albedo = (*s).albedo * (0.7 + 0.45 * strands) * (0.85 + 0.3 * clump);
            (*s).rough = 0.55;
            (*s).hair = 1.0;
        }
        case 5u: { // wood (grain along dominant axis)
            let uv = box_axis_uv(wp, n);
            let warp = fbm(vec3<f32>(uv.x * 2.0, uv.y * 14.0, 0.0)) * 3.0;
            let grain = sin((uv.y * 70.0 + warp * 6.0)) * 0.5 + 0.5;
            let fine = vnoise(vec3<f32>(uv.x * 3.0, uv.y * 180.0, 1.0));
            (*s).albedo = (*s).albedo * (0.8 + 0.18 * grain + 0.12 * fine);
            (*s).rough = clamp((*s).rough + (fine - 0.5) * 0.15, 0.2, 0.95);
        }
        case 6u: { // floor planks along x
            let pw = 0.19;
            let row = floor(wp.z / pw);
            let off = hash12(vec2<f32>(row, 3.0)) * 1.3;
            let pl = 1.3;
            let col = floor((wp.x + off) / pl);
            let id = hash12(vec2<f32>(row, col));
            let fz = fract(wp.z / pw);
            let fx = fract((wp.x + off) / pl);
            let gap = smoothstep(0.0, 0.035, fz) * smoothstep(1.0, 0.965, fz) * smoothstep(0.0, 0.006, fx) * smoothstep(1.0, 0.994, fx);
            let warp = fbm(vec3<f32>(wp.x * 1.5, wp.z * 20.0, id * 10.0)) * 4.0;
            let grain = sin(wp.z * 160.0 + warp * 5.0 + id * 20.0) * 0.5 + 0.5;
            let tone = 0.78 + 0.34 * id;
            (*s).albedo = (*s).albedo * tone * (0.85 + 0.15 * grain) * mix(0.35, 1.0, gap);
            (*s).rough = mix(0.8, 0.28 + 0.12 * id, gap);
            (*s).ao = (*s).ao * mix(0.6, 1.0, gap);
        }
        case 7u: { // tiles
            let uv = box_axis_uv(wp, n) / 0.3;
            let f = fract(uv);
            let id = hash12(floor(uv));
            let grout = smoothstep(0.0, 0.03, f.x) * smoothstep(1.0, 0.97, f.x) * smoothstep(0.0, 0.03, f.y) * smoothstep(1.0, 0.97, f.y);
            (*s).albedo = mix(vec3<f32>(0.55, 0.53, 0.5), (*s).albedo * (0.93 + 0.1 * id), grout);
            (*s).rough = mix(0.9, 0.18, grout);
        }
        case 8u: { // plaster
            let v = fbm(wp * 6.0);
            let v2 = vnoise(wp * 80.0);
            (*s).albedo = (*s).albedo * (0.95 + 0.07 * v + 0.03 * v2);
            (*s).rough = 0.92;
        }
        case 9u: { // carpet / rug
            let r = length(lp.xz * vec2<f32>(1.0, 1.4));
            let bands = smoothstep(0.02, 0.0, abs(fract(r * 4.0) - 0.5) - 0.42);
            let fuzz = vnoise(lp * 400.0);
            (*s).albedo = (*s).albedo * (0.82 + 0.25 * fuzz) * (1.0 - 0.25 * bands);
            (*s).rough = 1.0;
            (*s).sheen = 0.5;
        }
        case 10u: { // eye: local +Z is the gaze direction, unit sphere
            let d = normalize(lp);
            let r = length(d.xy);
            let ang = atan2(d.y, d.x);
            let iris_r = 0.52;
            let pupil_r = 0.17 + 0.04 * ip.x;
            if (d.z > 0.0 && r < iris_r) {
                let fib = vnoise(vec3<f32>(ang * 14.0, r * 26.0, 0.0)) * 0.6 + vnoise(vec3<f32>(ang * 40.0, r * 8.0, 3.0)) * 0.4;
                var iris = (*s).albedo * (0.75 + 0.9 * fib) * mix(1.5, 0.75, r / iris_r);
                let limbal = smoothstep(iris_r * 0.78, iris_r, r);
                iris = iris * (1.0 - 0.7 * limbal);
                let pupil = smoothstep(pupil_r + 0.02, pupil_r - 0.01, r);
                (*s).albedo = mix(iris, vec3<f32>(0.01, 0.008, 0.008), pupil);
            } else {
                let edge = smoothstep(0.3, -0.5, d.z);
                (*s).albedo = mix(vec3<f32>(0.82, 0.8, 0.78), vec3<f32>(0.7, 0.52, 0.5), edge * 0.7);
            }
            (*s).rough = 0.05;
            (*s).sss = 0.2;
        }
        case 11u: { // computer screen content
            let uv = lp.xy * vec2<f32>(3.4, 5.0);
            let rows = step(0.5, fract(uv.y * 6.0));
            let bars = step(0.3, fract(uv.x * 2.3 + floor(uv.y * 6.0) * 0.37));
            let hdr = smoothstep(0.6, 0.62, fract(uv.y * 0.25 + 0.4));
            var c = mix(vec3<f32>(0.09, 0.13, 0.25), vec3<f32>(0.2, 0.55, 0.95), rows * bars * 0.6);
            c = mix(c, vec3<f32>(0.95, 0.55, 0.25), hdr * 0.5);
            (*s).emissive = c * (*s).emissive * 3.0;
            (*s).albedo = vec3<f32>(0.02);
            (*s).rough = 0.15;
        }
        case 12u: { // foliage
            let v = fbm(lp * 9.0 + wp * 0.5);
            let leaves = vnoise(wp * 18.0);
            (*s).albedo = (*s).albedo * (0.7 + 0.6 * v) * (0.8 + 0.4 * leaves);
            (*s).rough = 0.75;
            (*s).translucent = 1.0;
            (*s).sss = 0.6;
        }
        case 13u: { // asphalt
            let v = vnoise(wp * 60.0);
            let b = fbm(wp * 0.6);
            (*s).albedo = (*s).albedo * (0.8 + 0.3 * v) * (0.85 + 0.3 * b);
            (*s).rough = 0.85 - 0.2 * step(0.8, v);
        }
        case 14u: { // pavers
            let uv = wp.xz / vec2<f32>(0.4, 0.2);
            let rowo = floor(uv.y) * 0.5;
            let f = fract(vec2<f32>(uv.x + rowo, uv.y));
            let id = hash12(floor(vec2<f32>(uv.x + rowo, uv.y)));
            let m = smoothstep(0.0, 0.05, f.x) * smoothstep(1.0, 0.95, f.x) * smoothstep(0.0, 0.08, f.y) * smoothstep(1.0, 0.92, f.y);
            (*s).albedo = (*s).albedo * (0.85 + 0.25 * id) * mix(0.55, 1.0, m) * (0.9 + 0.2 * vnoise(wp * 20.0));
            (*s).rough = 0.85;
            (*s).ao = (*s).ao * mix(0.7, 1.0, m);
        }
        case 15u: { // brick
            let uv = box_axis_uv(wp, n) / vec2<f32>(0.24, 0.075);
            let rowo = floor(uv.y) * 0.5;
            let f = fract(vec2<f32>(uv.x + rowo, uv.y));
            let id = hash12(floor(vec2<f32>(uv.x + rowo, uv.y)));
            let m = smoothstep(0.0, 0.04, f.x) * smoothstep(1.0, 0.96, f.x) * smoothstep(0.0, 0.12, f.y) * smoothstep(1.0, 0.88, f.y);
            (*s).albedo = mix(vec3<f32>(0.62, 0.6, 0.56), (*s).albedo * (0.8 + 0.35 * id), m);
            (*s).rough = 0.9;
            (*s).ao = (*s).ao * mix(0.75, 1.0, m);
        }
        case 16u: { // grass
            let v = fbm(wp * 1.3);
            let fine = vnoise(wp * 45.0);
            (*s).albedo = (*s).albedo * (0.7 + 0.45 * v) * (0.85 + 0.3 * fine);
            (*s).rough = 0.95;
        }
        case 17u: { // brushed metal
            let st = vnoise(vec3<f32>(lp.x * 3.0, lp.y * 400.0, lp.z * 3.0));
            (*s).albedo = (*s).albedo * (0.9 + 0.15 * st);
            (*s).rough = clamp((*s).rough + (st - 0.5) * 0.1, 0.05, 1.0);
        }
        case 19u: { // poster art, seed in ip.x
            let uv = lp.xy;
            let seed = ip.x;
            let c1 = vec3<f32>(hash12(vec2<f32>(seed, 1.0)), hash12(vec2<f32>(seed, 2.0)), hash12(vec2<f32>(seed, 3.0)));
            let c2 = vec3<f32>(hash12(vec2<f32>(seed, 4.0)), hash12(vec2<f32>(seed, 5.0)), hash12(vec2<f32>(seed, 6.0)));
            let grad = mix(c1 * c1, c2 * vec3<f32>(1.0, 0.7, 0.9), clamp(uv.y * 2.0 + 0.5, 0.0, 1.0));
            let circ = smoothstep(0.012, 0.0, length(uv - vec2<f32>(0.03, 0.06)) - 0.12);
            let stripe = step(0.5, fract(uv.y * 14.0)) * step(uv.y, -0.12);
            var c = mix(grad, vec3<f32>(1.0, 0.85, 0.55), circ * 0.9);
            c = mix(c, c * 0.25, stripe * 0.8);
            (*s).albedo = c;
            (*s).rough = 0.5;
        }
        case 20u: { // books: stripes along local x, per-book colour
            let bw = 0.028;
            let id = floor(lp.x / bw + ip.x);
            let f = fract(lp.x / bw);
            let h = hash12(vec2<f32>(id, 7.0));
            var pal = array<vec3<f32>, 6>(
                vec3<f32>(0.55, 0.12, 0.1), vec3<f32>(0.1, 0.22, 0.45), vec3<f32>(0.85, 0.7, 0.35),
                vec3<f32>(0.15, 0.35, 0.25), vec3<f32>(0.85, 0.85, 0.8), vec3<f32>(0.3, 0.2, 0.35));
            let ci = u32(h * 5.99);
            let height_cut = 0.75 + 0.25 * hash12(vec2<f32>(id, 3.0));
            (*s).albedo = pal[ci] * (0.85 + 0.2 * hash12(vec2<f32>(id, 9.0)));
            let band = smoothstep(0.02, 0.0, abs(fract(lp.y * 7.0 + h) - 0.5) - 0.44);
            (*s).albedo = mix((*s).albedo, vec3<f32>(0.9, 0.8, 0.5), band * 0.6);
            (*s).ao = (*s).ao * (0.55 + 0.45 * smoothstep(0.0, 0.12, f) * smoothstep(1.0, 0.88, f));
            (*s).rough = 0.7;
            if (lp.y > height_cut * 0.24 - 0.12) {
                (*s).albedo = (*s).albedo * 0.2;
            }
        }
        case 21u: { // plaid
            let a = step(0.55, fract(lp.x * 9.0));
            let b = step(0.55, fract(lp.z * 9.0));
            let th = sin(lp.x * 600.0) * sin(lp.z * 600.0);
            (*s).albedo = (*s).albedo * (0.72 + 0.18 * a + 0.18 * b + 0.04 * th);
            (*s).rough = 0.9;
            (*s).sheen = 0.5;
        }
        case 22u: { // lamp shade (lit from within: ip.y)
            (*s).translucent = 0.9;
            (*s).emissive = (*s).albedo * ip.y * 2.2;
            (*s).rough = 0.9;
        }
        case 23u: { // leather
            let v = vnoise(lp * 300.0);
            (*s).albedo = (*s).albedo * (0.9 + 0.12 * v);
            (*s).rough = 0.45 + 0.15 * v;
        }
        case 26u: { // phone screen
            let uv = lp.xy / vec2<f32>(0.034, 0.07);
            let top = smoothstep(0.55, 0.57, uv.y);
            let card = step(abs(uv.x), 0.8) * step(abs(uv.y - 0.1), 0.25);
            var c = mix(vec3<f32>(0.05, 0.07, 0.12), vec3<f32>(0.25, 0.75, 0.55), card * 0.7);
            c = mix(c, vec3<f32>(0.9, 0.9, 1.0), top * 0.2);
            // cracks when ip.z > 0
            if (ip.z > 0.0) {
                let cp = uv - vec2<f32>(0.4, -0.3);
                let a = atan2(cp.y, cp.x);
                let crack = smoothstep(0.08, 0.0, abs(fract(a * 1.6 + vnoise(vec3<f32>(uv * 6.0, 1.0)) * 0.8) - 0.5) - 0.46);
                c = mix(c, vec3<f32>(0.9), crack * ip.z * 0.8);
            }
            (*s).emissive = c * (*s).emissive * 4.0;
            (*s).albedo = vec3<f32>(0.01);
            (*s).rough = 0.08;
        }
        case 27u: { // stripes
            let st = step(0.5, fract(lp.y * 16.0));
            (*s).albedo = mix((*s).albedo, vec3<f32>(0.92, 0.92, 0.9), st * 0.85);
            (*s).rough = 0.85;
            (*s).sheen = 0.5;
        }
        case 28u: { // concrete
            let v = fbm(wp * 2.0);
            let v2 = vnoise(wp * 50.0);
            (*s).albedo = (*s).albedo * (0.82 + 0.25 * v + 0.08 * v2);
            (*s).rough = 0.9;
        }
        case 29u: { // sign / neon (emissive modulated by ip.y flicker)
            (*s).emissive = (*s).albedo * (*s).emissive * 6.0 * (0.9 + 0.1 * sin(t * 3.0 + ip.x));
        }
        case 30u: { // water
            let w1 = sin(wp.x * 6.0 + t * 1.3) * sin(wp.z * 5.0 - t * 1.1);
            let w2 = sin((wp.x + wp.z) * 11.0 - t * 2.0);
            (*s).n = normalize(n + vec3<f32>(w1 * 0.08 + w2 * 0.03, 0.0, w2 * 0.05));
            (*s).albedo = vec3<f32>(0.02, 0.05, 0.06);
            (*s).rough = 0.04;
        }
        case 31u: { // knit
            let k = abs(sin(lp.x * 520.0 + abs(sin(lp.y * 260.0)) * 2.0));
            (*s).albedo = (*s).albedo * (0.8 + 0.22 * k);
            (*s).rough = 0.95;
            (*s).sheen = 0.7;
        }
        default: {}
    }
}

// ---------------------------------------------------------------- lighting

fn shadow_factor(wp: vec3<f32>, n: vec3<f32>) -> f32 {
    if (g.sun_dir.w <= 0.0) {
        return 1.0;
    }
    let texel = g.params.y;
    let biased = wp + n * 0.02 + g.sun_dir.xyz * 0.01;
    let sp = g.shadow_mat * vec4<f32>(biased, 1.0);
    let ndc = sp.xyz / sp.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5);
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0) {
        return 1.0;
    }
    let z = ndc.z - 0.0015;
    var sum = 0.0;
    var offs = array<vec2<f32>, 8>(
        vec2<f32>(-0.94, -0.4), vec2<f32>(0.94, 0.46), vec2<f32>(-0.09, -0.93), vec2<f32>(0.34, 0.29),
        vec2<f32>(-0.5, 0.7), vec2<f32>(0.62, -0.72), vec2<f32>(-0.26, 0.05), vec2<f32>(0.1, 0.95));
    // phones take 4 taps (g.params.w); each tap is already a 2x2 hardware PCF
    let taps = i32(g.params.w);
    for (var i = 0; i < 8; i = i + 1) {
        if (i >= taps) {
            break;
        }
        sum = sum + textureSampleCompareLevel(shadow_tex, shadow_smp, uv + offs[i] * texel * 1.6, z);
    }
    let s = sum / f32(taps);
    return mix(1.0, s, g.sun_dir.w);
}

fn d_ggx(nh: f32, a: f32) -> f32 {
    let a2 = a * a;
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * d * d + 1e-6);
}

fn v_smith(nv: f32, nl: f32, a: f32) -> f32 {
    let k = a * 0.5;
    let gv = nv / (nv * (1.0 - k) + k);
    let gl = nl / (nl * (1.0 - k) + k);
    return gv * gl / max(4.0 * nv * nl, 1e-4);
}

fn fresnel(f0: vec3<f32>, vh: f32) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - vh, 5.0);
}

fn light_contrib(s: Surface, v: vec3<f32>, l: vec3<f32>, radiance: vec3<f32>) -> vec3<f32> {
    let n = s.n;
    let nl_raw = dot(n, l);
    let wrap = s.sss * 0.45;
    let nl = max((nl_raw + wrap) / (1.0 + wrap), 0.0);
    let nl_spec = max(nl_raw, 0.0);
    let h = normalize(v + l);
    let nv = max(dot(n, v), 1e-3);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(s.rough * s.rough, 0.002);
    let f0 = mix(vec3<f32>(0.04), s.albedo, s.metal);
    let f = fresnel(f0, vh);
    let spec = d_ggx(nh, a) * v_smith(nv, max(nl_spec, 1e-3), a) * f * nl_spec;
    var diff = s.albedo * (1.0 - s.metal) * (vec3<f32>(1.0) - f) / 3.14159265 * nl;
    // subsurface: warm the terminator
    if (s.sss > 0.0) {
        let term = smoothstep(-0.3, 0.3, nl_raw) * (1.0 - smoothstep(0.1, 0.8, nl_raw));
        diff = diff + s.albedo * vec3<f32>(0.55, 0.12, 0.06) * term * 0.25 * s.sss;
    }
    // translucency (leaves, lamp shades)
    let back = max(-nl_raw, 0.0) * s.translucent;
    diff = diff + s.albedo * back * 0.35;
    // sheen (cloth)
    let sheen = s.sheen * pow(1.0 - nv, 4.0) * nl * 0.25;
    var hair = vec3<f32>(0.0);
    if (s.hair > 0.0) {
        let up = vec3<f32>(0.0, 1.0, 0.0);
        let tng = normalize(up - n * dot(n, up) + vec3<f32>(1e-4, 0.0, 0.0));
        let th = dot(tng, h);
        let sin_th = sqrt(max(1.0 - th * th, 0.0));
        let th2 = dot(tng, normalize(h + n * 0.2));
        let sin2 = sqrt(max(1.0 - th2 * th2, 0.0));
        hair = (vec3<f32>(0.06) * pow(sin_th, 180.0) + s.albedo * 0.25 * pow(sin2, 40.0)) * nl;
    }
    return (diff + spec * (1.0 - s.hair * 0.7) + vec3<f32>(sheen) * s.albedo + hair) * radiance;
}

fn shade(s: Surface, wp: vec3<f32>, is_char: bool) -> vec3<f32> {
    let v = normalize(g.cam_pos.xyz - wp);
    let n = s.n;
    var col = vec3<f32>(0.0);
    // sun / moon
    let sun = g.sun_dir.xyz;
    let sh = shadow_factor(wp, n);
    col = col + light_contrib(s, v, sun, g.sun_color.rgb * sh);
    // point lights
    let count = i32(g.params.x);
    for (var i = 0; i < 8; i = i + 1) {
        if (i >= count) {
            break;
        }
        let lpv = g.lights[i].pos;
        let d = lpv.xyz - wp;
        let dist = length(d);
        if (dist < lpv.w) {
            let l = d / max(dist, 1e-4);
            let x = dist / lpv.w;
            let win = clamp(1.0 - x * x * x * x, 0.0, 1.0);
            let att = win * win / (dist * dist + 0.25);
            col = col + light_contrib(s, v, l, g.lights[i].color.rgb * att);
        }
    }
    // ambient hemisphere
    let hemi = mix(g.sky_down.rgb, g.sky_up.rgb, n.y * 0.5 + 0.5) * g.sky_up.w;
    let nv = max(dot(n, v), 0.0);
    let f0 = mix(vec3<f32>(0.04), s.albedo, s.metal);
    let fr = f0 + (max(vec3<f32>(1.0 - s.rough), f0) - f0) * pow(1.0 - nv, 5.0);
    col = col + hemi * s.albedo * (1.0 - s.metal) * s.ao * (1.0 - fr * 0.5);
    let r = reflect(-v, n);
    let env = mix(g.sky_down.rgb, g.sky_up.rgb, clamp(r.y * 0.5 + 0.5, 0.0, 1.0)) * g.sky_down.w;
    let gloss = (1.0 - s.rough) * (1.0 - s.rough);
    col = col + env * fr * gloss * s.ao * (1.0 - s.hair * 0.5);
    // cinematic rim light on characters
    if (is_char) {
        let rim = pow(1.0 - nv, 3.0) * g.params.z;
        col = col + g.rim.rgb * rim * (0.4 + 0.6 * s.ao);
    }
    col = col + s.emissive;
    return col;
}

// Interiors are cut-away rooms: whatever lies beside or in front of the room is
// dimmed like the dark part of a stage. What is behind the back wall (the view
// through the windows) keeps its daylight.
fn stage_dim(c: vec3<f32>, wp: vec3<f32>) -> vec3<f32> {
    if (g.rim.w >= 1.0) {
        return c;
    }
    let side = max(max(g.room.x - wp.x, wp.x - g.room.z), 0.0);
    let front = max(wp.z - g.room.w, 0.0);
    let behind = smoothstep(0.0, 2.5, g.room.y - wp.z);
    let k = smoothstep(0.15, 3.0, max(side, front)) * (1.0 - behind);
    let grey = vec3<f32>(dot(c, vec3<f32>(0.3, 0.55, 0.15)));
    return mix(c, mix(grey, c, 0.45) * g.rim.w, k);
}

fn apply_fog(c0: vec3<f32>, wp: vec3<f32>) -> vec3<f32> {
    let c = stage_dim(c0, wp);
    if (g.fog.w <= 0.0) {
        return c;
    }
    let d = distance(wp, g.cam_pos.xyz);
    let f = 1.0 - exp(-d * g.fog.w);
    let vdir = normalize(wp - g.cam_pos.xyz);
    let sunamt = pow(max(dot(vdir, g.sun_dir.xyz), 0.0), 6.0);
    let fc = g.fog.rgb + g.sun_color.rgb * sunamt * 0.08;
    return mix(c, fc, clamp(f, 0.0, 1.0));
}

fn build_surface(i: VOut, front: bool) -> Surface {
    var s: Surface;
    s.albedo = srgb_to_lin(i.color.rgb) * i.tint.rgb;
    s.ao = i.color.a;
    s.rough = max(i.mat.x, 0.04);
    s.metal = i.mat.y;
    s.emissive = vec3<f32>(i.mat.z) * srgb_to_lin(i.color.rgb) * 8.0 * i.tint.rgb;
    var n = normalize(i.nrm);
    if (!front) {
        n = -n;
    }
    s.n = n;
    s.sss = 0.0;
    s.sheen = 0.0;
    s.hair = 0.0;
    s.translucent = 0.0;
    s.alpha = i.tint.a;
    let kind = u32(i.mat.w * 255.0 + 0.5);
    if (kind == 11u || kind == 26u || kind == 29u) {
        s.emissive = vec3<f32>(i.mat.z) * i.tint.rgb;
    }
    apply_pattern(&s, kind, i.lpos, i.wpos, i.iparams, g.cam_pos.w);
    return s;
}

@fragment
fn fs_main(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let s = build_surface(i, front);
    var c = shade(s, i.wpos, i.iparams.w > 0.5);
    // interaction highlight (ip.y on non-shade kinds uses fresnel glow)
    let kind = u32(i.mat.w * 255.0 + 0.5);
    if (i.iparams.y > 0.0 && kind != 22u) {
        let v = normalize(g.cam_pos.xyz - i.wpos);
        let fr = pow(1.0 - max(dot(s.n, v), 0.0), 2.0);
        c = c + vec3<f32>(1.0, 0.72, 0.35) * fr * i.iparams.y * 1.2;
    }
    c = apply_fog(c, i.wpos);
    return vec4<f32>(c, 1.0);
}

@fragment
fn fs_transparent(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    var s = build_surface(i, front);
    let v = normalize(g.cam_pos.xyz - i.wpos);
    let nv = max(dot(s.n, v), 0.0);
    let kind = u32(i.mat.w * 255.0 + 0.5);
    var c = shade(s, i.wpos, false);
    var a = s.alpha;
    if (kind == 24u) {
        // soft contact shadow (local plane in [-0.5, 0.5])
        let r = length(i.lpos.xz) * 2.0;
        a = a * smoothstep(1.0, 0.1, r) * smoothstep(1.0, 0.55, r);
        return vec4<f32>(vec3<f32>(0.0), a);
    }
    if (kind == 18u) {
        // glass: mostly reflection, fresnel-weighted alpha
        let fr = 0.04 + 0.96 * pow(1.0 - nv, 5.0);
        let r = reflect(-v, s.n);
        let env = mix(g.sky_down.rgb, g.sky_up.rgb, clamp(r.y * 0.5 + 0.5, 0.0, 1.0)) * 0.6;
        c = env * fr + s.albedo * 0.05 + s.emissive;
        a = clamp(s.alpha + fr * 0.6, 0.0, 1.0);
    }
    c = apply_fog(c, i.wpos);
    return vec4<f32>(c * a, a);
}

@fragment
fn fs_additive(i: VOut) -> @location(0) vec4<f32> {
    let kind = u32(i.mat.w * 255.0 + 0.5);
    let base = srgb_to_lin(i.color.rgb) * i.tint.rgb;
    var a = i.tint.a;
    if (kind == 24u) {
        // soft sprite: local xy in [-0.5, 0.5]
        let r = length(i.lpos.xy) * 2.0;
        a = a * smoothstep(1.0, 0.0, r);
    } else if (kind == 25u) {
        // light beam: fade along local y (0 top -> 1 bottom) and toward edges
        let edge = 1.0 - abs(i.lpos.x) * 2.0;
        let along = clamp(i.lpos.y, 0.0, 1.0);
        let dust = 0.8 + 0.2 * vnoise(i.wpos * 3.0 + vec3<f32>(0.0, g.cam_pos.w * 0.05, 0.0));
        a = a * smoothstep(0.0, 0.5, edge) * (1.0 - along) * smoothstep(0.0, 0.15, along) * dust;
        let v = normalize(g.cam_pos.xyz - i.wpos);
        a = a * (0.35 + 0.65 * abs(dot(normalize(i.nrm), v)));
    }
    return vec4<f32>(base * a, 0.0);
}

// ---------------------------------------------------------------- sky

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) vi: u32) -> SkyOut {
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    var o: SkyOut;
    o.clip = vec4<f32>(x, y, 1.0, 1.0);
    o.ndc = vec2<f32>(x, y);
    return o;
}

@fragment
fn fs_sky(i: SkyOut) -> @location(0) vec4<f32> {
    let wp = g.inv_view_proj * vec4<f32>(i.ndc, 1.0, 1.0);
    let dir = normalize(wp.xyz / wp.w - g.cam_pos.xyz);
    let t = g.cam_pos.w;
    let sun = g.sun_dir.xyz;
    let up = max(dir.y, 0.0);
    var c = mix(g.sky_horizon.rgb, g.sky_zenith.rgb, pow(up, 0.55));
    // horizon haze below 0
    if (dir.y < 0.0) {
        c = mix(g.sky_horizon.rgb, g.fog.rgb * 0.8, clamp(-dir.y * 4.0, 0.0, 1.0));
    }
    let sd = max(dot(dir, sun), 0.0);
    // sun glow + disc
    c = c + g.sun_color.rgb * (pow(sd, 6.0) * 0.12 + pow(sd, 64.0) * 0.35) * g.sun_color.w;
    c = c + g.sun_color.rgb * smoothstep(0.9993, 0.9997, sd) * 12.0 * g.sun_color.w;
    // stars
    if (g.sky_horizon.w > 0.0 && dir.y > 0.0) {
        let q = dir * 180.0;
        let cell = floor(q);
        let h = hash13(cell);
        let fp = fract(q) - 0.5;
        let star = smoothstep(0.08, 0.0, length(fp)) * step(0.985, h);
        let tw = 0.6 + 0.4 * sin(t * 2.0 + h * 50.0);
        c = c + vec3<f32>(0.9, 0.95, 1.0) * star * tw * g.sky_horizon.w * 2.0 * smoothstep(0.0, 0.2, dir.y);
        // moon (opposite of the sun direction is not guaranteed, use sun dir as moon at night)
        c = c + vec3<f32>(0.8, 0.85, 1.0) * smoothstep(0.9990, 0.9994, sd) * 3.0 * g.sky_horizon.w;
    }
    // clouds on a plane
    if (dir.y > 0.01 && g.sky_zenith.w > 0.0) {
        let uv = dir.xz / (dir.y + 0.12) * 1.3 + vec2<f32>(t * 0.004, t * 0.002);
        let n = fbm2(uv * 1.4);
        let cover = g.sky_zenith.w;
        let dens = smoothstep(1.0 - cover, 1.0 - cover + 0.35, n);
        let lit = g.sun_color.rgb * 0.18 * (0.6 + 0.4 * pow(sd, 3.0)) + g.sky_zenith.rgb * 0.6 + g.sky_horizon.rgb * 0.3;
        let shade_c = mix(lit, lit * 0.55, smoothstep(0.4, 1.0, fbm2(uv * 2.8 + 3.0)));
        c = mix(c, shade_c, dens * smoothstep(0.01, 0.2, dir.y) * 0.9);
    }
    return vec4<f32>(c, 1.0);
}
