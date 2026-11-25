//! (PROJECT ONE) Barycentrics on a host that has none.
//!
//! A pixel shader that reads its barycentric weights (`SV_Barycentrics`)
//! or one vertex's value of an input (`attributeAtVertex`) needs
//! `D3D12_OPTIONS3.BarycentricsSupported`, which RDNA1 cards (RX 5600 XT)
//! do not have: the host refuses the whole pipeline and its draws are
//! dropped (the title's skin, hair strands, windshield wiper mask).
//!
//! Translated with [`translate_with_barycentric_inputs`], such a pixel
//! shader reads them from ordinary inputs instead:
//!
//! - the weights from an input named [`WEIGHTS`], three floats declared
//!   with the interpolation mode the console's code asked for (one element
//!   per mode);
//! - vertex `k`'s value of the element at register `row`, column `col`
//!   from a constant input named [`CORNER`] of semantic index
//!   [`corner_index`]`(row, col, k)`, of the element's columns and type.
//!
//! The geometry shader [`barycentric_gs`] puts between the vertex shader
//! and that pixel shader writes them: each vertex of the triangle carries
//! its one-hot weights (1, 0, 0), (0, 1, 0) or (0, 0, 1), which the
//! rasteriser interpolates into the barycentrics, perspective-correct or
//! not as the element's mode says; and every vertex carries the three
//! vertices' values of each element read per vertex. Vertex 0 is the
//! provoking vertex, as `SV_Barycentrics` orders them.

use std::cell::Cell;

use super::HostShader;
use super::bitcode::{Attr, Cast, Term, V};
use super::container::SigElement;
use super::shader::{Geometry, Shader};
use crate::ShaderError;
use crate::dxbc::{Container, FourCC};

/// The pixel shader input that carries the barycentric weights.
pub const WEIGHTS: &str = "VICTORYBARY";
/// The pixel shader inputs that carry one vertex's value of an element.
pub const CORNER: &str = "VICTORYVTX";

/// Vertices the geometry shader emits: the triangle it gets.
const VERTICES: u32 = 3;
/// The scalars a geometry shader may emit per invocation.
const MAX_SCALARS: u32 = 1024;
/// Input registers a pixel shader has.
const ROWS: u32 = 32;

thread_local! {
    /// Set while a pixel shader is translated with barycentric inputs.
    static FROM_INPUTS: Cell<bool> = const { Cell::new(false) };
}

/// Whether the shader being translated on this thread reads barycentrics
/// from inputs (see the module documentation).
pub(crate) fn from_inputs() -> bool {
    FROM_INPUTS.get()
}

/// The translation of a console shader container whose barycentrics and
/// per-vertex inputs are read from the inputs [`barycentric_gs`] writes.
pub fn translate_with_barycentric_inputs(container: &[u8]) -> Result<HostShader, ShaderError> {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            FROM_INPUTS.set(false);
        }
    }
    FROM_INPUTS.set(true);
    let _reset = Reset;
    super::translate(container)
}

/// `f` run as a translation with barycentric inputs (tests that call the
/// lowering directly).
#[cfg(test)]
pub(crate) fn with_inputs<T>(f: impl FnOnce() -> T) -> T {
    FROM_INPUTS.set(true);
    let r = f();
    FROM_INPUTS.set(false);
    r
}

/// The semantic index of the [`CORNER`] input that carries vertex `k`'s
/// value of the element beginning at register `row`, column `col`.
pub const fn corner_index(row: u32, col: u8, k: u8) -> u32 {
    (row * 4 + col as u32) * 3 + k as u32
}

/// The register, column and vertex a [`CORNER`] semantic index names.
const fn corner_of(index: u32) -> (u32, u8, u8) {
    let at = index / 3;
    (at / 4, (at % 4) as u8, (index % 3) as u8)
}

/// A register no element of `inputs` is in, from the last one down: rows
/// the console's own layout leaves free.
pub(crate) fn free_row(inputs: &[SigElement]) -> Option<u32> {
    (0..ROWS).rev().find(|&r| !inputs.iter().any(|e| e.row == Some(r)))
}

/// The geometry shader (a DXIL container) between the translated vertex
/// shader `vs` and a pixel shader `ps` translated with
/// [`translate_with_barycentric_inputs`] (both as
/// [`super::HostShader::dxil`]): one triangle in, the same triangle out,
/// with the vertex shader's outputs register for register and the inputs
/// `ps` reads barycentrics and per-vertex values from.
pub fn barycentric_gs(vs: &[u8], ps: &[u8]) -> Result<Vec<u8>, ShaderError> {
    let sig = |dxil: &[u8], fourcc| -> Result<Vec<SigElement>, ShaderError> {
        let c = Container::parse(dxil)?;
        match c.part(fourcc) {
            Some(part) => super::signature(part.data),
            None => Ok(Vec::new()),
        }
    };
    build(sig(vs, FourCC::OSG1)?, &sig(ps, FourCC::ISG1)?)
}

/// What an output of the geometry shader beyond the vertex shader's holds.
enum Extra {
    /// The one-hot weights of the emitted vertex.
    Weights,
    /// Vertex `k`'s value of the vertex shader's register `row` from
    /// column `col` on.
    Corner { row: u32, col: u8, k: u8 },
    /// The triangle's number.
    Primitive,
}

fn build(mut outputs: Vec<SigElement>, ps_inputs: &[SigElement]) -> Result<Vec<u8>, ShaderError> {
    let unsupported = |what| ShaderError::Unsupported { offset: 0, what };
    if outputs.iter().any(|e| e.row.is_none()) {
        return Err(unsupported("a vertex output without a register"));
    }
    for e in &mut outputs {
        // As `emit_program` declares a vertex shader's outputs (`rect.rs`).
        e.interp = match (e.kind, e.comp) {
            (3, _) => 4,
            (0 | 6 | 7, 9) => 2,
            (0, _) => 1,
            _ => 0,
        };
        e.usage = (1 << e.cols) - 1;
    }
    let vs_outputs = outputs.len();
    let mut extras = Vec::new();
    for e in ps_inputs {
        let extra = if e.name == WEIGHTS {
            Extra::Weights
        } else if e.name == CORNER {
            let (row, col, k) = corner_of(e.index);
            Extra::Corner { row, col, k }
        } else if e.kind == 10 && e.row.is_some() && !outputs.iter().any(|o| o.kind == 10) {
            Extra::Primitive
        } else {
            continue;
        };
        if outputs.iter().any(|o| o.row == e.row) {
            return Err(unsupported("a barycentric input in a register the vertex shader writes"));
        }
        // As a vertex shader's float outputs (`rect.rs`): the mode that
        // counts is the one the pixel shader declares, which a container's
        // signature part does not hold.
        let interp = match extra {
            Extra::Weights => 2,
            _ => 1,
        };
        outputs.push(SigElement {
            interp,
            usage: (1 << e.cols) - 1,
            ..e.clone()
        });
        extras.push(extra);
    }
    if extras.is_empty() {
        return Err(unsupported("a pixel shader without barycentric inputs"));
    }
    let scalars: u32 = outputs.iter().map(|e| u32::from(e.cols)).sum();
    if scalars * VERTICES > MAX_SCALARS {
        return Err(unsupported("more output components than a geometry shader emits"));
    }

    let mut s = Shader::new(false);
    s.geometry = Some(Geometry {
        max_vertices: VERTICES,
    });
    s.inputs = outputs[..vs_outputs].to_vec();
    s.outputs = outputs;
    let b = s.entry;

    // Every input of the three vertices: [element][column][vertex].
    let mut corners: Vec<Vec<[V; 3]>> = Vec::new();
    for id in 0..vs_outputs {
        let mut columns = Vec::new();
        for col in 0..s.inputs[id].cols {
            columns.push([0, 1, 2].map(|vertex| load(&mut s, id, col, vertex)));
        }
        corners.push(columns);
    }

    // What each extra output holds at each vertex: [extra][column][vertex].
    let (zero, one) = (s.m.f32(0), s.m.f32(1.0f32.to_bits()));
    let mut values: Vec<Vec<[V; 3]>> = Vec::new();
    for (n, extra) in extras.iter().enumerate() {
        let out = s.outputs[vs_outputs + n].clone();
        let columns = match *extra {
            Extra::Weights => (0..out.cols)
                .map(|c| [0, 1, 2].map(|v| if u32::from(c) == v { one } else { zero }))
                .collect(),
            Extra::Corner { row, col, k } => {
                let mut columns = Vec::new();
                for c in 0..out.cols {
                    let at = col + c;
                    let (id, within) = (0..vs_outputs)
                        .find_map(|id| {
                            let e = &s.inputs[id];
                            (e.row == Some(row) && e.col <= at && at < e.col + e.cols).then(|| (id, at - e.col))
                        })
                        .ok_or(unsupported("a per-vertex input the vertex shader does not write"))?;
                    let mut x = corners[id][usize::from(within)][usize::from(k)];
                    // The pixel shader's type of the value, from the vertex
                    // shader's.
                    let (from_float, to_float) = (s.inputs[id].comp == 9, out.comp == 9);
                    if from_float != to_float {
                        let ty = if to_float { s.t.f32 } else { s.t.i32 };
                        x = s.m.cast(b, Cast::Bitcast, x, ty);
                    }
                    columns.push([x; 3]);
                }
                columns
            }
            Extra::Primitive => {
                let n = s.op(b, "primitiveID.i32", Attr::ReadNone, s.t.i32, 108, &[]);
                vec![[n; 3]]
            }
        };
        values.push(columns);
    }

    let stream = s.m.i8(0);
    for vertex in 0..VERTICES as usize {
        for (id, columns) in corners.iter().chain(values.iter()).enumerate() {
            let name = if s.outputs[id].comp == 9 {
                "storeOutput.f32"
            } else {
                "storeOutput.i32"
            };
            for (col, at) in columns.iter().enumerate() {
                let args = [s.m.i32(id as u32), s.m.i32(0), s.m.i8(col as u8), at[vertex]];
                s.op(b, name, Attr::NoUnwind, s.t.void, 5, &args);
            }
        }
        s.op(b, "emitStream", Attr::NoUnwind, s.t.void, 97, &[stream]);
    }
    s.m.terminate(b, Term::Ret);
    Ok(s.finish())
}

/// Column `col` of input `id` at vertex `vertex` of the triangle.
fn load(s: &mut Shader, id: usize, col: u8, vertex: u32) -> V {
    let (name, ty) = if s.inputs[id].comp == 9 {
        ("loadInput.f32", s.t.f32)
    } else {
        ("loadInput.i32", s.t.i32)
    };
    let args = [s.m.i32(id as u32), s.m.i32(0), s.m.i8(col), s.m.i32(vertex)];
    let entry = s.entry;
    s.op(entry, name, Attr::ReadNone, ty, 4, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An element: `(kind, comp)` its system value and type, `(row, col,
    /// cols)` where it is.
    fn element(name: &str, index: u32, (kind, comp): (u8, u8), (row, col, cols): (u32, u8, u8), interp: u8) -> SigElement {
        SigElement {
            name: name.into(),
            index,
            kind,
            comp,
            interp,
            row: Some(row),
            col,
            cols,
            usage: (1 << cols) - 1,
            half: false,
        }
    }

    #[test]
    fn a_corner_index_names_its_register_column_and_vertex() {
        for (row, col, k) in [(0, 0, 0), (5, 2, 1), (31, 3, 2), (12, 1, 0)] {
            assert_eq!(corner_of(corner_index(row, col, k)), (row, col, k));
        }
    }

    #[test]
    fn a_free_row_is_one_no_input_is_in() {
        let inputs = [
            element("SV_Position", 0, (3, 9), (0, 0, 4), 4),
            element("TEXCOORD", 0, (0, 9), (31, 0, 4), 2),
        ];
        assert_eq!(free_row(&inputs), Some(30));
        let full: Vec<SigElement> = (0..ROWS).map(|r| element("TEXCOORD", r, (0, 9), (r, 0, 4), 2)).collect();
        assert_eq!(free_row(&full), None);
    }

    /// The vertex shader writes a position and a colour; the pixel shader
    /// reads both, perspective barycentrics, and the colour's three
    /// vertices' red.
    fn signatures() -> (Vec<SigElement>, Vec<SigElement>) {
        let vs = vec![
            element("SV_Position", 0, (3, 9), (0, 0, 4), 0),
            element("TEXCOORD", 0, (0, 9), (1, 0, 4), 0),
        ];
        let mut ps = vec![
            element("SV_Position", 0, (3, 9), (0, 0, 4), 4),
            element("TEXCOORD", 0, (0, 9), (1, 0, 4), 1),
            element(WEIGHTS, 0, (0, 9), (31, 0, 3), 2),
        ];
        for k in 0..3 {
            ps.push(element(CORNER, corner_index(1, 0, k), (0, 9), (30 - u32::from(k), 0, 1), 1));
        }
        (vs, ps)
    }

    #[test]
    fn the_geometry_shader_writes_what_the_pixel_shader_reads() {
        let (vs, ps) = signatures();
        let gs = build(vs, &ps).expect("a geometry shader");
        let c = Container::parse(&gs).expect("a container");
        let out = super::super::signature(c.part(FourCC::OSG1).expect("outputs").data).expect("a signature");
        for wanted in &ps {
            assert!(
                out.iter().any(|o| o.name == wanted.name && o.index == wanted.index && o.row == wanted.row),
                "{} {} is written",
                wanted.name,
                wanted.index
            );
        }
    }

    #[test]
    fn a_barycentric_input_in_a_written_register_is_refused() {
        let (vs, mut ps) = signatures();
        ps[2].row = Some(1);
        assert!(build(vs, &ps).is_err());
    }

    #[test]
    fn a_per_vertex_input_the_vertex_shader_lacks_is_refused() {
        let (vs, mut ps) = signatures();
        ps.push(element(CORNER, corner_index(7, 0, 0), (0, 9), (20, 0, 1), 1));
        assert!(build(vs, &ps).is_err());
    }

    /// The geometry shader passes DXIL validation (where a validator is).
    #[cfg(windows)]
    #[test]
    fn the_geometry_shader_validates() {
        use super::super::validator::Validator;
        let (vs, ps) = signatures();
        let gs = build(vs, &ps).expect("a geometry shader");
        if let Ok(mut v) = Validator::load(Validator::default_path().as_deref()) {
            v.validate(&mut gs.clone()).unwrap();
        }
    }
}
