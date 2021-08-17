#version 300 es
precision highp float;

uniform vec2 u_resolution;
uniform float u_time;

out vec4 outColor;


void main() {
    vec2 st = gl_FragCoord.xy / u_resolution;

    outColor = vec4(abs(sin(u_time)), sin(st.x), sin(st.y), 1.0);
}
