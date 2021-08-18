#version 300 es
precision highp float;

#define PI                  3.14159265359
#define RESET_TIME_SECONDS  2.0

uniform vec2        u_resolution;
uniform sampler2D   u_state;
uniform float       u_time;

out vec4 outColor;

int grid_resolution = 4;
mat4 last_render = mat4(
    0, 0, 1, 0,
    0, 1, 0, 1,
    0, 0, 1, 0,
    0, 0, 0, 0
);


int get_neighbor_at(ivec2 coords) {
    return int(last_render[coords.x % grid_resolution][coords.y % grid_resolution]);
}

ivec2 get_mapped_coords(vec2 coords) {
    return ivec2(ceil(coords * (vec2(grid_resolution) / u_resolution))) - 1;
}

int get_neighbor_count(ivec2 mapped_coords) {
    int count = 0;

    count += get_neighbor_at(mapped_coords + ivec2(-1, -1));
    count += get_neighbor_at(mapped_coords + ivec2(0, -1));
    count += get_neighbor_at(mapped_coords + ivec2(1, -1));
    count += get_neighbor_at(mapped_coords + ivec2(-1, 0));
    count += get_neighbor_at(mapped_coords + ivec2(1, 0));
    count += get_neighbor_at(mapped_coords + ivec2(-1, 1));
    count += get_neighbor_at(mapped_coords + ivec2(0, 1));
    count += get_neighbor_at(mapped_coords + ivec2(1, 1));

    return count;
}

bool is_self_alive(ivec2 mapped_coords) {
    return last_render[mapped_coords.x][mapped_coords.y] == 1.0;
}

bool should_live() {
    ivec2 mapped_coords = get_mapped_coords(gl_FragCoord.xy);
    int neighbor_count = get_neighbor_count(mapped_coords);

    return neighbor_count == 3 || (
        neighbor_count == 2 && is_self_alive(mapped_coords)
    );
}

void main() {
    vec2 st = gl_FragCoord.xy / u_resolution;

    float opacity = 0.0;
    if (should_live()) {
        opacity = 1.0;
    }

    outColor = vec4(abs(sin(u_time)), sin(st.x), sin(st.y), opacity);
}
