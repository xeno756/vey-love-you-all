//! Mesh programs on a host without mesh shaders: the console's mesh program
//! runs as a compute shader that writes what each thread group outputs to a
//! record buffer, and a generated vertex shader draws the records with the
//! title's pixel shader unchanged.
//!
//! A group's record, at `group * group_bytes` from the start of the buffer
//! the host binds (the root UAV `u<SLOT_MESH_RECORDS>` of the compute shader,
//! the root SRV `t<SLOT_MESH_RECORDS>` of the vertex shader, both in
//! [`ROOT_CBV_SPACE`], each bound at the first record of the dispatch):
//!
//! - `max_vertices` vertices of `vertex_bytes` each: output element `i` of
//!   the mesh program's output signature in dwords `4i..4i + 4` (column `c`
//!   at `4i + c`), the raw 32 bits the program exported;
//! - then `max_primitives` dwords, one per primitive: three 10-bit vertex
//!   indices (bits 0-9, 10-19, 20-29), or [`CULLED`] for a primitive culled or
//!   never exported.
//!
//! The compute shader's group `g` (`SV_GroupID.x`) writes record `g`; the
//! title's code sees group `g + base`, `base` the user-data dword of kind
//! [`USER_MESH_GROUP_BASE`], so a draw split into several dispatches keeps the
//! title's group numbers. The vertex shader draws `groups * max_primitives *
//! 3` vertices from vertex 0: vertex `v` is corner `v % 3` of primitive
//! `v / 3 % max_primitives` of record `v / (3 * max_primitives)`.

use super::container::SigElement;
use super::HostShader;
use crate::ShaderError;

/// The root UAV (compute) and root SRV (vertex shader) of the record buffer,
/// in [`ROOT_CBV_SPACE`]: next to the translator's own slots
/// (`SLOT_FAULTS` 253 up).
pub const SLOT_MESH_RECORDS: u8 = 252;

/// `RootType::Other` code of a user-data dword the host computes: the
/// console group number of the dispatch's first group.
pub const USER_MESH_GROUP_BASE: u32 = 0x104;

/// A primitive word of a culled primitive (bit 31, as the console's own).
pub const CULLED: u32 = 1 << 31;

/// Where things are in a group's record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordLayout {
    /// Output elements per vertex, each four dwords.
    pub vertex_slots: u32,
    pub max_vertices: u32,
    pub max_primitives: u32,
}

impl RecordLayout {
    pub const fn vertex_bytes(&self) -> u32 {
        self.vertex_slots * 16
    }

    /// The byte offset of the first primitive word in a record.
    pub const fn primitives_at(&self) -> u32 {
        self.max_vertices * self.vertex_bytes()
    }

    pub const fn group_bytes(&self) -> u32 {
        self.primitives_at() + self.max_primitives * 4
    }

    /// The byte offset of column `col` of element `slot` of vertex `vertex`
    /// of group `group`.
    pub const fn vertex_at(&self, group: u32, vertex: u32, slot: u32, col: u32) -> u64 {
        group as u64 * self.group_bytes() as u64
            + (vertex * self.vertex_bytes() + slot * 16 + col * 4) as u64
    }

    /// The byte offset of primitive `primitive`'s word in group `group`.
    pub const fn primitive_at(&self, group: u32, primitive: u32) -> u64 {
        group as u64 * self.group_bytes() as u64 + (self.primitives_at() + primitive * 4) as u64
    }

    /// Vertices the vertex shader draws for `groups` groups.
    pub const fn draw_vertices(&self, groups: u32) -> u32 {
        groups * self.max_primitives * 3
    }

    /// Groups whose records fit in `bytes`.
    pub const fn groups_in(&self, bytes: u64) -> u64 {
        bytes / self.group_bytes() as u64
    }
}

/// A primitive word: `CULLED`, or the three indices packed.
pub const fn primitive_word(culled: bool, indices: [u32; 3]) -> u32 {
    if culled {
        CULLED
    } else {
        (indices[0] & 0x3FF) | (indices[1] & 0x3FF) << 10 | (indices[2] & 0x3FF) << 20
    }
}

/// A console mesh program as a compute shader.
#[derive(Debug, Clone)]
pub struct MeshCompute {
    /// The compute shader: `cs_6_6`, the mesh program's thread-group size.
    pub cs: HostShader,
    pub layout: RecordLayout,
    /// The mesh program's output signature, which the vertex shader declares.
    pub(crate) outputs: Vec<SigElement>,
}

thread_local! {
    /// Set while a mesh program is lowered as compute.
    static AS_COMPUTE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// What the last mesh program lowered as compute on this thread wrote.
    static LAST: std::cell::RefCell<Option<(RecordLayout, Vec<SigElement>)>> =
        const { std::cell::RefCell::new(None) };
}

/// Whether the mesh program being lowered on this thread is lowered as compute.
pub(crate) fn as_compute() -> bool {
    AS_COMPUTE.get()
}

/// Lowering of a mesh program as compute finished with this layout and these outputs.
pub(crate) fn note(layout: RecordLayout, outputs: Vec<SigElement>) {
    LAST.set(Some((layout, outputs)));
}

/// Translates the mesh program of a console shader container to a compute
/// shader that writes the records described in the module documentation.
/// Programs with per-primitive outputs are refused (the title's have none).
pub fn translate_mesh_as_compute(container: &[u8]) -> Result<MeshCompute, ShaderError> {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            AS_COMPUTE.set(false);
        }
    }
    let c = crate::dxbc::Container::parse(container)?;
    let dxil = c
        .part(crate::dxbc::FourCC::DXIL)
        .ok_or(ShaderError::Malformed("no DXIL part"))?;
    if !crate::native_programs(dxil.data)?
        .iter()
        .any(|p| p.stage == crate::native::HwStage::Ms)
    {
        return Err(ShaderError::Unsupported {
            offset: 0,
            what: "a container without a mesh program",
        });
    }
    LAST.set(None);
    AS_COMPUTE.set(true);
    let _reset = Reset;
    let cs = super::translate(container)?;
    let (layout, outputs) = LAST.take().ok_or(ShaderError::Unsupported {
        offset: 0,
        what: "a mesh program that did not lower as compute",
    })?;
    Ok(MeshCompute {
        cs,
        layout,
        outputs,
    })
}

impl MeshCompute {
    /// The vertex shader that draws this program's records (see the module
    /// documentation): its outputs are the mesh program's, register for
    /// register, so the title's pixel shader links with it unchanged.
    pub fn vertex_shader(&self) -> Result<HostShader, ShaderError> {
        vertex_shader(self.layout, &self.outputs)
    }
}

/// Where a culled primitive's corners go: one point outside the clip
/// volume, so the triangle has no area and is clipped anyway.
const CULLED_POSITION: [f32; 4] = [2.0, 2.0, 0.5, 1.0];

fn vertex_shader(layout: RecordLayout, outputs: &[SigElement]) -> Result<HostShader, ShaderError> {
    use super::bitcode::{Attr, Bin, Cast, Term, V, pred};
    let unsupported = |what| ShaderError::Unsupported { offset: 0, what };
    if outputs.iter().any(|e| e.row.is_none()) {
        return Err(unsupported("a mesh output without a register"));
    }
    let Some(position) = outputs.iter().position(|e| e.kind == 3 && e.comp == 9 && e.cols == 4)
    else {
        return Err(unsupported("a mesh program without a four-component position"));
    };
    if layout.vertex_slots as usize != outputs.len() || layout.max_primitives == 0 {
        return Err(unsupported("mesh records that do not match the outputs"));
    }
    let mut s = super::shader::Shader::new(false);
    // Interpolation as `emit_program` declares a vertex shader's outputs.
    s.outputs = outputs
        .iter()
        .map(|e| SigElement {
            interp: match (e.kind, e.comp) {
                (3, _) => 4,
                (0 | 6 | 7, 9) => 2,
                (0, _) => 1,
                _ => 0,
            },
            usage: (1 << e.cols) - 1,
            ..e.clone()
        })
        .collect();
    s.inputs = vec![SigElement {
        name: "SV_VertexID".into(),
        index: 0,
        kind: 1,
        comp: 5,
        interp: 0,
        row: Some(0),
        col: 0,
        cols: 1,
        usage: 1,
        half: false,
    }];
    let b = s.entry;
    let i32t = s.t.i32;
    let k = |s: &mut super::shader::Shader, v: u32| s.m.i32(v);
    let iu = s.m.undef(i32t);
    let args = [k(&mut s, 0), k(&mut s, 0), s.m.i8(0), iu];
    let vertex = s.op(b, "loadInput.i32", Attr::ReadNone, i32t, 4, &args);
    // Record, primitive and corner of this vertex.
    let per_record = k(&mut s, 3 * layout.max_primitives);
    let record = s.m.bin(b, Bin::UDiv, vertex, per_record);
    let within = s.m.bin(b, Bin::URem, vertex, per_record);
    let three = k(&mut s, 3);
    let primitive = s.m.bin(b, Bin::UDiv, within, three);
    let corner = s.m.bin(b, Bin::URem, within, three);
    let group_bytes = k(&mut s, layout.group_bytes());
    let base = s.m.bin(b, Bin::Mul, record, group_bytes);
    let records = s.root_srv_handle(SLOT_MESH_RECORDS);
    let align = k(&mut s, 4);
    let load = |s: &mut super::shader::Shader, at: V, mask: u8| {
        let mask = s.m.i8(mask);
        let res = s.t.res_ret;
        s.op(b, "rawBufferLoad.f32", Attr::ReadOnly, res, 139, &[records, at, iu, mask, align])
    };
    // The primitive word, and this corner's index in it.
    let four = k(&mut s, 4);
    let word_at = s.m.bin(b, Bin::Mul, primitive, four);
    let prims_at = k(&mut s, layout.primitives_at());
    let word_at = s.m.bin(b, Bin::Add, word_at, prims_at);
    let word_at = s.m.bin(b, Bin::Add, word_at, base);
    let word = load(&mut s, word_at, 1);
    let word = s.m.extract(b, word, 0);
    let word = s.m.cast(b, Cast::Bitcast, word, i32t);
    let cull_bit = k(&mut s, CULLED);
    let cull = s.m.bin(b, Bin::And, word, cull_bit);
    let zero = k(&mut s, 0);
    let culled = s.m.cmp(b, pred::NE, cull, zero);
    let ten = k(&mut s, 10);
    let shift = s.m.bin(b, Bin::Mul, corner, ten);
    let index = s.m.bin(b, Bin::LShr, word, shift);
    let low = k(&mut s, 0x3FF);
    let index = s.m.bin(b, Bin::And, index, low);
    let vertex_bytes = k(&mut s, layout.vertex_bytes());
    let vertex_at = s.m.bin(b, Bin::Mul, index, vertex_bytes);
    let vertex_at = s.m.bin(b, Bin::Add, vertex_at, base);
    // Each output from its four dwords of the vertex.
    for id in 0..s.outputs.len() {
        let offset = k(&mut s, id as u32 * 16);
        let at = s.m.bin(b, Bin::Add, vertex_at, offset);
        let cols = s.outputs[id].cols;
        let values = load(&mut s, at, ((1u16 << cols) - 1) as u8);
        let float = s.outputs[id].comp == 9;
        for col in 0..cols {
            let mut value = s.m.extract(b, values, u32::from(col));
            if id == position {
                let away = s.m.f32(CULLED_POSITION[col as usize].to_bits());
                value = s.m.select(b, culled, away, value);
            }
            let (name, value) = if float {
                ("storeOutput.f32", value)
            } else {
                ("storeOutput.i32", s.m.cast(b, Cast::Bitcast, value, i32t))
            };
            let args = [k(&mut s, id as u32), k(&mut s, 0), s.m.i8(col), value];
            s.op(b, name, Attr::NoUnwind, s.t.void, 5, &args);
        }
    }
    s.m.terminate(b, Term::Ret);
    Ok(HostShader {
        stage: crate::native::HwStage::Vs,
        dxil: s.finish(),
        user_data: Vec::new(),
        samplers: Vec::new(),
        static_samplers: Vec::new(),
        root_cbvs: Vec::new(),
        root_srvs: vec![SLOT_MESH_RECORDS],
        root_uavs: Vec::new(),
        root_indexed_views: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::{HwStage, MeshIo, NativeProgram, StageIo, Usage, VsOutput};

    const S_ENDPGM: u32 = 0xBF81_0000;

    /// A mesh program of 128 lanes, two console waves of 64: lane n of its
    /// wave exports the position (n, 0, 0, 1), parameter 0 (n, n, n, n) and
    /// the primitive (n, 0, 0).
    const PROGRAM: [u32; 16] = [
        0xD765_0000,
        0x0001_00C1, // v_mbcnt_lo_u32_b32 v0, -1, 0
        0xD766_0000,
        0x0002_00C1, // v_mbcnt_hi_u32_b32 v0, -1, v0
        0x7E02_0D00, // v_cvt_f32_u32 v1, v0
        0x7E04_0280, // v_mov_b32 v2, 0
        0x7E06_0280, // v_mov_b32 v3, 0
        0x7E08_02F2, // v_mov_b32 v4, 1.0
        0x7E12_0300, // v_mov_b32 v9, v0
        0xF800_00CF,
        0x0403_0201, // exp pos0 v1, v2, v3, v4
        0xF800_020F,
        0x0101_0101, // exp param0 v1, v1, v1, v1
        0xF800_0941,
        0x0000_0009, // exp prim v9, off, off, off done
        S_ENDPGM,
    ];

    fn element(name: &str, kind: u8, row: u32) -> SigElement {
        SigElement {
            name: name.into(),
            index: 0,
            kind,
            comp: 9,
            interp: 0,
            row: Some(row),
            col: 0,
            cols: 4,
            usage: 0,
            half: false,
        }
    }

    /// `PROGRAM` lowered as compute, as `translate_mesh_as_compute` lowers a
    /// container's mesh program, validated by `dxil.dll` when there is one.
    fn lowered() -> (HostShader, RecordLayout, Vec<SigElement>) {
        let code: Vec<u8> = PROGRAM.iter().flat_map(|w| w.to_le_bytes()).collect();
        let names: Vec<_> = crate::gfx10::decode(&code)
            .unwrap()
            .iter()
            .map(|i| i.name().unwrap_or("?"))
            .collect();
        assert_eq!(names[..2], ["v_mbcnt_lo_u32_b32", "v_mbcnt_hi_u32_b32"]);
        let native = NativeProgram {
            stage: HwStage::Ms,
            wave64: true,
            vgprs: 16,
            layout: Vec::new(),
            used: Vec::new(),
            io: StageIo::Ms(MeshIo {
                outputs: vec![VsOutput {
                    usage: Usage::Generic,
                    row: 1,
                    param: 0,
                    fp16: false,
                    pack2: false,
                }],
                clip_cntl: 0,
                threads: 128,
                max_vertices: 128,
                max_primitives: 128,
                topology: 2,
                lds_bytes: 0,
            }),
            code,
        };
        let p = crate::ir::build(&native.code, native.wave64, native.vgprs).unwrap();
        let st = crate::structure::structure(&p).unwrap();
        let res = crate::resource::track(&p, &native.layout, native.stage, Some(&st)).unwrap();
        let io = crate::io::raise(&p, &native).unwrap();
        LAST.set(None);
        AS_COMPUTE.set(true);
        let lowered = super::super::lower::lower(&super::super::lower::Parts {
            native: &native,
            p: &p,
            st: &st,
            io: &io,
            res: &res,
            inputs: Vec::new(),
            dropped: Vec::new(),
            outputs: vec![element("SV_Position", 3, 0), element("TEXCOORD", 0, 1)],
            prims: Vec::new(),
            mesh_stride: 128,
        });
        AS_COMPUTE.set(false);
        let lowered = lowered.unwrap();
        let (layout, outputs) = LAST.take().expect("the layout");
        let cs = HostShader {
            stage: HwStage::Cs,
            dxil: lowered.dxil,
            user_data: lowered.user_data,
            samplers: lowered.samplers,
            static_samplers: Vec::new(),
            root_cbvs: lowered.root_cbvs,
            root_srvs: lowered.root_srvs,
            root_uavs: lowered.root_uavs,
            root_indexed_views: res.root_indexed_views,
        };
        #[cfg(windows)]
        if let Ok(mut v) = super::super::validator::Validator::load(
            super::super::validator::Validator::default_path().as_deref(),
        ) {
            let mut copy = cs.dxil.clone();
            v.validate(&mut copy).expect("DXIL validation");
        }
        (cs, layout, outputs)
    }

    #[test]
    fn a_mesh_program_lowers_to_a_compute_shader_with_its_record_slot() {
        let (cs, layout, outputs) = lowered();
        assert_eq!(
            layout,
            RecordLayout {
                vertex_slots: 2,
                max_vertices: 128,
                max_primitives: 128,
            }
        );
        assert_eq!(outputs.len(), 2);
        assert_eq!(cs.root_uavs, [SLOT_MESH_RECORDS]);
        // A compute program (kind 5 in the DXIL part's version word).
        let c = crate::dxbc::Container::parse(&cs.dxil).unwrap();
        let dxil = c.part(crate::dxbc::FourCC::DXIL).unwrap().data;
        assert_eq!(u32::from_le_bytes(dxil[..4].try_into().unwrap()) >> 16, 5);
    }

    /// The compute shader runs on the host GPU (compute needs no mesh
    /// shaders) and leaves each group's record as the mesh program exported it.
    #[test]
    #[cfg(windows)]
    fn a_mesh_program_as_compute_writes_its_records() {
        use super::super::host::{Dispatch, Host, Root};
        let (cs, layout, _) = lowered();
        let _one = super::super::valu_tests::ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Ok(host) = Host::new(false) else {
            eprintln!("no SM 6.6 hardware device; skipped");
            return;
        };
        let (root, pso) = host.compute_pipeline(&cs).expect("pipeline");
        let groups = 2;
        let out = host
            .dispatch(
                &root,
                &pso,
                &Dispatch {
                    roots: vec![
                        (0, Root::Constants(vec![0; cs.user_data_dwords().max(1) as usize])),
                        (1, Root::Uav(layout.group_bytes() / 4 * groups)),
                    ],
                    views: &[],
                    samplers: &[],
                    groups: [groups, 1, 1],
                },
            )
            .expect("dispatch");
        let records = &out[0];
        let at = |offset: u64| records[(offset / 4) as usize];
        let one = 1.0f32.to_bits();
        for g in 0..groups {
            for n in 0..128 {
                let lane = (n % 64) as f32;
                let position = [0, 1, 2, 3].map(|c| at(layout.vertex_at(g, n, 0, c)));
                assert_eq!(position, [lane.to_bits(), 0, 0, one], "group {g} vertex {n}");
                let param = [0, 1, 2, 3].map(|c| at(layout.vertex_at(g, n, 1, c)));
                assert_eq!(param, [lane.to_bits(); 4], "group {g} vertex {n} param");
                assert_eq!(
                    at(layout.primitive_at(g, n)),
                    primitive_word(false, [n % 64, 0, 0]),
                    "group {g} primitive {n}"
                );
            }
        }
    }

    /// The title's mesh programs: 128 vertices and primitives, a median of
    /// 27 vertex scalars (seven elements).
    const TITLE: RecordLayout = RecordLayout {
        vertex_slots: 7,
        max_vertices: 128,
        max_primitives: 128,
    };

    #[test]
    fn a_record_holds_the_vertices_then_the_primitive_words() {
        assert_eq!(TITLE.vertex_bytes(), 112);
        assert_eq!(TITLE.primitives_at(), 128 * 112);
        assert_eq!(TITLE.group_bytes(), 128 * 112 + 512);
        assert_eq!(TITLE.vertex_at(0, 0, 0, 0), 0);
        assert_eq!(TITLE.vertex_at(0, 1, 2, 3), 112 + 32 + 12);
        assert_eq!(TITLE.vertex_at(3, 0, 0, 0), 3 * u64::from(TITLE.group_bytes()));
        assert_eq!(TITLE.primitive_at(0, 0), u64::from(TITLE.primitives_at()));
        assert_eq!(
            TITLE.primitive_at(2, 127),
            2 * u64::from(TITLE.group_bytes()) + u64::from(TITLE.primitives_at()) + 508
        );
        // The last primitive word ends the record.
        assert_eq!(TITLE.primitive_at(0, 127) + 4, u64::from(TITLE.group_bytes()));
    }

    #[test]
    fn draws_and_buffers_count_whole_groups() {
        assert_eq!(TITLE.draw_vertices(1), 384);
        assert_eq!(TITLE.draw_vertices(10), 3840);
        let group = u64::from(TITLE.group_bytes());
        assert_eq!(TITLE.groups_in(group * 5 - 1), 4);
        assert_eq!(TITLE.groups_in(group * 5), 5);
        // The largest outputs the title declares (71 scalars: 18 elements)
        // still fit two thousand groups in 128 MiB.
        let big = RecordLayout {
            vertex_slots: 18,
            ..TITLE
        };
        assert!(big.groups_in(128 << 20) >= 2000);
    }

    #[test]
    fn primitive_words_pack_three_indices_or_cull() {
        assert_eq!(primitive_word(true, [1, 2, 3]), CULLED);
        assert_eq!(primitive_word(false, [1, 2, 3]), 1 | 2 << 10 | 3 << 20);
        assert_eq!(primitive_word(false, [1023, 0, 1023]), 1023 | 1023 << 20);
        assert_eq!(primitive_word(false, [127, 127, 127]) & CULLED, 0);
    }

    /// A pixel shader that writes its `TEXCOORD0` (`outputs()` row 1) to
    /// target 0.
    fn texcoord_ps() -> HostShader {
        use super::super::bitcode::{Attr, Term};
        let mut s = super::super::shader::Shader::new(true);
        let mut position = element("SV_Position", 3, 0);
        position.interp = 4;
        let mut texcoord = element("TEXCOORD", 0, 1);
        texcoord.interp = 2;
        texcoord.usage = 0xF;
        s.inputs = vec![position, texcoord];
        let mut target = element("SV_Target", 16, 0);
        target.usage = 0xF;
        s.outputs = vec![target];
        let b = s.entry;
        let iu = s.m.undef(s.t.i32);
        for col in 0..4 {
            let args = [s.m.i32(1), s.m.i32(0), s.m.i8(col), iu];
            let v = s.op(b, "loadInput.f32", Attr::ReadNone, s.t.f32, 4, &args);
            let args = [s.m.i32(0), s.m.i32(0), s.m.i8(col), v];
            s.op(b, "storeOutput.f32", Attr::NoUnwind, s.t.void, 5, &args);
        }
        s.m.terminate(b, Term::Ret);
        HostShader {
            stage: HwStage::Ps,
            dxil: s.finish(),
            user_data: Vec::new(),
            samplers: Vec::new(),
            static_samplers: Vec::new(),
            root_cbvs: Vec::new(),
            root_srvs: Vec::new(),
            root_uavs: Vec::new(),
            root_indexed_views: Vec::new(),
        }
    }

    /// One record whose vertices 5, 6 and 7 make a triangle over the whole
    /// target with parameter 0 `color`, primitive 0 naming them (or culled),
    /// the other primitives culled.
    fn record(layout: RecordLayout, color: [f32; 4], culled: bool) -> Vec<u32> {
        let mut r = vec![0u32; layout.group_bytes() as usize / 4];
        let mut put = |offset: u64, v: u32| r[(offset / 4) as usize] = v;
        for (n, [x, y]) in [(5, [-1.0f32, -1.0]), (6, [-1.0, 3.0]), (7, [3.0, -1.0])] {
            for (c, v) in [x, y, 0.0, 1.0].into_iter().enumerate() {
                put(layout.vertex_at(0, n, 0, c as u32), v.to_bits());
            }
            for (c, v) in color.into_iter().enumerate() {
                put(layout.vertex_at(0, n, 1, c as u32), v.to_bits());
            }
        }
        for p in 0..layout.max_primitives {
            put(layout.primitive_at(0, p), CULLED);
        }
        put(layout.primitive_at(0, 0), primitive_word(culled, [5, 6, 7]));
        r
    }

    #[test]
    fn the_vertex_shader_validates_and_declares_the_mesh_outputs() {
        let (cs, layout, outputs) = lowered();
        let vs = MeshCompute { cs, layout, outputs }.vertex_shader().unwrap();
        assert_eq!(vs.root_srvs, [SLOT_MESH_RECORDS]);
        let c = crate::dxbc::Container::parse(&vs.dxil).unwrap();
        let osg1 = super::super::signature(c.part(crate::dxbc::FourCC::OSG1).unwrap().data).unwrap();
        let rows: Vec<_> = osg1.iter().map(|e| (e.name.as_str(), e.row)).collect();
        assert_eq!(rows, [("SV_Position", Some(0)), ("TEXCOORD", Some(1))]);
        #[cfg(windows)]
        if let Ok(mut v) = super::super::validator::Validator::load(
            super::super::validator::Validator::default_path().as_deref(),
        ) {
            let mut copy = vs.dxil.clone();
            v.validate(&mut copy).expect("DXIL validation");
        }
    }

    /// The vertex shader on the host GPU: a primitive's three corners come
    /// from the vertices its word names, with the outputs the pixel shader
    /// reads; a culled primitive draws nothing.
    #[test]
    #[cfg(windows)]
    fn the_vertex_shader_draws_a_record_and_skips_culled_primitives() {
        use super::super::host::{Draw, Host, Root};
        let (cs, layout, outputs) = lowered();
        let vs = MeshCompute { cs, layout, outputs }.vertex_shader().unwrap();
        let ps = texcoord_ps();
        let _one = super::super::valu_tests::ONE_AT_A_TIME
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Ok(host) = Host::new(false) else {
            eprintln!("no SM 6.6 hardware device; skipped");
            return;
        };
        let (root, pso) = host.pipeline(&vs, &ps).expect("pipeline");
        let color = [0.25, 0.5, 0.75, 1.0];
        let draw = |culled| {
            host.draw(
                &root,
                &pso,
                &Draw {
                    roots: vec![
                        (0, Root::Constants(vec![0])),
                        (1, Root::Constants(vec![0])),
                        (
                            2,
                            Root::View {
                                srv: true,
                                data: record(layout, color, culled),
                            },
                        ),
                    ],
                    textures: &[],
                    samplers: &[],
                    vertices: &[],
                    stride: 0,
                    size: (8, 8),
                    mesh_groups: 0,
                },
            )
            .expect("draw")
        };
        let drawn = draw(false);
        assert!(drawn.iter().all(|&t| t == color), "{:?}", &drawn[..4]);
        let skipped = draw(true);
        assert!(skipped.iter().all(|&t| t != color), "{:?}", &skipped[..4]);
    }
}
