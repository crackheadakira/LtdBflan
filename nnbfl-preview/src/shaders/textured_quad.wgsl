struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv0:      vec2<f32>,
    @location(2) uv1:      vec2<f32>,
    @location(3) uv2:      vec2<f32>,
    @location(4) tint:     vec4<f32>,
    @location(5) tex_aspects: vec3<f32>,
    @location(6) quad_size: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv0:  vec2<f32>,
    @location(1) uv1:  vec2<f32>,
    @location(2) uv2:  vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) pos_mesh: vec2<f32>,
    @location(5) quad_size: vec2<f32>,
}

struct StandardMaterial {
    interpolate_width:  vec4<f32>,
    interpolate_offset: vec4<f32>,
    combine_mode:  u32,
    combine_mode2: u32,

    texture_count: u32,
    alpha_select:  u32,
    tex_gen_mode:  u32,
    visible:       u32,
    use_texture_only: u32,
    use_thresholding_alpha_interpolation: u32,

    // 0 = 0ff
    // 1 = tex0
    // 2 = tex1
    // 3 = tex2
    // 4 = post-combine
    // 5 = indirect raw vector offset
    // 6 = indirect displaced UV coordinates
    // 7 = indirect isolated sample output
    // 8 = composite layer alpha visualized as grayscale
    // 9 = final
    debug_stage: u32,
    is_plain: u32,

    indirect_mtx0: vec4<f32>,
    indirect_mtx1: vec4<f32>,
    proj_mtx0:     array<vec4<f32>, 2>,
    proj_mtx1:     array<vec4<f32>, 2>,
    proj_mtx2:     array<vec4<f32>, 2>,
}

//   stage_bits[6]
//
//     .x bits  0- 3  srcRgb0       (TevSource enum value, 4 bits)
//     .x bits  4- 7  srcRgb1
//     .x bits  8-11  srcRgb2
//     .x bits 12-15  opRgb0        (TevColorOp enum value, 4 bits)
//     .x bits 16-19  opRgb1
//     .x bits 20-23  opRgb2
//     .x bits 24-27  combineRgb    (DetailedCombinerStageMode, 4 bits)
//     .x bits 28-29  scaleRgb      (TevScale: 0=x1 1=x2 2=x4)
//     .x bit  30     savePrevRgb   (copy output→buffer before this stage)
//
//     .y bits  0- 3  srcAlpha0
//     .y bits  4- 7  srcAlpha1
//     .y bits  8-11  srcAlpha2
//     .y bits 12-15  opAlpha0      (TevAlphaOp enum value, 4 bits)
//     .y bits 16-19  opAlpha1
//     .y bits 20-23  opAlpha2
//     .y bits 24-27  combineAlpha
//     .y bits 28-29  scaleAlpha
//     .y bit  30     savePrevAlpha
//
//     .z bits  0- 3  konstRgb      (index into constantColors[], 4 bits)
//     .z bits  4- 7  konstAlpha    (index into constantColors[], 4 bits)
//
//     .w bits  0- 3  rgbSourceCount   (1-3)
//     .w bits  4- 7  alphaSourceCount (1-3)
//
struct DetailedCombinerMaterial {
    constant_colors: array<vec4<f32>, 7>,

    stage_count: u32,
    stage_bits: array<vec4<i32>, 6>,

    texture_count: u32,
}

@group(0) @binding(0) var<uniform> u_projection: mat4x4<f32>;

@group(1) @binding(0) var t_texture0: texture_2d<f32>;
@group(1) @binding(1) var s_sampler0: sampler;
@group(1) @binding(2) var t_texture1: texture_2d<f32>;
@group(1) @binding(3) var s_sampler1: sampler;
@group(1) @binding(4) var t_texture2: texture_2d<f32>;
@group(1) @binding(5) var s_sampler2: sampler;
@group(1) @binding(6) var<uniform> u_standard:  StandardMaterial;
@group(1) @binding(7) var<uniform> u_detailed:  DetailedCombinerMaterial;

fn scale_vertex_uv(uv: vec2<f32>, ratio: f32) -> vec2<f32> {
    var scaled = uv;

    if (ratio > 1.0) {
        scaled.y = uv.y * ratio;
    } else {
        scaled.x = uv.x * (1.0 / ratio); 
    }

    return scaled;
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = u_projection * vec4<f32>(in.position, 0.0, 1.0);
    out.tint = in.tint;
    out.pos_mesh = in.position;
    out.quad_size = in.quad_size;

    out.uv0 = scale_vertex_uv(in.uv0, in.tex_aspects.x);
    out.uv1 = scale_vertex_uv(in.uv1, in.tex_aspects.y);
    out.uv2 = scale_vertex_uv(in.uv2, in.tex_aspects.z);
    
    return out;
}

const PLAIN_BORDER_THICKNESS: f32 = 2.0;
const PLAIN_FILL_ALPHA_MULT: f32 = 0.15;

fn plain_preview_color(tint: vec4<f32>, uv: vec2<f32>, quad_size: vec2<f32>) -> vec4<f32> {
    let pixel_pos = uv * quad_size;

    let dist_to_left   = pixel_pos.x;
    let dist_to_right  = quad_size.x - pixel_pos.x;
    let dist_to_top    = pixel_pos.y;
    let dist_to_bottom = quad_size.y - pixel_pos.y;

    let min_dist = min(min(dist_to_left, dist_to_right), min(dist_to_top, dist_to_bottom));

    var out_color = tint;
    if (min_dist < PLAIN_BORDER_THICKNESS) {
        out_color.a = tint.a;
    } else {
        out_color.a = tint.a * PLAIN_FILL_ALPHA_MULT;
    }
    
    return out_color;
}

const TEV_MODE_REPLACE:             u32 = 0u;
const TEV_MODE_MODULATE:            u32 = 1u;
const TEV_MODE_ADD:                 u32 = 2u;
const TEV_MODE_ADD_SIGNED:          u32 = 3u;
const TEV_MODE_INTERPOLATE:         u32 = 4u;
const TEV_MODE_SUBTRACT:            u32 = 5u;
const TEV_MODE_ADD_MULTIPLICATE:    u32 = 6u;
const TEV_MODE_MULTIPLICATE_ADD:    u32 = 7u;
const TEV_MODE_OVERLAY:             u32 = 8u;
const TEV_MODE_LIGHTEN:             u32 = 9u;
const TEV_MODE_DARKEN:              u32 = 10u;
const TEV_MODE_INDIRECT:            u32 = 11u;
const TEV_MODE_BLEND_INDIRECT:      u32 = 12u;
const TEV_MODE_EACH_INDIRECT:       u32 = 13u;


fn combine_layer(
    base:             vec4<f32>,
    tex:              vec4<f32>,
    mode:             u32,
    select_alpha_max: bool,
) -> vec4<f32> {
    var out_rgb: vec3<f32>;
    let src_rgb = tex.rgb * tex.a;
    let inv_a   = 1.0 - tex.a;

    switch mode {
        case TEV_MODE_REPLACE: {
            out_rgb = src_rgb + (base.rgb * inv_a);
        }
        case TEV_MODE_MODULATE: {
            out_rgb = base.rgb * tex.rgb;
        }
        case TEV_MODE_ADD: {
            out_rgb = base.rgb + src_rgb;
        }
        case TEV_MODE_SUBTRACT: {
            out_rgb = base.rgb - src_rgb;
        }
        case TEV_MODE_ADD_SIGNED: {
            out_rgb = (1.0 - src_rgb) * base.rgb + (1.0 - base.rgb) * src_rgb;
        }
        case TEV_MODE_ADD_MULTIPLICATE: {
            out_rgb = clamp(base.rgb / (1.00001 - src_rgb), vec3<f32>(0.0), vec3<f32>(1.0));
        }
        case TEV_MODE_MULTIPLICATE_ADD: {
            out_rgb = clamp(1.0 - (1.00001 - base.rgb) / tex.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        }
        case TEV_MODE_OVERLAY: {
            let multi = 2.0 * base.rgb * tex.rgb;
            let screen = 1.0 - 2.0 * (1.0 - base.rgb) * (1.0 - tex.rgb);
            let mk = vec3<f32>(
                select(0.0, 1.0, base.r < 0.5),
                select(0.0, 1.0, base.g < 0.5),
                select(0.0, 1.0, base.b < 0.5)
            );
            out_rgb = mix(screen, multi, mk);
        }
        case TEV_MODE_LIGHTEN: {
            out_rgb = max(base.rgb, tex.rgb);
        }
        case TEV_MODE_DARKEN: {
            out_rgb = min(base.rgb, tex.rgb);
        }
        default: {
            out_rgb = tex.rgb;
        }
    }

    let out_a = select(min(base.a, tex.a), max(base.a, tex.a), select_alpha_max);
    return vec4<f32>(out_rgb, out_a);
}

fn sample_indirect(
    ic:  vec4<f32>, uv: vec2<f32>,
    m0:  vec4<f32>, m1: vec4<f32>,
    t:   texture_2d<f32>, s: sampler,
) -> vec4<f32> {
    let iv  = vec4<f32>(ic.xy, 0.0, 1.0);
    let offset = vec2<f32>(dot(iv, m0), dot(iv, m1));
    
    var c = textureSample(t, s, uv + offset);

    c.a = min(c.a, ic.a);
    return c;
}

fn sample_double_indirect(
    ic0: vec4<f32>,
    ic1: vec4<f32>,
    uv: vec2<f32>,
    m0: vec4<f32>,
    m1: vec4<f32>,
    t: texture_2d<f32>,
    s: sampler,
    combine_mode2: u32,
) -> vec4<f32> {
    let alpha = ic0.a * ic1.a;

    var color0 = vec4<f32>(ic0.rgb, 1.0);
    let color1 = vec4<f32>(ic1.rgb, 1.0);

    color0 = combine_layer(color0, color1, combine_mode2, true);

    let off = vec2<f32>(
        dot(color0.xy, m0.xy), 
        dot(color0.xy, m1.xy)
    ) * 0.5;

    var c = textureSample(t, s, uv + off);
    c.a = min(c.a, alpha);

    return c;
}

fn get_bits(bit: u32, pos: u32, len: u32) -> u32 {
    let mask = ~(0xFFFFFFFFu << len);
    return (bit >> pos) & mask;
}

const COLOR_OP_RGB:         u32 = 0u;
const COLOR_OP_INV_RGB:     u32 = 1u;
const COLOR_OP_ALPHA:       u32 = 2u;
const COLOR_OP_INV_ALPHA:   u32 = 3u;
const COLOR_OP_RRR:         u32 = 4u;
const COLOR_OP_INV_RRR:     u32 = 5u;
const COLOR_OP_GGG:         u32 = 6u;
const COLOR_OP_INV_GGG:     u32 = 7u;
const COLOR_OP_BBB:         u32 = 8u;
const COLOR_OP_INV_BBB:     u32 = 9u;

fn dc_op_rgb(op: u32, c: vec4<f32>) -> vec3<f32> {
    switch op {
        case COLOR_OP_RGB: { return c.rgb; }
        case COLOR_OP_INV_RGB: { return 1.0 - c.rgb; }
        case COLOR_OP_ALPHA: { return vec3<f32>(c.a); }
        case COLOR_OP_INV_ALPHA: { return vec3<f32>(1.0 - c.a); }
        case COLOR_OP_RRR: { return vec3<f32>(c.r); }
        case COLOR_OP_INV_RRR: { return vec3<f32>(1.0 - c.r); }
        case COLOR_OP_GGG: { return vec3<f32>(c.g); }
        case COLOR_OP_INV_GGG: { return vec3<f32>(1.0 - c.g); } 
        case COLOR_OP_BBB: { return vec3<f32>(c.b); }
        case COLOR_OP_INV_BBB: { return vec3<f32>(1.0 - c.b); } 
        default:   { return vec3<f32>(0.0); }
    }
}

const ALPHA_OP_ALPHA:       u32 = 0u;
const ALPHA_OP_INV_ALPHA:   u32 = 1u;
const ALPHA_OP_R:           u32 = 2u;
const ALPHA_OP_INV_R:       u32 = 3u;
const ALPHA_OP_G:           u32 = 4u;
const ALPHA_OP_INV_G:       u32 = 5u;
const ALPHA_OP_B:           u32 = 6u;
const ALPHA_OP_INV_B:       u32 = 7u;

fn dc_op_alpha(op: u32, c: vec4<f32>) -> f32 {
    switch op {
        case ALPHA_OP_ALPHA: { return c.a; }
        case ALPHA_OP_INV_ALPHA: { return 1.0 - c.a; }
        case ALPHA_OP_R: { return c.r; }
        case ALPHA_OP_INV_R: { return 1.0 - c.r; }
        case ALPHA_OP_G: { return c.g; }
        case ALPHA_OP_INV_G: { return 1.0 - c.g; }
        case ALPHA_OP_B: { return c.b; }
        case ALPHA_OP_INV_B: { return 1.0 - c.b; }
        default:   { return 0.0; }
    }
}

const DC_SRC_PRIMARY:   u32 = 0u;
const DC_SRC_TEXTURE0:  u32 = 3u;
const DC_SRC_TEXTURE1:  u32 = 4u;
const DC_SRC_TEXTURE2:  u32 = 5u;
const DC_SRC_REGISTER:  u32 = 13u;
const DC_SRC_CONSTANT:  u32 = 14u;
const DC_SRC_PREVIOUS:  u32 = 15u;

fn dc_src(
    src_id: u32,
    const_color: vec4<f32>,
    tex0: vec4<f32>, tex1: vec4<f32>, tex2: vec4<f32>,
    primary: vec4<f32>, previous: vec4<f32>, prev_buf: vec4<f32>,
) -> vec4<f32> {
    switch src_id {
        case DC_SRC_PRIMARY:    { return primary; }
        case DC_SRC_TEXTURE0:   { return tex0; }
        case DC_SRC_TEXTURE1:   { return tex1; }
        case DC_SRC_TEXTURE2:   { return tex2; }
        case DC_SRC_REGISTER:   { return prev_buf; }
        case DC_SRC_CONSTANT:   { return const_color; }
        case DC_SRC_PREVIOUS:   { return previous; }
        default:                { return vec4<f32>(0.0); }
    }
}

fn dc_mode_rgb(mode: u32, s: array<vec3<f32>, 3>) -> vec3<f32> {
    switch mode {
        case 0x0u: { return s[0]; }                                 // Replace
        case 0x1u: { return s[0] * s[1]; }                          // Modulate
        case 0x2u: { return s[0] + s[1]; }                          // Add
        case 0x3u: { return s[0] + s[1] - 0.5; }                    // AddSigned
        case 0x4u: { return s[0] * s[2] + s[1] * (1.0 - s[2]); }    // Interpolate
        case 0x5u: { return s[0] - s[1]; }                          // Subtract
        case 0x8u: { return (s[0] + s[1]) * s[2]; }                 // AddMult
        case 0x9u: { return s[0] * s[1] + s[2]; }                   // MultiplicateAdd
        default:   { return vec3<f32>(0.0); }                       // DOT3/unknown
    }
}

fn dc_mode_alpha(mode: u32, s: array<f32, 3>) -> f32 {
    switch mode {
        case 0x0u: { return s[0]; }
        case 0x1u: { return s[0] * s[1]; }
        case 0x2u: { return s[0] + s[1]; }
        case 0x3u: { return s[0] + s[1] - 0.5; }
        case 0x4u: { return s[0] * s[2] + s[1] * (1.0 - s[2]); }
        case 0x5u: { return s[0] - s[1]; }
        case 0x8u: { return (s[0] + s[1]) * s[2]; }
        case 0x9u: { return s[0] * s[1] + s[2]; }
        default:   { return 0.0; }
    }
}

fn dc_scale(s: u32) -> f32 {
    switch s {
        case 0u:  { return 1.0; }
        case 1u:  { return 2.0; }
        case 2u:  { return 4.0; }
        default:  { return 0.0; }
    }
}

fn dc_stage(
    stage_bit: vec4<i32>,
    constant_colors: array<vec4<f32>, 7>,
    tex0: vec4<f32>, tex1: vec4<f32>, tex2: vec4<f32>,
    primary:  vec4<f32>,
    output:   vec4<f32>, 
    buf_in:   vec4<f32>,  
) -> vec4<f32> { 

    let bx = u32(stage_bit.x);
    let by = u32(stage_bit.y);
    let bz = u32(stage_bit.z);
    let bw = u32(stage_bit.w);

    let konst_rgb_idx   = get_bits(bz, 0u, 4u);
    let konst_alpha_idx = get_bits(bz, 4u, 4u);

    let kc = vec4<f32>(
        constant_colors[konst_rgb_idx].r,
        constant_colors[konst_rgb_idx].g,
        constant_colors[konst_rgb_idx].b,
        constant_colors[konst_alpha_idx].a,
    );

    let rgb_count   = get_bits(bw, 0u, 4u);
    let alpha_count = get_bits(bw, 4u, 4u);

    var rgb_src = array<vec3<f32>, 3>(vec3<f32>(0.0), vec3<f32>(0.0), vec3<f32>(0.0));

    if rgb_count >= 1u {
        let sid = get_bits(bx,  0u, 4u);
        let op  = get_bits(bx, 12u, 4u);
        rgb_src[0] = dc_op_rgb(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    if rgb_count >= 2u {
        let sid = get_bits(bx,  4u, 4u);
        let op  = get_bits(bx, 16u, 4u);
        rgb_src[1] = dc_op_rgb(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    if rgb_count >= 3u {
        let sid = get_bits(bx,  8u, 4u);
        let op  = get_bits(bx, 20u, 4u);
        rgb_src[2] = dc_op_rgb(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    let rgb_mode  = get_bits(bx, 24u, 4u);
    let rgb_scale = get_bits(bx, 28u, 2u);
    let rgb_out   = clamp(dc_mode_rgb(rgb_mode, rgb_src) * dc_scale(rgb_scale),
                          vec3<f32>(0.0), vec3<f32>(1.0));

    var alpha_src = array<f32, 3>(0.0, 0.0, 0.0);

    if alpha_count >= 1u {
        let sid = get_bits(by,  0u, 4u);
        let op  = get_bits(by, 12u, 4u);
        alpha_src[0] = dc_op_alpha(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    if alpha_count >= 2u {
        let sid = get_bits(by,  4u, 4u);
        let op  = get_bits(by, 16u, 4u);
        alpha_src[1] = dc_op_alpha(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    if alpha_count >= 3u {
        let sid = get_bits(by,  8u, 4u);
        let op  = get_bits(by, 20u, 4u);
        alpha_src[2] = dc_op_alpha(op, dc_src(sid, kc, tex0, tex1, tex2, primary, output, buf_in));
    }

    let alpha_mode  = get_bits(by, 24u, 4u);
    let alpha_scale = get_bits(by, 28u, 2u);
    let alpha_out   = clamp(dc_mode_alpha(alpha_mode, alpha_src) * dc_scale(alpha_scale),
                            0.0, 1.0);

    return vec4<f32>(rgb_out, alpha_out);
}

fn dc_run(
    dc:      DetailedCombinerMaterial,
    tex0:    vec4<f32>,
    tex1:    vec4<f32>,
    tex2:    vec4<f32>,
    primary: vec4<f32>,
) -> vec4<f32> {
    var buf = dc.constant_colors[0];
    var out = vec4<f32>(0.0);

    if dc.stage_count >= 1u {
        out = dc_stage(dc.stage_bits[0], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    if dc.stage_count >= 2u {
        if get_bits(u32(dc.stage_bits[1].x), 30u, 1u) == 1u { buf.r = out.r; buf.g = out.g; buf.b = out.b; }
        if get_bits(u32(dc.stage_bits[1].y), 30u, 1u) == 1u { buf.a = out.a; }

        out = dc_stage(dc.stage_bits[1], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    if dc.stage_count >= 3u {
        if get_bits(u32(dc.stage_bits[2].x), 30u, 1u) == 1u { buf.r = out.r; buf.g = out.g; buf.b = out.b; }
        if get_bits(u32(dc.stage_bits[2].y), 30u, 1u) == 1u { buf.a = out.a; }

        out = dc_stage(dc.stage_bits[2], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    if dc.stage_count >= 4u {
        if get_bits(u32(dc.stage_bits[3].x), 30u, 1u) == 1u { buf.r = out.r; buf.g = out.g; buf.b = out.b; }
        if get_bits(u32(dc.stage_bits[3].y), 30u, 1u) == 1u { buf.a = out.a; }

        out = dc_stage(dc.stage_bits[3], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    if dc.stage_count >= 5u {
        if get_bits(u32(dc.stage_bits[4].x), 30u, 1u) == 1u { buf.r = out.r; buf.g = out.g; buf.b = out.b; }
        if get_bits(u32(dc.stage_bits[4].y), 30u, 1u) == 1u { buf.a = out.a; }

        out = dc_stage(dc.stage_bits[4], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    if dc.stage_count >= 6u {
        if get_bits(u32(dc.stage_bits[5].x), 30u, 1u) == 1u { buf.r = out.r; buf.g = out.g; buf.b = out.b; }
        if get_bits(u32(dc.stage_bits[5].y), 30u, 1u) == 1u { buf.a = out.a; }

        out = dc_stage(dc.stage_bits[5], dc.constant_colors, tex0, tex1, tex2, primary, out, buf);
    }

    return out;
}

fn sample_textures(count: u32, uv0: vec2<f32>, uv1: vec2<f32>, uv2: vec2<f32>, pos_mesh: vec2<f32>, mat: StandardMaterial)
    -> array<vec4<f32>, 3>
{
    var t = array<vec4<f32>, 3>(vec4<f32>(1.0), vec4<f32>(1.0), vec4<f32>(1.0));

    let byte0 = mat.tex_gen_mode & 0xFFu;
    let byte1 = (mat.tex_gen_mode >> 8u) & 0xFFu;
    let byte2 = (mat.tex_gen_mode >> 16u) & 0xFFu;

    let pos4 = vec4<f32>(pos_mesh, 0.0, 1.0);

    if count > 0u {
        var uv = uv0;
        if (byte0 & 0x3u) != 0u {
            uv = vec2<f32>(dot(pos4, mat.proj_mtx0[0]), dot(pos4, mat.proj_mtx0[1]));
        }

        t[0] = textureSample(t_texture0, s_sampler0, uv);
    }

    if count > 1u {
        var uv = uv1;
        if (byte1 & 0x3u) != 0u {
            uv = vec2<f32>(dot(pos4, mat.proj_mtx1[0]), dot(pos4, mat.proj_mtx1[1]));
        }

        t[1] = textureSample(t_texture1, s_sampler1, uv);
    }

    if count > 2u {
        var uv = uv2;
        if (byte2 & 0x3u) != 0u {
            uv = vec2<f32>(dot(pos4, mat.proj_mtx2[0]), dot(pos4, mat.proj_mtx2[1]));
        }

        t[2] = textureSample(t_texture2, s_sampler2, uv);
    }

    return t;
}

@fragment
fn fs_standard(in: VertexOutput) -> @location(0) vec4<f32> {
    let mat = u_standard;
    if mat.visible == 0u {
        discard;
    }

    if mat.is_plain == 1u {
        return plain_preview_color(in.tint, in.uv0, in.quad_size);
    }

    if mat.use_texture_only == 1u {
        var color = textureSample(t_texture0, s_sampler0, in.uv0);
        if mat.use_thresholding_alpha_interpolation == 1u {
            color.a = select(0.0, 1.0, color.a >= 0.5);
        }
        
        return color;
    }
    
    let t = sample_textures(mat.texture_count, in.uv0, in.uv1, in.uv2, in.pos_mesh, mat);

    if mat.debug_stage == 1u { return t[0]; }
    if mat.debug_stage == 2u { return t[1]; }
    if mat.debug_stage == 3u { return t[2]; }

    let byte0 = mat.tex_gen_mode & 0xFFu;
    let byte1 = (mat.tex_gen_mode >> 8u) & 0xFFu;
    let pos4  = vec4<f32>(in.pos_mesh, 0.0, 1.0);

    let is_proj0    = (byte0 & 0x3u) != 0u;
    let adjust_sr0  = (byte0 & (1u << 4u)) != 0u;
    let uv0_indirect = select(
        in.uv0,
        vec2<f32>(dot(pos4, mat.proj_mtx0[0]), dot(pos4, mat.proj_mtx0[1])),
        is_proj0 && !adjust_sr0,
    );

    let is_proj1    = (byte1 & 0x3u) != 0u;
    let adjust_sr1  = (byte1 & (1u << 4u)) != 0u;
    let uv1_indirect = select(
        in.uv1,
        vec2<f32>(dot(pos4, mat.proj_mtx1[0]), dot(pos4, mat.proj_mtx1[1])),
        is_proj1 && !adjust_sr1,
    );

    let iv = vec4<f32>(t[1].xyz, 1.0);
    let offset = vec2<f32>(dot(iv, mat.indirect_mtx0), dot(iv, mat.indirect_mtx1));

    if mat.debug_stage == 5u { return vec4<f32>(offset, 0.0, 1.0); }
    if mat.debug_stage == 6u { return vec4<f32>(uv0_indirect + offset, 0.0, 1.0); }

    let sel_a1 = (mat.alpha_select & 1u) != 0u;
    let sel_a2 = (mat.alpha_select & 2u) != 0u;

    var tex_color: vec4<f32>;

    if mat.texture_count == 0u {
        tex_color = vec4<f32>(1.0);
    } else if mat.texture_count == 1u {
        tex_color = t[0];
    } else if mat.texture_count == 2u {
        if mat.combine_mode == TEV_MODE_INDIRECT {
            tex_color = sample_indirect(t[1], uv0_indirect,
                mat.indirect_mtx0, mat.indirect_mtx1, t_texture0, s_sampler0);

            if mat.debug_stage == 7u { return tex_color; }
        } else if mat.combine_mode == TEV_MODE_BLEND_INDIRECT || mat.combine_mode == TEV_MODE_EACH_INDIRECT {
            tex_color = sample_double_indirect(t[1], t[1], uv0_indirect,
                mat.indirect_mtx0, mat.indirect_mtx1, t_texture0, s_sampler0, mat.combine_mode2);
        } else {
            tex_color = combine_layer(t[0], t[1], mat.combine_mode, sel_a1);
        }
    } else {
        if mat.combine_mode == TEV_MODE_INDIRECT {
            let ai = sample_indirect(t[1], uv0_indirect,
                mat.indirect_mtx0, mat.indirect_mtx1, t_texture0, s_sampler0);

            if mat.debug_stage == 7u { return ai; }
            tex_color = combine_layer(ai, t[2], mat.combine_mode2, sel_a2);
        } else if mat.combine_mode2 == TEV_MODE_INDIRECT {
            let ai = sample_indirect(t[2], uv1_indirect,
                mat.indirect_mtx0, mat.indirect_mtx1, t_texture1, s_sampler1);
            tex_color = combine_layer(t[0], ai, mat.combine_mode, sel_a1);
        } else if mat.combine_mode == TEV_MODE_BLEND_INDIRECT || mat.combine_mode == TEV_MODE_EACH_INDIRECT {
            tex_color = sample_double_indirect(t[1], t[2], uv0_indirect,
                mat.indirect_mtx0, mat.indirect_mtx1, t_texture0, s_sampler0, mat.combine_mode2);
        } else {
            let l1 = combine_layer(t[0], t[1], mat.combine_mode,  sel_a1);
            tex_color = combine_layer(l1,   t[2], mat.combine_mode2, sel_a2);
        }
    }

    if mat.debug_stage == 4u { return tex_color; }

    if mat.debug_stage == 8u { return vec4<f32>(vec3<f32>(tex_color.a), 1.0); }

    var color = mat.interpolate_offset + mat.interpolate_width * tex_color;
    color.a     = clamp(color.a, 0.0, 1.0);

    if mat.debug_stage == 9u { return color; }

    if mat.use_thresholding_alpha_interpolation == 1u {
        color.a = select(0.0, 1.0, color.a >= 0.5);
    }

    return color;
}

@fragment
fn fs_detailed(in: VertexOutput) -> @location(0) vec4<f32> {
    let dc = u_detailed;
    
    var dummy_mat: StandardMaterial;
    dummy_mat.tex_gen_mode = u_standard.tex_gen_mode;
    dummy_mat.indirect_mtx0 = u_standard.indirect_mtx0;
    dummy_mat.indirect_mtx1 = u_standard.indirect_mtx1;

    let t = sample_textures(dc.texture_count, in.uv0, in.uv1, in.uv2, in.pos_mesh, dummy_mat);

    let primary = in.tint;

    var color = dc_run(dc, t[0], t[1], t[2], primary);
    color     *= in.tint;
    color.a    = clamp(color.a, 0.0, 1.0);
    
    return color;
}