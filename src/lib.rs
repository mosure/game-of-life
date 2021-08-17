use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

extern crate console_error_panic_hook;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    WebGl2RenderingContext,
    WebGlFramebuffer,
    WebGlProgram,
    WebGlShader,
    WebGlTexture,
};


/*
 * Two Programs:
 *   - compute the state of the system
 *   - renders the system
 */

fn window() -> web_sys::Window {
    return web_sys::window().expect("should have a window in this context");
}

fn document() -> web_sys::Document {
    return window().document().expect("should have a document in this context");
}

fn canvas() -> web_sys::HtmlCanvasElement {
    return document()
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("should have a canvas in this context");
}

fn body() -> web_sys::HtmlBodyElement {
    return document()
        .body()
        .unwrap()
        .dyn_into::<web_sys::HtmlBodyElement>()
        .expect("should have a body in this context");
}

fn performance() -> web_sys::Performance {
    return window().performance().expect("should have a performance in this context");
}

fn set_canvas_dimensions(width: u32, height: u32) {
    canvas().set_width(width);
    canvas().set_height(height);
}

struct AppContext {
    width: u32,
    height: u32,
    boot_time: SystemTime,
}

trait Stage {
    fn init(&mut self) -> ();
    fn render(&mut self) -> ();
}

struct DrawStage {
    gl: Rc<WebGl2RenderingContext>,
    program: Rc<WebGlProgram>,
    vert_count: u32,
    vertices: [f32; 18],
}

struct ComputeStage {
    gl: Rc<WebGl2RenderingContext>,
    ctx: Rc<AppContext>,
    program: Rc<WebGlProgram>,
    framebuffer: WebGlFramebuffer,
    state: WebGlTexture,
    draw_stage: DrawStage,
}

impl DrawStage {
    pub fn new(
        gl: Rc<WebGl2RenderingContext>,
        program: Rc<WebGlProgram>,
    ) -> DrawStage {
        let vertices: [f32; 18] = [
            -1.0, -1.0, 0.0,
            -1.0, 1.0, 0.0,
            1.0, 1.0, 0.0,

            1.0, 1.0, 0.0,
            1.0, -1.0, 0.0,
            -1.0, -1.0, 0.0,
        ];

        return DrawStage {
            gl: gl,
            program: program,
            vertices: vertices,
            vert_count: (vertices.len() / 3) as u32,
        };
    }
}

impl Stage for DrawStage {
    fn init(&mut self) -> () {
        let position_attribute_location = self.gl.get_attrib_location(&self.program, "position");
        let buffer = self.gl.create_buffer();
        self.gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, buffer.as_ref());

        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        unsafe {
            let positions_array_buf_view = js_sys::Float32Array::view(&self.vertices);

            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &positions_array_buf_view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        let vao = self.gl.create_vertex_array().expect("should have a vertex array object");
        self.gl.bind_vertex_array(Some(&vao));

        self.gl.vertex_attrib_pointer_with_i32(0, 3, WebGl2RenderingContext::FLOAT, false, 0, 0);
        self.gl.enable_vertex_attrib_array(position_attribute_location as u32);

        self.gl.bind_vertex_array(Some(&vao));
    }

    fn render(&mut self) -> () {
        self.gl.clear_color(1.0, 0.0, 0.0, 1.0);
        self.gl.clear(
            WebGl2RenderingContext::COLOR_BUFFER_BIT |
            WebGl2RenderingContext::DEPTH_BUFFER_BIT
        );

        self.gl.draw_arrays(
            WebGl2RenderingContext::TRIANGLES,
            0,
            self.vert_count as i32
        );
    }
}

impl ComputeStage {
    fn new(
        gl: Rc<WebGl2RenderingContext>,
        ctx: Rc<AppContext>,
    ) -> ComputeStage {
        let vert_shader = compile_shader(
            &gl,
            WebGl2RenderingContext::VERTEX_SHADER,
            include_str!("shaders/vert.glsl"),
        ).expect("expect vertex shader");

        let frag_shader = compile_shader(
            &gl,
            WebGl2RenderingContext::FRAGMENT_SHADER,
            include_str!("shaders/frag.glsl"),
        ).expect("expect frag shader");

        let program = Rc::new(
            link_program(
                &gl,
                &vert_shader,
                &frag_shader
            ).expect("expect linked program")
        );

        let framebuffer = gl.create_framebuffer().expect("should have a framebuffer");
        let state = gl.create_texture().expect("should have a texture");

        let draw_stage = DrawStage::new(
            Rc::clone(&gl),
            Rc::clone(&program),
        );

        return ComputeStage {
            gl: gl,
            ctx: ctx,
            program: program,
            framebuffer: framebuffer,
            state: state,
            draw_stage: draw_stage,
        };
    }
}

impl Stage for ComputeStage {
    fn init(&mut self) {
        self.draw_stage.init();

        self.gl.use_program(Some(&self.program));

        self.gl.viewport(0, 0, self.ctx.width as i32, self.ctx.height as i32);

        // self.gl.bind_framebuffer(
        //     WebGl2RenderingContext::FRAMEBUFFER,
        //     Some(&self.framebuffer)
        // );

        self.gl.active_texture(
            WebGl2RenderingContext::TEXTURE0,
        );

        self.gl.bind_texture(
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.state)
        );

        self.gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MAG_FILTER,
            WebGl2RenderingContext::LINEAR as i32
        );
        self.gl.tex_parameteri(
            WebGl2RenderingContext::TEXTURE_2D,
            WebGl2RenderingContext::TEXTURE_MIN_FILTER,
            WebGl2RenderingContext::LINEAR as i32
        );

        self.gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA as i32,
            self.ctx.width as i32,
            self.ctx.height as i32,
            0,
            WebGl2RenderingContext::RGBA as u32,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            None,
        ).expect("expect tex image 2d result");

        self.gl.framebuffer_texture_2d(
            WebGl2RenderingContext::FRAMEBUFFER,
            WebGl2RenderingContext::COLOR_ATTACHMENT1,
            WebGl2RenderingContext::TEXTURE_2D,
            Some(&self.state),
            0,
        );
    }

    fn render(&mut self) {
        // self.gl.bind_framebuffer(
        //     WebGl2RenderingContext::FRAMEBUFFER,
        //     Some(&self.framebuffer)
        // );

        let u_resolution = self.gl.get_uniform_location(&self.program, "u_resolution");
        let u_time = self.gl.get_uniform_location(&self.program, "u_time");

        let mut u_res_val = [self.ctx.width as f32, self.ctx.height as f32];
        self.gl.uniform2fv_with_f32_array(u_resolution.as_ref(), &mut u_res_val);

        let start = perf_to_system(performance().now());
        let since_the_epoch = start
            .duration_since(self.ctx.boot_time)
            .expect("Time went backwards");

        self.gl.uniform1f(u_time.as_ref(), since_the_epoch.as_secs_f32());

        self.draw_stage.render();

        // self.gl.bind_framebuffer(
        //     WebGl2RenderingContext::FRAMEBUFFER,
        //     None
        // );
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let gl = Rc::new(
        canvas()
            .get_context("webgl2")?
            .unwrap()
            .dyn_into::<WebGl2RenderingContext>()?
    );

    let ctx = Rc::new(
        AppContext {
            width: body().client_width() as u32,
            height: body().client_height() as u32,
            boot_time: perf_to_system(performance().now()),
        }
    );

    set_canvas_dimensions(ctx.width, ctx.height);

    let mut compute_stage = ComputeStage::new(
        Rc::clone(&gl),
        Rc::clone(&ctx),
    );
    compute_stage.init();


    let f = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        compute_stage.render();

        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());

    Ok(())
}



fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window().expect("no global `window` exists")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

pub fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    gl: &WebGl2RenderingContext,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = gl
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    gl.attach_shader(&program, vert_shader);
    gl.attach_shader(&program, frag_shader);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}

fn perf_to_system(amt: f64) -> SystemTime {
    let secs = (amt as u64) / 1_000;
    let nanos = (((amt as u64) % 1_000) as u32) * 1_000_000;
    UNIX_EPOCH + Duration::new(secs, nanos)
}
