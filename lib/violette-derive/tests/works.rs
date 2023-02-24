use bytemuck::{offset_of, Pod, Zeroable};
use violette::vertex::{VertexAttributes, VertexDesc};
use violette_derive::VertexAttributes;

#[test]
fn works() {
    #[derive(Debug, Default, Clone, Copy, Pod, Zeroable, VertexAttributes)]
    #[repr(C)]
    struct TestVertex {
        pos: [f32; 3],
        uv: [f32; 2],
        entity_id: u32,
    }

    let mut expected = vec![
        VertexDesc::from_gl_type::<[f32; 3]>(0),
        VertexDesc::from_gl_type::<[f32; 2]>(offset_of!(TestVertex, uv)),
        VertexDesc::from_gl_type::<u32>(offset_of!(TestVertex, entity_id)),
    ];
    let mut actual = TestVertex::attributes().to_vec();
    expected.sort_by_key(|d| d.offset);
    actual.sort_by_key(|d| d.offset);
    assert_eq!(actual, expected);
}
