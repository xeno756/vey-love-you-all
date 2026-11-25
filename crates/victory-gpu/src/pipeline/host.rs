//! Host pipelines for title pipelines (decision 0007, step 3): the
//! title's graphics state with the translated shaders, under a root
//! signature of our own that the translated shaders' bindings define
//! (`victory_shader::emit`): per stage one block of root constants for the
//! user data, root views for the title's root views, static samplers for
//! the immediate samplers, and directly indexed heaps.

use std::ffi::{CStr, CString};
use std::sync::Arc;

use victory_shader::emit::{ROOT_CBV_SPACE, SAMPLER_SPACE, USER_DATA_SPACE};
use windows::Win32::Graphics::Direct3D::ID3DBlob;
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::PCSTR;

use super::{Shader, Translated};
use crate::host::host;

/// A stream's graphics state, owned (the title's stream lives only for the
/// creation call). Subobjects the stream leaves out keep D3D12's defaults.
pub(crate) struct GraphicsState {
    blend: D3D12_BLEND_DESC,
    sample_mask: u32,
    rasterizer: D3D12_RASTERIZER_DESC,
    depth: D3D12_DEPTH_STENCIL_DESC,
    /// Whether the depth bounds test is on ([`bounds_pipeline`]).
    bounds: bool,
    /// Input elements, their names pointing into `names`.
    inputs: Vec<D3D12_INPUT_ELEMENT_DESC>,
    names: Vec<CString>,
    strip_cut: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE,
    pub(crate) topology: D3D12_PRIMITIVE_TOPOLOGY_TYPE,
    pub(crate) targets: D3D12_RT_FORMAT_ARRAY,
    depth_format: DXGI_FORMAT,
    samples: DXGI_SAMPLE_DESC,
}

impl Default for GraphicsState {
    fn default() -> Self {
        let target = D3D12_RENDER_TARGET_BLEND_DESC {
            SrcBlend: D3D12_BLEND_ONE,
            DestBlend: D3D12_BLEND_ZERO,
            BlendOp: D3D12_BLEND_OP_ADD,
            SrcBlendAlpha: D3D12_BLEND_ONE,
            DestBlendAlpha: D3D12_BLEND_ZERO,
            BlendOpAlpha: D3D12_BLEND_OP_ADD,
            LogicOp: D3D12_LOGIC_OP_NOOP,
            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
            ..Default::default()
        };
        let keep = D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D12_STENCIL_OP_KEEP,
            StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
            StencilPassOp: D3D12_STENCIL_OP_KEEP,
            StencilFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        };
        GraphicsState {
            blend: D3D12_BLEND_DESC {
                RenderTarget: [target; 8],
                ..Default::default()
            },
            sample_mask: u32::MAX,
            rasterizer: D3D12_RASTERIZER_DESC {
                FillMode: D3D12_FILL_MODE_SOLID,
                CullMode: D3D12_CULL_MODE_BACK,
                DepthClipEnable: true.into(),
                ..Default::default()
            },
            depth: D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: true.into(),
                DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ALL,
                DepthFunc: D3D12_COMPARISON_FUNC_LESS,
                StencilReadMask: 0xff,
                StencilWriteMask: 0xff,
                FrontFace: keep,
                BackFace: keep,
                ..Default::default()
            },
            bounds: false,
            inputs: Vec::new(),
            names: Vec::new(),
            strip_cut: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
            topology: D3D12_PRIMITIVE_TOPOLOGY_TYPE_UNDEFINED,
            targets: D3D12_RT_FORMAT_ARRAY::default(),
            depth_format: DXGI_FORMAT_UNKNOWN,
            samples: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
        }
    }
}

impl GraphicsState {
    /// Takes in one subobject of a stream: its type, then its data at +4
    /// (+8 for those holding pointers).
    ///
    /// # Safety
    /// `at` points at a whole subobject of type `kind`.
    pub(crate) unsafe fn read(&mut self, kind: u32, at: *const u8) {
        // SAFETY: the caller passes a whole subobject.
        unsafe {
            let data = at.add(4);
            match kind {
                8 => self.blend = data.cast::<D3D12_BLEND_DESC>().read_unaligned(),
                9 => self.sample_mask = data.cast::<u32>().read_unaligned(),
                10 => self.rasterizer = data.cast::<D3D12_RASTERIZER_DESC>().read_unaligned(),
                11 => self.depth = data.cast::<D3D12_DEPTH_STENCIL_DESC>().read_unaligned(),
                21 => {
                    let d = data.cast::<D3D12_DEPTH_STENCIL_DESC1>().read_unaligned();
                    self.bounds = d.DepthBoundsTestEnable.as_bool();
                    self.depth = D3D12_DEPTH_STENCIL_DESC {
                        DepthEnable: d.DepthEnable,
                        DepthWriteMask: d.DepthWriteMask,
                        DepthFunc: d.DepthFunc,
                        StencilEnable: d.StencilEnable,
                        StencilReadMask: d.StencilReadMask,
                        StencilWriteMask: d.StencilWriteMask,
                        FrontFace: d.FrontFace,
                        BackFace: d.BackFace,
                    };
                }
                12 => {
                    let layout = at.add(8).cast::<D3D12_INPUT_LAYOUT_DESC>().read_unaligned();
                    if layout.pInputElementDescs.is_null() {
                        return;
                    }
                    let elements = std::slice::from_raw_parts(
                        layout.pInputElementDescs,
                        layout.NumElements as usize,
                    );
                    self.names = elements
                        .iter()
                        .map(|e| CStr::from_ptr(e.SemanticName.0.cast()).to_owned())
                        .collect();
                    self.inputs = elements
                        .iter()
                        .zip(&self.names)
                        .map(|(e, name)| D3D12_INPUT_ELEMENT_DESC {
                            SemanticName: PCSTR(name.as_ptr().cast()),
                            Format: DXGI_FORMAT(
                                crate::resource::pc_format(e.Format.0 as u32) as i32
                            ),
                            ..*e
                        })
                        .collect();
                }
                13 => {
                    self.strip_cut =
                        D3D12_INDEX_BUFFER_STRIP_CUT_VALUE(data.cast::<i32>().read_unaligned())
                }
                14 => {
                    self.topology =
                        D3D12_PRIMITIVE_TOPOLOGY_TYPE(data.cast::<i32>().read_unaligned())
                }
                15 => {
                    self.targets = data.cast::<D3D12_RT_FORMAT_ARRAY>().read_unaligned();
                    for f in &mut self.targets.RTFormats {
                        f.0 = crate::resource::target_format(crate::resource::pc_format(
                            f.0 as u32,
                        )) as i32;
                    }
                }
                16 => {
                    let f = data.cast::<u32>().read_unaligned();
                    self.depth_format = DXGI_FORMAT(crate::resource::pc_format(f) as i32);
                }
                17 => self.samples = data.cast::<DXGI_SAMPLE_DESC>().read_unaligned(),
                _ => {}
            }
        }
    }

    /// The state of a precompiled pipeline's decoded state blocks
    /// ([`victory_game::pso_state`]), as [`GraphicsState::read`] takes in
    /// the same fields from a stream (formats through the same
    /// conversions).
    pub(crate) fn from_precompiled(d: &victory_game::pso_state::GraphicsDesc) -> GraphicsState {
        let mut g = GraphicsState::default();
        let target = |t: &victory_game::pso_state::TargetBlend| D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: t.blend.into(),
            LogicOpEnable: t.logic_op_enable.into(),
            SrcBlend: D3D12_BLEND(t.src as i32),
            DestBlend: D3D12_BLEND(t.dst as i32),
            BlendOp: D3D12_BLEND_OP(t.op as i32),
            SrcBlendAlpha: D3D12_BLEND(t.src_alpha as i32),
            DestBlendAlpha: D3D12_BLEND(t.dst_alpha as i32),
            BlendOpAlpha: D3D12_BLEND_OP(t.op_alpha as i32),
            LogicOp: D3D12_LOGIC_OP(t.logic_op as i32),
            RenderTargetWriteMask: t.write_mask,
        };
        g.blend = D3D12_BLEND_DESC {
            AlphaToCoverageEnable: d.alpha_to_coverage.into(),
            IndependentBlendEnable: d.independent_blend.into(),
            RenderTarget: d.targets_blend.each_ref().map(target),
        };
        g.sample_mask = d.sample_mask;
        g.rasterizer = D3D12_RASTERIZER_DESC {
            FillMode: D3D12_FILL_MODE(d.fill_mode as i32),
            CullMode: D3D12_CULL_MODE(d.cull_mode as i32),
            FrontCounterClockwise: d.front_ccw.into(),
            DepthBias: d.depth_bias,
            DepthBiasClamp: d.depth_bias_clamp,
            SlopeScaledDepthBias: d.slope_scaled_depth_bias,
            DepthClipEnable: d.depth_clip.into(),
            ..Default::default()
        };
        let face = |f: &victory_game::pso_state::StencilFace| D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D12_STENCIL_OP(f.fail as i32),
            StencilDepthFailOp: D3D12_STENCIL_OP(f.depth_fail as i32),
            StencilPassOp: D3D12_STENCIL_OP(f.pass as i32),
            StencilFunc: D3D12_COMPARISON_FUNC(f.func as i32),
        };
        g.depth = D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: d.depth_enable.into(),
            DepthWriteMask: D3D12_DEPTH_WRITE_MASK(d.depth_write as i32),
            DepthFunc: D3D12_COMPARISON_FUNC(d.depth_func as i32),
            StencilEnable: d.stencil_enable.into(),
            StencilReadMask: d.stencil_read_mask,
            StencilWriteMask: d.stencil_write_mask,
            FrontFace: face(&d.front),
            BackFace: face(&d.back),
        };
        g.bounds = d.depth_bounds;
        g.names = d
            .inputs
            .iter()
            .map(|e| CString::new(e.name.as_str()).unwrap_or_default())
            .collect();
        g.inputs = d
            .inputs
            .iter()
            .zip(&g.names)
            .map(|(e, name)| D3D12_INPUT_ELEMENT_DESC {
                SemanticName: PCSTR(name.as_ptr().cast()),
                SemanticIndex: e.index,
                Format: DXGI_FORMAT(crate::resource::pc_format(e.format) as i32),
                InputSlot: e.slot,
                AlignedByteOffset: e.offset,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION(e.class as i32),
                InstanceDataStepRate: e.step_rate,
            })
            .collect();
        g.strip_cut = D3D12_INDEX_BUFFER_STRIP_CUT_VALUE(d.strip_cut as i32);
        g.topology = D3D12_PRIMITIVE_TOPOLOGY_TYPE(d.topology as i32);
        g.targets = D3D12_RT_FORMAT_ARRAY {
            RTFormats: d.target_formats.map(|f| {
                DXGI_FORMAT(crate::resource::target_format(crate::resource::pc_format(f)) as i32)
            }),
            NumRenderTargets: d.num_targets,
        };
        g.depth_format = DXGI_FORMAT(crate::resource::pc_format(d.depth_format) as i32);
        g.samples = DXGI_SAMPLE_DESC {
            Count: d.sample_count,
            Quality: d.sample_quality,
        };
        g
    }

    /// Everything the state holds, as text: part of a pipeline's name in
    /// the library and of the key a title pipeline is started early by.
    pub(crate) fn describe(&self) -> String {
        let inputs: Vec<String> = self
            .inputs
            .iter()
            .zip(&self.names)
            .map(|(e, name)| {
                format!(
                    "{name:?} {} {} {} {} {} {}",
                    e.SemanticIndex, e.Format.0, e.InputSlot, e.AlignedByteOffset, e.InputSlotClass.0, e.InstanceDataStepRate
                )
            })
            .collect();
        // (Pipelines without the test keep the names they had.)
        let bounds = if self.bounds { " bounds" } else { "" };
        format!(
            "{:?} {} {:?} {:?} {inputs:?} {} {} {:?} {} {:?}{bounds}",
            self.blend,
            self.sample_mask,
            self.rasterizer,
            self.depth,
            self.strip_cut.0,
            self.topology.0,
            self.targets,
            self.depth_format.0,
            self.samples
        )
    }

    /// Its depth, stencil, cull and blend state in short, for the frame
    /// listing.
    fn short(&self) -> String {
        let (d, r, b) = (&self.depth, &self.rasterizer, &self.blend.RenderTarget[0]);
        let face = |f: &D3D12_DEPTH_STENCILOP_DESC| {
            format!(
                "fail {} depth-fail {} pass {} func {}",
                f.StencilFailOp.0, f.StencilDepthFailOp.0, f.StencilPassOp.0, f.StencilFunc.0
            )
        };
        format!(
            "depth {} write {} func {}; stencil {} read {:#x} write {:#x} front [{}] back [{}]; cull {} front-ccw {}; blend {} src {} dst {} op {} mask {:#x}",
            d.DepthEnable.as_bool(),
            d.DepthWriteMask.0,
            d.DepthFunc.0,
            d.StencilEnable.as_bool(),
            d.StencilReadMask,
            d.StencilWriteMask,
            face(&d.FrontFace),
            face(&d.BackFace),
            r.CullMode.0,
            r.FrontCounterClockwise.as_bool(),
            b.BlendEnable.as_bool(),
            b.SrcBlend.0,
            b.DestBlend.0,
            b.BlendOp.0,
            b.RenderTargetWriteMask
        )
    }

    /// The blend state for the host, where the console takes what D3D12
    /// refuses (it then refuses the whole pipeline, and every draw of it is
    /// dropped):
    /// - blending on a render target whose format cannot blend (integer
    ///   formats; debug layer 676, "this format does not support
    ///   blending"): off on those targets, as the console ignores it there;
    /// - blending with the second source (dual-source) anywhere but on the
    ///   first target alone (debug layer 112, 113, 115): the title's state
    ///   blocks give every target the first one's description, also the
    ///   targets a pipeline does not have. Kept on the first target, off on
    ///   the others. 1,827 of the title's mesh pipelines and some 440
    ///   others were refused for it on the first card it was counted on.
    ///
    /// With one blend description for all targets, it is copied to each
    /// first so that only those targets change.
    pub(crate) fn host_blend(&self) -> D3D12_BLEND_DESC {
        let mut blend = self.blend;
        let n = self.targets.NumRenderTargets.min(8) as usize;
        let independent = blend.IndependentBlendEnable.as_bool();
        let of = |i: usize| if independent { i } else { 0 };
        let integer: Vec<usize> = (0..n)
            .filter(|&i| integer_format(self.targets.RTFormats[i]))
            .filter(|&i| blend.RenderTarget[of(i)].BlendEnable.as_bool())
            .collect();
        let first_dual = blend.RenderTarget[0].BlendEnable.as_bool() && dual_source(&blend.RenderTarget[0]);
        // Which targets past the first D3D12 would refuse: with their own
        // descriptions all eight are checked, with one for all only those
        // the pipeline has.
        let past_first = if independent { 8 } else { n };
        let dual: Vec<usize> = (1..past_first)
            .filter(|&i| {
                let t = &blend.RenderTarget[of(i)];
                dual_source(t) || first_dual && t.BlendEnable.as_bool()
            })
            .collect();
        if integer.is_empty() && dual.is_empty() {
            return blend;
        }
        if !independent {
            blend.RenderTarget = [blend.RenderTarget[0]; 8];
            blend.IndependentBlendEnable = true.into();
            // (Its other entries were not looked at; now they are.)
            if first_dual {
                (1..8).for_each(|i| plain(&mut blend.RenderTarget[i]));
            }
        }
        for i in integer {
            blend.RenderTarget[i].BlendEnable = false.into();
        }
        for i in dual {
            plain(&mut blend.RenderTarget[i]);
        }
        blend
    }

    /// The vertex buffer slots the input layout reads.
    fn slots(&self) -> u32 {
        self.inputs
            .iter()
            .fold(0, |m, e| m | 1 << e.InputSlot.min(31))
    }

    /// The input elements with their byte offsets worked out.
    fn elements(&self) -> Vec<Input> {
        let mut next = [0u32; 32];
        self.inputs
            .iter()
            .map(|e| {
                let slot = e.InputSlot.min(31) as usize;
                let format = e.Format.0 as u32;
                let offset = if e.AlignedByteOffset == D3D12_APPEND_ALIGNED_ELEMENT {
                    next[slot]
                } else {
                    e.AlignedByteOffset
                };
                let bytes = crate::resource::element(format).map_or(0, |(b, _)| b as u32);
                next[slot] = offset + bytes;
                Input {
                    slot: slot as u32,
                    offset,
                    format,
                    per_instance: e.InputSlotClass == D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
                }
            })
            .collect()
    }
}

/// One input element: vertex buffer slot, byte offset, format.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Input {
    pub(crate) slot: u32,
    pub(crate) offset: u32,
    pub(crate) format: u32,
    pub(crate) per_instance: bool,
}

/// The console's own primitive topology types.
const RECTANGLES: i32 = 19;
const QUADS: i32 = 32;

/// Whether the title's root SRVs are bound as one-view tables of the
/// buffer's host copy, whose reads past its end return zeros, rather than
/// as root descriptors, which have no size: a mesh shader's vertex fetch
/// has no record limit, and a read past the copy reaches whatever the host
/// placed after it (memory it has freed: the device is removed, upstream
/// PR #493). `VICTORY_ROOT_SRV_DIRECT=1` goes back to root descriptors.
pub(crate) fn bounded_root_srvs() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("VICTORY_ROOT_SRV_DIRECT").is_none())
}

/// Which translation of its shader a stage runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Variant {
    /// The shader's own ([`Shader::translated`]).
    Title,
    /// A mesh program's compute shader ([`Shader::mesh_variant`]).
    MeshCompute,
    /// The vertex shader that draws what that compute shader wrote.
    MeshVertex,
}

fn translation(shader: &Shader, variant: Variant) -> Option<&Translated> {
    match variant {
        Variant::Title => shader.translated(),
        Variant::MeshCompute => shader.mesh_variant().map(|m| &m.compute),
        Variant::MeshVertex => shader.mesh_variant().map(|m| &m.vertex),
    }
}

/// Whether mesh pipelines run as a compute pipeline plus a vertex-shader
/// pipeline that draws what it wrote (`victory_shader::emit::mesh_compute`):
/// on a host without mesh shaders, or everywhere with
/// `VICTORY_MESH_COMPUTE=1`. `VICTORY_MESH_COMPUTE=0` refuses mesh pipelines
/// on such a host, as before (their draws are dropped).
pub(crate) fn mesh_as_compute() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        let on = match std::env::var("VICTORY_MESH_COMPUTE").as_deref() {
            Ok("1") => true,
            Ok("0") => false,
            _ => host().is_some_and(|h| !h.mesh_shaders()),
        };
        if on {
            tracing::info!("mesh pipelines run as compute plus a vertex shader");
        }
        on
    })
}

/// One stage of a host pipeline and where its bindings go.
pub(crate) struct Stage {
    pub(crate) shader: Arc<Shader>,
    /// Which of its translations it runs.
    pub(crate) variant: Variant,
    /// The root parameter of its user data, and its size in dwords.
    pub(crate) user_data: Option<(u32, u32)>,
    /// Root parameters of its root views: (title root parameter, ours).
    pub(crate) root_cbvs: Vec<(u8, u32)>,
    pub(crate) root_srvs: Vec<(u8, u32)>,
    /// Root UAVs: one-view tables like the SRVs (or root descriptors with
    /// `VICTORY_ROOT_SRV_DIRECT=1`).
    pub(crate) root_uavs: Vec<(u8, u32)>,
}

impl Stage {
    pub(crate) fn translated(&self) -> &Translated {
        translation(&self.shader, self.variant).expect("host pipelines are made from translated shaders")
    }
}

pub(crate) struct HostPipeline {
    /// A compute pipeline (else graphics).
    pub(crate) compute: bool,
    /// A mesh pipeline (graphics without vertex input).
    pub(crate) mesh: bool,
    pub(crate) pso: ID3D12PipelineState,
    pub(crate) root: ID3D12RootSignature,
    /// Vertex, then pixel stage, or the compute stage.
    pub(crate) stages: Vec<Stage>,
    /// The vertex buffer slots it reads (bit per slot).
    pub(crate) slots: u32,
    pub(crate) inputs: Vec<Input>,
    /// Has the geometry stage that completes the console's rectangles
    /// (`victory_shader::emit::rect_list_gs`): its rectangle draws go out
    /// as the title gives them, three vertices a rectangle.
    pub(crate) rects: bool,
    /// Its shaders, for reports of slow submissions.
    pub(crate) name: String,
    /// Its depth, stencil, cull and blend state in short, for the frame
    /// listing (empty for compute).
    #[cfg_attr(not(feature = "diagnostics"), allow(dead_code))]
    pub(crate) state: String,
    /// A mesh pipeline run as compute ([`mesh_as_compute`]): the compute
    /// pipeline of its mesh program, which writes each group's vertices and
    /// triangles to a record buffer; this pipeline's vertex shader draws
    /// them with the title's pixel shader (`State::dispatch_mesh`).
    pub(crate) as_compute: Option<Box<AsCompute>>,
}

pub(crate) struct AsCompute {
    pub(crate) pipeline: HostPipeline,
    pub(crate) layout: victory_shader::emit::mesh_compute::RecordLayout,
}

/// Logs a pipeline that gets no host pipeline, once per reason.
fn refuse(reason: &'static str, shaders: &[Arc<Shader>]) -> Option<HostPipeline> {
    static SEEN: std::sync::Mutex<Vec<&str>> = std::sync::Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if !seen.contains(&reason) {
        seen.push(reason);
        tracing::warn!(?shaders, "no host pipeline: {reason} (logged once)");
    }
    None
}

impl HostPipeline {
    /// A host pipeline for vertex and pixel shaders with the graphics
    /// state `g`, or for one compute shader.
    pub(crate) fn new(shaders: &[Arc<Shader>], g: Option<&GraphicsState>) -> Option<HostPipeline> {
        // For the report when the host device is removed: what this thread
        // has in the library from here on.
        super::library::label(format!("{shaders:?}"));
        let find = |stage| shaders.iter().find(|s| s.stage == stage);
        let compute = match shaders {
            [cs] if cs.stage == "compute" => Some(cs),
            _ => None,
        };
        if compute.is_none()
            && shaders
                .iter()
                .any(|s| !matches!(s.stage, "vertex" | "pixel" | "mesh" | "geometry"))
        {
            return refuse(
                "only vertex, geometry, mesh, pixel and compute stages are translated",
                shaders,
            );
        }
        let mesh = find("mesh");
        // A console geometry stage: the vertex stage then runs its container's
        // vertex variant (`victory_shader::merged`), the geometry stage after it.
        let gs = find("geometry");
        if gs.is_some() && mesh.is_some() {
            return refuse("a geometry stage after a mesh stage", shaders);
        }
        if let Some(ms) = mesh
            && compute.is_none()
            && mesh_as_compute()
        {
            return Self::mesh_as_compute(shaders, ms, find("pixel"), g);
        }
        let stages = if let Some(cs) = compute {
            vec![(cs, Variant::Title, D3D12_SHADER_VISIBILITY_ALL)]
        } else {
            let first = match (mesh, find("vertex")) {
                (Some(ms), _) => (ms, Variant::Title, D3D12_SHADER_VISIBILITY_MESH),
                (None, Some(vs)) => (vs, Variant::Title, D3D12_SHADER_VISIBILITY_VERTEX),
                (None, None) => return refuse("no vertex or mesh shader", shaders),
            };
            let mut stages = vec![first];
            stages.extend(find("pixel").map(|ps| (ps, Variant::Title, D3D12_SHADER_VISIBILITY_PIXEL)));
            stages.extend(gs.map(|gs| (gs, Variant::Title, D3D12_SHADER_VISIBILITY_GEOMETRY)));
            stages
        };
        // Where the pixel and geometry stages' code is in `stages`.
        let ps_at = stages.iter().position(|(s, _, _)| s.stage == "pixel");
        let gs_at = stages.iter().position(|(s, _, _)| s.stage == "geometry");
        if mesh.is_some() && !host()?.mesh_shaders() {
            return refuse("the host GPU has no mesh shaders (or no shader model 6.6)", shaders);
        }
        for &(s, v, _) in &stages {
            translation(s, v)?;
        }
        let host = host()?;
        let (bound, root, root_blob) = root_for(host, &stages, compute.is_some(), shaders)?;
        let code = |i: usize| {
            stages
                .get(i)
                .and_then(|&(s, v, _)| translation(s, v))
                .map_or(D3D12_SHADER_BYTECODE::default(), |t| bytecode(&t.host.dxil))
        };
        let none = D3D12_SHADER_BYTECODE::default;
        let ps_code = ps_at.map_or_else(none, &code);
        // The translated console geometry stage, if the pipeline has one.
        let real_gs = gs_at.map(&code);
        if compute.is_some() {
            let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
                // SAFETY: a borrowed pointer the description never releases.
                pRootSignature: unsafe { std::mem::transmute_copy(&root) },
                CS: code(0),
                ..Default::default()
            };
            let name = super::library::name("compute", &root_blob, &[&code_bytes(&code(0))], "");
            let made = super::library::pipeline(
                &name,
                // SAFETY: the description points at live data.
                |library, name| unsafe { library.LoadComputePipeline(name, &desc) },
                // SAFETY: as above.
                || unsafe { host.device.CreateComputePipelineState(&desc) },
            );
            let pso = match made {
                Ok(pso) => pso,
                Err(e) => {
                    let why = why_refused(host.caps(), &[&code_bytes(&code(0))]);
                    tracing::warn!(error = %e, ?shaders, why, "host pipeline refused");
                    host.log_messages();
                    return None;
                }
            };
            tracing::debug!(?shaders, "host pipeline");
            crate::host::name(&pso, || format!("{shaders:?}"));
            return Some(HostPipeline {
                compute: true,
                mesh: false,
                pso,
                root,
                stages: bound,
                name: format!("{shaders:?}"),
                slots: 0,
                inputs: Vec::new(),
                rects: false,
                state: String::new(),
                as_compute: None,
            });
        }
        let Some(g) = g else {
            return refuse("graphics state not kept (plain description)", shaders);
        };
        let state = g.describe();
        if let Some(ms) = mesh {
            // A mesh shader whose outputs pass 32 KiB runs linked to its
            // pixel shader (`super::linked`).
            let linked = super::linked(ms, find("pixel").map(|ps| &**ps));
            let bytes = |b: &Arc<Vec<u8>>| D3D12_SHADER_BYTECODE {
                pShaderBytecode: b.as_ptr().cast(),
                BytecodeLength: b.len(),
            };
            let code = |i: usize| match (&linked, i) {
                (Some((m, _)), 0) => bytes(m),
                (Some((_, Some(p))), _) => bytes(p),
                _ => code(i),
            };
            // "triangles": the stream names its topology since it did not
            // load from the library without one (see `mesh_pipeline`); a
            // new name keeps such entries, stored before, from being
            // looked up again.
            let name = super::library::name(
                "mesh triangles",
                &root_blob,
                &[&code_bytes(&code(0)), &code_bytes(&ps_code)],
                &state,
            );
            let made = if g.bounds && depth_bounds_test() {
                bounds_pipeline(&host.device, &root, [none(), none(), code(0), ps_code], g, &name)
            } else {
                mesh_pipeline(&host.device, &root, code(0), ps_code, g, &name)
            };
            let pso = match made {
                Ok(pso) => pso,
                Err(e) => {
                    tracing::warn!(error = %e, ?shaders, "host mesh pipeline refused");
                    host.log_messages();
                    return None;
                }
            };
            tracing::info!(?shaders, "host mesh pipeline");
            crate::host::name(&pso, || format!("{shaders:?}"));
            return Some(HostPipeline {
                compute: false,
                mesh: true,
                pso,
                root,
                stages: bound,
                name: format!("{shaders:?}"),
                slots: 0,
                inputs: Vec::new(),
                rects: false,
                state: String::new(),
                as_compute: None,
            });
        }

        // A pipeline the title makes for rectangles gets a geometry stage
        // that completes each one from the three vertices the vertex
        // shader made. Without it (refused here or by the host) the draws
        // fall back on what `draw` does for rectangles.
        let mut rect_gs = None;
        if g.topology.0 == RECTANGLES && real_gs.is_none() {
            let ps = ps_at.and_then(|i| translation(stages[i].0, stages[i].1));
            let vs = &stages[0].0.translated()?.host.dxil;
            match victory_shader::emit::rect_list_gs(vs, ps.map(|t| &t.host.dxil[..])) {
                Ok(gs) => rect_gs = Some(gs),
                Err(e) => tracing::warn!(error = %e, ?shaders, "no rectangle geometry stage"),
            }
        }
        let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            // SAFETY: a borrowed pointer the description never releases.
            pRootSignature: unsafe { std::mem::transmute_copy(&root) },
            VS: code(0),
            PS: ps_code,
            GS: real_gs.unwrap_or_else(|| {
                rect_gs.as_ref().map_or(D3D12_SHADER_BYTECODE::default(), |gs| {
                    D3D12_SHADER_BYTECODE {
                        pShaderBytecode: gs.as_ptr().cast(),
                        BytecodeLength: gs.len(),
                    }
                })
            }),
            BlendState: g.host_blend(),
            SampleMask: g.sample_mask,
            RasterizerState: g.rasterizer,
            DepthStencilState: g.depth,
            InputLayout: D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: g.inputs.as_ptr(),
                NumElements: g.inputs.len() as u32,
            },
            IBStripCutValue: g.strip_cut,
            // The console's rectangle and quad types draw as triangles on
            // the host (see `draw`).
            PrimitiveTopologyType: match g.topology.0 {
                RECTANGLES | QUADS => D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                _ => g.topology,
            },
            NumRenderTargets: g.targets.NumRenderTargets,
            RTVFormats: g.targets.RTFormats,
            DSVFormat: g.depth_format,
            SampleDesc: g.samples,
            ..Default::default()
        };
        // A pipeline with the geometry stage has a name of its own in the
        // library; the others keep theirs.
        let name = |gs: Option<&Vec<u8>>| match (real_gs, gs) {
            (Some(real), _) => super::library::name(
                "graphics geometry",
                &root_blob,
                &[&code_bytes(&code(0)), &code_bytes(&ps_code), &code_bytes(&real)],
                &state,
            ),
            (None, Some(gs)) => super::library::name(
                "graphics rectangles",
                &root_blob,
                &[&code_bytes(&code(0)), &code_bytes(&ps_code), &gs[..]],
                &state,
            ),
            (None, None) => super::library::name(
                "graphics",
                &root_blob,
                &[&code_bytes(&code(0)), &code_bytes(&ps_code)],
                &state,
            ),
        };
        let make = |desc: &D3D12_GRAPHICS_PIPELINE_STATE_DESC, name: &str| {
            if g.bounds && depth_bounds_test() {
                return bounds_pipeline(&host.device, &root, [desc.VS, desc.GS, none(), desc.PS], g, name);
            }
            super::library::pipeline(
                name,
                // SAFETY: the description points at live data.
                |library, name| unsafe { library.LoadGraphicsPipeline(name, desc) },
                // SAFETY: as above.
                || unsafe { host.device.CreateGraphicsPipelineState(desc) },
            )
        };
        let mut made = make(&desc, &name(rect_gs.as_ref()));
        if made.is_err() && rect_gs.is_some() {
            // The host refused the geometry stage: once more without it.
            tracing::warn!(?shaders, "host pipeline refused with the rectangle geometry stage");
            host.log_messages();
            desc.GS = D3D12_SHADER_BYTECODE::default();
            rect_gs = None;
            made = make(&desc, &name(None));
        }
        let pso = match made {
            Ok(pso) => pso,
            Err(e) => {
                let why = why_refused(host.caps(), &[&code_bytes(&desc.VS), &code_bytes(&desc.PS), &code_bytes(&desc.GS)]);
                tracing::warn!(error = %e, ?shaders, why, "host pipeline refused");
                host.log_messages();
                return None;
            }
        };
        tracing::debug!(?shaders, "host pipeline");
        crate::host::name(&pso, || format!("{shaders:?}"));
        tracing::debug!(
            ?shaders,
            depth = ?g.depth,
            dsv = g.depth_format.0,
            targets = ?&g.targets.RTFormats[..g.targets.NumRenderTargets.min(8) as usize],
            blend = ?g.blend.RenderTarget[0],
            cull = g.rasterizer.CullMode.0,
            "host graphics state"
        );
        Some(HostPipeline {
            compute: false,
            mesh: false,
            pso,
            root,
            stages: bound,
            name: format!("{shaders:?}"),
            slots: g.slots(),
            inputs: g.elements(),
            rects: rect_gs.is_some(),
            state: g.short(),
            as_compute: None,
        })
    }

    /// A mesh pipeline as two ([`mesh_as_compute`]): a compute pipeline
    /// that runs the mesh program `ms` and writes its records, and this
    /// graphics pipeline, whose vertex shader draws them as a triangle list
    /// through the pixel shader `ps` with the title's graphics state `g`.
    fn mesh_as_compute(
        shaders: &[Arc<Shader>],
        ms: &Arc<Shader>,
        ps: Option<&Arc<Shader>>,
        g: Option<&GraphicsState>,
    ) -> Option<HostPipeline> {
        let Some(g) = g else {
            return refuse("graphics state not kept (plain description)", shaders);
        };
        if let Some(ps) = ps {
            ps.translated()?;
        }
        let variant = ms.mesh_variant()?;
        let host = host()?;
        let compute = Self::mesh_compute_half(shaders, ms)?;

        let mut stages = vec![(ms, Variant::MeshVertex, D3D12_SHADER_VISIBILITY_VERTEX)];
        stages.extend(ps.map(|ps| (ps, Variant::Title, D3D12_SHADER_VISIBILITY_PIXEL)));
        let (bound, root, root_blob) = root_for(host, &stages, false, shaders)?;
        let vs = &variant.vertex.host.dxil;
        let ps_code = ps
            .and_then(|ps| ps.translated())
            .map_or(&[][..], |t| &t.host.dxil[..]);
        let desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            // SAFETY: a borrowed pointer the description never releases.
            pRootSignature: unsafe { std::mem::transmute_copy(&root) },
            VS: bytecode(vs),
            PS: if ps_code.is_empty() {
                D3D12_SHADER_BYTECODE::default()
            } else {
                bytecode(ps_code)
            },
            BlendState: g.host_blend(),
            SampleMask: g.sample_mask,
            RasterizerState: g.rasterizer,
            DepthStencilState: g.depth,
            IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
            PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            NumRenderTargets: g.targets.NumRenderTargets,
            RTVFormats: g.targets.RTFormats,
            DSVFormat: g.depth_format,
            SampleDesc: g.samples,
            ..Default::default()
        };
        let state = g.describe();
        let name = super::library::name("mesh as vertices", &root_blob, &[vs, ps_code], &state);
        let made = super::library::pipeline(
            &name,
            // SAFETY: the description points at live data.
            |library, name| unsafe { library.LoadGraphicsPipeline(name, &desc) },
            // SAFETY: as above.
            || unsafe { host.device.CreateGraphicsPipelineState(&desc) },
        );
        let pso = match made {
            Ok(pso) => pso,
            Err(e) => {
                let why = why_refused(host.caps(), &[vs, ps_code]);
                tracing::warn!(error = %e, ?shaders, why, "host pipeline drawing a mesh program's records refused");
                host.log_messages();
                return None;
            }
        };
        tracing::info!(?shaders, "host mesh pipeline as compute plus a vertex shader");
        crate::host::name(&pso, || format!("{shaders:?}"));
        Some(HostPipeline {
            compute: false,
            mesh: true,
            pso,
            root,
            stages: bound,
            name: format!("{shaders:?}"),
            slots: 0,
            inputs: Vec::new(),
            rects: false,
            state: g.short(),
            as_compute: Some(Box::new(AsCompute {
                pipeline: compute,
                layout: variant.layout,
            })),
        })
    }

    /// The compute pipeline that runs the mesh program `ms` and writes its
    /// records: the half of [`Self::mesh_as_compute`] that depends on the
    /// mesh program alone, so it can be made before a run
    /// (`crate::pipeline::prepare`). `shaders` names the pipeline in logs.
    pub(crate) fn mesh_compute_half(shaders: &[Arc<Shader>], ms: &Arc<Shader>) -> Option<HostPipeline> {
        let variant = ms.mesh_variant()?;
        let host = host()?;
        let (stages, root, blob) =
            root_for(host, &[(ms, Variant::MeshCompute, D3D12_SHADER_VISIBILITY_ALL)], true, shaders)?;
        let cs = &variant.compute.host.dxil;
        let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
            // SAFETY: a borrowed pointer the description never releases.
            pRootSignature: unsafe { std::mem::transmute_copy(&root) },
            CS: bytecode(cs),
            ..Default::default()
        };
        let name = super::library::name("mesh as compute", &blob, &[cs], "");
        let made = super::library::pipeline(
            &name,
            // SAFETY: the description points at live data.
            |library, name| unsafe { library.LoadComputePipeline(name, &desc) },
            // SAFETY: as above.
            || unsafe { host.device.CreateComputePipelineState(&desc) },
        );
        let pso = match made {
            Ok(pso) => pso,
            Err(e) => {
                tracing::warn!(error = %e, ?shaders, "host compute pipeline of a mesh program refused");
                host.log_messages();
                return None;
            }
        };
        crate::host::name(&pso, || format!("{shaders:?} as compute"));
        Some(HostPipeline {
            compute: true,
            mesh: false,
            pso,
            root,
            stages,
            name: format!("{shaders:?} as compute"),
            slots: 0,
            inputs: Vec::new(),
            rects: false,
            state: String::new(),
            as_compute: None,
        })
    }
}

/// Whether a target's blend description names the second source
/// (dual-source blending), enabled or not.
fn dual_source(t: &D3D12_RENDER_TARGET_BLEND_DESC) -> bool {
    [t.SrcBlend, t.DestBlend, t.SrcBlendAlpha, t.DestBlendAlpha].into_iter().any(|b| {
        matches!(
            b,
            D3D12_BLEND_SRC1_COLOR | D3D12_BLEND_INV_SRC1_COLOR | D3D12_BLEND_SRC1_ALPHA | D3D12_BLEND_INV_SRC1_ALPHA
        )
    })
}

/// Makes a target's blend description one that blends nothing and names
/// only the first source; its write mask stays.
fn plain(t: &mut D3D12_RENDER_TARGET_BLEND_DESC) {
    t.BlendEnable = false.into();
    (t.SrcBlend, t.DestBlend) = (D3D12_BLEND_ONE, D3D12_BLEND_ZERO);
    (t.SrcBlendAlpha, t.DestBlendAlpha) = (D3D12_BLEND_ONE, D3D12_BLEND_ZERO);
}

/// Whether a render target format is an integer one (`_UINT`, `_SINT`):
/// D3D12 blends none of them.
fn integer_format(f: DXGI_FORMAT) -> bool {
    matches!(
        f,
        DXGI_FORMAT_R32G32B32A32_UINT
            | DXGI_FORMAT_R32G32B32A32_SINT
            | DXGI_FORMAT_R32G32B32_UINT
            | DXGI_FORMAT_R32G32B32_SINT
            | DXGI_FORMAT_R16G16B16A16_UINT
            | DXGI_FORMAT_R16G16B16A16_SINT
            | DXGI_FORMAT_R32G32_UINT
            | DXGI_FORMAT_R32G32_SINT
            | DXGI_FORMAT_R10G10B10A2_UINT
            | DXGI_FORMAT_R8G8B8A8_UINT
            | DXGI_FORMAT_R8G8B8A8_SINT
            | DXGI_FORMAT_R16G16_UINT
            | DXGI_FORMAT_R16G16_SINT
            | DXGI_FORMAT_R32_UINT
            | DXGI_FORMAT_R32_SINT
            | DXGI_FORMAT_R8G8_UINT
            | DXGI_FORMAT_R8G8_SINT
            | DXGI_FORMAT_R16_UINT
            | DXGI_FORMAT_R16_SINT
            | DXGI_FORMAT_R8_UINT
            | DXGI_FORMAT_R8_SINT
    )
}

/// A mesh pipeline from the title's graphics state: the pipeline-state
/// stream form (no input layout).
///
/// The stream names a triangle topology, which D3D12 does not need for a
/// mesh shader: without it the pipeline library refuses to load a stored
/// pipeline with the very stream it was made from (`E_INVALIDARG`; debug
/// layer message 972, "does not match the one used to create the PSO"), so
/// every mesh pipeline was compiled again, and its store then refused (970,
/// "already exists"). The translation makes triangle mesh shaders only
/// (`mesh_prologue` refuses others), and the debug layer wants the two to
/// agree (1323).
fn mesh_pipeline(
    device: &ID3D12Device,
    root: &ID3D12RootSignature,
    ms: D3D12_SHADER_BYTECODE,
    ps: D3D12_SHADER_BYTECODE,
    g: &GraphicsState,
    name: &str,
) -> windows::core::Result<ID3D12PipelineState> {
    use windows::core::Interface;
    /// A stream subobject: its type, then its description, pointer-aligned.
    #[repr(C, align(8))]
    struct Sub<T>(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE, T);
    #[repr(C)]
    struct Stream {
        root: Sub<*mut std::ffi::c_void>,
        ms: Sub<D3D12_SHADER_BYTECODE>,
        ps: Sub<D3D12_SHADER_BYTECODE>,
        blend: Sub<D3D12_BLEND_DESC>,
        mask: Sub<u32>,
        raster: Sub<D3D12_RASTERIZER_DESC>,
        depth: Sub<D3D12_DEPTH_STENCIL_DESC>,
        targets: Sub<D3D12_RT_FORMAT_ARRAY>,
        dsv: Sub<DXGI_FORMAT>,
        samples: Sub<DXGI_SAMPLE_DESC>,
        topology: Sub<D3D12_PRIMITIVE_TOPOLOGY_TYPE>,
    }
    let mut stream = Stream {
        root: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_ROOT_SIGNATURE, root.as_raw()),
        ms: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_MS, ms),
        ps: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PS, ps),
        blend: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_BLEND, g.host_blend()),
        mask: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_MASK, g.sample_mask),
        raster: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER, g.rasterizer),
        depth: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL, g.depth),
        targets: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RENDER_TARGET_FORMATS, g.targets),
        dsv: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL_FORMAT, g.depth_format),
        samples: Sub(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_DESC, g.samples),
        topology: Sub(
            D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PRIMITIVE_TOPOLOGY,
            D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        ),
    };
    let desc = D3D12_PIPELINE_STATE_STREAM_DESC {
        SizeInBytes: size_of::<Stream>(),
        pPipelineStateSubobjectStream: (&raw mut stream).cast(),
    };
    let device = device.cast::<ID3D12Device2>()?;
    super::library::pipeline(
        name,
        // SAFETY: the stream holds whole subobjects pointing at live data.
        |library, name| unsafe { library.LoadPipeline(name, &desc) },
        // SAFETY: as above.
        || unsafe { device.CreatePipelineState(&desc) },
    )
}

/// The root signature the translated shaders of `stages` define with their
/// bindings (per stage one block of root constants for the user data, root
/// views, static samplers), and where each stage's bindings went.
fn root_for(
    host: &crate::host::Host,
    stages: &[(&Arc<Shader>, Variant, D3D12_SHADER_VISIBILITY)],
    compute: bool,
    shaders: &[Arc<Shader>],
) -> Option<(Vec<Stage>, ID3D12RootSignature, Vec<u8>)> {
    // The root signature the translated shaders' bindings define.
    let mut params: Vec<D3D12_ROOT_PARAMETER1> = Vec::new();
    // The ranges table parameters point at, alive until the root
    // signature is made.
    let mut ranges: Vec<Box<D3D12_DESCRIPTOR_RANGE1>> = Vec::new();
    let mut samplers = Vec::new();
    let mut bound = Vec::new();
    for &(shader, variant, visibility) in stages {
        let t = &translation(shader, variant)?.host;
        let mut stage = Stage {
            shader: shader.clone(),
            variant,
            user_data: None,
            root_cbvs: Vec::new(),
            root_srvs: Vec::new(),
            root_uavs: Vec::new(),
        };
        let dwords = t.user_data_dwords();
        if dwords > 0 {
            stage.user_data = Some((params.len() as u32, dwords));
            params.push(D3D12_ROOT_PARAMETER1 {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                Anonymous: D3D12_ROOT_PARAMETER1_0 {
                    Constants: D3D12_ROOT_CONSTANTS {
                        ShaderRegister: 0,
                        RegisterSpace: USER_DATA_SPACE,
                        Num32BitValues: dwords,
                    },
                },
                ShaderVisibility: visibility,
            });
        }
        for (slots, kind, out) in [
            (
                &t.root_cbvs,
                D3D12_ROOT_PARAMETER_TYPE_CBV,
                &mut stage.root_cbvs,
            ),
            (
                &t.root_srvs,
                D3D12_ROOT_PARAMETER_TYPE_SRV,
                &mut stage.root_srvs,
            ),
            (
                &t.root_uavs,
                D3D12_ROOT_PARAMETER_TYPE_UAV,
                &mut stage.root_uavs,
            ),
        ] {
            for &slot in slots {
                let uav = kind == D3D12_ROOT_PARAMETER_TYPE_UAV;
                // Ours, not the title's, stay root descriptors: the
                // table of strides, the directory of title memory by
                // address and its fault notes, a mesh program's records.
                let ours = slot == victory_shader::emit::mesh_compute::SLOT_MESH_RECORDS
                    || if uav {
                        slot == victory_shader::emit::SLOT_FAULTS
                    } else {
                        slot == victory_shader::emit::SLOT_STRIDES
                            || slot == victory_shader::emit::SLOT_WORDS
                            || slot == victory_shader::emit::SLOT_MEMORY
                    };
                if (uav || kind == D3D12_ROOT_PARAMETER_TYPE_SRV) && !ours && bounded_root_srvs() {
                    // A one-view table: the view's size bounds the
                    // reads (and a root UAV's writes), which a root
                    // descriptor leaves unbounded.
                    // (Our own table of strides stays a descriptor.)
                    ranges.push(Box::new(D3D12_DESCRIPTOR_RANGE1 {
                        RangeType: if uav {
                            D3D12_DESCRIPTOR_RANGE_TYPE_UAV
                        } else {
                            D3D12_DESCRIPTOR_RANGE_TYPE_SRV
                        },
                        NumDescriptors: 1,
                        BaseShaderRegister: u32::from(slot),
                        RegisterSpace: ROOT_CBV_SPACE,
                        Flags: D3D12_DESCRIPTOR_RANGE_FLAG_DESCRIPTORS_VOLATILE
                            | D3D12_DESCRIPTOR_RANGE_FLAG_DATA_VOLATILE,
                        OffsetInDescriptorsFromTableStart: 0,
                    }));
                    let range: *const D3D12_DESCRIPTOR_RANGE1 = &**ranges.last()?;
                    out.push((slot, params.len() as u32));
                    params.push(D3D12_ROOT_PARAMETER1 {
                        ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                        Anonymous: D3D12_ROOT_PARAMETER1_0 {
                            DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE1 {
                                NumDescriptorRanges: 1,
                                pDescriptorRanges: range,
                            },
                        },
                        ShaderVisibility: visibility,
                    });
                    continue;
                }
                out.push((slot, params.len() as u32));
                params.push(D3D12_ROOT_PARAMETER1 {
                    ParameterType: kind,
                    Anonymous: D3D12_ROOT_PARAMETER1_0 {
                        Descriptor: D3D12_ROOT_DESCRIPTOR1 {
                            ShaderRegister: u32::from(slot),
                            RegisterSpace: ROOT_CBV_SPACE,
                            Flags: D3D12_ROOT_DESCRIPTOR_FLAG_DATA_VOLATILE,
                        },
                    },
                    ShaderVisibility: visibility,
                });
            }
        }
        for (n, s) in t.static_samplers.iter().enumerate() {
            samplers.push(D3D12_STATIC_SAMPLER_DESC {
                Filter: D3D12_FILTER(s.filter as i32),
                AddressU: D3D12_TEXTURE_ADDRESS_MODE(s.address[0] as i32),
                AddressV: D3D12_TEXTURE_ADDRESS_MODE(s.address[1] as i32),
                AddressW: D3D12_TEXTURE_ADDRESS_MODE(s.address[2] as i32),
                MipLODBias: s.mip_lod_bias,
                MaxAnisotropy: s.max_anisotropy,
                ComparisonFunc: D3D12_COMPARISON_FUNC(s.comparison as i32),
                BorderColor: D3D12_STATIC_BORDER_COLOR(s.border as i32),
                MinLOD: s.min_lod,
                MaxLOD: s.max_lod,
                ShaderRegister: n as u32,
                RegisterSpace: SAMPLER_SPACE,
                ShaderVisibility: visibility,
            });
        }
        bound.push(stage);
    }
    match root_signature(&host.device, &params, &samplers, compute) {
        Ok((root, blob)) => Some((bound, root, blob)),
        Err(e) => {
            tracing::warn!(error = %e, ?shaders, "host root signature refused");
            None
        }
    }
}

fn bytecode(code: &[u8]) -> D3D12_SHADER_BYTECODE {
    D3D12_SHADER_BYTECODE {
        pShaderBytecode: code.as_ptr().cast(),
        BytecodeLength: code.len(),
    }
}

/// Whether the host has the depth bounds test (said once if not).
/// `VICTORY_DEPTH_BOUNDS=0` goes without it, as before.
pub(crate) fn depth_bounds_test() -> bool {
    static HAS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HAS.get_or_init(|| {
        if std::env::var_os("VICTORY_DEPTH_BOUNDS").is_some_and(|v| v == "0") {
            return false;
        }
        let mut options = D3D12_FEATURE_DATA_D3D12_OPTIONS2::default();
        // SAFETY: the description matches the feature asked about.
        let has = host().is_some_and(|host| unsafe {
            host.device
                .CheckFeatureSupport(
                    D3D12_FEATURE_D3D12_OPTIONS2,
                    (&raw mut options).cast(),
                    size_of_val(&options) as u32,
                )
                .is_ok()
        }) && options.DepthBoundsTestSupported.as_bool();
        if !has {
            tracing::warn!("no depth bounds test on this host: draws that ask for it pass at every depth");
        }
        has
    })
}

/// A graphics pipeline (vertex and geometry, or mesh, then pixel code) with
/// the depth bounds test, which only the stream form can ask for.
fn bounds_pipeline(
    device: &ID3D12Device,
    root: &ID3D12RootSignature,
    [vs, gs, ms, ps]: [D3D12_SHADER_BYTECODE; 4],
    g: &GraphicsState,
    name: &str,
) -> windows::core::Result<ID3D12PipelineState> {
    use windows::core::Interface;
    /// A stream subobject: its type, then its description, pointer-aligned.
    #[repr(C, align(8))]
    struct Sub<T>(D3D12_PIPELINE_STATE_SUBOBJECT_TYPE, T);
    /// Appends one to the stream, which is whole words.
    fn add<T>(stream: &mut Vec<u64>, kind: D3D12_PIPELINE_STATE_SUBOBJECT_TYPE, value: T) {
        let words = size_of::<Sub<T>>() / 8;
        let at = stream.len();
        stream.resize(at + words, 0);
        // SAFETY: room for one subobject, whose alignment is the words';
        // the descriptions are plain data (nothing is dropped).
        unsafe { stream.as_mut_ptr().add(at).cast::<Sub<T>>().write(Sub(kind, value)) };
    }
    let mut stream = Vec::new();
    let s = &mut stream;
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_ROOT_SIGNATURE, root.as_raw());
    for (kind, code) in [
        (D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_VS, vs),
        (D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_GS, gs),
        (D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_MS, ms),
        (D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PS, ps),
    ] {
        if !code.pShaderBytecode.is_null() {
            add(s, kind, code);
        }
    }
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_BLEND, g.host_blend());
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_MASK, g.sample_mask);
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RASTERIZER, g.rasterizer);
    let d = &g.depth;
    let depth = D3D12_DEPTH_STENCIL_DESC1 {
        DepthEnable: d.DepthEnable,
        DepthWriteMask: d.DepthWriteMask,
        DepthFunc: d.DepthFunc,
        StencilEnable: d.StencilEnable,
        StencilReadMask: d.StencilReadMask,
        StencilWriteMask: d.StencilWriteMask,
        FrontFace: d.FrontFace,
        BackFace: d.BackFace,
        DepthBoundsTestEnable: true.into(),
    };
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL1, depth);
    if ms.pShaderBytecode.is_null() {
        let inputs = D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: g.inputs.as_ptr(),
            NumElements: g.inputs.len() as u32,
        };
        add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_INPUT_LAYOUT, inputs);
        add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_IB_STRIP_CUT_VALUE, g.strip_cut);
    }
    let topology = match g.topology.0 {
        _ if !ms.pShaderBytecode.is_null() => D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        RECTANGLES | QUADS => D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        _ => g.topology,
    };
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_PRIMITIVE_TOPOLOGY, topology);
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_RENDER_TARGET_FORMATS, g.targets);
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_DEPTH_STENCIL_FORMAT, g.depth_format);
    add(s, D3D12_PIPELINE_STATE_SUBOBJECT_TYPE_SAMPLE_DESC, g.samples);
    let desc = D3D12_PIPELINE_STATE_STREAM_DESC {
        SizeInBytes: stream.len() * 8,
        pPipelineStateSubobjectStream: stream.as_mut_ptr().cast(),
    };
    let device = device.cast::<ID3D12Device2>()?;
    super::library::pipeline(
        name,
        // SAFETY: the stream holds whole subobjects pointing at live data.
        |library, name| unsafe { library.LoadPipeline(name, &desc) },
        // SAFETY: as above.
        || unsafe { device.CreatePipelineState(&desc) },
    )
}

/// The bytes of shader code (none for an absent stage).
/// Why the host may have refused a pipeline of these programs (DXIL
/// containers), where a capability it lacks tells; said with the refusal.
fn why_refused(caps: crate::host::Caps, programs: &[&[u8]]) -> &'static str {
    if !caps.barycentrics && programs.iter().any(|p| reads_barycentrics(p)) {
        "a program reads barycentrics (SV_Barycentrics) and the card has none (D3D12_OPTIONS3.BarycentricsSupported is false)"
    } else {
        "no capability the card lacks explains it; the debug layer of a diagnostics build names the cause (VICTORY_D3D12_DEBUG=1)"
    }
}

/// Whether the feature flags of a DXIL container (its `SFI0` part) say
/// that its program reads barycentrics.
fn reads_barycentrics(container: &[u8]) -> bool {
    /// `D3D_SHADER_REQUIRES_BARYCENTRICS`.
    const BARYCENTRICS: u64 = 0x2_0000;
    let word = |at: usize| container.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    if container.get(..4) != Some(b"DXBC") {
        return false;
    }
    // The header: 4 characters, a 16-byte hash, a version, the size, the
    // part count and then each part's offset; a part is 4 characters, its
    // size and its bytes.
    let Some(parts) = word(28) else { return false };
    (0..parts.min(64)).any(|i| {
        let Some(at) = word(32 + 4 * i) else { return false };
        container.get(at..at + 4) == Some(b"SFI0")
            && container
                .get(at + 8..at + 16)
                .is_some_and(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8])) & BARYCENTRICS != 0)
    })
}

fn code_bytes(code: &D3D12_SHADER_BYTECODE) -> Vec<u8> {
    if code.pShaderBytecode.is_null() {
        return Vec::new();
    }
    // SAFETY: a translation's own bytes, alive while its shader is.
    unsafe { std::slice::from_raw_parts(code.pShaderBytecode.cast::<u8>(), code.BytecodeLength) }.to_vec()
}

fn root_signature(
    device: &ID3D12Device,
    params: &[D3D12_ROOT_PARAMETER1],
    samplers: &[D3D12_STATIC_SAMPLER_DESC],
    compute: bool,
) -> windows::core::Result<(ID3D12RootSignature, Vec<u8>)> {
    let input = if compute {
        D3D12_ROOT_SIGNATURE_FLAG_NONE
    } else {
        D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT
    };
    let desc = D3D12_VERSIONED_ROOT_SIGNATURE_DESC {
        Version: windows::Win32::Graphics::Direct3D12::D3D_ROOT_SIGNATURE_VERSION_1_1,
        Anonymous: D3D12_VERSIONED_ROOT_SIGNATURE_DESC_0 {
            Desc_1_1: D3D12_ROOT_SIGNATURE_DESC1 {
                NumParameters: params.len() as u32,
                pParameters: params.as_ptr(),
                NumStaticSamplers: samplers.len() as u32,
                pStaticSamplers: samplers.as_ptr(),
                Flags: input
                    | D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED
                    | D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED,
            },
        },
    };
    // SAFETY: the description points at live arrays; the blob is read in
    // full before it is released.
    unsafe {
        let mut blob: Option<ID3DBlob> = None;
        let mut error: Option<ID3DBlob> = None;
        if let Err(e) = D3D12SerializeVersionedRootSignature(&desc, &mut blob, Some(&mut error)) {
            if let Some(error) = error {
                let text = std::slice::from_raw_parts(
                    error.GetBufferPointer().cast::<u8>(),
                    error.GetBufferSize(),
                );
                tracing::warn!("root signature: {}", String::from_utf8_lossy(text));
            }
            return Err(e);
        }
        let blob = blob.ok_or_else(windows::core::Error::empty)?;
        let bytes =
            std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize());
        Ok((device.CreateRootSignature(0, bytes)?, bytes.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;

    use super::*;

    fn blending(targets: &[DXGI_FORMAT], independent: bool) -> GraphicsState {
        let mut g = GraphicsState::default();
        g.blend.IndependentBlendEnable = independent.into();
        for t in &mut g.blend.RenderTarget {
            t.BlendEnable = true.into();
            t.SrcBlend = D3D12_BLEND_SRC_ALPHA;
        }
        g.targets.NumRenderTargets = targets.len() as u32;
        g.targets.RTFormats[..targets.len()].copy_from_slice(targets);
        g
    }

    #[test]
    fn integer_targets_do_not_blend() {
        let formats = [
            DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_R16G16B16A16_FLOAT,
            DXGI_FORMAT_R8_UINT,
            DXGI_FORMAT_R32_SINT,
        ];
        for independent in [false, true] {
            let b = blending(&formats, independent).host_blend();
            assert!(b.IndependentBlendEnable.as_bool());
            let on: Vec<bool> = (0..4)
                .map(|i| b.RenderTarget[i].BlendEnable.as_bool())
                .collect();
            assert_eq!(on, [true, true, false, false], "independent {independent}");
            // Everything else of the shared description is kept.
            assert!((0..4).all(|i| b.RenderTarget[i].SrcBlend == D3D12_BLEND_SRC_ALPHA));
        }
    }

    #[test]
    fn float_targets_keep_the_title_blend() {
        let g = blending(
            &[DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R11G11B10_FLOAT],
            false,
        );
        let b = g.host_blend();
        assert!(!b.IndependentBlendEnable.as_bool());
        assert!(b.RenderTarget[0].BlendEnable.as_bool());
    }

    /// Blending with the second source stays on the first target and goes
    /// from the others, which D3D12 refuses it on: also from the entries of
    /// targets the pipeline does not have, where the title's state repeats
    /// the first one's.
    #[test]
    fn dual_source_blending_stays_on_the_first_target_alone() {
        let dual = |independent: bool, targets: usize| {
            let mut g = blending(&[DXGI_FORMAT_R8G8B8A8_UNORM; 8][..targets], independent);
            for t in &mut g.blend.RenderTarget {
                t.DestBlend = D3D12_BLEND_INV_SRC1_COLOR;
                t.DestBlendAlpha = D3D12_BLEND_INV_SRC1_ALPHA;
            }
            g
        };
        // One target, one description for all: what D3D12 takes as it is.
        let b = dual(false, 1).host_blend();
        assert!(!b.IndependentBlendEnable.as_bool());
        assert_eq!(b.RenderTarget[0].DestBlend, D3D12_BLEND_INV_SRC1_COLOR);
        // One target, each entry its own (the title's decoded state): the
        // first keeps its blending, the seven others name the first source.
        let b = dual(true, 1).host_blend();
        assert!(b.RenderTarget[0].BlendEnable.as_bool());
        assert_eq!(b.RenderTarget[0].DestBlend, D3D12_BLEND_INV_SRC1_COLOR);
        for t in &b.RenderTarget[1..] {
            assert!(!t.BlendEnable.as_bool());
            assert_eq!((t.SrcBlend, t.DestBlend), (D3D12_BLEND_ONE, D3D12_BLEND_ZERO));
            assert_eq!((t.SrcBlendAlpha, t.DestBlendAlpha), (D3D12_BLEND_ONE, D3D12_BLEND_ZERO));
        }
        // Three targets with one description: the same, each its own now.
        let b = dual(false, 3).host_blend();
        assert!(b.IndependentBlendEnable.as_bool());
        assert!(b.RenderTarget[0].BlendEnable.as_bool());
        assert!(b.RenderTarget[1..].iter().all(|t| !t.BlendEnable.as_bool() && !dual_source(t)));
        // Blending without the second source on several targets is left alone.
        let g = blending(&[DXGI_FORMAT_R8G8B8A8_UNORM; 3], true);
        let b = g.host_blend();
        assert!(b.RenderTarget[..3].iter().all(|t| t.BlendEnable.as_bool()));
    }

    #[test]
    fn integer_targets_without_blending_are_unchanged() {
        let mut g = blending(&[DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8_UINT], true);
        g.blend.RenderTarget[1].BlendEnable = false.into();
        let b = g.host_blend();
        assert!(b.RenderTarget[0].BlendEnable.as_bool());
        assert!(!b.RenderTarget[1].BlendEnable.as_bool());
    }

    /// A triangle mesh shader of our own and a pixel shader for it.
    const MS: &str = "struct V { float4 p : SV_Position; };
        [outputtopology(\"triangle\")] [numthreads(1, 1, 1)]
        void main(out vertices V v[3], out indices uint3 t[1]) {
            SetMeshOutputCounts(3, 1);
            v[0].p = float4(0, 0, 0, 1);
            v[1].p = float4(1, 0, 0, 1);
            v[2].p = float4(0, 1, 0, 1);
            t[0] = uint3(0, 1, 2);
        }";
    const PS: &str = "float4 main() : SV_Target { return 1; }";

    /// The PC `dxc.exe` (`VICTORY_PC_DXC`, else the newest Windows SDK's).
    fn dxc() -> Option<PathBuf> {
        let dxc = match std::env::var_os("VICTORY_PC_DXC") {
            Some(p) => PathBuf::from(p),
            None => victory_shader::emit::validator::Validator::default_path()?
                .with_file_name("dxc.exe"),
        };
        dxc.exists().then_some(dxc)
    }

    fn compile(dxc: &Path, dir: &Path, src: &str, profile: &str) -> Vec<u8> {
        let (hlsl, out) = (
            dir.join(format!("{profile}.hlsl")),
            dir.join(format!("{profile}.dxil")),
        );
        std::fs::write(&hlsl, src).unwrap();
        let status = std::process::Command::new(dxc)
            .args(["-nologo", "-T", profile, "-E", "main", "-Fo"])
            .arg(&out)
            .arg(&hlsl)
            .status()
            .unwrap();
        assert!(status.success(), "dxc failed for {profile}");
        std::fs::read(out).unwrap()
    }

    fn code(bytes: &[u8]) -> D3D12_SHADER_BYTECODE {
        D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytes.as_ptr().cast(),
            BytecodeLength: bytes.len(),
        }
    }

    /// A mesh pipeline made once is loaded from the library the next time,
    /// not compiled again (a stream without a topology never loads).
    /// A pipeline with the depth bounds test is made, and loads again.
    #[test]
    fn a_pipeline_with_depth_bounds_is_made() {
        let Some(host) = host().filter(|_| super::depth_bounds_test()) else {
            eprintln!("no host depth bounds test; skipped");
            return;
        };
        use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
        use windows::core::s;
        let source = b"float4 main(uint id : SV_VertexID) : SV_Position { return float4(id, id, 0.5, 1); }";
        let mut blob: Option<ID3DBlob> = None;
        // SAFETY: the arguments are live and `blob` is the out pointer.
        unsafe {
            D3DCompile(source.as_ptr().cast(), source.len(), PCSTR::null(), None, None, s!("main"), s!("vs_5_1"), 0, 0, &mut blob, None)
        }
        .expect("compiles");
        let blob = blob.expect("code");
        // SAFETY: a live blob.
        let vs = unsafe {
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: blob.GetBufferPointer(),
                BytecodeLength: blob.GetBufferSize(),
            }
        };
        let none = D3D12_SHADER_BYTECODE::default;
        let (root, root_blob) = root_signature(&host.device, &[], &[], false).unwrap();
        let mut g = GraphicsState::default();
        let plain = g.describe();
        g.bounds = true;
        assert_ne!(plain, g.describe());
        g.topology = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE;
        g.depth_format = DXGI_FORMAT_D32_FLOAT;
        let name = super::super::library::name("bounds test", &root_blob, &[&code_bytes(&vs)], &g.describe());
        for _ in 0..2 {
            bounds_pipeline(&host.device, &root, [vs, none(), none(), none()], &g, &name).expect("made");
        }
    }

    #[test]
    fn a_mesh_pipeline_loads_from_the_library() {
        let Some(host) = host() else {
            return; // no host GPU here
        };
        if !host.mesh_shaders() {
            eprintln!("no host mesh shader support; skipped");
            return;
        }
        let Some(dxc) = dxc() else {
            eprintln!("no PC dxc.exe; skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let ms = compile(&dxc, dir.path(), MS, "ms_6_5");
        let ps = compile(&dxc, dir.path(), PS, "ps_6_5");
        let (root, root_blob) = root_signature(&host.device, &[], &[], false).unwrap();
        let mut g = GraphicsState::default();
        // This pipeline has only a color target, with no depth buffer.
        g.depth.DepthEnable = false.into();
        g.targets.NumRenderTargets = 1;
        g.targets.RTFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
        let name = super::super::library::name("mesh test", &root_blob, &[&ms, &ps], &g.describe());
        mesh_pipeline(&host.device, &root, code(&ms), code(&ps), &g, &name).expect("made");
        let loaded = super::super::library::LOADED.load(Ordering::Relaxed);
        mesh_pipeline(&host.device, &root, code(&ms), code(&ps), &g, &name).expect("made again");
        assert!(
            super::super::library::LOADED.load(Ordering::Relaxed) > loaded,
            "the mesh pipeline was compiled again instead of loaded from the library"
        );
    }

    /// A DXIL container of one `SFI0` part holding `flags`.
    fn container_with_features(flags: u64) -> Vec<u8> {
        let mut c = b"DXBC".to_vec();
        c.extend([0u8; 16]); // hash
        c.extend(1u32.to_le_bytes()); // version
        c.extend(52u32.to_le_bytes()); // size
        c.extend(1u32.to_le_bytes()); // parts
        c.extend(36u32.to_le_bytes()); // the part's offset
        c.extend(b"SFI0");
        c.extend(8u32.to_le_bytes());
        c.extend(flags.to_le_bytes());
        c
    }

    #[test]
    fn a_refusal_names_the_capability_the_card_lacks() {
        let plain = container_with_features(0x4000); // wave operations
        let reads = container_with_features(0x2_0000 | 0x4000);
        assert!(!reads_barycentrics(&plain));
        assert!(reads_barycentrics(&reads));
        assert!(!reads_barycentrics(b"DXBC"), "a container cut short");
        assert!(!reads_barycentrics(&[]));
        let without = crate::host::Caps::default();
        let with = crate::host::Caps { barycentrics: true, ..without };
        assert!(why_refused(without, &[&plain, &reads]).contains("SV_Barycentrics"));
        assert!(!why_refused(without, &[&plain]).contains("SV_Barycentrics"));
        assert!(!why_refused(with, &[&reads]).contains("SV_Barycentrics"), "a card that has them refused for another reason");
    }
}
