#version 300 es
precision highp float;

#define PI                  3.14159265359
#define RESET_TIME_SECONDS  0.5
#define PIXELS_PER_CELL     25
#define FPS                 10.0

uniform int         u_frame;
uniform vec2        u_resolution;
uniform sampler2D   u_state;
uniform float       u_time;

out vec4 outColor;


float random (in vec2 st) {
    return fract(sin(dot(st.xy,
                         vec2(12.9898,78.233)))
                 * 43758.5453123);
}

// 2D Noise based on Morgan McGuire @morgan3d
// https://www.shadertoy.com/view/4dS3Wd
float noise(in vec2 st) {
    vec2 i = floor(st);
    vec2 f = fract(st);

    // Four corners in 2D of a tile
    float a = random(i);
    float b = random(i + vec2(1.0, 0.0));
    float c = random(i + vec2(0.0, 1.0));
    float d = random(i + vec2(1.0, 1.0));

    // Smooth Interpolation

    // Cubic Hermine Curve.  Same as SmoothStep()
    vec2 u = f*f*(3.0-2.0*f);
    // u = smoothstep(0.,1.,f);

    // Mix 4 coorners percentages
    return mix(a, b, u.x) +
            (c - a)* u.y * (1.0 - u.x) +
            (d - b) * u.x * u.y;
}

vec4 get_cell_at(ivec2 coords) {
    vec2 lookup_coords = mod(vec2(coords) * vec2(PIXELS_PER_CELL), u_resolution);

    return texture(u_state, lookup_coords / u_resolution + 1.0 / (u_resolution * 10.0));
}

bool is_coord_alive(ivec2 coords) {
    return get_cell_at(coords).a == 1.0;
}

ivec2 get_mapped_coords(vec2 coords) {
    return ivec2(coords) / ivec2(PIXELS_PER_CELL);
}

int get_neighbor_count(ivec2 coords) {
    int count = 0;

    count += int(is_coord_alive(coords + ivec2(-1, -1)));
    count += int(is_coord_alive(coords + ivec2(0, -1)));
    count += int(is_coord_alive(coords + ivec2(1, -1)));
    count += int(is_coord_alive(coords + ivec2(-1, 0)));
    count += int(is_coord_alive(coords + ivec2(1, 0)));
    count += int(is_coord_alive(coords + ivec2(-1, 1)));
    count += int(is_coord_alive(coords + ivec2(0, 1)));
    count += int(is_coord_alive(coords + ivec2(1, 1)));

    return count;
}

bool should_live() {
    ivec2 mapped_coords = get_mapped_coords(gl_FragCoord.xy);

    if (u_frame % int(60.0 / FPS) != 0) {
        return is_coord_alive(mapped_coords);
    }

    int neighbor_count = get_neighbor_count(mapped_coords);

    return neighbor_count == 3 || (
        neighbor_count == 2 && is_coord_alive(mapped_coords)
    );
}

bool try_reset() {
    if (u_time < RESET_TIME_SECONDS) {
        vec2 st = gl_FragCoord.xy / u_resolution.xy;

        float rnd = floor(noise(st * 1000.0) + 0.5);

        outColor = vec4(vec3(0.0), rnd);
        return true;
    }

    return false;
}

void main() {
    if (try_reset()) {
        return;
    } else {
        vec2 st = gl_FragCoord.xy / u_resolution;

        float opacity = 0.0;
        if (should_live()) {
            opacity = 1.0;
        }

        outColor = vec4(abs(sin(u_time)), sin(st.x), sin(st.y), opacity);
    }
}
