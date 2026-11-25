//! Benchmarks on the host card, run by hand
//! (`cargo test -p victory-gpu --release bench -- --ignored --nocapture`):
//! what the card makes of the command patterns our lists record, measured
//! apart from any title.

use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, ID3DBlob};
use windows::Win32::Graphics::Direct3D12::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::core::{PCSTR, s};

use crate::host::{Host, transition};

/// A group's record as the title's mesh programs make them: 128 vertices
/// of seven elements, then 128 primitive words.
const VERTEX_BYTES: u32 = 7 * 16;
const GROUP_BYTES: u32 = 128 * VERTEX_BYTES + 128 * 4;

const COMPUTE: &str = r#"
#define RS "RootConstants(num32BitConstants=4, b0), UAV(u0)"
RWByteAddressBuffer records : register(u0);
cbuffer C : register(b0) { uint base; uint pad0; uint pad1; uint pad2; };
[RootSignature(RS)]
[numthreads(64, 1, 1)]
void main(uint3 thread : SV_GroupThreadID, uint3 group : SV_GroupID) {
    uint at = (base + group.x) * 14848;
    for (uint v = thread.x * 2; v < thread.x * 2 + 2; v++) {
        float x = (float)(v % 16) / 2048.0 + (float)(group.x % 32) / 20.0 - 0.9;
        float y = (float)(v / 16) / 2048.0 + (float)(base % 32) / 20.0 - 0.9;
        records.Store4(at + v * 112, asuint(float4(x, y, 0.5, 1.0)));
        for (uint e = 1; e < 7; e++) {
            records.Store4(at + v * 112 + e * 16, asuint(float4(v, e, group.x, 1.0) / 128.0));
        }
        records.Store(at + 128 * 112 + v * 4, v | ((v + 1) % 128) << 10 | ((v + 17) % 128) << 20);
    }
}
"#;

const DRAW: &str = r#"
#define RS "RootConstants(num32BitConstants=4, b0), SRV(t0)"
ByteAddressBuffer records : register(t0);
struct Out {
    float4 position : SV_Position;
    float4 a[6] : ATTRIBUTE;
};
[RootSignature(RS)]
Out vertex(uint id : SV_VertexID) {
    uint at = id / 384 * 14848;
    uint word = records.Load(at + 128 * 112 + id / 3 % 128 * 4);
    uint v = at + (word >> (10 * (id % 3)) & 1023) * 112;
    Out o;
    o.position = asfloat(records.Load4(v));
    for (uint e = 0; e < 6; e++) {
        o.a[e] = asfloat(records.Load4(v + 16 + e * 16));
    }
    return o;
}
[RootSignature(RS)]
float4 pixel(Out i) : SV_Target {
    return i.a[0] + i.a[5];
}
"#;

fn compile(source: &str, entry: PCSTR, target: PCSTR) -> Vec<u8> {
    let mut code: Option<ID3DBlob> = None;
    let mut errors: Option<ID3DBlob> = None;
    // SAFETY: the source and names live through the call; then the blobs'
    // own bytes.
    unsafe {
        let made = D3DCompile(
            source.as_ptr().cast(),
            source.len(),
            s!("bench"),
            None,
            None,
            entry,
            target,
            0,
            0,
            &mut code,
            Some(&mut errors),
        );
        if let Err(e) = made {
            let text = errors.map_or(String::new(), |b| {
                String::from_utf8_lossy(std::slice::from_raw_parts(b.GetBufferPointer().cast::<u8>(), b.GetBufferSize()))
                    .into_owned()
            });
            panic!("shader does not compile: {e} {text}");
        }
        let code = code.expect("code");
        std::slice::from_raw_parts(code.GetBufferPointer().cast::<u8>(), code.GetBufferSize()).to_vec()
    }
}

fn bytecode(code: &[u8]) -> D3D12_SHADER_BYTECODE {
    D3D12_SHADER_BYTECODE {
        pShaderBytecode: code.as_ptr().cast(),
        BytecodeLength: code.len(),
    }
}

struct MeshBench {
    compute_root: ID3D12RootSignature,
    compute: ID3D12PipelineState,
    draw_root: ID3D12RootSignature,
    draw: ID3D12PipelineState,
    records: ID3D12Resource,
    _target: ID3D12Resource,
    _views: ID3D12DescriptorHeap,
    view: D3D12_CPU_DESCRIPTOR_HANDLE,
}

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

impl MeshBench {
    fn new(host: &Host, record_bytes: u64) -> windows::core::Result<MeshBench> {
        let (cs, vs, ps) = (
            compile(COMPUTE, s!("main"), s!("cs_5_1")),
            compile(DRAW, s!("vertex"), s!("vs_5_1")),
            compile(DRAW, s!("pixel"), s!("ps_5_1")),
        );
        // SAFETY: plain object creation from live data.
        unsafe {
            let compute_root: ID3D12RootSignature = host.device.CreateRootSignature(0, &cs)?;
            let compute = host.device.CreateComputePipelineState(&D3D12_COMPUTE_PIPELINE_STATE_DESC {
                pRootSignature: std::mem::transmute_copy(&compute_root),
                CS: bytecode(&cs),
                ..Default::default()
            })?;
            let draw_root: ID3D12RootSignature = host.device.CreateRootSignature(0, &vs)?;
            let mut formats = [DXGI_FORMAT_UNKNOWN; 8];
            formats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
            let mut blend = D3D12_BLEND_DESC::default();
            blend.RenderTarget[0].RenderTargetWriteMask = D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8;
            let draw = host.device.CreateGraphicsPipelineState(&D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: std::mem::transmute_copy(&draw_root),
                VS: bytecode(&vs),
                PS: bytecode(&ps),
                BlendState: blend,
                SampleMask: u32::MAX,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                RTVFormats: formats,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                ..Default::default()
            })?;
            let records = host.uav_buffer(record_bytes).ok_or_else(windows::core::Error::empty)?;
            let mut target: Option<ID3D12Resource> = None;
            host.device.CreateCommittedResource(
                &D3D12_HEAP_PROPERTIES {
                    Type: D3D12_HEAP_TYPE_DEFAULT,
                    ..Default::default()
                },
                D3D12_HEAP_FLAG_NONE,
                &D3D12_RESOURCE_DESC {
                    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                    Width: u64::from(WIDTH),
                    Height: HEIGHT,
                    DepthOrArraySize: 1,
                    MipLevels: 1,
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                    Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET,
                    ..Default::default()
                },
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                None,
                &mut target,
            )?;
            let target = target.ok_or_else(windows::core::Error::empty)?;
            let views: ID3D12DescriptorHeap = host.device.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: 1,
                ..Default::default()
            })?;
            let view = views.GetCPUDescriptorHandleForHeapStart();
            host.device.CreateRenderTargetView(&target, None, view);
            Ok(MeshBench {
                compute_root,
                compute,
                draw_root,
                draw,
                records,
                _target: target,
                _views: views,
                view,
            })
        }
    }

    /// Records and runs `dispatches` mesh dispatches of `groups` groups
    /// each; the milliseconds from submission to the card's being done.
    /// `batch`: how many dispatches' compute runs before their draws (1: as
    /// `State::dispatch_mesh` records each mesh dispatch on its own).
    fn run(&self, host: &Host, dispatches: u32, groups: u32, batch: u32) -> f64 {
        let list = host.new_list().expect("list");
        let l = &list.list;
        let to_read = [transition(
            &self.records,
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
            D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
        )];
        let to_write = [transition(
            &self.records,
            D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
            D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
        )];
        // SAFETY: an open list of ours and live objects.
        unsafe {
            let gpu = self.records.GetGPUVirtualAddress();
            l.SetComputeRootSignature(&self.compute_root);
            l.SetGraphicsRootSignature(&self.draw_root);
            l.OMSetRenderTargets(1, Some(&self.view), false, None);
            l.RSSetViewports(&[D3D12_VIEWPORT {
                Width: WIDTH as f32,
                Height: HEIGHT as f32,
                MaxDepth: 1.0,
                ..Default::default()
            }]);
            l.RSSetScissorRects(&[windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: WIDTH as i32,
                bottom: HEIGHT as i32,
            }]);
            l.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            let mut first = 0;
            while first < dispatches {
                let n = batch.min(dispatches - first);
                l.SetPipelineState(&self.compute);
                l.SetComputeRootUnorderedAccessView(1, gpu);
                for i in 0..n {
                    // Each dispatch of the batch writes after the one before.
                    l.SetComputeRoot32BitConstant(0, i * groups, 0);
                    l.Dispatch(groups, 1, 1);
                }
                l.ResourceBarrier(&to_read);
                l.SetPipelineState(&self.draw);
                for i in 0..n {
                    l.SetGraphicsRootShaderResourceView(
                        1,
                        gpu + u64::from(i * groups) * u64::from(GROUP_BYTES),
                    );
                    l.DrawInstanced(groups * 384, 1, 0, 0);
                }
                l.ResourceBarrier(&to_write);
                first += n;
            }
        }
        list.close();
        let started = std::time::Instant::now();
        host.execute(&[&list]);
        let ms = started.elapsed().as_secs_f64() * 1e3;
        list.reset();
        ms
    }
}

/// What batching a run of mesh dispatches saves on this card: each
/// dispatch's compute, a transition, its draw and a transition back
/// (batch 1, as recorded today), against the run's computes, one
/// transition, its draws and one transition back.
#[test]
#[ignore = "a benchmark: run by hand with --ignored --nocapture"]
fn bench_mesh_dispatch_batching() {
    let Some(host) = crate::host::host() else {
        println!("no host card");
        return;
    };
    // (dispatches, groups each): a frame's direct mesh dispatches, and its
    // indirect ones once they run.
    for (dispatches, groups) in [(256u32, 4u32), (256, 18), (2304, 2)] {
        let most = [1u32, 8, 64, dispatches].into_iter().max().unwrap_or(1);
        let bytes = u64::from(most.min(dispatches) * groups) * u64::from(GROUP_BYTES);
        let bench = MeshBench::new(host, bytes).expect("bench objects");
        println!("{dispatches} mesh dispatches of {groups} groups ({} vertices drawn):", dispatches * groups * 384);
        let mut base = 0.0;
        for batch in [1u32, 8, 64, dispatches] {
            // The first run pays for whatever the driver makes on first use.
            bench.run(host, dispatches, groups, batch);
            let best = (0..5).map(|_| bench.run(host, dispatches, groups, batch)).fold(f64::MAX, f64::min);
            if batch == 1 {
                base = best;
            }
            println!(
                "  batches of {batch:5}: {best:8.2} ms on the card ({:6.1} us a dispatch, {:.2}x)",
                best * 1e3 / f64::from(dispatches),
                base / best
            );
        }
    }
}
