#version 300 es
precision highp float;

#define PI                  3.14159265359
#define RESET_TIME_SECONDS  2.0

uniform vec2        u_resolution;
uniform sampler2D   u_state;
uniform float       u_time;

out vec4 outColor;


vec2 shift(vec2 x, vec2 shift) {
    return mod((x + u_resolution + shift), u_resolution);
}

bool try_reset() {
    if (u_time < RESET_TIME_SECONDS) {
        vec2 st = gl_FragCoord.xy / u_resolution;

        outColor = vec4(sin(st.x * 20.0 * PI), 0.0235, 0.0235, 1.0);
        return true;
    }

    return false;
}

void main() {
    if (try_reset()) {
        return;
    }

    vec2 st = gl_FragCoord.xy / u_resolution;

    vec4 lookup = texture(u_state, gl_FragCoord.xy);

    outColor = vec4(abs(sin(u_time)), sin(st.x), sin(st.y), 1.0) - (lookup / 200.0);
}
