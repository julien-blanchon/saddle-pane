// HSV color plane shader — renders Saturation (X) × Value (Y) for a given Hue.
#import bevy_ui::ui_vertex_output::UiVertexOutput

struct ColorPlaneUniform {
    hue: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    _webgl2_padding_12b: vec3<f32>,
#endif
}

@group(1) @binding(0) var<uniform> uniform_data: ColorPlaneUniform;

// Convert HSV to linear RGB
fn hsv_to_linear_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let hp = h / 60.0;
    let x = c * (1.0 - abs(hp % 2.0 - 1.0));
    let m = v - c;

    var rgb: vec3<f32>;
    if hp < 1.0 {
        rgb = vec3(c, x, 0.0);
    } else if hp < 2.0 {
        rgb = vec3(x, c, 0.0);
    } else if hp < 3.0 {
        rgb = vec3(0.0, c, x);
    } else if hp < 4.0 {
        rgb = vec3(0.0, x, c);
    } else if hp < 5.0 {
        rgb = vec3(x, 0.0, c);
    } else {
        rgb = vec3(c, 0.0, x);
    }

    // sRGB to linear approximation (gamma 2.2)
    let srgb = rgb + vec3(m, m, m);
    return pow(srgb, vec3(2.2, 2.2, 2.2));
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    // X = saturation (0..1), Y = value (1 at top, 0 at bottom)
    let s = uv.x;
    let v = 1.0 - uv.y;
    let color = hsv_to_linear_rgb(uniform_data.hue, s, v);
    return vec4(color, 1.0);
}
